use std::{
    any::{Any, TypeId},
    fmt,
    marker::PhantomData,
    ops,
    sync::{Arc, atomic::AtomicBool},
};

use crate::{BoxedVarValueAny, VarAnyHookArgs, VarInstanceTag, VarUpdateId, VarValue, VarValueAny, animation::AnimationStopFn};
use bitflags::bitflags;
use smallbox::{SmallBox, smallbox};

pub(crate) mod shared;
pub use shared::{var, var_any, var_getter, var_state};

pub(crate) mod clone_on_write;
pub(crate) mod local;
pub use local::IntoVar;
pub(crate) mod flat_map;
pub(crate) mod map_ref;
pub(crate) mod map_ref_bidi;
pub(crate) mod read_only;

pub(crate) mod contextual;
pub use contextual::{ContextInitHandle, WeakContextInitHandle, var_ctx, var_ctx_any};

pub(crate) mod context_var;
pub use context_var::{__context_var_local, ContextVar, context_var_init};

pub(crate) mod merge;
pub use merge::{__var_merge, MergeInput, VarMergeBuilder, VarMergeInputs, var_merge, var_merge_input, var_merge_output, var_merge_with};

pub(crate) mod response_var;
pub use response_var::{ResponderVar, Response, ResponseVar, response_done_var, response_var};

pub(crate) mod when;
pub use when::{__var_when, VarWhenAnyBuilder, VarWhenBuilder};

pub(crate) mod expr;
pub use expr::{__var_expr, var_expr_as, var_expr_into, var_expr_map};

pub(crate) trait VarImpl: Any + Send + Sync {
    fn clone_boxed(&self) -> SmallBox<dyn VarImpl, smallbox::space::S2>;
    fn value_type(&self) -> TypeId;
    #[cfg(feature = "value_type_name")]
    fn value_type_name(&self) -> &'static str;
    fn strong_count(&self) -> usize;
    fn var_eq(&self, other: &dyn Any) -> bool;
    fn var_instance_tag(&self) -> VarInstanceTag;
    fn downgrade(&self) -> SmallBox<dyn WeakVarImpl, smallbox::space::S2>;
    fn capabilities(&self) -> VarCapability;
    fn with(&self, visitor: &mut dyn FnMut(&dyn VarValueAny));
    fn get(&self) -> BoxedVarValueAny;
    fn set(&self, new_value: BoxedVarValueAny) -> bool;
    fn update(&self) -> bool;
    fn modify(&self, modify: SmallBox<dyn FnMut(&mut VarModifyAny) + Send + 'static, smallbox::space::S4>) -> bool;
    fn hook(&self, on_new: SmallBox<dyn FnMut(&VarAnyHookArgs) -> bool + Send + 'static, smallbox::space::S4>) -> VarHandle;
    fn last_update(&self) -> VarUpdateId;
    fn modify_importance(&self) -> usize;
    fn is_animating(&self) -> bool;
    fn hook_animation_stop(&self, handler: AnimationStopFn) -> Result<(), AnimationStopFn>;
    fn current_context(&self) -> SmallBox<dyn VarImpl, smallbox::space::S2>;
}

pub(crate) trait WeakVarImpl: Any + Send + Sync {
    fn clone_boxed(&self) -> SmallBox<dyn WeakVarImpl, smallbox::space::S2>;
    fn strong_count(&self) -> usize;
    fn upgrade(&self) -> Option<SmallBox<dyn VarImpl, smallbox::space::S2>>;
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
    /// You can get the current capabilities of a var by using the [`VarAny::capabilities`] method.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct VarCapability: u8 {
        /// Variable value can change.
        ///
        /// If this is set the [`AnyVar::is_new`] can be `true` in some updates, a variable can `NEW`
        /// even if it cannot `MODIFY`, in this case the variable is a read-only wrapper on a read-write variable.
        const NEW = 0b0000_0010;

        /// Variable can be modified.
        ///
        /// If this is set [`Var::modify`] always returns `Ok`, if this is set `NEW` is also set.
        ///
        /// Note that modify requests from inside overridden animations can still be ignored, see [`VarAny::modify_importance`].
        const MODIFY = 0b0000_0011;

        /// Var capabilities can change.
        ///
        /// Var capabilities can only change in between app updates, just like the var value, but [`VarAny::last_update`]
        /// may not change when capability changes.
        const CAPS_CHANGE = 0b1000_0000;

        /// Var represents different inner variables depending on the context it is used.
        ///
        /// Any other capabilities set are from the inner variable.
        const CONTEXT = 0b1100_0000;

        /// Var is an *arc* reference to the value and variable state, cloning the variable only clones a
        /// reference to the variable, all references modify and notify the same state.
        const SHARE = 0b0010_0000;
    }
}
impl VarCapability {
    /// If cannot `NEW` and is not `CAPS_CHANGE`.
    pub fn is_always_static(self) -> bool {
        !self.contains(Self::NEW) && !self.contains(Self::CAPS_CHANGE)
    }

