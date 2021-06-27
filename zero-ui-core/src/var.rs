//! Variables.

use std::{
    fmt,
    ops::{Deref, DerefMut},
};

mod vars;
pub use vars::*;

mod boxed;
pub use boxed::*;

mod context;
pub use context::*;

mod read_only;
pub use read_only::*;

mod owned;
pub use owned::*;

mod rc;
pub use rc::*;

mod map;
pub use map::*;

mod map_ref;
pub use map_ref::*;

mod filter_map;
pub use filter_map::*;

mod merge;
pub use merge::*;

mod switch;
pub use switch::*;

mod when;
pub use when::*;

mod future;
pub use future::*;

/// A type that can be a [`Var`] value.
///
/// # Trait Alias
///
/// This trait is used like a type alias for traits and is
/// already implemented for all types it applies to.
pub trait VarValue: fmt::Debug + Clone + 'static {}
impl<T: fmt::Debug + Clone + 'static> VarValue for T {}

/// Represents a context variable.
///
/// Context variables are [`Var`] implements with different values defined in different **contexts**,
/// usually a parent widget.
///
/// Use [`context_var!`] to declare.
pub trait ContextVar: Clone + Copy + 'static {
    /// The variable type.
    type Type: VarValue;

    /// Default value, used when the variable is not set in a context.
    fn default_value() -> &'static Self::Type;

    /// Gets the variable.
    #[inline]
    fn new() -> ContextVarProxy<Self> {
        ContextVarProxy::new()
    }

    /// Use [`context_var!`] to implement context vars.
    ///
    /// If that is not possible copy the `thread_local` implementation generated
    /// by the macro as close as possible.
    #[doc(hidden)]
    fn thread_local_value() -> ContextVarLocalKey<Self>;
}

/// Error when trying to set or modify a read-only variable.
#[derive(Debug, Hash, PartialEq, Eq)]
pub struct VarIsReadOnly;
impl std::fmt::Display for VarIsReadOnly {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "cannot set or modify read-only variable")
    }
}

/// A value-to-[var](Var) conversion that consumes the value.
pub trait IntoVar<T: VarValue>: Clone {
    /// Variable type that will wrap the `T` value.
    ///
    /// This is the [`OwnedVar`] for most types.
    type Var: Var<T>;

    /// Converts the source value into a var.
    fn into_var(self) -> Self::Var;

    #[doc(hidden)]
    #[allow(non_snake_case)]
    fn allowed_in_when_property_requires_IntoVar_members(&self) -> Self::Var {
        self.clone().into_var()
    }
}

/// Like [`IntoVar`], but for values that don't change.
pub trait IntoValue<T: VarValue>: Into<T> + Clone {}
impl<T: VarValue> IntoValue<T> for T {}

