//! Variables.

use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    fmt,
    rc::Rc,
};

use crate::handler::{AppHandler, app_hn};

pub mod animation;
mod boxed;
mod channel;
mod context;
mod contextualized;
mod cow;
mod expr;
mod flat_map;
mod future;
mod local;
mod merge;
mod rc;
mod read_only;
mod response;
mod state;
mod tests;
mod util;
mod vars;
mod when;

pub use boxed::{BoxedAnyVar, BoxedAnyWeakVar, BoxedVar, BoxedWeakVar};
pub use channel::{response_channel, ResponseSender, VarModifySender, VarReceiver, VarSender};
pub use context::{context_var, with_context_var, with_context_var_init, ContextVar};
pub use expr::expr_var;
pub use local::LocalVar;
pub use merge::merge_var;
pub use rc::{var, var_from, RcVar};
pub use read_only::ReadOnlyRcVar;
pub use response::{response_done_var, response_var, ResponderVar, ResponseVar};
pub use state::*;
pub use util::*;
pub use vars::*;
pub use when::when_var;

use crate::{context::Updates, WidgetId};

/// Other variable types.
pub mod types {
    pub use super::boxed::{VarBoxed, WeakVarBoxed};
    pub use super::context::ContextData;
    pub use super::cow::{RcCowVar, WeakCowVar};
    pub use super::expr::__expr_var;
    pub use super::flat_map::{RcFlatMapVar, WeakFlatMapVar};
    pub use super::future::{WaitIsNewFut, WaitNewFut};
    pub use super::merge::{RcMergeVar, __merge_var, input_downcaster, WeakMergeVar};
    pub use super::rc::WeakRcVar;
    pub use super::read_only::{ReadOnlyVar, WeakReadOnlyVar};
    pub use super::response::Response;
    pub use super::when::{AnyWhenVarBuilder, RcWhenVar, WeakWhenVar, WhenVarBuilder, __when_var};
    pub use super::contextualized::{ContextualizedVar, WeakContextualizedVar};
}

/// A type that can be a [`Var<T>`] value.
///
/// # Trait Alias
///
/// This trait is used like a type alias for traits and is
/// already implemented for all types it applies to.
pub trait VarValue: fmt::Debug + Clone + Any {}
impl<T: fmt::Debug + Clone + Any> VarValue for T {}

/// Trait implemented for all [`VarValue`] types.
pub trait AnyVarValue: fmt::Debug + Any {
    /// Access to `dyn Any` methods.
    fn as_any(&self) -> &dyn Any;

    /// Access to `Box<dyn Any>` methods.
    fn into_any(self: Box<Self>) -> Box<dyn Any>;

    /// Clone the value.
    fn clone_boxed(&self) -> Box<dyn AnyVarValue>;
}

impl<T: VarValue> AnyVarValue for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_boxed(&self) -> Box<dyn AnyVarValue> {
        Box::new(self.clone())
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

/// A property value that is not a variable but can be inspected.
///
/// Property inputs are usually of the type `impl IntoVar<T>` because most properties can handle input updates, some
/// properties have a fixed value and can receive any other value type, a common pattern is receiving `impl Into<T>` in
/// this case, but values of this type cannot be [inspected], only the type name will show in the inspector.
///
/// Implementers can instead use `impl IntoValue<T>`, it represents a type that can be cloned and converted into a [`Debug`]
/// type that is the type expected by the property. In inspected builds this value is cloned and converted to the property type
/// to collect the debug strings.
///
/// # Examples
///
/// The example property receives two flags `a` and `b`, the inspector will show the value of `a` but only the type of `b`.
///
/// ```
/// # fn main() { }
/// # use zero_ui_core::{*, var::IntoValue};
/// #
/// #[property(context, allowed_in_when = false)]
/// pub fn foo(child: impl UiNode, a: impl IntoValue<bool>, b: impl Into<bool>) -> impl UiNode {
///     struct FooNode<C> {
///         child: C,
///         a: bool,
///         b: bool,
///     }
/// #    #[impl_ui_node(child)]
/// #    impl<C: UiNode> UiNode for FooNode<C> { }
///
///     FooNode {
///         child,
///         a: a.into(),
///         b: b.into()
///     }
/// }
/// ```
///
/// # Implementing
///
/// The trait is only auto-implemented for `T: Into<T> + Debug + Clone`, unfortunately actual type conversions
/// must be manually implemented, note that the [`impl_from_and_into_var!`] macro auto-implements this conversion.
///
/// [inspected]: crate::inspector
/// [`Debug`]: std::fmt::Debug
/// [`impl_from_and_into_var`]: crate::var::impl_from_and_into_var
pub trait IntoValue<T: fmt::Debug + Any>: Into<T> + Clone {}
impl<T: fmt::Debug + Clone + Any> IntoValue<T> for T {}

bitflags! {
    /// Kinds of interactions allowed by a [`Var<T>`] in the current update.
    ///
    /// You can get the current capabilities of a var by using the [`AnyVar::capabilities`] method.
    pub struct VarCapabilities: u8 {
        /// Var value can change.
        ///
        /// If this is set the [`Var::is_new`] can be `true` in some updates, a variable can `CHANGE`
        /// even if it cannot `MODIFY`, in this case the variable is a read-only wrapper on a read-write variable.
        const CHANGE = 0b0000_0010;

        /// Var can be modified.
        ///
        /// If this is set [`Var::modify`] does succeeds, if this is set `CHANGE` is also set.
        const MODIFY = 0b0000_0011;

        /// Var capabilities can change.
        ///
        /// Var capabilities can only change in between app updates, just like the var value, but [`AnyVar::last_update`]
        /// may not change when capability changes.
        const CAP_CHANGE = 0b1000_0000;
    }
}
impl VarCapabilities {
    /// Remove only the `MODIFY` flag without removing `CHANGE`.
    pub fn as_read_only(mut self) -> Self {
        self.bits &= 0b1111_1101;
        self
    }

