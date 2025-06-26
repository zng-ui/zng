use std::marker::PhantomData;

use super::*;

/// See [`Var::read_only`].
pub struct ReadOnlyVar<T, V>(PhantomData<T>, V);

/// Weak [`ReadOnlyVar<T>`].
pub struct WeakReadOnlyVar<T, V>(PhantomData<T>, V);

impl<T: VarValue, V: Var<T>> ReadOnlyVar<T, V> {
    pub(super) fn new(var: V) -> Self {
        ReadOnlyVar(PhantomData, var)
    }
}

impl<T: VarValue, V: Var<T>> Clone for ReadOnlyVar<T, V> {
    fn clone(&self) -> Self {
        Self(PhantomData, self.1.clone())
    }
}

impl<T: VarValue, V: WeakVar<T>> Clone for WeakReadOnlyVar<T, V> {
    fn clone(&self) -> Self {
        Self(PhantomData, self.1.clone())
    }
}

impl<T: VarValue, V: Var<T>> crate::private::Sealed for ReadOnlyVar<T, V> {}
impl<T: VarValue, V: WeakVar<T>> crate::private::Sealed for WeakReadOnlyVar<T, V> {}

impl<T: VarValue, V: Var<T>> AnyVar for ReadOnlyVar<T, V> {
    fn clone_any(&self) -> BoxedAnyVar {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_unboxed_any(&self) -> &dyn Any {
        self
    }

    fn double_boxed_any(self: Box<Self>) -> Box<dyn Any> {
        let me: BoxedVar<T> = self;
        Box::new(me)
    }

    fn var_type_id(&self) -> TypeId {
        self.1.var_type_id()
    }

    fn get_any(&self) -> Box<dyn AnyVarValue> {
        self.1.get_any()
    }

    fn with_any(&self, read: &mut dyn FnMut(&dyn AnyVarValue)) {
        self.1.with_any(read)
    }

    fn with_new_any(&self, read: &mut dyn FnMut(&dyn AnyVarValue)) -> bool {
        self.1.with_new_any(read)
    }

    fn set_any(&self, _: Box<dyn AnyVarValue>) -> Result<(), VarIsReadOnlyError> {
        Err(VarIsReadOnlyError {
            capabilities: self.capabilities(),
        })
    }

    fn last_update(&self) -> VarUpdateId {
        self.1.last_update()
    }

    fn is_contextual(&self) -> bool {
        self.1.is_contextual()
    }

    fn capabilities(&self) -> VarCapability {
        self.1.capabilities().as_read_only()
    }

    fn hook_any(&self, pos_modify_action: Box<dyn Fn(&AnyVarHookArgs) -> bool + Send + Sync>) -> VarHandle {
        self.1.hook_any(pos_modify_action)
    }

    fn hook_animation_stop(&self, handler: Box<dyn FnOnce() + Send>) -> Result<(), Box<dyn FnOnce() + Send>> {
        self.1.hook_animation_stop(handler)
    }

    fn strong_count(&self) -> usize {
        self.1.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.1.weak_count()
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        Box::new(self.clone())
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        Box::new(WeakReadOnlyVar(PhantomData, self.1.downgrade()))
    }

    fn is_animating(&self) -> bool {
        self.1.is_animating()
    }

    fn modify_importance(&self) -> usize {
        self.1.modify_importance()
    }

    fn var_ptr(&self) -> VarPtr<'_> {
        self.1.var_ptr()
    }

    fn get_debug(&self) -> Txt {
        self.with(var_debug)
    }

    fn update(&self) -> Result<(), VarIsReadOnlyError> {
        Var::modify(self, var_update)
    }

    fn map_debug(&self) -> BoxedVar<Txt> {
        Var::map(self, var_debug).boxed()
    }
}

impl<T: VarValue, V: WeakVar<T>> AnyWeakVar for WeakReadOnlyVar<T, V> {
    fn clone_any(&self) -> BoxedAnyWeakVar {
        Box::new(self.clone())
    }

    fn strong_count(&self) -> usize {
        self.1.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.1.weak_count()
    }