/// Represents an observable value.
pub trait Var<T: VarValue>: Clone + IntoVar<T> + 'static {
    /// The variable type that represents a read-only version of this type.
    type AsReadOnly: Var<T>;

    // TODO when GATs are stable:
    // type Map<B: VarValue, M: FnMut(&T) -> B> : Var<B>;
    // type MapBidi<B: VarValue, M: FnMut(&T) -> B, N: FnMut(&B) -> T> : Var<B>;

    /// References the value.
    fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a T;

    /// Copy the value.
    #[inline]
    fn copy<Vr: WithVarsRead>(&self, vars: &Vr) -> T
    where
        T: Copy,
    {
        vars.with(|v| *self.get(v))
    }

    /// Clone the value.
    #[inline]
    fn get_clone<Vr: WithVarsRead>(&self, vars: &Vr) -> T {
        vars.with(|v| self.get(v).clone())
    }

    /// References the value if [`is_new`](Self::is_new).
    fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a T>;

    /// Copy the value if [`is_new`](Self::is_new).
    #[inline]
    fn copy_new<Vw: WithVars>(&self, vars: &Vw) -> Option<T>
    where
        T: Copy,
    {
        vars.with_vars(|v| self.get_new(v).copied())
    }

    /// Returns a future that awaits for [`copy_new`](Var::copy_new) after the current update.
    ///
    /// You can `.await` this in UI thread bound async code, like in async event handlers. The future
    /// will unblock once for every time [`copy_new`](Var::copy_new) returns `Some(T)` in a different update.
    ///
    /// Note that if [`Var::can_update`] is `false` this will never awake and a warning will be logged.
    ///
    /// # Example
    ///
    /// ```
    /// # use zero_ui_core::var::*;
    /// # use zero_ui_core::handler::async_hn;
    /// # fn __() -> impl zero_ui_core::handler::WidgetHandler<()> {
    /// # let foo_var = var(10u32);
    /// async_hn!(foo_var, |ctx, _| {
    ///     let value = foo_var.wait_copy(&ctx).await;
    ///     assert_eq!(Some(value), foo_var.copy_new(&ctx));
    ///
    ///     let value = foo_var.wait_copy(&ctx).await;
    ///     assert_eq!(Some(value), foo_var.copy_new(&ctx));
    /// })
    /// # }
    /// ```
    ///
    /// In the example the handler awaits for the variable to have a new value, the code immediately after
    /// runs in the app update where the variable is new, the second `.await` does not poll immediately it awaits
    /// for the variable to be new again but in a different update.
    ///
    /// You can also reuse the future, but it is very cheap to just create a new one.
    #[inline]
    fn wait_copy<'a, Vw: WithVars>(&'a self, vars: &'a Vw) -> VarCopyNewFut<'a, Vw, T, Self>
    where
        T: Copy,
    {
        if !self.can_update() {
            log::warn!("`Var::wait_copy` called in a variable that never updates");
        }
        VarCopyNewFut::new(vars, self)
    }

    /// Clone the value if [`is_new`](Self::is_new).
    #[inline]
    fn clone_new<Vw: WithVars>(&self, vars: &Vw) -> Option<T> {
        vars.with_vars(|v| self.get_new(v).cloned())
    }

    /// Returns a future that awaits for [`clone_new`](Var::clone_new) after the current update.
    ///
    /// You can `.await` this in UI thread bound async code, like in async event handlers. The future
    /// will unblock once for every time [`clone_new`](Var::clone_new) returns `Some(T)` in a different update.
    ///
    /// Note that if [`Var::can_update`] is `false` this will never awake and a warning will be logged.
    ///
    /// # Example
    ///
    /// ```
    /// # use zero_ui_core::var::*;
    /// # use zero_ui_core::handler::async_hn;
    /// # fn __() -> impl zero_ui_core::handler::WidgetHandler<()> {
    /// # let foo_var = var(10u32);
    /// async_hn!(foo_var, |ctx, _| {
    ///     let value = foo_var.wait_clone(&ctx).await;
    ///     assert_eq!(Some(value), foo_var.clone_new(&ctx));
    ///
    ///     let value = foo_var.wait_clone(&ctx).await;
    ///     assert_eq!(Some(value), foo_var.clone_new(&ctx));
    /// })
    /// # }
    /// ```
    ///
    /// In the example the handler awaits for the variable to have a new value, the code immediately after
    /// runs in the app update where the variable is new, the second `.await` does not poll immediately it awaits
    /// for the variable to be new again but in a different update.
    ///
    /// You can also reuse the future, but it is very cheap to just create a new one.
    #[inline]
    fn wait_clone<'a, Vw: WithVars>(&'a self, vars: &'a Vw) -> VarCloneNewFut<'a, Vw, T, Self> {
        if !self.can_update() {
            log::warn!("`Var::wait_clone` called in a variable that never updates");
        }
        VarCloneNewFut::new(vars, self)
    }

    /// If the variable value changed in this update.
    ///
    /// When the variable value changes this stays `true` for one app update cycle.
    fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool;

    /// Returns a future that awaits for [`is_new`](Var::is_new) after the current update.
    ///
    /// You can `.await` this in UI thread bound async code, like in async event handlers. The future
    /// will unblock once for every time [`is_new`](Var::is_new) returns `true` in a different update.
    ///
    /// Note that if [`Var::can_update`] is `false` this will never awake and a warning will be logged.
    /// ```
    /// # use zero_ui_core::var::*;
    /// # use zero_ui_core::handler::async_hn;
    /// # fn __() -> impl zero_ui_core::handler::WidgetHandler<()> {
    /// # let foo_var = var(10u32);
    /// async_hn!(foo_var, |ctx, _| {
    ///     foo_var.wait_new(&ctx).await;
    ///     assert!(foo_var.is_new(&ctx));
    ///
    ///     foo_var.wait_new(&ctx).await;
    ///     assert!(foo_var.is_new(&ctx));
    /// })
    /// # }
    /// ```
    ///
    /// In the example the handler awaits for the variable to have a new value, the code immediately after
    /// runs in the app update where the variable is new, the second `.await` does not poll immediately it awaits
    /// for the variable to be new again but in a different update.
    ///
    /// You can also reuse the future, but it is very cheap to just create a new one.
    #[inline]
    fn wait_new<'a, Vw: WithVars>(&'a self, vars: &'a Vw) -> VarIsNewFut<'a, Vw, T, Self> {
        if !self.can_update() {
            log::warn!("`Var::wait_new` called in a variable that never updates");
        }
        VarIsNewFut::new(vars, self)
    }

    /// Gets the variable value version.
    ///
    /// The version is a different number every time the value is modified, you can use this to monitor
    /// variable change outside of the window of opportunity of [`is_new`](Self::is_new).
    fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> u32;

    /// If the variable cannot be set or modified right now.
    ///
    /// **Note** this can change unless the variable is [`always_read_only`](Self::always_read_only).
    fn is_read_only<Vw: WithVars>(&self, vars: &Vw) -> bool;

    /// If the variable can never be set or modified.
    ///
    /// **Note** the value still be new by an internal change if [`can_update`](Self::can_update) is `true`.
    fn always_read_only(&self) -> bool;

    /// If the variable value can change.
    ///
    /// **Note** this can be `true` even if the variable is [`always_read_only`](Self::always_read_only).
    fn can_update(&self) -> bool;

    /// Convert this variable to the value, if the variable is a reference, clones the value.
    fn into_value<Vr: WithVarsRead>(self, vars: &Vr) -> T;

    /// Schedule a modification of the variable value.
    ///
    /// The variable is marked as *new* only if the closure input is dereferenced as `mut`.
    fn modify<Vw, M>(&self, vars: &Vw, modify: M) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        M: FnOnce(&mut VarModify<T>) + 'static;

    /// Causes the variable to notify update without changing the value.
    #[inline]
    fn touch<Vw: WithVars>(&self, vars: &Vw) -> Result<(), VarIsReadOnly> {
        self.modify(vars, |v| v.touch())
    }

    /// Schedule a new value for the variable.
    #[inline]
    fn set<Vw, N>(&self, vars: &Vw, new_value: N) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<T>,
    {
        let new_value = new_value.into();
        self.modify(vars, move |v| **v = new_value)
    }

    /// Schedule a new value for the variable, but only if the current value is not equal to `new_value`.
    #[inline]
    fn set_ne<Vw, N>(&self, vars: &Vw, new_value: N) -> Result<bool, VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<T>,
        T: PartialEq,
    {
        if self.is_read_only(vars) {
            Err(VarIsReadOnly)
        } else {
            let new_value = new_value.into();
            vars.with_vars(|vars| {
                if self.get(vars) != &new_value {
                    let _r = self.set(vars, new_value);
                    debug_assert!(
                        _r.is_ok(),
                        "variable type `{}` said it was not read-only but returned `VarIsReadOnly` on set",
                        std::any::type_name::<Self>()
                    );
                    Ok(true)
                } else {
                    Ok(false)
                }
            })
        }
    }

    /// Box this var.
    #[inline]
    fn boxed(self) -> BoxedVar<T>
    where
        Self: VarBoxed<T> + Sized,
    {
        Box::new(self)
    }

    /// Convert this variable to one that cannot be set or modified.
    fn into_read_only(self) -> Self::AsReadOnly;

    /// Create a read-only variable with a value that is mapped from this variable.
    ///
    /// The value of the map variable is kept up-to-date with the value of this variable, `map` is called every
    /// time the value needs to update.
    ///
    /// Also see [`Var::bind`] to create a *map binding* between two existing variables.
    #[inline]
    fn map<O, M>(&self, map: M) -> RcMapVar<T, O, M, Self>
    where
        O: VarValue,
        M: FnMut(&T) -> O + 'static,
    {
        RcMapVar::new(self.clone(), map)
    }

    /// Create a read-only variable with a value that is dereferenced from this variable value.
    ///
    /// This is a lightweight alternative to [`map`](Var::map) that can be used when the *mapped* value already
    /// exist in the source variable, `map` is called every time the mapped value is accessed.
    #[inline]
    fn map_ref<O, M>(&self, map: M) -> MapRefVar<T, O, M, Self>
    where
        O: VarValue,
        M: Fn(&T) -> &O + Clone + 'static,
    {
        MapRefVar::new(self.clone(), map)
    }

    /// Create a read-write variable with a value that is mapped from and to this variable.
    ///
    /// The value of the map variable is kept up-to-date with the value of this variable, `map` is
    /// called every time the value needs to update. When the mapped variable is assigned, `map_back` is
    /// called to generate a value that is assigned back to this variable.
    ///
    /// Also see [`bind_bidi`](Var::bind_bidi) to create a *map binding* between two existing variables.
    #[inline]
    fn map_bidi<O, M, N>(&self, map: M, map_back: N) -> RcMapBidiVar<T, O, M, N, Self>
    where
        O: VarValue,
        M: FnMut(&T) -> O + 'static,
        N: FnMut(O) -> T + 'static,
    {
        RcMapBidiVar::new(self.clone(), map, map_back)
    }

    /// Create a read-write variable with a value that is dereferenced from this variable value.
    ///
    /// This is a lightweight alternative to [`map_bidi`](Var::map_bidi) that can be used when the *mapped* value already
    /// exist in the source variable, `map` is called every time the mapped value is accessed and `map_mut` is called
    /// to get a mutable reference to the value when the mapped variable is assigned.
    #[inline]
    fn map_bidi_ref<O, M, N>(&self, map: M, map_mut: N) -> MapBidiRefVar<T, O, M, N, Self>
    where
        O: VarValue,
        M: Fn(&T) -> &O + Clone + 'static,
        N: Fn(&mut T) -> &mut O + Clone + 'static,
    {
        MapBidiRefVar::new(self.clone(), map, map_mut)
    }

    /// Create a read-only variable with a value that is mapped from this variable, but only if it passes a filter.
    ///
    /// The value of the map variable is kept up-to-date with the value of this variable, `map` is called every
    /// time the value needs to update, if it returns `Some(T)` the mapped variable value updates.
    ///
    /// The `fallback_init` can be called once if the first call to `map` returns `None`, it must return a *fallback* initial value.
    ///
    /// Also see [`filter_bind`](Var::filter_bind) to create a *map binding* between two existing variables.
    #[inline]
    fn filter_map<O, I, M>(&self, fallback_init: I, map: M) -> RcFilterMapVar<T, O, I, M, Self>
    where
        O: VarValue,
        I: FnOnce(&T) -> O + 'static,
        M: FnMut(&T) -> Option<O> + 'static,
    {
        RcFilterMapVar::new(self.clone(), fallback_init, map)
    }

    /// Create a read-write variable with a value that is mapped from and to this variable, but only if the values pass the filters.
    ///
    /// The value of the map variable is kept up-to-date with the value of this variable, `map` is
    /// called every time the value needs to update, if it returns `Some(T)` the mapped variable value updates.
    ///
    /// When the mapped variable is assigned, `map_back` is called, if it returns `Some(T)` the value is assigned back to this variable.
    ///
    /// Also see [`filter_bind_bidi`](Var::filter_bind_bidi) to create a *map binding* between two existing variables.
    #[inline]
    fn filter_map_bidi<O, I, M, N>(&self, fallback_init: I, map: M, map_back: N) -> RcFilterMapBidiVar<T, O, I, M, N, Self>
    where
        O: VarValue,
        I: FnOnce(&T) -> O + 'static,
        M: FnMut(&T) -> Option<O> + 'static,
        N: FnMut(O) -> Option<T> + 'static,
    {
        RcFilterMapBidiVar::new(self.clone(), fallback_init, map, map_back)
    }

    /// Creates a sender that can set `self` from other threads and without access to [`Vars`].
    ///
    /// If the variable is read-only when a value is received it is silently dropped.
    ///
    /// Drop the sender to release one reference to `self`.
    #[inline]
    fn sender<Vw>(&self, vars: &Vw) -> VarSender<T>
    where
        T: Send,
        Vw: WithVars,
    {
        vars.with_vars(|vars| vars.sender(self))
    }

    /// Creates a sender that modify `self` from other threads and without access to [`Vars`].
    ///
    /// If the variable is read-only when a modification is received it is silently dropped.
    ///
    /// Drop the sender to release one reference to `self`.
    #[inline]
    fn modify_sender<Vw: WithVars>(&self, vars: &Vw) -> VarModifySender<T> {
        vars.with_vars(|vars| vars.modify_sender(self))
    }

    /// Creates a channel that can receive `var` updates from another thread.
    ///
    /// Every time the variable updates a clone of the value is sent to the receiver. The current value is sent immediately.
    ///
    /// Drop the receiver to release one reference to `var`.
    #[inline]
    fn receiver<Vr>(&self, vars: &Vr) -> VarReceiver<T>
    where
        T: Send,
        Vr: WithVarsRead,
    {
        vars.with(|vars| vars.receiver(self))
    }

    /// Create a [`map`](Var::map) like binding between two existing variables.
    ///
    /// The binding flows from `self` to `to_var`, every time `self` updates `map` is called to generate a value that is assigned `to_var`.
    ///
    /// Both `self` and `to_var` notify a new value in the same app update, this is different then a manually implemented *binding*
    /// where the assign to `to_var` would cause a second update.
    #[inline]
    fn bind<Vw, T2, V2, M>(&self, vars: &Vw, to_var: &V2, mut map: M) -> VarBindingHandle
    where
        Vw: WithVars,
        T2: VarValue,
        V2: Var<T2>,
        M: FnMut(&VarBinding, &T) -> T2 + 'static,
    {
        vars.with_vars(|vars| {
            let to_var = to_var.clone();
            vars.bind_one(self, move |vars, info, from_var| {
                let new_value = map(info, from_var.get(vars));
                let _ = to_var.set(vars, new_value);
            })
        })
    }

    /// Create a [`map_bidi`](Var::map_bidi) like binding between two existing variables.
    ///
    /// The bindings **maps** from `self` to `other_var` and **maps-back** from `other_var` to `self`.
    /// Every time `self` updates `map` is called to generate a value that is assigned to `other_var` and every time `other_var`
    /// updates `map_back` is called to generate a value that is assigned back to `self`.
    ///
    /// Both `self` and `other_var` notify a new value in the same app update, this is different then a manually implemented *binding*
    /// when the assign to the second variable would cause a second update.
    #[inline]
    fn bind_bidi<Vw, T2, V2, M, N>(&self, vars: &Vw, other_var: &V2, mut map: M, mut map_back: N) -> VarBindingHandle
    where
        Vw: WithVars,
        T2: VarValue,
        V2: Var<T2>,
        M: FnMut(&VarBinding, &T) -> T2 + 'static,
        N: FnMut(&VarBinding, &T2) -> T + 'static,
    {
        vars.with_vars(|vars| {
            vars.bind_two(self, other_var, move |vars, info, from_var, to_var| {
                if let Some(new_value) = from_var.get_new(vars) {
                    let new_value = map(&info, new_value);
                    let _ = to_var.set(vars, new_value);
                }
                if let Some(new_value) = to_var.get_new(vars) {
                    let new_value = map_back(&info, new_value);
                    let _ = from_var.set(vars, new_value);
                }
            })
        })
    }

    /// Create a [`filter_map`](Var::filter_map) like binding between two existing variables.
    ///
    /// The binding flows from `self` to `to_var`, every time `self` updates `map` is called to generate a value, if it does, that value
    /// is assigned `to_var`.
    ///
    /// Both `self` and `to_var` notify a new value in the same app update, this is different then a manually implemented *binding*
    /// where the assign to `to_var` would cause a second update.
    #[inline]
    fn filter_bind<Vw, T2, V2, M>(&self, vars: &Vw, to_var: &V2, mut map: M) -> VarBindingHandle
    where
        Vw: WithVars,
        T2: VarValue,
        V2: Var<T2>,
        M: FnMut(&VarBinding, &T) -> Option<T2> + 'static,
    {
        vars.with_vars(|vars| {
            let to_var = to_var.clone();
            vars.bind_one(self, move |vars, info, from_var| {
                if let Some(new_value) = map(&info, from_var.get(vars)) {
                    let _ = to_var.set(vars, new_value);
                }
            })
        })
    }

    /// Create a [`filter_map_bidi`](Var::filter_map_bidi) like binding between two existing variables.
    ///
    /// The bindings **maps** from `self` to `other_var` and **maps-back** from `other_var` to `self`.
    /// Every time `self` updates `map` is called to generate a value that is assigned to `other_var` and every time `other_var`
    /// updates `map_back` is called to generate a value that is assigned back to `self`. In both cases the second variable only
    /// updates if the map function returns a value.
    ///
    /// Both `self` and `other_var` notify a new value in the same app update, this is different then a manually implemented *binding*
    /// when the assign to the second variable would cause a second update.
    #[inline]
    fn filter_bind_bidi<Vw, T2, V2, M, N>(&self, vars: &Vw, other_var: &V2, mut map: M, mut map_back: N) -> VarBindingHandle
    where
        Vw: WithVars,
        T2: VarValue,
        V2: Var<T2>,
        M: FnMut(&VarBinding, &T) -> Option<T2> + 'static,
        N: FnMut(&VarBinding, &T2) -> Option<T> + 'static,
    {
        vars.with_vars(|vars| {
            vars.bind_two(self, other_var, move |vars, info, from_var, to_var| {
                if let Some(new_value) = from_var.get_new(vars) {
                    if let Some(new_value) = map(info, new_value) {
                        let _ = to_var.set(vars, new_value);
                    }
                }
                if let Some(new_value) = to_var.get_new(vars) {
                    if let Some(new_value) = map_back(&info, new_value) {
                        let _ = from_var.set(vars, new_value);
                    }
                }
            })
        })
    }
}