    /// If cannot `MODIFY` and is not `CAP_CHANGE`.
    pub fn is_always_read_only(self) -> bool {
        !self.contains(Self::MODIFY) && !self.contains(Self::CAP_CHANGE)
    }

    /// If cannot `CHANGE` and is not `CAP_CHANGE`.
    pub fn is_always_static(self) -> bool {
        self.is_empty()
    }
}

/// Error when an attempt to modify a variable without the [`MODIFY`] capability is made.
///
/// [`MODIFY`]: VarCapabilities::MODIFY
#[derive(Debug, Clone, Copy)]
pub struct VarIsReadOnlyError {
    /// Variable capabilities when the request was made.
    pub capabilities: VarCapabilities,
}
impl fmt::Display for VarIsReadOnlyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cannot modify variable")
    }
}
impl std::error::Error for VarIsReadOnlyError {}

/// Represents a variable value in the [`Var::modify`] closure.
///
/// This `struct` provides shared and mutable access to the the value, if mutable access is requested the value
/// is marked as *touched* and the variable will be *new* in the next app update.
pub struct VarModifyValue<'a, T: VarValue> {
    update_id: VarUpdateId,
    value: &'a mut T,
    touched: bool,
}
impl<'a, T: VarValue> VarModifyValue<'a, T> {
    /// Gets a shared reference, allows inspecting the value without causing a variable update.
    pub fn get(&self) -> &T {
        self.value
    }

    /// Get the mutable reference, marks the value as new.
    pub fn get_mut(&mut self) -> &mut T {
        self.touched = true;
        self.value
    }

    /// Causes a variable update.
    pub fn touch(&mut self) {
        self.touched = true;
    }

    /// Update ID that will be used for the variable if the value is touched.
    pub fn update_id(&self) -> VarUpdateId {
        self.update_id
    }
}

struct VarHandleData {
    perm: Cell<bool>,
    pos_modify_action: Box<dyn Fn(&Vars, &mut Updates, &dyn AnyVarValue) -> bool>,
}

struct VarHook(Rc<VarHandleData>);
impl VarHook {
    /// Callback, returns `true` if the handle must be retained.
    fn call(&self, vars: &Vars, update: &mut Updates, value: &dyn AnyVarValue) -> bool {
        (Rc::strong_count(&self.0) > 1 || self.0.perm.get()) && (self.0.pos_modify_action)(vars, update, value)
    }
}

/// Handle to a variable hook.
///
/// This can represent an widget subscriber, a var binding, var app handler or animation, dropping the handler stops
/// the behavior it represents.
#[derive(Clone)]
#[must_use = "var handle stops the behaviour it represents on drop"]
pub struct VarHandle(Option<Rc<VarHandleData>>);
impl VarHandle {
    fn new(pos_modify_action: Box<dyn Fn(&Vars, &mut Updates, &dyn AnyVarValue) -> bool>) -> (VarHandle, VarHook) {
        let c = Rc::new(VarHandleData {
            perm: Cell::new(false),
            pos_modify_action,
        });
        (VarHandle(Some(c.clone())), VarHook(c))
    }

    /// Handle to no variable.
    pub fn dummy() -> Self {
        VarHandle(None)
    }

    /// Returns `true` if the handle is a [`dummy`].
    ///
    /// [`dummy`]: VarWidgetHandle::dummy
    pub fn is_dummy(&self) -> bool {
        self.0.is_none()
    }

    /// Drop the handle without stopping the behavior it represents.
    ///
    /// Not that the behavior can still be stopped by dropping the involved variables.
    pub fn perm(self) {
        if let Some(s) = &self.0 {
            s.perm.set(true)
        }
    }
}

/// Methods of [`Var<T>`] that don't depend on the value type.
///
/// This trait is [sealed] and cannot be implemented for types outside of `zero_ui_core`.
///
/// [sealed]: https://rust-lang.github.io/api-guidelines/future-proofing.html#sealed-traits-protect-against-downstream-implementations-c-sealed
pub trait AnyVar: Any + crate::private::Sealed {
    /// Clone the variable into a type erased box, this is never [`BoxedVar<T>`].
    fn clone_any(&self) -> BoxedAnyVar;

    /// Access to `dyn Any` methods.
    fn as_any(&self) -> &dyn Any;

    /// Access to `Box<dyn Any>` methods, with the [`BoxedVar<T>`] type.
    /// 
    /// This is a double-boxed to allow downcast to [`BoxedVar<T>`].
    fn into_boxed_any(self: Box<Self>) -> Box<dyn Any>;

    /// Gets the [`TypeId`] of `T` in `Var<T>`.
    fn var_type_id(&self) -> TypeId;

    /// Get a clone of the current value, with type erased.
    fn get_any(&self) -> Box<dyn AnyVarValue>;

