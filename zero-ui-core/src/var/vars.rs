use retain_mut::RetainMut;

use super::*;
use crate::{
    app::{AppEventSender, AppShutdown, RecvFut, TimeoutOrAppShutdown},
    context::{AppContext, Updates},
    crate_util::{Handle, HandleOwner, PanicPayload, RunOnDrop},
    handler::{AppHandler, AppHandlerArgs, AppWeakHandle},
};
use std::{
    any::type_name,
    cell::{Cell, RefCell},
    fmt,
    ops::Deref,
    time::{Duration, Instant},
};

thread_singleton!(SingletonVars);

type SyncEntry = Box<dyn Fn(&Vars) -> Retain>;
type Retain = bool;

type VarBindingFn = Box<dyn FnMut(&Vars) -> Retain>;

/// Read-only access to variables.
///
/// In some contexts variables can be set, so a full [`Vars`] reference is given, in other contexts
/// variables can only be read, so a [`VarsRead`] reference is given.
///
/// [`Vars`] dereferences to to this type and a reference to it is available in [`InfoContext`] and [`RenderContext`].
/// Methods that expect the [`VarsRead`] reference usually abstract using the [`WithVarsRead`] trait, that allows passing in
/// the full context reference or references to async contexts.
///
/// # Examples
///
/// You can [`get`] a variable value using the [`VarsRead`] reference:
///
/// ```
/// # use zero_ui_core::var::{Var, VarsRead};
/// fn get(var: &impl Var<bool>, vars: &VarsRead) -> bool {
///     *var.get(vars)
/// }
/// ```
///
/// And because of auto-dereference you can can use the same method using a full [`Vars`] reference:
///
/// ```
/// # use zero_ui_core::var::{Var, Vars};
/// fn get(var: &impl Var<bool>, vars: &Vars) -> bool {
///     *var.get(vars)
/// }
/// ```
///
/// But [`get`] actually receives any [`WithVarsRead`] implementer so you can just use the full context reference, if you are
/// not borrowing another part of it:
///
/// ```
/// # use zero_ui_core::{var::{Var, VarsRead}, context::LayoutContext};
/// fn get(var: &impl Var<bool>, ctx: &LayoutContext) -> bool {
///     *var.get(ctx)
/// }
/// ```
///
/// # Context Vars
///
/// Context variables can be changed in a context using the [`VarsRead`] instance, the `with_context_var` method calls
/// a closure while a context variable is set to a [`ContextVarData`] value.
///
/// ```
/// # use zero_ui_core::{*, context::*, var::*};
/// # context_var! { pub struct FooVar: bool = false; }
/// # struct FooNode<C, V> { child: C, var: V }
/// # #[impl_ui_node(child)]
/// impl<C: UiNode, V: Var<bool>> UiNode for FooNode<C, V> {
///     fn update(&mut self, ctx: &mut WidgetContext) {
///         ctx.vars.with_context_var(FooVar, ContextVarData::var(ctx.vars, &self.var), || self.child.update(ctx));
///     }
/// }
/// ```
///
/// The example binds a `FooVar` to another `var` for the duration of the [`update`] call. The `var` value and version
/// are accessible in inner widgets using only the `FooVar`.
///
/// Note that the example is incomplete, [`init`], [`deinit`] and the other methods should also be implemented. You can use
/// the [`with_context_var`] helper function to declare a node that binds a context var in all [`UiNode`] methods.
///
/// [new]: Var::is_new
/// [`render`]: crate::UiNode::render
/// [`UiNode`]: crate::UiNode
/// [`render_update`]: crate::UiNode::render_update
/// [`get`]: Var::get
/// [`InfoContext`]: crate::context::InfoContext
/// [`RenderContext`]: crate::context::RenderContext
pub struct VarsRead {
    _singleton: SingletonVars,
    update_id: u32,
    #[allow(clippy::type_complexity)]
    widget_clear: RefCell<Vec<Box<dyn Fn(bool)>>>,

    app_event_sender: AppEventSender,
    senders: RefCell<Vec<SyncEntry>>,
    receivers: RefCell<Vec<SyncEntry>>,
}
impl fmt::Debug for VarsRead {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VarsRead {{ .. }}")
    }
}
impl VarsRead {
    /// Id of the current update cycle, can be used to determinate if a variable value is new.
    pub(super) fn update_id(&self) -> u32 {
        self.update_id
    }

    /// Gets a var at the context level.
    pub(super) fn context_var<C: ContextVar>(&self) -> ContextVarData<C::Type> {
        let source = C::thread_local_value().get();

        // SAFETY: this is safe as long we are the only one to call `C::thread_local_value().get()` in
        // `Self::with_context_var`.
        //
        // The reference is held for as long as it is accessible in here, at least:
        //
        // * The initial reference is actually the `static` default value.
        // * Other references are held by `Self::with_context_var` for the duration
        //   they can appear here.
        unsafe { source.to_safe(self) }
    }

    /// Calls `f` with the context var set to `source`.
    ///
    /// # Source Update
    ///
    /// Nodes within `f` expect the same source [`update_mask`] from the previous call, if you are swapping the
    /// entire `source` value for a new one you must request an [`info`] update.
    ///
    /// [`update_mask`]: ContextVarData::update_mask
    /// [`info`]: crate::context::Updates::info
    #[inline(always)]
    pub fn with_context_var<C, R, F>(&self, context_var: C, data: ContextVarData<C::Type>, f: F) -> R
    where
        C: ContextVar,
        F: FnOnce() -> R,
    {
        // SAFETY: `Self::context_var` makes safety assumptions about this code
        // don't change before studying it.

        let _ = context_var;

        let prev = C::thread_local_value().replace(data.to_raw());
        let _restore = RunOnDrop::new(move || {
            C::thread_local_value().set(prev);
        });

        f()

        // _prev restores the parent reference here on drop
    }

    /// Calls `f` with the context var set to `value`, but only for the current widget not its descendants.
    ///
    /// # Source Update
    ///
    /// Nodes within `f` expect the same source [`update_mask`] from the previous call, if you are swapping the
    /// entire `source` value for a new one you must request an [`info`] update.
    ///
    /// [`update_mask`]: ContextVarData::update_mask
    /// [`info`]: crate::context::Updates::info
    #[inline(always)]
    pub fn with_context_var_wgt_only<C, R, F>(&self, context_var: C, data: ContextVarData<C::Type>, f: F) -> R
    where
        C: ContextVar,
        F: FnOnce() -> R,
    {
        // SAFETY: `Self::context_var` makes safety assumptions about this code
        // don't change before studying it.

        let _ = context_var;

        let new = data.to_raw();
        let prev = C::thread_local_value().replace(new);

        self.widget_clear.borrow_mut().push(Box::new(move |undo| {
            if undo {
                C::thread_local_value().set(prev);
            } else {
                C::thread_local_value().set(new);
            }
        }));

        let _restore = RunOnDrop::new(move || {
            C::thread_local_value().set(prev);
        });

        f()
    }

