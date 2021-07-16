use retain_mut::RetainMut;
use zero_ui_proc_macros::impl_ui_node;

use super::*;
use crate::{
    app::{AppEventSender, AppShutdown, RecvFut, TimeoutOrAppShutdown},
    context::{LayoutContext, RenderContext, Updates, WidgetContext},
    crate_util::{Handle, HandleOwner, RunOnDrop},
    event::EventUpdateArgs,
    render::{FrameBuilder, FrameUpdate},
    units::LayoutSize,
    UiNode,
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
/// [`Vars`] dereferences to to this type and a reference to it is available in [`LayoutContext`] and [`RenderContext`].
/// Methods that expect the [`VarsRead`] reference usually abstract using the [`WithVarsRead`] trait, that allows passing in
/// the full context reference or references to async contexts.
///
/// There is only one
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
/// Context variables can be changed in a context using the [`VarsRead`] instance, the `with_context*` methods call
/// a closure while a context variable is set to a value or bound to another variable. These methods are *duplicated*
/// in [`Vars`], the difference is that in [`VarsRead`] the context vars cannot be [new], because variables cannot be
/// new in [`LayoutContext`] and [`RenderContext`].
///
/// ```
/// # use zero_ui_core::{*, context::*, var::*, render::*};
/// # context_var! { pub struct FooVar: bool = const false; }
/// # struct FooNode<C, V> { child: C, var: V }
/// # #[impl_ui_node(child)]
/// impl<C: UiNode, V: Var<bool>> UiNode for FooNode<C, V> {
///     fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
///         ctx.vars.with_context_bind(FooVar, &self.var, || self.child.render(ctx, frame));
///     }
/// }
/// ```
///
/// The example binds a `FooVar` to another `var` for the duration of the [`render`] call. The `var` value and version
/// are accessible in inner widgets using only the `FooVar`.
///
/// Note that the example is incomplete, [`render_update`] should also be implemented at least. You can use the [`with_context_var`]
/// helper function to declare a node that binds a context var in all [`UiNode`] methods.
///
/// [new]: Var::is_new
/// [`render`]: UiNode::render
/// [`render_update`]: UiNode::render_update
/// [`get`]: Var::get
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
    pub(super) fn context_var<C: ContextVar>(&self) -> (&C::Type, bool, u32) {
        let (value, is_new, version) = C::thread_local_value().get();

        (
            // SAFETY: this is safe as long we are the only one to call `C::thread_local_value().get()` in
            // `Self::with_context_var`.
            //
            // The reference is held for as long as it is accessible in here, at least:
            //
            // * The initial reference is actually the `static` default value.
            // * Other references are held by `Self::with_context_var` for the duration
            //   they can appear here.
            unsafe { &*value },
            is_new,
            version,
        )
    }

    /// Calls `f` with the context var set to `value`.
    ///
    /// Unlike [`Vars::with_context_var`] in this method the context-var is never [new].
    ///
    /// See also the [`with_context_var_expr`] helper function for declaring a property that sets a context var.
    ///
    /// [new]: Var::is_new
    #[inline(always)]
    pub fn with_context_var<C, R, F>(&self, context_var: C, value: &C::Type, version: u32, f: F) -> R
    where
        C: ContextVar,
        F: FnOnce() -> R,
    {
        self.with_context_var_impl(context_var, value, false, version, f)
    }
    #[inline(always)]
    fn with_context_var_impl<C, R, F>(&self, context_var: C, value: &C::Type, is_new: bool, version: u32, f: F) -> R
    where
        C: ContextVar,
        F: FnOnce() -> R,
    {
        // SAFETY: `Self::context_var` makes safety assumptions about this code
        // don't change before studying it.

        let _ = context_var;
        let prev = C::thread_local_value().replace((value as _, is_new, version));
        let _restore = RunOnDrop::new(move || {
            C::thread_local_value().set(prev);
        });

        f()

        // _prev restores the parent reference here on drop
    }

    /// Calls `f` with the context var set to `value`, but only for the current widget not its descendants.
    ///
    /// Unlike [`Vars::with_context_var_wgt_only`] in this method the context-var is never [new].
    ///
    /// See also the [`with_context_var_wgt_only_expr`] helper function to declare a property that sets a context var.
    ///
    /// [new]: Var::is_new
    #[inline(always)]
    pub fn with_context_var_wgt_only<C, R, F>(&self, context_var: C, value: &C::Type, version: u32, f: F) -> R
    where
        C: ContextVar,
        F: FnOnce() -> R,
    {
        self.with_context_var_wgt_only_impl(context_var, value, false, version, f)
    }
    #[inline(always)]
    fn with_context_var_wgt_only_impl<C, R, F>(&self, context_var: C, value: &C::Type, is_new: bool, version: u32, f: F) -> R
    where
        C: ContextVar,
        F: FnOnce() -> R,
    {
        // SAFETY: `Self::context_var` makes safety assumptions about this code
        // don't change before studying it.

        let _ = context_var;

        let new = (value as _, is_new, version);
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

    /// Calls `f` while `context_var` is bound to `other_var`.
    ///
    /// Unlike [`Vars::with_context_bind`] in this method the context-var is never [new].
    ///
    /// See also the [`with_context_var`] helper function to declare a property that sets a context var.
    ///
    /// [new]: Var::is_new
    #[inline(always)]
    pub fn with_context_bind<C, R, F, V>(&self, context_var: C, other_var: &V, f: F) -> R
    where
        C: ContextVar,
        F: FnOnce() -> R,
        V: Var<C::Type>,
    {
        self.with_context_var_impl(context_var, other_var.get(self), false, other_var.version(self), f)
    }

    /// Calls `f` while `context_var` is bound to `other_var`, but only for the current widget not its descendants.
    ///
    /// Unlike [`Vars::with_context_bind`] in this method the context-var is never [new].
    ///
    /// See also the [`with_context_var_wgt_only`] helper function to declare a property that sets a context var.
    ///
    /// [new]: Var::is_new
    #[inline(always)]
    pub fn with_context_bind_wgt_only<C: ContextVar, R, F: FnOnce() -> R, V: Var<C::Type>>(
        &self,
        context_var: C,
        other_var: &V,
        f: F,
    ) -> R {
        self.with_context_var_wgt_only_impl(context_var, other_var.get(self), false, other_var.version(self), f)
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

type PendingUpdate = Box<dyn FnOnce(u32) -> bool>;

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
/// # Context Vars
///
/// Context variables can be changed in a context using the [`Vars`] instance, the `with_context*` methods call
/// a closure while a context variable is set to a value or bound to another variable. These methods are *duplicated*
/// in [`VarsRead`], the difference is that in here the variable can be [new].
///
/// ```
/// # use zero_ui_core::{*, context::*, var::*};
/// # context_var! { pub struct FooVar: bool = const false; }
/// # struct FooNode<C, V> { child: C, var: V }
/// # #[impl_ui_node(child)]
/// impl<C: UiNode, V: Var<bool>> UiNode for FooNode<C, V> {
///     fn update(&mut self, ctx: &mut WidgetContext) {
///         let child = &mut self.child;
///         ctx.vars.with_context_bind(FooVar, &self.var, || child.update(ctx));
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
/// # Binding
///
/// Variables can be *bound* to one another using the `bind_*` methods of [`Var<T>`]. Those methods are implemented using [`bind`]
/// which creates an special update handler that can modify any captured variables *once* before the rest of the app sees the update.
/// You can use [`bind`] to create more exotic bindings that don't have the same shape as a mapping.
///
/// [`AppContext`]: crate::context::AppContext
/// [`WindowContext`]: crate::context::WindowContext
/// [`is_new`]: crate::var::Var::is_new
/// [new]: crate::var::Var::is_new
/// [`get`]: crate::var::Var::is_new
/// [`set`]: crate::var::Var::is_new
/// [`bind`]: crate::var::Vars::bind
/// [`init`]: crate::UiNode::init
/// [`update`]: crate::UiNode::init
/// [`deinit`]: crate::UiNode::deinit
pub struct Vars {
    read: VarsRead,

    binding_update_id: u32,
    bindings: RefCell<Vec<VarBindingFn>>,

    #[allow(clippy::type_complexity)]
    pending: RefCell<Vec<PendingUpdate>>,
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
                update_id: 0u32.wrapping_sub(13),
                app_event_sender,
                widget_clear: Default::default(),
                senders: RefCell::default(),
                receivers: RefCell::default(),
            },
            binding_update_id: 0u32.wrapping_sub(13),
            bindings: RefCell::default(),
            pending: Default::default(),
        }
    }

    /// Calls `f` with the context var set to `value`.
    ///
    /// The value is visible for the duration of `f`, unless `f` recursive overwrites it again.
    ///
    /// See also the [`with_context_var_expr`] helper function for declaring a property that sets a context var.
    #[inline(always)]
    pub fn with_context_var<C: ContextVar, F: FnOnce()>(&self, context_var: C, value: &C::Type, is_new: bool, version: u32, f: F) {
        self.with_context_var_impl(context_var, value, is_new, version, f)
    }

    /// Calls `f` with the context var set to `value`, but only for the current widget not its descendants.
    ///
    /// The value is visible for the duration of `f` and only for the parts of it that are inside the current widget context.
    ///
    /// The value can be overwritten by a recursive call to [`with_context_var`](Vars::with_context_var) or
    /// this method, subsequent values from this same widget context are not visible in inner widget contexts.
    ///
    /// See also the [`with_context_var_wgt_only_expr`] helper function for declaring a property that sets a context var.
    #[inline(always)]
    pub fn with_context_var_wgt_only<C: ContextVar, F: FnOnce()>(&self, context_var: C, value: &C::Type, is_new: bool, version: u32, f: F) {
        self.with_context_var_wgt_only_impl(context_var, value, is_new, version, f)
    }

    /// Calls `f` while `context_var` is bound to `other_var`.
    ///
    /// See also the [`with_context_var`] helper function to declare a property that sets a context var.
    #[inline(always)]
    pub fn with_context_bind<C: ContextVar, F: FnOnce(), V: Var<C::Type>>(&self, context_var: C, other_var: &V, f: F) {
        self.with_context_var_impl(context_var, other_var.get(self), other_var.is_new(self), other_var.version(self), f)
    }

    /// Calls `f` while `context_var` is bound to `other_var`, but only for the current widget not its descendants.
    ///
    /// See also the [`with_context_var_wgt_only`] helper function to declare a property that sets a context var.
    #[inline(always)]
    pub fn with_context_bind_wgt_only<C: ContextVar, F: FnOnce(), V: Var<C::Type>>(&self, context_var: C, other_var: &V, f: F) {
        self.with_context_var_wgt_only(context_var, other_var.get(self), other_var.is_new(self), other_var.version(self), f)
    }

    /// Schedule set/modify.
    pub(super) fn push_change(&self, change: PendingUpdate) {
        self.pending.borrow_mut().push(change);
    }

    /// Apply scheduled set/modify.
    pub(crate) fn apply_updates(&mut self, updates: &mut Updates) {
        self.read.update_id = self.update_id.wrapping_add(1);

        let pending = self.pending.get_mut();
        if !pending.is_empty() {
            let mut modified = false;
            for f in pending.drain(..) {
                modified |= f(self.read.update_id);
            }

            if modified {
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
                            f(self.read.update_id);
                        }
                    }
                }

                // send values.
                self.senders.borrow_mut().retain(|f| f(self));

                // does an app update because some vars have new values.
                updates.update();
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
}
impl Deref for Vars {
    type Target = VarsRead;