    /// Try to schedule a new `value` for the variable, it will be set in the end of the current app update.
    ///
    /// # Panics
    ///
    /// Panics if the `value` is not of the same [`var_type_id`].
    ///
    /// [`var_type_id`]: AnyVar::var_type_id
    fn set_any(&self, vars: &Vars, value: Box<dyn AnyVarValue>) -> Result<(), VarIsReadOnlyError>;

    /// Last update ID a variable was modified, if the ID is equal to [`VarsRead::update_id`] the variable is *new*.
    fn last_update(&self) -> VarUpdateId;

    /// Flags that indicate what operations the variable is capable of.
    fn capabilities(&self) -> VarCapabilities;

    /// Setups a callback for just after the variable value is touched.
    ///
    /// Variables store a weak reference to the callback if they have the `MODIFY` or `CAP_CHANGE` capabilities, otherwise
    /// the callback is immediately discarded and [`VarHandle::dummy`] returned.
    ///
    /// This is the most basic callback, used by the variables themselves, you can create a more elaborate handle using [`on_update`].
    ///
    /// [`on_update`]: Var::on_update
    fn hook(&self, pos_modify_action: Box<dyn Fn(&Vars, &mut Updates, &dyn AnyVarValue) -> bool>) -> VarHandle;

    /// Register the widget to receive update when this variable is new.
    ///
    /// Variables without the [`CHANGE`] capability return [`VarHandle::dummy`].
    ///
    /// [`CHANGE`]: VarCapabilities::CHANGE
    fn subscribe(&self, widget_id: WidgetId) -> VarHandle {
        self.hook(var_subscribe(widget_id))
    }

    /// Gets the number of strong references to the variable.
    ///
    /// This is the [`Rc::strong_count`] for *Rc* variables, the represented var count for [`ContextVar<T>`], the boxed var count
    /// for [`BoxedVar<T>`] and `0` for [`LocalVar<T>`].
    fn strong_count(&self) -> usize;

    /// Gets the number of weak references to the variable.
    ///
    /// This is the [`Rc::weak_count`] for *Rc* variables, the represented var count for [`ContextVar<T>`], the boxed var count
    /// for [`BoxedVar<T>`] and `0` for [`LocalVar<T>`].
    fn weak_count(&self) -> usize;

    /// Gets a clone of the represented var from [`ContextVar<T>`], gets a clone of `self` for other var types.
    fn actual_var_any(&self) -> BoxedAnyVar;

    /// Create a weak reference to this *Rc* variable.
    ///
    /// The weak reference is made to the [`actual_var`], if the actual var is a [`LocalVar<T>`]
    /// a [`types::weak_var<T>`] is returned, for *Rc* vars an actual weak reference is made.
    ///
    /// [`actual_var`]: Var::actual_var
    fn downgrade_any(&self) -> BoxedAnyWeakVar;
}

/// Represents a weak reference to a boxed [`AnyVar`].
pub trait AnyWeakVar: Any + crate::private::Sealed {
    /// Clone the weak reference.
    fn clone_any(&self) -> BoxedAnyWeakVar;

    /// Gets the number of strong references to the variable.
    ///
    /// This is the same as [`AnyVar::strong_count`].
    fn strong_count(&self) -> usize;

    /// Gets the number of weak references to the variable.
    ///
    /// This is the same as [`AnyVar::weak_count`].
    fn weak_count(&self) -> usize;

    /// Upgrade to a strong [`AnyVar`] clone.
    ///
    /// Returns `None` if the [`strong_count`] is zero.
    ///
    /// [`strong_count`]: AnyWeakVar
    fn upgrade_any(&self) -> Option<BoxedAnyVar>;
}

/// Represents a weak reference to a [`Var<T>`].
pub trait WeakVar<T: VarValue>: AnyWeakVar + Clone {
    /// Output of [`WeakVar::upgrade`].
    type Upgrade: Var<T>;

    /// Upgrade to a strong [`BoxedVar<T>`] clone.
    ///
    /// Returns `None` if the [`strong_count`] is zero.
    ///
    /// [`strong_count`]: AnyWeakVar
    fn upgrade(&self) -> Option<Self::Upgrade>;

    /// Gets the weak reference a as [`BoxedWeakVar<T>`], does not double box.
    fn boxed(self) -> BoxedWeakVar<T>
    where
        Self: Sized,
    {
        Box::new(self)
    }
}