    /// Clears widget only context var values, calls `f` and restores widget only context var values.
    ///
    /// This is called by the layout and render contexts.
    #[inline(always)]
    pub(crate) fn with_widget_clear<R, F: FnOnce() -> R>(&self, f: F) -> R {
        let wgt_clear = std::mem::take(&mut *self.widget_clear.borrow_mut());
        for clear in &wgt_clear {
            clear(true);
        }

        let _restore = RunOnDrop::new(move || {
            for clear in &wgt_clear {
                clear(false);
            }
            *self.widget_clear.borrow_mut() = wgt_clear;
        });

        f()
    }

    /// Creates a channel that can receive `var` updates from another thread.
    ///
    /// Every time the variable updates a clone of the value is sent to the receiver. The current value is sent immediately.
    ///
    /// This is called by [`Var::receiver`].
    pub(super) fn receiver<T, V>(&self, var: &V) -> VarReceiver<T>
    where
        T: VarValue + Send,
        V: Var<T>,
    {
        let (sender, receiver) = flume::unbounded();
        let _ = sender.send(var.get(self).clone());

        if var.always_read_only() {
            self.senders.borrow_mut().push(Box::new(move |_| {
                // retain if not disconnected.
                !sender.is_disconnected()
            }));
        } else {
            let var = var.clone();
            self.senders.borrow_mut().push(Box::new(move |vars| {
                if let Some(new) = var.get_new(vars) {
                    sender.send(new.clone()).is_ok()
                } else {
                    !sender.is_disconnected()
                }
            }));
        }

        VarReceiver { receiver }
    }
}

/// Applies pending update and returns the var update mask if it updated, otherwise returns `UpdateMask::none`.
type PendingUpdate = Box<dyn FnOnce(u32) -> UpdateMask>;

/// Read-write access to variables.
///
/// Only a single instance of this struct exists per-app and a reference to it is available in
/// [`AppContext`], [`WindowContext`] and [`WidgetContext`].
///
/// This struct dereferences to [`VarsRead`] and implements [`WithVarsRead`] so you can use it
/// in any context that requests read-only access to variables, but it also allows setting or modifying
/// variables and checking if a variable value [`is_new`].
///
/// # Examples
///
/// You can [`get`] and [`set`] variables using the [`Vars`] reference:
///
/// ```
/// # use zero_ui_core::var::*;
/// fn get_set(var: &impl Var<bool>, vars: &Vars) {
///     let flag = *var.get(vars);
///     var.set(vars, !flag).ok();
/// }
/// ```
///
/// But most methods actually receives any [`WithVars`] implementer so you can just use the full context reference, if you are
/// not borrowing another part of it:
///
/// ```
/// # use zero_ui_core::{var::*, context::WidgetContext};
/// fn get_set(var: &impl Var<bool>, ctx: &mut WidgetContext) {
///     let flag = *var.get(ctx);
///     var.set(ctx, !flag).ok();
/// }
/// ```
///
/// Variable values are stored in the variable not in the [`Vars`] and yet methods like [`get`] tie-in the [`Vars`] lifetime
/// with the variable lifetime when you borrow the value, this is a compile time validation that no variable values are borrowed
/// when they are replaced. Internally a runtime validation verifies that [`Vars`] is the only instance in the thread and it
/// must be exclusively borrowed to apply the variable changes, this let variables be implemented very cheaply without needing
/// to use a mechanism like `RefCell`.
///
/// # Binding
///
/// Variables can be *bound* to one another using the `bind_*` methods of [`Var<T>`]. Those methods are implemented using [`bind`]
/// which creates an special update handler that can modify any captured variables *once* before the rest of the app sees the update.
/// You can use [`bind`] to create more exotic bindings that don't have the same shape as a mapping.
///
/// [`AppContext`]: crate::context::AppContext
/// [`WindowContext`]: crate::context::WindowContext
/// [`WidgetContext`]: crate::context::WidgetContext
/// [`is_new`]: crate::var::Var::is_new
/// [new]: crate::var::Var::is_new
/// [`get`]: crate::var::Var::is_new
/// [`set`]: crate::var::Var::is_new
/// [`bind`]: crate::var::Vars::bind
/// [`init`]: crate::UiNode::init
/// [`update`]: crate::UiNode::init
/// [`deinit`]: crate::UiNode::deinit
/// [`UiNode`]: crate::UiNode
pub struct Vars {
    read: VarsRead,

    binding_update_id: u32,
    bindings: RefCell<Vec<VarBindingFn>>,

    #[allow(clippy::type_complexity)]
    pending: RefCell<Vec<PendingUpdate>>,

    pre_handlers: RefCell<Vec<OnVarHandler>>,
    pos_handlers: RefCell<Vec<OnVarHandler>>,
}
impl fmt::Debug for Vars {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Vars {{ .. }}")
    }
}
impl Vars {
    /// If an instance of `Vars` already exists in the  current thread.
    #[inline]
    pub(crate) fn instantiated() -> bool {
        SingletonVars::in_use()
    }

    /// Produces the instance of `Vars`. Only a single
    /// instance can exist in a thread at a time, panics if called
    /// again before dropping the previous instance.
    #[inline]
    pub(crate) fn instance(app_event_sender: AppEventSender) -> Self {
        Vars {
            read: VarsRead {
                _singleton: SingletonVars::assert_new("Vars"),
                update_id: 1u32,
                app_event_sender,
                widget_clear: Default::default(),
                senders: RefCell::default(),
                receivers: RefCell::default(),
            },
            binding_update_id: 0u32.wrapping_sub(13),
            bindings: RefCell::default(),
            pending: Default::default(),
            pre_handlers: RefCell::default(),
            pos_handlers: RefCell::default(),
        }
    }

    /// Schedule set/modify.
    pub(super) fn push_change(&self, change: PendingUpdate) {
        self.pending.borrow_mut().push(change);
    }

    pub(crate) fn has_pending_updates(&mut self) -> bool {
        !self.pending.get_mut().is_empty()
    }

