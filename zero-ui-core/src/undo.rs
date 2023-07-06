//! Undo-redo app extension, service and commands.
//!

use std::{
    any::Any,
    fmt, mem,
    sync::{atomic::AtomicBool, Arc},
    time::{Duration, Instant},
};

use atomic::{Atomic, Ordering};
use parking_lot::Mutex;

use crate::{
    app::AppExtension,
    app_local, clmv, command,
    context::{StaticStateId, WIDGET},
    context_local,
    crate_util::RunOnDrop,
    event::{AnyEventArgs, Command, CommandNameExt, CommandScope},
    focus::commands::CommandFocusExt,
    gesture::CommandShortcutExt,
    keyboard::KEYBOARD,
    shortcut,
    text::Txt,
    var::*,
    widget_info::WidgetInfo,
    widget_instance::WidgetId,
};

/// Undo-redo app extension.
///
/// # Services
///
/// Services provided by this extension.
///
/// * [`UNDO`]
///
/// # Default
///
/// This extension is included in the [default app].
///
/// [default app]: crate::app::App::default
#[derive(Default)]
pub struct UndoManager {}

impl AppExtension for UndoManager {
    fn event(&mut self, update: &mut crate::event::EventUpdate) {
        // app scope handler
        if let Some(args) = UNDO_CMD.on_unhandled(update) {
            args.propagation().stop();
            UNDO.undo_n(args.param::<u32>().copied().unwrap_or(1));
        } else if let Some(args) = UNDO_CMD.on_unhandled(update) {
            args.propagation().stop();
            UNDO.redo_n(args.param::<u32>().copied().unwrap_or(1));
        }
    }
}

/// Undo-redo service.
pub struct UNDO;
impl UNDO {
    /// Gets or sets the size limit of each undo stack of each scope.
    ///
    /// Is `u32::MAX` by default. If the limit is reached the oldest undo action is dropped without redo.
    pub fn max_undo(&self) -> BoxedVar<u32> {
        UNDO_SV.read().max_undo.clone()
    }

    /// Gets or sets the time interval that [`undo`] and [`redo`] cover each call.
    ///
    /// This value applies to all scopes and defines the max interval between actions
    /// that are undone in a single call.
    ///
    /// Is the [keyboard repeat interval] times 4 by default.
    ///
    /// [`undo`]: Self::undo
    /// [`redo`]: Self::redo
    /// [keyboard repeat interval]: crate::keyboard::KEYBOARD::repeat_config
    pub fn undo_interval(&self) -> BoxedVar<Duration> {
        UNDO_SV.read().undo_interval.clone()
    }

    /// Gets if the undo service is enabled in the current context.
    ///
    /// If `false` calls to [`register`] are ignored.
    ///
    /// [`register`]: Self::register
    pub fn is_enabled(&self) -> bool {
        UNDO_SCOPE_CTX.get().enabled.load(Ordering::Relaxed) && UNDO_SV.read().max_undo.get() > 0
    }

    /// Undo `n` times in the current scope.
    pub fn undo_n(&self, n: u32) {
        UNDO_SCOPE_CTX.get().undo_n(n);
    }

    /// Redo `n` times in the current scope.
    pub fn redo_n(&self, n: u32) {
        UNDO_SCOPE_CTX.get().redo_n(n);
    }

    /// Undo all actions within the `t` interval of each other, starting from the most recent action.
    pub fn undo_t(&self, t: Duration) {
        UNDO_SCOPE_CTX.get().undo_t(t);
    }

    /// Redo all actions within the `t` interval of each other, starting from the most recent action.
    pub fn redo_t(&self, t: Duration) {
        UNDO_SCOPE_CTX.get().redo_t(t);
    }

    /// Undo all actions within the [`undo_interval`].
    ///
    /// [`undo_interval`]: Self::undo_interval
    pub fn undo(&self) {
        self.undo_t(UNDO_SV.read().undo_interval.get());
    }

    /// Redo all actions within the [`undo_interval`].
    ///
    /// [`undo_interval`]: Self::undo_interval
    pub fn redo(&self) {
        self.redo_t(UNDO_SV.read().undo_interval.get());
    }