    fn deref(&self) -> &Self::Target {
        &self.read
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
impl<'a, 'w> WithVars for crate::context::AppContext<'a, 'w> {
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
impl<'a, 'w> WithVarsRead for crate::context::AppContext<'a, 'w> {
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
impl<'a, 'w> AsRef<VarsRead> for crate::context::AppContext<'a, 'w> {
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
impl<'a, 'w> AsRef<Vars> for crate::context::AppContext<'a, 'w> {
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
    vars.with_vars(|vars| (vars.sender(&responder), response))
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

/// Helper for declaring properties that sets a context var.
///
/// The method presents the `value` as the [`ContextVar<Type=T>`] in the widget and widget descendants.
/// The context var [`version`] and [`is_new`] status are always equal to the `value` var status.
///
/// The generated [`UiNode`] delegates each method to `child` inside a call to [`Vars::with_context_bind`].
///
/// # Examples
///
/// A simple context property declaration:
///
/// ```
/// # fn main() -> () { }
/// # use zero_ui_core::{*, var::*};
/// context_var! {
///     pub struct FooVar: u32 = const 0;
/// }
///
/// /// Sets the [`FooVar`] in the widgets and its content.
/// #[property(context, default(FooVar))]
/// pub fn foo(child: impl UiNode, value: impl IntoVar<u32>) -> impl UiNode {
///     with_context_var(child, FooVar, value)
/// }
/// ```
///
/// When set in a widget, the `value` is accessible in all inner nodes of the widget, using `FooVar.get`, and if `value` is set to a
/// variable the `FooVar` will also reflect its [`is_new`] and [`version`].
///
/// Also note that the property [`default`] is set to the same `FooVar`, this causes the property to *pass-through* the outer context
/// value, as if it was not set.
///
/// [`version`]: Var::version
/// [`is_new`]: Var::is_new
/// [`default`]: crate::property#default
pub fn with_context_var<T: VarValue>(child: impl UiNode, var: impl ContextVar<Type = T>, value: impl IntoVar<T>) -> impl UiNode {
    struct WithContextVarNode<U, C, V> {
        child: U,
        var: C,
        value: V,
    }
    impl<U, T, C, V> UiNode for WithContextVarNode<U, C, V>
    where
        U: UiNode,
        T: VarValue,
        C: ContextVar<Type = T>,
        V: Var<T>,
    {
        #[inline(always)]
        fn init(&mut self, ctx: &mut WidgetContext) {
            let child = &mut self.child;
            ctx.vars.with_context_bind(self.var, &self.value, || child.init(ctx));
        }
        #[inline(always)]
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            let child = &mut self.child;
            ctx.vars.with_context_bind(self.var, &self.value, || child.deinit(ctx));
        }
        #[inline(always)]
        fn update(&mut self, ctx: &mut WidgetContext) {
            let child = &mut self.child;
            ctx.vars.with_context_bind(self.var, &self.value, || child.update(ctx));
        }
        #[inline(always)]
        fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
            let child = &mut self.child;
            ctx.vars.with_context_bind(self.var, &self.value, || child.event(ctx, args));
        }
        #[inline(always)]
        fn measure(&mut self, ctx: &mut LayoutContext, available_size: LayoutSize) -> LayoutSize {
            let child = &mut self.child;
            ctx.vars
                .with_context_bind(self.var, &self.value, || child.measure(ctx, available_size))
        }
        #[inline(always)]
        fn arrange(&mut self, ctx: &mut LayoutContext, final_size: LayoutSize) {
            let child = &mut self.child;
            ctx.vars.with_context_bind(self.var, &self.value, || child.arrange(ctx, final_size));
        }
        #[inline(always)]
        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            let child = &self.child;
            ctx.vars.with_context_bind(self.var, &self.value, || child.render(ctx, frame));
        }
        #[inline(always)]
        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            let child = &self.child;
            ctx.vars
                .with_context_bind(self.var, &self.value, || child.render_update(ctx, update));
        }
    }
    WithContextVarNode {
        child,
        var,
        value: value.into_var(),
    }
}

/// Helper for declaring properties that sets a context var for the widget only.
///
/// This is similar to [`with_context_var`] except the context var value is visible only inside
/// the `child` nodes that are part of the same widget that is the parent of the return node.
///
/// # Examples
///
/// ```
/// # fn main() -> () { }
/// # use zero_ui_core::{*, var::*, border::BorderRadius};
/// context_var! {
///     pub struct CornersClipVar: BorderRadius = once BorderRadius::zero();
/// }
///
/// /// Sets widget content clip corner radius.
/// #[property(context, default(CornersClipVar))]
/// pub fn corners_clip(child: impl UiNode, radius: impl IntoVar<BorderRadius>) -> impl UiNode {
///     with_context_var_wgt_only(child, CornersClipVar, radius)
/// }
/// ```
pub fn with_context_var_wgt_only<T: VarValue>(child: impl UiNode, var: impl ContextVar<Type = T>, value: impl IntoVar<T>) -> impl UiNode {
    struct WithContextVarWidgetOnlyNode<U, C, V> {
        child: U,
        var: C,
        value: V,
    }
    #[impl_ui_node(child)]
    impl<U, T, C, V> UiNode for WithContextVarWidgetOnlyNode<U, C, V>
    where
        U: UiNode,
        T: VarValue,
        C: ContextVar<Type = T>,
        V: Var<T>,
    {
        #[inline(always)]
        fn init(&mut self, ctx: &mut WidgetContext) {
            let child = &mut self.child;
            ctx.vars.with_context_bind_wgt_only(self.var, &self.value, || child.init(ctx));
        }
        #[inline(always)]
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            let child = &mut self.child;
            ctx.vars.with_context_bind_wgt_only(self.var, &self.value, || child.deinit(ctx));
        }
        #[inline(always)]
        fn update(&mut self, ctx: &mut WidgetContext) {
            let child = &mut self.child;
            ctx.vars.with_context_bind_wgt_only(self.var, &self.value, || child.update(ctx));
        }
        #[inline(always)]
        fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            let child = &mut self.child;
            ctx.vars
                .with_context_bind_wgt_only(self.var, &self.value, || child.event(ctx, args));
        }
        #[inline(always)]
        fn measure(&mut self, ctx: &mut LayoutContext, available_size: LayoutSize) -> LayoutSize {
            let child = &mut self.child;
            ctx.vars
                .with_context_bind_wgt_only(self.var, &self.value, || child.measure(ctx, available_size))
        }
        #[inline(always)]
        fn arrange(&mut self, ctx: &mut LayoutContext, final_size: LayoutSize) {
            let child = &mut self.child;
            ctx.vars
                .with_context_bind_wgt_only(self.var, &self.value, || child.arrange(ctx, final_size))
        }
        #[inline(always)]
        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            ctx.vars
                .with_context_bind_wgt_only(self.var, &self.value, || self.child.render(ctx, frame));
        }
        #[inline(always)]
        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            ctx.vars
                .with_context_bind_wgt_only(self.var, &self.value, || self.child.render_update(ctx, update));
        }
    }
    WithContextVarWidgetOnlyNode {
        child,
        var,
        value: value.into_var(),
    }
}
/// Helper for declaring properties that sets a context var using a closure.
///
/// The method presents the `initial_value` in the widget and widget descendants. In every [`UiNode::update`]
/// the `update` closure is called, if it returns a new value the context var *updates*, for the same [`UiNode::update`]
/// it [`is_new`] and it the new value is retained and presented in each subsequent [`UiNode`] method call.
///
/// The generated [`UiNode`] delegates each method to `child` inside a call to [`Vars::with_context_var`].
///
/// [`is_new`]: Var::is_new
pub fn with_context_var_expr<T: VarValue>(
    child: impl UiNode,
    var: impl ContextVar<Type = T>,
    initial_value: T,
    update: impl FnMut(&mut WidgetContext) -> Option<T> + 'static,
) -> impl UiNode {
    struct WithContextVarExprNode<C, V, U, T> {
        child: C,
        var: V,
        update: U,

        value: T,
        version: u32,
    }
    #[impl_ui_node(child)]
    impl<C, V, U, T> UiNode for WithContextVarExprNode<C, V, U, T>
    where
        C: UiNode,
        V: ContextVar<Type = T>,
        U: FnMut(&mut WidgetContext) -> Option<T> + 'static,
        T: VarValue,
    {
        #[inline(always)]
        fn init(&mut self, ctx: &mut WidgetContext) {
            let child = &mut self.child;
            ctx.vars.with_context_var(self.var, &self.value, false, self.version, || {
                child.init(ctx);
            });
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            let mut is_new = false;
            if let Some(value) = (self.update)(ctx) {
                self.value = value;
                self.version = self.version.wrapping_add(1);
                is_new = true;
            }
            let child = &mut self.child;

            ctx.vars.with_context_var(self.var, &self.value, is_new, self.version, || {
                child.update(ctx);
            });
        }

        #[inline(always)]
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            let child = &mut self.child;
            ctx.vars.with_context_var(self.var, &self.value, false, self.version, || {
                child.deinit(ctx);
            });
        }

        #[inline(always)]
        fn event<A: crate::event::EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            let child = &mut self.child;
            ctx.vars.with_context_var(self.var, &self.value, false, self.version, || {
                child.event(ctx, args);
            });
        }

        #[inline(always)]
        fn measure(
            &mut self,
            ctx: &mut crate::context::LayoutContext,
            available_size: crate::units::LayoutSize,
        ) -> crate::units::LayoutSize {
            let child = &mut self.child;
            ctx.vars
                .with_context_var(self.var, &self.value, self.version, || child.measure(ctx, available_size))
        }

        #[inline(always)]
        fn arrange(&mut self, ctx: &mut crate::context::LayoutContext, final_size: crate::units::LayoutSize) {
            let child = &mut self.child;
            ctx.vars.with_context_var(self.var, &self.value, self.version, || {
                child.arrange(ctx, final_size);
            });
        }

        #[inline(always)]
        fn render(&self, ctx: &mut crate::context::RenderContext, frame: &mut crate::render::FrameBuilder) {
            let value = &self.value;
            ctx.vars.with_context_var(self.var, value, self.version, || {
                self.child.render(ctx, frame);
            });
        }

        #[inline(always)]
        fn render_update(&self, ctx: &mut crate::context::RenderContext, update: &mut crate::render::FrameUpdate) {
            let value = &self.value;
            ctx.vars.with_context_var(self.var, value, self.version, || {
                self.child.render_update(ctx, update);
            });
        }
    }
    WithContextVarExprNode {
        child,
        var,
        update,

        value: initial_value,
        version: 0,
    }
}

