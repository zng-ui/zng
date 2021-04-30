//! Variables.

use std::{
    cell::RefCell,
    cell::{Cell, UnsafeCell},
    fmt::Debug,
    marker::PhantomData,
    mem::MaybeUninit,
    rc::Rc,
    thread::LocalKey,
};

mod boxed_var;
pub use boxed_var::*;

mod owned_var;
pub use owned_var::*;

mod rc_var;
pub use rc_var::*;

mod force_read_only_var;
pub use force_read_only_var::*;

mod cloning_local_var;
pub use cloning_local_var::*;

mod rc_map_var;
pub use rc_map_var::*;

mod map_ref_var;
pub use map_ref_var::*;

mod map_bidi_ref_var;
pub use map_bidi_ref_var::*;

mod rc_map_bidi_var;
pub use rc_map_bidi_var::*;

mod context_var;
pub use context_var::*;

mod rc_merge_var;
pub use rc_merge_var::*;

mod rc_switch_var;
pub use rc_switch_var::*;

mod rc_when_var;
pub use rc_when_var::*;

mod vars;
pub use vars::*;

/// A type that can be a [`Var`](crate::var::Var) value.
///
/// # Trait Alias
///
/// This trait is used like a type alias for traits and is
/// already implemented for all types it applies to.
pub trait VarValue: Debug + Clone + 'static {}
impl<T: Debug + Clone + 'static> VarValue for T {}

/// Type Id if a contextual variable.
pub trait ContextVar: Clone + Copy + 'static {
    /// The variable type.
    type Type: VarValue;

    /// Default value, used when the variable is not set in a context.
    fn default_value() -> &'static Self::Type;

    /// Gets the variable.
    fn var() -> &'static ContextVarProxy<Self>;

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

mod protected {
    /// Ensures that only `zero-ui` can implement var types.
    pub trait Var {}
}

/// Part of [`Var`] that can be boxed.
pub trait VarObj<T: VarValue>: protected::Var + 'static {
    /// References the current value.
    fn get<'a>(&'a self, vars: &'a VarsRead) -> &'a T;

    /// References the current value if it [is new](Self::is_new).
    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a T>;

    /// If [`set`](Self::set) or [`modify`](Var::modify) where called in the previous update.
    ///
    /// When you set the variable, the new value is only applied after the UI tree finishes
    /// the current update. The value is then applied causing a new update to happen, in the new
    /// update this method returns `true`. After the new update it returns `false` again.
    ///
    /// The new value can still be equal to the previous value, the variable does not check equality on assign.
    fn is_new(&self, vars: &Vars) -> bool;

    /// Version of the current value.
    ///
    /// The version number changes every update where [`set`](Self::set) or [`modify`](Var::modify) are called.
    fn version(&self, vars: &VarsRead) -> u32;

    /// If the variable cannot be set.
    ///
    /// Variables can still change if [`can_update`](Self::can_update) is `true`.
    ///
    /// Some variables can stop being read-only after an update, see also [`always_read_only`](Self::always_read_only).
    fn is_read_only(&self, vars: &Vars) -> bool;

    /// If the variable type is read-only, unlike [`is_read_only`](Self::is_read_only) this never changes.
    fn always_read_only(&self) -> bool;

    /// If the variable type allows the value to change.
    ///
    /// Some variables can change even if they are read-only, for example mapping variables.
    fn can_update(&self) -> bool;

    /// Schedules an assign for after the current update.
    ///
    /// Variables are not changed immediately, the full UI tree gets a chance to see the current value,
    /// after the current UI update, the values set here are applied.
    ///
    /// If the result is `Ok` the variable will be flagged as [new](Self::is_new) in the next update. Value
    /// equality is not checked, setting to an equal value still flags a *new*.
    ///
    /// ### Error
    ///
    /// Returns [`VarIsReadOnly`] if [`is_read_only`](Self::is_read_only) is `true`.
    fn set(&self, vars: &Vars, new_value: T) -> Result<(), VarIsReadOnly>;

    /// Boxed version of the [`modify`](Var::modify) method.
    fn modify_boxed(&self, vars: &Vars, change: Box<dyn FnOnce(&mut T)>) -> Result<(), VarIsReadOnly>;

    /// Boxes `self`.
    ///
    /// A boxed var is also a var, that implementation just returns `self`.
    fn boxed(self) -> BoxedVar<T>
    where
        Self: Sized,
    {
        Box::new(self)
    }
}

/// Represents a variable that has a value that can be accessed directly.
///
/// For the normal variables you need a reference to [`Vars`] to access the value,
/// this reference is not available in all [`UiNode`](crate::UiNode) methods.
///
/// Some variable types are safe to reference the inner value at any moment, other variables
/// can be wrapped in a type that makes a local clone of the current value. You can get any
/// variable as a local variable by calling [`Var::into_local`].
pub trait VarLocal<T: VarValue>: VarObj<T> {
    /// Reference the value.
    fn get_local(&self) -> &T;

    /// Initializes local clone of the value, if needed.
    ///
    /// This must be called in the [`UiNode::init`](crate::UiNode::init) method.
    ///
    /// Returns a reference to the local value for convenience.
    fn init_local(&mut self, vars: &Vars) -> &T;