    /// Apply scheduled set/modify.
    pub(crate) fn apply_updates(&mut self, updates: &mut Updates) {
        self.read.update_id = self.update_id.wrapping_add(1);

        let pending = self.pending.get_mut();
        if !pending.is_empty() {
            let mut mask = UpdateMask::none();
            for f in pending.drain(..) {
                mask |= f(self.read.update_id);
            }

            if !mask.is_none() {
                // update bindings
                if !self.bindings.get_mut().is_empty() {
                    self.binding_update_id = self.binding_update_id.wrapping_add(1);

                    loop {
                        self.bindings.borrow_mut().retain_mut(|f| f(self));

                        let pending = self.pending.get_mut();
                        if pending.is_empty() {
                            break;
                        }
                        for f in pending.drain(..) {
                            mask |= f(self.read.update_id);
                        }
                    }
                }

                // send values.
                self.senders.borrow_mut().retain(|f| f(self));

                // does an app update because some vars have new values.
                updates.update(mask);
            }
        }
    }

    /// Receive and apply set/modify from [`VarSender`] and [`VarModifySender`] instances.
    pub(crate) fn receive_sended_modify(&self) {
        self.receivers.borrow_mut().retain(|f| f(self));
    }

    /// Creates a channel that can set `var` from other threads.
    ///
    /// The channel wakes the app and causes a variable update.
    ///
    /// This is called by [`Var::receiver`].
    pub(super) fn sender<T, V>(&self, var: &V) -> VarSender<T>
    where
        T: VarValue + Send,
        V: Var<T>,
    {
        let (sender, receiver) = flume::unbounded();

        if var.always_read_only() {
            self.receivers.borrow_mut().push(Box::new(move |_| {
                receiver.drain();
                !receiver.is_disconnected()
            }));
        } else {
            let var = var.clone();
            self.receivers.borrow_mut().push(Box::new(move |vars| {
                if let Some(new_value) = receiver.try_iter().last() {
                    let _ = var.set(vars, new_value);
                }
                !receiver.is_disconnected()
            }));
        };

        VarSender {
            wake: self.app_event_sender.clone(),
            sender,
        }
    }

    /// Creates a channel that can modify `var` from other threads.
    ///
    /// If the variable is read-only when a modification is received it is silently dropped.
    ///
    /// This is called by [`Var::modify_sender`].
    pub(super) fn modify_sender<T, V>(&self, var: &V) -> VarModifySender<T>
    where
        T: VarValue,
        V: Var<T>,
    {
        let (sender, receiver) = flume::unbounded::<Box<dyn FnOnce(&mut VarModify<T>) + Send>>();

        if var.always_read_only() {
            self.receivers.borrow_mut().push(Box::new(move |_| {
                receiver.drain();
                !receiver.is_disconnected()
            }));
        } else {
            let var = var.clone();
            self.receivers.borrow_mut().push(Box::new(move |vars| {
                for modify in receiver.try_iter() {
                    let _ = var.modify(vars, modify);
                }
                !receiver.is_disconnected()
            }));
        }

        VarModifySender {
            wake: self.app_event_sender.clone(),
            sender,
        }
    }

    /// Adds a handler to all var updates that can modify captured variables **without** causing a second update.
    ///
    /// This is used by the [`Var`] map binding methods, it enables the effect of bound variables getting a new
    /// value in the same update as the variables that caused the new value.
    ///
    /// Returns a [`VarBindingHandle`] that can be used to monitor the binding status and to [`unbind`] or to
    /// make the binding [`permanent`].
    ///
    /// # Examples
    ///
    /// The example updates `squared_var` and `count_var` *at the same time* as `source_var`:
    ///
    /// ```
    /// # use zero_ui_core::{var::*, *};
    /// fn bind_square(
    ///     vars: &Vars,
    ///     source_var: &impl Var<u64>,
    ///     squared_var: &impl Var<u64>,
    ///     count_var: &impl Var<u32>
    /// ) {
    ///     count_var.set(vars, 0u32).ok();
    ///     vars.bind(clone_move!(source_var, squared_var, count_var, |vars, binding| {
    ///         if let Some(i) = source_var.copy_new(vars) {
    ///             if let Some(squared) = i.checked_mul(i) {
    ///                 squared_var.set(vars, squared).ok();
    ///                 count_var.modify(vars, |c| **c += 1).ok();
    ///             } else {
    ///                 binding.unbind();
    ///             }
    ///         }
    ///     })).permanent();
    /// }
    /// ```
    ///
    /// Note that the binding can be undone from the inside, the closure second parameter is a [`VarBinding`]. In
    /// the example this is the only way to stop the binding, because we called [`permanent`]. Bindings hold a clone
    /// of the variables and exist for the duration of the app if not unbound.
    ///
    /// In the example all three variables will update *at the same time* until the binding finishes. They will
    /// **not** update just from creating the binding, the `squared_var` will have its old value until `source_var` updates, you
    /// can cause an update immediately after creating a binding by calling [`Var::touch`].
    ///
    /// You can *chain* bindings, if you have two bindings `VarA -> VarB` and `VarB -> VarC`, `VarC` will update
    /// when `VarA` updates. It is not possible to create an infinite loop however, because `binding` is not called again in an
    /// app update if it modifies any variable, so if you add an extra binding `VarC -> VarA` it will run, but it will not cause
    /// the first binding to run again.
    ///
    /// [`unbind`]: VarBindingHandle::unbind
    /// [`permanent`]: VarBindingHandle::permanent
    pub fn bind<B>(&self, mut binding: B) -> VarBindingHandle
    where
        B: FnMut(&Vars, &VarBinding) + 'static,
    {
        let (handle_owner, handle) = VarBindingHandle::new();

        let mut last_update_id = self.binding_update_id;

        self.bindings.borrow_mut().push(Box::new(move |vars| {
            let mut retain = !handle_owner.is_dropped();

            if vars.binding_update_id == last_update_id {
                return retain;
            }

            let changes_count = vars.pending.borrow().len();

            if retain {
                let info = VarBinding::new();
                binding(vars, &info);
                retain = !info.unbind.get();
            }

            if retain && vars.pending.borrow().len() > changes_count {
                // binding caused change, stop it from running this app update.
                last_update_id = vars.binding_update_id;
            }

            retain
        }));

        handle
    }

    /// If one or more variables have pending updates.
    pub fn update_requested(&self) -> bool {
        !self.pending.borrow().is_empty()
    }