/// Helper for declaring properties that sets a context var using a closure for the widget only.
pub fn with_context_var_wgt_only_expr<T: VarValue>(
    child: impl UiNode,
    var: impl ContextVar<Type = T>,
    initial_value: T,
    update: impl FnMut(&mut WidgetContext) -> Option<T> + 'static,
) -> impl UiNode {
    struct WithContextVarWidgetOnlyExprNode<C, V, U, T> {
        child: C,
        var: V,
        update: U,

        value: T,
        version: u32,
    }
    #[impl_ui_node(child)]
    impl<C, V, U, T> UiNode for WithContextVarWidgetOnlyExprNode<C, V, U, T>
    where
        C: UiNode,
        V: ContextVar<Type = T>,
        U: FnMut(&mut WidgetContext) -> Option<T> + 'static,
        T: VarValue,
    {
        #[inline(always)]
        fn init(&mut self, ctx: &mut WidgetContext) {
            let child = &mut self.child;
            ctx.vars.with_context_var_wgt_only(self.var, &self.value, false, self.version, || {
                child.init(ctx);
            });
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            let mut is_new = false;
            if let Some(value) = (self.update)(ctx) {
                self.value = value;
                self.version = self.version.wrapping_add(1);
                is_new = true;
            }
            let child = &mut self.child;

            ctx.vars.with_context_var_wgt_only(self.var, &self.value, is_new, self.version, || {
                child.update(ctx);
            });
        }

        #[inline(always)]
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            let child = &mut self.child;
            ctx.vars.with_context_var_wgt_only(self.var, &self.value, false, self.version, || {
                child.deinit(ctx);
            });
        }

        #[inline(always)]
        fn event<A: crate::event::EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            let child = &mut self.child;
            ctx.vars.with_context_var_wgt_only(self.var, &self.value, false, self.version, || {
                child.event(ctx, args);
            });
        }

        #[inline(always)]
        fn measure(&mut self, ctx: &mut LayoutContext, available_size: crate::units::LayoutSize) -> crate::units::LayoutSize {
            let child = &mut self.child;
            ctx.vars
                .with_context_var_wgt_only(self.var, &self.value, self.version, || child.measure(ctx, available_size))
        }

        #[inline(always)]
        fn arrange(&mut self, ctx: &mut LayoutContext, final_size: crate::units::LayoutSize) {
            let child = &mut self.child;
            ctx.vars.with_context_var_wgt_only(self.var, &self.value, self.version, || {
                child.arrange(ctx, final_size);
            });
        }

        #[inline(always)]
        fn render(&self, ctx: &mut RenderContext, frame: &mut crate::render::FrameBuilder) {
            let value = &self.value;
            ctx.vars.with_context_var_wgt_only(self.var, value, self.version, || {
                self.child.render(ctx, frame);
            });
        }

        #[inline(always)]
        fn render_update(&self, ctx: &mut RenderContext, update: &mut crate::render::FrameUpdate) {
            let value = &self.value;
            ctx.vars.with_context_var_wgt_only(self.var, value, self.version, || {
                self.child.render_update(ctx, update);
            });
        }
    }
    WithContextVarWidgetOnlyExprNode {
        child,
        var,
        update,

        value: initial_value,
        version: 0,
    }
}

