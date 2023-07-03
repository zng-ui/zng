//! Undo-redo app extension, service and commands.
//!

use std::{
    any::Any,
    fmt, mem,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
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
    shortcut,
    text::Txt,
    units::*,
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
            UNDO.undo(args.param::<u32>().copied().unwrap_or(1));
        } else if let Some(args) = UNDO_CMD.on_unhandled(update) {
            args.propagation().stop();
            UNDO.redo(args.param::<u32>().copied().unwrap_or(1));
        }
    }
}

/// Undo-redo service.
pub struct UNDO;
impl UNDO {
    /// Gets or sets the size limit of each undo stack in all scopes.
    pub fn max_undo(&self) -> ArcVar<u32> {
        UNDO_SV.read().max_undo.clone()
    }

    /// Gets or sets the time interval that groups actions together in all scopes.
    pub fn undo_interval(&self) -> ArcVar<Duration> {
        UNDO_SV.read().undo_interval.clone()
    }

    /// Gets if the undo service is enabled in the current context.
    ///
    /// If `false` calls to [`register`] are ignored.
    ///
    /// [`register`]: Self::register
    pub fn is_enabled(&self) -> bool {
        UNDO_SCOPE_CTX.get().enabled.load(Ordering::Relaxed)
    }

    /// Undo `count` times in the current scope.
    pub fn undo(&self, count: u32) {
        UNDO_SCOPE_CTX.get().undo(count);
    }

    /// Redo `count` times in the current scope.
    pub fn redo(&self, count: u32) {
        UNDO_SCOPE_CTX.get().redo(count);
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
                let _ = undo.undo();
            }
        }));

        let mut scope = Some(t_scope);
        UNDO_SCOPE_CTX.with_context(&mut scope, actions);

        let scope = scope.unwrap();
        let undo = mem::take(&mut *scope.undo.lock());

        UndoTransaction { undo }
    }
}

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
    undo: Vec<Box<dyn UndoAction>>,
}
impl UndoTransaction {
    /// If the transaction did not capture any undo action.
    pub fn is_empty(&self) -> bool {
        self.undo.is_empty()
    }

    /// Push all undo actions captured by the transaction into the current undo scope.
    pub fn commit(mut self) {
        let undo = mem::take(&mut self.undo);
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
            let _ = undo.undo();
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
struct UndoScope {
    id: Atomic<Option<WidgetId>>,
    undo: Mutex<Vec<Box<dyn UndoAction>>>,
    redo: Mutex<Vec<Box<dyn RedoAction>>>,
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
    fn register(&self, action: Box<dyn UndoAction>) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        self.undo.lock().push(action);
        self.redo.lock().clear();
    }

    fn undo(&self, mut count: u32) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        while count > 0 {
            count -= 1;

            if let Some(undo) = self.undo.lock().pop() {
                let redo = undo.undo();
                self.redo.lock().push(redo);
            }
        }
    }

    fn redo(&self, mut count: u32) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        while count > 0 {
            count -= 1;

            if let Some(redo) = self.redo.lock().pop() {
                let undo = redo.redo();
                self.undo.lock().push(undo);
            }
        }
    }

    fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
        if !enabled {
            self.undo.lock().clear();
            self.redo.lock().clear();
        }
    }

    fn id(&self) -> Option<WidgetId> {
        self.id.load(Ordering::Relaxed)
    }
}

struct UndoGroup {
    description: Txt,
    undo: Vec<Box<dyn UndoAction>>,
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
            redo.push(undo.undo());
        }
        Box::new(RedoGroup {
            description: self.description,
            redo,
        })
    }
}
struct RedoGroup {
    description: Txt,
    redo: Vec<Box<dyn RedoAction>>,
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
            undo.push(redo.redo());
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
    max_undo: ArcVar<u32>,
    undo_interval: ArcVar<Duration>,
}

impl Default for UndoService {
    fn default() -> Self {
        Self {
            max_undo: var(u32::MAX),
            undo_interval: var(100.ms()),
        }
    }
}

context_local! {
    static UNDO_SCOPE_CTX: UndoScope = UndoScope::default();
}
app_local! {
    static UNDO_SV: UndoService = UndoService::default();
}

