//! Undo-redo app extension, service and commands.
//!

use std::{
    any::Any,
    mem,
    sync::{atomic::AtomicBool, Arc},
    time::{Duration, Instant},
};

use atomic::{Atomic, Ordering};
use parking_lot::Mutex;

use crate::{
    app::AppExtension,
    app_local, clmv, command,
    context::{StateMapRef, StaticStateId, WIDGET},
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
            if let Some(c) = args.param::<u32>() {
                UNDO.undo_select(*c);
            } else if let Some(i) = args.param::<Duration>() {
                UNDO.undo_select(*i);
            } else if let Some(t) = args.param::<Instant>() {
                UNDO.undo_select(*t);
            } else {
                UNDO.undo();
            }
        } else if let Some(args) = REDO_CMD.on_unhandled(update) {
            args.propagation().stop();
            if let Some(c) = args.param::<u32>() {
                UNDO.redo_select(*c);
            } else if let Some(i) = args.param::<Duration>() {
                UNDO.redo_select(*i);
            } else if let Some(t) = args.param::<Instant>() {
                UNDO.redo_select(*t);
            } else {
                UNDO.redo();
            }
        }
    }
}

context_var! {
    /// Contextual undo limit.
    ///
    /// Is [`UNDO.undo_limit`] by default.
    ///
    /// [`UNDO.undo_limit`]: UNDO::undo_limit
    pub static UNDO_LIMIT_VAR: u32 = UNDO.undo_limit();

    /// Contextual undo interval.
    ///
    /// Is [`UNDO.undo_interval`] by default.
    ///
    /// [`UNDO.undo_interval`]: UNDO::undo_interval
    pub static UNDO_INTERVAL_VAR: Duration = UNDO.undo_interval();
}

/// Undo-redo service.
pub struct UNDO;
impl UNDO {
    /// Gets or sets the maximum length of each undo stack of each scope.
    ///
    /// Is `u32::MAX` by default. If the limit is reached the oldest undo action is dropped without redo.
    ///
    /// Note that undo scopes get the max undo from [`UNDO_LIMIT_VAR`] in context, the context var is
    /// set to this var by default.
    pub fn undo_limit(&self) -> BoxedVar<u32> {
        UNDO_SV.read().undo_limit.clone()
    }

