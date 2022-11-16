//! Variables.

use std::{
    any::{Any, TypeId},
    borrow::Cow,
    cell::{Cell, RefCell},
    fmt,
    marker::PhantomData,
    ops,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use crate::{
    handler::{app_hn, app_hn_once, AppHandler, AppHandlerArgs},
    units::*,
};

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
mod map_ref;
mod merge;
mod rc;
mod read_only;
mod response;
mod state;
mod vars;
mod when;

#[macro_use]
mod util;

use parking_lot::Mutex;
#[doc(inline)]
pub use util::impl_from_and_into_var;

mod tests;

pub use animation::easing;
pub use boxed::{BoxedAnyVar, BoxedAnyWeakVar, BoxedVar, BoxedWeakVar};
pub use channel::{response_channel, ResponseSender, VarModifySender, VarReceiver, VarSender};
#[doc(inline)]
pub use context::{context_var, with_context_var, with_context_var_init, ContextInitHandle, ContextVar, ReadOnlyContextVar};
#[doc(inline)]
pub use expr::expr_var;
pub use local::LocalVar;
#[doc(inline)]
pub use merge::merge_var;
pub use rc::{var, var_default, var_from, RcVar};
pub use read_only::ReadOnlyRcVar;
pub use response::{response_done_var, response_var, ResponderVar, ResponseVar};
pub use state::*;
pub use vars::*;
#[doc(inline)]
pub use when::when_var;

use crate::{context::Updates, widget_instance::WidgetId};

/// Other variable types.
pub mod types {
    pub use super::boxed::{VarBoxed, WeakVarBoxed};
    pub use super::context::{with_new_context_init_id, ContextData, WeakContextInitHandle};
    pub use super::contextualized::{ContextualizedVar, WeakContextualizedVar};
    pub use super::cow::{RcCowVar, WeakCowVar};
    pub use super::expr::__expr_var;
    pub use super::flat_map::{RcFlatMapVar, WeakFlatMapVar};
    pub use super::future::{WaitIsNewFut, WaitIsNotAnimatingFut, WaitNewFut};
    pub use super::map_ref::{MapRef, MapRefBidi, WeakMapRef, WeakMapRefBidi};
    pub use super::merge::{ContextualizedRcMergeVar, RcMergeVar, RcMergeVarInput, WeakMergeVar, __merge_var};
    pub use super::rc::WeakRcVar;
    pub use super::read_only::{ReadOnlyVar, WeakReadOnlyVar};
    pub use super::response::Response;
    pub use super::when::{AnyWhenVarBuilder, ContextualizedRcWhenVar, RcWhenVar, WeakWhenVar, WhenVarBuilder, __when_var};

    use super::*;

    /// Helper type for debug printing [`Var<T>`].
    ///
    /// You can use [`Var::debug`] to get an instance.
    pub struct VarDebug<'a, T: VarValue, V: Var<T>> {
        pub(super) var: &'a V,
        pub(super) _t: PhantomData<fn() -> T>,
    }
    impl<'a, T: VarValue, V: Var<T>> fmt::Debug for VarDebug<'a, T, V> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.var.with(|t| fmt::Debug::fmt(t, f))
        }
    }

    /// Helper type for display printing [`Var<T>`].
    ///
    /// You can use [`Var::display`] to get an instance.
    pub struct VarDisplay<'a, T: VarValue + fmt::Display, V: Var<T>> {
        pub(super) var: &'a V,
        pub(super) _t: PhantomData<fn() -> T>,
    }
    impl<'a, T: VarValue + fmt::Display, V: Var<T>> fmt::Display for VarDisplay<'a, T, V> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.var.with(|t| fmt::Display::fmt(t, f))
        }
    }
}

/// A type that can be a [`Var<T>`] value.
///
/// # Trait Alias
///
/// This trait is used like a type alias for traits and is
/// already implemented for all types it applies to.
pub trait VarValue: fmt::Debug + Clone + Any + Send + Sync {}
impl<T: fmt::Debug + Clone + Any + Send + Sync> VarValue for T {}

/// Trait implemented for all [`VarValue`] types.
pub trait AnyVarValue: fmt::Debug + Any + Send + Sync {
    /// Access to `dyn Any` methods.
    fn as_any(&self) -> &dyn Any;

    /// Access to `Box<dyn Any>` methods.
    fn into_any(self: Box<Self>) -> Box<dyn Any>;

    /// Clone the value.
    fn clone_boxed(&self) -> Box<dyn AnyVarValue>;

    /// Clone the value into a new boxed [`LocalVar<Self>`].
    fn clone_boxed_var(&self) -> BoxedAnyVar;
}

