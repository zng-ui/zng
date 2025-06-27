use std::any::TypeId;

use super::*;

/// Represents a [`Var<T>`] boxed.
pub type BoxedVar<T> = Box<dyn VarBoxed<T>>;

/// Represents a weak reference to a [`BoxedVar<T>`].
pub type BoxedWeakVar<T> = Box<dyn WeakVarBoxed<T>>;

/// Represents a type erased boxed var.
pub type BoxedAnyVar = Box<dyn AnyVar>;

/// Represents a weak reference to a [`BoxedAnyVar`].
pub type BoxedAnyWeakVar = Box<dyn AnyWeakVar>;

impl<T: VarValue> Clone for BoxedWeakVar<T> {
    fn clone(&self) -> Self {
        self.clone_boxed()
    }
}

impl Clone for BoxedAnyVar {
    fn clone(&self) -> Self {
        self.clone_any()
    }
}

impl Clone for BoxedAnyWeakVar {
    fn clone(&self) -> Self {
        self.clone_any()
    }
}

impl crate::private::Sealed for Box<dyn AnyVar> {}

impl AnyVar for Box<dyn AnyVar> {
    fn clone_any(&self) -> BoxedAnyVar {
        (**self).clone_any()
    }

    fn as_any(&self) -> &dyn Any {
        (**self).as_any()
    }

    fn as_unboxed_any(&self) -> &dyn Any {
        (**self).as_unboxed_any()
    }

    fn double_boxed_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn var_type_id(&self) -> TypeId {
        (**self).var_type_id()
    }

    fn get_any(&self) -> Box<dyn AnyVarValue> {
        (**self).get_any()
    }

    fn with_any(&self, read: &mut dyn FnMut(&dyn AnyVarValue)) {
        (**self).with_any(read)
    }

    fn with_new_any(&self, read: &mut dyn FnMut(&dyn AnyVarValue)) -> bool {
        (**self).with_new_any(read)
    }

    fn set_any(&self, value: Box<dyn AnyVarValue>) -> Result<(), VarIsReadOnlyError> {
        (**self).set_any(value)
    }

    fn last_update(&self) -> VarUpdateId {
        (**self).last_update()
    }

    fn is_contextual(&self) -> bool {
        (**self).is_contextual()
    }

    fn capabilities(&self) -> VarCapability {
        (**self).capabilities()
    }

    fn is_animating(&self) -> bool {
        (**self).is_animating()
    }

    fn modify_importance(&self) -> usize {
        (**self).modify_importance()
    }

    fn hook_any(&self, pos_modify_action: Box<dyn Fn(&AnyVarHookArgs) -> bool + Send + Sync>) -> VarHandle {
        (**self).hook_any(pos_modify_action)
    }

    fn hook_animation_stop(&self, handler: Box<dyn FnOnce() + Send>) -> Result<(), Box<dyn FnOnce() + Send>> {
        (**self).hook_animation_stop(handler)
    }

    fn strong_count(&self) -> usize {
        (**self).strong_count()
    }

    fn weak_count(&self) -> usize {
        (**self).weak_count()
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        (**self).actual_var_any()
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        (**self).downgrade_any()
    }

    fn var_ptr(&self) -> VarPtr<'_> {
        (**self).var_ptr()
    }

    fn get_debug(&self) -> Txt {
        (**self).get_debug()
    }

    fn update(&self) -> Result<(), VarIsReadOnlyError> {
        (**self).update()
    }

    fn map_debug(&self) -> BoxedVar<Txt> {
        (**self).map_debug()
    }
}

#[doc(hidden)]
pub trait VarBoxed<T: VarValue>: AnyVar {
    fn clone_boxed(&self) -> BoxedVar<T>;
    fn with_boxed(&self, read: &mut dyn FnMut(&T));
    fn modify_boxed(&self, modify: Box<dyn FnOnce(&mut VarModify<T>) + Send>) -> Result<(), VarIsReadOnlyError>;
    fn actual_var_boxed(self: Box<Self>) -> BoxedVar<T>;
    fn downgrade_boxed(&self) -> BoxedWeakVar<T>;
    fn read_only_boxed(&self) -> BoxedVar<T>;
    fn boxed_any_boxed(self: Box<Self>) -> BoxedAnyVar;
}
impl<T: VarValue, V: Var<T>> VarBoxed<T> for V {
    fn clone_boxed(&self) -> BoxedVar<T> {
        self.clone().boxed()
    }

    fn with_boxed(&self, read: &mut dyn FnMut(&T)) {
        self.with(read)
    }

    fn modify_boxed(&self, modify: Box<dyn FnOnce(&mut VarModify<T>) + Send>) -> Result<(), VarIsReadOnlyError> {
        self.modify(modify)
    }

