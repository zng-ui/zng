//! Variables.

use std::{
    cell::{Cell, RefCell},
    convert::{TryFrom, TryInto},
    fmt,
    marker::PhantomData,
    mem,
    ops::{self, Deref, DerefMut},
    rc::Rc,
    str::FromStr,
    time::Duration,
};

use crate::{
    handler::AppHandler,
    text::{Text, ToText},
    units::{EasingStep, EasingTime, Factor, FactorUnits},
    widget_info::UpdateMask,
    WidgetId,
};

#[macro_use]
mod any;

mod binding;
pub use binding::*;

mod vars;
pub use vars::*;

mod boxed;
pub use boxed::*;

mod context;
pub use context::*;

mod state;
pub use state::*;

mod read_only;

mod cow;
mod expr;
mod filter_map;
mod flat_map;
mod future;
mod local;
mod map;
mod map_ref;
mod merge;
mod rc;
mod switch;
mod when;

pub mod animation;

pub use animation::easing;

use animation::{AnimationHandle, ChaseAnimation, ChaseMsg, Transition, TransitionKeyed, Transitionable};

/// Variable types.
///
/// These types are mostly implementation details, you should use the [`Var<T>`] interface when possible.
pub mod types {
    pub use super::cow::*;
    pub use super::expr::*;
    pub use super::filter_map::*;
    pub use super::flat_map::*;
    pub use super::future::*;
    pub use super::local::*;
    pub use super::map::*;
    pub use super::map_ref::*;
    pub use super::merge::*;
    pub use super::rc::*;
    pub use super::read_only::*;
    pub use super::switch::*;
    pub use super::when::*;
}

#[doc(inline)]
pub use types::{
    expr_var, merge_var, response_done_var, response_var, state_var, switch_var, var, var_default, var_from, when_var, LocalVar, RcCowVar,
    RcVar, ReadOnlyRcVar, ResponderVar, Response, ResponseVar, StateVar,
};

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
///
/// See [`ContextVarProxy<C>`] for details about context var behavior as a var.
#[cfg_attr(doc_nightly, doc(notable_trait))]
pub trait ContextVar: Clone + Copy + 'static {
    /// The variable type.
    type Type: VarValue;

    /// New default value.
    ///
    /// Returns a value that is equal to the variable value when it is not set in any context.
    fn default_value() -> Self::Type;

    /// Gets the variable.
    fn new() -> ContextVarProxy<Self> {
        ContextVarProxy::new()
    }

    /// Use [`context_var!`] to implement context vars.
    ///
    /// If that is not possible copy the `thread_local` implementation generated
    /// by the macro as close as possible.
    #[doc(hidden)]
    fn thread_local_value() -> ContextVarLocalKey<Self::Type>;
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
///
/// Every [`Var`] implements this to convert to it-self, every [`VarValue`] implements this to
/// convert to an [`LocalVar`].
///
/// This trait is used by used by most properties, it allows then to accept literal values, variables and context variables
/// all with a single signature. Together with [`Var`] this gives properties great flexibility of usage, at zero-cost. Widget
/// `when` blocks also use [`IntoVar`] to support *changing* the property value depending on the widget state.
///
/// Value types can also manually implement this to support a shorthand literal syntax for when they are used in properties,
/// this converts the *shorthand value* like a tuple into the actual value type and wraps it into a variable, usually [`LocalVar`]
/// too. They can implement the trait multiple times to support different shorthand syntaxes or different types in the shorthand
/// value.
///
/// # Examples
///
/// A value type using [`IntoVar`] twice to support a shorthand initialization syntax:
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
pub trait IntoValue<T: fmt::Debug>: Into<T> + Clone {}
impl<T: fmt::Debug + Clone> IntoValue<T> for T {}

/// Represents a weak reference to a [`Var<T>`] that is a shared pointer.
pub trait WeakVar<T: VarValue>: Clone + crate::private::Sealed + 'static {
    /// The strong var type.
    type Strong: Var<T>;

    /// Gets the variable if it still exists.
    fn upgrade(&self) -> Option<Self::Strong>;

    /// Gets the number of strong references to the variable.
    fn strong_count(&self) -> usize;

    /// Gets the number of weak references to the variable.
    ///
    /// If no strong references remain, returns zero.
    fn weak_count(&self) -> usize;

    /// Gets an opaque raw pointer to the shared variable inner data.
    ///
    /// This can only be used for comparisons, the only guarantee about the inner data is that it is not dynamic.
    fn as_ptr(&self) -> *const ();

    /// If `self` and `other` are both weak references to the same variable.
    fn ptr_eq<W: WeakVar<T>>(&self, other: &W) -> bool {
        self.as_ptr() == other.as_ptr()
    }

    /// Box this weak var.
    fn boxed(self) -> BoxedWeakVar<T>
    where
        Self: WeakVarBoxed<T> + Sized,
    {
        Box::new(self)
    }
}

#[doc(hidden)]
pub struct NoneWeakVar<T: VarValue>(PhantomData<T>);
impl<T: VarValue> Clone for NoneWeakVar<T> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}
impl<T: VarValue> Default for NoneWeakVar<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}
impl<T: VarValue> crate::private::Sealed for NoneWeakVar<T> {}
impl<T: VarValue> WeakVar<T> for NoneWeakVar<T> {
    type Strong = LocalVar<T>;

    fn upgrade(&self) -> Option<Self::Strong> {
        None
    }

    fn strong_count(&self) -> usize {
        0
    }

    fn weak_count(&self) -> usize {
        0
    }

    fn as_ptr(&self) -> *const () {
        std::ptr::null()
    }
}

#[cfg(dyn_closure)]
macro_rules! DefaultMapVar {
    ($T:ty, $O:ty, $M:ty, $V:ty) => {
        types::RcMapVar<$T, $O, Box<dyn FnMut(&$T) -> $O>, BoxedVar<T>>
    }
}
#[cfg(not(dyn_closure))]
macro_rules! DefaultMapVar {
    ($T:ty, $O:ty, $M:ty, $V:ty) => {
        types::RcMapVar<$T, $O, $M, $V>
    }
}