    /// If does not have `MODIFY` capability and is not `CAPS_CHANGE`.
    pub fn is_always_read_only(&self) -> bool {
        !self.contains(Self::MODIFY) && !self.contains(Self::CAPS_CHANGE)
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
    /// Remove only the `MODIFY` flag without removing `NEW`.
    pub fn as_read_only(self) -> Self {
        let mut out = self;
        out.remove(Self::MODIFY);
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

pub(crate) enum VarModifyAnyValue<'a> {
    /// Preferred way, SharedVar needs to provide this, other wrapper vars carefully use this to
    /// store state, like CowVar stores the source var here before the first write
    Boxed(&'a mut BoxedVarValueAny),
    /// MapRefBidi needs this as it can only deref_mut to a reference inside the value.
    RefOnly(&'a mut dyn VarValueAny),
}

impl<'a> ops::Deref for VarModifyAnyValue<'a> {
    type Target = dyn VarValueAny;

    fn deref(&self) -> &Self::Target {
        match self {
            VarModifyAnyValue::Boxed(b) => &***b,
            VarModifyAnyValue::RefOnly(r) => &**r,
        }
    }
}
impl<'a> ops::DerefMut for VarModifyAnyValue<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            VarModifyAnyValue::Boxed(b) => &mut ***b,
            VarModifyAnyValue::RefOnly(r) => &mut **r,
        }
    }
}

/// Mutable reference to a variable value.
///
/// The variable will notify an update only on `deref_mut`.
pub struct VarModifyAny<'a> {
    pub(crate) value: VarModifyAnyValue<'a>,
    pub(crate) update: VarModifyUpdate,
    pub(crate) tags: Vec<BoxedVarValueAny>,
    pub(crate) custom_importance: Option<usize>,
}
impl<'a> VarModifyAny<'a> {
    /// Replace the value if not equal.
    ///
    /// Note that you can also deref_mut to modify the value.
    pub fn set(&mut self, mut new_value: BoxedVarValueAny) -> bool {
        if *self.value != *new_value {
            assert!(
                self.value.try_swap(&mut *new_value),
                "modify set new_value was not of the same type"
            );
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
    pub fn tags(&self) -> &[BoxedVarValueAny] {
        &self.tags
    }

    /// Add a custom tag object that will be shared with the var hooks if the value updates.
    pub fn push_tag(&mut self, tag: impl VarValueAny) {
        self.tags.push(BoxedVarValueAny::new(tag));
    }

    /// Sets a custom [`VarAny::modify_importance`] value.
    ///
    /// Note that the modify info is already automatically set, using a custom value here
    /// can easily break all future modify requests for this variable. The importance is set even if the
    /// variable does not update (no actual value change or update request).
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
    pub fn value(&self) -> &dyn VarValueAny {
        &**self
    }

    /// Mutable reference to the value.
    ///
    /// Getting a mutable reference to the value flags the variable to notify update.
    ///
    /// Note that you can also simply deref to the value.
    pub fn value_mut(&mut self) -> &mut dyn VarValueAny {
        &mut **self
    }
}
impl<'a> ops::Deref for VarModifyAny<'a> {
    type Target = dyn VarValueAny;

    fn deref(&self) -> &Self::Target {
        self.value.deref()
    }
}
impl<'a> ops::DerefMut for VarModifyAny<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.update |= VarModifyUpdate::TOUCHED;
        self.value.deref_mut()
    }
}

/// Mutable reference to a variable value.
///
/// The variable will notify an update only on `deref_mut`.
pub struct VarModify<'s, 'a, T: VarValue> {
    inner: &'s mut VarModifyAny<'a>,
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
    pub fn tags(&self) -> &[BoxedVarValueAny] {
        self.inner.tags()
    }

    /// Add a custom tag object that will be shared with the var hooks if the value updates.
    pub fn push_tag(&mut self, tag: impl VarValueAny) {
        self.inner.push_tag(tag);
    }

    /// Sets a custom [`VarAny::modify_importance`] value.
    ///
    /// Note that the modify info is already automatically set, using a custom value here
    /// can easily break all future modify requests for this variable. The importance is set even if the
    /// variable does not update (no actual value change or update request).
    pub fn set_modify_importance(&mut self, importance: usize) {
        self.inner.set_modify_importance(importance);
    }

    /// Type erased reference.
    pub fn as_any(&mut self) -> &mut VarModifyAny<'a> {
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

/// Handle to a variable hook.
///
/// This can represent a widget subscriber, a var binding, var app handler or animation, dropping the handler stops
/// the behavior it represents.
#[derive(Clone, Default)]
#[must_use = "var handle stops the behavior it represents on drop"]
pub struct VarHandle(Option<Arc<AtomicBool>>); // !!: TODO, this should drop immediately on last drop, that was the case before 
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

    pub(crate) fn new(handle: Arc<AtomicBool>) -> Self {
        Self(Some(handle))
    }

    /// Returns `true` if the handle is a [`dummy`].
    ///
    /// [`dummy`]: VarHandle::dummy
    pub fn is_dummy(&self) -> bool {
        self.0.is_some()
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
    pub fn with(self, other: Self) -> VarHandles {
        VarHandles(smallvec::smallvec![self, other])
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

#[cfg(feature = "value_type_name")]
fn value_type_name(var: &dyn VarImpl) -> &'static str {
    var.value_type_name()
}
#[cfg(not(feature = "value_type_name"))]
#[inline(always)]
fn value_type_name(var: &dyn VarImpl) -> &'static str {
    ""
}