    /// Updates the local clone of the value, if needed.
    ///
    /// This must be called in the [`UiNode::update`](crate::UiNode::update) method.
    ///
    /// Returns a reference to the local value if the value is new.
    fn update_local(&mut self, vars: &Vars) -> Option<&T>;

    /// Boxes `self`.
    fn boxed_local(self) -> BoxedLocalVar<T>
    where
        Self: Sized,
    {
        Box::new(self)
    }
}

/// Represents a variable.
///
/// Most of the methods are declared in the [`VarObj`] trait to support boxing.
pub trait Var<T: VarValue>: VarObj<T> + Clone + IntoVar<T> {
    /// Return type of [`into_read_only`](Var::into_read_only).
    type AsReadOnly: Var<T>;

    /// Return type of [`into_local`](Var::into_local).
    type AsLocal: VarLocal<T>;

    /// Schedules a closure to modify the value after the current update.
    ///
    /// This is a variation of the [`set`](VarObj::set) method that does not require
    /// an entire new value to be instantiated.
    ///
    /// If the result is `Ok` the variable will be flagged as [new](VarObj::is_new) in the next update,
    /// even if `change` does not do anything.
    fn modify<F: FnOnce(&mut T) + 'static>(&self, vars: &Vars, change: F) -> Result<(), VarIsReadOnly>;

    /// Returns the variable as a type that is [`always_read_only`](VarObj::always_read_only).
    fn into_read_only(self) -> Self::AsReadOnly;

    /// Returns the variable as a type that implements [`VarLocal`].
    fn into_local(self) -> Self::AsLocal;

    /// Returns a variable with value generated from `self`.
    ///
    /// The value is new when the `self` value is new, `map` is only called once per new value.
    ///
    /// The variable is read-only, use [`map_bidi`](Self::map_bidi) to propagate changes back to `self`.
    ///
    /// Use [`map_ref`](Self::map_ref) if you don't need to generate a new value.
    ///
    /// Use [`into_map`](Self::into_map) if you will not use this copy of `self` anymore.
    fn map<O: VarValue, F: FnMut(&T) -> O + 'static>(&self, map: F) -> RcMapVar<T, O, Self, F>;

    /// Returns a variable with value referenced from `self`.
    ///
    /// The value is new when the `self` value is new, `map` is called every time [`get`](VarObj::get) is called.
    ///
    /// The variable is read-only.
    ///
    /// Use [`into_map_ref`](Self::into_map_ref) if you will not use this copy of `self` anymore.
    fn map_ref<O: VarValue, F: Fn(&T) -> &O + Clone + 'static>(&self, map: F) -> MapRefVar<T, O, Self, F>;

    /// Returns a variable whos value is mapped to and from `self`.
    ///
    /// The value is new when the `self` value is new, `map` is only called once per new value.
    ///
    /// The variable can be set if `self` is not read-only, when set `map_back` is called to generate
    /// a new value for `self`.
    ///
    /// Use [`map_bidi_ref`](Self::map_bidi_ref) if you don't need to generate a new value.
    ///
    /// Use [`into_map_bidi`](Self::into_map_bidi) if you will not use this copy of `self` anymore.
    fn map_bidi<O: VarValue, F: FnMut(&T) -> O + 'static, G: FnMut(O) -> T + 'static>(
        &self,
        map: F,
        map_back: G,
    ) -> RcMapBidiVar<T, O, Self, F, G>;

    /// Returns a variable with value mapped to and from `self` using references.
    ///
    /// The value is new when the `self` value is new, `map` is called every time [`get`](VarObj::get) is called,
    /// `map_mut` is called every time the value is set or modified.
    ///
    /// Use [`into_map`](Self::into_map) if you will not use this copy of `self` anymore.
    fn map_bidi_ref<O: VarValue, F: Fn(&T) -> &O + Clone + 'static, G: Fn(&mut T) -> &mut O + Clone + 'static>(
        &self,
        map: F,
        map_mut: G,
    ) -> MapBidiRefVar<T, O, Self, F, G>;

    /// Taking variant of [`map`](Self::map).
    fn into_map<O: VarValue, F: FnMut(&T) -> O + 'static>(self, map: F) -> RcMapVar<T, O, Self, F>;

    /// Taking variant of [`map_ref`](Self::map_ref).
    fn into_map_ref<O: VarValue, F: Fn(&T) -> &O + Clone + 'static>(self, map: F) -> MapRefVar<T, O, Self, F>;

    /// Taking variant of [`map_bidi`](Self::map_bidi).
    fn into_map_bidi<O: VarValue, F: FnMut(&T) -> O + 'static, G: FnMut(O) -> T + 'static>(
        self,
        map: F,
        map_back: G,
    ) -> RcMapBidiVar<T, O, Self, F, G>;

    /// Taking variant of [`map_bidi_ref`](Self::map_bidi_ref).
    fn into_map_bidi_ref<O: VarValue, F: Fn(&T) -> &O + Clone + 'static, G: Fn(&mut T) -> &mut O + Clone + 'static>(
        self,
        map: F,
        map_mut: G,
    ) -> MapBidiRefVar<T, O, Self, F, G>;
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
/// The interpolation result value is the [`VarObj::get`] return value.
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