    fn actual_var_boxed(self: Box<Self>) -> BoxedVar<T> {
        (*self).actual_var().boxed()
    }

    fn downgrade_boxed(&self) -> BoxedWeakVar<T> {
        self.downgrade().boxed()
    }

    fn read_only_boxed(&self) -> BoxedVar<T> {
        self.read_only().boxed()
    }

    fn boxed_any_boxed(self: Box<Self>) -> BoxedAnyVar {
        self
    }
}

#[doc(hidden)]
pub trait WeakVarBoxed<T: VarValue>: AnyWeakVar {
    fn clone_boxed(&self) -> BoxedWeakVar<T>;
    fn upgrade_boxed(&self) -> Option<BoxedVar<T>>;
}
impl<T: VarValue, W: WeakVar<T>> WeakVarBoxed<T> for W {
    fn clone_boxed(&self) -> BoxedWeakVar<T> {
        self.clone().boxed()
    }

    fn upgrade_boxed(&self) -> Option<BoxedVar<T>> {
        self.upgrade().map(Var::boxed)
    }
}

impl<T: VarValue> crate::private::Sealed for BoxedWeakVar<T> {}

impl<T: VarValue> AnyWeakVar for BoxedWeakVar<T> {
    fn clone_any(&self) -> BoxedAnyWeakVar {
        (**self).clone_any()
    }

    fn strong_count(&self) -> usize {
        (**self).strong_count()
    }

    fn weak_count(&self) -> usize {
        (**self).weak_count()
    }