    /// Gets the parent ID that defines an undo scope, or `None` if undo is registered globally for
    /// the entire app.
    pub fn scope(&self) -> Option<WidgetId> {
        UNDO_SCOPE_CTX.get().id()
    }

    /// Register an already executed action for undo in the current scope.
    pub fn register(&self, action: impl UndoAction) {
        UNDO_SCOPE_CTX.get().register(Box::new(action))
    }

    /// Register an already executed action for undo in the current scope.
    ///
    /// The action is defined as a closure `op` that matches over [`UndoOp`] to implement undo and redo.
    pub fn register_op(&self, description: impl Into<Txt>, op: impl FnMut(UndoOp) + Send + 'static) {
        self.register(UndoRedoOp {
            description: description.into(),
            op: Box::new(op),
        })
    }

    /// Run the `action` and register the undo in the current scope.
    pub fn run(&self, action: impl RedoAction) {
        UNDO_SCOPE_CTX.get().register(Box::new(action).redo())
    }

    /// Run the `op` once with [`UndoOp::Redo`] and register it for undo in the current scope.
    pub fn run_op(&self, description: impl Into<Txt>, op: impl FnMut(UndoOp) + Send + 'static) {
        self.run(UndoRedoOp {
            description: description.into(),
            op: Box::new(op),
        })
    }

    /// Run `actions` as a [`transaction`] and commits as a group if any undo action is captured.
    ///
    /// [`transaction`]: Self::transaction
    pub fn group(&self, description: impl Into<Txt>, actions: impl FnOnce()) -> bool {
        let t = self.transaction(actions);
        let any = !t.is_empty();
        if any {
            t.commit_group(description);
        }
        any
    }

    /// Run `actions` in a new undo scope, capturing all undo actions inside it into a new
    /// [`UndoTransaction`].
    ///
    /// The transaction can be immediately undone or committed.
    pub fn transaction(&self, actions: impl FnOnce()) -> UndoTransaction {
        let mut scope = UndoScope::default();
        let parent_scope = UNDO_SCOPE_CTX.get();
        *scope.enabled.get_mut() = parent_scope.enabled.load(Ordering::Relaxed);
        *scope.id.get_mut() = parent_scope.id.load(Ordering::Relaxed);

        let t_scope = Arc::new(scope);
        let _panic_undo = RunOnDrop::new(clmv!(t_scope, || {
            for undo in mem::take(&mut *t_scope.undo.lock()).into_iter().rev() {
                let _ = undo.action.undo();
            }
        }));

        let mut scope = Some(t_scope);
        UNDO_SCOPE_CTX.with_context(&mut scope, actions);

        let scope = scope.unwrap();
        let undo = mem::take(&mut *scope.undo.lock());

        UndoTransaction { undo }
    }

    /// Run `actions` as a [`transaction`] and commits as a group if the result is `Ok(O)` and at least one
    /// undo action was registered, or undoes all if result is `Err(E)`.
    ///
    /// [`transaction`]: Self::transaction
    pub fn try_group<O, E>(&self, description: impl Into<Txt>, actions: impl FnOnce() -> Result<O, E>) -> Result<O, E> {
        let mut r = None;
        let t = self.transaction(|| r = Some(actions()));
        let r = r.unwrap();
        if !t.is_empty() {
            if r.is_ok() {
                t.commit_group(description);
            } else {
                t.undo();
            }
        }
        r
    }

    /// Run `actions` as a [`transaction`] and commits if the result is `Ok(O)`, or undoes all if result is `Err(E)`.
    ///
    /// [`transaction`]: Self::transaction
    pub fn try_commit<O, E>(&self, actions: impl FnOnce() -> Result<O, E>) -> Result<O, E> {
        let mut r = None;
        let t = self.transaction(|| r = Some(actions()));
        let r = r.unwrap();
        if !t.is_empty() {
            if r.is_ok() {
                t.commit();
            } else {
                t.undo();
            }
        }
        r
    }