    /// Create a variable update preview handler.
    ///
    /// The `handler` is called every time the `var` value is set, modified or touched. The handler is called before
    /// the UI update that notified the variable update, and after all other previous registered handlers.
    ///
    /// Returns a [`OnVarHandle`] that can be used to unsubscribe, you can also unsubscribe from inside the handler by calling
    /// [`unsubscribe`](crate::handler::AppWeakHandle::unsubscribe) in the third parameter of [`app_hn!`] or [`async_app_hn!`].
    ///
    /// The handler also auto-unsubscribes if the variable [`strong_count`] becomes `1`.
    ///
    /// If the `var` cannot update it and the `handler` are immediately dropped and the [`dummy`] handle is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use zero_ui_core::var::*;
    /// # use zero_ui_core::handler::app_hn;
    /// fn trace_var<T: VarValue>(var: &impl Var<T>, vars: &Vars) {
    ///     let mut prev_value = format!("{:?}", var.get(vars));
    ///     vars.on_pre_var(var.clone(), app_hn!(|_ctx, new_value, _subscription| {
    ///         let new_value = format!("{:?}", new_value);
    ///         println!("{} -> {}", prev_value, new_value);
    ///         prev_value = new_value;
    ///     })).permanent();
    /// }
    /// ```
    ///
    /// The example traces the value changes of a variable.
    ///
    /// # Handlers
    ///
    /// the handler can be any type that implements [`AppHandler`], there are multiple flavors of handlers, including
    /// async handlers that allow calling `.await`. The handler closures can be declared using [`app_hn!`], [`async_app_hn!`],
    /// [`app_hn_once!`] and [`async_app_hn_once!`].
    ///
    /// [`dummy`]: OnVarHandle::dummy
    /// [`app_hn!`]: crate::handler::app_hn!
    /// [`async_app_hn!`]: crate::handler::async_app_hn!
    /// [`app_hn_once!`]: crate::handler::app_hn_once!
    /// [`async_app_hn!`]: crate::handler::async_app_hn_once!
    /// [`strong_count`]: Var::strong_count
    pub fn on_pre_var<T, V, H>(&self, var: V, handler: H) -> OnVarHandle
    where
        T: VarValue,
        V: Var<T>,
        H: AppHandler<T>,
    {
        if !var.can_update() {
            return OnVarHandle::dummy();
        }

        Self::push_var_handler(&self.pre_handlers, true, var, handler)
    }

    /// Create a variable update handler.
    ///
    /// The `handler` is called every time the `var` value is set, modified or touched, the call happens after
    /// all other app components where notified.
    ///
    /// Returns a [`OnVarHandle`] that can be used to unsubscribe, you can also unsubscribe from inside the handler by calling
    /// [`unsubscribe`](crate::handler::AppWeakHandle::unsubscribe) in the third parameter of [`app_hn!`] or [`async_app_hn!`].
    ///
    /// The handler also auto-unsubscribes if the variable [`strong_count`] becomes `1`.
    ///
    /// If the `var` cannot update it and the `handler` are immediately dropped and the [`dummy`] handle is returned.
    ///
    /// # Handlers
    ///
    /// the event handler can be any type that implements [`AppHandler`], there are multiple flavors of handlers, including
    /// async handlers that allow calling `.await`. The handler closures can be declared using [`app_hn!`], [`async_app_hn!`],
    /// [`app_hn_once!`] and [`async_app_hn_once!`].
    ///
    /// [`dummy`]: OnVarHandle::dummy
    /// [`app_hn!`]: crate::handler::app_hn!
    /// [`async_app_hn!`]: crate::handler::async_app_hn!
    /// [`app_hn_once!`]: crate::handler::app_hn_once!
    /// [`async_app_hn!`]: crate::handler::async_app_hn_once!
    /// [`strong_count`]: Var::strong_count
    pub fn on_var<T, V, H>(&self, var: V, handler: H) -> OnVarHandle
    where
        T: VarValue,
        V: Var<T>,
        H: AppHandler<T>,
    {
        if !var.can_update() {
            return OnVarHandle::dummy();
        }

        Self::push_var_handler(&self.pos_handlers, false, var, handler)
    }

    fn push_var_handler<T, V, H>(handlers: &RefCell<Vec<OnVarHandler>>, is_preview: bool, var: V, mut handler: H) -> OnVarHandle
    where
        T: VarValue,
        V: Var<T>,
        H: AppHandler<T>,
    {
        if !var.can_update() {
            return OnVarHandle::dummy();
        }

        let (handle_owner, handle) = OnVarHandle::new();
        let handler = move |ctx: &mut AppContext, handle: &dyn AppWeakHandle| {
            if let Some(new_value) = var.get_new(ctx.vars) {
                handler.event(ctx, new_value, &AppHandlerArgs { handle, is_preview });
            }
            if var.strong_count() == 1 {
                handle.unsubscribe();
            }
        };

        handlers.borrow_mut().push(OnVarHandler {
            handle: handle_owner,
            handler: Box::new(handler),
        });

        handle
    }

    pub(crate) fn on_pre_vars(ctx: &mut AppContext) {
        Self::on_vars_impl(&ctx.vars.pre_handlers, ctx)
    }

    pub(crate) fn on_vars(ctx: &mut AppContext) {
        Self::on_vars_impl(&ctx.vars.pos_handlers, ctx)
    }

    fn on_vars_impl(handlers: &RefCell<Vec<OnVarHandler>>, ctx: &mut AppContext) {
        let mut current = std::mem::take(&mut *handlers.borrow_mut());

        current.retain_mut(|e| {
            !e.handle.is_dropped() && {
                (e.handler)(ctx, &e.handle.weak_handle());
                !e.handle.is_dropped()
            }
        });

        let mut new = handlers.borrow_mut();
        current.extend(std::mem::take(&mut *new));
        *new = current;
    }
}
impl Deref for Vars {
    type Target = VarsRead;

    fn deref(&self) -> &Self::Target {
        &self.read
    }
}

struct OnVarHandler {
    handle: HandleOwner<()>,
    handler: Box<dyn FnMut(&mut AppContext, &dyn AppWeakHandle)>,
}

/// Represents an app context handler created by [`Vars::on_var`] or [`Vars::on_pre_var`].
///
/// Drop all clones of this handle to drop the handler, or call [`unsubscribe`](Self::unsubscribe) to drop the handle
/// without dropping the handler.
#[derive(Clone)]
#[must_use = "the handler unsubscribes if the handle is dropped"]
pub struct OnVarHandle(Handle<()>);
impl OnVarHandle {
    fn new() -> (HandleOwner<()>, OnVarHandle) {
        let (owner, handle) = Handle::new(());
        (owner, OnVarHandle(handle))
    }

    /// Create a handle to nothing, the handle always in the *unsubscribed* state.
    #[inline]
    pub fn dummy() -> Self {
        OnVarHandle(Handle::dummy(()))
    }

    /// Drop the handle but does **not** unsubscribe.
    ///
    /// The handler stays in memory for the duration of the app or until another handle calls [`unsubscribe`](Self::unsubscribe.)
    #[inline]
    pub fn permanent(self) {
        self.0.permanent();
    }