impl<T: VarValue> AnyVarValue for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_boxed(&self) -> Box<dyn AnyVarValue> {
        Box::new(self.clone())
    }

    fn clone_boxed_var(&self) -> BoxedAnyVar {
        Box::new(LocalVar(self.clone()))
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

/// A property value that is not a variable but can be inspected.
///
/// # Examples
///
/// The example property receives the input `a` and `b`, they cannot change.
///
/// ```
/// # fn main() { }
/// # use zero_ui_core::{*, widget_instance::*, var::IntoValue};
/// #
/// #[property(CONTEXT)]
/// pub fn foo(child: impl UiNode, a: impl IntoValue<bool>, b: impl IntoValue<bool>) -> impl UiNode {
///     #[ui_node(struct FooNode {
///         child: impl UiNode,
///         a: bool,
///         b: bool,
///     })]
///     impl UiNode for FooNode { }
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
/// The trait is only auto-implemented for `T: Into<T> + VarValue`, unfortunately actual type conversions
/// must be manually implemented, note that the [`impl_from_and_into_var!`] macro auto-implements this conversion.
///
/// [inspected]: crate::inspector
/// [`Debug`]: std::fmt::Debug
/// [`impl_from_and_into_var`]: crate::var::impl_from_and_into_var
pub trait IntoValue<T: VarValue>: Into<T> {}
impl<T: VarValue> IntoValue<T> for T {}

bitflags! {
    /// Kinds of interactions allowed by a [`Var<T>`] in the current update.
    ///
    /// You can get the current capabilities of a var by using the [`AnyVar::capabilities`] method.
    pub struct VarCapabilities: u8 {
        /// Var value can change.
        ///
        /// If this is set the [`Var::is_new`] can be `true` in some updates, a variable can `NEW`
        /// even if it cannot `MODIFY`, in this case the variable is a read-only wrapper on a read-write variable.
        const NEW = 0b0000_0010;

        /// Var can be modified.
        ///
        /// If this is set [`Var::modify`] does succeeds, if this is set `NEW` is also set.
        const MODIFY = 0b0000_0011;

        /// Var capabilities can change.
        ///
        /// Var capabilities can only change in between app updates, just like the var value, but [`AnyVar::last_update`]
        /// may not change when capability changes.
        const CAPS_CHANGE = 0b1000_0000;
    }
}
impl VarCapabilities {
    /// Remove only the `MODIFY` flag without removing `NEW`.
    pub fn as_read_only(mut self) -> Self {
        self.bits &= 0b1111_1110;
        self
    }

    /// If cannot `MODIFY` and is not `CAPS_CHANGE`.
    pub fn is_always_read_only(self) -> bool {
        !self.contains(Self::MODIFY) && !self.contains(Self::CAPS_CHANGE)
    }

    /// If cannot `NEW` and is not `CAPS_CHANGE`.
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

struct VarHandleData {
    perm: AtomicBool,
    pos_modify_action: Box<dyn Fn(&Vars, &mut Updates, &dyn AnyVarValue) -> bool + Send + Sync>,
}

struct VarHook(Arc<VarHandleData>);
impl VarHook {
    /// Callback, returns `true` if the handle must be retained.
    fn call(&self, vars: &Vars, update: &mut Updates, value: &dyn AnyVarValue) -> bool {
        (Arc::strong_count(&self.0) > 1 || self.0.perm.load(Ordering::Relaxed)) && (self.0.pos_modify_action)(vars, update, value)
    }
}

/// Handle to a variable hook.
///
/// This can represent an widget subscriber, a var binding, var app handler or animation, dropping the handler stops
/// the behavior it represents.
#[derive(Clone)]
#[must_use = "var handle stops the behaviour it represents on drop"]
pub struct VarHandle(Option<Arc<VarHandleData>>);
impl VarHandle {
    fn new(pos_modify_action: Box<dyn Fn(&Vars, &mut Updates, &dyn AnyVarValue) -> bool + Send + Sync>) -> (VarHandle, VarHook) {
        let c = Arc::new(VarHandleData {
            perm: AtomicBool::new(false),
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
    /// [`dummy`]: VarHandle::dummy
    pub fn is_dummy(&self) -> bool {
        self.0.is_none()
    }

    /// Drop the handle without stopping the behavior it represents.
    ///
    /// Note that the behavior can still be stopped by dropping the involved variables.
    pub fn perm(self) {
        if let Some(s) = &self.0 {
            s.perm.store(true, Ordering::Relaxed);
        }
    }

    /// Create a [`VarHandles`] collection with `self` and `other`.
    pub fn with(self, other: Self) -> VarHandles {
        [self, other].into()
    }
}
impl PartialEq for VarHandle {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (None, None) => true,
            (None, Some(_)) | (Some(_), None) => false,
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
        }
    }
}
impl Eq for VarHandle {}
impl std::hash::Hash for VarHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let i = match &self.0 {
            Some(rc) => Arc::as_ptr(rc) as usize,
            None => 0,
        };
        state.write_usize(i);
    }
}
impl fmt::Debug for VarHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let i = match &self.0 {
            Some(rc) => Arc::as_ptr(rc) as usize,
            None => 0,
        };
        f.debug_tuple("VarHandle").field(&i).finish()
    }
}

/// Represents a collection of var handles.
#[derive(Clone, Default)]
pub struct VarHandles(pub Vec<VarHandle>);
impl VarHandles {
    /// Empty collection.
    pub fn dummy() -> Self {
        VarHandles(vec![])
    }

    /// Returns `true` if empty or all handles are dummy.
    pub fn is_dummy(&self) -> bool {
        self.0.is_empty() || self.0.iter().all(VarHandle::is_dummy)
    }

    /// Drop all handles without stopping their behavior.
    pub fn perm(self) {
        for handle in self.0 {
            handle.perm()
        }
    }

    /// Add the `other` handle to the collection, if it is not dummy.
    pub fn push(&mut self, other: VarHandle) -> &mut Self {
        if !other.is_dummy() {
            self.0.push(other);
        }
        self
    }

    /// Drop all handles.
    pub fn clear(&mut self) {
        self.0.clear()
    }
}
impl FromIterator<VarHandle> for VarHandles {
    fn from_iter<T: IntoIterator<Item = VarHandle>>(iter: T) -> Self {
        VarHandles(iter.into_iter().filter(|h| !h.is_dummy()).collect())
    }
}
impl<const N: usize> From<[VarHandle; N]> for VarHandles {
    fn from(handles: [VarHandle; N]) -> Self {
        handles.into_iter().collect()
    }
}
impl Extend<VarHandle> for VarHandles {
    fn extend<T: IntoIterator<Item = VarHandle>>(&mut self, iter: T) {
        for handle in iter {
            self.push(handle);
        }
    }
}
impl IntoIterator for VarHandles {
    type Item = VarHandle;

    type IntoIter = std::vec::IntoIter<VarHandle>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// Methods of [`Var<T>`] that don't depend on the value type.
///
/// This trait is [sealed] and cannot be implemented for types outside of `zero_ui_core`.
///
/// [sealed]: https://rust-lang.github.io/api-guidelines/future-proofing.html#sealed-traits-protect-against-downstream-implementations-c-sealed
pub trait AnyVar: Any + Send + Sync + crate::private::Sealed {
    /// Clone the variable into a type erased box, this is never [`BoxedVar<T>`].
    fn clone_any(&self) -> BoxedAnyVar;

    /// Access to `dyn Any` methods.
    fn as_any(&self) -> &dyn Any;

    /// Access to `Box<dyn Any>` methods, with the [`BoxedVar<T>`] type.
    ///
    /// This is a double-boxed to allow downcast to [`BoxedVar<T>`].
    fn double_boxed_any(self: Box<Self>) -> Box<dyn Any>;

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

    /// Last update ID a variable was modified, if the ID is equal to [`Vars::update_id`] the variable is *new*.
    fn last_update(&self) -> VarUpdateId;

    /// Flags that indicate what operations the variable is capable of.
    fn capabilities(&self) -> VarCapabilities;

    /// if the variable current value was set by an active animation.
    ///
    /// The variable [`is_new`] when this changes to `true`, but it can change to `false` at any time
    /// without the value updating.
    ///
    /// [`is_new`]: Var::is_new
    fn is_animating(&self) -> bool;