    /// Runs `f` in a new `scope`. All undo actions inside `f` are registered in the `scope`.
    pub fn with_scope<R>(&self, scope: &mut WidgetUndoScope, f: impl FnOnce() -> R) -> R {
        UNDO_SCOPE_CTX.with_context(&mut scope.0, f)
    }

    /// Runs `f` in a disabled scope, all undo actions registered inside `f` are ignored.
    pub fn with_disabled<R>(&self, f: impl FnOnce() -> R) -> R {
        let mut scope = UndoScope::default();
        let parent_scope = UNDO_SCOPE_CTX.get();
        *scope.enabled.get_mut() = false;
        *scope.id.get_mut() = parent_scope.id.load(Ordering::Relaxed);

        UNDO_SCOPE_CTX.with_context_value(scope, f)
    }

    /// Track changes on `var`, registering undo actions for it.
    ///
    /// The variable will be tracked until the returned handle or the var is dropped.
    ///
    /// Note that this will keep strong clones of previous and new value every time the variable changes, but
    /// it will only keep weak references to the variable. Dropping the handle or the var will not remove undo/redo
    /// entries for it, they will still try to assign the variable, failing silently if the variable is dropped too.
    ///
    /// Var updates caused by undo and redo are tagged with [`UndoVarModifyTag`].
    pub fn watch_var<T: VarValue>(&self, description: impl Into<Txt>, var: impl Var<T>) -> VarHandle {
        if var.capabilities().is_always_read_only() {
            return VarHandle::dummy();
        }
        let var = var.actual_var();
        let wk_var = var.downgrade();

        let mut prev_value = Some(var.get());
        let description = description.into();

        var.trace_value(move |args| {
            if args.downcast_tags::<UndoVarModifyTag>().next().is_none() {
                let prev = prev_value.take().unwrap();
                let new = args.value();
                if &prev == new {
                    // no actual change
                    prev_value = Some(prev);
                    return;
                }
                prev_value = Some(new.clone());
                UNDO.register_op(
                    description.clone(),
                    clmv!(wk_var, new, |op| if let Some(var) = wk_var.upgrade() {
                        let _ = match op {
                            UndoOp::Undo => var.modify(clmv!(prev, |args| {
                                args.set(prev);
                                args.push_tag(UndoVarModifyTag);
                            })),
                            UndoOp::Redo => var.modify(clmv!(new, |args| {
                                args.set(new);
                                args.push_tag(UndoVarModifyTag);
                            })),
                        };
                    }),
                );
            }
        })
    }

    /// Clear all redo actions.
    pub fn clear_redo(&self) {
        UNDO_SCOPE_CTX.get().redo.lock().clear();
    }

    /// Clear all undo and redo actions.
    pub fn clear(&self) {
        let ctx = UNDO_SCOPE_CTX.get();
        let mut u = ctx.undo.lock();
        u.clear();
        ctx.redo.lock().clear();
    }
}

/// Identifies that a var modify requested by undo/redo action.
///
/// See [`UNDO.watch_var`] for more details.
///
/// [`UNDO.watch_var`]: UNDO::watch_var
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UndoVarModifyTag;

/// Represents an undo or redo action.
///
/// If formatted to display it should provide a short description of the action
/// that will be undone or redone.
pub trait UndoRedoItem: fmt::Debug + fmt::Display + Send + Any {}

/// Represents a single undo action.
pub trait UndoAction: UndoRedoItem {
    /// Undo action and returns a [`RedoAction`] that redoes it.
    fn undo(self: Box<Self>) -> Box<dyn RedoAction>;
}

/// Represents a single redo action.
pub trait RedoAction: UndoRedoItem {
    /// Redo action and returns a [`UndoAction`] that undoes it.
    fn redo(self: Box<Self>) -> Box<dyn UndoAction>;
}

/// Represents an undo/redo action.
///
/// This can be used to implement undo & redo in a single closure. See [`UNDO.register_op`] and
/// [`UNDO.run_op`] for more details.
///
/// [`UNDO.register_op`]: UNDO::register_op
/// [`UNDO.run_op`]: UNDO::run_op
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndoOp {
    /// Undo the action.
    Undo,
    /// Redo the action.
    Redo,
}

