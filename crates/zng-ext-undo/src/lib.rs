#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! Undo-redo app extension, service and commands.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]
#![recursion_limit = "256"]
// suppress nag about very simple boxed closure signatures.
#![expect(clippy::type_complexity)]

use std::{
    any::Any,
    fmt, mem,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use atomic::Atomic;
use parking_lot::Mutex;
use zng_app::{
    APP, AppExtension, DInstant, INSTANT,
    event::{AnyEventArgs, Command, CommandNameExt, CommandScope, command},
    shortcut::{CommandShortcutExt, shortcut},
    update::EventUpdate,
    widget::{
        WIDGET, WidgetId,
        info::{WidgetInfo, WidgetInfoBuilder},
    },
};
use zng_app_context::{RunOnDrop, app_local, context_local};
use zng_clone_move::clmv;
use zng_ext_input::{focus::cmd::CommandFocusExt, keyboard::KEYBOARD};
use zng_state_map::{StateId, StateMapRef, static_id};
use zng_txt::Txt;
use zng_var::{Var, VarHandle, VarValue, context_var, var};
use zng_wgt::{CommandIconExt as _, ICONS, wgt_fn};

mod private {
    // https://rust-lang.github.io/api-guidelines/future-proofing.html#sealed-traits-protect-against-downstream-implementations-c-sealed
    pub trait Sealed {}
}

/// Undo-redo app extension.
///
/// # Services
///
/// Services provided by this extension.
///
/// * [`UNDO`]
#[derive(Default)]
pub struct UndoManager {}

impl AppExtension for UndoManager {
    fn event(&mut self, update: &mut EventUpdate) {
        // app scope handler
        if let Some(args) = UNDO_CMD.on_unhandled(update) {
            args.propagation().stop();
            if let Some(c) = args.param::<u32>() {
                UNDO.undo_select(*c);
            } else if let Some(i) = args.param::<Duration>() {
                UNDO.undo_select(*i);
            } else if let Some(t) = args.param::<DInstant>() {
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
            } else if let Some(t) = args.param::<DInstant>() {
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
///
/// # Provider
///
/// This service is provided by the [`UndoManager`] extension, it will panic if used in an app not extended.
pub struct UNDO;
impl UNDO {
    /// Gets or sets the maximum length of each undo stack of each scope.
    ///
    /// Is `u32::MAX` by default. If the limit is reached the oldest undo action is dropped without redo.
    ///
    /// Note that undo scopes get the max undo from [`UNDO_LIMIT_VAR`] in context, the context var is
    /// set to this var by default.
    pub fn undo_limit(&self) -> Var<u32> {
        UNDO_SV.read().undo_limit.clone()
    }

    /// Gets or sets the time interval that [`undo`] and [`redo`] cover each call.
    ///
    /// This value applies to all scopes and defines the max interval between actions
    /// that are undone in a single call.
    ///
    /// Is the [keyboard repeat start delay + interval] by default.
    ///
    /// Note that undo scopes get the interval from [`UNDO_INTERVAL_VAR`] in context, the context var is
    /// set to this var by default.
    ///
    /// [`undo`]: Self::undo
    /// [`redo`]: Self::redo
    /// [keyboard repeat start delay + interval]: zng_ext_input::keyboard::KEYBOARD::repeat_config
    pub fn undo_interval(&self) -> Var<Duration> {
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
    /// * `DInstant` - Inclusive timestamp to undo back to.
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
    pub fn register(&self, action: impl UndoAction) {
        UNDO_SCOPE_CTX.get().register(Box::new(action))
    }

    /// Register an already executed action for undo in the current scope.
    ///
    /// The action is defined as a closure `op` that matches over [`UndoOp`] to implement undo and redo.
    pub fn register_op(&self, info: impl UndoInfo, op: impl FnMut(UndoOp) + Send + 'static) {
        self.register(UndoRedoOp {
            info: info.into_dyn(),
            op: Box::new(op),
        })
    }

    /// Register an already executed action for undo in the current scope.
    ///
    /// The action is defined as a closure `op` that matches over [`UndoFullOp`] referencing `data` to implement undo and redo.
    pub fn register_full_op<D>(&self, data: D, mut op: impl FnMut(&mut D, UndoFullOp) + Send + 'static)
    where
        D: Any + Send + 'static,
    {
        self.register(UndoRedoFullOp {
            data: Box::new(data),
            op: Box::new(move |d, o| {
                op(d.downcast_mut::<D>().unwrap(), o);
            }),
        })
    }

    /// Run the `action` and register the undo in the current scope.
    pub fn run(&self, action: impl RedoAction) {
        UNDO_SCOPE_CTX.get().register(Box::new(action).redo())
    }

    /// Run the `op` once with [`UndoOp::Redo`] and register it for undo in the current scope.
    pub fn run_op(&self, info: impl UndoInfo, op: impl FnMut(UndoOp) + Send + 'static) {
        self.run(UndoRedoOp {
            info: info.into_dyn(),
            op: Box::new(op),
        })
    }

    /// Run the `op` once with `UndoFullOp::Init { .. }` and `UndoFullOp::Op(UndoOp::Redo)` and register it for undo in the current scope.
    pub fn run_full_op<D>(&self, mut data: D, mut op: impl FnMut(&mut D, UndoFullOp) + Send + 'static)
    where
        D: Any + Send + 'static,
    {
        let mut redo = true;
        op(&mut data, UndoFullOp::Init { redo: &mut redo });

        if redo {
            self.run(UndoRedoFullOp {
                data: Box::new(data),
                op: Box::new(move |d, o| {
                    op(d.downcast_mut::<D>().unwrap(), o);
                }),
            })
        }
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

        UNDO_SCOPE_CTX.with_context(&mut Some(Arc::new(scope)), f)
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
    pub fn watch_var<T: VarValue>(&self, info: impl UndoInfo, var: Var<T>) -> VarHandle {
        if var.capabilities().is_always_read_only() {
            return VarHandle::dummy();
        }
        let var = var.current_context();
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
                        match op {
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
    pub fn undo_stack(&self) -> UndoStackInfo {
        UndoStackInfo::undo(&UNDO_SCOPE_CTX.get(), UNDO_INTERVAL_VAR.get())
    }

    /// Clones the timestamp and info of all entries in the current redo stack.
    ///
    /// The latest undone action is the last entry in the list. Note that the
    /// timestamp marks the moment the original undo registered the action, so the
    /// newest timestamp is in the first entry.
    pub fn redo_stack(&self) -> UndoStackInfo {
        UndoStackInfo::redo(&UNDO_SCOPE_CTX.get(), UNDO_INTERVAL_VAR.get())
    }
}

/// Snapshot of the undo or redo stacks in an [`UNDO`] scope.
#[derive(Clone)]
pub struct UndoStackInfo {
    /// Clones the timestamp and info of all entries in the current undo stack.
    ///
    /// In an undo list the latest undo action (first to undo) is the last entry in the list and has the latest timestamp.
    ///
    /// In an redo list the latest undone action is the last entry (first to redo). Note that the
    /// timestamp marks the moment the original undo registered the action, so the
    /// newest timestamp is in the first entry for redo lists.
    pub stack: Vec<(DInstant, Arc<dyn UndoInfo>)>,

    /// Grouping interval.
    pub undo_interval: Duration,
}
impl UndoStackInfo {
    fn undo(ctx: &UndoScope, undo_interval: Duration) -> Self {
        Self {
            stack: ctx.undo.lock().iter_mut().map(|e| (e.timestamp, e.action.info())).collect(),
            undo_interval,
        }
    }
    fn redo(ctx: &UndoScope, undo_interval: Duration) -> Self {
        Self {
            stack: ctx.redo.lock().iter_mut().map(|e| (e.timestamp, e.action.info())).collect(),
            undo_interval,
        }
    }

    /// Iterate over the `stack`, grouped by `undo_interval`.
    pub fn iter_groups(&self) -> impl DoubleEndedIterator<Item = &[(DInstant, Arc<dyn UndoInfo>)]> {
        struct Iter<'a> {
            stack: &'a [(DInstant, Arc<dyn UndoInfo>)],
            interval: Duration,
            ts_inverted: bool,
        }
        impl<'a> Iterator for Iter<'a> {
            type Item = &'a [(DInstant, Arc<dyn UndoInfo>)];

            fn next(&mut self) -> Option<Self::Item> {
                if self.stack.is_empty() {
                    None
                } else {
                    let mut older = self.stack[0].0;

                    let mut r = self.stack;

                    if let Some(i) = self.stack.iter().position(|(newer, _)| {
                        let (a, b) = if self.ts_inverted { (older, *newer) } else { (*newer, older) };
                        let break_ = a.saturating_duration_since(b) > self.interval;
                        older = *newer;
                        break_
                    }) {
                        r = &self.stack[..i];
                        self.stack = &self.stack[i..];
                    } else {
                        self.stack = &[];
                    }

                    Some(r)
                }
            }
        }
        impl DoubleEndedIterator for Iter<'_> {
            fn next_back(&mut self) -> Option<Self::Item> {
                if self.stack.is_empty() {
                    None
                } else {
                    let mut newer = self.stack[self.stack.len() - 1].0;

                    let mut r = self.stack;

                    if let Some(i) = self.stack.iter().rposition(|(older, _)| {
                        let (a, b) = if self.ts_inverted { (*older, newer) } else { (newer, *older) };
                        let break_ = a.saturating_duration_since(b) > self.interval;
                        newer = *older;
                        break_
                    }) {
                        let i = i + 1;
                        r = &self.stack[i..];
                        self.stack = &self.stack[..i];
                    } else {
                        self.stack = &[];
                    }

                    Some(r)
                }
            }
        }
        Iter {
            stack: &self.stack,
            interval: self.undo_interval,
            ts_inverted: self.stack.len() > 1 && self.stack[0].0 > self.stack[self.stack.len() - 1].0,
        }
    }
}

/// Identifies var modify requests by undo/redo action.
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
    /// edit action for example, or an icon.
    ///
    /// Is empty by default.
    fn meta(&self) -> StateMapRef<'_, UNDO> {
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
impl UndoInfo for Var<Txt> {
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

    fn meta(&self) -> StateMapRef<'_, UNDO> {
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
    /// Gets display info about the action that registered this undo.
    fn info(&mut self) -> Arc<dyn UndoInfo>;

    /// Undo action and returns a [`RedoAction`] that redoes it.
    fn undo(self: Box<Self>) -> Box<dyn RedoAction>;

    /// Access `dyn Any` methods.
    fn as_any(&mut self) -> &mut dyn Any;

    /// Try merge the `next` action with the previous `self`.
    ///
    /// This is called when `self` is the latest registered action and `next` is registered.
    ///
    /// This can be used to optimize high-volume actions, but note that [`UNDO.undo`] will undo all actions
    /// within the [`UNDO.undo_interval`] of the previous, even if not merged, and merged actions always show as one action
    /// for [`UNDO.undo_select`].
    ///
    /// [`UNDO.undo_interval`]: UNDO::undo_interval
    /// [`UNDO.undo_select`]: UNDO::undo_select
    /// [`UNDO.undo`]: UNDO::undo
    fn merge(self: Box<Self>, args: UndoActionMergeArgs) -> Result<Box<dyn UndoAction>, (Box<dyn UndoAction>, Box<dyn UndoAction>)>;
}

/// Arguments for [`UndoAction::merge`].
pub struct UndoActionMergeArgs {
    /// The action that was registered after the one receiving this arguments.
    pub next: Box<dyn UndoAction>,

    /// Timestamp of the previous action registered.
    pub prev_timestamp: DInstant,

    /// If the `prev_timestamp` is within the [`UNDO.undo_interval`]. Undo actions
    /// can choose to ignore this and merge anyway.
    ///
    /// [`UNDO.undo_interval`]: UNDO::undo_interval
    pub within_undo_interval: bool,
}

/// Represents a single redo action.
pub trait RedoAction: Send + Any {
    /// Gets display info about the action that will be redone.
    fn info(&mut self) -> Arc<dyn UndoInfo>;

    /// Redo action and returns a [`UndoAction`] that undoes it.
    fn redo(self: Box<Self>) -> Box<dyn UndoAction>;
}

/// Represents an undo/redo action.
///
/// This can be used to implement undo and redo in a single closure. See [`UNDO.register_op`] and
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
impl UndoOp {
    /// Gets the command that represents the OP.
    pub fn cmd(self) -> Command {
        match self {
            UndoOp::Undo => UNDO_CMD,
            UndoOp::Redo => REDO_CMD,
        }
    }
}

/// Represents a full undo/redo action.
///
/// This can be used to implement undo and redo in a single closure. See [`UNDO.register_full_op`] and
/// [`UNDO.run_full_op`] for more details.
///
/// [`UNDO.register_full_op`]: UNDO::register_full_op
/// [`UNDO.run_full_op`]: UNDO::run_full_op
pub enum UndoFullOp<'r> {
    /// Initialize data in the execution context.
    ///
    /// This is called once before the initial `Op(UndoOp::Redo)` call, it
    /// can be used to skip registering no-ops.
    Init {
        /// If the op must actually be executed.
        ///
        /// This is `true` by default, if set to `false` the OP will be dropped without ever executing and
        /// will not be registered for undo.
        redo: &'r mut bool,
    },

    /// Normal undo/redo.
    Op(UndoOp),
    /// Collect display info.
    Info {
        /// Set this to respond.
        ///
        /// If not set the info will be some generic "action" text.
        info: &'r mut Option<Arc<dyn UndoInfo>>,
    },
    /// Try merge the `next_data` onto self data (at the undone state).
    Merge {
        /// Closure data for the next undo action.
        ///
        /// The data can be from any full undo closure action, only merge if the data
        /// indicates that it comes from actions that can be covered by the `self` closure.
        next_data: &'r mut dyn Any,

        /// Timestamp of the previous action registered.
        prev_timestamp: DInstant,

        /// If the `prev_timestamp` is within the [`UNDO.undo_interval`]. Undo actions
        /// can choose to ignore this and merge anyway.
        ///
        /// [`UNDO.undo_interval`]: UNDO::undo_interval
        within_undo_interval: bool,

        /// Set this to `true` if the next action can be dropped because the `self` closure
        /// now also implements it.
        merged: &'r mut bool,
    },
}
impl fmt::Debug for UndoFullOp<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Init { .. } => f.debug_struct("Init").finish_non_exhaustive(),
            Self::Op(arg0) => f.debug_tuple("Op").field(arg0).finish(),
            Self::Info { .. } => f.debug_struct("Info").finish_non_exhaustive(),
            Self::Merge { .. } => f.debug_struct("Merge").finish_non_exhaustive(),
        }
    }
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
        let now = INSTANT.now();
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
        UNDO.register(UndoGroup {
            info: info.into_dyn(),
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
    /// If the command parameter is a `u32`, `Duration` or `DInstant` calls [`undo_select`], otherwise calls
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
        l10n!: true,
        name: "Undo",
        shortcut: [shortcut!(CTRL + 'Z')],
        icon: wgt_fn!(|_| ICONS.get("undo")),
    };

    /// Represents the **redo** action.
    ///
    /// # Param
    ///
    /// If the command parameter is a `u32`, `Duration` or `DInstant` calls [`redo_select`], otherwise calls
    /// [`redo`].
    ///
    /// [`redo_select`]: UNDO::redo_select
    /// [`redo`]: UNDO::redo
    pub static REDO_CMD = {
        l10n!: true,
        name: "Redo",
        shortcut: [shortcut!(CTRL + 'Y')],
        icon: wgt_fn!(|_| ICONS.get("redo")),
    };

    /// Represents the **clear history** action.
    ///
    /// Implementers call [`clear`] in the undo scope.
    ///
    /// [`clear`]: UNDO::clear
    pub static CLEAR_HISTORY_CMD = {
        l10n!: true,
        name: "Clear History",
    };
}

/// Represents a widget undo scope.
///
/// See [`UNDO.with_scope`] for more details.
///
/// [`UNDO.with_scope`]: UNDO::with_scope
pub struct WidgetUndoScope(Option<Arc<UndoScope>>);
impl WidgetUndoScope {
    /// New, not inited in a widget.
    pub const fn new() -> Self {
        Self(None)
    }

    /// if the scope is already inited in a widget.
    pub fn is_inited(&self) -> bool {
        self.0.is_some()
    }

    /// Init the scope in the [`WIDGET`].
    ///
    /// [`WIDGET`]: zng_app::widget::WIDGET
    pub fn init(&mut self) {
        let mut scope = UndoScope::default();
        let id = WIDGET.id();
        *scope.id.get_mut() = Some(id);

        let scope = Arc::new(scope);
        let wk_scope = Arc::downgrade(&scope);
        let interval = UNDO_INTERVAL_VAR.current_context();

        UNDO_CMD
            .scoped(id)
            .with_meta(|m| m.set(*WEAK_UNDO_SCOPE_ID, (wk_scope.clone(), interval.clone())));
        REDO_CMD.scoped(id).with_meta(|m| m.set(*WEAK_UNDO_SCOPE_ID, (wk_scope, interval)));

        self.0 = Some(scope);
    }

    /// Sets the [`WIDGET`] info.
    ///
    /// [`WIDGET`]: zng_app::widget::WIDGET
    pub fn info(&mut self, info: &mut WidgetInfoBuilder) {
        info.flag_meta(*FOCUS_SCOPE_ID);
    }

    /// Deinit the scope in the [`WIDGET`].
    ///
    /// This clears the undo/redo stack of the scope.
    ///
    /// [`WIDGET`]: zng_app::widget::WIDGET
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
impl Default for WidgetUndoScope {
    fn default() -> Self {
        Self::new()
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

    fn register(&self, action: Box<dyn UndoAction>) {
        self.with_enabled_undo_redo(|undo, redo| {
            let now = INSTANT.now();
            if let Some(prev) = undo.pop() {
                match prev.action.merge(UndoActionMergeArgs {
                    next: action,
                    prev_timestamp: prev.timestamp,
                    within_undo_interval: now.duration_since(prev.timestamp) <= UNDO_SV.read().undo_interval.get(),
                }) {
                    Ok(merged) => undo.push(UndoEntry {
                        timestamp: now,
                        action: merged,
                    }),
                    Err((p, action)) => {
                        undo.push(UndoEntry {
                            timestamp: prev.timestamp,
                            action: p,
                        });
                        undo.push(UndoEntry { timestamp: now, action });
                    }
                }
            } else {
                undo.push(UndoEntry { timestamp: now, action });
            }
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
                action: undo,
            });
        }
    }

    fn id(&self) -> Option<WidgetId> {
        self.id.load(Ordering::Relaxed)
    }
}

struct UndoEntry {
    timestamp: DInstant,
    action: Box<dyn UndoAction>,
}

struct RedoEntry {
    pub timestamp: DInstant,
    pub action: Box<dyn RedoAction>,
}

struct UndoGroup {
    info: Arc<dyn UndoInfo>,
    undo: Vec<UndoEntry>,
}
impl UndoAction for UndoGroup {
    fn undo(self: Box<Self>) -> Box<dyn RedoAction> {
        let mut redo = Vec::with_capacity(self.undo.len());
        for undo in self.undo.into_iter().rev() {
            redo.push(RedoEntry {
                timestamp: undo.timestamp,
                action: undo.action.undo(),
            });
        }
        Box::new(RedoGroup { info: self.info, redo })
    }

    fn info(&mut self) -> Arc<dyn UndoInfo> {
        self.info.clone()
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn merge(self: Box<Self>, args: UndoActionMergeArgs) -> Result<Box<dyn UndoAction>, (Box<dyn UndoAction>, Box<dyn UndoAction>)> {
        Err((self, args.next))
    }
}
struct RedoGroup {
    info: Arc<dyn UndoInfo>,
    redo: Vec<RedoEntry>,
}
impl RedoAction for RedoGroup {
    fn redo(self: Box<Self>) -> Box<dyn UndoAction> {
        let mut undo = Vec::with_capacity(self.redo.len());
        for redo in self.redo.into_iter().rev() {
            undo.push(UndoEntry {
                timestamp: redo.timestamp,
                action: redo.action.redo(),
            });
        }
        Box::new(UndoGroup { info: self.info, undo })
    }

    fn info(&mut self) -> Arc<dyn UndoInfo> {
        self.info.clone()
    }
}

struct UndoRedoOp {
    info: Arc<dyn UndoInfo>,
    op: Box<dyn FnMut(UndoOp) + Send>,
}
impl UndoAction for UndoRedoOp {
    fn undo(mut self: Box<Self>) -> Box<dyn RedoAction> {
        (self.op)(UndoOp::Undo);
        self
    }

    fn info(&mut self) -> Arc<dyn UndoInfo> {
        self.info.clone()
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn merge(self: Box<Self>, args: UndoActionMergeArgs) -> Result<Box<dyn UndoAction>, (Box<dyn UndoAction>, Box<dyn UndoAction>)> {
        Err((self, args.next))
    }
}
impl RedoAction for UndoRedoOp {
    fn redo(mut self: Box<Self>) -> Box<dyn UndoAction> {
        (self.op)(UndoOp::Redo);
        self
    }

    fn info(&mut self) -> Arc<dyn UndoInfo> {
        self.info.clone()
    }
}

struct UndoRedoFullOp {
    data: Box<dyn Any + Send>,
    op: Box<dyn FnMut(&mut dyn Any, UndoFullOp) + Send>,
}
impl UndoAction for UndoRedoFullOp {
    fn info(&mut self) -> Arc<dyn UndoInfo> {
        let mut info = None;
        (self.op)(&mut self.data, UndoFullOp::Info { info: &mut info });
        info.unwrap_or_else(|| Arc::new("action"))
    }

    fn undo(mut self: Box<Self>) -> Box<dyn RedoAction> {
        (self.op)(&mut self.data, UndoFullOp::Op(UndoOp::Undo));
        self
    }

    fn merge(mut self: Box<Self>, mut args: UndoActionMergeArgs) -> Result<Box<dyn UndoAction>, (Box<dyn UndoAction>, Box<dyn UndoAction>)>
    where
        Self: Sized,
    {
        if let Some(u) = args.next.as_any().downcast_mut::<Self>() {
            let mut merged = false;
            (self.op)(
                &mut self.data,
                UndoFullOp::Merge {
                    next_data: &mut u.data,
                    prev_timestamp: args.prev_timestamp,
                    within_undo_interval: args.within_undo_interval,
                    merged: &mut merged,
                },
            );
            if merged { Ok(self) } else { Err((self, args.next)) }
        } else {
            Err((self, args.next))
        }
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}
impl RedoAction for UndoRedoFullOp {
    fn info(&mut self) -> Arc<dyn UndoInfo> {
        let mut info = None;
        (self.op)(&mut self.data, UndoFullOp::Info { info: &mut info });
        info.unwrap_or_else(|| Arc::new("action"))
    }

    fn redo(mut self: Box<Self>) -> Box<dyn UndoAction> {
        (self.op)(&mut self.data, UndoFullOp::Op(UndoOp::Redo));
        self
    }
}

struct UndoService {
    undo_limit: Var<u32>,
    undo_interval: Var<Duration>,
}

impl Default for UndoService {
    fn default() -> Self {
        Self {
            undo_limit: var(u32::MAX),
            undo_interval: KEYBOARD.repeat_config().map(|c| c.start_delay + c.interval).cow(),
        }
    }
}

context_local! {
    static UNDO_SCOPE_CTX: UndoScope = UndoScope::default();
}
app_local! {
    static UNDO_SV: UndoService = {
        APP.extensions().require::<UndoManager>();
        UndoService::default()
    };
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
        self.meta().flagged(*FOCUS_SCOPE_ID)
    }

    fn undo_scope(&self) -> Option<WidgetInfo> {
        self.ancestors().find(WidgetInfoUndoExt::is_undo_scope)
    }
}

static_id! {
    static ref FOCUS_SCOPE_ID: StateId<()>;
}

/// Undo extension methods for commands.
pub trait CommandUndoExt {
    /// Gets the command scoped in the undo scope widget that is or contains the focused widget, or
    /// scoped on the app if there is no focused undo scope.
    fn undo_scoped(self) -> Var<Command>;

    /// Latest undo stack for the given scope, same as calling [`UNDO::undo_stack`] inside the scope.
    fn undo_stack(self) -> UndoStackInfo;
    /// Latest undo stack for the given scope, same as calling [`UNDO::redo_stack`] inside the scope.
    fn redo_stack(self) -> UndoStackInfo;
}
impl CommandUndoExt for Command {
    fn undo_scoped(self) -> Var<Command> {
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

    fn undo_stack(self) -> UndoStackInfo {
        let scope = self.with_meta(|m| m.get(*WEAK_UNDO_SCOPE_ID));
        if let Some(scope) = scope
            && let Some(s) = scope.0.upgrade()
        {
            return UndoStackInfo::undo(&s, scope.1.get());
        }

        if let CommandScope::App = self.scope() {
            let mut r = UNDO_SCOPE_CTX.with_default(|| UNDO.undo_stack());
            r.undo_interval = UNDO.undo_interval().get();
            return r;
        }

        UndoStackInfo {
            stack: vec![],
            undo_interval: Duration::ZERO,
        }
    }

    fn redo_stack(self) -> UndoStackInfo {
        let scope = self.with_meta(|m| m.get(*WEAK_UNDO_SCOPE_ID));
        if let Some(scope) = scope
            && let Some(s) = scope.0.upgrade()
        {
            return UndoStackInfo::redo(&s, scope.1.get());
        }

        if let CommandScope::App = self.scope() {
            let mut r = UNDO_SCOPE_CTX.with_default(|| UNDO.redo_stack());
            r.undo_interval = UNDO.undo_interval().get();
            return r;
        }

        UndoStackInfo {
            stack: vec![],
            undo_interval: Duration::ZERO,
        }
    }
}

static_id! {
    static ref WEAK_UNDO_SCOPE_ID: StateId<(std::sync::Weak<UndoScope>, Var<Duration>)>;
}

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
    fn include(&mut self, timestamp: DInstant) -> bool;
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
    fn include(&mut self, _: DInstant) -> bool {
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
    prev: Option<DInstant>,
    interval: Duration,
    op: UndoOp,
}
impl UndoSelect for UndoSelectInterval {
    fn include(&mut self, timestamp: DInstant) -> bool {
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
impl crate::private::Sealed for DInstant {}
impl UndoSelector for DInstant {
    type Select = UndoSelectLtEq;

    fn select(self, op: UndoOp) -> Self::Select {
        UndoSelectLtEq { instant: self, op }
    }
}
#[doc(hidden)]
pub struct UndoSelectLtEq {
    instant: DInstant,
    op: UndoOp,
}
impl UndoSelect for UndoSelectLtEq {
    fn include(&mut self, timestamp: DInstant) -> bool {
        match self.op {
            UndoOp::Undo => timestamp >= self.instant,
            UndoOp::Redo => timestamp <= self.instant,
        }
    }
}

#[cfg(test)]
mod tests {
    use zng_app::APP;
    use zng_ext_input::keyboard::KeyboardManager;

    use super::*;

    #[test]
    fn register() {
        let _a = APP
            .minimal()
            .extend(UndoManager::default())
            .extend(KeyboardManager::default())
            .run_headless(false);
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
        let _a = APP
            .minimal()
            .extend(UndoManager::default())
            .extend(KeyboardManager::default())
            .run_headless(false);
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
        let _a = APP
            .minimal()
            .extend(UndoManager::default())
            .extend(KeyboardManager::default())
            .run_headless(false);
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
        let _a = APP
            .minimal()
            .extend(UndoManager::default())
            .extend(KeyboardManager::default())
            .run_headless(false);
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
        let _a = APP
            .minimal()
            .extend(UndoManager::default())
            .extend(KeyboardManager::default())
            .run_headless(false);
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
        std::thread::sleep(Duration::from_millis(100));
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
        let _a = APP
            .minimal()
            .extend(UndoManager::default())
            .extend(KeyboardManager::default())
            .run_headless(false);
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
        undo_redo_t_large(Duration::from_secs(10));
    }

    fn undo_redo_t_large(t: Duration) {
        let _a = APP
            .minimal()
            .extend(UndoManager::default())
            .extend(KeyboardManager::default())
            .run_headless(false);
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
        let mut app = APP
            .minimal()
            .extend(UndoManager::default())
            .extend(KeyboardManager::default())
            .run_headless(false);

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

        fn info(&mut self) -> Arc<dyn UndoInfo> {
            Arc::new("push")
        }

        fn as_any(&mut self) -> &mut dyn Any {
            self
        }

        fn merge(self: Box<Self>, args: UndoActionMergeArgs) -> Result<Box<dyn UndoAction>, (Box<dyn UndoAction>, Box<dyn UndoAction>)> {
            Err((self, args.next))
        }
    }
    impl RedoAction for PushAction {
        fn redo(self: Box<Self>) -> Box<dyn UndoAction> {
            self.data.lock().push(self.item);
            self
        }

        fn info(&mut self) -> Arc<dyn UndoInfo> {
            Arc::new("push")
        }
    }
}