    /// Setups a callback for just after the variable value update is applied, the closure runs in the root app context, just like
    /// the `modify` closure.
    ///
    /// Variables store a weak[^1] reference to the callback if they have the `MODIFY` or `CAPS_CHANGE` capabilities, otherwise
    /// the callback is discarded and [`VarHandle::dummy`] returned.
    ///
    /// This is the most basic callback, used by the variables themselves, you can create a more elaborate handle using [`on_new`].
    ///
    /// [`on_new`]: Var::on_new
    /// [^1]: You can use the [`VarHandle::perm`] to make the stored reference *strong*.
    fn hook(&self, pos_modify_action: Box<dyn Fn(&Vars, &mut Updates, &dyn AnyVarValue) -> bool + Send + Sync>) -> VarHandle;

    /// Register the widget to receive update when this variable is new.
    ///
    /// Variables without the [`NEW`] capability return [`VarHandle::dummy`].
    ///
    /// [`NEW`]: VarCapabilities::NEW
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
    /// a [`types::WeakRcVar<T>`] is returned, for *Rc* vars an actual weak reference is made.
    ///
    /// [`actual_var`]: Var::actual_var
    fn downgrade_any(&self) -> BoxedAnyWeakVar;

    /// Var *pointer*, that can be used to identify if two variables point to the same *rc* or *context*.
    ///
    /// If two of these values are equal, both variables point to the same *rc* or *context* at the moment of comparison.
    /// Note that you can't store this or actually get unsafe access to the var internals, this is only for comparison.
    fn var_ptr(&self) -> VarPtr;
}

/// Represents an [`AnyVar`] *pointer* that can be used for comparison.
///
/// If two of these values are equal, both variables point to the same *rc* or *context* at the moment of comparison.
pub struct VarPtr<'a> {
    _lt: std::marker::PhantomData<&'a ()>,
    eq: VarPtrData,
}
impl<'a> VarPtr<'a> {
    fn new_arc<T: ?Sized>(rc: &'a Arc<T>) -> Self {
        Self {
            _lt: std::marker::PhantomData,
            eq: VarPtrData::Arc(Arc::as_ptr(rc) as _),
        }
    }

    fn new_thread_local<T>(tl: &'static std::thread::LocalKey<T>) -> Self {
        Self {
            _lt: std::marker::PhantomData,
            eq: VarPtrData::Static((tl as *const std::thread::LocalKey<T>) as _),
        }
    }

    fn new_never_eq(_: &'a impl Any) -> Self {
        Self {
            _lt: std::marker::PhantomData,
            eq: VarPtrData::NeverEq,
        }
    }
}
impl<'a> PartialEq for VarPtr<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.eq == other.eq
    }
}
impl<'a> fmt::Debug for VarPtr<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("VarPtr").field(&self.eq).finish()
    }
}

#[derive(Debug)]
enum VarPtrData {
    Static(*const ()),
    Arc(*const ()),
    NeverEq,
}
impl PartialEq for VarPtrData {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Static(l0), Self::Static(r0)) => l0 == r0,
            (Self::Arc(l0), Self::Arc(r0)) => l0 == r0,
            _ => false,
        }
    }
}

/// Represents a weak reference to a boxed [`AnyVar`].
pub trait AnyWeakVar: Any + Send + Sync + crate::private::Sealed {
    /// Clone the weak reference.
    fn clone_any(&self) -> BoxedAnyWeakVar;

    /// Access to `dyn Any` methods.
    fn as_any(&self) -> &dyn Any;

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
/// This trait is used by most properties, it allows then to accept literal values, variables and context variables
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
/// # use zero_ui_core::{*, var::*, widget_instance::*};
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
/// #[property(SIZE)]
/// pub fn size(child: impl UiNode, size: impl IntoVar<Size>) -> impl UiNode {
///     // ...
///     # child
/// }
/// # #[widget($crate::blank)]
/// # mod blank { inherit!(zero_ui_core::widget_base::base); }
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
/// # use zero_ui_core::{*, var::*, text::*, context::*, widget_instance::*};
/// #[property(LAYOUT)]
/// pub fn foo(child: impl UiNode, bar: impl IntoVar<u32>) -> impl UiNode {
///     #[ui_node(struct FooNode {
///         child: impl UiNode,
///         bar: impl Var<u32>,
///     })]
///     impl UiNode for FooNode {
///         fn init(&mut self, ctx: &mut WidgetContext) {
///             self.child.init(ctx);
///             println!("init: {}", self.bar.get());
///         }
///         
///         fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
///             self.child.update(ctx, updates);
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
/// # pub mod blank { inherit!(zero_ui_core::widget_base::base); }
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
///     when !*#enabled {
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
pub trait IntoVar<T: VarValue> {
    /// Variable type that will wrap the `T` value.
    ///
    /// This is the [`LocalVar`] for most types or `Self` for variable types.
    type Var: Var<T>;

    /// Converts the source value into a var.
    fn into_var(self) -> Self::Var;
}

macro_rules! impl_infallible_write {
    (for<$T:ident>) => {
        /// Infallible [`Var::modify`].
        pub fn modify(&self, vars: &impl WithVars, modify: impl FnOnce(&mut Cow<$T>) + 'static) {
            Var::modify(self, vars, modify).unwrap()
        }

        /// Infallible [`Var::set`].
        pub fn set(&self, vars: &impl WithVars, value: impl Into<$T>) {
            Var::set(self, vars, value).unwrap()
        }

        /// Infallible [`Var::set_ne`].
        pub fn set_ne(&self, vars: &impl WithVars, value: impl Into<$T>)
        where
            $T: PartialEq,
        {
            Var::set_ne(self, vars, value).unwrap()
        }

        /// Infallible [`Var::touch`].
        pub fn touch(&self, vars: &impl WithVars) {
            Var::touch(self, vars).unwrap()
        }
    };
}
use impl_infallible_write;

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
    /// This is `Self` for vars that are always read-only, or [`types::ReadOnlyVar<T, Self>`] for others.
    type ReadOnly: Var<T>;

    /// Output of [`Var::actual_var`].
    ///
    /// This is [`BoxedVar<T>`] for [`ContextVar<T>`], `V` for [`types::RcFlatMapVar<T, V>`] and `Self` for others.
    type ActualVar: Var<T>;

    /// Output of [`Var::downgrade`].
    type Downgrade: WeakVar<T>;