/// Represents captured undo actions in an [`UNDO.transaction`] operation.
///
/// [`UNDO.transaction`]: UNDO::transaction
#[must_use = "dropping the transaction undoes all captured actions"]
pub struct UndoTransaction {
    undo: Vec<UndoEntry>,
}
impl UndoTransaction {
    /// If the transaction did not capture any undo action.
    pub fn is_empty(&self) -> bool {
        self.undo.is_empty()
    }

    /// Push all undo actions captured by the transaction into the current undo scope.
    pub fn commit(mut self) {
        let mut undo = mem::take(&mut self.undo);
        let now = Instant::now();
        for u in &mut undo {
            u.timestamp = now;
        }
        let ctx = UNDO_SCOPE_CTX.get();
        let mut ctx_undo = ctx.undo.lock();
        if ctx_undo.is_empty() {
            *ctx_undo = undo;
        } else {
            ctx_undo.extend(undo);
        }
    }

    /// Push a single action in the current undo scope that undoes/redoes all the captured
    /// actions in the transaction.
    ///
    /// Note that this will register a group item even if the transaction is empty.
    pub fn commit_group(mut self, description: impl Into<Txt>) {
        UNDO.register(UndoGroup {
            description: description.into(),
            undo: mem::take(&mut self.undo),
        })
    }

    /// Cancel the transaction, undoes all captured actions.
    ///
    /// This is the same as dropping the transaction.
    pub fn undo(self) {
        let _ = self;
    }
}
impl Drop for UndoTransaction {
    fn drop(&mut self) {
        for undo in self.undo.drain(..).rev() {
            let _ = undo.action.undo();
        }
    }
}

command! {
    /// Represents the **undo** action.
    ///
    /// # Param
    ///
    /// If the command parameter is a `u32` it is the count of undo actions to run, otherwise runs `1` action.
    ///
    /// # Scope
    ///
    /// You can use [`CommandUndoExt::undo_scoped`] to get a command variable that is always scoped on the
    /// focused undo scope.
    pub static UNDO_CMD = {
        name: "Undo",
        shortcut: [shortcut!(CTRL+Z)],
    };

    /// Represents the clipboard **redo** action.
    ///
    /// # Param
    ///
    /// If the command parameter is a `u32` it is the count of redo actions to run, otherwise runs `1` action.
    pub static REDO_CMD = {
        name: "Redo",
        shortcut: [shortcut!(CTRL+Y)],
    };
}

/// Represents an widget undo scope.
///
/// See [`UNDO.with_scope`] for more details.
///
/// [`UNDO.with_scope`]: UNDO::with_scope
pub struct WidgetUndoScope(Option<Arc<UndoScope>>);
impl WidgetUndoScope {
    /// New, not inited in an widget.
    pub const fn new() -> Self {
        Self(None)
    }

    /// if the scope is already inited in a widget.
    pub fn is_inited(&self) -> bool {
        self.0.is_some()
    }

    /// Init the scope in the [`WIDGET`].
    pub fn init(&mut self) {
        let mut scope = UndoScope::default();
        *scope.id.get_mut() = Some(WIDGET.id());
        self.0 = Some(Arc::new(scope));
    }

    /// Sets the [`WIDGET`] info.
    pub fn info(&mut self, info: &mut crate::widget_info::WidgetInfoBuilder) {
        info.flag_meta(&FOCUS_SCOPE_ID);
    }

    /// Deinit the scope in the [`WIDGET`].
    ///
    /// This clears the undo/redo stack of the scope.
    pub fn deinit(&mut self) {
        self.0 = None;
    }

    /// Sets if the undo/redo is enabled in this scope.
    ///
    /// Is `true` by default.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.0.as_ref().unwrap().enabled.store(enabled, Ordering::Relaxed);
    }

    /// Gets if the undo stack is not empty.
    pub fn can_undo(&self) -> bool {
        !self.0.as_ref().unwrap().undo.lock().is_empty()
    }

    /// Gets if the redo stack is not empty.
    pub fn can_redo(&self) -> bool {
        !self.0.as_ref().unwrap().redo.lock().is_empty()
    }
}