    fn upgrade_any(&self) -> Option<BoxedAnyVar> {
        self.1.upgrade().map(|inner| Box::new(inner.read_only()) as _)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<T: VarValue, V: Var<T>> IntoVar<T> for ReadOnlyVar<T, V> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T: VarValue, V: Var<T>> Var<T> for ReadOnlyVar<T, V> {
    type ReadOnly = Self;

    type ActualVar = <V::ActualVar as Var<T>>::ReadOnly;

    type Downgrade = WeakReadOnlyVar<T, V::Downgrade>;

    type Map<O: VarValue> = V::Map<O>;
    type MapBidi<O: VarValue> = V::Map<O>;

    type FlatMap<O: VarValue, VF: Var<O>> = V::FlatMap<O, VF>;

    type FilterMap<O: VarValue> = V::FilterMap<O>;
    type FilterMapBidi<O: VarValue> = V::FilterMap<O>;

    type MapRef<O: VarValue> = V::MapRef<O>;
    type MapRefBidi<O: VarValue> = V::MapRef<O>;

    type Easing = <V::Easing as Var<T>>::ReadOnly;

    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.1.with(read)
    }

    fn modify<F>(&self, _: F) -> Result<(), VarIsReadOnlyError>
    where
        F: FnOnce(&mut VarModify<T>) + 'static,
    {
        Err(VarIsReadOnlyError {
            capabilities: self.capabilities(),
        })
    }

    fn boxed(self) -> BoxedVar<T>
    where
        Self: Sized,
    {
        Box::new(self)
    }

    fn actual_var(self) -> Self::ActualVar {
        self.1.actual_var().read_only()
    }

    fn downgrade(&self) -> Self::Downgrade {
        WeakReadOnlyVar(PhantomData, self.1.downgrade())
    }

    fn into_value(self) -> T {
        self.1.into_value()
    }

    fn read_only(&self) -> Self::ReadOnly {
        self.clone()
    }

    fn map<O, M>(&self, map: M) -> Self::Map<O>
    where
        O: VarValue,
        M: FnMut(&T) -> O + Send + 'static,
    {
        self.1.map(map)
    }

    fn map_bidi<O, M, B>(&self, map: M, _: B) -> Self::MapBidi<O>
    where
        O: VarValue,
        M: FnMut(&T) -> O + Send + 'static,
        B: FnMut(&O) -> T + Send + 'static,
    {
        self.1.map(map)
    }

    fn flat_map<O, VF, M>(&self, map: M) -> Self::FlatMap<O, VF>
    where
        O: VarValue,
        VF: Var<O>,
        M: FnMut(&T) -> VF + Send + 'static,
    {
        self.1.flat_map(map)
    }

    fn filter_map<O, M, I>(&self, map: M, fallback: I) -> Self::FilterMap<O>
    where
        O: VarValue,
        M: FnMut(&T) -> Option<O> + Send + 'static,
        I: Fn() -> O + Send + Sync + 'static,
    {
        self.1.filter_map(map, fallback)
    }

    fn filter_map_bidi<O, M, B, I>(&self, map: M, _: B, fallback: I) -> Self::FilterMapBidi<O>
    where
        O: VarValue,
        M: FnMut(&T) -> Option<O> + Send + 'static,
        B: FnMut(&O) -> Option<T> + Send + 'static,
        I: Fn() -> O + Send + Sync + 'static,
    {
        self.1.filter_map(map, fallback)
    }

    fn map_ref<O, M>(&self, map: M) -> Self::MapRef<O>
    where
        O: VarValue,
        M: Fn(&T) -> &O + Send + Sync + 'static,
    {
        self.1.map_ref(map)
    }

    fn map_ref_bidi<O, M, B>(&self, map: M, _: B) -> Self::MapRefBidi<O>
    where
        O: VarValue,
        M: Fn(&T) -> &O + Send + Sync + 'static,
        B: Fn(&mut T) -> &mut O + Send + Sync + 'static,
    {
        self.1.map_ref(map)
    }

    fn easing<F>(&self, duration: Duration, easing: F) -> Self::Easing
    where
        T: Transitionable,
        F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
    {
        self.1.easing(duration, easing).read_only()
    }

    fn easing_with<F, S>(&self, duration: Duration, easing: F, sampler: S) -> Self::Easing
    where
        T: Transitionable,
        F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
        S: Fn(&animation::Transition<T>, EasingStep) -> T + Send + Sync + 'static,
    {
        self.1.easing_with(duration, easing, sampler).read_only()
    }
}

impl<T: VarValue, V: WeakVar<T>> WeakVar<T> for WeakReadOnlyVar<T, V> {
    type Upgrade = <V::Upgrade as Var<T>>::ReadOnly;

    fn upgrade(&self) -> Option<Self::Upgrade> {
        self.1.upgrade().map(|inner| inner.read_only())
    }
}

/// Read-only [`ArcVar<T>`].
pub type ReadOnlyArcVar<T> = ReadOnlyVar<T, ArcVar<T>>;
