//! Batch updated variables in an app context.

// suppress nag about very simple boxed closure signatures.
#![allow(clippy::type_complexity)]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use animation::{
    easing::{EasingStep, EasingTime},
    Transitionable,
};
use bitflags::bitflags;
use parking_lot::Mutex;
use std::{
    any::{Any, TypeId},
    borrow::Cow,
    fmt,
    marker::PhantomData,
    ops,
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc,
    },
    time::Duration,
};
use zero_ui_app_context::ContextLocal;
use zero_ui_clone_move::clmv;
use zero_ui_txt::{formatx, ToText, Txt};
use zero_ui_unique_id::unique_id_32;
use zero_ui_unit::{Factor, FactorUnits};

pub mod animation;
mod arc;
mod boxed;
mod impls;

mod context;
mod contextualized;
mod cow;
mod expr;
mod flat_map;
mod future;
mod local;
mod map_ref;
mod merge;
mod read_only;
mod response;
mod vars;
mod vec;
mod when;

#[macro_use]
mod util;

pub use arc::{getter_var, state_var, var, var_default, var_from, ArcVar};
pub use boxed::{BoxedAnyVar, BoxedAnyWeakVar, BoxedVar, BoxedWeakVar};
#[doc(inline)]
pub use context::{ContextInitHandle, ContextVar, ReadOnlyContextVar};
pub use local::LocalVar;
#[doc(inline)]
pub use merge::MergeVarBuilder;
pub use read_only::ReadOnlyArcVar;
pub use response::{response_done_var, response_var, ResponderVar, ResponseVar};
pub use vars::*;
pub use vec::ObservableVec;

/// Other variable types.
pub mod types {
    use std::marker::PhantomData;

    #[doc(hidden)]
    pub use zero_ui_app_context::context_local;

    pub use super::arc::WeakArcVar;
    pub use super::boxed::{VarBoxed, WeakVarBoxed};
    pub use super::context::{context_var_init, WeakContextInitHandle};
    pub use super::contextualized::{ContextualizedVar, WeakContextualizedVar};
    pub use super::cow::{ArcCowVar, WeakCowVar};
    pub use super::expr::{__expr_var, expr_var_as, expr_var_into, expr_var_map};
    pub use super::flat_map::{ArcFlatMapVar, WeakFlatMapVar};
    pub use super::future::{WaitIsNotAnimatingFut, WaitUpdateFut};
    pub use super::map_ref::{MapRef, MapRefBidi, WeakMapRef, WeakMapRefBidi};
    pub use super::merge::{ArcMergeVar, ArcMergeVarInput, ContextualizedArcMergeVar, MergeVarInputs, WeakMergeVar, __merge_var};
    pub use super::read_only::{ReadOnlyVar, WeakReadOnlyVar};
    pub use super::response::Response;
    pub use super::vec::VecChange;
    pub use super::when::{AnyWhenVarBuilder, ArcWhenVar, ContextualizedArcWhenVar, WeakWhenVar, WhenVarBuilder, __when_var};

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

mod private {
    // https://rust-lang.github.io/api-guidelines/future-proofing.html#sealed-traits-protect-against-downstream-implementations-c-sealed
    pub trait Sealed {}
}

/// A type that can be a [`Var<T>`] value.
///
/// # Trait Alias
///
/// This trait is used like a type alias for traits and is
/// already implemented for all types it applies to.
///
/// # Implementing
///
/// Types need to be `Debug + Clone + PartialEq + Send + Sync + Any` to auto-implement this trait,
/// if you want to place an external type in a variable and it does not implement all the traits
/// you may need to declare a *newtype* wrapper, if the external type is `Debug + Send + Sync + Any` at
/// least you can use the [`ArcEq<T>`] wrapper to quickly implement `Clone + PartialEq`, this is particularly
/// useful for error types in [`ResponseVar<Result<_, E>>`].
pub trait VarValue: fmt::Debug + Clone + PartialEq + Any + Send + Sync {}
impl<T: fmt::Debug + Clone + Any + PartialEq + Send + Sync> VarValue for T {}

/// Trait implemented for all [`VarValue`] types.
pub trait AnyVarValue: fmt::Debug + Any + Send + Sync {
    /// Access to `dyn Any` methods.
    fn as_any(&self) -> &dyn Any;

    /// Access to mut `dyn Any` methods.
    fn as_any_mut(&mut self) -> &mut dyn Any;

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

