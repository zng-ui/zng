use std::{
    any::{Any, TypeId},
    fmt,
    marker::PhantomData,
    ops,
    sync::{Arc, atomic::AtomicBool},
};

use crate::{
    AnyVarHookArgs, AnyVarValue, BoxAnyVarValue, VarInstanceTag, VarUpdateId, VarValue,
    animation::{AnimationStopFn, ModifyInfo},
};
use bitflags::bitflags;
use smallbox::{SmallBox, smallbox};

pub(crate) mod shared_var;
pub use shared_var::{any_var, any_var_derived, var, var_derived, var_getter, var_state};

pub(crate) mod const_var;
pub(crate) mod cow_var;
pub use const_var::IntoVar;
pub(crate) mod flat_map_var;
pub(crate) mod read_only_var;

pub(crate) mod contextual_var;
pub use contextual_var::{ContextInitHandle, WeakContextInitHandle, any_contextual_var, contextual_var};

pub(crate) mod context_var;
pub use context_var::{__context_var_local, ContextVar, context_var_init};

pub(crate) mod merge_var;
pub use merge_var::{
    __merge_var, MergeInput, MergeVarBuilder, VarMergeInputs, merge_var, merge_var_input, merge_var_output, merge_var_with,
};

pub(crate) mod response_var;
pub use response_var::{ResponderVar, Response, ResponseVar, response_done_var, response_var};

pub(crate) mod when_var;
pub use when_var::{__when_var, AnyWhenVarBuilder, WhenVarBuilder};

pub(crate) mod expr_var;
pub use expr_var::{__expr_var, expr_var_as, expr_var_into, expr_var_map};

pub(crate) enum DynAnyVar {
    Const(const_var::ConstVar),
    Shared(shared_var::SharedVar),
    Context(context_var::ContextVarImpl),
    Cow(cow_var::CowVar),
    Contextual(contextual_var::ContextualVar),
    FlatMap(flat_map_var::FlatMapVar),
    Merge(merge_var::MergeVar),
    When(when_var::WhenVar),
    ReadOnly(read_only_var::ReadOnlyVar),
}
macro_rules! dispatch {
    ($self:ident, $var:ident => $($tt:tt)+) => {
        match $self {
            DynAnyVar::Const($var) => $($tt)+,
            DynAnyVar::Shared($var) => $($tt)+,
            DynAnyVar::Context($var) => $($tt)+,
            DynAnyVar::Cow($var) => $($tt)+,
            DynAnyVar::Contextual($var) => $($tt)+,
            DynAnyVar::FlatMap($var) => $($tt)+,
            DynAnyVar::Merge($var) => $($tt)+,
            DynAnyVar::When($var) => $($tt)+,
            DynAnyVar::ReadOnly($var) => $($tt)+,
        }
    };
}
impl fmt::Debug for DynAnyVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        dispatch!(self, v => fmt::Debug::fmt(v, f))
    }
}

pub(crate) enum DynWeakAnyVar {
    Const(const_var::WeakConstVar),
    Shared(shared_var::WeakSharedVar),
    Context(context_var::ContextVarImpl),
    Cow(cow_var::WeakCowVar),
    Contextual(contextual_var::WeakContextualVar),
    FlatMap(flat_map_var::WeakFlatMapVar),
    Merge(merge_var::WeakMergeVar),
    When(when_var::WeakWhenVar),
    ReadOnly(read_only_var::WeakReadOnlyVar),
}
macro_rules! dispatch_weak {
    ($self:ident, $var:ident => $($tt:tt)+) => {
        match $self {
            DynWeakAnyVar::Const($var) => $($tt)+,
            DynWeakAnyVar::Shared($var) => $($tt)+,
            DynWeakAnyVar::Context($var) => $($tt)+,
            DynWeakAnyVar::Cow($var) => $($tt)+,
            DynWeakAnyVar::Contextual($var) => $($tt)+,
            DynWeakAnyVar::FlatMap($var) => $($tt)+,
            DynWeakAnyVar::Merge($var) => $($tt)+,
            DynWeakAnyVar::When($var) => $($tt)+,
            DynWeakAnyVar::ReadOnly($var) => $($tt)+,
        }
    };
}
impl fmt::Debug for DynWeakAnyVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        dispatch_weak!(self, v => fmt::Debug::fmt(v, f))
    }
}