    /// If another handle has called [`permanent`](Self::permanent).
    /// If `true` the var binding will stay active until the app shutdown, unless [`unsubscribe`](Self::unsubscribe) is called.
    #[inline]
    pub fn is_permanent(&self) -> bool {
        self.0.is_permanent()
    }

    /// Drops the handle and forces the handler to drop.
    #[inline]
    pub fn unsubscribe(self) {
        self.0.force_drop()
    }

    /// If another handle has called [`unsubscribe`](Self::unsubscribe).
    ///
    /// The handler is already dropped or will be dropped in the next app update, this is irreversible.
    #[inline]
    pub fn is_unsubscribed(&self) -> bool {
        self.0.is_dropped()
    }
}

/// Represents a type that can provide access to a [`Vars`] inside the window of function call.
///
/// This is used to make vars assign less cumbersome to use, it is implemented to all sync and async context types and [`Vars`] it-self.
///
/// # Example
///
/// The example demonstrate how this `trait` simplifies calls to [`Var::set`]. The same applies to [`Var::modify`] and [`Var::set_ne`].
///
/// ```
/// # use zero_ui_core::{var::*, context::*};
/// # struct Foo { foo_var: RcVar<&'static str> } impl Foo {
/// fn update(&mut self, ctx: &mut WidgetContext) {
///     self.foo_var.set(ctx, "we are not borrowing `ctx` so can use it directly");
///
///    // ..
///    let services = &mut ctx.services;
///    self.foo_var.set(ctx.vars, "we are partially borrowing `ctx` but not `ctx.vars` so we use that");
/// }
///
/// async fn handler(&mut self, ctx: WidgetContextMut) {
///     self.foo_var.set(&ctx, "async contexts can also be used");
/// }
/// # }
/// ```
pub trait WithVars {
    /// Calls `action` with the [`Vars`] reference.
    fn with_vars<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&Vars) -> R;
}
impl WithVars for Vars {
    fn with_vars<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&Vars) -> R,
    {
        action(self)
    }
}
impl<'a> WithVars for crate::context::AppContext<'a> {
    fn with_vars<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&Vars) -> R,
    {
        action(self.vars)
    }
}
impl<'a> WithVars for crate::context::WindowContext<'a> {
    fn with_vars<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&Vars) -> R,
    {
        action(self.vars)
    }
}
impl<'a> WithVars for crate::context::WidgetContext<'a> {
    fn with_vars<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&Vars) -> R,
    {
        action(self.vars)
    }
}
impl<'a> WithVars for crate::context::LayoutContext<'a> {
    fn with_vars<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&Vars) -> R,
    {
        action(self.vars)
    }
}
impl WithVars for crate::context::AppContextMut {
    fn with_vars<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&Vars) -> R,
    {
        self.with(move |ctx| action(ctx.vars))
    }
}
impl WithVars for crate::context::WidgetContextMut {
    fn with_vars<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&Vars) -> R,
    {
        self.with(move |ctx| action(ctx.vars))
    }
}
#[cfg(any(test, doc, feature = "test_util"))]
impl WithVars for crate::context::TestWidgetContext {
    fn with_vars<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&Vars) -> R,
    {
        action(&self.vars)
    }
}
impl WithVars for crate::app::HeadlessApp {
    fn with_vars<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&Vars) -> R,
    {
        action(self.vars())
    }
}

/// Represents a type that can provide access to a [`VarsRead`] inside the window of function call.
///
/// This is used to make vars value-read less cumbersome to use, it is implemented to all sync and async context
/// types and [`Vars`] it-self.
pub trait WithVarsRead {
    /// Calls `action` with the [`Vars`] reference.
    fn with_vars_read<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&VarsRead) -> R;
}
impl WithVarsRead for Vars {
    fn with_vars_read<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&VarsRead) -> R,
    {
        action(self)
    }
}
impl WithVarsRead for VarsRead {
    fn with_vars_read<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&VarsRead) -> R,
    {
        action(self)
    }
}
impl<'a> WithVarsRead for crate::context::AppContext<'a> {
    fn with_vars_read<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&VarsRead) -> R,
    {
        action(self.vars)
    }
}
impl<'a> WithVarsRead for crate::context::WindowContext<'a> {
    fn with_vars_read<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&VarsRead) -> R,
    {
        action(self.vars)
    }
}
impl<'a> WithVarsRead for crate::context::WidgetContext<'a> {
    fn with_vars_read<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&VarsRead) -> R,
    {
        action(self.vars)
    }
}
impl WithVarsRead for crate::context::AppContextMut {
    fn with_vars_read<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&VarsRead) -> R,
    {
        self.with(move |ctx| action(ctx.vars))
    }
}
impl WithVarsRead for crate::context::WidgetContextMut {
    fn with_vars_read<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&VarsRead) -> R,
    {
        self.with(move |ctx| action(ctx.vars))
    }
}
impl<'a> WithVarsRead for crate::context::LayoutContext<'a> {
    fn with_vars_read<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&VarsRead) -> R,
    {
        action(self.vars)
    }
}
impl<'a> WithVarsRead for crate::context::RenderContext<'a> {
    fn with_vars_read<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&VarsRead) -> R,
    {
        action(self.vars)
    }
}
impl<'a> WithVarsRead for crate::context::InfoContext<'a> {
    fn with_vars_read<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&VarsRead) -> R,
    {
        action(self.vars)
    }
}
#[cfg(any(test, doc, feature = "test_util"))]
impl WithVarsRead for crate::context::TestWidgetContext {
    fn with_vars_read<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&VarsRead) -> R,
    {
        action(&self.vars)
    }
}
impl WithVarsRead for crate::app::HeadlessApp {
    fn with_vars_read<R, A>(&self, action: A) -> R
    where
        A: FnOnce(&VarsRead) -> R,
    {
        action(self.vars())
    }
}