    fn as_any_mut(&mut self) -> &mut dyn Any {
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
/// # macro_rules! _demo { () => {
/// #[property(CONTEXT)]
/// pub fn foo(child: impl UiNode, a: impl IntoValue<bool>, b: impl IntoValue<bool>) -> impl UiNode {
///     let a = a.into();
///     let b = b.into();
///
///     match_node(child, move |_, op| match op {
///         UiNodeOp::Init => {
///             println!("a: {a:?}, b: {b:?}");
///         },
///         _ => {}
///     })
/// }
/// # }}
/// ```
///
/// # Implementing
///
/// The trait is only auto-implemented for `T: Into<T> + VarValue`, unfortunately actual type conversions
/// must be manually implemented, note that the [`impl_from_and_into_var!`] macro auto-implements this conversion.
///
/// [inspected]: crate::inspector
/// [`Debug`]: std::fmt::Debug
/// [`impl_from_and_into_var`]: impl_from_and_into_var
pub trait IntoValue<T: VarValue>: Into<T> {}
impl<T: VarValue> IntoValue<T> for T {}

bitflags! {
    /// Kinds of interactions allowed by a [`Var<T>`] in the current update.
    ///
    /// You can get the current capabilities of a var by using the [`AnyVar::capabilities`] method.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct VarCapabilities: u8 {
        /// Var value can change.
        ///
        /// If this is set the [`Var::is_new`] can be `true` in some updates, a variable can `NEW`
        /// even if it cannot `MODIFY`, in this case the variable is a read-only wrapper on a read-write variable.
        const NEW = 0b0000_0010;

        /// Var can be modified.
        ///
        /// If this is set [`Var::modify`] always returns `Ok`, if this is set `NEW` is also set.
        ///
        /// Note that modify requests from inside overridden animations can still be ignored, see [`Var::modify_importance`].
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
    pub fn as_read_only(self) -> Self {
        Self::from_bits_truncate(self.bits() & 0b1111_1110)
    }

    /// If cannot `MODIFY` and is not `CAPS_CHANGE`.
    pub fn is_always_read_only(self) -> bool {
        !self.contains(Self::MODIFY) && !self.contains(Self::CAPS_CHANGE)
    }

    /// If cannot `NEW` and is not `CAPS_CHANGE`.
    pub fn is_always_static(self) -> bool {
        self.is_empty()
    }

    /// Has the `MODIFY` capability.
    pub fn can_modify(self) -> bool {
        self.contains(Self::MODIFY)
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
    action: Box<dyn Fn(&AnyVarHookArgs) -> bool + Send + Sync>,
}

/// Represents the var side of a [`VarHandle`].
struct VarHook(Arc<VarHandleData>);
impl VarHook {
    /// Calls the handle action, returns `true` if the handle must be retained.
    pub fn call(&self, args: &AnyVarHookArgs) -> bool {
        self.is_alive() && (self.0.action)(args)
    }

    /// If the handle is still held or is permanent.
    pub fn is_alive(&self) -> bool {
        Arc::strong_count(&self.0) > 1 || self.0.perm.load(Relaxed)
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
    /// New handle, the `action` depends on the behavior the handle represents.
    fn new(action: Box<dyn Fn(&AnyVarHookArgs) -> bool + Send + Sync>) -> (VarHandle, VarHook) {
        let c = Arc::new(VarHandleData {
            perm: AtomicBool::new(false),
            action,
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
            s.perm.store(true, Relaxed);
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
impl Default for VarHandle {
    fn default() -> Self {
        Self::dummy()
    }
}

/// Represents a collection of var handles.
#[must_use = "var handles stops the behaviour they represents on drop"]
#[derive(Clone, Default)]
pub struct VarHandles(pub Vec<VarHandle>);
impl VarHandles {
    /// Empty collection.
    pub const fn dummy() -> Self {
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

/// Arc value that implements equality by pointer comparison.
///
/// This type allows types external types that are only `Debug + Send + Sync` to become
/// a full [`VarValue`] to be allowed as a variable value.
pub struct ArcEq<T: fmt::Debug + Send + Sync>(pub Arc<T>);
impl<T: fmt::Debug + Send + Sync> ops::Deref for ArcEq<T> {
    type Target = Arc<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T: fmt::Debug + Send + Sync> ArcEq<T> {
    /// Constructs a new `ArcEq<T>`.
    pub fn new(value: T) -> Self {
        Self(Arc::new(value))
    }
}
impl<T: fmt::Debug + Send + Sync> PartialEq for ArcEq<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl<T: fmt::Debug + Send + Sync> Eq for ArcEq<T> {}
impl<T: fmt::Debug + Send + Sync> Clone for ArcEq<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}
impl<T: fmt::Debug + Send + Sync> fmt::Debug for ArcEq<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&*self.0, f)
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

    /// Access to `dyn Any` methods, on the underlying variable type if boxed.
    fn as_unboxed_any(&self) -> &dyn Any;

    /// Access to `Box<dyn Any>` methods, with the [`BoxedVar<T>`] type.
    ///
    /// This is a double-boxed to allow downcast to [`BoxedVar<T>`].
    fn double_boxed_any(self: Box<Self>) -> Box<dyn Any>;

    /// Gets the [`TypeId`] of `T` in `Var<T>`.
    fn var_type_id(&self) -> TypeId;

    /// Get a clone of the current value, with type erased.
    fn get_any(&self) -> Box<dyn AnyVarValue>;

    /// Visit the current value of the variable.
    fn with_any(&self, read: &mut dyn FnMut(&dyn AnyVarValue));

    /// Visit the current value of the variable, if it [`is_new`].
    ///
    /// [`is_new`]: AnyVar::is_new
    fn with_new_any(&self, read: &mut dyn FnMut(&dyn AnyVarValue)) -> bool;

    /// Schedule a new `value` for the variable, it will be set in the end of the current app update.
    ///
    /// # Panics
    ///
    /// Panics if the `value` is not of the same [`var_type_id`].
    ///
    /// [`var_type_id`]: AnyVar::var_type_id
    fn set_any(&self, value: Box<dyn AnyVarValue>) -> Result<(), VarIsReadOnlyError>;

    /// Last update ID a variable was modified, if the ID is equal to [`VARS.update_id`] the variable is *new*.
    fn last_update(&self) -> VarUpdateId;

    /// If the variable represents different values depending on the context where they are read.
    fn is_contextual(&self) -> bool;

    /// Flags that indicate what operations the variable is capable of.
    fn capabilities(&self) -> VarCapabilities;

    /// Gets if the [`last_update`] is the current update, meaning the variable value just changed.
    ///
    /// [`last_update`]: AnyVar::last_update
    fn is_new(&self) -> bool {
        VARS.update_id() == self.last_update()
    }

    /// If the variable current value was set by an active animation.
    ///
    /// The variable [`is_new`] when this changes to `true`, but it **may not be new** when the value changes to `false`.
    /// If the variable is not updated at the last frame of the animation that has last set it, it will not update
    /// just because that animation has ended. You can use [`hook_animation_stop`] to get a notification when the
    /// last animation stops, or use [`wait_animation`] to get a future that is ready when `is_animating` changes
    /// from `true` to `false`.
    ///
    /// [`is_new`]: AnyVar::is_new
    /// [`hook_animation_stop`]: AnyVar::hook_animation_stop
    /// [`wait_animation`]: Var::wait_animation
    fn is_animating(&self) -> bool;

    /// Gets a value that indicates the *importance* clearance that is needed to modify this variable.
    ///
    /// If the variable has the [`MODIFY`] capability, the requests will return `Ok(())`, but they will be ignored
    /// if the [`VARS.current_modify`] importance is less than the variable's at the moment the request is made.
    ///
    /// Note that [`VARS.current_modify`] outside animations always overrides this value, so direct modify requests
    /// always override running animations.
    ///
    /// This is the mechanism that ensures that only the latest animation has *control* of the variable value, most animations
    /// check this value and automatically cancel if overridden, but event assigns from custom animations made using [`VARS.animate`]
    /// are ignored if the variable is modified from a newer source then the animation.
    ///
    /// If the variable does not have [`MODIFY`] capability the value returned is undefined.
    ///
    /// [`MODIFY`]: VarCapabilities::MODIFY
    fn modify_importance(&self) -> usize;

    /// Setups a callback for just after the variable value update is applied, the closure runs in the root app context, just like
    /// the `modify` closure. The closure can returns if it is retained after each call.
    ///
    /// Variables store a weak[^1] reference to the callback if they have the `MODIFY` or `CAPS_CHANGE` capabilities, otherwise
    /// the callback is discarded and [`VarHandle::dummy`] returned.
    ///
    /// [^1]: You can use the [`VarHandle::perm`] to make the stored reference *strong*.
    fn hook_any(&self, pos_modify_action: Box<dyn Fn(&AnyVarHookArgs) -> bool + Send + Sync>) -> VarHandle;

    /// Register a `handler` to be called when the current animation stops.
    ///
    /// Note that the `handler` is owned by the animation, not the variable, it will only be called/dropped when the
    /// animation stops.
    ///
    /// Returns the `handler` as an error if the variable is not animating. Note that if you are interacting
    /// with the variable from a non-UI thread the variable can stops animating between checking [`is_animating`]
    /// and registering the hook, in this case the `handler` will be returned as an error as well.
    ///
    /// [`modify_importance`]: AnyVar::modify_importance
    /// [`is_animating`]: AnyVar::is_animating
    fn hook_animation_stop(&self, handler: Box<dyn FnOnce() + Send>) -> Result<(), Box<dyn FnOnce() + Send>>;

    /// Gets the number of strong references to the variable.
    ///
    /// This is the [`Arc::strong_count`] for *Arc* variables, the represented var count for [`ContextVar<T>`], the boxed var count
    /// for [`BoxedVar<T>`] and `0` for [`LocalVar<T>`].
    fn strong_count(&self) -> usize;

    /// Gets the number of weak references to the variable.
    ///
    /// This is the [`Arc::weak_count`] for *Arc* variables, the represented var count for [`ContextVar<T>`], the boxed var count
    /// for [`BoxedVar<T>`] and `0` for [`LocalVar<T>`].
    fn weak_count(&self) -> usize;

    /// Gets a clone of the represented var from [`ContextVar<T>`], gets a clone of `self` for other var types.
    fn actual_var_any(&self) -> BoxedAnyVar;

    /// Create a weak reference to this *Arc* variable.
    ///
    /// The weak reference is made to the [`actual_var`], if the actual var is a [`LocalVar<T>`]
    /// a [`types::WeakArcVar<T>`] is returned, for *Arc* vars an actual weak reference is made.
    ///
    /// [`actual_var`]: Var::actual_var
    fn downgrade_any(&self) -> BoxedAnyWeakVar;

    /// Var *pointer*, that can be used to identify if two variables point to the same *rc* or *context*.
    ///
    /// If two of these values are equal, both variables point to the same *rc* or *context* at the moment of comparison.
    /// Note that you can't store this or actually get unsafe access to the var internals, this is only for comparison.
    fn var_ptr(&self) -> VarPtr;

    /// Get the value as a debug [`Txt`].
    ///
    /// [`Txt`]: Txt
    fn get_debug(&self) -> Txt;

    /// Schedule a variable update, even if the value does no change.
    ///
    /// Usually variables only notify update if the value is changed to a different one, calling
    /// this method flags the variable to notify even if the value is equal.
    fn update(&self) -> Result<(), VarIsReadOnlyError>;

    /// Create a [`map`] that converts from `T` to a [`Txt`] debug print.
    ///
    /// [`map`]: Var::map
    /// [`Txt`]: Txt
    fn map_debug(&self) -> BoxedVar<Txt>;
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

/// Represents an [`AnyVar`] *pointer* that can be used for comparison.
///
/// If two of these values are equal, both variables point to the same *arc* or *context* at the moment of comparison.
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

    fn new_ctx_local<T: Send + Sync>(tl: &'static ContextLocal<T>) -> Self {
        Self {
            _lt: std::marker::PhantomData,
            eq: VarPtrData::Static((tl as *const ContextLocal<T>) as _),
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
        if f.alternate() {
            f.debug_tuple("VarPtr").field(&self.eq).finish()
        } else {
            fmt::Debug::fmt(&self.eq, f)
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
/// # use zero_ui_var::*;
/// #[derive(Debug, Clone, PartialEq)]
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
/// # macro_rules! _demo { () => {
/// #[property(SIZE)]
/// pub fn size(child: impl UiNode, size: impl IntoVar<Size>) -> impl UiNode {
///     // ...
///     # child
/// }
///
/// // shorthand #1:
/// let w = Wgt! {
///     size = (800, 600);
/// };
///
/// // shorthand #2:
/// let w = Wgt! {
///     size = (800.1, 600.2);
/// };
///
/// // full:
/// let w = Wgt! {
///     size = Size { width: 800.0, height: 600.0 };
/// };
/// # }}
/// ```
///
/// A property implemented using [`IntoVar`]:
///
/// ```
/// # macro_rules! _demo { () => {
/// #[property(LAYOUT)]
/// pub fn foo(child: impl UiNode, bar: impl IntoVar<u32>) -> impl UiNode {
///     let bar = bar.into_var();
///     match_node(child, move |_, op| match op {
///         UiNodeOp::Init => {
///             WIDGET.sub_var(&bar);
///             println!("init: {}", bar.get());
///         }
///         UiNodeOp::Update { .. } => {
///             if let Some(new) = bar.get_new() {
///                 println!("update: {new}");
///             }
///         }
///         _ => {}
///     })
/// }
///
/// // literal assign:
/// let wgt = Wgt! {
///     foo = 42;
/// };
///
/// // variable assign:
/// let variable = var(42);
/// let wgt = Wgt! {
///     foo = variable;
/// };
///
/// // widget when:
/// let wgt = Wgt! {
///     foo = 42;
///
///     when !*#is_enabled {
///         foo = 32;
///     }
/// };
/// # }}
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

    /// Converts into [`BoxedVar<T>`].
    ///
    /// This method exists to help the type system infer the type in this scenario:
    ///
    /// ```
    /// # use zero_ui_var::*;
    /// # let bar = true;
    /// # let BAR_VAR = var(true);
    /// #
    /// fn foo(foo: impl IntoVar<bool>) { }
    ///
    /// foo(if bar {
    ///     BAR_VAR.map(|b| !*b).boxed()
    /// } else {
    ///     true.into_boxed_var()
    /// });
    /// ```
    ///
    /// We need a `BoxedVar<bool>` to unify the input types that can be a `map` var or a `LocalVar<bool>`. Writing `true.into_var().boxed()`
    /// causes the type inference to fail, requiring us to write `IntoVar::<bool>::into_var(true).boxed()`.
    fn into_boxed_var(self) -> BoxedVar<T>
    where
        Self: Sized,
    {
        self.into_var().boxed()
    }
}

/// Represents the current value in a [`Var::modify`] handler.
pub struct VarModify<'a, T: VarValue> {
    current_value: &'a T,
    value: Cow<'a, T>,
    update: bool,
    tags: Vec<Box<dyn AnyVarValue>>,
}
impl<'a, T: VarValue> VarModify<'a, T> {
    /// Replace the value.
    ///
    /// The variable will update if the new value is not equal to the previous after all modify closures apply.
    pub fn set(&mut self, new_value: T) {
        self.value = Cow::Owned(new_value);
    }

    /// Notify an update, even if the value does not actually change.
    pub fn update(&mut self) {
        self.update = true;
    }

    /// Returns a mutable reference for modification.
    ///
    /// Note that this clones the current value if this is the first modify closure requesting it.
    ///
    /// The variable will update if the new value is not equal to the previous after all modify closures apply.
    pub fn to_mut(&mut self) -> &mut T {
        self.value.to_mut()
    }

    /// Custom tags that will be shared with the var hooks if the value updates.
    ///
    /// The tags where set by previous modify closures or this one during this update cycle, so
    /// tags can also be used to communicate between modify closures.
    pub fn tags(&self) -> &[Box<dyn AnyVarValue>] {
        &self.tags
    }

    /// Add a custom tag object that will be shared with the var hooks if the value updates.
    pub fn push_tag(&mut self, tag: impl AnyVarValue) {
        self.tags.push(Box::new(tag));
    }

    /// Add all custom tags.
    pub fn push_tags(&mut self, tags: Vec<Box<dyn AnyVarValue>>) {
        if self.tags.is_empty() {
            self.tags = tags;
        } else {
            self.tags.extend(tags);
        }
    }

    /// New from current value.
    pub fn new(current_value: &'a T) -> Self {
        Self {
            current_value,
            value: Cow::Borrowed(current_value),
            update: false,
            tags: vec![],
        }
    }

    /// Returns `(notify, new_value, update, tags)`.
    pub fn finish(self) -> (bool, Option<T>, bool, Vec<Box<dyn AnyVarValue>>) {
        match self.value {
            Cow::Borrowed(_) => {
                if self.update {
                    return (true, None, true, self.tags);
                }
            }
            Cow::Owned(v) => {
                if self.update || self.current_value != &v {
                    return (true, Some(v), self.update, self.tags);
                }
            }
        }
        (false, None, false, vec![])
    }
}
impl<'a, T: VarValue> ops::Deref for VarModify<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}
impl<'a, T: VarValue> std::convert::AsRef<T> for VarModify<'a, T> {
    fn as_ref(&self) -> &T {
        &self.value
    }
}

/// Arguments for [`AnyVar::hook_any`].
pub struct AnyVarHookArgs<'a> {
    value: &'a dyn AnyVarValue,
    update: bool,
    tags: &'a [Box<dyn AnyVarValue>],
}
impl<'a> AnyVarHookArgs<'a> {
    /// New from updated value and custom tag.
    pub fn new(value: &'a dyn AnyVarValue, update: bool, tags: &'a [Box<dyn AnyVarValue>]) -> Self {
        Self { value, update, tags }
    }

    /// Reference the updated value.
    pub fn value(&self) -> &'a dyn AnyVarValue {
        self.value
    }

    /// If update was explicitly requested.
    ///
    /// Note that bindings/mappings propagate this update request.
    pub fn update(&self) -> bool {
        self.update
    }

    /// Value type ID.
    pub fn value_type(&self) -> TypeId {
        self.value.as_any().type_id()
    }

    /// Custom tag objects.
    pub fn tags(&self) -> &[Box<dyn AnyVarValue>] {
        self.tags
    }

    /// Clone the custom tag objects set by the code that updated the value.
    pub fn tags_vec(&self) -> Vec<Box<dyn AnyVarValue>> {
        self.tags.iter().map(|t| (*t).clone_boxed()).collect()
    }

    /// Reference the value, if it is of type `T`.
    pub fn downcast_value<T: VarValue>(&self) -> Option<&T> {
        self.value.as_any().downcast_ref()
    }

    /// Reference all custom tag values of type `T`.
    pub fn downcast_tags<T: VarValue>(&self) -> impl Iterator<Item = &T> + '_ {
        self.tags.iter().filter_map(|t| (*t).as_any().downcast_ref::<T>())
    }

    /// Try cast to strongly typed args.
    pub fn as_strong<T: VarValue>(&self) -> Option<VarHookArgs<T>> {
        if TypeId::of::<T>() == self.value_type() {
            Some(VarHookArgs {
                any: self,
                _t: PhantomData,
            })
        } else {
            None
        }
    }
}

/// Arguments for [`Var::hook`].
pub struct VarHookArgs<'a, T: VarValue> {
    any: &'a AnyVarHookArgs<'a>,
    _t: PhantomData<&'a T>,
}
impl<'a, T: VarValue> VarHookArgs<'a, T> {
    /// Reference the updated value.
    pub fn value(&self) -> &'a T {
        self.any.value.as_any().downcast_ref::<T>().unwrap()
    }
}
impl<'a, T: VarValue> ops::Deref for VarHookArgs<'a, T> {
    type Target = AnyVarHookArgs<'a>;

    fn deref(&self) -> &Self::Target {
        self.any
    }
}

/// Args for [`Var::trace_value`].
pub struct TraceValueArgs<'a, T: VarValue> {
    args: &'a AnyVarHookArgs<'a>,
    _type: PhantomData<&'a T>,
}
impl<'a, T: VarValue> ops::Deref for TraceValueArgs<'a, T> {
    type Target = AnyVarHookArgs<'a>;

    fn deref(&self) -> &Self::Target {
        self.args
    }
}
impl<'a, T: VarValue> TraceValueArgs<'a, T> {
    /// Strongly-typed reference to the new value.
    pub fn value(&self) -> &'a T {
        self.args.downcast_value::<T>().unwrap()
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
    /// This is `Self` for vars that are always read-only, or [`types::ReadOnlyVar<T, Self>`] for others.
    type ReadOnly: Var<T>;

    /// Output of [`Var::actual_var`].
    ///
    /// This is [`BoxedVar<T>`] for [`ContextVar<T>`], `V` for [`types::ArcFlatMapVar<T, V>`] and `Self` for others.
    type ActualVar: Var<T>;

    /// Output of [`Var::downgrade`].
    type Downgrade: WeakVar<T>;

    /// Output of [`Var::map`].
    type Map<O: VarValue>: Var<O>;

    /// Output of [`Var::map_bidi`].
    type MapBidi<O: VarValue>: Var<O>;

    /// Output of [`Var::map_bidi`].
    type FlatMap<O: VarValue, V: Var<O>>: Var<O>;

    /// Output of [`Var::filter_map`].
    type FilterMap<O: VarValue>: Var<O>;

    /// Output of [`Var::filter_map_bidi`].
    type FilterMapBidi<O: VarValue>: Var<O>;

    /// Output of [`Var::map_ref`].
    type MapRef<O: VarValue>: Var<O>;

    /// Output of [`Var::map_ref_bidi`].
    type MapRefBidi<O: VarValue>: Var<O>;

    /// Output of [`Var::easing`].
    type Easing: Var<T>;

    /// Visit the current value of the variable.
    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&T) -> R;

    /// Schedule a variable update, it will be applied on the end of the current app update.
    ///
    /// The variable only updates if the [`VarModify`] explicitly requests update, or set/modified.
    fn modify<F>(&self, modify: F) -> Result<(), VarIsReadOnlyError>
    where
        F: FnOnce(&mut VarModify<T>) + Send + 'static;

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
    /// and [`types::ArcFlatMapVar<T, V>`].
    fn actual_var(self) -> Self::ActualVar;

    /// Create a weak reference to this *Arc* variable.
    ///
    /// The weak reference is made to the [`actual_var`], if the actual var is a [`LocalVar<T>`]
    /// a clone of it is returned, for *Arc* vars an actual weak reference is made.
    ///
    /// [`actual_var`]: Var::actual_var
    fn downgrade(&self) -> Self::Downgrade;

    /// Convert this variable to the value, if possible moves the value, if it is shared clones it.
    fn into_value(self) -> T;

    /// Gets a clone of the var that is always read-only.
    ///
    /// The returned variable can still update if `self` is modified, but it does not have the `MODIFY` capability.
    fn read_only(&self) -> Self::ReadOnly;

    /// Setups a callback for just after the variable value update is applied, the closure runs in the root app context, just like
    /// the `modify` closure. The closure can returns if it is retained after each call.
    ///
    /// Variables store a weak[^1] reference to the callback if they have the `MODIFY` or `CAPS_CHANGE` capabilities, otherwise
    /// the callback is discarded and [`VarHandle::dummy`] returned.
    ///
    /// [^1]: You can use the [`VarHandle::perm`] to make the stored reference *strong*.
    fn hook(&self, pos_modify_action: impl Fn(&VarHookArgs<T>) -> bool + Send + Sync + 'static) -> VarHandle {
        // !!: TODO, rename after refactor, so we don't miss any .hook.
        self.hook_any(Box::new(move |a| pos_modify_action(&a.as_strong().unwrap())))
    }

    /// Create a future that awaits for the [`last_update`] to change.
    ///
    /// The future can be reused. Note that [`is_new`] will be `true` when the future elapses only when polled
    /// in sync with the UI, but it will elapse in any thread when the variable updates after the future is instantiated.
    ///
    /// Note that outside of the UI tree there is no variable synchronization across multiple var method calls, so
    /// a sequence of `get(); wait_is_new().await; get();` can miss a value between `get` and `wait_is_new`.
    ///
    /// [`get`]: Var::get
    /// [`last_update`]: AnyVar::last_update
    /// [`is_new`]: AnyVar::is_new
    fn wait_update(&self) -> types::WaitUpdateFut<Self> {
        types::WaitUpdateFut::new(self)
    }

    /// Create a future that awaits for [`is_animating`] to change from `true` to `false`.
    ///
    /// The future can only be used in app bound async code, it can be reused. If the variable
    /// is not animating at the moment of this call the future will await until the animation starts and stops.
    ///
    /// If the variable does have the [`VarCapabilities::NEW`] the returned future is always ready.
    ///
    /// [`is_animating`]: AnyVar::is_animating
    fn wait_animation(&self) -> types::WaitIsNotAnimatingFut<Self> {
        types::WaitIsNotAnimatingFut::new(self)
    }

    /// Visit the current value of the variable, if it [`is_new`].
    ///
    /// [`is_new`]: AnyVar::is_new
    fn with_new<R, F>(&self, read: F) -> Option<R>
    where
        F: FnOnce(&T) -> R,
    {
        if self.is_new() {
            Some(self.with(read))
        } else {
            None
        }
    }

    /// Get a clone of the current value.
    fn get(&self) -> T {
        self.with(Clone::clone)
    }

    /// Gets the value as a display [`Txt`].
    ///
    /// [`Txt`]: Txt
    fn get_text(&self) -> Txt
    where
        T: fmt::Display,
    {
        self.with(ToText::to_text)
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
    fn get_ne(&self, value: &mut T) -> bool {
        self.with(var_get_ne(value))
    }

    /// Get a clone of the current value, if it [`is_new`].
    ///
    /// [`is_new`]: AnyVar::is_new
    fn get_new(&self) -> Option<T> {
        if self.is_new() {
            Some(self.with(Clone::clone))
        } else {
            None
        }
    }

    /// Get a clone of the current value into `value` if the current value [`is_new`].
    ///
    /// [`is_new`]: AnyVar::is_new
    fn get_new_into(&self, value: &mut T) -> bool {
        let is_new = self.is_new();
        if is_new {
            self.with(var_get_into(value));
        }
        is_new
    }

    /// Get a clone of the current value into `value` if the variable value [`is_new`] and not equal to the `value`.
    ///
    /// [`is_new`]: AnyVar::is_new
    fn get_new_ne(&self, value: &mut T) -> bool {
        self.is_new() && self.get_ne(value)
    }

    /// Schedule a new `value` for the variable, it will be set in the end of the current app update.
    fn set<I>(&self, value: I) -> Result<(), VarIsReadOnlyError>
    where
        I: Into<T>,
    {
        self.modify(var_set(value.into()))
    }

    /// Schedule a new `value` for the variable, it will be set in the end of the current app update to the value
    /// of `other` at the time it is set. If `other` already has a scheduled that new value will be set, this is
    /// particularly important when starting a new binding.
    ///  
    /// # Examples
    ///
    /// Starting a *set-bind* with `set` can cause unexpected initial value:
    ///
    /// ```
    /// # use zero_ui_var::*;
    /// # mod task { pub async fn yield_one() { } }
    /// # async {
    /// let a = var(0);
    /// let b = var(0);
    ///
    /// a.set(1);
    /// b.set(a.get());
    /// a.bind(&b).perm();
    ///
    /// task::yield_one().await;
    /// // scheduled updates apply in this order:
    /// //  - a.set(1);
    /// //    - b.set(1); // caused by the binding.
    /// //  - b.set(0); // caused by the request `b.set(a.get())`.
    ///
    /// assert_eq!(1, a.get());
    /// assert_eq!(0, b.get()); // not 1!
    /// # };
    /// ```
    ///
    /// Starting the binding with `set_from`:
    ///
    /// ```
    /// # use zero_ui_var::*;
    /// # mod task { pub async fn yield_one() { } }
    /// # async {
    /// let a = var(0);
    /// let b = var(0);
    ///
    /// a.set(1);
    /// b.set_from(&a);
    /// a.bind(&b).perm();
    ///
    /// task::yield_one().await;
    /// // scheduled updates apply in this order:
    /// //  - a.set(1);
    /// //    - b.set(1); // caused by the binding.
    /// //  - b.set(1); // caused by the request `b.set_from(&a)`.
    ///
    /// assert_eq!(1, a.get());
    /// assert_eq!(1, b.get());
    /// # };
    /// ```
    fn set_from<I>(&self, other: &I) -> Result<(), VarIsReadOnlyError>
    where
        I: Var<T>,
    {
        if other.capabilities().is_always_static() {
            self.set(other.get())
        } else {
            self.modify(var_set_from(other.clone().actual_var()))
        }
    }

    /// Set from `other` value at the time of update, mapped to the type of `self`.
    fn set_from_map<Iv, I, M>(&self, other: &I, map: M) -> Result<(), VarIsReadOnlyError>
    where
        Iv: VarValue,
        I: Var<Iv>,
        M: FnOnce(&Iv) -> T + Send + 'static,
    {
        if other.capabilities().is_always_static() {
            self.set(other.with(map))
        } else {
            self.modify(var_set_from_map(other.clone().actual_var(), map))
        }
    }

    /// Create a ref-counted var that redirects to this variable until the first value update, then it behaves like a [`ArcVar<T>`].
    ///
    /// The return variable is *clone-on-write* and has the `MODIFY` capability independent of the source capabilities, when
    /// a modify request is made the source value is cloned and offered for modification, if modified the source variable is dropped
    /// and the cow var behaves like a [`ArcVar<T>`], if the modify closure does not update the cloned value it is dropped and the cow
    /// continues to redirect to the source variable.
    fn cow(&self) -> types::ArcCowVar<T, Self> {
        types::ArcCowVar::new(self.clone())
    }

    /// Creates a ref-counted var that maps from this variable.
    ///
    /// The `map` closure is called once on initialization, and then once every time
    /// the source variable updates.
    ///
    /// The mapping variable is read-only, you can use [`map_bidi`] to map back.
    ///
    /// Note that the mapping var is [contextualized] for context vars, meaning the map binding will initialize in
    /// the fist usage context, not the creation context, so `property = CONTEXT_VAR.map(|&b|!b);` will bind with
    /// the `CONTEXT_VAR` in the `property` context, not the property instantiation. The `map` closure itself runs in
    /// the root app context, trying to read other context variables inside it will only read the default value.
    ///
    /// For other variables types the `map` can run once in the caller context.
    ///
    /// [`map_bidi`]: Var::map_bidi
    /// [contextualized]: types::ContextualizedVar
    fn map<O, M>(&self, map: M) -> Self::Map<O>
    where
        O: VarValue,
        M: FnMut(&T) -> O + Send + 'static;

    /// Creates a [`map`] that converts from `T` to `O` using [`Into<O>`].
    ///
    /// [`map`]: Var::map
    fn map_into<O>(&self) -> Self::Map<O>
    where
        O: VarValue,
        T: Into<O>,
    {
        self.map(|v| v.clone().into())
    }

    /// Creates a [`map`] that converts from `T` to [`Txt`] using [`ToText`].
    ///
    /// [`map`]: Var::map
    /// [`Txt`]: Txt
    /// [`ToText`]: ToText
    fn map_to_text(&self) -> Self::Map<Txt>
    where
        T: ToText,
    {
        self.map(ToText::to_text)
    }

    /// Create a [`map`] that converts from `T` to [`String`] using [`ToString`].
    ///
    /// [`map`]: Var::map
    fn map_to_string(&self) -> Self::Map<String>
    where
        T: ToString,
    {
        self.map(ToString::to_string)
    }

    /// Create a ref-counted var that maps from this variable on read and to it on write.
    ///
    /// The `map` closure is called once on initialization, and then once every time
    /// the source variable updates, the `map_back` closure is called every time the output value is modified directly.
    ///
    /// The mapping var is [contextualized], see [`Var::map`] for more details.
    ///
    /// [contextualized]: types::ContextualizedVar
    fn map_bidi<O, M, B>(&self, map: M, map_back: B) -> Self::MapBidi<O>
    where
        O: VarValue,
        M: FnMut(&T) -> O + Send + 'static,
        B: FnMut(&O) -> T + Send + 'static;

    /// Create a ref-counted var that maps to an inner variable that is found inside the value of this variable.
    ///
    /// The mapping var is [contextualized] if self is contextual, otherwise `map` evaluates immediately to start. Note
    /// that the "mapped-to" var can be contextual even when the mapping var is not.
    ///
    /// The mapping var has the same capabilities of the inner var + `CAPS_CHANGE`, modifying the mapping var modifies the inner var.
    ///
    /// [contextualized]: types::ContextualizedVar
    fn flat_map<O, V, M>(&self, map: M) -> Self::FlatMap<O, V>
    where
        O: VarValue,
        V: Var<O>,
        M: FnMut(&T) -> V + Send + 'static;

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
    fn filter_map<O, M, I>(&self, map: M, fallback: I) -> Self::FilterMap<O>
    where
        O: VarValue,
        M: FnMut(&T) -> Option<O> + Send + 'static,
        I: Fn() -> O + Send + Sync + 'static;

    /// Create a [`filter_map`] that tries to convert from `T` to `O` using [`TryInto<O>`].
    ///
    /// [`filter_map`]: Var::filter_map
    fn filter_try_into<O, I>(&self, fallback: I) -> Self::FilterMap<O>
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
    fn filter_parse<O, I>(&self, fallback: I) -> Self::FilterMap<O>
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
    /// is called every time the output value is modified directly, if it returns `Some(_)` the source variable is set.
    ///
    /// The mapping var is [contextualized], see [`Var::map`] for more details.
    ///
    /// [contextualized]: types::ContextualizedVar
    fn filter_map_bidi<O, M, B, I>(&self, map: M, map_back: B, fallback: I) -> Self::FilterMapBidi<O>
    where
        O: VarValue,
        M: FnMut(&T) -> Option<O> + Send + 'static,
        B: FnMut(&O) -> Option<T> + Send + 'static,
        I: Fn() -> O + Send + Sync + 'static;

    /// Create a mapping wrapper around `self`. The `map` closure is called for each value access, it must reference the
    /// value `O` that already exists in `T`.
    fn map_ref<O, M>(&self, map: M) -> Self::MapRef<O>
    where
        O: VarValue,
        M: Fn(&T) -> &O + Send + Sync + 'static;

    /// Create a mapping wrapper around `self`. The `map` closure is called for each value access, it must reference the
    /// value `O` that already exists in `T`, the `map_mut` closure is called for every modify request, it must do the same
    /// as `map` but with mutable access.
    fn map_ref_bidi<O, M, B>(&self, map: M, map_mut: B) -> Self::MapRefBidi<O>
    where
        O: VarValue,
        M: Fn(&T) -> &O + Send + Sync + 'static,
        B: Fn(&mut T) -> &mut O + Send + Sync + 'static;
    /// Setup a hook that assigns `other` with the new values of `self` transformed by `map`.
    ///
    /// Only a weak reference to the `other` variable is held, both variables update in the same app update cycle.
    ///
    /// Note that the current value is not assigned, only the subsequent updates, you can use [`set_from_map`]
    /// to sync the initial value.
    ///
    /// [`set_from_map`]: Self::set_from_map
    fn bind_map<T2, V2, M>(&self, other: &V2, map: M) -> VarHandle
    where
        T2: VarValue,
        V2: Var<T2>,
        M: FnMut(&T) -> T2 + Send + 'static,
    {
        var_bind_map(self, other, map)
    }

    /// Setup a hook that assigns `other` with the new values of `self` transformed by `map`, if the closure returns a value.
    ///
    /// Only a weak reference to the `other` variable is held, both variables update in the same app update cycle.
    ///
    /// Note that the current value is not assigned, only the subsequent updates, you can assign
    /// `other` and then bind to fully sync the variables.
    fn bind_filter_map<T2, V2, F>(&self, other: &V2, map: F) -> VarHandle
    where
        T2: VarValue,
        V2: Var<T2>,
        F: FnMut(&T) -> Option<T2> + Send + 'static,
    {
        var_bind_filter_map(self, other, map)
    }

    /// Bind `self` to `other` and back without causing an infinite loop.
    ///
    /// Only a weak reference to each variable is held by the other, if both variables are scheduled to update in the same cycle
    /// both get assigned, but only one bind transfer per app cycle is allowed for each variable. Returns two handles on the
    /// the *map* hook and one for the *map-back* hook.
    ///
    /// Note that the current value is not assigned, only the subsequent updates, you can assign
    /// `other` and `self` and then bind to fully sync the variables.
    fn bind_map_bidi<T2, V2, M, B>(&self, other: &V2, map: M, map_back: B) -> VarHandles
    where
        T2: VarValue,
        V2: Var<T2>,
        M: FnMut(&T) -> T2 + Send + 'static,
        B: FnMut(&T2) -> T + Send + 'static,
    {
        var_bind_map_bidi(self, other, map, map_back)
    }

    /// Bind `self` to `other` and back with the new values of `self` transformed by `map` and the new values of `other` transformed
    /// by `map_back`, the value is assigned in a update only if the closures returns a value.
    ///
    /// Only a weak reference to each variable is held by the other, both variables update in the same app update cycle.
    ///
    /// Note that the current value is not assigned, only the subsequent updates, you can assign
    /// `other` and then bind to fully sync the variables.
    fn bind_filter_map_bidi<T2, V2, M, B>(&self, other: &V2, map: M, map_back: B) -> VarHandles
    where
        T2: VarValue,
        V2: Var<T2>,
        M: FnMut(&T) -> Option<T2> + Send + 'static,
        B: FnMut(&T2) -> Option<T> + Send + 'static,
    {
        var_bind_filter_map_bidi(self, other, map, map_back)
    }

    /// Setup a hook that assigns `other` with the new values of `self`.
    ///
    /// Only a weak reference to the `other` variable is held.
    ///
    /// Note that the current value is not assigned, only the subsequent updates, you can use
    /// [`set_from`] to sync the initial value.
    ///
    /// [`set_from`]: Self::set_from
    fn bind<V2>(&self, other: &V2) -> VarHandle
    where
        V2: Var<T>,
    {
        self.bind_map(other, Clone::clone)
    }

    /// Setup two hooks that assigns `other` with the new values of `self` and `self` with the new values of `other`.
    ///
    /// Only a  weak reference to each variable is held by the other.
    ///
    /// Note that the current value is not assigned, only the subsequent updates, you can assign
    /// `other` and then bind to fully sync the variables.
    fn bind_bidi<V2>(&self, other: &V2) -> VarHandles
    where
        V2: Var<T>,
    {
        self.bind_map_bidi(other, Clone::clone, Clone::clone)
    }

    /// Debug helper for tracing the lifetime of a value in this variable.
    ///
    /// The `enter_value` closure is called every time the variable updates, it can return
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
    /// # use zero_ui_var::*;
    /// # struct Fake; impl Fake { pub fn entered(self) { } }
    /// # #[macro_export]
    /// # macro_rules! info_span { ($($tt:tt)*) => { Fake }; }
    /// # mod tracing {  pub use crate::info_span; }
    /// # fn trace_var<T: VarValue>(var: &impl Var<T>) {
    /// var.trace_value(|a| {
    ///     tracing::info_span!("my_var", ?a.value(), track = "<vars>").entered()
    /// }).perm();
    /// # }
    /// ```
    ///
    /// Note that you don't need to use any external tracing crate, this method also works with the standard printing:
    ///
    /// ```
    /// # use zero_ui_var::*;
    /// # fn trace_var(var: &impl Var<u32>) {
    /// var.trace_value(|a| println!("value: {:?}", a.value())).perm();
    /// # }
    /// ```
    ///
    /// [`tracing`]: https://docs.rs/tracing/
    /// [`actual_var`]: Var::actual_var
    fn trace_value<E, S>(&self, mut enter_value: E) -> VarHandle
    where
        E: FnMut(&TraceValueArgs<T>) -> S + Send + 'static,
        S: Send + 'static,
    {
        let span = self.with(|v| {
            enter_value(&TraceValueArgs {
                args: &AnyVarHookArgs::new(v, false, &[]),
                _type: PhantomData,
            })
        });
        let data = Mutex::new((Some(span), enter_value));
        self.hook_any(Box::new(move |args| {
            let mut data = data.lock();
            let (span, enter_value) = &mut *data;
            let _ = span.take();
            *span = Some(enter_value(&TraceValueArgs { args, _type: PhantomData }));
            true
        }))
    }

    /// Schedule an animation that targets this variable.
    ///
    /// If the variable is always read-only no animation is created and a dummy handle returned. The animation
    /// targets the current [`actual_var`] and is stopped if the variable is dropped.
    ///
    /// The `animate` closure is called every frame, starting after next frame, the closure inputs are
    /// the [`Animation`] args and *modify* access to the variable value, the args
    /// can be used to calculate the new variable value and to control or stop the animation.
    ///
    /// [`actual_var`]: Var::actual_var
    /// [`Animation`]: animation::Animation
    fn animate<A>(&self, animate: A) -> animation::AnimationHandle
    where
        A: FnMut(&animation::Animation, &mut VarModify<T>) + Send + 'static,
    {
        animation::var_animate(self, animate)
    }

    /// Schedule animations started by `animate`, the closure is called once at the start to begin, then again every time
    /// the variable stops animating.
    ///
    /// This can be used to create a sequence of animations or to repeat an animation. The sequence stops when `animate` returns
    /// a dummy handle or the variable is modified outside of `animate`, or animations are disabled, or the returned handle is dropped.
    fn sequence<A>(&self, animate: A) -> VarHandle
    where
        A: FnMut(&<<Self::ActualVar as Var<T>>::Downgrade as WeakVar<T>>::Upgrade) -> animation::AnimationHandle + Send + 'static,
    {
        animation::var_sequence(self, animate)
    }

    /// Schedule an easing transition from the `start_value` to `end_value`.
    ///
    /// The variable updates every time the [`EasingStep`] for each frame changes and a different value is sampled.
    ///
    /// See [`Var::animate`] for details about animations.
    fn set_ease<S, E, F>(&self, start_value: S, end_value: E, duration: Duration, easing: F) -> animation::AnimationHandle
    where
        T: Transitionable,
        S: Into<T>,
        E: Into<T>,
        F: Fn(EasingTime) -> EasingStep + Send + 'static,
    {
        self.set_ease_with(start_value, end_value, duration, easing, animation::Transition::sample)
    }

    /// Schedule an easing transition from the `start_value` to `end_value` using a custom value sampler.
    ///
    /// The variable updates every time the [`EasingStep`] for each frame changes and a different value is sampled.
    ///
    /// See [`Var::animate`] for details about animations.
    fn set_ease_with<S, E, F, Sa>(
        &self,
        start_value: S,
        end_value: E,
        duration: Duration,
        easing: F,
        sampler: Sa,
    ) -> animation::AnimationHandle
    where
        T: Transitionable,
        S: Into<T>,
        E: Into<T>,
        F: Fn(EasingTime) -> EasingStep + Send + 'static,
        Sa: Fn(&animation::Transition<T>, EasingStep) -> T + Send + 'static,
    {
        self.animate(animation::var_set_ease_with(
            start_value.into(),
            end_value.into(),
            duration,
            easing,
            999.fct(),
            sampler,
        ))
    }

    /// Schedule an easing transition from the current value to `new_value`.
    ///
    /// The variable updates every time the [`EasingStep`] for each frame changes and a different value is sampled.
    ///
    /// See [`Var::animate`] for details about animations.
    fn ease<E, F>(&self, new_value: E, duration: Duration, easing: F) -> animation::AnimationHandle
    where
        T: Transitionable,
        E: Into<T>,
        F: Fn(EasingTime) -> EasingStep + Send + 'static,
    {
        self.ease_with(new_value, duration, easing, animation::Transition::sample)
    }

    /// Schedule an easing transition from the current value to `new_value` using a custom value sampler.
    ///
    /// The variable updates every time the [`EasingStep`] for each frame changes and a different value is sampled.
    ///
    /// See [`Var::animate`] for details about animations.
    fn ease_with<E, F, S>(&self, new_value: E, duration: Duration, easing: F, sampler: S) -> animation::AnimationHandle
    where
        T: Transitionable,
        E: Into<T>,
        F: Fn(EasingTime) -> EasingStep + Send + 'static,
        S: Fn(&animation::Transition<T>, EasingStep) -> T + Send + 'static,
    {
        self.animate(animation::var_set_ease_with(
            self.get(),
            new_value.into(),
            duration,
            easing,
            0.fct(),
            sampler,
        ))
    }

    /// Schedule a keyframed transition animation for the variable, starting from the first key.
    ///
    /// The variable will be set to to the first keyframe, then animated across all other keys.
    ///
    /// See [`Var::animate`] for details about animations.
    fn set_ease_keyed<F>(&self, keys: Vec<(Factor, T)>, duration: Duration, easing: F) -> animation::AnimationHandle
    where
        T: Transitionable,
        F: Fn(EasingTime) -> EasingStep + Send + 'static,
    {
        self.set_ease_keyed_with(keys, duration, easing, animation::TransitionKeyed::sample)
    }

    /// Schedule a keyframed transition animation for the variable, starting from the first key, using a custom value sampler.
    ///
    /// The variable will be set to to the first keyframe, then animated across all other keys.
    ///
    /// See [`Var::animate`] for details about animations.
    fn set_ease_keyed_with<F, S>(&self, keys: Vec<(Factor, T)>, duration: Duration, easing: F, sampler: S) -> animation::AnimationHandle
    where
        T: Transitionable,
        F: Fn(EasingTime) -> EasingStep + Send + 'static,
        S: Fn(&animation::TransitionKeyed<T>, EasingStep) -> T + Send + 'static,
    {
        if let Some(transition) = animation::TransitionKeyed::new(keys) {
            self.animate(animation::var_set_ease_keyed_with(transition, duration, easing, 999.fct(), sampler))
        } else {
            animation::AnimationHandle::dummy()
        }
    }

    /// Schedule a keyframed transition animation for the variable, starting from the current value.
    ///
    /// The variable will be set to to the first keyframe, then animated across all other keys.
    ///
    /// See [`Var::animate`] for details about animations.
    fn ease_keyed<F>(&self, keys: Vec<(Factor, T)>, duration: Duration, easing: F) -> animation::AnimationHandle
    where
        T: Transitionable,
        F: Fn(EasingTime) -> EasingStep + Send + 'static,
    {
        self.ease_keyed_with(keys, duration, easing, animation::TransitionKeyed::sample)
    }

    /// Schedule a keyframed transition animation for the variable, starting from the current value, using a custom value sampler.
    ///
    /// The variable will be set to to the first keyframe, then animated across all other keys.
    ///
    /// See [`Var::animate`] for details about animations.
    fn ease_keyed_with<F, S>(&self, mut keys: Vec<(Factor, T)>, duration: Duration, easing: F, sampler: S) -> animation::AnimationHandle
    where
        T: Transitionable,
        F: Fn(EasingTime) -> EasingStep + Send + 'static,
        S: Fn(&animation::TransitionKeyed<T>, EasingStep) -> T + Send + 'static,
    {
        keys.insert(0, (0.fct(), self.get()));

        let transition = animation::TransitionKeyed::new(keys).unwrap();
        self.animate(animation::var_set_ease_keyed_with(transition, duration, easing, 0.fct(), sampler))
    }

    /// Set the variable to `new_value` after a `delay`.
    ///
    /// The variable [`is_animating`] until the delay elapses and the value is set.
    ///
    /// See [`Var::animate`] for details about animations.
    ///
    /// [`is_animating`]: AnyVar::is_animating
    fn step<N>(&self, new_value: N, delay: Duration) -> animation::AnimationHandle
    where
        N: Into<T>,
    {
        self.animate(animation::var_step(new_value.into(), delay))
    }

    /// Oscillate between the current value and `new_value`, every time the `delay` elapses the variable is set to the next value.
    ///
    /// The variable will be set a maximum of `count` times.
    fn step_oci<N>(&self, new_value: N, delay: Duration, count: usize) -> animation::AnimationHandle
    where
        N: Into<T>,
    {
        self.animate(animation::var_step_oci([self.get(), new_value.into()], delay, count))
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
    /// # use zero_ui_var::{*, animation::easing};
    /// # use zero_ui_txt::*;
    /// # use zero_ui_unit::*;
    /// # fn demo(text_var: impl Var<Txt>) {
    /// let steps = (0..=100).step_by(5).map(|i| (i.pct().fct(), formatx!("{i}%"))).collect();
    /// # let _ =
    /// text_var.steps(steps, 5.secs(), easing::linear)
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
    fn steps<F>(&self, steps: Vec<(Factor, T)>, duration: Duration, easing: F) -> animation::AnimationHandle
    where
        F: Fn(EasingTime) -> EasingStep + Send + 'static,
    {
        self.animate(animation::var_steps(steps, duration, easing))
    }

    /// Starts an easing animation that *chases* a target value that can be changed using the [`ChaseAnimation<T>`] handle.
    ///
    /// [`ChaseAnimation<T>`]: animation::ChaseAnimation
    fn chase<N, F>(&self, first_target: N, duration: Duration, easing: F) -> animation::ChaseAnimation<T>
    where
        N: Into<T>,
        F: Fn(EasingTime) -> EasingStep + Send + 'static,
        T: Transitionable,
    {
        animation::var_chase(self.clone().boxed(), first_target.into(), duration, easing)
    }

    /// Create a vars that [`ease`] to each new value of `self`.
    ///
    /// Note that the mapping var is [contextualized], meaning the binding will initialize in the fist usage context, not
    /// the creation context, so `property = CONTEXT_VAR.easing(500.ms(), easing::linear);` will bind with the `CONTEXT_VAR`
    /// in the `property` context, not the property instantiation.
    ///
    /// [contextualized]: types::ContextualizedVar
    /// [`ease`]: Var::ease
    fn easing<F>(&self, duration: Duration, easing: F) -> Self::Easing
    where
        T: Transitionable,
        F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static;

    /// Create a vars that [`ease_with`] to each new value of `self`.
    ///
    /// Note that the mapping var is [contextualized], meaning the binding will initialize in the fist usage context, not
    /// the creation context, so `property = CONTEXT_VAR.easing(500.ms(), easing::linear);` will bind with the `CONTEXT_VAR`
    /// in the `property` context, not the property instantiation.
    ///
    /// [contextualized]: types::ContextualizedVar
    /// [`ease_with`]: Var::ease_with
    fn easing_with<F, S>(&self, duration: Duration, easing: F, sampler: S) -> Self::Easing
    where
        T: Transitionable,
        F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
        S: Fn(&animation::Transition<T>, EasingStep) -> T + Send + Sync + 'static;

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
}

fn var_bind_map<T, T2, V2, M>(source: &impl Var<T>, other: &V2, map: M) -> VarHandle
where
    T: VarValue,
    T2: VarValue,
    V2: Var<T2>,
    M: FnMut(&T) -> T2 + Send + 'static,
{
    #[cfg(dyn_closure)]
    let map: Box<dyn FnMut(&T) -> T2 + Send> = Box::new(map);
    var_bind_map_impl(source, other, map)
}
fn var_bind_map_impl<T, T2, V2, M>(source: &impl Var<T>, other: &V2, mut map: M) -> VarHandle
where
    T: VarValue,
    T2: VarValue,
    V2: Var<T2>,
    M: FnMut(&T) -> T2 + Send + 'static,
{
    var_bind(source, other, move |value, args, other| {
        let value = map(value);
        let update = args.update;
        let _ = other.modify(move |vm| {
            vm.set(value);
            if update {
                vm.update();
            }
        });
    })
}

fn var_bind_filter_map<T, T2, V2, F>(source: &impl Var<T>, other: &V2, map: F) -> VarHandle
where
    T: VarValue,
    T2: VarValue,
    V2: Var<T2>,
    F: FnMut(&T) -> Option<T2> + Send + 'static,
{
    #[cfg(dyn_closure)]
    let map: Box<dyn FnMut(&T) -> Option<T2> + Send> = Box::new(map);
    var_bind_filter_map_impl(source, other, map)
}
fn var_bind_filter_map_impl<T, T2, V2, F>(source: &impl Var<T>, other: &V2, mut map: F) -> VarHandle
where
    T: VarValue,
    T2: VarValue,
    V2: Var<T2>,
    F: FnMut(&T) -> Option<T2> + Send + 'static,
{
    var_bind(source, other, move |value, args, other| {
        if let Some(value) = map(value) {
            let update = args.update;
            let _ = other.modify(move |vm| {
                vm.set(value);
                if update {
                    vm.update();
                }
            });
        }
    })
}

fn var_bind_map_bidi<T, T2, V2, M, B>(source: &impl Var<T>, other: &V2, map: M, map_back: B) -> VarHandles
where
    T: VarValue,
    T2: VarValue,
    V2: Var<T2>,
    M: FnMut(&T) -> T2 + Send + 'static,
    B: FnMut(&T2) -> T + Send + 'static,
{
    #[cfg(dyn_closure)]
    let map: Box<dyn FnMut(&T) -> T2 + Send + 'static> = Box::new(map);
    #[cfg(dyn_closure)]
    let map_back: Box<dyn FnMut(&T2) -> T + Send + 'static> = Box::new(map_back);

    var_bind_map_bidi_impl(source, other, map, map_back)
}

fn var_bind_map_bidi_impl<T, T2, V2, M, B>(source: &impl Var<T>, other: &V2, mut map: M, mut map_back: B) -> VarHandles
where
    T: VarValue,
    T2: VarValue,
    V2: Var<T2>,
    M: FnMut(&T) -> T2 + Send + 'static,
    B: FnMut(&T2) -> T + Send + 'static,
{
    let binding_tag = BindMapBidiTag::new_unique();

    let self_to_other = var_bind(source, other, move |value, args, other| {
        let is_from_other = args.downcast_tags::<BindMapBidiTag>().any(|&b| b == binding_tag);
        if !is_from_other {
            let value = map(value);
            let update = args.update;
            let _ = other.modify(move |vm| {
                vm.set(value);
                vm.push_tag(binding_tag);
                if update {
                    vm.update();
                }
            });
        }
    });

    let other_to_self = var_bind(other, source, move |value, args, self_| {
        let is_from_self = args.downcast_tags::<BindMapBidiTag>().any(|&b| b == binding_tag);
        if !is_from_self {
            let value = map_back(value);
            let update = args.update;
            let _ = self_.modify(move |vm| {
                vm.set(value);
                vm.push_tag(binding_tag);
                if update {
                    vm.update();
                }
            });
        }
    });

    [self_to_other, other_to_self].into_iter().collect()
}

fn var_bind_filter_map_bidi<T, T2, V2, M, B>(source: &impl Var<T>, other: &V2, map: M, map_back: B) -> VarHandles
where
    T: VarValue,
    T2: VarValue,
    V2: Var<T2>,
    M: FnMut(&T) -> Option<T2> + Send + 'static,
    B: FnMut(&T2) -> Option<T> + Send + 'static,
{
    #[cfg(dyn_closure)]
    let map: Box<dyn FnMut(&T) -> Option<T2> + Send + 'static> = Box::new(map);
    #[cfg(dyn_closure)]
    let map_back: Box<dyn FnMut(&T2) -> Option<T> + Send + 'static> = Box::new(map_back);

    var_bind_filter_map_bidi_impl(source, other, map, map_back)
}

fn var_bind_filter_map_bidi_impl<T, T2, V2, M, B>(source: &impl Var<T>, other: &V2, mut map: M, mut map_back: B) -> VarHandles
where
    T: VarValue,
    T2: VarValue,
    V2: Var<T2>,
    M: FnMut(&T) -> Option<T2> + Send + 'static,
    B: FnMut(&T2) -> Option<T> + Send + 'static,
{
    let binding_tag = BindMapBidiTag::new_unique();

    let self_to_other = var_bind(source, other, move |value, args, other| {
        let is_from_other = args.downcast_tags::<BindMapBidiTag>().any(|&b| b == binding_tag);
        if !is_from_other {
            if let Some(value) = map(value) {
                let update = args.update;
                let _ = other.modify(move |vm| {
                    vm.set(value);
                    vm.push_tag(binding_tag);
                    if update {
                        vm.update();
                    }
                });
            }
        }
    });

    let other_to_self = var_bind(other, source, move |value, args, self_| {
        let is_from_self = args.downcast_tags::<BindMapBidiTag>().any(|&b| b == binding_tag);
        if !is_from_self {
            if let Some(value) = map_back(value) {
                let update = args.update;
                let _ = self_.modify(move |vm| {
                    vm.set(value);
                    vm.push_tag(binding_tag);
                    if update {
                        vm.update();
                    }
                });
            }
        }
    });

    [self_to_other, other_to_self].into_iter().collect()
}

fn var_map<T: VarValue, O: VarValue>(source: &impl Var<T>, map: impl FnMut(&T) -> O + Send + 'static) -> ReadOnlyArcVar<O> {
    #[cfg(dyn_closure)]
    let map: Box<dyn FnMut(&T) -> O + Send> = Box::new(map);
    var_map_impl(source, map)
}
fn var_map_impl<T: VarValue, O: VarValue>(source: &impl Var<T>, mut map: impl FnMut(&T) -> O + Send + 'static) -> ReadOnlyArcVar<O> {
    let mapped = var(source.with(&mut map));
    var_bind_map_impl(source, &mapped, map).perm();
    mapped.read_only()
}
fn var_map_ctx<T: VarValue, O: VarValue>(
    source: &impl Var<T>,
    map: impl FnMut(&T) -> O + Send + 'static,
) -> contextualized::ContextualizedVar<O> {
    #[cfg(dyn_closure)]
    let map: Box<dyn FnMut(&T) -> O + Send> = Box::new(map);
    var_map_ctx_impl(source, map)
}
fn var_map_ctx_impl<T: VarValue, O: VarValue>(
    source: &impl Var<T>,
    map: impl FnMut(&T) -> O + Send + 'static,
) -> contextualized::ContextualizedVar<O> {
    let source = source.clone();
    let map = Arc::new(Mutex::new(map));
    types::ContextualizedVar::new(move || {
        let other = var(source.with(&mut *map.lock()));
        let map = map.clone();
        source.bind_map(&other, move |t| map.lock()(t)).perm();
        other.read_only()
    })
}
fn var_map_mixed<T: VarValue, O: VarValue>(source: &impl Var<T>, map: impl FnMut(&T) -> O + Send + 'static) -> BoxedVar<O> {
    #[cfg(dyn_closure)]
    let map: Box<dyn FnMut(&T) -> O + Send> = Box::new(map);

    if source.is_contextual() {
        var_map_ctx_impl(source, map).boxed()
    } else if source.capabilities().is_always_static() {
        LocalVar(source.with(map)).boxed()
    } else {
        var_map_impl(source, map).boxed()
    }
}

fn var_map_bidi<T, O, M, B>(source: &impl Var<T>, map: M, map_back: B) -> ArcVar<O>
where
    T: VarValue,
    O: VarValue,
    M: FnMut(&T) -> O + Send + 'static,
    B: FnMut(&O) -> T + Send + 'static,
{
    #[cfg(dyn_closure)]
    let map: Box<dyn FnMut(&T) -> O + Send> = Box::new(map);
    #[cfg(dyn_closure)]
    let map_back: Box<dyn FnMut(&O) -> T + Send> = Box::new(map_back);

    var_map_bidi_impl(source, map, map_back)
}
fn var_map_bidi_impl<T, O, M, B>(source: &impl Var<T>, mut map: M, map_back: B) -> ArcVar<O>
where
    T: VarValue,
    O: VarValue,
    M: FnMut(&T) -> O + Send + 'static,
    B: FnMut(&O) -> T + Send + 'static,
{
    let mapped = var(source.with(&mut map));
    var_bind_map_bidi_impl(source, &mapped, map, map_back).perm();
    mapped
}
fn var_map_bidi_ctx<T, O, M, B>(source: &impl Var<T>, map: M, map_back: B) -> types::ContextualizedVar<O>
where
    T: VarValue,
    O: VarValue,
    M: FnMut(&T) -> O + Send + 'static,
    B: FnMut(&O) -> T + Send + 'static,
{
    #[cfg(dyn_closure)]
    let map: Box<dyn FnMut(&T) -> O + Send> = Box::new(map);
    #[cfg(dyn_closure)]
    let map_back: Box<dyn FnMut(&O) -> T + Send> = Box::new(map_back);

    var_map_bidi_ctx_impl(source, map, map_back)
}
fn var_map_bidi_ctx_impl<T, O, M, B>(source: &impl Var<T>, map: M, map_back: B) -> types::ContextualizedVar<O>
where
    T: VarValue,
    O: VarValue,
    M: FnMut(&T) -> O + Send + 'static,
    B: FnMut(&O) -> T + Send + 'static,
{
    let me = source.clone();
    let map = Arc::new(Mutex::new(map));
    let map_back = Arc::new(Mutex::new(map_back));
    types::ContextualizedVar::new(move || {
        let other = var(me.with(&mut *map.lock()));
        let map = map.clone();
        let map_back = map_back.clone();
        me.bind_map_bidi(&other, move |i| map.lock()(i), move |o| map_back.lock()(o)).perm();
        other
    })
}
fn var_map_bidi_mixed<T, O, M, B>(source: &impl Var<T>, map: M, map_back: B) -> BoxedVar<O>
where
    T: VarValue,
    O: VarValue,
    M: FnMut(&T) -> O + Send + 'static,
    B: FnMut(&O) -> T + Send + 'static,
{
    #[cfg(dyn_closure)]
    let map: Box<dyn FnMut(&T) -> O + Send> = Box::new(map);
    #[cfg(dyn_closure)]
    let map_back: Box<dyn FnMut(&O) -> T + Send> = Box::new(map_back);

    if source.is_contextual() {
        var_map_bidi_ctx_impl(source, map, map_back).boxed()
    } else if source.capabilities().is_always_static() {
        LocalVar(source.with(map)).boxed()
    } else {
        var_map_bidi_impl(source, map, map_back).boxed()
    }
}

fn var_flat_map<T, O, V, M>(source: &impl Var<T>, map: M) -> types::ArcFlatMapVar<O, V>
where
    T: VarValue,
    O: VarValue,
    V: Var<O>,
    M: FnMut(&T) -> V + Send + 'static,
{
    #[cfg(dyn_closure)]
    let map: Box<dyn FnMut(&T) -> V + Send + 'static> = Box::new(map);

    var_flat_map_impl(source, map)
}
fn var_flat_map_impl<T, O, V, M>(source: &impl Var<T>, map: M) -> types::ArcFlatMapVar<O, V>
where
    T: VarValue,
    O: VarValue,
    V: Var<O>,
    M: FnMut(&T) -> V + Send + 'static,
{
    types::ArcFlatMapVar::new(source, map)
}
fn var_flat_map_ctx<T, O, V, M>(source: &impl Var<T>, map: M) -> types::ContextualizedVar<O>
where
    T: VarValue,
    O: VarValue,
    V: Var<O>,
    M: FnMut(&T) -> V + Send + 'static,
{
    #[cfg(dyn_closure)]
    let map: Box<dyn FnMut(&T) -> V + Send + 'static> = Box::new(map);

    var_flat_map_ctx_impl(source, map)
}
fn var_flat_map_ctx_impl<T, O, V, M>(source: &impl Var<T>, map: M) -> types::ContextualizedVar<O>
where
    T: VarValue,
    O: VarValue,
    V: Var<O>,
    M: FnMut(&T) -> V + Send + 'static,
{
    let me = source.clone();
    let map = Arc::new(Mutex::new(map));
    types::ContextualizedVar::new(move || {
        let map = map.clone();
        types::ArcFlatMapVar::new(&me, move |i| map.lock()(i))
    })
}
fn var_flat_map_mixed<T, O, V, M>(source: &impl Var<T>, map: M) -> BoxedVar<O>
where
    T: VarValue,
    O: VarValue,
    V: Var<O>,
    M: FnMut(&T) -> V + Send + 'static,
{
    #[cfg(dyn_closure)]
    let map: Box<dyn FnMut(&T) -> V + Send + 'static> = Box::new(map);

    if source.is_contextual() {
        var_flat_map_ctx_impl(source, map).boxed()
    } else if source.capabilities().is_always_static() {
        source.with(map).boxed()
    } else {
        var_flat_map_impl(source, map).boxed()
    }
}

fn var_filter_map<T, O, M, I>(source: &impl Var<T>, map: M, fallback: I) -> ReadOnlyArcVar<O>
where
    T: VarValue,
    O: VarValue,
    M: FnMut(&T) -> Option<O> + Send + 'static,
    I: Fn() -> O + Send + Sync + 'static,
{
    #[cfg(dyn_closure)]
    let map: Box<dyn FnMut(&T) -> Option<O> + Send + 'static> = Box::new(map);
    #[cfg(dyn_closure)]
    let fallback: Box<dyn Fn() -> O + Send + Sync + 'static> = Box::new(fallback);

    var_filter_map_impl(source, map, fallback)
}
fn var_filter_map_impl<T, O, M, I>(source: &impl Var<T>, mut map: M, fallback: I) -> ReadOnlyArcVar<O>
where
    T: VarValue,
    O: VarValue,
    M: FnMut(&T) -> Option<O> + Send + 'static,
    I: Fn() -> O + Send + Sync + 'static,
{
    let other = var(source.with(&mut map).unwrap_or_else(&fallback));
    source.bind_filter_map(&other, map).perm();
    other.read_only()
}
fn var_filter_map_ctx<T, O, M, I>(source: &impl Var<T>, map: M, fallback: I) -> types::ContextualizedVar<O>
where
    T: VarValue,
    O: VarValue,
    M: FnMut(&T) -> Option<O> + Send + 'static,
    I: Fn() -> O + Send + Sync + 'static,
{
    #[cfg(dyn_closure)]
    let map: Box<dyn FnMut(&T) -> Option<O> + Send + 'static> = Box::new(map);
    #[cfg(dyn_closure)]
    let fallback: Box<dyn Fn() -> O + Send + Sync + 'static> = Box::new(fallback);

    var_filter_map_ctx_impl(source, map, fallback)
}
fn var_filter_map_ctx_impl<T, O, M, I>(source: &impl Var<T>, map: M, fallback: I) -> types::ContextualizedVar<O>
where
    T: VarValue,
    O: VarValue,
    M: FnMut(&T) -> Option<O> + Send + 'static,
    I: Fn() -> O + Send + Sync + 'static,
{
    let me = source.clone();
    let map = Arc::new(Mutex::new(map));
    types::ContextualizedVar::new(move || {
        let other = var(me.with(&mut *map.lock()).unwrap_or_else(&fallback));
        let map = map.clone();
        me.bind_filter_map(&other, move |i| map.lock()(i)).perm();
        other.read_only()
    })
}
fn var_filter_map_mixed<T, O, M, I>(source: &impl Var<T>, map: M, fallback: I) -> BoxedVar<O>
where
    T: VarValue,
    O: VarValue,
    M: FnMut(&T) -> Option<O> + Send + 'static,
    I: Fn() -> O + Send + Sync + 'static,
{
    #[cfg(dyn_closure)]
    let map: Box<dyn FnMut(&T) -> Option<O> + Send + 'static> = Box::new(map);
    #[cfg(dyn_closure)]
    let fallback: Box<dyn Fn() -> O + Send + Sync + 'static> = Box::new(fallback);

    if source.is_contextual() {
        var_filter_map_ctx_impl(source, map, fallback).boxed()
    } else if source.capabilities().is_always_static() {
        LocalVar(source.with(map).unwrap_or_else(fallback)).boxed()
    } else {
        var_filter_map_impl(source, map, fallback).boxed()
    }
}

fn var_filter_map_bidi<T, O, M, B, I>(source: &impl Var<T>, map: M, map_back: B, fallback: I) -> ArcVar<O>
where
    T: VarValue,
    O: VarValue,
    M: FnMut(&T) -> Option<O> + Send + 'static,
    B: FnMut(&O) -> Option<T> + Send + 'static,
    I: Fn() -> O + Send + Sync + 'static,
{
    #[cfg(dyn_closure)]
    let map: Box<dyn FnMut(&T) -> Option<O> + Send + 'static> = Box::new(map);
    #[cfg(dyn_closure)]
    let map_back: Box<dyn FnMut(&O) -> Option<T> + Send + 'static> = Box::new(map_back);
    #[cfg(dyn_closure)]
    let fallback: Box<dyn Fn() -> O + Send + Sync + 'static> = Box::new(fallback);

    var_filter_map_bidi_impl(source, map, map_back, fallback)
}
fn var_filter_map_bidi_impl<T, O, M, B, I>(source: &impl Var<T>, mut map: M, map_back: B, fallback: I) -> ArcVar<O>
where
    T: VarValue,
    O: VarValue,
    M: FnMut(&T) -> Option<O> + Send + 'static,
    B: FnMut(&O) -> Option<T> + Send + 'static,
    I: Fn() -> O + Send + Sync + 'static,
{
    let other = var(source.with(&mut map).unwrap_or_else(&fallback));
    source.bind_filter_map_bidi(&other, map, map_back).perm();
    other
}
fn var_filter_map_bidi_ctx<T, O, M, B, I>(source: &impl Var<T>, map: M, map_back: B, fallback: I) -> types::ContextualizedVar<O>
where
    T: VarValue,
    O: VarValue,
    M: FnMut(&T) -> Option<O> + Send + 'static,
    B: FnMut(&O) -> Option<T> + Send + 'static,
    I: Fn() -> O + Send + Sync + 'static,
{
    #[cfg(dyn_closure)]
    let map: Box<dyn FnMut(&T) -> Option<O> + Send + 'static> = Box::new(map);
    #[cfg(dyn_closure)]
    let map_back: Box<dyn FnMut(&O) -> Option<T> + Send + 'static> = Box::new(map_back);
    #[cfg(dyn_closure)]
    let fallback: Box<dyn Fn() -> O + Send + Sync + 'static> = Box::new(fallback);

    var_filter_map_bidi_ctx_impl(source, map, map_back, fallback)
}
fn var_filter_map_bidi_ctx_impl<T, O, M, B, I>(source: &impl Var<T>, map: M, map_back: B, fallback: I) -> types::ContextualizedVar<O>
where
    T: VarValue,
    O: VarValue,
    M: FnMut(&T) -> Option<O> + Send + 'static,
    B: FnMut(&O) -> Option<T> + Send + 'static,
    I: Fn() -> O + Send + Sync + 'static,
{
    let me = source.clone();
    let map = Arc::new(Mutex::new(map));
    let map_back = Arc::new(Mutex::new(map_back));
    types::ContextualizedVar::new(move || {
        let other = var(me.with(&mut *map.lock()).unwrap_or_else(&fallback));
        let map = map.clone();
        let map_back = map_back.clone();
        me.bind_filter_map_bidi(&other, move |i| map.lock()(i), move |o| map_back.lock()(o))
            .perm();
        other
    })
}
fn var_filter_map_bidi_mixed<T, O, M, B, I>(source: &impl Var<T>, map: M, map_back: B, fallback: I) -> BoxedVar<O>
where
    T: VarValue,
    O: VarValue,
    M: FnMut(&T) -> Option<O> + Send + 'static,
    B: FnMut(&O) -> Option<T> + Send + 'static,
    I: Fn() -> O + Send + Sync + 'static,
{
    #[cfg(dyn_closure)]
    let map: Box<dyn FnMut(&T) -> Option<O> + Send + 'static> = Box::new(map);
    #[cfg(dyn_closure)]
    let map_back: Box<dyn FnMut(&O) -> Option<T> + Send + 'static> = Box::new(map_back);
    #[cfg(dyn_closure)]
    let fallback: Box<dyn Fn() -> O + Send + Sync + 'static> = Box::new(fallback);

    if source.is_contextual() {
        var_filter_map_bidi_ctx_impl(source, map, map_back, fallback).boxed()
    } else if source.capabilities().is_always_static() {
        LocalVar(source.with(map).unwrap_or_else(fallback)).boxed()
    } else {
        var_filter_map_bidi_impl(source, map, map_back, fallback).boxed()
    }
}

fn var_map_ref<T, S, O, M>(source: &S, map: M) -> types::MapRef<T, O, S>
where
    T: VarValue,
    S: Var<T>,
    O: VarValue,
    M: Fn(&T) -> &O + Send + Sync + 'static,
{
    types::MapRef::new(source.clone(), Arc::new(map))
}

fn var_map_ref_bidi<T, S, O, M, B>(source: &S, map: M, map_mut: B) -> types::MapRefBidi<T, O, S>
where
    T: VarValue,
    S: Var<T>,
    O: VarValue,
    M: Fn(&T) -> &O + Send + Sync + 'static,
    B: Fn(&mut T) -> &mut O + Send + Sync + 'static,
{
    types::MapRefBidi::new(source.clone(), Arc::new(map), Arc::new(map_mut))
}

fn var_easing<T, F>(source: &impl Var<T>, duration: Duration, easing: F) -> ReadOnlyArcVar<T>
where
    T: VarValue + Transitionable,
    F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
{
    let easing_fn = Arc::new(easing);
    let easing_var = var(source.get());
    let mut _anim_handle = animation::AnimationHandle::dummy();
    var_bind(source, &easing_var, move |value, args, easing_var| {
        _anim_handle = easing_var.ease(value.clone(), duration, clmv!(easing_fn, |t| easing_fn(t)));
        if args.update {
            easing_var.update();
        }
    })
    .perm();
    easing_var.read_only()
}
fn var_easing_ctx<T, F>(source: &impl Var<T>, duration: Duration, easing: F) -> types::ContextualizedVar<T>
where
    T: VarValue + Transitionable,
    F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
{
    let source = source.clone();
    let easing_fn = Arc::new(easing);
    types::ContextualizedVar::new(move || {
        let easing_var = var(source.get());

        let easing_fn = easing_fn.clone();
        let mut _anim_handle = animation::AnimationHandle::dummy();
        var_bind(&source, &easing_var, move |value, args, easing_var| {
            let easing_fn = easing_fn.clone();
            _anim_handle = easing_var.ease(value.clone(), duration, move |t| easing_fn(t));
            if args.update {
                easing_var.update();
            }
        })
        .perm();
        easing_var.read_only()
    })
}
fn var_easing_mixed<T, F>(source: &impl Var<T>, duration: Duration, easing: F) -> BoxedVar<T>
where
    T: VarValue + Transitionable,
    F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
{
    if source.is_contextual() {
        var_easing_ctx(source, duration, easing).boxed()
    } else if source.capabilities().is_always_static() {
        source.clone().boxed()
    } else {
        var_easing(source, duration, easing).boxed()
    }
}

fn var_easing_with<T, F, S>(source: &impl Var<T>, duration: Duration, easing: F, sampler: S) -> ReadOnlyArcVar<T>
where
    T: VarValue + Transitionable,
    F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
    S: Fn(&animation::Transition<T>, EasingStep) -> T + Send + Sync + 'static,
{
    let fns = Arc::new((easing, sampler));
    let easing_var = var(source.get());

    let mut _anim_handle = animation::AnimationHandle::dummy();
    var_bind(source, &easing_var, move |value, args, easing_var| {
        _anim_handle = easing_var.ease_with(
            value.clone(),
            duration,
            clmv!(fns, |t| (fns.0)(t)),
            clmv!(fns, |t, s| (fns.1)(t, s)),
        );
        if args.update {
            easing_var.update();
        }
    })
    .perm();
    easing_var.read_only()
}
fn var_easing_with_ctx<T, F, S>(source: &impl Var<T>, duration: Duration, easing: F, sampler: S) -> types::ContextualizedVar<T>
where
    T: VarValue + Transitionable,
    F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
    S: Fn(&animation::Transition<T>, EasingStep) -> T + Send + Sync + 'static,
{
    let source = source.clone();
    let fns = Arc::new((easing, sampler));
    types::ContextualizedVar::new(move || {
        let easing_var = var(source.get());

        let fns = fns.clone();
        let mut _anim_handle = animation::AnimationHandle::dummy();
        var_bind(&source, &easing_var, move |value, args, easing_var| {
            _anim_handle = easing_var.ease_with(
                value.clone(),
                duration,
                clmv!(fns, |t| (fns.0)(t)),
                clmv!(fns, |t, s| (fns.1)(t, s)),
            );
            if args.update {
                easing_var.update();
            }
        })
        .perm();
        easing_var.read_only()
    })
}
fn var_easing_with_mixed<T, F, S>(source: &impl Var<T>, duration: Duration, easing: F, sampler: S) -> BoxedVar<T>
where
    T: VarValue + Transitionable,
    F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
    S: Fn(&animation::Transition<T>, EasingStep) -> T + Send + Sync + 'static,
{
    if source.is_contextual() {
        var_easing_with_ctx(source, duration, easing, sampler).boxed()
    } else if source.capabilities().is_always_static() {
        source.clone().boxed()
    } else {
        var_easing_with(source, duration, easing, sampler).boxed()
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
fn var_set<T>(value: T) -> impl FnOnce(&mut VarModify<T>)
where
    T: VarValue,
{
    move |var_value| {
        var_value.set(value);
    }
}
fn var_set_from<T, I>(other: I) -> impl FnOnce(&mut VarModify<T>)
where
    T: VarValue,
    I: Var<T>,
{
    move |var_value| {
        other.with(|other| {
            if var_value.as_ref() != other {
                var_value.set(other.clone());
            }
        })
    }
}

fn var_set_from_map<T, Iv, I, M>(other: I, map: M) -> impl FnOnce(&mut VarModify<T>)
where
    Iv: VarValue,
    I: Var<Iv>,
    M: FnOnce(&Iv) -> T + Send + 'static,
    T: VarValue,
{
    move |var_value| {
        let value = other.with(map);
        var_value.set(value);
    }
}

fn var_set_any<T>(value: Box<dyn AnyVarValue>) -> impl FnOnce(&mut VarModify<T>)
where
    T: VarValue,
{
    match value.into_any().downcast::<T>() {
        Ok(value) => var_set(*value),
        Err(_) => panic!("cannot `set_any`, incompatible type"),
    }
}

fn var_update<T>(var_value: &mut VarModify<T>)
where
    T: VarValue,
{
    var_value.update();
}

fn var_debug<T>(value: &T) -> Txt
where
    T: VarValue,
{
    formatx!("{value:?}")
}

fn var_bind<I, O, V>(
    input: &impl Var<I>,
    output: &V,
    update_output: impl FnMut(&I, &AnyVarHookArgs, <V::Downgrade as WeakVar<O>>::Upgrade) + Send + 'static,
) -> VarHandle
where
    I: VarValue,
    O: VarValue,
    V: Var<O>,
{
    if input.capabilities().is_always_static() || output.capabilities().is_always_read_only() {
        VarHandle::dummy()
    } else {
        var_bind_ok(input, output.downgrade(), update_output)
    }
}

fn var_bind_ok<I, O, W>(
    input: &impl Var<I>,
    wk_output: W,
    update_output: impl FnMut(&I, &AnyVarHookArgs, W::Upgrade) + Send + 'static,
) -> VarHandle
where
    I: VarValue,
    O: VarValue,
    W: WeakVar<O>,
{
    let update_output = Mutex::new(update_output);
    input.hook_any(Box::new(move |args| {
        if let Some(output) = wk_output.upgrade() {
            if output.capabilities().contains(VarCapabilities::MODIFY) {
                if let Some(value) = args.downcast_value::<I>() {
                    update_output.lock()(value, args, output);
                }
            }
            true
        } else {
            false
        }
    }))
}

unique_id_32! {
    /// Used to stop an extra "map_back" caused by "map" itself
    #[derive(Debug)]
    pub(crate)struct BindMapBidiTag;
}

macro_rules! impl_infallible_write {
    (for<$T:ident>) => {
        /// Infallible [`Var::modify`].
        pub fn modify(&self, modify: impl FnOnce(&mut $crate::VarModify<$T>) + Send + 'static) {
            Var::modify(self, modify).unwrap()
        }

        /// Infallible [`Var::set`].
        pub fn set(&self, value: impl Into<$T>) {
            Var::set(self, value).unwrap()
        }

        /// Infallible [`AnyVar::update`].
        pub fn update(&self) {
            AnyVar::update(self).unwrap()
        }

        /// Infallible [`Var::set_from`].
        pub fn set_from<I: Var<$T>>(&self, other: &I) {
            Var::set_from(self, other).unwrap()
        }

        /// Infallible [`Var::set_from_map`].
        pub fn set_from_map<Iv, I, M>(&self, other: &I, map: M)
        where
            Iv: VarValue,
            I: Var<Iv>,
            M: FnOnce(&Iv) -> $T + Send + 'static,
        {
            Var::set_from_map(self, other, map).unwrap()
        }
    };
}
use impl_infallible_write;