    /// Visit the current value of the variable, inside `read` the variable is locked/borrowed and cannot
    /// be modified.
    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&T) -> R;

    /// Try to schedule a variable update, it will be applied on the end of the current app update.
    ///
    /// The variable only updates if the [`Cow`] is upgraded to [`Cow::Owned`].
    fn modify<V, F>(&self, vars: &V, modify: F) -> Result<(), VarIsReadOnlyError>
    where
        V: WithVars,
        F: FnOnce(&mut Cow<T>) + 'static;

    /// Gets the variable as a [`BoxedVar<T>`], does not double box.
    fn boxed(self) -> BoxedVar<T>
    where
        Self: Sized,
    {
        Box::new(self)
    }

    /// Gets the variable as a [`BoxedAnyVar`], does not double box.
    fn boxed_any(self) -> BoxedAnyVar
    where
        Self: Sized,
    {
        Box::new(self)
    }

    /// Gets the current *inner* var represented by this var. This is the same var, except for [`ContextVar<T>`]
    /// and [`types::RcFlatMapVar<T, V>`].
    fn actual_var(self) -> Self::ActualVar;

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

    /// Create a future that awaits for [`is_new`] to be `true`.
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

    /// Create a future that awaits for [`is_animating`] to be `false`.
    ///
    /// The future can only be used in app bound async code, it can be reused.
    ///
    /// [`is_animating`]: AnyVar::is_animating
    fn wait_animation(&self) -> types::WaitIsNotAnimatingFut<T, Self> {
        types::WaitIsNotAnimatingFut::new(self)
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

    /// Get the value as a debug [`Text`].
    ///
    /// [`Text`]: crate::text::Text
    fn get_debug(&self) -> crate::text::Text {
        self.with(|v| crate::text::formatx!("{v:?}"))
    }

    /// Gets the value as a display [`Text`].
    ///
    /// [`Text`]: crate::text::Text
    fn get_text(&self) -> crate::text::Text
    where
        T: fmt::Display,
    {
        self.with(crate::text::ToText::to_text)
    }

    /// Gets the value as a display [`String`].
    fn get_string(&self) -> String
    where
        T: fmt::Display,
    {
        self.with(ToString::to_string)
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
    /// The `map` closure is called once on initialization, and then once every time
    /// the source variable updates.
    ///
    /// The mapping variable is read-only, you can use [`map_bidi`] to map back.
    ///
    /// Note that the mapping var is [contextualized], meaning the map binding will initialize in the fist usage context, not
    /// the creation context, so `property = CONTEXT_VAR.map(|&b|!b);` will bind with the `CONTEXT_VAR` in the `property` context,
    /// not the property instantiation. The `map` closure itself runs in the root app context, trying to read other context variables
    /// inside it will only read the default value.
    ///
    /// [`map_bidi`]: Var::map_bidi
    /// [contextualized]: types::ContextualizedVar
    fn map<O, M>(&self, map: M) -> types::ContextualizedVar<O, ReadOnlyRcVar<O>>
    where
        O: VarValue,
        M: FnMut(&T) -> O + 'static + Send,
    {
        let me = self.clone();
        let map = Arc::new(Mutex::new(map));
        types::ContextualizedVar::new(Arc::new(move || {
            let other = var(me.with(&mut *map.lock()));
            let map = map.clone();
            me.bind_map(&other, move |t| map.lock()(t)).perm();
            other.read_only()
        }))
    }

    /// Creates a [`map`] that converts from `T` to `O` using [`Into<O>`].
    ///
    /// [`map`]: Var::map
    fn map_into<O>(&self) -> types::ContextualizedVar<O, ReadOnlyRcVar<O>>
    where
        O: VarValue,
        T: Into<O>,
    {
        self.map(|v| v.clone().into())
    }

    /// Creates a [`map`] that converts from `T` to [`Text`] using [`ToText`].
    ///
    /// [`map`]: Var::map
    /// [`Text`]: crate::text::Text
    /// [`ToText`]: crate::text::ToText
    fn map_to_text(&self) -> types::ContextualizedVar<crate::text::Text, ReadOnlyRcVar<crate::text::Text>>
    where
        T: crate::text::ToText,
    {
        self.map(crate::text::ToText::to_text)
    }

    /// Create a [`map`] that converts from `T` to [`String`] using [`ToString`].
    ///
    /// [`map`]: Var::map
    fn map_to_string(&self) -> types::ContextualizedVar<String, ReadOnlyRcVar<String>>
    where
        T: ToString,
    {
        self.map(ToString::to_string)
    }

    /// Create a [`map`] that converts from `T` to a [`Text`] debug print.
    ///
    /// [`map`]: Var::map
    /// [`Text`]: crate::text::Text
    fn map_debug(&self) -> types::ContextualizedVar<crate::text::Text, ReadOnlyRcVar<crate::text::Text>> {
        self.map(|v| crate::text::formatx!("{v:?}"))
    }

    /// Create a ref-counted var that maps from this variable on read and to it on write.
    ///
    /// The `map` closure is called once on initialization, and then once every time
    /// the source variable updates, the `map_back` closure is called every time the output value is modified.
    ///
    /// The mapping var is [contextualized], see [`Var::map`] for more details.
    ///
    /// [contextualized]: types::ContextualizedVar
    fn map_bidi<O, M, B>(&self, map: M, map_back: B) -> types::ContextualizedVar<O, RcVar<O>>
    where
        O: VarValue,
        M: FnMut(&T) -> O + Send + 'static,
        B: FnMut(&O) -> T + Send + 'static,
    {
        let me = self.clone();
        let map = Arc::new(Mutex::new(map));
        let map_back = Arc::new(Mutex::new(map_back));
        types::ContextualizedVar::new(Arc::new(move || {
            let other = var(me.with(&mut *map.lock()));
            let map = map.clone();
            let map_back = map_back.clone();
            me.bind_map_bidi(&other, move |i| map.lock()(i), move |o| map_back.lock()(o)).perm();
            other
        }))
    }

    /// Create a ref-counted var that maps to an inner variable that is found inside the value of this variable.
    ///
    /// The `map` closure is called immediately to clone the initial inner var, and than once every time
    /// the source variable updates.
    ///
    /// The mapping var has the same capabilities of the inner var + `CAPS_CHANGE`, modifying the mapping var modifies the inner var.
    ///
    /// The mapping var is [contextualized], see [`Var::map`] for more details.
    ///
    /// [contextualized]: types::ContextualizedVar
    fn flat_map<O, V, M>(&self, map: M) -> types::ContextualizedVar<O, types::RcFlatMapVar<O, V>>
    where
        O: VarValue,
        V: Var<O>,
        M: FnMut(&T) -> V + Send + 'static,
    {
        let me = self.clone();
        let map = Arc::new(Mutex::new(map));
        types::ContextualizedVar::new(Arc::new(move || {
            let map = map.clone();
            types::RcFlatMapVar::new(&me, move |i| map.lock()(i))
        }))
    }

    /// Creates a ref-counted var that maps from this variable, but can retain a previous mapped value.
    ///
    /// The `map` closure is called once on initialization, if it returns `None` the `fallback` closure is called to generate
    /// a fallback value, after, the `map` closure is called once every time
    /// the mapping variable reads and is out of sync with the source variable, if it returns `Some(_)` the mapping variable value changes,
    /// otherwise the previous value is retained, either way the mapping variable is *new*.
    ///
    /// The mapping variable is read-only, use [`filter_map_bidi`] to map back.
    ///
    /// The mapping var is [contextualized], see [`Var::map`] for more details.
    ///
    /// [contextualized]: types::ContextualizedVar
    /// [`map_bidi`]: Var::map_bidi
    /// [`filter_map_bidi`]: Var::filter_map_bidi
    fn filter_map<O, M, I>(&self, map: M, fallback: I) -> types::ContextualizedVar<O, ReadOnlyRcVar<O>>
    where
        O: VarValue,
        M: FnMut(&T) -> Option<O> + Send + 'static,
        I: Fn() -> O + Send + Sync + 'static,
    {
        let me = self.clone();
        let map = Arc::new(Mutex::new(map));
        types::ContextualizedVar::new(Arc::new(move || {
            let other = var(me.with(&mut *map.lock()).unwrap_or_else(&fallback));
            let map = map.clone();
            me.bind_filter_map(&other, move |i| map.lock()(i)).perm();
            other.read_only()
        }))
    }

    /// Create a [`filter_map`] that tries to convert from `T` to `O` using [`TryInto<O>`].
    ///
    /// [`filter_map`]: Var::filter_map
    fn filter_try_into<O, I>(&self, fallback: I) -> types::ContextualizedVar<O, ReadOnlyRcVar<O>>
    where
        O: VarValue,
        T: TryInto<O>,
        I: Fn() -> O + Send + Sync + 'static,
    {
        self.filter_map(|v| v.clone().try_into().ok(), fallback)
    }

    /// Create a [`filter_map`] that tries to convert from `T` to `O` using [`FromStr`].
    ///
    /// [`filter_map`]: Var::filter_map
    /// [`FromStr`]: std::str::FromStr
    fn filter_parse<O, I>(&self, fallback: I) -> types::ContextualizedVar<O, ReadOnlyRcVar<O>>
    where
        O: VarValue + std::str::FromStr,
        T: AsRef<str>,
        I: Fn() -> O + Send + Sync + 'static,
    {
        self.filter_map(|v| v.as_ref().parse().ok(), fallback)
    }

    /// Create a ref-counted var that maps from this variable on read and to it on write, mapping in both directions can skip
    /// a value, retaining the previous mapped value.
    ///
    /// The `map` closure is called once on initialization, if it returns `None` the `fallback` closure is called
    /// to generate a fallback value, after, the `map` closure is called once every time
    /// the mapping variable reads and is out of sync with the source variable, if it returns `Some(_)` the mapping variable value changes,
    /// otherwise the previous value is retained, either way the mapping variable is *new*. The `map_back` closure
    /// is called every time the output value is modified, if it returns `Some(_)` the source variable is set, otherwise the source
    /// value is not touched.
    ///
    /// The mapping var is [contextualized], see [`Var::map`] for more details.
    ///
    /// [contextualized]: types::ContextualizedVar
    fn filter_map_bidi<O, M, B, I>(&self, map: M, map_back: B, fallback: I) -> types::ContextualizedVar<O, RcVar<O>>
    where
        O: VarValue,
        M: FnMut(&T) -> Option<O> + Send + 'static,
        B: FnMut(&O) -> Option<T> + Send + 'static,
        I: Fn() -> O + Send + Sync + 'static,
    {
        let me = self.clone();
        let map = Arc::new(Mutex::new(map));
        let map_back = Arc::new(Mutex::new(map_back));
        types::ContextualizedVar::new(Arc::new(move || {
            let other = var(me.with(&mut *map.lock()).unwrap_or_else(&fallback));
            let map = map.clone();
            let map_back = map_back.clone();
            me.bind_filter_map_bidi(&other, move |i| map.lock()(i), move |o| map_back.lock()(o))
                .perm();
            other
        }))
    }

    /// Create a mapping wrapper around `self`. The `map` closure is called for each value access, it must reference the
    /// value `O` that already exists in `T`.
    fn map_ref<O, M>(&self, map: M) -> types::MapRef<T, O, Self>
    where
        O: VarValue,
        M: Fn(&T) -> &O + Send + Sync + 'static,
    {
        types::MapRef::new(self.clone(), Arc::new(map))
    }

    /// Create a mapping wrapper around `self`. The `map` closure is called for each value access, it must reference the
    /// value `O` that already exists in `T`, the `map_mut` closure is called for every modify request, it must do the same
    /// as `map` but with mutable access.
    fn map_ref_bidi<O, M, B>(&self, map: M, map_mut: B) -> types::MapRefBidi<T, O, Self>
    where
        O: VarValue,
        M: Fn(&T) -> &O + Send + Sync + 'static,
        B: Fn(&mut T) -> &mut O + Send + Sync + 'static,
    {
        types::MapRefBidi::new(self.clone(), Arc::new(map), Arc::new(map_mut))
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
        M: FnMut(&T) -> T2 + Send + 'static,
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
        F: FnMut(&T) -> Option<T2> + Send + 'static,
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
    fn bind_map_bidi<T2, V2, M, B>(&self, other: &V2, mut map: M, mut map_back: B) -> VarHandles
    where
        T2: VarValue,
        V2: Var<T2>,
        M: FnMut(&T) -> T2 + Send + 'static,
        B: FnMut(&T2) -> T + Send + 'static,
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

        [self_to_other, other_to_self].into_iter().collect()
    }

    /// Bind `self` to `other` and back with the new values of `self` transformed by `map` and the new values of `other` transformed
    /// by `map_back`, the value is assigned in a update only if the closures returns a value.
    ///
    /// Only a weak reference to the `other` variable is held, both variables update in the same app update cycle.
    ///
    /// Note that the current value is not assigned, only the subsequent updates, you can assign
    /// `other` and then bind to fully sync the variables.
    fn bind_filter_map_bidi<T2, V2, M, B>(&self, other: &V2, mut map: M, mut map_back: B) -> VarHandles
    where
        T2: VarValue,
        V2: Var<T2>,
        M: FnMut(&T) -> Option<T2> + Send + 'static,
        B: FnMut(&T2) -> Option<T> + Send + 'static,
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

        [self_to_other, other_to_self].into_iter().collect()
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
    fn bind_bidi<V2>(&self, other: &V2) -> VarHandles
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
        vars.with_vars(|vars| VarSender::new(vars, self))
    }

    /// Creates a sender that modify `self` from other threads and without access to [`Vars`].
    ///
    /// If the variable is read-only when a modification is received it is silently dropped.
    fn modify_sender<V>(&self, vars: &V) -> VarModifySender<T>
    where
        V: WithVars,
    {
        vars.with_vars(|vars| VarModifySender::new(vars, self))
    }

    /// Creates a channel that can receive `var` updates from another thread.
    ///
    /// Every time the variable updates a clone of the value is sent to the receiver. The current value is sent immediately.
    fn receiver<V>(&self) -> VarReceiver<T>
    where
        T: Send,
    {
        VarReceiver::new(self, true)
    }

    /// Add a preview `handler` that is called every time this variable value is set, modified or touched,
    /// the handler is called before all other UI updates.
    ///
    /// Note that the handler runs on the app context, all [`ContextVar<T>`] read inside read the default value.
    fn on_pre_new<H>(&self, handler: H) -> VarHandle
    where
        H: AppHandler<T>,
    {
        var_on_new(self, handler, true)
    }

    // Add a `handler` that is called every time this variable value is set, modified or touched,
    /// the handler is called after all other UI updates.
    ///
    /// Note that the handler runs on the app context, all [`ContextVar<T>`] read inside read the default value.
    fn on_new<H>(&self, handler: H) -> VarHandle
    where
        H: AppHandler<T>,
    {
        var_on_new(self, handler, false)
    }

    /// Debug helper for tracing the lifetime of a value in this variable.
    ///
    /// The `enter_value` closure is called every time the variable value is set, modified or touched, it can return
    /// an implementation agnostic *scope* or *span* `S` that is only dropped when the variable updates again.
    ///
    /// The `enter_value` is also called immediately when this method is called to start tracking the first value.
    ///
    /// Returns a [`VarHandle`] that can be used to stop tracing. Making the handle permanent means that the tracing will happen
    /// for the variable or app, the tracing handler only holds a weak reference to the variable.
    ///
    /// If this variable can never update the span is immediately dropped and a dummy handle is returned. Note that
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
    /// [`actual_var`]: Var::actual_var
    fn trace_value<E, S>(&self, mut enter_value: E) -> VarHandle
    where
        E: FnMut(&T) -> S + Send + 'static,
        S: Send + 'static,
    {
        let mut span = Some(self.with(&mut enter_value));
        self.on_pre_new(app_hn!(|_, value, _| {
            let _ = span.take();
            span = Some(enter_value(value));
        }))
    }

    /// Schedule an animation that targets this variable.
    ///
    /// If the variable is always read-only no animation is created and a dummy handle returned. The animation
    /// targets the current [`actual_var`] and is stopped if the variable is dropped.
    ///
    /// The `animate` closure is called every frame with the [`AnimationArgs`] and *modify* access to the variable value, the args
    /// can be used to calculate the new variable value and to control or stop the animation.
    ///
    /// [`actual_var`]: Var::actual_var
    /// [`AnimationArgs`]: animation::AnimationArgs
    fn animate<V, A>(&self, vars: &V, animate: A) -> animation::AnimationHandle
    where
        V: WithVars,
        A: FnMut(&animation::AnimationArgs, &mut Cow<T>) + 'static,
    {
        vars.with_vars(move |vars| animation::var_animate(vars, self, animate))
    }

    /// Schedule an easing transition from the `start_value` to `end_value`.
    ///
    /// The variable updates every time the [`EasingStep`] for each frame changes, it will update even
    /// if the [`animation::Transition`] samples the same value, you can use [`ease_ne`] to only update
    /// when the value changes.
    ///
    /// See [`Var::animate`] for details about animations.
    ///
    /// [`ease_ne`]: Var::ease_ne
    fn set_ease<V, S, E, F>(&self, vars: &V, start_value: S, end_value: E, duration: Duration, easing: F) -> animation::AnimationHandle
    where
        T: animation::Transitionable,
        V: WithVars,
        S: Into<T>,
        E: Into<T>,
        F: Fn(EasingTime) -> EasingStep + 'static,
    {
        self.animate(
            vars,
            animation::var_set_ease(start_value.into(), end_value.into(), duration, easing, 999.fct()),
        )
    }

    /// Schedule an easing transition from the current value to `new_value`.
    ///
    /// The variable updates every time the [`EasingStep`] for each frame changes, it will update even
    /// if the [`animation::Transition`] samples the same value, you can use [`ease_ne`] to only update
    /// when the value changes.
    ///
    /// See [`Var::animate`] for details about animations.
    ///
    /// [`ease_ne`]: Var::ease_ne
    fn ease<V, E, F>(&self, vars: &V, new_value: E, duration: Duration, easing: F) -> animation::AnimationHandle
    where
        T: animation::Transitionable,
        V: WithVars,
        E: Into<T>,
        F: Fn(EasingTime) -> EasingStep + 'static,
    {
        self.animate(
            vars,
            animation::var_set_ease(self.get(), new_value.into(), duration, easing, 0.fct()),
        )
    }

    /// Like [`set_ease`] but checks if the sampled value actually changed before updating.
    ///
    /// [`set_ease`]: Var::set_ease
    fn set_ease_ne<V, S, E, F>(&self, vars: &V, start_value: S, end_value: E, duration: Duration, easing: F) -> animation::AnimationHandle
    where
        T: animation::Transitionable + PartialEq,
        V: WithVars,
        S: Into<T>,
        E: Into<T>,
        F: Fn(EasingTime) -> EasingStep + 'static,
    {
        self.animate(
            vars,
            animation::var_set_ease_ne(start_value.into(), end_value.into(), duration, easing, 999.fct()),
        )
    }

    /// Like [`ease`] but checks if the sampled value actually changed before updating.
    ///
    /// [`ease`]: Var::ease
    fn ease_ne<V, E, F>(&self, vars: &V, new_value: E, duration: Duration, easing: F) -> animation::AnimationHandle
    where
        T: animation::Transitionable + PartialEq,
        V: WithVars,
        E: Into<T>,
        F: Fn(EasingTime) -> EasingStep + 'static,
    {
        self.animate(
            vars,
            animation::var_set_ease_ne(self.get(), new_value.into(), duration, easing, 0.fct()),
        )
    }

    /// Schedule a keyframed transition animation for the variable, starting from the first key.
    ///
    /// The variable will be set to to the first keyframe, then animated across all other keys.
    ///
    /// See [`Var::animate`] for details about animations.
    fn set_ease_keyed<V, F>(&self, vars: &V, keys: Vec<(Factor, T)>, duration: Duration, easing: F) -> animation::AnimationHandle
    where
        T: animation::Transitionable,
        V: WithVars,
        F: Fn(EasingTime) -> EasingStep + 'static,
    {
        if let Some(transition) = animation::TransitionKeyed::new(keys) {
            self.animate(vars, animation::var_set_ease_keyed(transition, duration, easing, 999.fct()))
        } else {
            animation::AnimationHandle::dummy()
        }
    }

    /// Schedule a keyframed transition animation for the variable, starting from the current value.
    ///
    /// The variable will be set to to the first keyframe, then animated across all other keys.
    ///
    /// See [`Var::animate`] for details about animations.
    fn ease_keyed<V, F>(&self, vars: &V, mut keys: Vec<(Factor, T)>, duration: Duration, easing: F) -> animation::AnimationHandle
    where
        T: animation::Transitionable,
        V: WithVars,
        F: Fn(EasingTime) -> EasingStep + 'static,
    {
        keys.insert(0, (0.fct(), self.get()));

        let transition = animation::TransitionKeyed::new(keys).unwrap();
        self.animate(vars, animation::var_set_ease_keyed(transition, duration, easing, 0.fct()))
    }

    /// Like [`set_ease_keyed`] but checks if the sampled value actually changed before updating.
    ///
    /// [`set_ease_keyed`]: Var::set_ease_keyed
    fn set_ease_keyed_ne<V, F>(&self, vars: &V, keys: Vec<(Factor, T)>, duration: Duration, easing: F) -> animation::AnimationHandle
    where
        T: animation::Transitionable + PartialEq,
        V: WithVars,
        F: Fn(EasingTime) -> EasingStep + 'static,
    {
        if let Some(transition) = animation::TransitionKeyed::new(keys) {
            self.animate(vars, animation::var_set_ease_keyed_ne(transition, duration, easing, 999.fct()))
        } else {
            animation::AnimationHandle::dummy()
        }
    }

    /// Like [`ease_keyed`] but checks if the sampled value actually changed before updating.
    ///
    /// [`ease_keyed`]: Var::ease_keyed
    fn ease_keyed_ne<V, F>(&self, vars: &V, mut keys: Vec<(Factor, T)>, duration: Duration, easing: F) -> animation::AnimationHandle
    where
        T: animation::Transitionable + PartialEq,
        V: WithVars,
        F: Fn(EasingTime) -> EasingStep + 'static,
    {
        keys.insert(0, (0.fct(), self.get()));

        let transition = animation::TransitionKeyed::new(keys).unwrap();
        self.animate(vars, animation::var_set_ease_keyed_ne(transition, duration, easing, 0.fct()))
    }

    /// Set the variable to `new_value` after a `delay`.
    ///
    /// The variable [`is_animating`] until the delay elapses and the value is set.
    ///
    /// See [`Var::animate`] for details about animations.
    ///
    /// [`is_animating`]: AnyVar::is_animating
    fn step<V, N>(&self, vars: &V, new_value: N, delay: Duration) -> animation::AnimationHandle
    where
        V: WithVars,
        N: Into<T>,
    {
        self.animate(vars, animation::var_step(new_value.into(), delay))
    }

    /// Like [`step`], but only update the variable if the `new_value` is not equal at the moment the `delay` elapses.
    ///
    /// [`step`]: Var::step
    fn step_ne<V, N>(&self, vars: &V, new_value: N, delay: Duration) -> animation::AnimationHandle
    where
        T: PartialEq,
        V: WithVars,
        N: Into<T>,
    {
        self.animate(vars, animation::var_step_ne(new_value.into(), delay))
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
    /// [`is_animating`]: AnyVar::is_animating
    /// [`AnimationHandle`]: animation::AnimationHandle
    fn steps<V, F>(&self, vars: &V, steps: Vec<(Factor, T)>, duration: Duration, easing: F) -> animation::AnimationHandle
    where
        V: WithVars,
        F: Fn(EasingTime) -> EasingStep + 'static,
    {
        self.animate(vars, animation::var_steps(steps, duration, easing))
    }

    /// Like [`steps`], but the variable only updates if the selected step is not equal.
    ///
    /// [`steps`]: Var::steps
    fn steps_ne<V, F>(&self, vars: &V, steps: Vec<(Factor, T)>, duration: Duration, easing: F) -> animation::AnimationHandle
    where
        T: PartialEq,
        V: WithVars,
        F: Fn(EasingTime) -> EasingStep + 'static,
    {
        self.animate(vars, animation::var_steps_ne(steps, duration, easing))
    }

    /// Starts an easing animation that *chases* a target value that can be changed using the [`ChaseAnimation<T>`] handle.
    ///
    /// [`ChaseAnimation<T>`]: animation::ChaseAnimation
    fn chase<V, N, F>(&self, vars: &V, first_target: N, duration: Duration, easing: F) -> animation::ChaseAnimation<T>
    where
        V: WithVars,
        N: Into<T>,
        F: Fn(EasingTime) -> EasingStep + 'static,
        T: animation::Transitionable,
    {
        let (anim, next_target) = animation::var_chase(self.get(), first_target.into(), duration, easing);
        let handle = self.animate(vars, anim);
        animation::ChaseAnimation { handle, next_target }
    }

    /// Starts a [`chase`] animation that eases to a target value, but does not escape `bounds`.
    ///
    /// [`chase`]: Var::chase
    fn chase_bounded<V, N, F>(
        &self,
        vars: &V,
        first_target: N,
        duration: Duration,
        easing: F,
        bounds: ops::RangeInclusive<T>,
    ) -> animation::ChaseAnimation<T>
    where
        V: WithVars,
        N: Into<T>,
        F: Fn(EasingTime) -> EasingStep + 'static,
        T: animation::Transitionable + std::cmp::PartialOrd<T>,
    {
        let (anim, next_target) = animation::var_chase_bounded(self.get(), first_target.into(), duration, easing, bounds);
        let handle = self.animate(vars, anim);
        animation::ChaseAnimation { handle, next_target }
    }

    /// Create a vars that [`ease`] to each new value of `self`.
    ///
    /// Note that the mapping var is [contextualized], meaning the binding will initialize in the fist usage context, not
    /// the creation context, so `property = CONTEXT_VAR.easing(500.ms(), easing::linear);` will bind with the `CONTEXT_VAR` in the `property` context,
    /// not the property instantiation.
    ///
    /// [contextualized]: types::ContextualizedVar
    /// [`ease`]: Var::ease
    fn easing<F>(&self, duration: Duration, easing: F) -> types::ContextualizedVar<T, ReadOnlyRcVar<T>>
    where
        T: animation::Transitionable,
        F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
    {
        let source = self.clone();
        let easing_fn = Arc::new(easing);
        types::ContextualizedVar::new(Arc::new(move || {
            let easing_var = var(source.get());

            let easing_fn = easing_fn.clone();
            let mut _anim_handle = animation::AnimationHandle::dummy();
            var_bind(&source, &easing_var, move |vars, _, value, easing_var| {
                let easing_fn = easing_fn.clone();
                _anim_handle = easing_var.ease(vars, value.clone(), duration, move |t| easing_fn(t));
            })
            .perm();
            easing_var.read_only()
        }))
    }

    /// Line [`easing`], but uses [`ease_ne`] to animate.
    ///
    /// [`easing`]: Var::easing
    /// [`ease_ne`]: Var::ease_ne
    fn easing_ne<F>(&self, duration: Duration, easing: F) -> types::ContextualizedVar<T, ReadOnlyRcVar<T>>
    where
        T: animation::Transitionable + PartialEq,
        F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
    {
        let source = self.clone();
        let easing_fn = Arc::new(easing);
        types::ContextualizedVar::new(Arc::new(move || {
            let easing_var = var(source.get());

            let easing_fn = easing_fn.clone();

            let mut _anim_handle = animation::AnimationHandle::dummy();
            var_bind(&source, &easing_var, move |vars, _, value, easing_var| {
                let easing_fn = easing_fn.clone();
                _anim_handle = easing_var.ease_ne(vars, value.clone(), duration, move |t| easing_fn(t));
            })
            .perm();
            easing_var.read_only()
        }))
    }

    /// Returns a wrapper that implements [`fmt::Debug`] to write the var value.
    fn debug(&self) -> types::VarDebug<T, Self> {
        types::VarDebug {
            var: self,
            _t: PhantomData,
        }
    }

    /// Returns a wrapper that implements [`fmt::Display`] to write the var value.
    fn display(&self) -> types::VarDisplay<T, Self>
    where
        T: fmt::Display,
    {
        types::VarDisplay {
            var: self,
            _t: PhantomData,
        }
    }

    /*
    after https://github.com/rust-lang/rust/issues/20041

    /// Replaces `self` with the current [`actual_var`] if both are the same type.
    fn actualize_in_place(&mut self) where Self::ActualVar = Self {
        take_mut::take(self, Var::actual_var)
    }
    */
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
fn var_set<T>(value: T) -> impl FnOnce(&mut Cow<T>)
where
    T: VarValue,
{
    move |var_value| {
        *var_value = Cow::Owned(value);
    }
}
fn var_set_ne<T>(value: T) -> impl FnOnce(&mut Cow<T>)
where
    T: VarValue + PartialEq,
{
    move |var_value| {
        if var_value.as_ref() != &value {
            *var_value = Cow::Owned(value);
        }
    }
}
fn var_set_any<T>(value: Box<dyn AnyVarValue>) -> impl FnOnce(&mut Cow<T>)
where
    T: VarValue,
{
    match value.into_any().downcast::<T>() {
        Ok(value) => var_set(*value),
        Err(_) => panic!("cannot `set_any`, incompatible type"),
    }
}

fn var_touch<T>(var_value: &mut Cow<T>)
where
    T: VarValue,
{
    var_value.to_mut();
}

fn var_subscribe(widget_id: WidgetId) -> Box<dyn Fn(&Vars, &mut Updates, &dyn AnyVarValue) -> bool + Send + Sync> {
    Box::new(move |_, updates, _| {
        updates.update(widget_id);
        true
    })
}

fn var_bind<I: VarValue, O: VarValue, V: Var<O>>(
    input: &impl Var<I>,
    output: &V,
    update_output: impl FnMut(&Vars, &mut Updates, &I, <V::Downgrade as WeakVar<O>>::Upgrade) + Send + 'static,
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
    update_output: impl FnMut(&Vars, &mut Updates, &I, W::Upgrade) + Send + 'static,
) -> VarHandle {
    let update_output = Mutex::new(update_output);
    input.hook(Box::new(move |vars, updates, value| {
        if let Some(output) = wk_output.upgrade() {
            if output.capabilities().contains(VarCapabilities::MODIFY) {
                if let Some(value) = value.as_any().downcast_ref::<I>() {
                    update_output.lock()(vars, updates, value, output);
                }
            }
            true
        } else {
            false
        }
    }))
}

fn var_on_new<T: VarValue>(var: &impl Var<T>, handler: impl AppHandler<T>, is_preview: bool) -> VarHandle {
    if var.capabilities().is_always_static() {
        return VarHandle::dummy();
    }

    let handler = Arc::new(Mutex::new(handler));
    let (inner_handle_owner, inner_handle) = crate::crate_util::Handle::new(());
    var.hook(Box::new(move |_, updates, value| {
        if inner_handle_owner.is_dropped() {
            return false;
        }

        if let Some(value) = value.as_any().downcast_ref::<T>() {
            let handle = inner_handle.downgrade();
            let update_once = app_hn_once!(handler, value, |ctx, _| {
                handler.lock().event(
                    ctx,
                    &value,
                    &AppHandlerArgs {
                        handle: &handle,
                        is_preview,
                    },
                );
            });

            if is_preview {
                updates.on_pre_update(update_once).perm();
            } else {
                updates.on_update(update_once).perm();
            }
        }
        true
    }))
}