    fn upgrade_any(&self) -> Option<BoxedAnyVar> {
        (**self).upgrade_any()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
impl<T: VarValue> WeakVar<T> for BoxedWeakVar<T> {
    type Upgrade = BoxedVar<T>;

    fn upgrade(&self) -> Option<Self::Upgrade> {
        (**self).upgrade_boxed()
    }
}

impl<T: VarValue> crate::private::Sealed for BoxedVar<T> {}

impl<T: VarValue> Clone for BoxedVar<T> {
    fn clone(&self) -> Self {
        (**self).clone_boxed()
    }
}

impl<T: VarValue> AnyVar for BoxedVar<T> {
    fn clone_any(&self) -> BoxedAnyVar {
        (**self).clone_any()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_unboxed_any(&self) -> &dyn Any {
        (**self).as_unboxed_any()
    }

    fn double_boxed_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn var_type_id(&self) -> TypeId {
        (**self).var_type_id()
    }

    fn get_any(&self) -> Box<dyn AnyVarValue> {
        (**self).get_any()
    }

    fn with_any(&self, read: &mut dyn FnMut(&dyn AnyVarValue)) {
        (**self).with_any(read)
    }

    fn with_new_any(&self, read: &mut dyn FnMut(&dyn AnyVarValue)) -> bool {
        (**self).with_new_any(read)
    }

    fn set_any(&self, value: Box<dyn AnyVarValue>) -> Result<(), VarIsReadOnlyError> {
        (**self).set_any(value)
    }

    fn last_update(&self) -> VarUpdateId {
        (**self).last_update()
    }

    fn is_contextual(&self) -> bool {
        (**self).is_contextual()
    }

    fn capabilities(&self) -> VarCapability {
        (**self).capabilities()
    }

    fn hook_any(&self, pos_modify_action: Box<dyn Fn(&AnyVarHookArgs) -> bool + Send + Sync>) -> VarHandle {
        (**self).hook_any(pos_modify_action)
    }

    fn hook_animation_stop(&self, handler: Box<dyn FnOnce() + Send>) -> Result<(), Box<dyn FnOnce() + Send>> {
        (**self).hook_animation_stop(handler)
    }

    fn strong_count(&self) -> usize {
        (**self).strong_count()
    }

    fn weak_count(&self) -> usize {
        (**self).weak_count()
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        (**self).actual_var_any()
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        (**self).downgrade_any()
    }

    fn is_animating(&self) -> bool {
        (**self).is_animating()
    }

    fn modify_importance(&self) -> usize {
        (**self).modify_importance()
    }

    fn var_ptr(&self) -> VarPtr<'_> {
        (**self).var_ptr()
    }

    fn get_debug(&self) -> Txt {
        (**self).get_debug()
    }

    fn update(&self) -> Result<(), VarIsReadOnlyError> {
        (**self).update()
    }

    fn map_debug(&self) -> BoxedVar<Txt> {
        (**self).map_debug()
    }
}

impl<T: VarValue> IntoVar<T> for BoxedVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T: VarValue> Var<T> for BoxedVar<T> {
    type ReadOnly = BoxedVar<T>;

    type ActualVar = BoxedVar<T>;

    type Downgrade = BoxedWeakVar<T>;

    type Map<O: VarValue> = BoxedVar<O>;
    type MapBidi<O: VarValue> = BoxedVar<O>;

    type FlatMap<O: VarValue, V: Var<O>> = BoxedVar<O>;

    type FilterMap<O: VarValue> = BoxedVar<O>;
    type FilterMapBidi<O: VarValue> = BoxedVar<O>;

    type MapRef<O: VarValue> = BoxedVar<O>;
    type MapRefBidi<O: VarValue> = BoxedVar<O>;

    type Easing = BoxedVar<T>;

    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        #[cfg(feature = "dyn_closure")]
        let read: Box<dyn FnOnce(&T) -> R> = Box::new(read);
        boxed_var_with(self, read)
    }

    fn modify<F>(&self, modify: F) -> Result<(), VarIsReadOnlyError>
    where
        F: FnOnce(&mut VarModify<T>) + Send + 'static,
    {
        let modify = Box::new(modify);
        (**self).modify_boxed(modify)
    }

    fn boxed(self) -> BoxedVar<T> {
        self
    }

    fn boxed_any(self) -> BoxedAnyVar
    where
        Self: Sized,
    {
        // fix after https://github.com/rust-lang/rust/issues/65991
        self.clone_any()
    }

    fn actual_var(self) -> BoxedVar<T> {
        self.actual_var_boxed()
    }

    fn downgrade(&self) -> BoxedWeakVar<T> {
        (**self).downgrade_boxed()
    }

    fn into_value(self) -> T {
        self.get()
    }

    fn read_only(&self) -> Self::ReadOnly {
        if self.capabilities().is_always_read_only() {
            self.clone()
        } else {
            (**self).read_only_boxed()
        }
    }

    fn map<O, M>(&self, map: M) -> Self::Map<O>
    where
        O: VarValue,
        M: FnMut(&T) -> O + Send + 'static,
    {
        var_map_mixed(self, map)
    }

    fn map_bidi<O, M, B>(&self, map: M, map_back: B) -> Self::MapBidi<O>
    where
        O: VarValue,
        M: FnMut(&T) -> O + Send + 'static,
        B: FnMut(&O) -> T + Send + 'static,
    {
        var_map_bidi_mixed(self, map, map_back)
    }

    fn flat_map<O, V, M>(&self, map: M) -> Self::FlatMap<O, V>
    where
        O: VarValue,
        V: Var<O>,
        M: FnMut(&T) -> V + Send + 'static,
    {
        var_flat_map_mixed(self, map)
    }

    fn filter_map<O, M, I>(&self, map: M, fallback: I) -> Self::FilterMap<O>
    where
        O: VarValue,
        M: FnMut(&T) -> Option<O> + Send + 'static,
        I: Fn() -> O + Send + Sync + 'static,
    {
        var_filter_map_mixed(self, map, fallback)
    }

    fn filter_map_bidi<O, M, B, I>(&self, map: M, map_back: B, fallback: I) -> Self::FilterMapBidi<O>
    where
        O: VarValue,
        M: FnMut(&T) -> Option<O> + Send + 'static,
        B: FnMut(&O) -> Option<T> + Send + 'static,
        I: Fn() -> O + Send + Sync + 'static,
    {
        var_filter_map_bidi_mixed(self, map, map_back, fallback)
    }

    fn map_ref<O, M>(&self, map: M) -> Self::MapRef<O>
    where
        O: VarValue,
        M: Fn(&T) -> &O + Send + Sync + 'static,
    {
        var_map_ref(self, map).boxed()
    }

    fn map_ref_bidi<O, M, B>(&self, map: M, map_mut: B) -> Self::MapRefBidi<O>
    where
        O: VarValue,
        M: Fn(&T) -> &O + Send + Sync + 'static,
        B: Fn(&mut T) -> &mut O + Send + Sync + 'static,
    {
        var_map_ref_bidi(self, map, map_mut).boxed()
    }

    fn easing<F>(&self, duration: Duration, easing: F) -> Self::Easing
    where
        T: Transitionable,
        F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
    {
        var_easing_mixed(self, duration, easing)
    }

    fn easing_with<F, S>(&self, duration: Duration, easing: F, sampler: S) -> Self::Easing
    where
        T: Transitionable,
        F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
        S: Fn(&animation::Transition<T>, EasingStep) -> T + Send + Sync + 'static,
    {
        var_easing_with_mixed(self, duration, easing, sampler)
    }
}

fn boxed_var_with<T: VarValue, R, F>(var: &BoxedVar<T>, read: F) -> R
where
    F: FnOnce(&T) -> R,
{
    let mut read = Some(read);
    let mut result = None;
    (**var).with_boxed(&mut |var_value| {
        let read = read.take().unwrap();
        result = Some(read(var_value));
    });
    result.take().unwrap()
}