    /// Gets or sets the time interval that [`undo`] and [`redo`] cover each call.
    ///
    /// This value applies to all scopes and defines the max interval between actions
    /// that are undone in a single call.
    ///
    /// Is the [keyboard repeat interval] times 4 by default.
    ///
    /// Note that undo scopes get the interval from [`UNDO_INTERVAL_VAR`] in context, the context var is
    /// set to this var by default.
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
        UNDO_SCOPE_CTX.get().enabled.load(Ordering::Relaxed) && UNDO_SV.read().undo_limit.get() > 0
    }

    /// Undo a selection of actions.
    ///
    /// # Selectors
    ///
    /// These types can be used as selector:
    ///
    /// * `u32` - Count of actions to undo.
    /// * `Duration` - Interval between each action.
    /// * `Instant` - Inclusive timestamp to undo back to.
    pub fn undo_select(&self, selector: impl UndoSelector) {
        UNDO_SCOPE_CTX.get().undo_select(selector);
    }

    /// Redo a selection of actions.
    pub fn redo_select(&self, selector: impl UndoSelector) {
        UNDO_SCOPE_CTX.get().redo_select(selector);
    }

    /// Undo all actions within the [`undo_interval`].
    ///
    /// [`undo_interval`]: Self::undo_interval
    pub fn undo(&self) {
        self.undo_select(UNDO_INTERVAL_VAR.get());
    }

    /// Redo all actions within the [`undo_interval`].
    ///
    /// [`undo_interval`]: Self::undo_interval
    pub fn redo(&self) {
        self.redo_select(UNDO_INTERVAL_VAR.get());
    }

    /// Gets the parent ID that defines an undo scope, or `None` if undo is registered globally for
    /// the entire app.
    pub fn scope(&self) -> Option<WidgetId> {
        UNDO_SCOPE_CTX.get().id()
    }

    /// Register an already executed action for undo in the current scope.
    pub fn register(&self, info: impl UndoInfo, action: impl UndoAction) {
        UNDO_SCOPE_CTX.get().register(info.into_dyn(), Box::new(action))
    }

    /// Register an already executed action for undo in the current scope.
    ///
    /// The action is defined as a closure `op` that matches over [`UndoOp`] to implement undo and redo.
    pub fn register_op(&self, info: impl UndoInfo, op: impl FnMut(UndoOp) + Send + 'static) {
        self.register(info, UndoRedoOp { op: Box::new(op) })
    }

    /// Run the `action` and register the undo in the current scope.
    pub fn run(&self, info: impl UndoInfo, action: impl RedoAction) {
        UNDO_SCOPE_CTX.get().register(info.into_dyn(), Box::new(action).redo())
    }

    /// Run the `op` once with [`UndoOp::Redo`] and register it for undo in the current scope.
    pub fn run_op(&self, info: impl UndoInfo, op: impl FnMut(UndoOp) + Send + 'static) {
        self.run(info, UndoRedoOp { op: Box::new(op) })
    }

    /// Run `actions` as a [`transaction`] and commits as a group if any undo action is captured.
    ///
    /// [`transaction`]: Self::transaction
    pub fn group(&self, info: impl UndoInfo, actions: impl FnOnce()) -> bool {
        let t = self.transaction(actions);
        let any = !t.is_empty();
        if any {
            t.commit_group(info);
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
    pub fn try_group<O, E>(&self, info: impl UndoInfo, actions: impl FnOnce() -> Result<O, E>) -> Result<O, E> {
        let mut r = None;
        let t = self.transaction(|| r = Some(actions()));
        let r = r.unwrap();
        if !t.is_empty() {
            if r.is_ok() {
                t.commit_group(info);
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
    pub fn watch_var<T: VarValue>(&self, info: impl UndoInfo, var: impl Var<T>) -> VarHandle {
        if var.capabilities().is_always_read_only() {
            return VarHandle::dummy();
        }
        let var = var.actual_var();
        let wk_var = var.downgrade();

        let mut prev_value = Some(var.get());
        let info = info.into_dyn();

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
                    info.clone(),
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

    /// If the undo stack is not empty.
    pub fn can_undo(&self) -> bool {
        !UNDO_SCOPE_CTX.get().undo.lock().is_empty()
    }

    /// If the redo stack is not empty.
    pub fn can_redo(&self) -> bool {
        !UNDO_SCOPE_CTX.get().redo.lock().is_empty()
    }

    /// Clones the timestamp and info of all entries in the current undo stack.
    ///
    /// The latest undo action is the last entry in the list.
    pub fn undo_stack(&self) -> Vec<(Instant, Arc<dyn UndoInfo>)> {
        UNDO_SCOPE_CTX
            .get()
            .undo
            .lock()
            .iter()
            .map(|e| (e.timestamp, e.info.clone()))
            .collect()
    }

    /// Clones the timestamp and info of all entries in the current redo stack.
    ///
    /// The latest undone action is the last entry in the list. Note that the
    /// timestamp is marks the moment the original undo registered the action, so the
    /// newest timestamp is in the first entry.
    pub fn redo_stack(&self) -> Vec<(Instant, Arc<dyn UndoInfo>)> {
        UNDO_SCOPE_CTX
            .get()
            .redo
            .lock()
            .iter()
            .map(|e| (e.timestamp, e.info.clone()))
            .collect()
    }
}

/// Identifies that a var modify requested by undo/redo action.
///
/// See [`UNDO.watch_var`] for more details.
///
/// [`UNDO.watch_var`]: UNDO::watch_var
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UndoVarModifyTag;

/// Metadata info about an action registered for undo action.
pub trait UndoInfo: Send + Sync + Any {
    /// Short display description of the action that will be undone/redone.
    fn description(&self) -> Txt;

    /// Any extra metadata associated with the item. This can be a thumbnail of an image
    ///  edit action for example, or an icon.
    ///
    /// Is empty by default.
    fn meta(&self) -> StateMapRef<UNDO> {
        StateMapRef::empty()
    }

    /// Into `Arc<dyn UndoInfo>` without double wrapping.
    fn into_dyn(self) -> Arc<dyn UndoInfo>
    where
        Self: Sized,
    {
        Arc::new(self)
    }
}
impl UndoInfo for Txt {
    fn description(&self) -> Txt {
        self.clone()
    }
}
impl UndoInfo for BoxedVar<Txt> {
    fn description(&self) -> Txt {
        self.get()
    }
}
impl UndoInfo for &'static str {
    fn description(&self) -> Txt {
        Txt::from_static(self)
    }
}
impl UndoInfo for Arc<dyn UndoInfo> {
    fn description(&self) -> Txt {
        self.as_ref().description()
    }

    fn meta(&self) -> StateMapRef<UNDO> {
        self.as_ref().meta()
    }

    fn into_dyn(self) -> Arc<dyn UndoInfo>
    where
        Self: Sized,
    {
        self
    }
}
/// Represents a single undo action.
pub trait UndoAction: Send + Any {
    /// Undo action and returns a [`RedoAction`] that redoes it.
    fn undo(self: Box<Self>) -> Box<dyn RedoAction>;
}

/// Represents a single redo action.
pub trait RedoAction: Send + Any {
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
    pub fn commit_group(mut self, info: impl UndoInfo) {
        UNDO.register(
            info,
            UndoGroup {
                undo: mem::take(&mut self.undo),
            },
        )
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
    /// If the command parameter is a `u32`, `Duration` or `Instant` calls [`undo_select`], otherwise calls
    /// [`undo`].
    ///
    /// [`undo_select`]: UNDO::undo_select
    /// [`undo`]: UNDO::undo
    ///
    /// # Scope
    ///
    /// You can use [`CommandUndoExt::undo_scoped`] to get a command variable that is always scoped on the
    /// focused undo scope.
    pub static UNDO_CMD = {
        name: "Undo",
        shortcut: [shortcut!(CTRL+Z)],
    };

    /// Represents the **redo** action.
    ///
    /// # Param
    ///
    /// If the command parameter is a `u32`, `Duration` or `Instant` calls [`redo_select`], otherwise calls
    /// [`redo`].
    ///
    /// [`redo_select`]: UNDO::redo_select
    /// [`redo`]: UNDO::redo
    pub static REDO_CMD = {
        name: "Redo",
        shortcut: [shortcut!(CTRL+Y)],
    };

    /// Represents the **clear history** action.
    ///
    /// Implementers call [`clear`] in the undo scope.
    ///
    /// [`clear`]: UNDO::clear
    pub static CLEAR_HISTORY_CMD = {
        name: "Clear History",
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
        let id = WIDGET.id();
        *scope.id.get_mut() = Some(id);

        let scope = Arc::new(scope);
        let wk_scope = Arc::downgrade(&scope);

        UNDO_CMD.scoped(id).with_meta(|m| m.set(&WEAK_UNDO_SCOPE_ID, wk_scope.clone()));
        REDO_CMD.scoped(id).with_meta(|m| m.set(&WEAK_UNDO_SCOPE_ID, wk_scope));

        self.0 = Some(scope);
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
            UNDO_LIMIT_VAR.get() as usize
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

    fn register(&self, info: Arc<dyn UndoInfo>, action: Box<dyn UndoAction>) {
        self.with_enabled_undo_redo(|undo, redo| {
            undo.push(UndoEntry {
                timestamp: Instant::now(),
                info,
                action,
            });
            redo.clear();
        });
    }

    fn undo_select(&self, selector: impl UndoSelector) {
        let mut actions = vec![];

        self.with_enabled_undo_redo(|undo, _| {
            let mut select = selector.select(UndoOp::Undo);
            while let Some(entry) = undo.last() {
                if select.include(entry.timestamp) {
                    actions.push(undo.pop().unwrap());
                } else {
                    break;
                }
            }
        });

        for undo in actions {
            let redo = undo.action.undo();
            self.redo.lock().push(RedoEntry {
                timestamp: undo.timestamp,
                info: undo.info,
                action: redo,
            });
        }
    }

    fn redo_select(&self, selector: impl UndoSelector) {
        let mut actions = vec![];

        self.with_enabled_undo_redo(|_, redo| {
            let mut select = selector.select(UndoOp::Redo);
            while let Some(entry) = redo.last() {
                if select.include(entry.timestamp) {
                    actions.push(redo.pop().unwrap());
                } else {
                    break;
                }
            }
        });

        for redo in actions {
            let undo = redo.action.redo();
            self.undo.lock().push(UndoEntry {
                timestamp: redo.timestamp,
                info: redo.info,
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
    info: Arc<dyn UndoInfo>,
    action: Box<dyn UndoAction>,
}

struct RedoEntry {
    pub timestamp: Instant,
    info: Arc<dyn UndoInfo>,
    pub action: Box<dyn RedoAction>,
}

struct UndoGroup {
    undo: Vec<UndoEntry>,
}
impl UndoAction for UndoGroup {
    fn undo(self: Box<Self>) -> Box<dyn RedoAction> {
        let mut redo = Vec::with_capacity(self.undo.len());
        for undo in self.undo.into_iter().rev() {
            redo.push(RedoEntry {
                timestamp: undo.timestamp,
                info: undo.info,
                action: undo.action.undo(),
            });
        }
        Box::new(RedoGroup { redo })
    }
}
struct RedoGroup {
    redo: Vec<RedoEntry>,
}
impl RedoAction for RedoGroup {
    fn redo(self: Box<Self>) -> Box<dyn UndoAction> {
        let mut undo = Vec::with_capacity(self.redo.len());
        for redo in self.redo.into_iter().rev() {
            undo.push(UndoEntry {
                timestamp: redo.timestamp,
                info: redo.info,
                action: redo.action.redo(),
            });
        }
        Box::new(UndoGroup { undo })
    }
}

struct UndoRedoOp {
    op: Box<dyn FnMut(UndoOp) + Send>,
}
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
    undo_limit: BoxedVar<u32>,
    undo_interval: BoxedVar<Duration>,
}

impl Default for UndoService {
    fn default() -> Self {
        Self {
            undo_limit: var(u32::MAX).boxed(),
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

    /// Latest undo stack for the given scope, same as calling [`UNDO::undo_stack`] inside the scope.
    fn undo_stack(self) -> Vec<(Instant, Arc<dyn UndoInfo>)>;
    /// Latest undo stack for the given scope, same as calling [`UNDO::redo_stack`] inside the scope.
    fn redo_stack(self) -> Vec<(Instant, Arc<dyn UndoInfo>)>;
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

    fn undo_stack(self) -> Vec<(Instant, Arc<dyn UndoInfo>)> {
        let scope = self.with_meta(|m| m.get(&WEAK_UNDO_SCOPE_ID));
        if let Some(scope) = scope {
            if let Some(scope) = scope.upgrade() {
                return scope.undo.lock().iter().map(|e| (e.timestamp, e.info.clone())).collect();
            }
        }

        if let CommandScope::App = self.scope() {
            return UNDO_SCOPE_CTX.with_default(|| UNDO.undo_stack());
        }

        vec![]
    }

    fn redo_stack(self) -> Vec<(Instant, Arc<dyn UndoInfo>)> {
        let scope = self.with_meta(|m| m.get(&WEAK_UNDO_SCOPE_ID));
        if let Some(scope) = scope {
            if let Some(scope) = scope.upgrade() {
                return scope.redo.lock().iter().map(|e| (e.timestamp, e.info.clone())).collect();
            }
        }

        if let CommandScope::App = self.scope() {
            return UNDO_SCOPE_CTX.with_default(|| UNDO.redo_stack());
        }

        vec![]
    }
}

static WEAK_UNDO_SCOPE_ID: StaticStateId<std::sync::Weak<UndoScope>> = StaticStateId::new_unique();

/// Represents a type that can select actions for undo or redo once.
///
/// This API is sealed, only core crate can implement it.
///
/// See [`UNDO::undo_select`] for more details.
pub trait UndoSelector: crate::private::Sealed {
    /// Selection collector.
    type Select: UndoSelect;

    /// Start selecting action for the `op`.
    fn select(self, op: UndoOp) -> Self::Select;
}

/// Selects actions to undo or redo.
pub trait UndoSelect {
    /// Called for each undo or redo action from the last item in the stack and back.
    ///
    /// The `timestamp` is the moment the item was pushed in the undo stack, if this
    /// function is called for [`UndoOp::Redo`] it will not be more recent than the next action.
    fn include(&mut self, timestamp: Instant) -> bool;
}
impl crate::private::Sealed for u32 {}
impl UndoSelector for u32 {
    type Select = u32;

    fn select(self, op: UndoOp) -> Self::Select {
        let _ = op;
        self
    }
}
impl UndoSelect for u32 {
    fn include(&mut self, _: Instant) -> bool {
        let i = *self > 0;
        if i {
            *self -= 1;
        }
        i
    }
}
impl crate::private::Sealed for Duration {}
impl UndoSelector for Duration {
    type Select = UndoSelectInterval;

    fn select(self, op: UndoOp) -> Self::Select {
        UndoSelectInterval {
            prev: None,
            interval: self,
            op,
        }
    }
}
#[doc(hidden)]
pub struct UndoSelectInterval {
    prev: Option<Instant>,
    interval: Duration,
    op: UndoOp,
}
impl UndoSelect for UndoSelectInterval {
    fn include(&mut self, timestamp: Instant) -> bool {
        if let Some(prev) = &mut self.prev {
            let (older, newer) = match self.op {
                UndoOp::Undo => (timestamp, *prev),
                UndoOp::Redo => (*prev, timestamp),
            };
            if newer.saturating_duration_since(older) <= self.interval {
                *prev = timestamp;
                true
            } else {
                false
            }
        } else {
            self.prev = Some(timestamp);
            true
        }
    }
}
impl crate::private::Sealed for Instant {}
impl UndoSelector for Instant {
    type Select = UndoSelectLtEq;

    fn select(self, op: UndoOp) -> Self::Select {
        UndoSelectLtEq { instant: self, op }
    }
}
#[doc(hidden)]
pub struct UndoSelectLtEq {
    instant: Instant,
    op: UndoOp,
}
impl UndoSelect for UndoSelectLtEq {
    fn include(&mut self, timestamp: Instant) -> bool {
        match self.op {
            UndoOp::Undo => timestamp <= self.instant,
            UndoOp::Redo => timestamp >= self.instant,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::App;
    use crate::units::*;

    use super::*;

    #[test]
    fn register() {
        let _a = App::minimal();
        let data = Arc::new(Mutex::new(vec![1, 2]));

        UNDO.register(
            "push",
            PushAction {
                data: data.clone(),
                item: 1,
            },
        );
        UNDO.register(
            "push",
            PushAction {
                data: data.clone(),
                item: 2,
            },
        );
        assert_eq!(&[1, 2], &data.lock()[..]);

        UNDO.undo_select(1);
        assert_eq!(&[1], &data.lock()[..]);
        UNDO.undo_select(1);
        assert_eq!(&[] as &[u8], &data.lock()[..]);

        UNDO.redo_select(1);
        assert_eq!(&[1], &data.lock()[..]);
        UNDO.redo_select(1);
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

        UNDO.undo_select(1);
        assert_eq!(&[1], &data.lock()[..]);
        UNDO.undo_select(1);
        assert_eq!(&[] as &[u8], &data.lock()[..]);

        UNDO.redo_select(1);
        assert_eq!(&[1], &data.lock()[..]);
        UNDO.redo_select(1);
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
        UNDO.undo_select(1);
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
        UNDO.undo_select(1);
        assert_eq!(&[1, 2], &data.lock()[..]);

        t.commit();

        UNDO.undo_select(1);
        assert_eq!(&[1], &data.lock()[..]);
        UNDO.undo_select(1);
        assert_eq!(&[] as &[u8], &data.lock()[..]);

        UNDO.redo_select(1);
        assert_eq!(&[1], &data.lock()[..]);
        UNDO.redo_select(1);
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
        UNDO.undo_select(1);
        assert_eq!(&[1, 2], &data.lock()[..]);

        t.commit_group("push 1, 2");

        UNDO.undo_select(1);
        assert_eq!(&[] as &[u8], &data.lock()[..]);

        UNDO.redo_select(1);
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

        UNDO.undo_select(Duration::ZERO);
        assert_eq!(&[1], &data.lock()[..]);
        UNDO.undo_select(Duration::ZERO);
        assert_eq!(&[] as &[u8], &data.lock()[..]);

        UNDO.redo_select(Duration::ZERO);
        assert_eq!(&[1], &data.lock()[..]);
        UNDO.redo_select(Duration::ZERO);
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

        UNDO.undo_select(t);
        assert_eq!(&[] as &[u8], &data.lock()[..]);

        UNDO.redo_select(t);
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

        UNDO.undo_select(1);
        app.update(false).assert_wait();
        assert_eq!(10, test_var.get());

        UNDO.undo_select(1);
        app.update(false).assert_wait();
        assert_eq!(0, test_var.get());

        UNDO.redo_select(1);
        app.update(false).assert_wait();
        assert_eq!(10, test_var.get());

        UNDO.redo_select(1);
        app.update(false).assert_wait();
        assert_eq!(20, test_var.get());
    }

    struct PushAction {
        data: Arc<Mutex<Vec<u8>>>,
        item: u8,
    }
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
