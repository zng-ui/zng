use std::{
    fmt,
    ops::{Deref, DerefMut},
};

mod vars;
pub use vars::*;

mod boxed_var;
pub use boxed_var::*;

mod cloning_local_var;
pub use cloning_local_var::*;

mod context_var;
pub use context_var::*;

mod read_only_var;
pub use read_only_var::*;

mod owned_var;
pub use owned_var::*;

mod rc_var;
pub use rc_var::*;

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

    /// Shortcut call `self.into_var().into_local()`.
    fn into_local(self) -> <<Self as IntoVar<T>>::Var as Var<T>>::AsLocal
    where
        Self: Sized,
    {
        Var::into_local(self.into_var())
    }

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
    /// The variable type that represents a version of this type that provides direct access
    /// to its value without a [`VarsRead`] reference.
    type AsLocal: VarLocal<T>;

    // TODO when GATs are stable:
    // type Map<B: VarValue, M: FnMut(&T) -> B> : Var<B>;
    // type MapBidi<B: VarValue, M: FnMut(&T) -> B, N: FnMut(&B) -> T> : Var<B>;

    /// References the value.
    fn get<'a>(&'a self, vars: &'a VarsRead) -> &'a T;

    /// References the value if [`is_new`](Self::is_new).
    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a T>;

    /// If the variable value changed in this update.
    ///
    /// When the variable value changes this stays `true` for one app update cycle.
    #[inline]
    fn is_new(&self, vars: &Vars) -> bool {
        self.get_new(vars).is_some()
    }

    /// Gets the variable value version.
    ///
    /// The version is a different number every time the value is modified, you can use this to monitor
    /// variable change outside of the window of opportunity of [`is_new`](Self::is_new).
    fn version(&self, vars: &VarsRead) -> u32;

    /// If the variable cannot be set or modified right now.
    ///
    /// **Note** this can change unless the variable is [`always_read_only`](Self::always_read_only).
    fn is_read_only(&self, vars: &VarsRead) -> bool;

    /// If the variable can never be set or modified.
    ///
    /// **Note** the value still be new by an internal change if [`can_update`](Self::can_update) is `true`.
    fn always_read_only(&self) -> bool;

    /// If the variable value can change.
    ///
    /// **Note** this can be `true` even if the variable is [`always_read_only`](Self::always_read_only).
    fn can_update(&self) -> bool;

    /// Schedule a modification of the variable value.
    ///
    /// The variable is marked as *new* only if the closure input is dereferenced as `mut`.
    fn modify<M>(&self, vars: &Vars, modify: M) -> Result<(), VarIsReadOnly>
    where
        M: FnOnce(&mut VarModify<T>) + 'static;

    /// Schedule a new value for the variable.
    #[inline]
    fn set(&self, vars: &Vars, new_value: T) -> Result<(), VarIsReadOnly> {
        self.modify(vars, move |v| **v = new_value)
    }

    /// Schedule a new value for the variable, the value is checked for equality before assign
    /// and the variable is flagged as *new* only if the value is actually different.
    #[inline]
    fn set_ne(&self, vars: &Vars, new_value: T) -> Result<(), VarIsReadOnly>
    where
        T: PartialEq,
    {
        self.modify(vars, move |v| {
            if v.eq(&new_value) {
                **v = new_value;
            }
        })
    }

    /// Convert this variable to one that cannot be set or modified.
    fn into_read_only(self) -> Self::AsReadOnly;

    /// Convert this variable to one that provides direct access to the current value.
    fn into_local(self) -> Self::AsLocal;

    /// Create a variable who's value is always generated by this variable's value.
    ///
    /// Every time the output variable probes the value and its version is not the same as this variables version `map` is
    /// called to generate a new value.
    ///
    /// The output variable is always read-only.
    #[inline]
    fn map<O, M>(&self, map: M) -> RcMapVar<T, O, M, Self>
    where
        O: VarValue,
        M: FnMut(&T) -> O + 'static,
    {
        self.clone().into_map(map)
    }

    /// Create a [map](Self::map) variable consuming this variable.
    ///
    /// If `self` is a *reference* var line `RcVar` this can avoid a clone.
    ///
    /// The output variable is always read-only.
    #[inline]
    fn into_map<O, M>(self, map: M) -> RcMapVar<T, O, M, Self>
    where
        O: VarValue,
        M: FnMut(&T) -> O + 'static,
    {
        RcMapVar::new(self, map)
    }

    /// Create a variable who's value is always generated by this variable's value and that sets this variable
    /// back when it is set or modified (bidirectional).
    ///
    /// Every time the output variable probes the value and its version is not the same as this variables version `map` is
    /// called to generate a new value; and every time the output value is set or modified `map_back` is called to generate
    /// a new value that is set in the input variable(`self`).
    ///
    /// The output variable can be read-only if `self` is read-only.
    #[inline]
    fn map_bidi<O, M, N>(&self, map: M, map_back: N) -> RcMapBidiVar<T, O, M, N, Self>
    where
        O: VarValue,
        M: FnMut(&T) -> O + 'static,
        N: FnMut(O) -> T + 'static,
    {
        self.clone().into_map_bidi(map, map_back)
    }

    /// Create a [bidirectional map](Self::map_bidi) variable consuming this variable.
    ///
    /// If `self` is a *reference* var line `RcVar` this can avoid a clone.
    #[inline]
    fn into_map_bidi<O, M, N>(self, map: M, map_back: N) -> RcMapBidiVar<T, O, M, N, Self>
    where
        O: VarValue,
        M: FnMut(&T) -> O + 'static,
        N: FnMut(O) -> T + 'static,
    {
        RcMapBidiVar::new(self, map, map_back)
    }

    /// Box this var.
    #[inline]
    fn boxed(self) -> BoxedVar<T>
    where
        Self: VarBoxed<T> + Sized,
    {
        Box::new(self)
    }
}

/// Argument for [`Var::modify`]. This is a wrapper around a mutable reference to the variable value, if
/// [`DerefMut`] is used to get the variable value the variable value is flagged as *new*.
pub struct VarModify<'a, T: VarValue> {
    value: &'a mut T,
    modified: bool,
}
impl<'a, T: VarValue> VarModify<'a, T> {
    /// New wrapper.
    pub fn new(value: &'a mut T) -> Self {
        VarModify { value, modified: false }
    }

    /// If `deref_mut` was used or [`touch`](Self::touch) was called.
    #[inline]
    pub fn touched(&self) -> bool {
        self.modified
    }

    /// Flags the value as modified.
    #[inline]
    pub fn touch(&mut self) {
        self.modified = true;
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
        self.modified = true;
        self.value
    }
}

/// A [`Var`] that provide direct access to its value without holding a [`Vars`] reference.
///
/// This is only possible if the value is local, so variable with shared values
/// will keep a clone of the value locally if converted to [`VarLocal`].
pub trait VarLocal<T: VarValue>: Var<T> {
    /// Reference the current value.
    fn get_local(&self) -> &T;

    /// Initializes local clone of the value, if needed.
    ///
    /// This must be called in the [`UiNode::init`](crate::UiNode::init) method.
    ///
    /// Returns a reference to the local value for convenience.
    fn init_local<'a>(&'a mut self, vars: &'a Vars) -> &'a T;

    /// Updates the local clone of the value, if needed.
    ///
    /// This must be called in the [`UiNode::update`](crate::UiNode::update) method.
    ///
    /// Returns a reference to the local value if the value is new.
    fn update_local<'a>(&'a mut self, vars: &'a Vars) -> Option<&'a T>;
}