/// Represents an observable value.
///
/// This trait is [sealed] and cannot be implemented for types outside of `zero_ui_core`.
///
/// [sealed]: https://rust-lang.github.io/api-guidelines/future-proofing.html#sealed-traits-protect-against-downstream-implementations-c-sealed
#[cfg_attr(doc_nightly, doc(notable_trait))]
pub trait Var<T: VarValue>: Clone + IntoVar<T> + any::AnyVar + crate::private::Sealed + 'static {
    /**
     * README Before Adding Methods/Docs
     *
     * - If you are updating the docs of a method, you should also review all methods of the same name in all var implementers,
     *   they are all inside ./var and can declare the "same" method directly for multiple reasons.
     *
     * - If the new method starts an animation, update `easing::EasingVar` to pass-through the new method.
     *
     * - If the method modifies the value it must return `Result<Foo, VarIsReadOnly>` and variables
     *   that are never read-only should declare the same method name and input signature, but without returning a Result,
     *   see `RcVar::set` vs `Var::set` for an example.
     *
     */

    /// The variable type that represents a read-only version of this type.
    type AsReadOnly: Var<T>;

    /// The type of an weak reference to the variable, if it is a shared reference.
    type Weak: WeakVar<T>;

    // TODO when GATs are stable:
    // type Map<B: VarValue, M: FnMut(&T) -> B> : Var<B>;
    // type MapBidi<B: VarValue, M: FnMut(&T) -> B, N: FnMut(&B) -> T> : Var<B>;

    /// References the value.
    fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a T;

    /// Copy the value.
    fn copy<Vr: WithVarsRead>(&self, vars: &Vr) -> T
    where
        T: Copy,
    {
        vars.with_vars_read(|v| *self.get(v))
    }

    /// Clone the value.
    fn get_clone<Vr: WithVarsRead>(&self, vars: &Vr) -> T {
        vars.with_vars_read(|v| self.get(v).clone())
    }

    /// If the current value is not equal to the `output`, clones the value and set `output`.
    fn get_clone_ne<Vr: WithVarsRead>(&self, vars: &Vr, output: &mut T) -> bool
    where
        T: PartialEq,
    {
        let mut ne = false;
        vars.with_vars_read(|vars| {
            let value = self.get(vars);
            if value != output {
                ne = true;
                *output = value.clone();
            }
        });
        ne
    }

    /// References the value if [`is_new`](Self::is_new).
    fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a T>;

    /// Copy the value if [`is_new`](Self::is_new).
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
    /// # Examples
    ///
    /// ```
    /// # use zero_ui_core::{context::*, handler::*, var::*};
    /// # let foo_var = var(10i32);
    /// # TestWidgetContext::doc_test_multi((), vec![
    /// # Box::new(async_hn!(foo_var, |ctx, _| {
    /// #     foo_var.set(&ctx, 0);
    /// #     ctx.update().await;
    /// #     foo_var.set(&ctx, 10);
    /// #     ctx.update().await;
    /// # })),
    /// # Box::new(
    /// async_hn!(foo_var, |ctx, _| {
    ///     let value = foo_var.wait_copy(&ctx).await;
    ///     assert_eq!(Some(value), foo_var.copy_new(&ctx));
    ///
    ///     let value = foo_var.wait_copy(&ctx).await;
    ///     assert_eq!(Some(value), foo_var.copy_new(&ctx));
    /// })
    /// # ),], );
    /// ```
    ///
    /// In the example the handler awaits for the variable to have a new value, the code immediately after
    /// runs in the app update where the variable is new, the second `.await` does not poll immediately it awaits
    /// for the variable to be new again but in a different update.
    ///
    /// You can also reuse the future, but it is very cheap to just create a new one.
    fn wait_copy<'a, Vw: WithVars>(&'a self, vars: &'a Vw) -> types::VarCopyNewFut<'a, Vw, T, Self>
    where
        T: Copy,
    {
        if !self.can_update() {
            tracing::warn!("`Var::wait_copy` called in a variable that never updates");
        }
        types::VarCopyNewFut::new(vars, self)
    }

    /// Clone the value if [`is_new`](Self::is_new).
    fn clone_new<Vw: WithVars>(&self, vars: &Vw) -> Option<T> {
        vars.with_vars(|v| self.get_new(v).cloned())
    }

    /// If the current value is new and not equal to the current `output` it is cloned and set on the `output`.
    fn clone_new_ne<Vw: WithVars>(&self, vars: &Vw, output: &mut T) -> bool
    where
        T: PartialEq,
    {
        let mut ne = false;
        vars.with_vars(|vars| {
            if let Some(value) = self.get_new(vars) {
                if value != output {
                    ne = true;
                    *output = value.clone();
                }
            }
        });
        ne
    }

    /// Returns a future that awaits for [`clone_new`](Var::clone_new) after the current update.
    ///
    /// You can `.await` this in UI thread bound async code, like in async event handlers. The future
    /// will unblock once for every time [`clone_new`](Var::clone_new) returns `Some(T)` in a different update.
    ///
    /// Note that if [`Var::can_update`] is `false` this will never awake and a warning will be logged.
    ///
    /// # Examples
    ///
    /// ```
    /// # use zero_ui_core::{context::*, handler::*, var::*};
    /// # let foo_var = var(10i32);
    /// # TestWidgetContext::doc_test_multi((), vec![
    /// # Box::new(async_hn!(foo_var, |ctx, _| {
    /// #     foo_var.set(&ctx, 0);
    /// #     ctx.update().await;
    /// #     foo_var.set(&ctx, 10);
    /// #     ctx.update().await;
    /// # })),
    /// # Box::new(
    /// async_hn!(foo_var, |ctx, _| {
    ///     let value = foo_var.wait_clone(&ctx).await;
    ///     assert_eq!(Some(value), foo_var.clone_new(&ctx));
    ///
    ///     let value = foo_var.wait_clone(&ctx).await;
    ///     assert_eq!(Some(value), foo_var.clone_new(&ctx));
    /// })
    /// # ),], );
    /// ```
    ///
    /// In the example the handler awaits for the variable to have a new value, the code immediately after
    /// runs in the app update where the variable is new, the second `.await` does not poll immediately it awaits
    /// for the variable to be new again but in a different update.
    ///
    /// You can also reuse the future, but it is very cheap to just create a new one.
    fn wait_clone<'a, Vw: WithVars>(&'a self, vars: &'a Vw) -> types::VarCloneNewFut<'a, Vw, T, Self> {
        if !self.can_update() {
            tracing::warn!("`Var::wait_clone` called in a variable that never updates");
        }
        types::VarCloneNewFut::new(vars, self)
    }

    /// If the variable value changed in this update.
    ///
    /// When the variable value changes this stays `true` for the next app update cycle.
    /// An app update is requested only if the variable is shared (strong count > 1).
    fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool;

    /// Returns a future that awaits for [`is_new`] after the current update.
    ///
    /// You can `.await` this in UI thread bound async code, like in async event handlers. The future
    /// will unblock once for every time [`is_new`] returns `true` in a different update.
    ///
    /// Note that if [`Var::can_update`] is `false` this will never awake and a warning will be logged.
    ///
    /// # Examples
    ///
    /// ```
    /// # use zero_ui_core::{context::*, handler::*, var::*};
    /// # let foo_var = var(10i32);
    /// # TestWidgetContext::doc_test_multi((), vec![
    /// # Box::new(async_hn!(foo_var, |ctx, _| {
    /// #     foo_var.set(&ctx, 0);
    /// #     ctx.update().await;
    /// #     foo_var.set(&ctx, 10);
    /// #     ctx.update().await;
    /// # })),
    /// # Box::new(
    /// async_hn!(foo_var, |ctx, _| {
    ///     foo_var.wait_new(&ctx).await;
    ///     assert!(foo_var.is_new(&ctx));
    ///
    ///     foo_var.wait_new(&ctx).await;
    ///     assert!(foo_var.is_new(&ctx));
    /// })
    /// # ),], );
    /// ```
    ///
    /// In the example the handler awaits for the variable to have a new value, the code immediately after
    /// runs in the app update where the variable is new, the second `.await` does not poll immediately it awaits
    /// for the variable to be new again but in a different update.
    ///
    /// You can also reuse the future, but it is very cheap to just create a new one.
    ///
    /// [`is_new`]: Var::is_new
    fn wait_new<'a, Vw: WithVars>(&'a self, vars: &'a Vw) -> types::VarIsNewFut<'a, Vw, T, Self> {
        if !self.can_update() {
            tracing::warn!("`Var::wait_new` called in a variable that never updates");
        }
        types::VarIsNewFut::new(vars, self)
    }

    /// Gets the variable value version.
    ///
    /// The version is different every time the value is modified, you can use this to monitor
    /// variable change outside of the window of opportunity of [`is_new`](Self::is_new).
    ///
    /// If the variable [`is_contextual`](Self::is_contextual) the version is also different for each context.
    fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> VarVersion;

    /// If the variable cannot be set or modified right now.
    ///
    /// **Note** this can change unless the variable is [`always_read_only`](Self::always_read_only).
    fn is_read_only<Vw: WithVars>(&self, vars: &Vw) -> bool;

    /// If the variable can never be set or modified.
    ///
    /// **Note** the value can still be new by an internal change if [`can_update`](Self::can_update) is `true`.
    fn always_read_only(&self) -> bool;

    /// If the variable is a [`ContextVar`] or depends on one right now.
    ///
    /// If `true` the version is unique for each context, this in turn can cause mapping variables to re-evaluate
    /// if used in more then one context.
    fn is_contextual(&self) -> bool;

    /// Returns a clone of the underlying variable that owns the value.
    ///
    /// If the variable [`is_contextual`], this is a clone of the underlying variable that is currently assigned to the context
    /// var, if not it is a boxed clone of `self`. If the variable [`is_contextual`] because it depends on a [`ContextVar`], the
    /// variable effect is recreated on the actual var, for example, in a map var, the mapping closures are shared with a new
    /// mapping var that depends directly on the actual variable.
    ///
    /// [`is_contextual`]: Self::is_contextual
    fn actual_var<Vw: WithVars>(&self, vars: &Vw) -> BoxedVar<T>;

    /// If the variable is implemented as a shared reference to the value.
    ///
    /// If `true` cloning the variable is very cheap, only incrementing a reference count.
    fn is_rc(&self) -> bool;

    /// If the variable value can change.
    ///
    /// **Note** this can be `true` even if the variable is [`always_read_only`](Self::always_read_only).
    fn can_update(&self) -> bool;

    /// if the variable current value was set by an active animation.
    ///
    /// The variable [`is_new`] when this changes to `true`, but it can change to `false` at any time
    /// without the value updating.
    ///
    /// [`is_new`]: Var::is_new
    fn is_animating<Vr: WithVarsRead>(&self, vars: &Vr) -> bool;

    /// Returns a future that awaits for the next time [`is_animating`] changes to `false` after the current update.
    ///
    /// You can `.await` this in UI thread bound async code, like in async event handlers. The future
    /// will unblock once for every time [`is_animating`] changes from `true` to `false` in a different update.
    ///
    /// Note that if [`Var::can_update`] is `false` this will never awake and a warning will be logged.
    ///
    /// [`is_animating`]: Var::is_animating
    fn wait_animation<'a, Vr: WithVarsRead>(&'a self, vars: &'a Vr) -> types::VarIsNotAnimatingFut<'a, Vr, T, Self> {
        if !self.can_update() {
            tracing::warn!("`Var::wait_animation` called in a variable that never updates");
        }
        types::VarIsNotAnimatingFut::new(vars, self)
    }

    /// Convert this variable to the value, if the variable is a reference, clones the value.
    fn into_value<Vr: WithVarsRead>(self, vars: &Vr) -> T;

    /// Returns a weak reference to the variable if it [`is_rc`].
    ///
    /// [`is_rc`]: Self::is_rc
    fn downgrade(&self) -> Option<Self::Weak>;

    /// Returns the number of strong references to this variable if it [`is_rc`].
    ///
    /// Returns zero if the variable is not implemented as a shared reference.
    ///
    /// [`is_rc`]: Self::is_rc
    fn strong_count(&self) -> usize;

    /// Returns the number of weak references to this variable if it [`is_rc`].
    ///
    /// [`is_rc`]: Self::is_rc
    fn weak_count(&self) -> usize;

    /// Returns an opaque pointer to the variable inner data.
    ///
    /// This is only useful for identifying the variable, the only guarantee is that the inner data is not dynamic. Variables
    /// that are not [`is_rc`] always return `null`.
    ///
    /// [`is_rc`]: Self::is_rc
    fn as_ptr(&self) -> *const ();

    /// Returns `true` if both `self` and `other` point to the same address of if both pointers are null.
    fn ptr_eq<V: Var<T>>(&self, other: &V) -> bool {
        self.as_ptr() == other.as_ptr()
    }

    /// Returns `true` if both `self` and `other` point to the same variable if both are [rc].
    ///
    /// Returns `false` if either pointer is null.
    ///
    /// [rc]: Self::is_rc
    fn partial_ptr_eq<V: Var<T>>(&self, other: &V) -> bool {
        let a = self.as_ptr();
        let b = other.as_ptr();
        a == b && !a.is_null() && !b.is_null()
    }

    /// Schedule a modification of the variable value.
    ///
    /// The variable is marked as *new* only if the closure input is dereferenced as `mut`, and if
    /// it is marked  as new then the same behavior of [`set`] applies.
    ///
    /// [`set`]: Var::set
    fn modify<Vw, M>(&self, vars: &Vw, modify: M) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        M: FnOnce(VarModify<T>) + 'static;

    /// Causes the variable to notify update without changing the value.
    ///
    /// The variable will get a new [`version`] and report that it [`is_new`] but the value
    /// will not actually change.
    ///
    /// [`version`]: Var::version
    /// [`is_new`]: Var::is_new
    fn touch<Vw: WithVars>(&self, vars: &Vw) -> Result<(), VarIsReadOnly> {
        self.modify(vars, |mut v| v.touch())
    }

    /// Schedule a new value for the variable.
    ///
    /// After the current app update finishes the `new_value` will be set, the variable will have
    /// a new [`version`] and [`is_new`] will be `true` for the next app update.
    ///
    /// [`version`]: Var::version
    /// [`is_new`]: Var::is_new
    fn set<Vw, N>(&self, vars: &Vw, new_value: N) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<T>,
    {
        let new_value = new_value.into();
        self.modify(vars, move |mut v| *v = new_value)
    }

    /// Schedule a new value for the variable, but only if the current value is not equal to `new_value`.
    fn set_ne<Vw, N>(&self, vars: &Vw, new_value: N) -> Result<bool, VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<T>,
        T: PartialEq,
    {
        vars.with_vars(|vars| {
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
        })
    }

    /// Create an animation that targets `self`.
    ///
    /// If the variable is [`always_read_only`] no animation is created, if the variable [`is_contextual`]
    /// the animation is created for the current [`actual_var`]. The animation is dropped the `self` is stopped.
    ///
    /// If the animation can be started `start` is called once with the current value of the variable, it can generate data to
    /// be used during the animation, or cancel by returning `None`. After starting the `animate` closure is called every frame
    /// with the [`AnimationArgs`], the current value of the variable and a mutable reference to the data generated by `start`.
    ///
    /// Returns an [`AnimationHandle`]. See [`Vars::animate`] for more details about animation.
    ///
    /// [`always_read_only`]: Var::always_read_only
    /// [`actual_var`]: Var::actual_var
    /// [`is_contextual`]: Var::is_contextual
    /// [`AnimationArgs`]: animation::AnimationArgs
    fn animate<Vw, D, S, A>(&self, vars: &Vw, start: S, mut animate: A) -> AnimationHandle
    where
        Vw: WithVars,
        D: 'static,
        S: FnOnce(&T) -> Option<D>,
        A: FnMut(&animation::AnimationArgs, &T, &mut D) -> Option<T> + 'static,
    {
        if self.always_read_only() {
            AnimationHandle::dummy()
        } else if self.is_contextual() {
            vars.with_vars(|vars| {
                let actual_var = self.actual_var(vars);
                if let Some(mut data) = start(actual_var.get(vars)) {
                    let mut is_animating = false;
                    let wk_var = BindActualWeak::new(self, actual_var);
                    vars.animate(move |vars, args| {
                        if let Some(var) = wk_var.upgrade() {
                            if let Some(value) = animate(args, var.get(vars), &mut data) {
                                let _ = var.set(vars, value);
                                is_animating = true;
                            }

                            if !is_animating {
                                is_animating = true;
                                // ensure `Var::is_animating` is `true`.
                                let _ = var.touch(vars);
                            }
                        } else {
                            args.stop();
                        }
                    })
                } else {
                    AnimationHandle::dummy()
                }
            })
        } else {
            debug_assert!(self.is_rc());
            let wk_var = self.downgrade().unwrap();

            vars.with_vars(|vars| {
                if let Some(mut data) = start(self.get(vars)) {
                    let mut is_animating = false;
                    vars.animate(move |vars, args| {
                        if let Some(var) = wk_var.upgrade() {
                            if let Some(value) = animate(args, var.get(vars), &mut data) {
                                let _ = var.set(vars, value);
                                is_animating = true;
                            }

                            if !is_animating {
                                is_animating = true;
                                // ensure `Var::is_animating` is `true`.
                                let _ = var.touch(vars);
                            }
                        } else {
                            args.stop();
                        }
                    })
                } else {
                    AnimationHandle::dummy()
                }
            })
        }
    }

    /// Create a transition animation for the variable, starting from the current value transitioning to `new_value`.
    ///
    /// Returns an [`AnimationHandle`]. See [`Var::animate`] for details about animations.
    fn ease<Vw, E, F>(&self, vars: &Vw, new_value: E, duration: Duration, easing: F) -> AnimationHandle
    where
        Vw: WithVars,
        E: Into<T>,
        F: Fn(EasingTime) -> EasingStep + 'static,

        T: Transitionable,
    {
        let mut prev_step = 0.fct();
        self.animate(
            vars,
            |value| Some(Transition::new(value.clone(), new_value.into())),
            move |animation, _, transition| {
                let step = easing(animation.elapsed_stop(duration));
                if step != prev_step {
                    prev_step = step;
                    return Some(transition.sample(step));
                }
                None
            },
        )
    }

    /// Create a transition animation for the variable.  The variable only
    /// updates when the animation yields values that are not equal to the previous value.
    ///
    /// Returns an [`AnimationHandle`]. See [`Var::animate`] for details about animations.
    fn ease_ne<Vw, E, F>(&self, vars: &Vw, new_value: E, duration: Duration, easing: F) -> AnimationHandle
    where
        Vw: WithVars,
        E: Into<T>,
        F: Fn(EasingTime) -> EasingStep + 'static,

        T: PartialEq + Transitionable,
    {
        let mut prev_step = 0.fct();
        self.animate(
            vars,
            |value| Some(Transition::new(value.clone(), new_value.into())),
            move |animation, value, transition| {
                let step = easing(animation.elapsed_stop(duration));
                if step != prev_step {
                    prev_step = step;
                    let new_value = transition.sample(step);
                    if value != &new_value {
                        return Some(new_value);
                    }
                }
                None
            },
        )
    }

    /// Create a transition animation for the variable, starting from `start` transitioning to `end`.
    ///
    /// Returns an [`AnimationHandle`]. See [`Var::animate`] for details about animations.
    fn set_ease<Vw, S, E, F>(&self, vars: &Vw, start: S, end: E, duration: Duration, easing: F) -> AnimationHandle
    where
        Vw: WithVars,
        S: Into<T>,
        E: Into<T>,
        F: Fn(EasingTime) -> EasingStep + 'static,

        T: Transitionable,
    {
        let mut prev_step = 999.fct(); // ensure that we set for `0.fct()`
        self.animate(
            vars,
            |_| Some(Transition::new(start.into(), end.into())),
            move |animation, _, transition| {
                let step = easing(animation.elapsed_stop(duration));
                if step != prev_step {
                    prev_step = step;
                    return Some(transition.sample(step));
                }
                None
            },
        )
    }

    /// Create a transition animation for the variable, starting from `start` transitioning to `end`. The variable only
    /// updates when the animation yields value that are not equal to the previous value.
    ///
    /// Returns an [`AnimationHandle`]. See [`Var::animate`] for details about animations.
    fn set_ease_ne<Vw, S, E, F>(&self, vars: &Vw, start: S, end: E, duration: Duration, easing: F) -> AnimationHandle
    where
        Vw: WithVars,
        S: Into<T>,
        E: Into<T>,
        F: Fn(EasingTime) -> EasingStep + 'static,

        T: PartialEq + Transitionable,
    {
        let mut prev_step = 999.fct(); // ensure that we set for `0.fct()`
        self.animate(
            vars,
            |_| Some(Transition::new(start.into(), end.into())),
            move |animation, value, transition| {
                let step = easing(animation.elapsed_stop(duration));
                if step != prev_step {
                    prev_step = step;
                    let new_value = transition.sample(step);

                    if value != &new_value {
                        return Some(new_value);
                    }
                }
                None
            },
        )
    }

    /// Create a keyframed transition animation for the variable.
    ///
    /// After the current app update finishes the variable will start animating from the current value to the first key
    /// in `keys`, going across all keys for the `duration`. The `easing` function applies across the entire animation,
    /// the base interpolation between keys is linear.
    ///
    /// Returns an [`AnimationHandle`]. See [`Var::animate`] for details about animations.
    fn ease_keyed<Vw, F>(&self, vars: &Vw, mut keys: Vec<(Factor, T)>, duration: Duration, easing: F) -> AnimationHandle
    where
        Vw: WithVars,
        F: Fn(EasingTime) -> EasingStep + 'static,

        T: Transitionable,
    {
        let mut prev_step = 0.fct();
        self.animate(
            vars,
            |value| {
                keys.insert(0, (keys[0].0.min(0.fct()), value.clone()));
                TransitionKeyed::new(keys)
            },
            move |animation, _, transition| {
                let step = easing(animation.elapsed_stop(duration));
                if step != prev_step {
                    prev_step = step;
                    return Some(transition.sample(step));
                }
                None
            },
        )
    }

    /// Schedule a keyframed transition animation for the variable, starting from the first key.
    ///
    /// The variable will be set to to the first keyframe, then animated across all other keys.
    ///
    /// Returns an [`AnimationHandle`]. See [`Var::animate`] for details about animations.
    fn set_ease_keyed<Vw, F>(&self, vars: &Vw, keys: Vec<(Factor, T)>, duration: Duration, easing: F) -> AnimationHandle
    where
        Vw: WithVars,
        F: Fn(EasingTime) -> EasingStep + 'static,

        T: Transitionable,
    {
        let mut prev_step = 0.fct();
        self.animate(
            vars,
            |_| TransitionKeyed::new(keys),
            move |animation, _, transition| {
                let step = easing(animation.elapsed_stop(duration));
                if step != prev_step {
                    prev_step = step;
                    return Some(transition.sample(step));
                }
                None
            },
        )
    }

    /// Set the variable to `new_value` after a `delay`.
    ///
    /// The variable [`is_animating`] until the delay elapses and the value is set.
    ///
    /// Returns an [`AnimationHandle`]. See [`Var::animate`] for details about animations.
    ///
    /// [`is_animating`]: Var::is_animating
    fn step<Vw, N>(&self, vars: &Vw, new_value: N, delay: Duration) -> AnimationHandle
    where
        Vw: WithVars,
        N: Into<T>,
    {
        self.animate(
            vars,
            |_| Some(Some(new_value.into())),
            move |animation, _, new_value| {
                if !animation.animations_enabled() || animation.elapsed_dur() >= delay {
                    animation.stop();
                    new_value.take()
                } else {
                    animation.sleep(delay);
                    None
                }
            },
        )
    }

    /// Set the variable to `new_value` after a `delay`, but only if the new value is not equal to the current value.
    ///
    /// The variable [`is_animating`] until the delay elapses and the value is set.
    ///
    /// Returns an [`AnimationHandle`]. See [`Var::animate`] for details about animations.
    ///
    /// [`is_animating`]: Var::is_animating
    fn step_ne<Vw, N>(&self, vars: &Vw, new_value: N, delay: Duration) -> AnimationHandle
    where
        Vw: WithVars,
        N: Into<T>,
        T: PartialEq,
    {
        self.animate(
            vars,
            |value| {
                let new_value = new_value.into();
                if value != &new_value {
                    Some(Some(new_value))
                } else {
                    None
                }
            },
            move |animation, _, new_value| {
                if !animation.animations_enabled() || animation.elapsed_dur() >= delay {
                    animation.stop();
                    new_value.take()
                } else {
                    animation.sleep(delay);
                    None
                }
            },
        )
    }

    /// Set the variable to a sequence of values as a time `duration` elapses.
    ///
    /// An animation curve is used to find the first factor in `steps` above or at the curve line at the current time,
    /// the variable is set to this step value, continuing animating across the next steps until the last or the animation end.
    /// The variable [`is_animating`] from the start, even if no step applies and stays *animating* until the last *step* applies
    /// or the duration is reached.
    ///
    /// # Examples
    ///
    /// Creates a variable that outputs text every 5% of a 5 seconds animation, advanced linearly.
    ///
    /// ```
    /// # use zero_ui_core::{var::*, units::*, text::*};
    /// # fn demo(text_var: impl Var<Text>, vars: &Vars) {
    /// let steps = (0..=100).step_by(5).map(|i| (i.pct().fct(), formatx!("{i}%"))).collect();
    /// # let _ =
    /// text_var.steps(vars, steps, 5.secs(), easing::linear)
    /// # ;}
    /// ```
    ///
    /// The variable is set to `"0%"`, after 5% of the `duration` elapses it is set to `"5%"` and so on
    /// until the value is set to `"100%` at the end of the animation.
    ///
    /// Returns an [`AnimationHandle`]. See [`Var::animate`] for details about animations.
    ///
    /// [`is_animating`]: Var::is_animating
    fn steps<Vw, F>(&self, vars: &Vw, steps: Vec<(Factor, T)>, duration: Duration, easing: F) -> AnimationHandle
    where
        Vw: WithVars,
        F: Fn(EasingTime) -> EasingStep + 'static,
    {
        let mut prev_step = 999.fct();
        self.animate(
            vars,
            |_| Some(()),
            move |animation, _, _| {
                let step = easing(animation.elapsed_stop(duration));
                if step != prev_step {
                    prev_step = step;
                    steps.iter().find(|(f, _)| *f >= step).map(|(_, step)| step.clone())
                } else {
                    None
                }
            },
        )
    }

    /// Same behavior as [`steps`], but checks for equality before setting each step.
    ///
    /// [`steps`]: Var::steps
    fn steps_ne<Vw, F>(&self, vars: &Vw, steps: Vec<(Factor, T)>, duration: Duration, easing: F) -> AnimationHandle
    where
        Vw: WithVars,
        F: Fn(EasingTime) -> EasingStep + 'static,
        T: PartialEq,
    {
        let mut prev_step = 999.fct();
        self.animate(
            vars,
            |_| Some(()),
            move |animation, value, _| {
                let step = easing(animation.elapsed_stop(duration));
                if step != prev_step {
                    prev_step = step;
                    if let Some((_, new_value)) = steps.iter().find(|(f, _)| *f >= step) {
                        if value != new_value {
                            return Some(new_value.clone());
                        }
                    }
                }
                None
            },
        )
    }

    /// Starts an easing animation that *chases* a target value that can be changed using the [`ChaseAnimation`] handle.
    fn chase<Vw, F>(&self, vars: &Vw, first_target: T, duration: Duration, easing: F) -> ChaseAnimation<T>
    where
        Vw: WithVars,
        F: Fn(EasingTime) -> EasingStep + 'static,
        T: Transitionable,
    {
        let mut prev_step = 0.fct();
        let next_target = Rc::new(RefCell::new(ChaseMsg::None));
        let handle = self.animate(
            vars,
            |value| Some(Transition::new(value.clone(), first_target)),
            clone_move!(next_target, |animation, _, transition: &mut Transition<T>| {
                let step = easing(animation.elapsed_stop(duration));
                match mem::take(&mut *next_target.borrow_mut()) {
                    ChaseMsg::Add(inc) => {
                        animation.restart();
                        let from = transition.sample(step);
                        transition.start = from.clone();
                        transition.increment += inc;
                        if step != prev_step {
                            prev_step = step;
                            return Some(from);
                        }
                    }
                    ChaseMsg::Replace(new_target) => {
                        animation.restart();
                        let from = transition.sample(step);
                        *transition = Transition::new(from.clone(), new_target);
                        if step != prev_step {
                            prev_step = step;
                            return Some(from);
                        }
                    }
                    ChaseMsg::None => {
                        if step != prev_step {
                            prev_step = step;
                            return Some(transition.sample(step));
                        }
                    }
                }
                None
            }),
        );
        ChaseAnimation { handle, next_target }
    }

    /// Starts a [`chase`] animation that eases to a target value, but does not escape `bounds`.
    ///
    /// [`chase`]: Var::chase
    fn chase_bounded<Vw, F>(
        &self,
        vars: &Vw,
        first_target: T,
        duration: Duration,
        easing: F,
        bounds: ops::RangeInclusive<T>,
    ) -> ChaseAnimation<T>
    where
        Vw: WithVars,
        F: Fn(EasingTime) -> EasingStep + 'static,
        T: Transitionable + std::cmp::PartialOrd<T>,
    {
        let mut prev_step = 0.fct();
        let mut check_linear = !bounds.contains(&first_target);

        let next_target = Rc::new(RefCell::new(ChaseMsg::None));
        let handle = self.animate(
            vars,
            |value| Some(Transition::new(value.clone(), first_target)),
            clone_move!(next_target, |animation, _, transition: &mut Transition<T>| {
                let mut time = animation.elapsed_stop(duration);
                let mut step = easing(time);
                match mem::take(&mut *next_target.borrow_mut()) {
                    // to > bounds
                    // stop animation when linear sampling > bounds
                    ChaseMsg::Add(inc) => {
                        animation.restart();

                        let partial_inc = transition.increment.clone() * step;
                        let from = transition.start.clone() + partial_inc.clone();
                        let to = from.clone() + transition.increment.clone() - partial_inc + inc;

                        check_linear = !bounds.contains(&to);

                        *transition = Transition::new(from, to);

                        step = 0.fct();
                        prev_step = 1.fct();
                        time = EasingTime::start();
                    }
                    ChaseMsg::Replace(new_target) => {
                        animation.restart();
                        let from = transition.sample(step);

                        check_linear = !bounds.contains(&new_target);

                        *transition = Transition::new(from, new_target);

                        step = 0.fct();
                        prev_step = 1.fct();
                        time = EasingTime::start();
                    }
                    ChaseMsg::None => {
                        // normal execution
                    }
                }

                if step != prev_step {
                    prev_step = step;

                    if check_linear {
                        let linear_sample = transition.sample(time.fct());
                        if &linear_sample > bounds.end() {
                            animation.stop();
                            return Some(bounds.end().clone());
                        } else if &linear_sample < bounds.start() {
                            animation.stop();
                            return Some(bounds.start().clone());
                        }
                    }
                    Some(transition.sample(step))
                } else {
                    None
                }
            }),
        );
        ChaseAnimation { handle, next_target }
    }

    /// Wraps the variable into another that turns assigns into transition animations.
    ///
    /// Redirects calls to [`Var::set`] to [`Var::ease`] and [`Var::set_ne`] to [`Var::ease_ne`], calls to the
    /// animation methods are not affected by the var easing.
    fn easing<F>(self, duration: Duration, easing: F) -> animation::EasingVar<T, Self, F>
    where
        F: Fn(EasingTime) -> EasingStep + 'static,

        T: Transitionable,
    {
        animation::EasingVar::new(self, duration, easing)
    }

    /// Box this var.
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
    /// Also see [`Var::bind_map`] to create a *map binding* between two existing variables.
    fn map<O, M>(&self, map: M) -> DefaultMapVar![T, O, M, Self]
    where
        O: VarValue,
        M: FnMut(&T) -> O + 'static,
    {
        #[cfg(dyn_closure)]
        return types::RcMapVar::new(self.clone().boxed(), Box::new(map));

        #[cfg(not(dyn_closure))]
        types::RcMapVar::new(self.clone(), map)
    }

    /// Create a [`map`](Var::map) that uses [`Into`] to convert from `T` to `O`.
    fn map_into<O>(&self) -> DefaultMapVar![T, O, fn(&T) -> O, Self]
    where
        O: VarValue + From<T>,
    {
        self.map(|t| t.clone().into())
    }

    /// Create a [`map`](Var::map) that uses [`ToText`](crate::text::ToText) to convert `T` to [`Text`](crate::text::ToText).
    fn map_to_text(&self) -> DefaultMapVar![T, crate::text::Text, fn(&T) -> crate::text::Text, Self]
    where
        T: crate::text::ToText,
    {
        self.map(|t| t.to_text())
    }

    /// Create a [`map`](Var::map) that maps to a debug [`Text`](crate::text::ToText) using the `{:?}` format.
    fn map_debug(&self) -> DefaultMapVar![T, crate::text::Text, fn(&T) -> crate::text::Text, Self] {
        self.map(|t| crate::formatx!("{t:?}"))
    }

    /// Create a read-only variable with a value that is dereferenced from this variable value.
    ///
    /// This is a lightweight alternative to [`map`](Var::map) that can be used when the *mapped* value already
    /// exist in the source variable, `map` is called every time the mapped value is accessed.
    fn map_ref<O, M>(&self, map: M) -> types::MapRefVar<T, O, M, Self>
    where
        O: VarValue,
        M: Fn(&T) -> &O + 'static,
    {
        types::MapRefVar::new(self.clone(), map)
    }

    /// Create a read-write variable with a value that is mapped from and to this variable.
    ///
    /// The value of the map variable is kept up-to-date with the value of this variable, `map` is
    /// called every time the value needs to update. When the mapped variable is assigned, `map_back` is
    /// called to generate a value that is assigned back to this variable.
    ///
    /// Also see [`bind_map_bidi`](Var::bind_map_bidi) to create a *map binding* between two existing variables.
    fn map_bidi<O, M, N>(&self, map: M, map_back: N) -> types::RcMapBidiVar<T, O, M, N, Self>
    where
        O: VarValue,
        M: FnMut(&T) -> O + 'static,
        N: FnMut(O) -> T + 'static,
    {
        types::RcMapBidiVar::new(self.clone(), map, map_back)
    }

    /// Create a [`map_bidi`](Var::map_bidi) that uses [`Into`] to convert between `T` and `O`.
    #[allow(clippy::type_complexity)]
    fn map_into_bidi<O>(&self) -> types::RcMapBidiVar<T, O, fn(&T) -> O, fn(O) -> T, Self>
    where
        O: VarValue + From<T>,
        T: From<O>,
    {
        self.map_bidi(|t| t.clone().into(), |o| o.into())
    }

    /// Create a read-write variable with a value that is dereferenced from this variable value.
    ///
    /// This is a lightweight alternative to [`map_bidi`](Var::map_bidi) that can be used when the *mapped* value already
    /// exist in the source variable, `map` is called every time the mapped value is accessed and `map_mut` is called
    /// to get a mutable reference to the value when the mapped variable is assigned.
    fn map_ref_bidi<O, M, N>(&self, map: M, map_mut: N) -> types::MapBidiRefVar<T, O, M, N, Self>
    where
        O: VarValue,
        M: Fn(&T) -> &O + 'static,
        N: Fn(&mut T) -> &mut O + 'static,
    {
        types::MapBidiRefVar::new(self.clone(), map, map_mut)
    }

    /// Map to a variable selected from the value of `self`. The result variable outputs the
    /// value of the selected value but it updates when both `self` and the selected variable updates.
    ///
    /// If the selected variable can be modified setting the result variable sets the selected variable.
    fn flat_map<O, M, V>(&self, map: M) -> types::RcFlatMapVar<T, O, V, M, Self>
    where
        O: VarValue,
        V: Var<O>,
        M: FnMut(&T) -> V + 'static,
    {
        types::RcFlatMapVar::new(self.clone(), map)
    }

    /// Create a read-only variable with a value that is mapped from this variable, but only if it passes a filter.
    ///
    /// The value of the map variable is kept up-to-date with the value of this variable, `map` is called every
    /// time the value needs to update, if it returns `Some(T)` the mapped variable value updates.
    ///
    /// The `fallback_init` can be called once if the first call to `map` returns `None`, it must return a *fallback* initial value.
    ///
    /// Also see [`bind_filter`](Var::bind_filter) to create a *map binding* between two existing variables.
    fn filter_map<O, I, M>(&self, fallback_init: I, map: M) -> types::RcFilterMapVar<T, O, I, M, Self>
    where
        O: VarValue,
        I: FnOnce(&T) -> O + 'static,
        M: FnMut(&T) -> Option<O> + 'static,
    {
        types::RcFilterMapVar::new(self.clone(), fallback_init, map)
    }

    /// Create a [`filter_map`] that uses [`TryInto`] to convert from `T` to `O`.
    ///
    /// [`filter_map`]: Var::filter_map
    #[allow(clippy::type_complexity)]
    fn filter_try_into<O, I>(&self, fallback_init: I) -> types::RcFilterMapVar<T, O, I, fn(&T) -> Option<O>, Self>
    where
        O: VarValue + TryFrom<T>,
        I: FnOnce(&T) -> O + 'static,
    {
        types::RcFilterMapVar::new(self.clone(), fallback_init, |v| v.clone().try_into().ok())
    }

    /// Create a [`filter_map`] that uses [`FromStr`] to convert from `T` to `O`.
    ///
    /// [`filter_map`]: Var::filter_map
    #[allow(clippy::type_complexity)]
    fn filter_parse<O, I>(&self, fallback_init: I) -> types::RcFilterMapVar<T, O, I, fn(&T) -> Option<O>, Self>
    where
        O: VarValue + FromStr,
        T: AsRef<str>,
        I: FnOnce(&T) -> O + 'static,
    {
        types::RcFilterMapVar::new(self.clone(), fallback_init, |v| v.as_ref().parse().ok())
    }

    /// Create a [`filter_map`](Var::filter_map) that passes when `T` is [`Ok`].
    #[allow(clippy::type_complexity)]
    fn filter_ok<O, I>(&self, fallback_init: I) -> types::RcFilterMapVar<T, O, I, fn(&T) -> Option<O>, Self>
    where
        O: VarValue,
        I: FnOnce(&T) -> O + 'static,
        T: ResultOk<O>,
    {
        self.filter_map(fallback_init, |t| t.r_ok().cloned())
    }

    /// Create a [`filter_map`](Var::filter_map) that passes when `T` is [`Ok`] and maps the result to [`Text`].
    #[allow(clippy::type_complexity)]
    fn filter_ok_text<I>(&self, fallback_init: I) -> types::RcFilterMapVar<T, Text, I, fn(&T) -> Option<Text>, Self>
    where
        I: FnOnce(&T) -> Text + 'static,
        T: ResultOkText,
    {
        self.filter_map(fallback_init, |t| t.r_ok_text())
    }

    /// Create a [`filter_map`](Var::filter_map) that passes when `T` is [`Err`].
    #[allow(clippy::type_complexity)]
    fn filter_err<O, I>(&self, fallback_init: I) -> types::RcFilterMapVar<T, O, I, fn(&T) -> Option<O>, Self>
    where
        O: VarValue,
        I: FnOnce(&T) -> O + 'static,
        T: ResultErr<O>,
    {
        self.filter_map(fallback_init, |t| t.r_err().cloned())
    }

    /// Create a [`filter_map`](Var::filter_map) that passes when `T` is [`Err`] and maps the error to [`Text`].
    #[allow(clippy::type_complexity)]
    fn filter_err_text<I>(&self, fallback_init: I) -> types::RcFilterMapVar<T, Text, I, fn(&T) -> Option<Text>, Self>
    where
        I: FnOnce(&T) -> Text + 'static,
        T: ResultErrText,
    {
        self.filter_map(fallback_init, |t| t.r_err_text())
    }

    /// Create a [`filter_map`](Var::filter_map) that passes when `T` is [`Some`].
    #[allow(clippy::type_complexity)]
    fn filter_some<O, I>(&self, fallback_init: I) -> types::RcFilterMapVar<T, O, I, fn(&T) -> Option<O>, Self>
    where
        O: VarValue,
        I: FnOnce(&T) -> O + 'static,
        T: OptionSome<O>,
    {
        self.filter_map(fallback_init, |t| t.opt_some().cloned())
    }

    /// Create a [`filter_map`](Var::filter_map) that passes when `T` is [`Some`] and convert the value to [`Text`].
    #[allow(clippy::type_complexity)]
    fn filter_some_text<I>(&self, fallback_init: I) -> types::RcFilterMapVar<T, Text, I, fn(&T) -> Option<Text>, Self>
    where
        I: FnOnce(&T) -> Text + 'static,
        T: OptionSomeText,
    {
        self.filter_map(fallback_init, |t| t.opt_some_text())
    }

    /// Create a read-write variable with a value that is mapped from and to this variable, but only if the values pass the filters.
    ///
    /// The value of the map variable is kept up-to-date with the value of this variable, `map` is
    /// called every time the value needs to update, if it returns `Some(T)` the mapped variable value updates.
    ///
    /// When the mapped variable is assigned, `map_back` is called, if it returns `Some(T)` the value is assigned back to this variable.
    ///
    /// Also see [`bind_filter_bidi`](Var::bind_filter_bidi) to create a *map binding* between two existing variables.
    fn filter_map_bidi<O, I, M, N>(&self, fallback_init: I, map: M, map_back: N) -> types::RcFilterMapBidiVar<T, O, I, M, N, Self>
    where
        O: VarValue,
        I: FnOnce(&T) -> O + 'static,
        M: FnMut(&T) -> Option<O> + 'static,
        N: FnMut(O) -> Option<T> + 'static,
    {
        types::RcFilterMapBidiVar::new(self.clone(), fallback_init, map, map_back)
    }

    /// Create a [`filter_map_bidi`] that uses [`TryInto`] to convert between `T` and `O`.
    ///
    /// [`filter_map_bidi`]: Var::filter_map_bidi
    #[allow(clippy::type_complexity)]
    fn filter_try_into_bidi<O, I>(
        &self,
        fallback_init: I,
    ) -> types::RcFilterMapBidiVar<T, O, I, fn(&T) -> Option<O>, fn(O) -> Option<T>, Self>
    where
        O: VarValue + TryFrom<T>,
        I: FnOnce(&T) -> O + 'static,
        T: TryFrom<O>,
    {
        types::RcFilterMapBidiVar::new(self.clone(), fallback_init, |t| t.clone().try_into().ok(), |o| o.try_into().ok())
    }

    /// Create a [`filter_map_bidi`] that uses [`FromStr`] to convert from `T` to `O` and [`ToText`] to convert from `O` to `T`.
    ///
    /// The `fallback_init` is called to generate a value if the first [`str::parse`] call fails.
    ///
    /// [`filter_map_bidi`]: Var::filter_map_bidi
    #[allow(clippy::type_complexity)]
    fn filter_parse_bidi<O, I>(&self, fallback_init: I) -> types::RcFilterMapBidiVar<T, O, I, fn(&T) -> Option<O>, fn(O) -> Option<T>, Self>
    where
        O: VarValue + FromStr + ToText,
        I: FnOnce(&T) -> O + 'static,
        T: AsRef<str> + From<Text>,
    {
        types::RcFilterMapBidiVar::new(
            self.clone(),
            fallback_init,
            |t| t.as_ref().parse().ok(),
            |o| Some(o.to_text().into()),
        )
    }

    /// Create a [`filter_map_bidi`] that uses [`FromStr`] to convert from `O` to `T` and [`ToText`] to convert from `T` to `O`.
    ///
    /// [`filter_map_bidi`]: Var::filter_map_bidi
    #[allow(clippy::type_complexity)]
    fn filter_to_text_bidi<O>(&self) -> types::RcFilterMapBidiVar<T, O, fn(&T) -> O, fn(&T) -> Option<O>, fn(O) -> Option<T>, Self>
    where
        O: VarValue + AsRef<str> + From<Text>,
        T: FromStr + ToText,
    {
        types::RcFilterMapBidiVar::new(
            self.clone(),
            |_| unreachable!(),
            |t| Some(t.to_text().into()),
            |o| o.as_ref().parse().ok(),
        )
    }

    /// Creates a sender that can set `self` from other threads and without access to [`Vars`].
    ///
    /// If the variable is read-only when a value is received it is silently dropped.
    ///
    /// Drop the sender to release one reference to `self`.
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
    fn modify_sender<Vw: WithVars>(&self, vars: &Vw) -> VarModifySender<T> {
        vars.with_vars(|vars| vars.modify_sender(self))
    }

    /// Creates a channel that can receive `var` updates from another thread.
    ///
    /// Every time the variable updates a clone of the value is sent to the receiver. The current value is sent immediately.
    ///
    /// Drop the receiver to release one reference to `var`.
    fn receiver<Vr>(&self, vars: &Vr) -> VarReceiver<T>
    where
        T: Send,
        Vr: WithVarsRead,
    {
        vars.with_vars_read(|vars| vars.receiver(self))
    }

    /// Create a *map* like binding between two existing variables.
    ///
    /// The binding flows from `self` to `to_var`, every time `self` updates `map` is called to generate a value that is assigned `to_var`.
    ///
    /// Both `self` and `to_var` notify a new value in the same app update, this is different then a manually implemented *binding*
    /// where the assign to `to_var` would cause a second update.
    ///
    /// Note that the current value is **not** transferred just by creating a binding, only subsequent updates of `self` will
    /// assign `to_var`. No binding is set if `self` is not [`can_update`] or if `to_var` is [`always_read_only`]. If either
    /// `self` or `to_var` are [`is_contextual`] the binding is set on the [`actual_var`] of both. The binding is dropped
    /// if `self` or `to_var` are dropped.
    ///
    /// Returns a [`VarBindingHandle`]. See [`Vars::bind`] for more details about variable binding.
    ///
    /// [`can_update`]: Var::can_update
    /// [`always_read_only`]: Var::always_read_only
    /// [`is_contextual`]: Var::is_contextual
    /// [`actual_var`]: Var::actual_var
    fn bind_map<Vw, T2, V2, M>(&self, vars: &Vw, to_var: &V2, mut map: M) -> VarBindingHandle
    where
        Vw: WithVars,
        T2: VarValue,
        V2: Var<T2>,
        M: FnMut(&VarBinding, &T) -> T2 + 'static,
    {
        if !self.can_update() || to_var.always_read_only() {
            VarBindingHandle::dummy()
        } else if self.is_contextual() || to_var.is_contextual() {
            vars.with_vars(|vars| {
                // bind the actual vars.

                let actual_from = self.actual_var(vars);
                let actual_to = to_var.actual_var(vars);

                debug_assert!(actual_from.is_rc());
                debug_assert!(actual_to.is_rc());

                // ensure that `actual_var` remaps are not just dropped immediately.
                let wk_from_var = BindActualWeak::new(self, actual_from);
                let wk_to_var = BindActualWeak::new(to_var, actual_to);

                vars.bind(move |vars, binding| match (wk_from_var.upgrade(), wk_to_var.upgrade()) {
                    (Some(from), Some(to)) => {
                        if let Some(new_value) = from.get_new(vars) {
                            let new_value = map(binding, new_value);
                            let _ = to.set(vars, new_value);
                        }
                    }
                    _ => binding.unbind(),
                })
            })
        } else {
            // self can-update, to_var can be set and both are not contextual.

            debug_assert!(self.is_rc());
            debug_assert!(to_var.is_rc());

            let wk_from_var = self.downgrade().unwrap();
            let wk_to_var = to_var.downgrade().unwrap();

            vars.with_vars(|vars| {
                vars.bind(move |vars, binding| match (wk_from_var.upgrade(), wk_to_var.upgrade()) {
                    (Some(from), Some(to)) => {
                        if let Some(new_value) = from.get_new(vars) {
                            let new_value = map(binding, new_value);
                            let _ = to.set(vars, new_value);
                        }
                    }
                    _ => binding.unbind(),
                })
            })
        }
    }

    /// Create a [`bind_map`] that uses clones the value for `to_var`.
    ///
    /// [`bind_map`]: Var::bind_map
    fn bind<Vw, V2>(&self, vars: &Vw, to_var: &V2) -> VarBindingHandle
    where
        Vw: WithVars,
        V2: Var<T>,
    {
        self.bind_map(vars, to_var, |_, v| v.clone())
    }

    /// Create a [`bind_map`] that uses [`Into`] to convert `T` to `T2`.
    ///
    /// [`bind_map`]: Var::bind_map
    fn bind_into<Vw, T2, V2>(&self, vars: &Vw, to_var: &V2) -> VarBindingHandle
    where
        Vw: WithVars,
        T2: VarValue + From<T>,
        V2: Var<T2>,
    {
        self.bind_map(vars, to_var, |_, v| v.clone().into())
    }

    /// Create a [`bind_map`] that uses [`ToText`] to convert `T` to [`Text`].
    ///
    /// [`bind_map`]: Var::bind_map
    /// [`ToText`]: crate::text::ToText
    /// [`Text`]: crate::text::Text
    fn bind_to_text<Vw, V>(&self, vars: &Vw, text_var: &V) -> VarBindingHandle
    where
        Vw: WithVars,
        V: Var<Text>,
        T: crate::text::ToText,
    {
        self.bind_map(vars, text_var, |_, v| v.to_text())
    }

    /// Create a *map_bidi* like binding between two existing variables.
    ///
    /// The bindings **maps** from `self` to `other_var` and **maps-back** from `other_var` to `self`.
    /// Every time `self` updates `map` is called to generate a value that is assigned to `other_var` and every time `other_var`
    /// updates `map_back` is called to generate a value that is assigned back to `self`.
    ///
    /// Both `self` and `other_var` notify a new value in the same app update, this is different then a manually implemented *binding*
    /// when the assign to the second variable would cause a second update.
    ///
    /// Note that the current value is **not** transferred just by creating a binding, only subsequent updates will assigns variables.
    /// No binding is set if `self` or `other_var` are not [`can_update`] or are [`always_read_only`]. If either
    /// `self` or `to_var` are [`is_contextual`] the binding is set on the [`actual_var`] of both. The binding is dropped
    /// if `self` or `to_var` are dropped.
    ///
    /// Returns a [`VarBindingHandle`]. See [`Vars::bind`] for more details about variable binding.
    ///
    /// [`can_update`]: Var::can_update
    /// [`always_read_only`]: Var::always_read_only
    /// [`is_contextual`]: Var::is_contextual
    /// [`actual_var`]: Var::actual_var
    fn bind_map_bidi<Vw, T2, V2, M, N>(&self, vars: &Vw, other_var: &V2, mut map: M, mut map_back: N) -> VarBindingHandle
    where
        Vw: WithVars,
        T2: VarValue,
        V2: Var<T2>,
        M: FnMut(&VarBinding, &T) -> T2 + 'static,
        N: FnMut(&VarBinding, &T2) -> T + 'static,
    {
        if !self.can_update() {
            // self cannot generate new values (and cannot be set)

            debug_assert!(self.always_read_only());
            return VarBindingHandle::dummy();
        }
        if !other_var.can_update() {
            // other cannot generate new values (and cannot be set)

            debug_assert!(other_var.always_read_only());
            return VarBindingHandle::dummy();
        }

        if self.always_read_only() {
            // self cannot be set, so this is, maybe, only a `bind_map`
            return self.bind_map(vars, other_var, map);
        }

        if other_var.always_read_only() {
            // other_var cannot be set, so this is, maybe, only a *bind_map_back*
            return other_var.bind_map(vars, self, map_back);
        }

        if self.is_contextual() || other_var.is_contextual() {
            return vars.with_vars(|vars| {
                // bind the actual vars.

                let actual_self = self.actual_var(vars);
                let actual_other = other_var.actual_var(vars);

                debug_assert!(actual_self.is_rc());
                debug_assert!(actual_other.is_rc());

                // ensure that `actual_var` remaps are not just dropped immediately.
                let wk_self_var = BindActualWeak::new(self, actual_self);
                let wk_other_var = BindActualWeak::new(other_var, actual_other);

                vars.bind(move |vars, binding| match (wk_self_var.upgrade(), wk_other_var.upgrade()) {
                    (Some(self_var), Some(other_var)) => {
                        if let Some(new_value) = self_var.get_new(vars) {
                            let new_value = map(binding, new_value);
                            let _ = other_var.set(vars, new_value);
                        }
                        if let Some(new_value) = other_var.get_new(vars) {
                            let new_value = map_back(binding, new_value);
                            let _ = self_var.set(vars, new_value);
                        }
                    }
                    _ => binding.unbind(),
                })
            });
        }

        debug_assert!(self.is_rc());
        debug_assert!(other_var.is_rc());

        let wk_self_var = self.downgrade().unwrap();
        let wk_other_var = other_var.downgrade().unwrap();

        vars.with_vars(|vars| {
            vars.bind(move |vars, binding| match (wk_self_var.upgrade(), wk_other_var.upgrade()) {
                (Some(from_var), Some(to_var)) => {
                    if let Some(new_value) = from_var.get_new(vars) {
                        let new_value = map(binding, new_value);
                        let _ = to_var.set(vars, new_value);
                    }
                    if let Some(new_value) = to_var.get_new(vars) {
                        let new_value = map_back(binding, new_value);
                        let _ = from_var.set(vars, new_value);
                    }
                }
                _ => binding.unbind(),
            })
        })
    }

    /// Create a [`bind_map_bidi`] that clones values in between `self` and `other_var`.
    ///
    /// [`bind_map_bidi`]: Var::bind_map_bidi
    fn bind_bidi<Vw, V2>(&self, vars: &Vw, other_var: &V2) -> VarBindingHandle
    where
        Vw: WithVars,
        V2: Var<T>,
    {
        self.bind_map_bidi(vars, other_var, |_, v| v.clone(), |_, v| v.clone())
    }

    /// Create a [`bind_map_bidi`] that uses [`Into`] to convert between `self` and `other_var`.
    ///
    /// [`bind_map_bidi`]: Var::bind_map_bidi
    fn bind_into_bidi<Vw, T2, V2>(&self, vars: &Vw, other_var: &V2) -> VarBindingHandle
    where
        Vw: WithVars,
        T2: VarValue + From<T>,
        V2: Var<T2>,
        T: From<T2>,
    {
        self.bind_map_bidi(vars, other_var, |_, t| t.clone().into(), |_, t2| t2.clone().into())
    }

    /// Create a *filter_map* like binding between two existing variables.
    ///
    /// The binding flows from `self` to `to_var`, every time `self` updates `map` is called to generate a value, if it does, that value
    /// is assigned `to_var`.
    ///
    /// Both `self` and `to_var` notify a new value in the same app update, this is different then a manually implemented *binding*
    /// where the assign to `to_var` would cause a second update.
    ///
    /// Note that the current value is **not** transferred just by creating a binding, only subsequent updates of `self` will
    /// assign `to_var`. No binding is set if `self` is not [`can_update`] or if `to_var` is [`always_read_only`]. If either
    /// `self` or `to_var` are [`is_contextual`] the binding is set on the [`actual_var`] of both. The binding only holds weak
    /// references to the vars, if any is dropped the binding is also dropped.
    ///
    /// Returns a [`VarBindingHandle`]. See [`Vars::bind`] for more details about variable binding.
    ///
    /// [`can_update`]: Var::can_update
    /// [`always_read_only`]: Var::always_read_only
    /// [`is_contextual`]: Var::is_contextual
    /// [`actual_var`]: Var::actual_var
    fn bind_filter<Vw, T2, V2, M>(&self, vars: &Vw, to_var: &V2, mut map: M) -> VarBindingHandle
    where
        Vw: WithVars,
        T2: VarValue,
        V2: Var<T2>,
        M: FnMut(&VarBinding, &T) -> Option<T2> + 'static,
    {
        if !self.can_update() || to_var.always_read_only() {
            VarBindingHandle::dummy()
        } else if self.is_contextual() || to_var.is_contextual() {
            vars.with_vars(|vars| {
                // bind the actual vars.

                let actual_from = self.actual_var(vars);
                let actual_to = to_var.actual_var(vars);

                debug_assert!(actual_from.is_rc());
                debug_assert!(actual_to.is_rc());

                // ensure that `actual_var` remaps are not just dropped immediately.
                let wk_from_var = BindActualWeak::new(self, actual_from);
                let wk_to_var = BindActualWeak::new(to_var, actual_to);

                vars.bind(move |vars, binding| match (wk_from_var.upgrade(), wk_to_var.upgrade()) {
                    (Some(from), Some(to)) => {
                        if let Some(new_value) = from.get_new(vars) {
                            if let Some(new_value) = map(binding, new_value) {
                                let _ = to.set(vars, new_value);
                            }
                        }
                    }
                    _ => binding.unbind(),
                })
            })
        } else {
            // self can-update, to_var can be set and both are not contextual.

            debug_assert!(self.is_rc());
            debug_assert!(to_var.is_rc());

            let wk_from_var = self.downgrade().unwrap();
            let wk_to_var = to_var.downgrade().unwrap();

            vars.with_vars(|vars| {
                vars.bind(move |vars, binding| match (wk_from_var.upgrade(), wk_to_var.upgrade()) {
                    (Some(from), Some(to)) => {
                        if let Some(new_value) = from.get_new(vars) {
                            if let Some(new_value) = map(binding, new_value) {
                                let _ = to.set(vars, new_value);
                            }
                        }
                    }
                    _ => binding.unbind(),
                })
            })
        }
    }

    /// Create a [`bind_filter`] that uses [`TryInto`] to convert from `self` to `to_var`.
    ///
    /// [`bind_filter`]: Var::bind_filter
    fn bind_try_into<Vw, T2, V2>(&self, vars: &Vw, to_var: &V2) -> VarBindingHandle
    where
        Vw: WithVars,
        T2: VarValue + TryFrom<T>,
        V2: Var<T2>,
    {
        self.bind_filter(vars, to_var, |_, t| t.clone().try_into().ok())
    }

    /// Create a [`bind_filter`] that uses [`FromStr`] to convert from `self` to `to_var`.
    ///
    /// [`bind_filter`]: Var::bind_filter
    fn bind_parse<Vw, T2, V2>(&self, vars: &Vw, parsed_var: &V2) -> VarBindingHandle
    where
        Vw: WithVars,
        T2: VarValue + FromStr,
        V2: Var<T2>,
        T: AsRef<str>,
    {
        self.bind_filter(vars, parsed_var, |_, t| t.as_ref().parse().ok())
    }

    /// Create a [`bind_filter`] that sets `to_var` when `T` is [`Ok`].
    ///
    /// [`bind_filter`]: Var::bind_filter
    fn bind_ok<Vw, T2, V2>(&self, vars: &Vw, to_var: &V2) -> VarBindingHandle
    where
        Vw: WithVars,
        T2: VarValue,
        V2: Var<T2>,
        T: ResultOk<T2>,
    {
        self.bind_filter(vars, to_var, |_, t| t.r_ok().cloned())
    }

    /// Create a [`bind_filter`] that sets `text_var` when `T` is [`Ok`].
    ///
    /// [`bind_filter`]: Var::bind_filter
    fn bind_ok_text<Vw, V2>(&self, vars: &Vw, text_var: &V2) -> VarBindingHandle
    where
        Vw: WithVars,
        V2: Var<Text>,
        T: ResultOkText,
    {
        self.bind_filter(vars, text_var, |_, t| t.r_ok_text())
    }

    /// Create a [`bind_filter`] that sets `to_var` when `T` is [`Err`].
    ///
    /// [`bind_filter`]: Var::bind_filter
    fn bind_err<Vw, T2, V2>(&self, vars: &Vw, to_var: &V2) -> VarBindingHandle
    where
        Vw: WithVars,
        T2: VarValue,
        V2: Var<T2>,
        T: ResultErr<T2>,
    {
        self.bind_filter(vars, to_var, |_, t| t.r_err().cloned())
    }

    /// Create a [`bind_filter`] that sets `text_var` when `T` is [`Err`].
    ///
    /// [`bind_filter`]: Var::bind_filter
    fn bind_err_text<Vw, V2>(&self, vars: &Vw, text_var: &V2) -> VarBindingHandle
    where
        Vw: WithVars,
        V2: Var<Text>,
        T: ResultErrText,
    {
        self.bind_filter(vars, text_var, |_, t| t.r_err_text())
    }

    /// Create a [`bind_filter`] that sets `to_var` when `T` is [`Some`].
    ///
    /// [`bind_filter`]: Var::bind_filter
    fn bind_some<Vw, T2, V2>(&self, vars: &Vw, to_var: &V2) -> VarBindingHandle
    where
        Vw: WithVars,
        T2: VarValue,
        V2: Var<T2>,
        T: OptionSome<T2>,
    {
        self.bind_filter(vars, to_var, |_, t| t.opt_some().cloned())
    }

    /// Create a [`bind_filter`] that sets `text_var` when `T` is [`Some`].
    ///
    /// [`bind_filter`]: Var::bind_filter
    fn bind_some_text<Vw, V2>(&self, vars: &Vw, text_var: &V2) -> VarBindingHandle
    where
        Vw: WithVars,
        V2: Var<Text>,
        T: OptionSomeText,
    {
        self.bind_filter(vars, text_var, |_, t| t.opt_some_text())
    }

    /// Create a *filter_map_bidi* like binding between two existing variables.
    ///
    /// The bindings **maps** from `self` to `other_var` and **maps-back** from `other_var` to `self`.
    /// Every time `self` updates `map` is called to generate a value that is assigned to `other_var` and every time `other_var`
    /// updates `map_back` is called to generate a value that is assigned back to `self`. In both cases the second variable only
    /// updates if the map function returns a value.
    ///
    /// Both `self` and `other_var` notify a new value in the same app update, this is different then a manually implemented *binding*
    /// when the assign to the second variable would cause a second update.
    ///
    /// Note that the current value is **not** transferred just by creating a binding, only subsequent updates will assigns variables.
    /// No binding is set if `self` or `other_var` are not [`can_update`] or are [`always_read_only`]. If either
    /// `self` or `to_var` are [`is_contextual`] the binding is set on the [`actual_var`] of both. The binding only holds weak
    /// references to the vars, if any is dropped the binding is also dropped.
    ///
    /// Returns a [`VarBindingHandle`]. See [`Vars::bind`] for more details about variable binding.
    ///
    /// [`can_update`]: Var::can_update
    /// [`always_read_only`]: Var::always_read_only
    /// [`is_contextual`]: Var::is_contextual
    /// [`actual_var`]: Var::actual_var
    fn bind_filter_bidi<Vw, T2, V2, M, N>(&self, vars: &Vw, other_var: &V2, mut map: M, mut map_back: N) -> VarBindingHandle
    where
        Vw: WithVars,
        T2: VarValue,
        V2: Var<T2>,
        M: FnMut(&VarBinding, &T) -> Option<T2> + 'static,
        N: FnMut(&VarBinding, &T2) -> Option<T> + 'static,
    {
        if !self.can_update() {
            // self cannot generate new values (and cannot be set)

            debug_assert!(self.always_read_only());
            return VarBindingHandle::dummy();
        }
        if !other_var.can_update() {
            // other cannot generate new values (and cannot be set)

            debug_assert!(other_var.always_read_only());
            return VarBindingHandle::dummy();
        }

        if self.always_read_only() {
            // self cannot be set, so this is, maybe, only a `bind_map`
            return self.bind_filter(vars, other_var, map);
        }

        if other_var.always_read_only() {
            // other_var cannot be set, so this is, maybe, only a *bind_map_back*
            return other_var.bind_filter(vars, self, map_back);
        }

        if self.is_contextual() || other_var.is_contextual() {
            return vars.with_vars(|vars| {
                // bind the actual vars.

                let actual_self = self.actual_var(vars);
                let actual_other = other_var.actual_var(vars);

                debug_assert!(actual_self.is_rc());
                debug_assert!(actual_other.is_rc());

                // ensure that `actual_var` remaps are not just dropped immediately.
                let wk_self_var = BindActualWeak::new(self, actual_self);
                let wk_other_var = BindActualWeak::new(other_var, actual_other);

                vars.with_vars(|vars| {
                    vars.bind(move |vars, binding| match (wk_self_var.upgrade(), wk_other_var.upgrade()) {
                        (Some(self_var), Some(other_var)) => {
                            if let Some(new_value) = self_var.get_new(vars) {
                                if let Some(new_value) = map(binding, new_value) {
                                    let _ = other_var.set(vars, new_value);
                                }
                            }
                            if let Some(new_value) = other_var.get_new(vars) {
                                if let Some(new_value) = map_back(binding, new_value) {
                                    let _ = self_var.set(vars, new_value);
                                }
                            }
                        }
                        _ => binding.unbind(),
                    })
                })
            });
        }

        debug_assert!(self.is_rc());
        debug_assert!(other_var.is_rc());

        let wk_self_var = self.downgrade().unwrap();
        let wk_other_var = other_var.downgrade().unwrap();

        vars.with_vars(|vars| {
            vars.bind(move |vars, binding| match (wk_self_var.upgrade(), wk_other_var.upgrade()) {
                (Some(self_var), Some(other_var)) => {
                    if let Some(new_value) = self_var.get_new(vars) {
                        if let Some(new_value) = map(binding, new_value) {
                            let _ = other_var.set(vars, new_value);
                        }
                    }
                    if let Some(new_value) = other_var.get_new(vars) {
                        if let Some(new_value) = map_back(binding, new_value) {
                            let _ = self_var.set(vars, new_value);
                        }
                    }
                }
                _ => binding.unbind(),
            })
        })
    }

    /// Create a [`bind_filter_bidi`] that uses [`TryInto`] to convert between `self` and `other_var`.
    ///
    /// [`bind_filter_bidi`]: Var::bind_filter_bidi
    fn bind_try_into_bidi<Vw, T2, V2>(&self, vars: &Vw, other_var: &V2) -> VarBindingHandle
    where
        Vw: WithVars,
        T2: VarValue + TryFrom<T>,
        V2: Var<T2>,
        T: TryFrom<T2>,
    {
        self.bind_filter_bidi(vars, other_var, |_, t| t.clone().try_into().ok(), |_, o| o.clone().try_into().ok())
    }

    /// Create a [`bind_filter_bidi`] that uses [`FromStr`] to convert from `self` to `other_var` and [`ToText`]
    /// to convert from `other_var` to `self`.
    ///
    /// [`bind_filter_bidi`]: Var::bind_filter_bidi
    fn bind_parse_bidi<Vw, T2, V2>(&self, vars: &Vw, other_var: &V2) -> VarBindingHandle
    where
        Vw: WithVars,
        T2: VarValue + FromStr + ToText,
        V2: Var<T2>,
        T: AsRef<str> + From<Text>,
    {
        self.bind_filter_bidi(vars, other_var, |_, t| t.as_ref().parse().ok(), |_, o| Some(o.to_text().into()))
    }

    /// Add a preview `handler` that is called every time this variable value is set, modified or touched,
    /// the handler is called before all other UI updates.
    ///
    /// See [`Vars::on_pre_var`] for more details.
    fn on_pre_new<Vw, H>(&self, vars: &Vw, handler: H) -> OnVarHandle
    where
        Vw: WithVars,
        H: AppHandler<T>,
    {
        if self.can_update() {
            vars.with_vars(|vars| vars.on_pre_var(self, handler))
        } else {
            OnVarHandle::dummy()
        }
    }

    /// Add a `handler` that is called every time this variable value is set, modified or touched,
    /// the handler is called after all other UI updates.
    ///
    /// See [`Vars::on_var`] for more details.
    fn on_new<Vw, H>(&self, vars: &Vw, handler: H) -> OnVarHandle
    where
        Vw: WithVars,
        H: AppHandler<T>,
    {
        if self.can_update() {
            vars.with_vars(|vars| vars.on_var(self, handler))
        } else {
            OnVarHandle::dummy()
        }
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
    /// # fn trace_var<T: VarValue>(var: &impl Var<T>, vars: &Vars) {
    /// var.trace_value(vars, |value| {
    ///     tracing::info_span!("my_var", ?value, track = "<vars>").entered()
    /// }).perm();
    /// # }
    /// ```
    ///
    /// Note that you don't need to use any external tracing crate, this method also works with the standard printing:
    ///
    /// ```
    /// # use zero_ui_core::var::*;
    /// # fn trace_var(var: &impl Var<u32>, vars: &Vars) {
    /// var.trace_value(vars, |v| println!("value: {v:?}")).perm();
    /// # }
    /// ```
    ///
    /// [`tracing`]: https://docs.rs/tracing/
    /// [`is_contextual`]: Var::is_contextual
    /// [`actual_var`]: Var::actual_var
    fn trace_value<Vw, S, E>(&self, vars: &Vw, mut enter_value: E) -> OnVarHandle
    where
        Vw: WithVars,
        E: FnMut(&T) -> S + 'static,
        S: 'static,
    {
        vars.with_vars(|vars| {
            let mut span = Some(enter_value(self.get(vars)));
            self.on_pre_new(
                vars,
                app_hn!(|_, value, _| {
                    let _ = span.take();
                    span = Some(enter_value(value));
                }),
            )
        })
    }

    /// Returns a [`UpdateMask`] that represents all the variables that can cause this variable to update.
    fn update_mask<Vr: WithVarsRead>(&self, vars: &Vr) -> UpdateMask;
}