macro_rules! declare {
    ($(
        $(#[$meta:meta])*
        fn $method:ident(&self $(, $arg:ident : $Input:ty)*) $(-> $Output:ty)?;
    )+) => {
        pub(crate) trait VarImpl: fmt::Debug + Any + Send + Sync {
            $(
                $(#[$meta])*
                fn $method(&self $(, $arg: $Input)*) $(-> $Output)?;
            )+
        }

        impl VarImpl for DynAnyVar {
            $(
                $(#[$meta])*
                fn $method(&self $(, $arg: $Input)*) $(-> $Output)? {
                    dispatch!(self, v => VarImpl::$method(v$(, $arg)*))
                }
            )+
        }
    };
}
declare! {
    fn clone_dyn(&self) -> DynAnyVar;
    fn value_type(&self) -> TypeId;
    #[cfg(feature = "type_names")]
    fn value_type_name(&self) -> &'static str;
    fn strong_count(&self) -> usize;
    fn var_eq(&self, other: &DynAnyVar) -> bool;
    fn var_instance_tag(&self) -> VarInstanceTag;
    fn downgrade(&self) -> DynWeakAnyVar;
    fn capabilities(&self) -> VarCapability;
    fn with(&self, visitor: &mut dyn FnMut(&dyn AnyVarValue));
    fn get(&self) -> BoxAnyVarValue;
    fn set(&self, new_value: BoxAnyVarValue) -> bool;
    fn update(&self) -> bool;
    fn modify(&self, modify: SmallBox<dyn FnMut(&mut AnyVarModify) + Send + 'static, smallbox::space::S4>) -> bool;
    fn hook(&self, on_new: SmallBox<dyn FnMut(&AnyVarHookArgs) -> bool + Send + 'static, smallbox::space::S4>) -> VarHandle;
    fn last_update(&self) -> VarUpdateId;
    fn modify_importance(&self) -> usize;
    fn is_animating(&self) -> bool;
    fn hook_animation_stop(&self, handler: AnimationStopFn) -> VarHandle;
    fn current_context(&self) -> DynAnyVar;
    fn modify_info(&self) -> ModifyInfo;
}

macro_rules! declare_weak {
        ($(
        fn $method:ident(&self $(, $arg:ident : $Input:ty)*) $(-> $Output:ty)?;
    )+) => {
        pub(crate) trait WeakVarImpl: fmt::Debug + Any + Send + Sync {
            $(
                fn $method(&self $(, $arg: $Input)*) $(-> $Output)?;
            )+
        }

        impl WeakVarImpl for DynWeakAnyVar {
            $(
                fn $method(&self $(, $arg: $Input)*) $(-> $Output)? {
                    dispatch_weak!(self, v => WeakVarImpl::$method(v$(, $arg)*))
                }
            )+
        }
    };
}
declare_weak! {
    fn clone_dyn(&self) -> DynWeakAnyVar;
    fn strong_count(&self) -> usize;
    fn upgrade(&self) -> Option<DynAnyVar>;
}

/// Error when an attempt to modify a variable without the [`MODIFY`] capability is made.
///
/// [`MODIFY`]: VarCapability::MODIFY
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct VarIsReadOnlyError {}
impl fmt::Display for VarIsReadOnlyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cannot modify read-only variable")
    }
}
impl std::error::Error for VarIsReadOnlyError {}

bitflags! {
    /// Kinds of interactions allowed by a [`Var<T>`] in the current update.
    ///
    /// You can get the current capabilities of a var by using the [`AnyVar::capabilities`] method.
    ///
    /// [`Var<T>`]: crate::Var
    /// [`AnyVar::capabilities`]: crate::AnyVar::capabilities
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct VarCapability: u8 {
        /// Variable value can change.
        ///
        /// If this is set the [`AnyVar::is_new`] can be `true` in some updates, a variable can `NEW`
        /// even if it cannot `MODIFY`, in this case the variable is a read-only wrapper on a read-write variable.
        ///
        /// [`AnyVar::is_new`]: crate::AnyVar::is_new
        const NEW = 0b0000_0010;

        /// Variable can be modified.
        ///
        /// If this is set [`Var::try_modify`] always returns `Ok`, if this is set `NEW` is also set.
        ///
        /// Note that modify requests from inside overridden animations can still be ignored, see [`AnyVar::modify_importance`].
        ///
        /// [`AnyVar::modify_importance`]: crate::AnyVar::modify_importance
        /// [`Var::try_modify`]: crate::Var::try_modify
        const MODIFY = 0b0000_0011;

        /// Var represents different inner variables depending on the context it is used.
        const CONTEXT = 0b1000_0000;

        /// Variable capabilities can change to sometimes have the `MODIFY` capability.
        const MODIFY_CHANGES = 0b0100_0000;
        /// Variable capabilities can change to sometimes have the `CONTEXT` capability.
        const CONTEXT_CHANGES = 0b0010_0000;

        /// Var is an *arc* reference to the value and variable state, cloning the variable only clones a
        /// reference to the variable, all references modify and notify the same state.
        const SHARE = 0b0001_0000;
    }
}
impl VarCapability {
    /// If cannot `NEW` and is not `MODIFY_CHANGES`.
    pub fn is_const(self) -> bool {
        !self.contains(Self::NEW) && !self.contains(Self::MODIFY_CHANGES)
    }

    /// If does not have `MODIFY` capability and is not `MODIFY_CHANGES`.
    pub fn is_always_read_only(&self) -> bool {
        !self.contains(Self::MODIFY) && !self.contains(Self::MODIFY_CHANGES)
    }

    /// If does not have `MODIFY` capability.
    pub fn is_read_only(self) -> bool {
        !self.can_modify()
    }

    /// Has the `MODIFY` capability.
    pub fn can_modify(self) -> bool {
        self.contains(Self::MODIFY)
    }

    /// Has the `CONTEXT` capability.
    pub fn is_contextual(self) -> bool {
        self.contains(Self::CONTEXT)
    }

    /// Has the `CONTEXT` capability and does not have `CONTEXT_CHANGES`.
    pub fn is_always_contextual(self) -> bool {
        self.contains(Self::CONTEXT) && !self.contains(Self::CONTEXT_CHANGES)
    }

    /// Has the `SHARE` capability.
    pub fn is_share(&self) -> bool {
        self.contains(Self::SHARE)
    }

    /// Does not have the `SHARE` capability.
    ///
    /// Cloning this variable clones the value.
    pub fn is_local(&self) -> bool {
        !self.is_share()
    }
}
impl VarCapability {
    pub(crate) fn as_always_read_only(self) -> Self {
        let mut out = self;

        // can be new, but not modify
        out.remove(Self::MODIFY & !Self::NEW);
        // never will allow modify
        out.remove(Self::MODIFY_CHANGES);

        out
    }
}

bitflags! {
    #[derive(Clone, Copy)]
    pub(crate) struct VarModifyUpdate: u8 {
        /// Value was deref_mut or update was called
        const UPDATE = 0b001;
        /// Method update was called
        const REQUESTED = 0b011;
        /// Value was deref_mut
        const TOUCHED = 0b101;
    }
}

/// Mutable reference to a variable value.
///
/// The variable will notify an update only on `deref_mut`.
pub struct AnyVarModify<'a> {
    pub(crate) value: &'a mut BoxAnyVarValue,
    pub(crate) update: VarModifyUpdate,
    pub(crate) tags: Vec<BoxAnyVarValue>,
    pub(crate) custom_importance: Option<usize>,
}
impl<'a> AnyVarModify<'a> {
    /// Replace the value if not equal.
    ///
    /// Note that you can also deref_mut to modify the value.
    pub fn set(&mut self, mut new_value: BoxAnyVarValue) -> bool {
        if **self.value != *new_value {
            if !self.value.try_swap(&mut *new_value) {
                #[cfg(feature = "type_names")]
                panic!(
                    "cannot AnyVarModify::set `{}` on variable of type `{}`",
                    new_value.type_name(),
                    self.value.type_name()
                );
                #[cfg(not(feature = "type_names"))]
                panic!("cannot modify set, type mismatch");
            }
            self.update |= VarModifyUpdate::TOUCHED;
            true
        } else {
            false
        }
    }

    /// Notify an update, even if the value does not actually change.
    pub fn update(&mut self) {
        self.update |= VarModifyUpdate::REQUESTED;
    }

    /// Custom tags that will be shared with the var hooks if the value updates.
    ///
    /// The tags where set by previous modify closures or this one during this update cycle, so
    /// tags can also be used to communicate between modify closures.
    pub fn tags(&self) -> &[BoxAnyVarValue] {
        &self.tags
    }

    /// Add a custom tag object that will be shared with the var hooks if the value updates.
    pub fn push_tag(&mut self, tag: impl AnyVarValue) {
        self.tags.push(BoxAnyVarValue::new(tag));
    }

    /// Sets a custom [`AnyVar::modify_importance`] value.
    ///
    /// Note that the modify info is already automatically set, using a custom value here
    /// can easily break all future modify requests for this variable. The importance is set even if the
    /// variable does not update (no actual value change or update request).
    ///
    /// [`AnyVar::modify_importance`]: crate::AnyVar::modify_importance
    pub fn set_modify_importance(&mut self, importance: usize) {
        self.custom_importance = Some(importance);
    }

    /// Strongly typed reference, if it is of the same type.
    pub fn downcast<'s, T: VarValue>(&'s mut self) -> Option<VarModify<'s, 'a, T>> {
        if self.value.is::<T>() {
            Some(VarModify {
                inner: self,
                _t: PhantomData,
            })
        } else {
            None
        }
    }

    /// Immutable reference to the value.
    ///
    /// Note that you can also simply deref to the value.
    pub fn value(&self) -> &dyn AnyVarValue {
        &**self
    }

    /// Mutable reference to the value.
    ///
    /// Getting a mutable reference to the value flags the variable to notify update.
    ///
    /// Note that you can also simply deref to the value.
    pub fn value_mut(&mut self) -> &mut dyn AnyVarValue {
        &mut **self
    }
}
impl<'a> ops::Deref for AnyVarModify<'a> {
    type Target = dyn AnyVarValue;

    fn deref(&self) -> &Self::Target {
        &**self.value
    }
}
impl<'a> ops::DerefMut for AnyVarModify<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.update |= VarModifyUpdate::TOUCHED;
        self.value.deref_mut()
    }
}

