//! Variables.

use std::{
    convert::{TryFrom, TryInto},
    fmt,
    ops::{Deref, DerefMut},
    str::FromStr,
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

mod cow;
pub use cow::*;

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
#[cfg_attr(doc_nightly, doc(notable_trait))]
pub trait ContextVar: Clone + Copy + 'static {
    /// The variable type.
    type Type: VarValue;

    /// New default value.
    ///
    /// Returns a value that is equal to the variable value when it is not set in any context.
    fn default_value() -> Self::Type;

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
///
/// Every [`Var`] implements this to convert to it-self, every [`VarValue`] implements this to
/// convert to an [`OwnedVar`].
///
/// This trait is used by used by most properties, it allows then to accept literal values, variables and context variables
/// all with a single signature. Together with [`Var`] this gives properties great flexibility of usage, at zero-cost. Widget
/// `when` blocks also use [`IntoVar`] to support *changing* the property value depending on the widget state.
///
/// Value types can also manually implement this to support a shorthand literal syntax for when they are used in properties,
/// this converts the *shorthand value* like a tuple into the actual value type and wraps it into a variable, usually [`OwnedVar`]
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
///     type Var = OwnedVar<Size>;
///
///     fn into_var(self) -> Self::Var {
///         OwnedVar(Size { width: self.0 as f32, height: self.1 as f32 })
///     }
/// }
/// impl IntoVar<Size> for (f32, f32) {
///     type Var = OwnedVar<Size>;
///
///     fn into_var(self) -> Self::Var {
///         OwnedVar(Size { width: self.0, height: self.1 })
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
/// #[property(outer)]
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
///                 println!("update: {}", new);
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
    /// This is the [`OwnedVar`] for most types or `Self` for variable types.
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
/// # use zero_ui_core::*;
/// #
/// #[property(context, allowed_in_when = false)]
/// pub fn foo(child: impl UiNode, a: impl IntoValue<bool>, b: impl Into<bool>) -> impl UiNode {
///     struct FooNode<C> {
///         child: C,
///         a: bool,
///         b: bool,
///     }
///
/// # let _ =      
///     FooNode {
///         child,
///         a: a.into(),
///         b: b.into()
///     }
/// # ; child
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

/// Represents an observable value.
///
/// This trait is [sealed] and cannot be implemented for types outside of `zero_ui_core`.
///
/// [sealed]: https://rust-lang.github.io/api-guidelines/future-proofing.html#sealed-traits-protect-against-downstream-implementations-c-sealed
#[cfg_attr(doc_nightly, doc(notable_trait))]
pub trait Var<T: VarValue>: Clone + IntoVar<T> + crate::private::Sealed + 'static {
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
        vars.with_vars_read(|v| *self.get(v))
    }

    /// Clone the value.
    #[inline]
    fn get_clone<Vr: WithVarsRead>(&self, vars: &Vr) -> T {
        vars.with_vars_read(|v| self.get(v).clone())
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
    #[inline]
    fn wait_copy<'a, Vw: WithVars>(&'a self, vars: &'a Vw) -> VarCopyNewFut<'a, Vw, T, Self>
    where
        T: Copy,
    {
        if !self.can_update() {
            tracing::warn!("`Var::wait_copy` called in a variable that never updates");
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
    #[inline]
    fn wait_clone<'a, Vw: WithVars>(&'a self, vars: &'a Vw) -> VarCloneNewFut<'a, Vw, T, Self> {
        if !self.can_update() {
            tracing::warn!("`Var::wait_clone` called in a variable that never updates");
        }
        VarCloneNewFut::new(vars, self)
    }

    /// If the variable value changed in this update.
    ///
    /// When the variable value changes this stays `true` for the next app update cycle.
    /// An app update is requested only if the variable is shared (strong count > 1).
    fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool;

    /// Returns a future that awaits for [`is_new`](Var::is_new) after the current update.
    ///
    /// You can `.await` this in UI thread bound async code, like in async event handlers. The future
    /// will unblock once for every time [`is_new`](Var::is_new) returns `true` in a different update.
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
    #[inline]
    fn wait_new<'a, Vw: WithVars>(&'a self, vars: &'a Vw) -> VarIsNewFut<'a, Vw, T, Self> {
        if !self.can_update() {
            tracing::warn!("`Var::wait_new` called in a variable that never updates");
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
    /// The variable is marked as *new* only if the closure input is dereferenced as `mut`, and if
    /// it is marked  as new then the same behavior of [`set`] applies.
    ///
    /// [`set`]: Var::set
    fn modify<Vw, M>(&self, vars: &Vw, modify: M) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        M: FnOnce(&mut VarModify<T>) + 'static;

    /// Causes the variable to notify update without changing the value.
    ///
    /// The variable will get a new [`version`] and report that it [`is_new`] but the value
    /// will not actually change. Note that an app update is only automatically requested if
    /// the variable is shared ([`strong_count`] > 1).
    ///
    /// [`version`]: Var::version
    /// [`is_new`]: Var::is_new
    /// [`strong_count`]: Var::strong_count
    #[inline]
    fn touch<Vw: WithVars>(&self, vars: &Vw) -> Result<(), VarIsReadOnly> {
        self.modify(vars, |v| v.touch())
    }

    /// Schedule a new value for the variable.
    ///
    /// After the current app update finishes the `new_value` will be set, the variable will have
    /// a new [`version`] and [`is_new`] will be `true` for the next app update. If the variable
    /// is shared ([`strong_count`] > 1) then an app update is also automatically generated.
    ///
    /// [`version`]: Var::version
    /// [`is_new`]: Var::is_new
    /// [`strong_count`]: Var::strong_count
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

    /// Gets the number of references to this variable.
    ///
    /// Returns `0` if the variable is not shareable.
    fn strong_count(&self) -> usize;

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
    /// Also see [`Var::bind_map`] to create a *map binding* between two existing variables.
    #[inline]
    fn map<O, M>(&self, map: M) -> RcMapVar<T, O, M, Self>
    where
        O: VarValue,
        M: FnMut(&T) -> O + 'static,
    {
        RcMapVar::new(self.clone(), map)
    }

    /// Create a [`map`](Var::map) that uses [`Into`] to convert from `T` to `O`.
    #[inline]
    fn map_into<O>(&self) -> RcMapVar<T, O, fn(&T) -> O, Self>
    where
        O: VarValue + From<T>,
    {
        self.map(|t| t.clone().into())
    }

    /// Create a [`map`](Var::map) that uses [`ToText`](crate::text::ToText) to convert `T` to [`Text`](crate::text::ToText).
    #[inline]
    fn map_to_text(&self) -> RcMapVar<T, crate::text::Text, fn(&T) -> crate::text::Text, Self>
    where
        T: crate::text::ToText,
    {
        self.map(|t| t.to_text())
    }

    /// Create a [`map`](Var::map) that maps to a debug [`Text`](crate::text::ToText) using the `{:?}` format.
    #[inline]
    fn map_debug(&self) -> RcMapVar<T, crate::text::Text, fn(&T) -> crate::text::Text, Self> {
        self.map(|t| crate::formatx!("{:?}", t))
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
    /// Also see [`bind_map_bidi`](Var::bind_map_bidi) to create a *map binding* between two existing variables.
    #[inline]
    fn map_bidi<O, M, N>(&self, map: M, map_back: N) -> RcMapBidiVar<T, O, M, N, Self>
    where
        O: VarValue,
        M: FnMut(&T) -> O + 'static,
        N: FnMut(O) -> T + 'static,
    {
        RcMapBidiVar::new(self.clone(), map, map_back)
    }

    /// Create a [`map_bidi`](Var::map_bidi) that uses [`Into`] to convert between `T` and `O`.
    #[inline]
    #[allow(clippy::type_complexity)]
    fn map_into_bidi<O>(&self) -> RcMapBidiVar<T, O, fn(&T) -> O, fn(O) -> T, Self>
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
    #[inline]
    fn map_ref_bidi<O, M, N>(&self, map: M, map_mut: N) -> MapBidiRefVar<T, O, M, N, Self>
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
    /// Also see [`bind_filter`](Var::bind_filter) to create a *map binding* between two existing variables.
    #[inline]
    fn filter_map<O, I, M>(&self, fallback_init: I, map: M) -> RcFilterMapVar<T, O, I, M, Self>
    where
        O: VarValue,
        I: FnOnce(&T) -> O + 'static,
        M: FnMut(&T) -> Option<O> + 'static,
    {
        RcFilterMapVar::new(self.clone(), fallback_init, map)
    }

    /// Create a [`filter_map`] that uses [`TryInto`] to convert from `T` to `O`.
    ///
    /// [`filter_map`]: Var::filter_map
    #[inline]
    #[allow(clippy::type_complexity)]
    fn filter_try_into<O, I>(&self, fallback_init: I) -> RcFilterMapVar<T, O, I, fn(&T) -> Option<O>, Self>
    where
        O: VarValue + TryFrom<T>,
        I: FnOnce(&T) -> O + 'static,
    {
        RcFilterMapVar::new(self.clone(), fallback_init, |v| v.clone().try_into().ok())
    }

    /// Create a [`filter_map`] that uses [`FromStr`] to convert from `T` to `O`.
    ///
    /// [`filter_map`]: Var::filter_map
    #[inline]
    #[allow(clippy::type_complexity)]
    fn filter_parse<O, I>(&self, fallback_init: I) -> RcFilterMapVar<T, O, I, fn(&T) -> Option<O>, Self>
    where
        O: VarValue + FromStr,
        T: AsRef<str>,
        I: FnOnce(&T) -> O + 'static,
    {
        RcFilterMapVar::new(self.clone(), fallback_init, |v| v.as_ref().parse().ok())
    }

    /// Create a [`filter_map`](Var::filter_map) that passes when `T` is [`Ok`].
    #[inline]
    #[allow(clippy::type_complexity)]
    fn filter_ok<O, I>(&self, fallback_init: I) -> RcFilterMapVar<T, O, I, fn(&T) -> Option<O>, Self>
    where
        O: VarValue,
        I: FnOnce(&T) -> O + 'static,
        T: ResultOk<O>,
    {
        self.filter_map(fallback_init, |t| t.r_ok().cloned())
    }

    /// Create a [`filter_map`](Var::filter_map) that passes when `T` is [`Ok`] and maps the result to [`Text`].
    #[inline]
    #[allow(clippy::type_complexity)]
    fn filter_ok_text<I>(&self, fallback_init: I) -> RcFilterMapVar<T, Text, I, fn(&T) -> Option<Text>, Self>
    where
        I: FnOnce(&T) -> Text + 'static,
        T: ResultOkText,
    {
        self.filter_map(fallback_init, |t| t.r_ok_text())
    }

    /// Create a [`filter_map`](Var::filter_map) that passes when `T` is [`Err`].
    #[inline]
    #[allow(clippy::type_complexity)]
    fn filter_err<O, I>(&self, fallback_init: I) -> RcFilterMapVar<T, O, I, fn(&T) -> Option<O>, Self>
    where
        O: VarValue,
        I: FnOnce(&T) -> O + 'static,
        T: ResultErr<O>,
    {
        self.filter_map(fallback_init, |t| t.r_err().cloned())
    }

    /// Create a [`filter_map`](Var::filter_map) that passes when `T` is [`Err`] and maps the error to [`Text`].
    #[inline]
    #[allow(clippy::type_complexity)]
    fn filter_err_text<I>(&self, fallback_init: I) -> RcFilterMapVar<T, Text, I, fn(&T) -> Option<Text>, Self>
    where
        I: FnOnce(&T) -> Text + 'static,
        T: ResultErrText,
    {
        self.filter_map(fallback_init, |t| t.r_err_text())
    }

    /// Create a [`filter_map`](Var::filter_map) that passes when `T` is [`Some`].
    #[inline]
    #[allow(clippy::type_complexity)]
    fn filter_some<O, I>(&self, fallback_init: I) -> RcFilterMapVar<T, O, I, fn(&T) -> Option<O>, Self>
    where
        O: VarValue,
        I: FnOnce(&T) -> O + 'static,
        T: OptionSome<O>,
    {
        self.filter_map(fallback_init, |t| t.opt_some().cloned())
    }

    /// Create a [`filter_map`](Var::filter_map) that passes when `T` is [`Some`] and convert the value to [`Text`].
    #[inline]
    #[allow(clippy::type_complexity)]
    fn filter_some_text<I>(&self, fallback_init: I) -> RcFilterMapVar<T, Text, I, fn(&T) -> Option<Text>, Self>
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

    /// Create a [`filter_map_bidi`] that uses [`TryInto`] to convert between `T` and `O`.
    ///
    /// [`filter_map_bidi`]: Var::filter_map_bidi
    #[inline]
    #[allow(clippy::type_complexity)]
    fn filter_try_into_bidi<O, I>(&self, fallback_init: I) -> RcFilterMapBidiVar<T, O, I, fn(&T) -> Option<O>, fn(O) -> Option<T>, Self>
    where
        O: VarValue + TryFrom<T>,
        I: FnOnce(&T) -> O + 'static,
        T: TryFrom<O>,
    {
        RcFilterMapBidiVar::new(self.clone(), fallback_init, |t| t.clone().try_into().ok(), |o| o.try_into().ok())
    }

    /// Create a [`filter_map_bidi`] that uses [`FromStr`] to convert from `T` to `O` and [`ToText`] to convert from `O` to `T`.
    ///
    /// The `fallback_init` is called to generate a value if the first [`str::parse`] call fails.
    ///
    /// [`filter_map_bidi`]: Var::filter_map_bidi
    #[inline]
    #[allow(clippy::type_complexity)]
    fn filter_parse_bidi<O, I>(&self, fallback_init: I) -> RcFilterMapBidiVar<T, O, I, fn(&T) -> Option<O>, fn(O) -> Option<T>, Self>
    where
        O: VarValue + FromStr + ToText,
        I: FnOnce(&T) -> O + 'static,
        T: AsRef<str> + From<Text>,
    {
        RcFilterMapBidiVar::new(
            self.clone(),
            fallback_init,
            |t| t.as_ref().parse().ok(),
            |o| Some(o.to_text().into()),
        )
    }

    /// Create a [`filter_map_bidi`] that uses [`FromStr`] to convert from `O` to `T` and [`ToText`] to convert from `T` to `O`.
    ///
    /// [`filter_map_bidi`]: Var::filter_map_bidi
    #[inline]
    #[allow(clippy::type_complexity)]
    fn filter_to_text_bidi<O>(&self) -> RcFilterMapBidiVar<T, O, fn(&T) -> O, fn(&T) -> Option<O>, fn(O) -> Option<T>, Self>
    where
        O: VarValue + AsRef<str> + From<Text>,
        T: FromStr + ToText,
    {
        RcFilterMapBidiVar::new(
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
        vars.with_vars_read(|vars| vars.receiver(self))
    }

    /// Create a binding with `to_var`. When `self` updates the `to_var` is assigned a clone of the new value.
    ///
    /// Both `self` and `other_var` notify a new value in the same app update, this is different then a manually implemented *binding*
    /// when the assign to the second variable would cause a second update.
    #[inline]
    fn bind<Vw, V2>(&self, vars: &Vw, to_var: &V2) -> VarBindingHandle
    where
        Vw: WithVars,
        V2: Var<T>,
    {
        self.bind_map(vars, to_var, |_, v| v.clone())
    }

    /// Create a bidirectional binding with `other_var`. When one of the vars update the other is
    /// assigned a clone of the new value.
    ///
    /// Both `self` and `other_var` notify a new value in the same app update, this is different then a manually implemented *binding*
    /// when the assign to the second variable would cause a second update.
    #[inline]
    fn bind_bidi<Vw, V2>(&self, vars: &Vw, other_var: &V2) -> VarBindingHandle
    where
        Vw: WithVars,
        V2: Var<T>,
    {
        self.bind_map_bidi(vars, other_var, |_, v| v.clone(), |_, v| v.clone())
    }

    /// Create a [`map`](Var::map) like binding between two existing variables.
    ///
    /// The binding flows from `self` to `to_var`, every time `self` updates `map` is called to generate a value that is assigned `to_var`.
    ///
    /// Both `self` and `to_var` notify a new value in the same app update, this is different then a manually implemented *binding*
    /// where the assign to `to_var` would cause a second update.
    #[inline]
    fn bind_map<Vw, T2, V2, M>(&self, vars: &Vw, to_var: &V2, mut map: M) -> VarBindingHandle
    where
        Vw: WithVars,
        T2: VarValue,
        V2: Var<T2>,
        M: FnMut(&VarBinding, &T) -> T2 + 'static,
    {
        vars.with_vars(|vars| {
            let from_var = self.clone();
            let to_var = to_var.clone();
            vars.bind(move |vars, info| {
                if let Some(new_value) = from_var.get_new(vars) {
                    let new_value = map(info, new_value);
                    let _ = to_var.set(vars, new_value);
                }
            })
        })
    }

    /// Create a [`bind_map`](Var::bind_map) that uses [`Into`] to convert `T` to `T2`.
    #[inline]
    fn bind_into<Vw, T2, V2>(&self, vars: &Vw, to_var: &V2) -> VarBindingHandle
    where
        Vw: WithVars,
        T2: VarValue + From<T>,
        V2: Var<T2>,
    {
        self.bind_map(vars, to_var, |_, v| v.clone().into())
    }

    /// Create a [`bind_map`](Var::bind_map) that uses [`ToText`](crate::text::ToText) to convert `T` to [`Text`](crate::text::ToText).
    #[inline]
    fn bind_to_text<Vw, V>(&self, vars: &Vw, text_var: &V) -> VarBindingHandle
    where
        Vw: WithVars,
        V: Var<Text>,
        T: crate::text::ToText,
    {
        self.bind_map(vars, text_var, |_, v| v.to_text())
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
    fn bind_map_bidi<Vw, T2, V2, M, N>(&self, vars: &Vw, other_var: &V2, mut map: M, mut map_back: N) -> VarBindingHandle
    where
        Vw: WithVars,
        T2: VarValue,
        V2: Var<T2>,
        M: FnMut(&VarBinding, &T) -> T2 + 'static,
        N: FnMut(&VarBinding, &T2) -> T + 'static,
    {
        vars.with_vars(|vars| {
            let from_var = self.clone();
            let to_var = other_var.clone();
            vars.bind(move |vars, info| {
                if let Some(new_value) = from_var.get_new(vars) {
                    let new_value = map(info, new_value);
                    let _ = to_var.set(vars, new_value);
                }
                if let Some(new_value) = to_var.get_new(vars) {
                    let new_value = map_back(info, new_value);
                    let _ = from_var.set(vars, new_value);
                }
            })
        })
    }

    /// Create a [`bind_map_bidi`](Var::bind_map_bidi) that uses [`Into`] to convert between `self` and `other_var`.
    #[inline]
    fn bind_into_bidi<Vw, T2, V2>(&self, vars: &Vw, other_var: &V2) -> VarBindingHandle
    where
        Vw: WithVars,
        T2: VarValue + From<T>,
        V2: Var<T2>,
        T: From<T2>,
    {
        self.bind_map_bidi(vars, other_var, |_, t| t.clone().into(), |_, t2| t2.clone().into())
    }

    /// Create a [`filter_map`](Var::filter_map) like binding between two existing variables.
    ///
    /// The binding flows from `self` to `to_var`, every time `self` updates `map` is called to generate a value, if it does, that value
    /// is assigned `to_var`.
    ///
    /// Both `self` and `to_var` notify a new value in the same app update, this is different then a manually implemented *binding*
    /// where the assign to `to_var` would cause a second update.
    #[inline]
    fn bind_filter<Vw, T2, V2, M>(&self, vars: &Vw, to_var: &V2, mut map: M) -> VarBindingHandle
    where
        Vw: WithVars,
        T2: VarValue,
        V2: Var<T2>,
        M: FnMut(&VarBinding, &T) -> Option<T2> + 'static,
    {
        vars.with_vars(|vars| {
            let from_var = self.clone();
            let to_var = to_var.clone();
            vars.bind(move |vars, info| {
                if let Some(new_value) = from_var.get_new(vars) {
                    if let Some(new_value) = map(info, new_value) {
                        let _ = to_var.set(vars, new_value);
                    }
                }
            })
        })
    }

    /// Create a [`bind_filter`] that uses [`TryInto`] to convert from `self` to `to_var`.
    ///
    /// [`bind_filter`]: Var::bind_filter
    #[inline]
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
    #[inline]
    fn bind_parse<Vw, T2, V2>(&self, vars: &Vw, parsed_var: &V2) -> VarBindingHandle
    where
        Vw: WithVars,
        T2: VarValue + FromStr,
        V2: Var<T2>,
        T: AsRef<str>,
    {
        self.bind_filter(vars, parsed_var, |_, t| t.as_ref().parse().ok())
    }

    /// Create a [`bind_filter`](Var::bind_filter) that sets `to_var` when `T` is [`Ok`].
    #[inline]
    fn bind_ok<Vw, T2, V2>(&self, vars: &Vw, to_var: &V2) -> VarBindingHandle
    where
        Vw: WithVars,
        T2: VarValue,
        V2: Var<T2>,
        T: ResultOk<T2>,
    {
        self.bind_filter(vars, to_var, |_, t| t.r_ok().cloned())
    }

    /// Create a [`bind_filter`](Var::bind_filter) that sets `text_var` when `T` is [`Ok`].
    #[inline]
    fn bind_ok_text<Vw, V2>(&self, vars: &Vw, text_var: &V2) -> VarBindingHandle
    where
        Vw: WithVars,
        V2: Var<Text>,
        T: ResultOkText,
    {
        self.bind_filter(vars, text_var, |_, t| t.r_ok_text())
    }

    /// Create a [`bind_filter`](Var::bind_filter) that sets `to_var` when `T` is [`Err`].
    #[inline]
    fn bind_err<Vw, T2, V2>(&self, vars: &Vw, to_var: &V2) -> VarBindingHandle
    where
        Vw: WithVars,
        T2: VarValue,
        V2: Var<T2>,
        T: ResultErr<T2>,
    {
        self.bind_filter(vars, to_var, |_, t| t.r_err().cloned())
    }

    /// Create a [`bind_filter`](Var::bind_filter) that sets `text_var` when `T` is [`Err`].
    #[inline]
    fn bind_err_text<Vw, V2>(&self, vars: &Vw, text_var: &V2) -> VarBindingHandle
    where
        Vw: WithVars,
        V2: Var<Text>,
        T: ResultErrText,
    {
        self.bind_filter(vars, text_var, |_, t| t.r_err_text())
    }

    /// Create a [`bind_filter`](Var::bind_filter) that sets `to_var` when `T` is [`Some`].
    #[inline]
    fn bind_some<Vw, T2, V2>(&self, vars: &Vw, to_var: &V2) -> VarBindingHandle
    where
        Vw: WithVars,
        T2: VarValue,
        V2: Var<T2>,
        T: OptionSome<T2>,
    {
        self.bind_filter(vars, to_var, |_, t| t.opt_some().cloned())
    }

    /// Create a [`bind_filter`](Var::bind_filter) that sets `text_var` when `T` is [`Some`].
    #[inline]
    fn bind_some_text<Vw, V2>(&self, vars: &Vw, text_var: &V2) -> VarBindingHandle
    where
        Vw: WithVars,
        V2: Var<Text>,
        T: OptionSomeText,
    {
        self.bind_filter(vars, text_var, |_, t| t.opt_some_text())
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
    fn bind_filter_bidi<Vw, T2, V2, M, N>(&self, vars: &Vw, other_var: &V2, mut map: M, mut map_back: N) -> VarBindingHandle
    where
        Vw: WithVars,
        T2: VarValue,
        V2: Var<T2>,
        M: FnMut(&VarBinding, &T) -> Option<T2> + 'static,
        N: FnMut(&VarBinding, &T2) -> Option<T> + 'static,
    {
        vars.with_vars(|vars| {
            let from_var = self.clone();
            let to_var = other_var.clone();
            vars.bind(move |vars, info| {
                if let Some(new_value) = from_var.get_new(vars) {
                    if let Some(new_value) = map(info, new_value) {
                        let _ = to_var.set(vars, new_value);
                    }
                }
                if let Some(new_value) = to_var.get_new(vars) {
                    if let Some(new_value) = map_back(info, new_value) {
                        let _ = from_var.set(vars, new_value);
                    }
                }
            })
        })
    }

    /// Create a [`bind_filter_bidi`] that uses [`TryInto`] to convert between `self` and `other_var`.
    ///
    /// [`bind_filter_bidi`]: Var::bind_filter_bidi
    #[inline]
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
    #[inline]
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
    #[inline]
    fn on_pre_new<Vw, H>(&self, vars: &Vw, handler: H) -> OnVarHandle
    where
        Vw: WithVars,
        H: AppHandler<T>,
    {
        if self.can_update() {
            vars.with_vars(|vars| vars.on_pre_var(self.clone(), handler))
        } else {
            OnVarHandle::dummy()
        }
    }

    /// Add a `handler` that is called every time this variable value is set, modified or touched,
    /// the handler is called after all other UI updates.
    ///
    /// See [`Vars::on_var`] for more details.
    #[inline]
    fn on_new<Vw, H>(&self, vars: &Vw, handler: H) -> OnVarHandle
    where
        Vw: WithVars,
        H: AppHandler<T>,
    {
        if self.can_update() {
            vars.with_vars(|vars| vars.on_var(self.clone(), handler))
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
    /// Returns a [`OnVarHandle`] that can be used to stop tracing.
    ///
    /// If this variable can never update the span is immediately dropped and a dummy handle is returned.
    ///
    /// # Examples
    ///
    /// Using the [`tracing`] crate to trace value spans:
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui_core::var::*;
    /// # macro_rules! info_span { ($($tt:tt)*) => { }; }
    /// # mod tracing {  pub use crate::info_span; }
    /// fn trace_var<T: VarValue>(var: &impl Var<T>, vars: &Vars) {
    ///     let handle = var.trace_value(vars, |value| {
    ///         tracing::info_span!("my_var", ?value, track = "<vars>")
    ///     }).entered();
    ///     handle.permanent();
    /// }
    /// ```
    ///
    /// Making the handle permanent means that the tracing will happen for the duration of the variable or app.
    ///
    /// [`tracing`]: https://docs.rs/tracing/
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

///<span data-inline></span> New [`impl Var<T>`](Var) from an expression with interpolated *vars*.
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
use crate::{
    handler::AppHandler,
    text::{Text, ToText},
};

#[doc(hidden)]
pub use zero_ui_proc_macros::expr_var as __expr_var;

///<span data-inline></span> Implements `U: From<T>`, `T: IntoVar<U>` and `T: IntoValue<U>` without boilerplate.
///
/// Unfortunately we cannot provide a blanket impl of `IntoVar` and `IntoValue` for all `From` in Rust stable, because
/// that would block all manual implementations of the trait, so you need to implement then manually to
/// enable the easy-to-use properties that are expected.
///
/// You can use this macro to implement both `U: From<T>`, `T: IntoVar<U>` and `T: IntoValue<U>` at the same time.
/// The macro syntax is one or more functions with signature `fn from(t: T) -> U`. The [`OwnedVar<U>`]
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
            #[inline]
            fn from($($input)+) -> Self
            $convert
        }

        impl $($generics)* $crate::var::IntoVar<$Output> for $Input {
            type Var = $crate::var::OwnedVar<$Output>;

            $($docs)*
            #[inline]
            fn into_var(self) -> Self::Var {
                $crate::var::OwnedVar(self.into())
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