mod properties {
    use std::sync::Arc;

    use super::*;
    use crate::{event::CommandHandle, widget_instance::*, *};

    /// Defines an undo/redo scope in the widget.
    ///
    /// If `enabled` is `false` undo/redo is disabled for the widget and descendants, if it is
    /// `true` all undo/redo actions
    #[property(WIDGET)]
    pub fn undo_scope(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
        let mut scope = None;
        let mut undo_cmd = CommandHandle::dummy();
        let mut redo_cmd = CommandHandle::dummy();
        let enabled = enabled.into_var();
        match_node(child, move |c, mut op| {
            match &mut op {
                UiNodeOp::Init => {
                    let id = WIDGET.id();
                    let s = UndoScope {
                        id: Atomic::new(Some(id)),
                        enabled: AtomicBool::new(enabled.get()),
                        ..Default::default()
                    };
                    scope = Some(Arc::new(s));

                    undo_cmd = UNDO_CMD.scoped(id).subscribe(false);
                    redo_cmd = REDO_CMD.scoped(id).subscribe(false);

                    WIDGET.sub_var(&enabled);
                }
                UiNodeOp::Deinit => {
                    UNDO_SCOPE_CTX.with_context(&mut scope, || c.deinit());
                    scope = None;
                    undo_cmd = CommandHandle::dummy();
                    redo_cmd = CommandHandle::dummy();
                    return;
                }
                UiNodeOp::Info { info } => {
                    info.flag_meta(&FOCUS_SCOPE_ID);
                }
                UiNodeOp::Event { update } => {
                    let id = WIDGET.id();
                    if let Some(args) = UNDO_CMD.scoped(id).on_unhandled(update) {
                        args.propagation().stop();
                        let scope = scope.as_ref().unwrap();
                        scope.undo(args.param::<u32>().copied().unwrap_or(1));
                    } else if let Some(args) = REDO_CMD.scoped(id).on_unhandled(update) {
                        args.propagation().stop();
                        let scope = scope.as_ref().unwrap();
                        scope.redo(args.param::<u32>().copied().unwrap_or(1));
                    }
                }
                UiNodeOp::Update { .. } => {
                    if let Some(enabled) = enabled.get_new() {
                        scope.as_ref().unwrap().set_enabled(enabled);
                    }
                }
                _ => {}
            }

            UNDO_SCOPE_CTX.with_context(&mut scope, || c.op(op));

            let scope = scope.as_ref().unwrap();
            undo_cmd.set_enabled(!scope.undo.lock().is_empty());
            redo_cmd.set_enabled(!scope.redo.lock().is_empty());
        })
    }
}
pub use properties::undo_scope;

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

        UNDO.undo(1);
        assert_eq!(&[1], &data.lock()[..]);
        UNDO.undo(1);
        assert_eq!(&[] as &[u8], &data.lock()[..]);

        UNDO.redo(1);
        assert_eq!(&[1], &data.lock()[..]);
        UNDO.redo(1);
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

        UNDO.undo(1);
        assert_eq!(&[1], &data.lock()[..]);
        UNDO.undo(1);
        assert_eq!(&[] as &[u8], &data.lock()[..]);

        UNDO.redo(1);
        assert_eq!(&[1], &data.lock()[..]);
        UNDO.redo(1);
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
        UNDO.undo(1);
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
        UNDO.undo(1);
        assert_eq!(&[1, 2], &data.lock()[..]);

        t.commit();

        UNDO.undo(1);
        assert_eq!(&[1], &data.lock()[..]);
        UNDO.undo(1);
        assert_eq!(&[] as &[u8], &data.lock()[..]);

        UNDO.redo(1);
        assert_eq!(&[1], &data.lock()[..]);
        UNDO.redo(1);
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
        UNDO.undo(1);
        assert_eq!(&[1, 2], &data.lock()[..]);

        t.commit_group("push 1, 2");

        UNDO.undo(1);
        assert_eq!(&[] as &[u8], &data.lock()[..]);

        UNDO.redo(1);
        assert_eq!(&[1, 2], &data.lock()[..]);
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