#[doc(hidden)]
pub trait ResultOk<T> {
    fn r_ok(&self) -> Option<&T>;
}
impl<T, E> ResultOk<T> for Result<T, E> {
    fn r_ok(&self) -> Option<&T> {
        self.as_ref().ok()
    }
}

#[doc(hidden)]
pub trait ResultOkText {
    fn r_ok_text(&self) -> Option<Text>;
}
impl<T: ToText, E> ResultOkText for Result<T, E> {
    fn r_ok_text(&self) -> Option<Text> {
        self.as_ref().ok().map(ToText::to_text)
    }
}

#[doc(hidden)]
pub trait ResultErr<E> {
    fn r_err(&self) -> Option<&E>;
}
impl<T, E> ResultErr<E> for Result<T, E> {
    fn r_err(&self) -> Option<&E> {
        self.as_ref().err()
    }
}

#[doc(hidden)]
pub trait ResultErrText {
    fn r_err_text(&self) -> Option<Text>;
}
impl<T, E: ToText> ResultErrText for Result<T, E> {
    fn r_err_text(&self) -> Option<Text> {
        self.as_ref().err().map(ToText::to_text)
    }
}

#[doc(hidden)]
pub trait OptionSome<T> {
    fn opt_some(&self) -> Option<&T>;
}
impl<T> OptionSome<T> for Option<T> {
    fn opt_some(&self) -> Option<&T> {
        self.as_ref()
    }
}