#[cfg(test)]
mod tests {
    use crate::app::App;
    use crate::text::ToText;
    use crate::var::{var, Var};

    #[test]
    fn one_way_binding() {
        let a = var(10);
        let b = var("".to_text());

        let mut app = App::blank().run_headless();

        a.bind_map(&app.ctx(), &b, |_, a| a.to_text()).permanent();

        let mut update_count = 0;
        app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(app.ctx().vars, 20);

        update_count = 0;
        app.update_observe(
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
        app.update_observe(
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

        let mut app = App::blank().run_headless();

        a.bind_map_bidi(&app.ctx(), &b, |_, a| a.to_text(), |_, b| b.parse().unwrap())
            .permanent();

        let mut update_count = 0;
        app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(app.ctx().vars, 20);

        update_count = 0;
        app.update_observe(
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
        app.update_observe(
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

        let mut app = App::blank().run_headless();

        a.bind_filter(&app.ctx(), &b, |_, a| if *a == 13 { None } else { Some(a.to_text()) })
            .permanent();

        let mut update_count = 0;
        app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(app.ctx().vars, 20);

        update_count = 0;
        app.update_observe(
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
        app.update_observe(
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

        let mut app = App::blank().run_headless();

        a.bind_filter_bidi(&app.ctx(), &b, |_, a| Some(a.to_text()), |_, b| b.parse().ok())
            .permanent();

        let mut update_count = 0;
        app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(app.ctx().vars, 20);

        update_count = 0;
        app.update_observe(
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
        app.update_observe(
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
        app.update_observe(
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

        let mut app = App::blank().run_headless();

        a.bind_map(&app.ctx(), &b, |_, a| *a + 1).permanent();
        b.bind_map(&app.ctx(), &c, |_, b| *b + 1).permanent();
        c.bind_map(&app.ctx(), &d, |_, c| *c + 1).permanent();

        let mut update_count = 0;
        app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(app.ctx().vars, 20);

        let mut update_count = 0;
        app.update_observe(
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
        app.update_observe(
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

        let mut app = App::blank().run_headless();

        a.bind_bidi(&app.ctx(), &b).permanent();
        b.bind_bidi(&app.ctx(), &c).permanent();
        c.bind_bidi(&app.ctx(), &d).permanent();

        let mut update_count = 0;
        app.update_observe(
            |_| {
                update_count += 1;
            },
            false,
        );
        assert_eq!(0, update_count);

        a.set(app.ctx().vars, 20);

        let mut update_count = 0;
        app.update_observe(
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
        app.update_observe(
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

        let mut app = App::blank().run_headless();

        let _handle = a.bind_map(&app.ctx(), &b, |info, i| {
            info.unbind();
            *i + 1
        });

        a.set(app.ctx().vars, 10);

        let mut update_count = 0;
        app.update_observe(
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
        app.update_observe(
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

        let mut app = App::blank().run_headless();

        let handle = a.bind_map(&app.ctx(), &b, |_, i| *i + 1);

        a.set(app.ctx().vars, 10);

        let mut update_count = 0;
        app.update_observe(
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
        app.update_observe(
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
}