/// A value-to-var conversion that consumes the value.
///
/// Every [`Var<T>`] implements this to convert to it-self, every [`VarValue`] implements this to
/// convert to an [`LocalVar<T>`].
///
/// This trait is used by used by most properties, it allows then to accept literal values, variables and context variables
/// all with a single signature. Together with [`Var<T>`] this gives properties great flexibility of usage, at zero-cost. Widget
/// `when` blocks also use [`IntoVar<T>`] to support *changing* the property value depending on the widget state.
///
/// Value types can also manually implement this to support a shorthand literal syntax for when they are used in properties,
/// this converts the *shorthand value* like a tuple into the actual value type and wraps it into a variable, usually [`LocalVar`]
/// too. They can implement the trait multiple times to support different shorthand syntaxes or different types in the shorthand
/// value.
///
/// # Examples
///
/// A value type using [`IntoVar<T>`] twice to support a shorthand initialization syntax:
///
/// ```
/// # use zero_ui_core::var::*;
/// # use zero_ui_core::*;
/// #[derive(Debug, Clone)]
/// pub struct Size {
///     width: f32,
///     height: f32
/// }
/// impl IntoVar<Size> for (u32, u32) {
///     type Var = LocalVar<Size>;
///
///     fn into_var(self) -> Self::Var {
///         LocalVar(Size { width: self.0 as f32, height: self.1 as f32 })
///     }
/// }
/// impl IntoVar<Size> for (f32, f32) {
///     type Var = LocalVar<Size>;
///
///     fn into_var(self) -> Self::Var {
///         LocalVar(Size { width: self.0, height: self.1 })
///     }
/// }
/// #[property(size)]
/// pub fn size(child: impl UiNode, size: impl IntoVar<Size>) -> impl UiNode {
///     // ...
///     # child
/// }
/// # #[widget($crate::blank)]
/// # mod blank { }
/// # fn main() {
/// // shorthand #1:
/// let wgt = blank! {
///     size = (800, 600);
/// };
///
/// // shorthand #2:
/// let wgt = blank! {
///     size = (800.1, 600.2);
/// };
///
/// // blanket impl:
/// let wgt = blank! {
///     size = Size { width: 800.0, height: 600.0 };
/// };
/// # }
/// ```
///
/// A property implemented using [`IntoVar`]:
///
/// ```
/// # use zero_ui_core::var::*;
/// # use zero_ui_core::text::*;
/// # use zero_ui_core::context::*;
/// # use zero_ui_core::*;
/// #[property(layout)]
/// pub fn foo(child: impl UiNode, bar: impl IntoVar<u32>) -> impl UiNode {
///     struct FooNode<C, V> {
///         child: C,
///         bar: V
///     }
///     #[impl_ui_node(child)]
///     impl<C: UiNode, V: Var<u32>> UiNode for FooNode<C, V> {
///         fn init(&mut self, ctx: &mut WidgetContext) {
///             self.child.init(ctx);
///             println!("init: {}", self.bar.get(ctx));
///         }
///         
///         fn update(&mut self, ctx: &mut WidgetContext) {
///             self.child.update(ctx);
///             if let Some(new) = self.bar.get_new(ctx) {
///                 println!("update: {new}");
///             }
///         }
///     }
///
///     FooNode { child, bar: bar.into_var() }
/// }
///
/// # #[widget($crate::blank)]
/// # pub mod blank { }
/// # fn main() {
/// // literal assign:
/// let wgt = blank! {
///     foo = 42;
/// };
///
/// // variable assign:
/// let variable = var(42);
/// let wgt = blank! {
///     foo = variable;
/// };
///
/// // widget when:
/// let wgt = blank! {
///     foo = 42;
///
///     when !self.enabled {
///         foo = 32;
///     }
/// };
/// # }
/// ```
///
/// The property implementation is minimal and yet it supports a variety of different inputs that
/// alter how it is compiled, from a static literal value that never changes to an updating variable to a changing widget state.
///
/// In the case of an static value the update code will be optimized away, but if assigned a variable it will become dynamic
/// reacting to state changes, the same applies to `when` that compiles to a single property assign with a generated variable.
pub trait IntoVar<T: VarValue>: Clone {
    /// Variable type that will wrap the `T` value.
    ///
    /// This is the [`LocalVar`] for most types or `Self` for variable types.
    type Var: Var<T>;

    /// Converts the source value into a var.
    fn into_var(self) -> Self::Var;

    #[doc(hidden)]
    #[allow(non_snake_case)]
    fn allowed_in_when_property_requires_IntoVar_members(&self) -> Self::Var {
        self.clone().into_var()
    }
}

/// Represents an observable value.
///
/// All variable types can be read, some can update, variables update only in between app updates so
/// all widgets observing a variable can see the full sequence of values. Variables can also be a [`ContextVar<T>`] that
/// is a reference to another variable provided by the calling context, so the variable value depends on where it is read.
///
/// This trait is [sealed] and cannot be implemented for types outside of `zero_ui_core`.
///
/// [sealed]: https://rust-lang.github.io/api-guidelines/future-proofing.html#sealed-traits-protect-against-downstream-implementations-c-sealed
pub trait Var<T: VarValue>: IntoVar<T, Var = Self> + AnyVar + Clone {
    /// Output of [`Var::read_only`].
    ///
    /// This is `Self` for vars that are always read-only, or [`ReadOnlyVar<T, Self>`] for others.
    type ReadOnly: Var<T>;

    /// Output of [`Var::actual_var`].
    ///
    /// This is [`BoxedVar<T>`] for [`ContextVar<T>`], `V` for [`types::FlatMapVar<T, V>`] and `Self` for others.
    type ActualVar: Var<T>;

    /// Output of [`Var::downgrade`].
    type Downgrade: WeakVar<T>;