/// Mutable reference to a variable value.
///
/// The variable will notify an update only on `deref_mut`.
pub struct VarModify<'s, 'a, T: VarValue> {
    inner: &'s mut AnyVarModify<'a>,
    _t: PhantomData<fn() -> &'a T>,
}
impl<'s, 'a, T: VarValue> VarModify<'s, 'a, T> {
    /// Replace the value if not equal.
    ///
    /// Note that you can also deref_mut to modify the value.
    pub fn set(&mut self, new_value: impl Into<T>) -> bool {
        let new_value = new_value.into();
        if **self != new_value {
            **self = new_value;
            true
        } else {
            false
        }
    }

    /// Notify an update, even if the value does not actually change.
    pub fn update(&mut self) {
        self.inner.update();
    }

    /// Custom tags that will be shared with the var hooks if the value updates.
    ///
    /// The tags where set by previous modify closures or this one during this update cycle, so
    /// tags can also be used to communicate between modify closures.
    pub fn tags(&self) -> &[BoxAnyVarValue] {
        self.inner.tags()
    }

    /// Add a custom tag object that will be shared with the var hooks if the value updates.
    pub fn push_tag(&mut self, tag: impl AnyVarValue) {
        self.inner.push_tag(tag);
    }

    /// Sets a custom [`AnyVar::modify_importance`] value.
    ///
    /// Note that the modify info is already automatically set, using a custom value here
    /// can easily break all future modify requests for this variable. The importance is set even if the
    /// variable does not update (no actual value change or update request).
    ///
    /// [`AnyVar::modify_importance`]: crate::AnyVar::modify_importance
    pub fn set_modify_importance(&mut self, importance: usize) {
        self.inner.set_modify_importance(importance);
    }