#[doc(hidden)]
pub trait OptionSomeText {
    fn opt_some_text(&self) -> Option<Text>;
}
impl<T: ToText> OptionSomeText for Option<T> {
    fn opt_some_text(&self) -> Option<Text> {
        self.as_ref().map(ToText::to_text)
    }
}

/// Argument for [`Var::modify`]. This is a wrapper around a mutable reference to the variable value, if
/// [`DerefMut`] is used to get the variable value the variable value is flagged as *new*.
pub struct VarModify<'a, T: VarValue> {
    value: &'a mut T,
    touched: &'a mut bool,
}
impl<'a, T: VarValue> VarModify<'a, T> {
    /// New wrapper.
    pub fn new(value: &'a mut T, touched: &'a mut bool) -> Self {
        *touched = false;
        VarModify { value, touched }
    }

    /// If `deref_mut` was used or [`touch`](Self::touch) was called.
    pub fn touched(&self) -> bool {
        *self.touched
    }

    /// Flags the value as modified.
    pub fn touch(&mut self) {
        *self.touched = true;
    }

    /// Runs `modify` with a mutable reference `B` derived from `T` using `map`.
    /// The touched flag is only set if `modify` touches the the value.
    ///
    /// Note that modifying the value inside `map` is a logic error, it will not flag as touched
    /// so the variable will have a new value that is not propagated, only use `map` to borrow the
    /// map target.
    pub fn map_ref<B, M, Mo>(&mut self, map: M, modify: Mo)
    where
        B: VarValue,
        M: Fn(&mut T) -> &mut B,
        Mo: FnOnce(VarModify<B>),
    {
        let mut touched = false;

        modify(VarModify {
            value: map(self.value),
            touched: &mut touched,
        });

        *self.touched |= touched;
    }