struct UndoScope {
    id: Atomic<Option<WidgetId>>,
    undo: Mutex<Vec<UndoEntry>>,
    redo: Mutex<Vec<RedoEntry>>,
    enabled: AtomicBool,
}
impl Default for UndoScope {
    fn default() -> Self {
        Self {
            id: Default::default(),
            undo: Default::default(),
            redo: Default::default(),
            enabled: AtomicBool::new(true),
        }
    }
}
impl UndoScope {
    fn with_enabled_undo_redo(&self, f: impl FnOnce(&mut Vec<UndoEntry>, &mut Vec<RedoEntry>)) {
        let mut undo = self.undo.lock();
        let mut redo = self.redo.lock();

        let max_undo = if self.enabled.load(Ordering::Relaxed) {
            UNDO_SV.read().max_undo.get() as usize
        } else {
            0
        };

        if undo.len() > max_undo {
            undo.reverse();
            while undo.len() > max_undo {
                undo.pop();
            }
            undo.reverse();
        }

        if redo.len() > max_undo {
            redo.reverse();
            while redo.len() > max_undo {
                redo.pop();
            }
            redo.reverse();
        }

        if max_undo > 0 {
            f(&mut undo, &mut redo);
        }
    }

    fn register(&self, action: Box<dyn UndoAction>) {
        self.with_enabled_undo_redo(|undo, redo| {
            undo.push(UndoEntry {
                timestamp: Instant::now(),
                action,
            });
            redo.clear();
        });
    }

    fn undo_n(&self, mut count: u32) {
        let mut actions = Vec::with_capacity(count.min(5) as usize);

        self.with_enabled_undo_redo(|undo, _| {
            while count > 0 {
                count -= 1;
                if let Some(undo) = undo.pop() {
                    actions.push(undo);
                } else {
                    break;
                }
            }
        });

        for undo in actions {
            let redo = undo.action.undo();
            self.redo.lock().push(RedoEntry {
                timestamp: undo.timestamp,
                action: redo,
            });
        }
    }

    fn redo_n(&self, mut count: u32) {
        let mut actions = Vec::with_capacity(count.min(5) as usize);

        self.with_enabled_undo_redo(|_, redo| {
            while count > 0 {
                count -= 1;
                if let Some(redo) = redo.pop() {
                    actions.push(redo);
                } else {
                    break;
                }
            }
        });

        for redo in actions {
            let undo = redo.action.redo();
            self.undo.lock().push(UndoEntry {
                timestamp: redo.timestamp,
                action: undo,
            });
        }
    }

    fn undo_t(&self, t: Duration) {
        let mut actions = vec![];

        self.with_enabled_undo_redo(|undo, _| {
            if let Some(mut prev_ts) = undo.last().map(|e| e.timestamp) {
                while let Some(action) = undo.pop() {
                    if prev_ts.checked_duration_since(action.timestamp).unwrap_or(t) <= t {
                        prev_ts = action.timestamp;
                        actions.push(action);
                    } else {
                        undo.push(action);
                        break;
                    }
                }
            }
        });

        for undo in actions {
            let redo = undo.action.undo();
            self.redo.lock().push(RedoEntry {
                timestamp: undo.timestamp,
                action: redo,
            });
        }
    }

    fn redo_t(&self, t: Duration) {
        let mut actions = vec![];

        self.with_enabled_undo_redo(|_, redo| {
            if let Some(mut prev_ts) = redo.last().map(|e| e.timestamp) {
                while let Some(action) = redo.pop() {
                    if action.timestamp.checked_duration_since(prev_ts).unwrap_or(t) <= t {
                        prev_ts = action.timestamp;
                        actions.push(action);
                    } else {
                        redo.push(action);
                        break;
                    }
                }
            }
        });

        for redo in actions {
            let undo = redo.action.redo();
            self.undo.lock().push(UndoEntry {
                timestamp: redo.timestamp,
                action: undo,
            });
        }
    }