    /// Type erased reference.
    pub fn as_any(&mut self) -> &mut AnyVarModify<'a> {
        self.inner
    }

    /// Immutable reference to the value.
    ///
    /// Note that you can also simply deref to the value.
    pub fn value(&self) -> &T {
        self
    }

    /// Mutable reference to the value.
    ///
    /// Getting a mutable reference to the value flags the variable to notify update.
    ///
    /// Note that you can also simply deref to the value.
    pub fn value_mut(&mut self) -> &mut T {
        self
    }
}
impl<'s, 'a, T: VarValue> ops::Deref for VarModify<'s, 'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.downcast_ref().unwrap()
    }
}
impl<'s, 'a, T: VarValue> ops::DerefMut for VarModify<'s, 'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.downcast_mut().unwrap()
    }
}

/// Handle to a variable or animation hook.
///
/// This can represent a widget subscriber, a var binding, var app handler or animation, dropping the handler stops
/// the behavior it represents.
/// 
/// Note that the hook closure is not dropped immediately when the handle is dropped, usually it will drop only the next
/// time it would have been called.
#[derive(Clone, Default)]
#[must_use = "var handle stops the behavior it represents on drop"]
pub struct VarHandle(Option<Arc<AtomicBool>>);
impl PartialEq for VarHandle {
    fn eq(&self, other: &Self) -> bool {
        if let Some(a) = &self.0
            && let Some(b) = &other.0
        {
            Arc::ptr_eq(a, b)
        } else {
            false
        }
    }
}
impl fmt::Debug for VarHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_dummy() {
            write!(f, "VarHandle(<dummy>)")
        } else {
            f.debug_tuple("VarHandle").finish_non_exhaustive()
        }
    }
}
impl VarHandle {
    /// Handle to no variable.
    pub const fn dummy() -> Self {
        VarHandle(None)
    }