impl AsRef<VarsRead> for VarsRead {
    fn as_ref(&self) -> &VarsRead {
        self
    }
}
impl AsRef<VarsRead> for Vars {
    fn as_ref(&self) -> &VarsRead {
        self
    }
}
impl<'a> AsRef<VarsRead> for crate::context::AppContext<'a> {
    fn as_ref(&self) -> &VarsRead {
        self.vars
    }
}
impl<'a> AsRef<VarsRead> for crate::context::WindowContext<'a> {
    fn as_ref(&self) -> &VarsRead {
        self.vars
    }
}
impl<'a> AsRef<VarsRead> for crate::context::WidgetContext<'a> {
    fn as_ref(&self) -> &VarsRead {
        self.vars
    }
}
impl<'a> AsRef<VarsRead> for crate::context::LayoutContext<'a> {
    fn as_ref(&self) -> &VarsRead {
        self.vars
    }
}
impl<'a> AsRef<VarsRead> for crate::context::RenderContext<'a> {
    fn as_ref(&self) -> &VarsRead {
        self.vars
    }
}
impl<'a> AsRef<VarsRead> for crate::context::InfoContext<'a> {
    fn as_ref(&self) -> &VarsRead {
        self.vars
    }
}
#[cfg(any(test, doc, feature = "test_util"))]
impl AsRef<VarsRead> for crate::context::TestWidgetContext {
    fn as_ref(&self) -> &VarsRead {
        &self.vars
    }
}
impl AsRef<VarsRead> for crate::app::HeadlessApp {
    fn as_ref(&self) -> &VarsRead {
        self.vars()
    }
}
impl AsRef<Vars> for Vars {
    fn as_ref(&self) -> &Vars {
        self
    }
}
impl<'a> AsRef<Vars> for crate::context::AppContext<'a> {
    fn as_ref(&self) -> &Vars {
        self.vars
    }
}
impl<'a> AsRef<Vars> for crate::context::WindowContext<'a> {
    fn as_ref(&self) -> &Vars {
        self.vars
    }
}
impl<'a> AsRef<Vars> for crate::context::WidgetContext<'a> {
    fn as_ref(&self) -> &Vars {
        self.vars
    }
}
#[cfg(any(test, doc, feature = "test_util"))]
impl AsRef<Vars> for crate::context::TestWidgetContext {
    fn as_ref(&self) -> &Vars {
        &self.vars
    }
}
impl AsRef<Vars> for crate::app::HeadlessApp {
    fn as_ref(&self) -> &Vars {
        self.vars()
    }
}

/// A variable update receiver that can be used from any thread and without access to [`Vars`].
///
/// Use [`Var::receiver`] to create a receiver, drop to stop listening.
pub struct VarReceiver<T: VarValue + Send> {
    receiver: flume::Receiver<T>,
}
impl<T: VarValue + Send> Clone for VarReceiver<T> {
    fn clone(&self) -> Self {
        VarReceiver {
            receiver: self.receiver.clone(),
        }
    }
}
impl<T: VarValue + Send> fmt::Debug for VarReceiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VarReceiver<{}>", type_name::<T>())
    }
}
impl<T: VarValue + Send> VarReceiver<T> {
    /// Receives the oldest sent update not received, blocks until the variable updates.
    #[inline]
    pub fn recv(&self) -> Result<T, AppShutdown<()>> {
        self.receiver.recv().map_err(|_| AppShutdown(()))
    }

    /// Tries to receive the oldest sent update, returns `Ok(args)` if there was at least
    /// one update, or returns `Err(None)` if there was no update or returns `Err(AppHasShutdown)` if the connected
    /// app has shutdown.
    #[inline]
    pub fn try_recv(&self) -> Result<T, Option<AppShutdown<()>>> {
        self.receiver.try_recv().map_err(|e| match e {
            flume::TryRecvError::Empty => None,
            flume::TryRecvError::Disconnected => Some(AppShutdown(())),
        })
    }

    /// Receives the oldest sent update, blocks until the event updates or until the `deadline` is reached.
    #[inline]
    pub fn recv_deadline(&self, deadline: Instant) -> Result<T, TimeoutOrAppShutdown> {
        self.receiver.recv_deadline(deadline).map_err(TimeoutOrAppShutdown::from)
    }

    /// Receives the oldest sent update, blocks until the event updates or until timeout.
    #[inline]
    pub fn recv_timeout(&self, dur: Duration) -> Result<T, TimeoutOrAppShutdown> {
        self.receiver.recv_timeout(dur).map_err(TimeoutOrAppShutdown::from)
    }

    /// Returns a future that receives the oldest sent update, awaits until an event update occurs.
    #[inline]
    pub fn recv_async(&self) -> RecvFut<T> {
        self.receiver.recv_async().into()
    }

    /// Turns into a future that receives the oldest sent update, awaits until an event update occurs.
    #[inline]
    pub fn into_recv_async(self) -> RecvFut<'static, T> {
        self.receiver.into_recv_async().into()
    }

    /// Creates a blocking iterator over event updates, if there are no updates in the buffer the iterator blocks,
    /// the iterator only finishes when the app shuts-down.
    #[inline]
    pub fn iter(&self) -> flume::Iter<T> {
        self.receiver.iter()
    }

    /// Create a non-blocking iterator over event updates, the iterator finishes if
    /// there are no more updates in the buffer.
    #[inline]
    pub fn try_iter(&self) -> flume::TryIter<T> {
        self.receiver.try_iter()
    }
}
impl<T: VarValue + Send> From<VarReceiver<T>> for flume::Receiver<T> {
    fn from(e: VarReceiver<T>) -> Self {
        e.receiver
    }
}
impl<'a, T: VarValue + Send> IntoIterator for &'a VarReceiver<T> {
    type Item = T;

    type IntoIter = flume::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.receiver.iter()
    }
}
impl<T: VarValue + Send> IntoIterator for VarReceiver<T> {
    type Item = T;

    type IntoIter = flume::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.receiver.into_iter()
    }
}

/// A variable update sender that can set a variable from any thread and without access to [`Vars`].
///
/// Use [`Var::sender`] to create a sender, drop to stop holding the paired variable in the UI thread.
pub struct VarSender<T>
where
    T: VarValue + Send,
{
    wake: AppEventSender,
    sender: flume::Sender<T>,
}
impl<T: VarValue + Send> Clone for VarSender<T> {
    fn clone(&self) -> Self {
        VarSender {
            wake: self.wake.clone(),
            sender: self.sender.clone(),
        }
    }
}
impl<T: VarValue + Send> fmt::Debug for VarSender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VarSender<{}>", type_name::<T>())
    }
}
impl<T> VarSender<T>
where
    T: VarValue + Send,
{
    /// Sends a new value for the variable, unless the connected app has shutdown.
    ///
    /// If the variable is read-only when the `new_value` is received it is silently dropped, if more then one
    /// value is sent before the app can process then, only the last value shows as an update in the UI thread.
    pub fn send(&self, new_value: T) -> Result<(), AppShutdown<T>> {
        self.sender.send(new_value).map_err(AppShutdown::from)?;
        let _ = self.wake.send_var();
        Ok(())
    }

    /// Resume a panic in the app thread.
    pub fn send_resume_unwind(&self, payload: PanicPayload) -> Result<(), AppShutdown<PanicPayload>> {
        self.wake.send_resume_unwind(payload)
    }
}