    /// Visit the current value of the variable, inside `read` the variable is locked/borrowed and cannot
    /// be modified.
    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&T) -> R;

    /// Try to schedule a variable update, it will be applied on the end of the current app update.
    fn modify<V, F>(&self, vars: &V, modify: F) -> Result<(), VarIsReadOnlyError>
    where
        V: WithVars,
        F: FnOnce(&mut VarModifyValue<T>) + 'static;

    /// Gets the variable as [`BoxedVar<T>`], does not double box.
    fn boxed(self) -> BoxedVar<T>
    where
        Self: Sized,
    {
        Box::new(self)
    }

    /// Gets a clone of the current *inner* var represented by this var. This is the same var, except for [`ContextVar<T>`]
    /// and [`types::FlatMapVar<T, V>`].
    fn actual_var(&self) -> Self::ActualVar;

    /// Create a weak reference to this *Rc* variable.
    ///
    /// The weak reference is made to the [`actual_var`], if the actual var is a [`LocalVar<T>`]
    /// a clone of it is returned, for *Rc* vars an actual weak reference is made.
    ///
    /// [`actual_var`]: Var::actual_var
    fn downgrade(&self) -> Self::Downgrade;

    /// Convert this variable to the value, if possible moves the value, if it is shared clones it.
    fn into_value(self) -> T;

    /// Gets a clone of the var that is always read-only.
    ///
    /// The returned variable can still update if `self` is modified, but it does not have the `MODIFY` capability.
    fn read_only(&self) -> Self::ReadOnly;

    /// Gets if the [`last_update`] is the current update, meaning the variable value just changed.
    ///
    /// [`last_update`]: AnyVar::last_update
    fn is_new<V>(&self, vars: &V) -> bool
    where
        V: WithVars,
    {
        vars.with_vars(Vars::update_id) == self.last_update()
    }

    /// Create a future that awaits and yields [`is_new`].
    ///
    /// The future can only be used in app bound async code, it can be reused.
    ///
    /// [`is_new`]: Var::is_new
    fn wait_is_new<'a, C>(&'a self, vars: &'a C) -> types::WaitIsNewFut<'a, C, T, Self>
    where
        C: WithVars,
    {
        types::WaitIsNewFut::new(vars, self)
    }

    /// Visit the current value of the variable, if it [`is_new`].
    ///
    /// [`is_new`]: Var::is_new
    fn with_new<V, R, F>(&self, vars: &V, read: F) -> Option<R>
    where
        V: WithVars,
        F: FnOnce(&T) -> R,
    {
        if self.is_new(vars) {
            Some(self.with(read))
        } else {
            None
        }
    }

    /// Get a clone of the current value.
    fn get(&self) -> T {
        self.with(Clone::clone)
    }

    /// Get a clone of the current value into `value`.
    fn get_into(&self, value: &mut T) {
        self.with(var_get_into(value))
    }

    /// Get a clone of the current value into `value` if the current value is not equal to it.
    fn get_ne(&self, value: &mut T) -> bool
    where
        T: PartialEq,
    {
        self.with(var_get_ne(value))
    }

    /// Get a clone of the current value, if it [`is_new`].
    ///
    /// [`is_new`]: Var::is_new
    fn get_new<V>(&self, vars: &V) -> Option<T>
    where
        V: WithVars,
    {
        if self.is_new(vars) {
            Some(self.with(Clone::clone))
        } else {
            None
        }
    }

    /// Get a clone of the current value into `value` if the current value [`is_new`].
    ///
    /// [`is_new`]: Var::is_new
    fn get_new_into<V>(&self, vars: &V, value: &mut T) -> bool
    where
        V: WithVars,
    {
        let is_new = self.is_new(vars);
        if is_new {
            self.with(var_get_into(value));
        }
        is_new
    }

    /// Get a clone of the current value into `value` if the variable value [`is_new`] and not equal to the `value`.
    ///
    /// [`is_new`]: Var::is_new
    fn get_new_ne<V>(&self, vars: &V, value: &mut T) -> bool
    where
        T: PartialEq,
        V: WithVars,
    {
        self.is_new(vars) && self.get_ne(value)
    }

    /// Create a future that awaits and yields [`get_new`].
    ///
    /// The future can only be used in app bound async code, it can be reused.
    ///
    /// [`get_new`]: Var::get_new
    fn wait_new<'a, C>(&'a self, vars: &'a C) -> types::WaitNewFut<'a, C, T, Self>
    where
        C: WithVars,
    {
        types::WaitNewFut::new(vars, self)
    }

    /// Try to schedule a new `value` for the variable, it will be set in the end of the current app update.
    fn set<V, I>(&self, vars: &V, value: I) -> Result<(), VarIsReadOnlyError>
    where
        V: WithVars,
        I: Into<T>,
    {
        self.modify(vars, var_set(value.into()))
    }

    /// Try to schedule a new `value` for the variable, it will be set in the end of the current app update if it is not
    /// equal to the variable value *at that time*, this only flags the variable as new if the values are not equal.
    ///
    /// Note that this is different from comparing with the current value and assigning,
    /// if another var modify request is already scheduled the `value` will be compared with the output of that operation.
    fn set_ne<V, I>(&self, vars: &V, value: I) -> Result<(), VarIsReadOnlyError>
    where
        T: PartialEq,
        V: WithVars,
        I: Into<T>,
    {
        self.modify(vars, var_set_ne(value.into()))
    }

    /// Causes a variable update without actually changing the variable value.
    fn touch<V>(&self, vars: &V) -> Result<(), VarIsReadOnlyError>
    where
        V: WithVars,
    {
        self.modify(vars, var_touch)
    }

    /// Create a ref-counted var that redirects to this variable until the first value touch, then it behaves like a [`RcVar<T>`].
    ///
    /// The return variable is *clone-on-write* and has the `MODIFY` capability independent of the source capabilities, when
    /// a modify request is made the source value is cloned and offered for modification, if modified the source variable is dropped
    /// and the cow var behaves like a [`RcVar<T>`], if the modify closure does not touch the cloned value it is dropped and the cow
    /// continues to redirect to the source variable.
    fn cow(&self) -> types::RcCowVar<T, Self> {
        types::RcCowVar::new(self.clone())
    }

    /// Creates a ref-counted var that maps from this variable.
    ///
    /// The `map` closure is called immediately to generate the initial value, and then once every time
    /// the source variable updates. The source variable is not held by the map variable, if dropped the map
    /// variable stops updating.
    ///
    /// The mapping variable is read-only, you can use [`map_bidi`] to map back.
    ///
    /// [`map_bidi`]: Var::map_bidi
    fn map<O, M>(&self, mut map: M) -> ReadOnlyRcVar<O>
    where
        O: VarValue,
        M: FnMut(&T) -> O + 'static,
    {
        let other = var(self.with(&mut map));
        self.bind_map(&other, map).perm();
        other.read_only()
    }

    /// Create a ref-counted var that maps from this variable on read and to it on write.
    ///
    /// The `map` closure is called immediately to generate the initial value, and then once every time
    /// the source variable updates, the `map_back` closure is called every time the output value is modified.
    fn map_bidi<O, M, B>(&self, mut map: M, map_back: B) -> RcVar<O>
    where
        O: VarValue,
        M: FnMut(&T) -> O + 'static,
        B: FnMut(&O) -> T + 'static,
    {
        let other = var(self.with(&mut map));
        let [h1, h2] = self.bind_map_bidi(&other, map, map_back);
        h1.perm();
        h2.perm();
        other
    }

    /// Create a ref-counted var that maps to an inner variable that is found inside the value of this variable.
    ///
    /// The `map` closure is called immediately to clone the initial inner var, and than once every time
    /// the source variable updates.
    ///
    /// The mapping var has the same capabilities of the inner var + `CAP_CHANGE`, modifying the mapping var modifies the inner var.
    fn flat_map<O, V, M>(&self, map: M) -> types::RcFlatMapVar<O, V>
    where
        O: VarValue,
        V: Var<O>,
        M: FnMut(&T) -> V + 'static,
    {
        types::RcFlatMapVar::new(self, map)
    }

    /// Creates a ref-counted var that maps from this variable, but can retain a previous mapped value.
    ///
    /// The `map` closure is called immediately to generate the initial value, if it returns `None` the `init` closure is called to generate
    /// a fallback value, after, the `map` closure is called once every time
    /// the mapping variable reads and is out of sync with the source variable, if it returns `Some(_)` the mapping variable value changes,
    /// otherwise the previous value is retained, either way the mapping variable is *new*.
    ///
    /// The mapping variable is read-only, use [`filter_map_bidi`] to map back.
    ///
    /// [`map_bidi`]: Var::map_bidi
    fn filter_map<O, M, I>(&self, mut map: M, init: I) -> ReadOnlyRcVar<O>
    where
        O: VarValue,
        M: FnMut(&T) -> Option<O> + 'static,
        I: FnOnce() -> O,
    {
        let other = var(self.with(&mut map).unwrap_or_else(init));
        self.bind_filter_map(&other, map).perm();
        other.read_only()
    }

    /// Create a ref-counted var that maps from this variable on read and to it on write, mapping in both directions can skip
    /// a value, retaining the previous mapped value.
    ///
    /// The `map` closure is called immediately to generate the initial value, if it returns `None` the `init` closure is called
    /// to generate a fallback value, after, the `map` closure is called once every time
    /// the mapping variable reads and is out of sync with the source variable, if it returns `Some(_)` the mapping variable value changes,
    /// otherwise the previous value is retained, either way the mapping variable is *new*. The `map_back` closure
    /// is called every time the output value is modified, if it returns `Some(_)` the source variable is set, otherwise the source
    /// value is not touched.
    fn filter_map_bidi<O, M, B, I>(&self, mut map: M, map_back: B, init: I) -> RcVar<O>
    where
        O: VarValue,
        M: FnMut(&T) -> Option<O> + 'static,
        B: FnMut(&O) -> Option<T> + 'static,
        I: FnOnce() -> O,
    {
        let other = var(self.with(&mut map).unwrap_or_else(init));
        let [h1, h2] = self.bind_filter_map_bidi(&other, map, map_back);
        h1.perm();
        h2.perm();
        other
    }

    /// Setup a hook that assigns `other` with the new values of `self` transformed by `map`.
    ///
    /// Only a weak reference to the `other` variable is held, both variables update in the same app update cycle.
    ///
    /// Note that the current value is not assigned, only the subsequent updates, you can assign
    /// `other` and then bind to fully sync the variables.
    fn bind_map<T2, V2, M>(&self, other: &V2, mut map: M) -> VarHandle
    where
        T2: VarValue,
        V2: Var<T2>,
        M: FnMut(&T) -> T2 + 'static,
    {
        var_bind(self, other, move |vars, _, value, other| {
            let _ = other.set(vars, map(value));
        })
    }

    /// Setup a hook that assigns `other` with the new values of `self` transformed by `map`, if the closure returns a value.
    ///
    /// Only a weak reference to the `other` variable is held, both variables update in the same app update cycle.
    ///
    /// Note that the current value is not assigned, only the subsequent updates, you can assign
    /// `other` and then bind to fully sync the variables.
    fn bind_filter_map<T2, V2, F>(&self, other: &V2, mut map: F) -> VarHandle
    where
        T2: VarValue,
        V2: Var<T2>,
        F: FnMut(&T) -> Option<T2> + 'static,
    {
        var_bind(self, other, move |vars, _, value, other| {
            if let Some(value) = map(value) {
                let _ = other.set(vars, value);
            }
        })
    }

    /// Bind `self` to `other` and back without causing an infinite loop.
    ///
    /// Only a weak reference to the `other` variable is held, if both variables are scheduled to update in the same cycle
    /// both get assigned, but only one bind transfer per app cycle is allowed for each variable. Returns two handles on the
    /// the *map* hook and one for the *map-back* hook.
    ///
    /// Note that the current value is not assigned, only the subsequent updates, you can assign
    /// `other` and `self` and then bind to fully sync the variables.
    fn bind_map_bidi<T2, V2, M, B>(&self, other: &V2, mut map: M, mut map_back: B) -> [VarHandle; 2]
    where
        T2: VarValue,
        V2: Var<T2>,
        M: FnMut(&T) -> T2 + 'static,
        B: FnMut(&T2) -> T + 'static,
    {
        let mut last_update = VarUpdateId::never();
        let self_to_other = var_bind(self, other, move |vars, _, value, other| {
            if vars.update_id() != last_update {
                last_update = vars.update_id();
                let _ = other.set(vars, map(value));
            }
        });

        let mut last_update = VarUpdateId::never();
        let other_to_self = var_bind(other, self, move |vars, _, value, self_| {
            if vars.update_id() != last_update {
                last_update = vars.update_id();
                let _ = self_.set(vars, map_back(value));
            }
        });

        [self_to_other, other_to_self]
    }

    /// Bind `self` to `other` and back with the new values of `self` transformed by `map` and the new values of `other` transformed
    /// by `map_back`, the value is assigned in a update only if the closures returns a value.
    ///
    /// Only a weak reference to the `other` variable is held, both variables update in the same app update cycle.
    ///
    /// Note that the current value is not assigned, only the subsequent updates, you can assign
    /// `other` and then bind to fully sync the variables.
    fn bind_filter_map_bidi<T2, V2, M, B>(&self, other: &V2, mut map: M, mut map_back: B) -> [VarHandle; 2]
    where
        T2: VarValue,
        V2: Var<T2>,
        M: FnMut(&T) -> Option<T2> + 'static,
        B: FnMut(&T2) -> Option<T> + 'static,
    {
        let mut last_update = VarUpdateId::never();
        let self_to_other = var_bind(self, other, move |vars, _, value, other| {
            if vars.update_id() != last_update {
                last_update = vars.update_id();
                if let Some(value) = map(value) {
                    let _ = other.set(vars, value);
                }
            }
        });

        let mut last_update = VarUpdateId::never();
        let other_to_self = var_bind(other, self, move |vars, _, value, self_| {
            if vars.update_id() != last_update {
                last_update = vars.update_id();
                if let Some(value) = map_back(value) {
                    let _ = self_.set(vars, value);
                }
            }
        });

        [self_to_other, other_to_self]
    }

    /// Setup a hook that assigns `other` with the new values of `self`.
    ///
    /// Only a weak reference to the `other` variable is held.
    ///
    /// Note that the current value is not assigned, only the subsequent updates, you can assign
    /// `other` and then bind to fully sync the variables.
    fn bind<V2>(&self, other: &V2) -> VarHandle
    where
        V2: Var<T>,
    {
        self.bind_map(other, Clone::clone)
    }

    /// Setup two hooks that assigns `other` with the new values of `self` and `self` with the new values of `other`.
    ///
    /// Only a weak reference to the variables is held.
    ///
    /// Note that the current value is not assigned, only the subsequent updates, you can assign
    /// `other` and then bind to fully sync the variables.
    fn bind_bidi<V2>(&self, other: &V2) -> [VarHandle; 2]
    where
        V2: Var<T>,
    {
        self.bind_map_bidi(other, Clone::clone, Clone::clone)
    }

    /// Creates a sender that can set `self` from other threads and without access to [`Vars`].
    ///
    /// If the variable is read-only when a value is received it is silently dropped.
    fn sender<V: WithVars>(&self, vars: &V) -> VarSender<T>
    where
        T: Send,
    {
        todo!()
    }

    /// Creates a sender that modify `self` from other threads and without access to [`Vars`].
    ///
    /// If the variable is read-only when a modification is received it is silently dropped.
    fn modify_sender<V>(&self, vars: &V) -> VarModifySender<T>
    where
        V: WithVars,
    {
        todo!()
    }

    /// Creates a channel that can receive `var` updates from another thread.
    ///
    /// Every time the variable updates a clone of the value is sent to the receiver. The current value is sent immediately.
    fn receiver<V>(&self, vars: &V) -> VarReceiver<T>
    where
        T: Send,
        V: WithVars,
    {
        todo!()
    }

    /// Add a preview `handler` that is called every time this variable value is set, modified or touched,
    /// the handler is called before all other UI updates.
    ///
    /// Note that the handler runs on the app context, all [`ContextVar<T>`] read inside read the default value.
    fn on_pre_new<H>(&self, handler: H) -> VarHandle
    where
        H: AppHandler<T>,
    {
        todo!()
    }

    // Add a `handler` that is called every time this variable value is set, modified or touched,
    /// the handler is called after all other UI updates.
    ///
    /// Note that the handler runs on the app context, all [`ContextVar<T>`] read inside read the default value.
    fn on_new<H>(&self, handler: H) -> VarHandle
    where
        H: AppHandler<T>,
    {
        todo!()
    }

    /// Debug helper for tracing the lifetime of a value in this variable.
    ///
    /// The `enter_value` closure is called every time the variable value is set, modified or touched, it can return
    /// an implementation agnostic *scope* or *span* `S` that is only dropped when the variable updates again.
    ///
    /// The `enter_value` is also called immediately when this method is called to start tracking the first value.
    ///
    /// Returns a [`OnVarHandle`] that can be used to stop tracing. Making the handle permanent means that the tracing will happen
    /// for the variable or app, the tracing handler only holds a weak reference to the variable.
    ///
    /// If this variable can never update the span is immediately dropped and a dummy handle is returned. If the variable [`is_contextual`]
    /// the trace is set on the [`actual_var`].
    ///
    /// # Examples
    ///
    /// Using the [`tracing`] crate to trace value spans:
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui_core::var::*;
    /// # struct Fake; impl Fake { pub fn entered(self) { } }
    /// # #[macro_export]
    /// # macro_rules! info_span { ($($tt:tt)*) => { Fake }; }
    /// # mod tracing {  pub use crate::info_span; }
    /// # fn trace_var<T: VarValue>(var: &impl Var<T>) {
    /// var.trace_value(|value| {
    ///     tracing::info_span!("my_var", ?value, track = "<vars>").entered()
    /// }).perm();
    /// # }
    /// ```
    ///
    /// Note that you don't need to use any external tracing crate, this method also works with the standard printing:
    ///
    /// ```
    /// # use zero_ui_core::var::*;
    /// # fn trace_var(var: &impl Var<u32>) {
    /// var.trace_value(|v| println!("value: {v:?}")).perm();
    /// # }
    /// ```
    ///
    /// [`tracing`]: https://docs.rs/tracing/
    /// [`is_contextual`]: Var::is_contextual
    /// [`actual_var`]: Var::actual_var
    fn trace_value<E, S>(&self, mut enter_value: E) -> VarHandle
    where
        E: FnMut(&T) -> S + 'static,
        S: 'static,
    {
        let mut span = Some(self.with(&mut enter_value));
        self.on_pre_new(app_hn!(|_, value, _| {
            let _ = span.take();
            span = Some(enter_value(value));
        }))
    }
}

