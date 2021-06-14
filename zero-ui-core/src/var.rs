//! Variables.

use std::{
    fmt,
    ops::{Deref, DerefMut},
};

mod vars;
pub use vars::*;

mod boxed;
pub use boxed::*;

mod cloning_local;
pub use cloning_local::*;

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
    fn is_read_only(&self, vars: &Vars) -> bool;

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

    /// Causes the variable to notify update without changing the value.
    #[inline]
    fn touch(&self, vars: &Vars) -> Result<(), VarIsReadOnly> {
        self.modify(vars, |v| v.touch())
    }

    /// Schedule a new value for the variable.
    #[inline]
    fn set<N>(&self, vars: &Vars, new_value: N) -> Result<(), VarIsReadOnly>
    where
        N: Into<T>,
    {
        let new_value = new_value.into();
        self.modify(vars, move |v| **v = new_value)
    }

    /// Schedule a new value for the variable, but only if the current value is not equal to `new_value`.
    #[inline]
    fn set_ne<N>(&self, vars: &Vars, new_value: N) -> Result<bool, VarIsReadOnly>
    where
        N: Into<T>,
        T: PartialEq,
    {
        if self.is_read_only(vars) {
            Err(VarIsReadOnly)
        } else {
            let new_value = new_value.into();
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

    /// Create a variable who's value is always a mapped reference to this variable's value.
    ///
    /// Every time the output variable probes the value, `map` is called to project the reference.
    ///
    /// The output variable is always read-only.
    #[inline]
    fn map_ref<O, M>(&self, map: M) -> MapRefVar<T, O, M, Self>
    where
        O: VarValue,
        M: Fn(&T) -> &O + Clone + 'static,
    {
        self.clone().into_map_ref(map)
    }

    /// Create a [map](Self::map) variable consuming this variable.
    ///
    /// If `self` is a *reference* var like `RcVar` this can avoid a clone.
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

    /// Create a [reference map](Self::map_ref) variable consuming this variable.
    ///
    /// If `self` is a *reference* var like `RcVar` this can avoid a clone.
    ///
    /// The output variable is always read-only.
    #[inline]
    fn into_map_ref<O, M>(self, map: M) -> MapRefVar<T, O, M, Self>
    where
        O: VarValue,
        M: Fn(&T) -> &O + Clone + 'static,
    {
        MapRefVar::new(self, map)
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

    /// Create a variable who's value is always a mapped reference to this variable's value.
    ///
    /// Every time the output variable probes the value, `map` is called to project the reference and
    /// every time the output variable is assigned or modified, `map_mut` is called to project a mutable reference.
    ///
    /// Modifying the value in `map_mut` is a logic error, see [`VarModify::map_ref`] for details.
    #[inline]
    fn map_bidi_ref<O, M, N>(&self, map: M, map_mut: N) -> MapBidiRefVar<T, O, M, N, Self>
    where
        O: VarValue,
        M: Fn(&T) -> &O + Clone + 'static,
        N: Fn(&mut T) -> &mut O + Clone + 'static,
    {
        self.clone().into_map_bidi_ref(map, map_mut)
    }

    /// Create a [bidirectional map](Self::map_bidi) variable consuming this variable.
    ///
    /// If `self` is a *reference* var like `RcVar` this can avoid a clone.
    #[inline]
    fn into_map_bidi<O, M, N>(self, map: M, map_back: N) -> RcMapBidiVar<T, O, M, N, Self>
    where
        O: VarValue,
        M: FnMut(&T) -> O + 'static,
        N: FnMut(O) -> T + 'static,
    {
        RcMapBidiVar::new(self, map, map_back)
    }

    /// Create a [bidirectional reference map](Self::map_bidi_ref) variable consuming this variable.
    ///
    /// If `self` is a *reference* var like `RcVar` this can avoid a clone.
    #[inline]
    fn into_map_bidi_ref<O, M, N>(self, map: M, map_mut: N) -> MapBidiRefVar<T, O, M, N, Self>
    where
        O: VarValue,
        M: Fn(&T) -> &O + Clone + 'static,
        N: Fn(&mut T) -> &mut O + Clone + 'static,
    {
        MapBidiRefVar::new(self, map, map_mut)
    }

    /// Create a variable who's value is generated by this variable's value but it only updates in some cases.
    ///
    /// Every time the output variable probes the value and its version is not the same as this variables version `map` is
    /// called, if it returns `Some(O)` the variable also indicates a new value.
    ///
    /// The `fallback_init` closure is used to provide the first value if the first value of `self` is filtered out.
    ///
    /// The output variable is always read-only.
    #[inline]
    fn filter_map<O, I, M>(&self, fallback_init: I, map: M) -> RcFilterMapVar<T, O, I, M, Self>
    where
        O: VarValue,
        I: FnOnce(&T) -> O + 'static,
        M: FnMut(&T) -> Option<O> + 'static,
    {
        self.clone().into_filter_map(fallback_init, map)
    }

    /// Create a [filtering map](Self::filter_map) variable consuming this variable.
    ///
    /// If `self` is a *reference* var like `RcVar` this can avoid a clone.
    #[inline]
    fn into_filter_map<O, I, M>(self, fallback_init: I, map: M) -> RcFilterMapVar<T, O, I, M, Self>
    where
        I: FnOnce(&T) -> O + 'static,
        O: VarValue,
        M: FnMut(&T) -> Option<O> + 'static,
    {
        RcFilterMapVar::new(self, fallback_init, map)
    }

    /// Create a [filtering bidirectional map](Self::filter_map_bidi) variable consuming this variable.
    ///
    /// If `self` is a *reference* var like `RcVar` this can avoid a clone.
    #[inline]
    fn filter_map_bidi<O, I, M, N>(&self, fallback_init: I, map: M, map_back: N) -> RcFilterMapBidiVar<T, O, I, M, N, Self>
    where
        O: VarValue,
        I: FnOnce(&T) -> O + 'static,
        M: FnMut(&T) -> Option<O> + 'static,
        N: FnMut(O) -> Option<T> + 'static,
    {
        self.clone().into_filter_map_bidi(fallback_init, map, map_back)
    }

    /// Create a [filtering bidirectional map](Self::filter_map_bidi) variable consuming this variable.
    ///
    /// If `self` is a *reference* var like `RcVar` this can avoid a clone.
    #[inline]
    fn into_filter_map_bidi<O, I, M, N>(self, fallback_init: I, map: M, map_back: N) -> RcFilterMapBidiVar<T, O, I, M, N, Self>
    where
        O: VarValue,
        I: FnOnce(&T) -> O + 'static,
        M: FnMut(&T) -> Option<O> + 'static,
        N: FnMut(O) -> Option<T> + 'static,
    {
        RcFilterMapBidiVar::new(self, fallback_init, map, map_back)
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