/// Argument for [`Var::modify`]. This is a wrapper around a mutable reference to the variable value, if
/// [`DerefMut`] is used to get the variable value the variable value is flagged as *new*.
pub struct VarModify<'a, T: VarValue> {
    value: &'a mut T,
    touched: bool,
}
impl<'a, T: VarValue> VarModify<'a, T> {
    /// New wrapper.
    pub fn new(value: &'a mut T) -> Self {
        VarModify { value, touched: false }
    }

    /// If `deref_mut` was used or [`touch`](Self::touch) was called.
    #[inline]
    pub fn touched(&self) -> bool {
        self.touched
    }

    /// Flags the value as modified.
    #[inline]
    pub fn touch(&mut self) {
        self.touched = true;
    }

    /// Runs `modify` with a mutable reference `B` derived from `T` using `map`.
    /// Only flag touched if `modify` touches the the value.
    ///
    /// This method does permit modifying the value without flagging the value as new, this is not `unsafe`
    /// but is an error that will the variable dependents to go out of sync.
    pub fn map_ref<B, M, Mo>(&mut self, map: M, modify: Mo)
    where
        B: VarValue,
        M: Fn(&mut T) -> &mut B,
        Mo: FnOnce(&mut VarModify<B>),
    {
        let mut mapped = VarModify {
            value: map(self.value),
            touched: false,
        };

        modify(&mut mapped);

        self.touched |= mapped.touched;
    }
}
impl<'a, T: VarValue> Deref for VarModify<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}
impl<'a, T: VarValue> DerefMut for VarModify<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.touched = true;
        self.value
    }
}