// Closure type independent of the variable type, hopefully reduces LLVM lines:

fn var_get_into<T>(value: &mut T) -> impl FnOnce(&T) + '_
where
    T: VarValue,
{
    move |var_value| value.clone_from(var_value)
}
fn var_get_ne<T>(value: &mut T) -> impl FnOnce(&T) -> bool + '_
where
    T: VarValue + PartialEq,
{
    move |var_value| {
        let ne = var_value != value;
        if ne {
            value.clone_from(var_value);
        }
        ne
    }
}
fn var_set<T>(value: T) -> impl FnOnce(&mut VarModifyValue<T>)
where
    T: VarValue,
{
    move |var_value| {
        *var_value.get_mut() = value;
    }
}
fn var_set_ne<T>(value: T) -> impl FnOnce(&mut VarModifyValue<T>)
where
    T: VarValue + PartialEq,
{
    move |var_value| {
        if var_value.get() != &value {
            *var_value.get_mut() = value;
        }
    }
}
fn var_set_any<T>(value: Box<dyn AnyVarValue>) -> impl FnOnce(&mut VarModifyValue<T>)
where
    T: VarValue,
{
    match value.into_any().downcast::<T>() {
        Ok(value) => var_set(*value),
        Err(_) => panic!("cannot `set_any`, incompatible type"),
    }
}