/// A variable modification sender that can be used to modify a variable from any thread and without access to [`Vars`].
///
/// Use [`Var::modify_sender`] to create a sender, drop to stop holding the paired variable in the UI thread.
pub struct VarModifySender<T>
where
    T: VarValue,
{
    wake: AppEventSender,
    sender: flume::Sender<Box<dyn FnOnce(&mut VarModify<T>) + Send>>,
}
impl<T: VarValue> Clone for VarModifySender<T> {
    fn clone(&self) -> Self {
        VarModifySender {
            wake: self.wake.clone(),
            sender: self.sender.clone(),
        }
    }
}
impl<T: VarValue> fmt::Debug for VarModifySender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VarModifySender<{}>", type_name::<T>())
    }
}
impl<T> VarModifySender<T>
where
    T: VarValue,
{
    /// Sends a modification for the variable, unless the connected app has shutdown.
    ///
    /// If the variable is read-only when the `modify` is received it is silently dropped, if more then one
    /// modification is sent before the app can process then, they all are applied in order sent.
    pub fn send<F>(&self, modify: F) -> Result<(), AppShutdown<()>>
    where
        F: FnOnce(&mut VarModify<T>) + Send + 'static,
    {
        self.sender.send(Box::new(modify)).map_err(|_| AppShutdown(()))?;
        let _ = self.wake.send_var();
        Ok(())
    }

    /// Resume a panic in the app thread.
    pub fn send_resume_unwind(&self, payload: PanicPayload) -> Result<(), AppShutdown<PanicPayload>> {
        self.wake.send_resume_unwind(payload)
    }
}

/// Variable sender used to notify the completion of an operation from any thread.
///
/// Use [`response_channel`] to init.
pub type ResponseSender<T> = VarSender<Response<T>>;
impl<T: VarValue + Send> ResponseSender<T> {
    /// Send the one time response.
    pub fn send_response(&self, response: T) -> Result<(), AppShutdown<T>> {
        self.send(Response::Done(response)).map_err(|e| {
            if let Response::Done(r) = e.0 {
                AppShutdown(r)
            } else {
                unreachable!()
            }
        })
    }
}

/// New paired [`ResponseSender`] and [`ResponseVar`] in the waiting state.
pub fn response_channel<T: VarValue + Send, Vw: WithVars>(vars: &Vw) -> (ResponseSender<T>, ResponseVar<T>) {
    let (responder, response) = response_var();
    vars.with_vars(|vars| (responder.sender(vars), response))
}

/// Represents a variable binding created by one of the `bind` methods of [`Vars`] or [`Var`].
///
/// Drop all clones of this handle to drop the binding, or call [`permanent`] to drop the handle
/// but keep the binding alive for the duration of the app.
///
/// [`permanent`]: VarBindingHandle::permanent
#[derive(Clone)]
#[must_use = "the var binding is undone if the handle is dropped"]
pub struct VarBindingHandle(Handle<()>);
impl VarBindingHandle {
    fn new() -> (HandleOwner<()>, VarBindingHandle) {
        let (owner, handle) = Handle::new(());
        (owner, VarBindingHandle(handle))
    }

    /// Create dummy handle that is always in the *unbound* state.
    #[inline]
    pub fn dummy() -> VarBindingHandle {
        VarBindingHandle(Handle::dummy(()))
    }

    /// Drop the handle but does **not** unbind.
    ///
    /// The var binding stays in memory for the duration of the app or until another handle calls [`unbind`](Self::unbind.)
    #[inline]
    pub fn permanent(self) {
        self.0.permanent();
    }

    /// If another handle has called [`permanent`](Self::permanent).
    /// If `true` the var binding will stay active until the app shutdown, unless [`unbind`](Self::unbind) is called.
    #[inline]
    pub fn is_permanent(&self) -> bool {
        self.0.is_permanent()
    }

    /// Drops the handle and forces the binding to drop.
    #[inline]
    pub fn unbind(self) {
        self.0.force_drop();
    }

    /// If another handle has called [`unbind`](Self::unbind).
    ///
    /// The var binding is already dropped or will be dropped in the next app update, this is irreversible.
    #[inline]
    pub fn is_unbound(&self) -> bool {
        self.0.is_dropped()
    }
}

/// Represents the variable binding in its binding closure.
///
/// All of the `bind` methods of [`Vars`] take a closure that take a reference to this info
/// as input, they can use it to drop the variable binding from the inside.
pub struct VarBinding {
    unbind: Cell<bool>,
}
impl VarBinding {
    fn new() -> Self {
        VarBinding { unbind: Cell::new(false) }
    }

    /// Drop the binding after applying the returned update.
    #[inline]
    pub fn unbind(&self) {
        self.unbind.set(true);
    }
}

#[cfg(test)]
mod tests {
    use crate::app::App;
    use crate::context::TestWidgetContext;
    use crate::text::ToText;
    use crate::var::{context_var, var, ContextVarData, Var};

    #[test]
    fn one_way_binding() {
        let a = var(10);
        let b = var("".to_text());

        let mut app = App::blank().run_headless(false);

        a.bind_map(&app.ctx(), &b, |_, a| a.to_text()).permanent();

        let mut update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(app.ctx().vars, 20);

        update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;
                assert_eq!(Some(20i32), a.copy_new(ctx));
                assert_eq!(Some("20".to_text()), b.clone_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);

        a.set(app.ctx().vars, 13);

        update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;
                assert_eq!(Some(13i32), a.copy_new(ctx));
                assert_eq!(Some("13".to_text()), b.clone_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);
    }

    #[test]
    fn two_way_binding() {
        let a = var(10);
        let b = var("".to_text());

        let mut app = App::blank().run_headless(false);

        a.bind_map_bidi(&app.ctx(), &b, |_, a| a.to_text(), |_, b| b.parse().unwrap())
            .permanent();

        let mut update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(app.ctx().vars, 20);

        update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;
                assert_eq!(Some(20i32), a.copy_new(ctx));
                assert_eq!(Some("20".to_text()), b.clone_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);

        b.set(app.ctx().vars, "55");

        update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;
                assert_eq!(Some("55".to_text()), b.clone_new(ctx));
                assert_eq!(Some(55i32), a.copy_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);
    }

    #[test]
    fn one_way_filtered_binding() {
        let a = var(10);
        let b = var("".to_text());

        let mut app = App::blank().run_headless(false);

        a.bind_filter(&app.ctx(), &b, |_, a| if *a == 13 { None } else { Some(a.to_text()) })
            .permanent();

        let mut update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(app.ctx().vars, 20);

        update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;
                assert_eq!(Some(20i32), a.copy_new(ctx));
                assert_eq!(Some("20".to_text()), b.clone_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);

        a.set(app.ctx().vars, 13);

        update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;
                assert_eq!(Some(13i32), a.copy_new(ctx));
                assert_eq!("20".to_text(), b.get_clone(ctx));
                assert!(!b.is_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);
    }