    /// Reborrows `self` so that exclusive access can be given away without moving.
    pub fn reborrow(&'a mut self) -> Self {
        VarModify {
            value: self.value,
            touched: self.touched,
        }
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
        *self.touched = true;
        self.value
    }
}

///<span data-del-macro-root></span> Implements `U: From<T>`, `T: IntoVar<U>` and `T: IntoValue<U>` without boilerplate.
///
/// Unfortunately we cannot provide a blanket impl of `IntoVar` and `IntoValue` for all `From` in Rust stable, because
/// that would block all manual implementations of the trait, so you need to implement then manually to
/// enable the easy-to-use properties that are expected.
///
/// You can use this macro to implement both `U: From<T>`, `T: IntoVar<U>` and `T: IntoValue<U>` at the same time.
/// The macro syntax is one or more functions with signature `fn from(t: T) -> U`. The [`LocalVar<U>`]
/// type is selected for variables.
///
/// Optionally you can declare generics using the pattern `fn from<const N: usize>(t: &'static [T; N]) -> U`
/// with multiple generic types and constrains, but not `where` constrains. You can also destruct the input
/// if it is a tuple using the pattern `fn from((a, b): (A, B)) -> U`, but no other pattern matching in
/// the input is supported.
///
/// # Examples
///
/// The example declares an `enum` that represents the values possible in a property `foo` and
/// then implements conversions from literals the user may want to type in an widget:
///
/// ```
/// # use zero_ui_core::var::impl_from_and_into_var;
/// #[derive(Debug, Clone)]
/// pub enum FooValue {
///     On,
///     Off,
///     NotSet
/// }
/// impl_from_and_into_var! {
///     fn from(b: bool) -> FooValue {
///         if b {
///             FooValue::On
///         } else {
///             FooValue::Off
///         }
///     }
///
///     fn from(s: &str) -> FooValue {
///         match s {
///             "on" => FooValue::On,
///             "off" => FooValue::Off,
///             _ => FooValue::NotSet
///         }
///     }
/// }
/// # fn assert(_: impl zero_ui_core::var::IntoVar<FooValue> + Into<FooValue>) { }
/// # assert(true);
/// # assert("on");
/// ```
///
/// The value then can be used in a property:
///
/// ```
/// # use zero_ui_core::{*, var::*};
/// # #[derive(Debug, Clone)]
/// # pub struct FooValue;
/// # impl_from_and_into_var! { fn from(b: bool) -> FooValue { FooValue } }
/// # #[widget($crate::bar)] pub mod bar { }
/// #[property(context)]
/// pub fn foo(child: impl UiNode, value: impl IntoVar<FooValue>) -> impl UiNode {
///     // ..
/// #   child
/// }
///
/// # fn main() {
/// # let _ =
/// bar! {
///     foo = true;
/// }
/// # ;
/// # }
/// ```
#[macro_export]
macro_rules! impl_from_and_into_var {
    ($($tt:tt)+) => {
        $crate::__impl_from_and_into_var! { $($tt)* }
    };
}
#[doc(inline)]
pub use crate::impl_from_and_into_var;