/// New [`impl Var<T>`](Var) from an expression with interpolated *vars*.
///
/// # Interpolation
///
/// Other variables can be interpolated by quoting the var with `#{..}`. When
/// an expression contains other interpolated vars the expression var updates when
/// any of the interpolated vars update.
///
/// # Example
///
/// ```
/// # use zero_ui_core::var::*;
/// let var_a = var(10);
/// let var_b = var(10);
/// let name = "var_eq";
/// let var_eq = expr_var! {
///     let eq = #{var_a} == #{var_b};
///     println!("{} updated: {} == {}: {}", name, #{var_a}, #{var_b}, eq);
///     eq
/// };
/// ```
///
/// In the example a `var_eq` of type `impl Var<bool>` is created. When either `var_a` or `var_b` are set
/// the value of `var_eq` is updated on the next read. Normal variables like `name` are moved in, like a closure capture.
///
/// # Capture Mode
///
/// The expression operates like a closure that captures by `move`. Both the interpolated variables and any
/// other `let` binding referenced from the scope are moved into the resulting variable.
///
/// # Interpolation
///
/// Variable interpolation is done by quoting the variable with `#{<var-expr>}`, the braces are required.
///
/// The `<var-expr>` is evaluated before *capturing* starts so if you interpolate `#{var_a.clone()}` `var_a`
/// will still be available after the `var_expr` call. Equal `<var-expr>` only evaluate once.
///
/// The interpolation result value is the [`Var::get`] return value.
///
/// # Expansion
///
/// The expression is transformed into different types of vars depending on the number of interpolated variables.
///
/// ## No Variables
///
/// An expression with no interpolation is simply evaluated into a var using [`IntoVar`].
///
/// # Single Variable
///
/// An expression with a single variable is transformed in a [`map`](Var::map) operation, unless the expression
/// is only the variable without any extra operation.
///
/// # Multiple Variables
///
/// An expression with multiple variables is transformed into a [`merge_var!`] call.
#[macro_export]
macro_rules! expr_var {
    ($($expr:tt)+) => {
        $crate::var::__expr_var! { $crate::var, $($expr)+ }
    };
}
#[doc(inline)]
pub use crate::expr_var;

#[doc(hidden)]
pub use zero_ui_proc_macros::expr_var as __expr_var;