    #[test]
    fn two_way_filtered_binding() {
        let a = var(10);
        let b = var("".to_text());

        let mut app = App::blank().run_headless(false);

        a.bind_filter_bidi(&app.ctx(), &b, |_, a| Some(a.to_text()), |_, b| b.parse().ok())
            .permanent();

        let mut update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(app.ctx().vars, 20);

        update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;
                assert_eq!(Some(20i32), a.copy_new(ctx));
                assert_eq!(Some("20".to_text()), b.clone_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);

        b.set(app.ctx().vars, "55");

        update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;
                assert_eq!(Some("55".to_text()), b.clone_new(ctx));
                assert_eq!(Some(55i32), a.copy_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);

        b.set(app.ctx().vars, "not a i32");

        update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;
                assert_eq!(Some("not a i32".to_text()), b.clone_new(ctx));
                assert_eq!(55i32, a.copy(ctx));
                assert!(!a.is_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);
    }

    #[test]
    fn binding_chain() {
        let a = var(0);
        let b = var(0);
        let c = var(0);
        let d = var(0);

        let mut app = App::blank().run_headless(false);

        a.bind_map(&app.ctx(), &b, |_, a| *a + 1).permanent();
        b.bind_map(&app.ctx(), &c, |_, b| *b + 1).permanent();
        c.bind_map(&app.ctx(), &d, |_, c| *c + 1).permanent();

        let mut update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(app.ctx().vars, 20);

        let mut update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;

                assert_eq!(Some(20), a.copy_new(ctx));
                assert_eq!(Some(21), b.copy_new(ctx));
                assert_eq!(Some(22), c.copy_new(ctx));
                assert_eq!(Some(23), d.copy_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);

        a.set(app.ctx().vars, 30);

        let mut update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;

                assert_eq!(Some(30), a.copy_new(ctx));
                assert_eq!(Some(31), b.copy_new(ctx));
                assert_eq!(Some(32), c.copy_new(ctx));
                assert_eq!(Some(33), d.copy_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);
    }

    #[test]
    fn binding_bidi_chain() {
        let a = var(0);
        let b = var(0);
        let c = var(0);
        let d = var(0);

        let mut app = App::blank().run_headless(false);

        a.bind_bidi(&app.ctx(), &b).permanent();
        b.bind_bidi(&app.ctx(), &c).permanent();
        c.bind_bidi(&app.ctx(), &d).permanent();

        let mut update_count = 0;
        let _ = app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(app.ctx().vars, 20);

        let mut update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;

                assert_eq!(Some(20), a.copy_new(ctx));
                assert_eq!(Some(20), b.copy_new(ctx));
                assert_eq!(Some(20), c.copy_new(ctx));
                assert_eq!(Some(20), d.copy_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);

        d.set(app.ctx().vars, 30);

        let mut update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;

                assert_eq!(Some(30), a.copy_new(ctx));
                assert_eq!(Some(30), b.copy_new(ctx));
                assert_eq!(Some(30), c.copy_new(ctx));
                assert_eq!(Some(30), d.copy_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);
    }

    #[test]
    fn binding_drop_from_inside() {
        let a = var(1);
        let b = var(1);

        let mut app = App::blank().run_headless(false);

        let _handle = a.bind_map(&app.ctx(), &b, |info, i| {
            info.unbind();
            *i + 1
        });

        a.set(app.ctx().vars, 10);

        let mut update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;

                assert_eq!(Some(10), a.copy_new(ctx));
                assert_eq!(Some(11), b.copy_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);

        assert_eq!(1, a.strong_count());
        assert_eq!(1, b.strong_count());

        a.set(app.ctx().vars, 100);

        update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;

                assert_eq!(Some(100), a.copy_new(ctx));
                assert!(!b.is_new(ctx));
                assert_eq!(11, b.copy(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);
    }

    #[test]
    fn binding_drop_from_outside() {
        let a = var(1);
        let b = var(1);

        let mut app = App::blank().run_headless(false);

        let handle = a.bind_map(&app.ctx(), &b, |_, i| *i + 1);

        a.set(app.ctx().vars, 10);

        let mut update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;

                assert_eq!(Some(10), a.copy_new(ctx));
                assert_eq!(Some(11), b.copy_new(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);

        drop(handle);

        a.set(app.ctx().vars, 100);

        update_count = 0;
        let _ = app.update_observe(
            |ctx| {
                update_count += 1;

                assert_eq!(Some(100), a.copy_new(ctx));
                assert!(!b.is_new(ctx));
                assert_eq!(11, b.copy(ctx));
            },
            false,
        );
        assert_eq!(1, update_count);

        assert_eq!(1, a.strong_count());
        assert_eq!(1, b.strong_count());
    }

    #[test]
    fn context_var_default() {
        let ctx = TestWidgetContext::new();
        let value = *TestVar::new().get(&ctx.vars);
        assert_eq!("default value", value);
    }

    #[test]
    fn context_var_with() {
        let ctx = TestWidgetContext::new();
        let value = ctx
            .vars
            .with_context_var(TestVar, ContextVarData::fixed(&"with value"), || *TestVar::new().get(&ctx.vars));

        assert_eq!("with value", value);

        let value = *TestVar::new().get(&ctx.vars);
        assert_eq!("default value", value);
    }

    #[test]
    fn context_var_with_other() {
        let ctx = TestWidgetContext::new();

        let value = ctx
            .vars
            .with_context_var(TestVar, ContextVarData::var(&ctx.vars, &TestVar2::new()), || {
                *TestVar::new().get(&ctx.vars)
            });

        assert_eq!("default value 2", value);
    }

    #[test]
    fn context_var_recursion1() {
        let ctx = TestWidgetContext::new();

        let value = ctx
            .vars
            .with_context_var(TestVar, ContextVarData::var(&ctx.vars, &TestVar::new()), || {
                *TestVar::new().get(&ctx.vars)
            });

        assert_eq!("default value", value);
    }

    #[test]
    fn context_var_recursion2() {
        let ctx = TestWidgetContext::new();

        let value = ctx
            .vars
            .with_context_var(TestVar, ContextVarData::var(&ctx.vars, &TestVar2::new()), || {
                // set to "default value 2"
                ctx.vars
                    .with_context_var(TestVar2, ContextVarData::var(&ctx.vars, &TestVar::new()), || {
                        // set to "default value 2"
                        *TestVar::new().get(&ctx.vars)
                    })
            });

        assert_eq!("default value 2", value);
    }

    context_var! {
        struct TestVar: &'static str = "default value";
        struct TestVar2: &'static str = "default value 2";
    }
}