    pub(crate) fn new() -> (VarHandlerOwner, Self) {
        let h = Arc::new(AtomicBool::new(false));
        (VarHandlerOwner(h.clone()), Self(Some(h)))
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
        if let Some(c) = &self.0 {
            c.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// Create a [`VarHandles`] collection with `self` and `other`.
    pub fn chain(self, other: Self) -> VarHandles {
        VarHandles(smallvec::smallvec![self, other])
    }
}
pub(crate) struct VarHandlerOwner(Arc<AtomicBool>);
impl fmt::Debug for VarHandlerOwner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Arc::strong_count(&self.0) - 1)?;
        if self.0.load(std::sync::atomic::Ordering::Relaxed) {
            write!(f, " perm")
        } else {
            Ok(())
        }
    }
}
impl VarHandlerOwner {
    pub fn is_alive(&self) -> bool {
        Arc::strong_count(&self.0) > 1 || self.0.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// Represents a collection of var handles.
#[must_use = "var handles stops the behavior they represents on drop"]
#[derive(Clone, Default)]
pub struct VarHandles(smallvec::SmallVec<[VarHandle; 2]>);
impl VarHandles {
    /// Empty collection.
    pub const fn dummy() -> Self {
        VarHandles(smallvec::SmallVec::new_const())
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

    type IntoIter = smallvec::IntoIter<[VarHandle; 2]>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
impl ops::Deref for VarHandles {
    type Target = smallvec::SmallVec<[VarHandle; 2]>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl ops::DerefMut for VarHandles {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl From<VarHandle> for VarHandles {
    fn from(value: VarHandle) -> Self {
        let mut r = VarHandles::dummy();
        r.push(value);
        r
    }
}

#[cfg(feature = "type_names")]
fn value_type_name(var: &dyn VarImpl) -> &'static str {
    var.value_type_name()
}
#[cfg(not(feature = "type_names"))]
#[inline(always)]
fn value_type_name(_: &dyn VarImpl) -> &'static str {
    ""
}