fn var_touch<T>(var_value: &mut VarModifyValue<T>)
where
    T: VarValue,
{
    var_value.touch()
}

fn var_subscribe(widget_id: WidgetId) -> Box<dyn Fn(&Vars, &mut Updates, &dyn AnyVarValue) -> bool> {
    Box::new(move |_, updates, _| {
        updates.update(widget_id);
        true
    })
}

fn var_bind<I: VarValue, O: VarValue, V: Var<O>>(
    input: &impl Var<I>,
    output: &V,
    update_output: impl FnMut(&Vars, &mut Updates, &I, <V::Downgrade as WeakVar<O>>::Upgrade) + 'static,
) -> VarHandle {
    if input.capabilities().is_always_static() || output.capabilities().is_always_read_only() {
        VarHandle::dummy()
    } else {
        var_bind_ok(input, output.downgrade(), update_output)
    }
}

fn var_bind_ok<I: VarValue, O: VarValue, W: WeakVar<O>>(
    input: &impl Var<I>,
    wk_output: W,
    update_output: impl FnMut(&Vars, &mut Updates, &I, W::Upgrade) + 'static,
) -> VarHandle {
    let update_output = RefCell::new(update_output);
    input.hook(Box::new(move |vars, updates, value| {
        if let Some(output) = wk_output.upgrade() {
            if output.capabilities().contains(VarCapabilities::MODIFY) {
                if let Some(value) = value.as_any().downcast_ref::<I>() {
                    update_output.borrow_mut()(vars, updates, value, output);
                }
            }
            true
        } else {
            false
        }
    }))
}