    fn id(&self) -> Option<WidgetId> {
        self.id.load(Ordering::Relaxed)
    }
}

struct UndoEntry {
    timestamp: Instant,
    action: Box<dyn UndoAction>,
}

struct RedoEntry {
    timestamp: Instant,
    action: Box<dyn RedoAction>,
}

struct UndoGroup {
    description: Txt,
    undo: Vec<UndoEntry>,
}
impl fmt::Debug for UndoGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UndoGroup")
            .field("description", &self.description)
            .field("len()", &self.undo.len())
            .finish()
    }
}
impl fmt::Display for UndoGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description)
    }
}
impl UndoRedoItem for UndoGroup {}
impl UndoAction for UndoGroup {
    fn undo(self: Box<Self>) -> Box<dyn RedoAction> {
        let mut redo = Vec::with_capacity(self.undo.len());
        for undo in self.undo.into_iter().rev() {
            redo.push(RedoEntry {
                timestamp: undo.timestamp,
                action: undo.action.undo(),
            });
        }
        Box::new(RedoGroup {
            description: self.description,
            redo,
        })
    }
}
struct RedoGroup {
    description: Txt,
    redo: Vec<RedoEntry>,
}
impl fmt::Debug for RedoGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RedoGroup")
            .field("description", &self.description)
            .field("len()", &self.redo.len())
            .finish()
    }
}
impl fmt::Display for RedoGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description)
    }
}
impl UndoRedoItem for RedoGroup {}
impl RedoAction for RedoGroup {
    fn redo(self: Box<Self>) -> Box<dyn UndoAction> {
        let mut undo = Vec::with_capacity(self.redo.len());
        for redo in self.redo.into_iter().rev() {
            undo.push(UndoEntry {
                timestamp: redo.timestamp,
                action: redo.action.redo(),
            });
        }
        Box::new(UndoGroup {
            description: self.description,
            undo,
        })
    }
}

struct UndoRedoOp {
    description: Txt,
    op: Box<dyn FnMut(UndoOp) + Send>,
}
impl fmt::Debug for UndoRedoOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UndoRedoOp")
            .field("description", &self.description)
            .finish_non_exhaustive()
    }
}
impl fmt::Display for UndoRedoOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description)
    }
}
impl UndoRedoItem for UndoRedoOp {}
impl UndoAction for UndoRedoOp {
    fn undo(mut self: Box<Self>) -> Box<dyn RedoAction> {
        (self.op)(UndoOp::Undo);
        self
    }
}
impl RedoAction for UndoRedoOp {
    fn redo(mut self: Box<Self>) -> Box<dyn UndoAction> {
        (self.op)(UndoOp::Redo);
        self
    }
}

struct UndoService {
    max_undo: BoxedVar<u32>,
    undo_interval: BoxedVar<Duration>,
}

impl Default for UndoService {
    fn default() -> Self {
        Self {
            max_undo: var(u32::MAX).boxed(),
            undo_interval: KEYBOARD.repeat_config().map(|c| c.interval * 4).cow().boxed(),
        }
    }
}

context_local! {
    static UNDO_SCOPE_CTX: UndoScope = UndoScope::default();
}
app_local! {
    static UNDO_SV: UndoService = UndoService::default();
}

/// Undo extension methods for widget info.
pub trait WidgetInfoUndoExt {
    /// Returns `true` if the widget is an undo scope.
    fn is_undo_scope(&self) -> bool;

    /// Gets the first ancestor that is an undo scope.
    fn undo_scope(&self) -> Option<WidgetInfo>;
}
impl WidgetInfoUndoExt for WidgetInfo {
    fn is_undo_scope(&self) -> bool {
        self.meta().flagged(&FOCUS_SCOPE_ID)
    }

    fn undo_scope(&self) -> Option<WidgetInfo> {
        self.ancestors().find(WidgetInfoUndoExt::is_undo_scope)
    }
}

static FOCUS_SCOPE_ID: StaticStateId<()> = StaticStateId::new_unique();