#[doc(hidden)]
#[macro_export]
macro_rules! __impl_from_and_into_var {
    // START:
    (
        $(#[$docs:meta])*
        fn from ( $($input:tt)+ )
        $($rest:tt)+
    ) => {
        $crate::__impl_from_and_into_var! {
            =input=>
            [
                input { $($input)+ }
                generics { }
                docs { $(#[$docs])* }
            ]
            ( $($input)+ ) $($rest)+
        }
    };
    // GENERICS START:
    (
        $(#[$docs:meta])*
        fn from <
        $($rest:tt)+
    ) => {
        $crate::__impl_from_and_into_var! {
            =generics=>
            [
                generics { < }
                docs { $(#[$docs])* }
            ]
            $($rest)+
        }
    };
    // GENERICS END `>`:
    (
        =generics=>
        [
            generics { $($generics:tt)+ }
            $($config:tt)*
        ]

        >( $($input:tt)+ ) $($rest:tt)+
    ) => {
        $crate::__impl_from_and_into_var! {
            =input=>
            [
                input { $($input)+ }
                generics { $($generics)+ > }
                $($config)*
            ]
            ( $($input)+ ) $($rest)+
        }
    };
    // GENERICS END `>>`:
    (
        =generics=>
        [
            generics { $($generics:tt)+ }
            $($config:tt)*
        ]

        >>( $($input:tt)+ ) $($rest:tt)+
    ) => {
        $crate::__impl_from_and_into_var! {
            =input=>
            [
                input { $($input)+ }
                generics { $($generics)+ >> }
                $($config)*
            ]
            ( $($input)+ ) $($rest)+
        }
    };
    // collect generics:
    (
        =generics=>
        [
            generics { $($generics:tt)+ }
            $($config:tt)*
        ]

        $tt:tt $($rest:tt)+
    ) => {
        //zero_ui_proc_macros::trace! {
        $crate::__impl_from_and_into_var! {
            =generics=>
            [
                generics { $($generics)+ $tt }
                $($config)*
            ]
            $($rest)*
        }
        //}
    };
    // INPUT SIMPLE:
    (
        =input=>
        [$($config:tt)*]
        ($ident:ident : $Input:ty) $($rest:tt)+
    ) => {
        $crate::__impl_from_and_into_var! {
            =output=>
            [
                input_type { $Input }
                $($config)*
            ]
            $($rest)+
        }
    };
    // INPUT TUPLE:
    (
        =input=>
        [$($config:tt)*]
        (( $($destruct:tt)+ ) : $Input:ty) $($rest:tt)+
    ) => {
        $crate::__impl_from_and_into_var! {
            =output=>
            [
                input_type { $Input }
                $($config)*
            ]
            $($rest)+
        }
    };
    // OUTPUT:
    (
        =output=>
        [
            input_type { $Input:ty }
            input { $($input:tt)+ }
            generics { $($generics:tt)* }
            docs { $($docs:tt)* }
        ]
        -> $Output:ty
        $convert:block

        $($rest:tt)*
    ) => {
        impl $($generics)* From<$Input> for $Output {
            $($docs)*

            fn from($($input)+) -> Self
            $convert
        }

        impl $($generics)* $crate::var::IntoVar<$Output> for $Input {
            type Var = $crate::var::LocalVar<$Output>;

            $($docs)*

            fn into_var(self) -> Self::Var {
                $crate::var::LocalVar(self.into())
            }
        }

        impl $($generics)* $crate::var::IntoValue<$Output> for $Input { }

        // NEXT CONVERSION:
        $crate::__impl_from_and_into_var! {
            $($rest)*
        }
    };
    () => {
        // END
    };
}

/// Identifies a variable value version.
///
/// Comparing
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct VarVersion {
    context: Option<WidgetId>,
    depth: u32,
    version: u32,
}
impl VarVersion {
    /// Version for a variable that has a value not affected by context.
    pub(crate) fn normal(version: u32) -> Self {
        VarVersion {
            context: None,
            depth: 0,
            version,
        }
    }

    /// Add to the version count.
    pub(crate) fn wrapping_add(mut self, add: u32) -> Self {
        self.depth = self.depth.wrapping_add(add);
        self
    }

    /// Set the context of `self` from a transition of `prev` to new.
    pub(crate) fn set_widget_context(&mut self, prev: &Self, new: WidgetId) {
        let new = Some(new);
        if prev.context == new {
            self.depth = self.depth.wrapping_add(1);
        } else {
            self.depth = 0;
        }
        self.context = new;
    }

    /// Set the context of `self` to a direct `with_context_var` usage at the `AppExtension` level.
    pub(crate) fn set_app_context(&mut self, count: u32) {
        self.context = None;
        self.depth = count;
    }
}

#[derive(Clone)]
pub(crate) struct VarVersionCell(Cell<VarVersion>);
impl VarVersionCell {
    pub fn new(version: u32) -> Self {
        VarVersionCell(Cell::new(VarVersion::normal(version)))
    }

    pub fn get(&self) -> VarVersion {
        self.0.get()
    }

    pub fn set(&self, version: VarVersion) {
        self.0.set(version)
    }
}

#[cfg(test)]
mod tests {
    use crate::context::TestWidgetContext;

    use super::*;

    #[test]
    fn filter_to_text_bidi() {
        fn make(n: i32) -> (impl Var<i32>, impl Var<Text>) {
            let input = var(n);
            let output = input.filter_to_text_bidi();
            (input, output)
        }

        let mut ctx = TestWidgetContext::new();

        let (i, o) = make(42);

        assert_eq!("42", o.get(&ctx));

        o.set(&ctx, "30").unwrap();

        ctx.apply_updates();

        assert_eq!(30, i.copy(&ctx));

        i.set(&ctx, 10).unwrap();

        ctx.apply_updates();

        assert_eq!("10", o.get(&ctx));
    }

    #[test]
    fn filter_parse_bidi() {
        fn make(s: &str) -> (impl Var<Text>, impl Var<i32>) {
            let input = var(s.to_text());
            let output = input.filter_parse_bidi(|s| s.len() as i32);
            (input, output)
        }

        let mut ctx = TestWidgetContext::new();

        let (i, o) = make("42");

        assert_eq!(42, o.copy(&ctx));

        o.set(&ctx, 30).unwrap();

        ctx.apply_updates();

        assert_eq!("30", i.get(&ctx));

        i.set(&ctx, "10").unwrap();

        ctx.apply_updates();

        assert_eq!(10, o.copy(&ctx));
    }
}