/// Undo extension methods for commands.
pub trait CommandUndoExt {
    /// Gets the command scoped in the undo scope widget that is or contains the focused widget, or
    /// scoped on the app if there is no focused undo scope.
    fn undo_scoped(self) -> BoxedVar<Command>;
}
impl CommandUndoExt for Command {
    fn undo_scoped(self) -> BoxedVar<Command> {
        self.focus_scoped_with(|w| match w {
            Some(w) => {
                if w.is_undo_scope() {
                    CommandScope::Widget(w.id())
                } else if let Some(scope) = w.undo_scope() {
                    CommandScope::Widget(scope.id())
                } else {
                    CommandScope::App
                }
            }
            None => CommandScope::App,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::app::App;

    use super::*;

    #[test]
    fn register() {
        let _a = App::minimal();
        let data = Arc::new(Mutex::new(vec![1, 2]));

        UNDO.register(PushAction {
            data: data.clone(),
            item: 1,
        });
        UNDO.register(PushAction {
            data: data.clone(),
            item: 2,
        });
        assert_eq!(&[1, 2], &data.lock()[..]);

        UNDO.undo_n(1);
        assert_eq!(&[1], &data.lock()[..]);
        UNDO.undo_n(1);
        assert_eq!(&[] as &[u8], &data.lock()[..]);

        UNDO.redo_n(1);
        assert_eq!(&[1], &data.lock()[..]);
        UNDO.redo_n(1);
        assert_eq!(&[1, 2], &data.lock()[..]);
    }

    fn push_1_2(data: &Arc<Mutex<Vec<u8>>>) {
        UNDO.run_op(
            "push 1",
            clmv!(data, |op| match op {
                UndoOp::Undo => assert_eq!(data.lock().pop(), Some(1)),
                UndoOp::Redo => data.lock().push(1),
            }),
        );
        UNDO.run_op(
            "push 2",
            clmv!(data, |op| match op {
                UndoOp::Undo => assert_eq!(data.lock().pop(), Some(2)),
                UndoOp::Redo => data.lock().push(2),
            }),
        );
    }

    #[test]
    fn run_op() {
        let _a = App::minimal();
        let data = Arc::new(Mutex::new(vec![]));

        push_1_2(&data);
        assert_eq!(&[1, 2], &data.lock()[..]);

        UNDO.undo_n(1);
        assert_eq!(&[1], &data.lock()[..]);
        UNDO.undo_n(1);
        assert_eq!(&[] as &[u8], &data.lock()[..]);

        UNDO.redo_n(1);
        assert_eq!(&[1], &data.lock()[..]);
        UNDO.redo_n(1);
        assert_eq!(&[1, 2], &data.lock()[..]);
    }

    #[test]
    fn transaction_undo() {
        let _a = App::minimal();
        let data = Arc::new(Mutex::new(vec![]));

        let t = UNDO.transaction(|| {
            push_1_2(&data);
        });

        assert_eq!(&[1, 2], &data.lock()[..]);
        UNDO.undo_n(1);
        assert_eq!(&[1, 2], &data.lock()[..]);

        t.undo();
        assert_eq!(&[] as &[u8], &data.lock()[..]);
    }

    #[test]
    fn transaction_commit() {
        let _a = App::minimal();
        let data = Arc::new(Mutex::new(vec![]));

        let t = UNDO.transaction(|| {
            push_1_2(&data);
        });

        assert_eq!(&[1, 2], &data.lock()[..]);
        UNDO.undo_n(1);
        assert_eq!(&[1, 2], &data.lock()[..]);

        t.commit();

        UNDO.undo_n(1);
        assert_eq!(&[1], &data.lock()[..]);
        UNDO.undo_n(1);
        assert_eq!(&[] as &[u8], &data.lock()[..]);

        UNDO.redo_n(1);
        assert_eq!(&[1], &data.lock()[..]);
        UNDO.redo_n(1);
        assert_eq!(&[1, 2], &data.lock()[..]);
    }

    #[test]
    fn transaction_group() {
        let _a = App::minimal();
        let data = Arc::new(Mutex::new(vec![]));

        let t = UNDO.transaction(|| {
            push_1_2(&data);
        });

        assert_eq!(&[1, 2], &data.lock()[..]);
        UNDO.undo_n(1);
        assert_eq!(&[1, 2], &data.lock()[..]);

        t.commit_group("push 1, 2");

        UNDO.undo_n(1);
        assert_eq!(&[] as &[u8], &data.lock()[..]);

        UNDO.redo_n(1);
        assert_eq!(&[1, 2], &data.lock()[..]);
    }

    fn push_1_sleep_2(data: &Arc<Mutex<Vec<u8>>>) {
        UNDO.run_op(
            "push 1",
            clmv!(data, |op| match op {
                UndoOp::Undo => assert_eq!(data.lock().pop(), Some(1)),
                UndoOp::Redo => data.lock().push(1),
            }),
        );
        std::thread::sleep(100.ms());
        UNDO.run_op(
            "push 2",
            clmv!(data, |op| match op {
                UndoOp::Undo => assert_eq!(data.lock().pop(), Some(2)),
                UndoOp::Redo => data.lock().push(2),
            }),
        );
    }

    #[test]
    fn undo_redo_t_zero() {
        let _a = App::minimal();
        let data = Arc::new(Mutex::new(vec![]));

        push_1_sleep_2(&data);
        assert_eq!(&[1, 2], &data.lock()[..]);

        UNDO.undo_t(Duration::ZERO);
        assert_eq!(&[1], &data.lock()[..]);
        UNDO.undo_t(Duration::ZERO);
        assert_eq!(&[] as &[u8], &data.lock()[..]);

        UNDO.redo_t(Duration::ZERO);
        assert_eq!(&[1], &data.lock()[..]);
        UNDO.redo_t(Duration::ZERO);
        assert_eq!(&[1, 2], &data.lock()[..]);
    }

    #[test]
    fn undo_redo_t_max() {
        undo_redo_t_large(Duration::MAX);
    }

    #[test]
    fn undo_redo_t_10s() {
        undo_redo_t_large(10.secs());
    }

    fn undo_redo_t_large(t: Duration) {
        let _a = App::minimal();
        let data = Arc::new(Mutex::new(vec![]));

        push_1_sleep_2(&data);
        assert_eq!(&[1, 2], &data.lock()[..]);

        UNDO.undo_t(t);
        assert_eq!(&[] as &[u8], &data.lock()[..]);

        UNDO.redo_t(t);
        assert_eq!(&[1, 2], &data.lock()[..]);
    }

    #[test]
    fn watch_var() {
        let mut app = App::minimal().run_headless(false);

        let test_var = var(0);
        UNDO.watch_var("set test var", test_var.clone()).perm();

        test_var.set(10);
        app.update(false).assert_wait();

        test_var.set(20);
        app.update(false).assert_wait();

        assert_eq!(20, test_var.get());

        UNDO.undo_n(1);
        app.update(false).assert_wait();
        assert_eq!(10, test_var.get());

        UNDO.undo_n(1);
        app.update(false).assert_wait();
        assert_eq!(0, test_var.get());

        UNDO.redo_n(1);
        app.update(false).assert_wait();
        assert_eq!(10, test_var.get());

        UNDO.redo_n(1);
        app.update(false).assert_wait();
        assert_eq!(20, test_var.get());
    }

    #[derive(Debug)]
    struct PushAction {
        data: Arc<Mutex<Vec<u8>>>,
        item: u8,
    }
    impl fmt::Display for PushAction {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "push {}", self.item)
        }
    }
    impl UndoRedoItem for PushAction {}
    impl UndoAction for PushAction {
        fn undo(self: Box<Self>) -> Box<dyn RedoAction> {
            assert_eq!(self.data.lock().pop(), Some(self.item));
            self
        }
    }
    impl RedoAction for PushAction {
        fn redo(self: Box<Self>) -> Box<dyn UndoAction> {
            self.data.lock().push(self.item);
            self
        }
    }
}
