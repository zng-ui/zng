use std::sync::{Arc, Weak};

use super::{util::VarData, *};

/// Reference counted read/write variable.
///
/// This is the primary variable type, it can be instantiated using the [`var`] and [`var_from`] functions.
#[derive(Clone)]
pub struct ArcVar<T: VarValue>(Arc<VarData>, PhantomData<T>);

/// Weak reference to a [`ArcVar<T>`].
#[derive(Clone)]
pub struct WeakArcVar<T: VarValue>(Weak<VarData>, PhantomData<T>);

/// New ref counted read/write variable with initial `value`.
pub fn var<T: VarValue>(value: T) -> ArcVar<T> {
    ArcVar(Arc::new(VarData::new(value)), PhantomData)
}

/// New ref counted read/write variable with initial value converted from `source`.
pub fn var_from<T: VarValue, U: Into<T>>(source: U) -> ArcVar<T> {
    var(source.into())
}

/// New ref counted read/write variable with default initial value.
pub fn var_default<T: VarValue + Default>() -> ArcVar<T> {
    var(T::default())
}

impl<T: VarValue> WeakArcVar<T> {
    /// New reference to nothing.
    pub fn new() -> Self {
        Self(Weak::new(), PhantomData)
    }
}

impl<T: VarValue> Default for WeakArcVar<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: VarValue> crate::private::Sealed for ArcVar<T> {}

impl<T: VarValue> crate::private::Sealed for WeakArcVar<T> {}

impl<T: VarValue> AnyVar for ArcVar<T> {
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
        TypeId::of::<T>()
    }

    fn get_any(&self) -> Box<dyn AnyVarValue> {
        Box::new(self.get())
    }

    fn with_any(&self, read: &mut dyn FnMut(&dyn AnyVarValue)) {
        self.with(|v| read(v))
    }

    fn with_new_any(&self, read: &mut dyn FnMut(&dyn AnyVarValue)) -> bool {
        self.with_new(|v| read(v)).is_some()
    }

    fn set_any(&self, value: Box<dyn AnyVarValue>) -> Result<(), VarIsReadOnlyError> {
        self.modify(var_set_any(value));
        Ok(())
    }

    fn last_update(&self) -> VarUpdateId {
        self.0.last_update()
    }

    fn is_contextual(&self) -> bool {
        false
    }

    fn capabilities(&self) -> VarCapability {
        VarCapability::MODIFY
    }

    fn hook_any(&self, pos_modify_action: Box<dyn Fn(&AnyVarHookArgs) -> bool + Send + Sync>) -> VarHandle {
        self.0.push_hook(pos_modify_action)
    }

    fn hook_animation_stop(&self, handler: Box<dyn FnOnce() + Send>) -> Result<(), Box<dyn FnOnce() + Send>> {
        self.0.push_animation_hook(handler)
    }

    fn strong_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }

    fn weak_count(&self) -> usize {
        Arc::weak_count(&self.0)
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        Box::new(self.clone())
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        Box::new(WeakArcVar(Arc::downgrade(&self.0), PhantomData::<T>))
    }

    fn is_animating(&self) -> bool {
        self.0.is_animating()
    }

    fn modify_importance(&self) -> usize {
        self.0.modify_importance()
    }

    fn var_ptr(&self) -> VarPtr<'_> {
        VarPtr::new_arc(&self.0)
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

impl<T: VarValue> AnyWeakVar for WeakArcVar<T> {
    fn clone_any(&self) -> BoxedAnyWeakVar {
        Box::new(self.clone())
    }

    fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.0.weak_count()
    }

    fn upgrade_any(&self) -> Option<BoxedAnyVar> {
        self.0.upgrade().map(|rc| Box::new(ArcVar(rc, PhantomData::<T>)) as _)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<T: VarValue> IntoVar<T> for ArcVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T: VarValue> ArcVar<T> {
    #[cfg(feature = "dyn_closure")]
    fn modify_impl(&self, modify: Box<dyn FnOnce(&mut VarModify<T>) + Send + 'static>) -> Result<(), VarIsReadOnlyError> {
        let me = self.clone();
        VARS.schedule_update(
            Box::new(move || {
                me.0.apply_modify(modify);
            }),
            std::any::type_name::<T>(),
        );
        Ok(())
    }

    #[cfg(not(feature = "dyn_closure"))]
    fn modify_impl(&self, modify: impl FnOnce(&mut VarModify<T>) + Send + 'static) -> Result<(), VarIsReadOnlyError> {
        let me = self.clone();
        VARS.schedule_update(
            Box::new(move || {
                me.0.apply_modify(modify);
            }),
            std::any::type_name::<T>(),
        );
        Ok(())
    }

    impl_infallible_write! {
        for<T>
    }
}

impl<T: VarValue> Var<T> for ArcVar<T> {
    type ReadOnly = types::ReadOnlyVar<T, Self>;

    type ActualVar = Self;

    type Downgrade = WeakArcVar<T>;

    type Map<O: VarValue> = ReadOnlyArcVar<O>;
    type MapBidi<O: VarValue> = ArcVar<O>;

    type FlatMap<O: VarValue, V: Var<O>> = types::ArcFlatMapVar<O, V>;

    type FilterMap<O: VarValue> = ReadOnlyArcVar<O>;
    type FilterMapBidi<O: VarValue> = ArcVar<O>;

    type MapRef<O: VarValue> = types::MapRef<T, O, Self>;
    type MapRefBidi<O: VarValue> = types::MapRefBidi<T, O, Self>;

    type Easing = ReadOnlyArcVar<T>;

    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.0.with(read)
    }

    fn modify<F>(&self, modify: F) -> Result<(), VarIsReadOnlyError>
    where
        F: FnOnce(&mut VarModify<T>) + Send + 'static,
    {
        #[cfg(feature = "dyn_closure")]
        let modify: Box<dyn FnOnce(&mut VarModify<T>) + Send + 'static> = Box::new(modify);
        self.modify_impl(modify)
    }

    fn actual_var(self) -> Self {
        self
    }

    fn downgrade(&self) -> WeakArcVar<T> {
        WeakArcVar(Arc::downgrade(&self.0), PhantomData::<T>)
    }

    fn into_value(self) -> T {
        match Arc::try_unwrap(self.0) {
            Ok(data) => data.into_value(),
            Err(rc) => Self(rc, PhantomData).get(),
        }
    }

    fn read_only(&self) -> Self::ReadOnly {
        types::ReadOnlyVar::new(self.clone())
    }

    fn map<O, M>(&self, map: M) -> Self::Map<O>
    where
        O: VarValue,
        M: FnMut(&T) -> O + Send + 'static,
    {
        var_map(self, map)
    }

    fn map_bidi<O, M, B>(&self, map: M, map_back: B) -> Self::MapBidi<O>
    where
        O: VarValue,
        M: FnMut(&T) -> O + Send + 'static,
        B: FnMut(&O) -> T + Send + 'static,
    {
        var_map_bidi(self, map, map_back)
    }

    fn flat_map<O, V, M>(&self, map: M) -> Self::FlatMap<O, V>
    where
        O: VarValue,
        V: Var<O>,
        M: FnMut(&T) -> V + Send + 'static,
    {
        var_flat_map(self, map)
    }

    fn filter_map<O, M, I>(&self, map: M, fallback: I) -> Self::FilterMap<O>
    where
        O: VarValue,
        M: FnMut(&T) -> Option<O> + Send + 'static,
        I: Fn() -> O + Send + Sync + 'static,
    {
        var_filter_map(self, map, fallback)
    }

    fn filter_map_bidi<O, M, B, I>(&self, map: M, map_back: B, fallback: I) -> Self::FilterMapBidi<O>
    where
        O: VarValue,
        M: FnMut(&T) -> Option<O> + Send + 'static,
        B: FnMut(&O) -> Option<T> + Send + 'static,
        I: Fn() -> O + Send + Sync + 'static,
    {
        var_filter_map_bidi(self, map, map_back, fallback)
    }

    fn map_ref<O, M>(&self, map: M) -> Self::MapRef<O>
    where
        O: VarValue,
        M: Fn(&T) -> &O + Send + Sync + 'static,
    {
        var_map_ref(self, map)
    }

    fn map_ref_bidi<O, M, B>(&self, map: M, map_mut: B) -> Self::MapRefBidi<O>
    where
        O: VarValue,
        M: Fn(&T) -> &O + Send + Sync + 'static,
        B: Fn(&mut T) -> &mut O + Send + Sync + 'static,
    {
        var_map_ref_bidi(self, map, map_mut)
    }

    fn easing<F>(&self, duration: Duration, easing: F) -> Self::Easing
    where
        T: Transitionable,
        F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
    {
        var_easing(self, duration, easing)
    }

    fn easing_with<F, S>(&self, duration: Duration, easing: F, sampler: S) -> Self::Easing
    where
        T: Transitionable,
        F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
        S: Fn(&animation::Transition<T>, EasingStep) -> T + Send + Sync + 'static,
    {
        var_easing_with(self, duration, easing, sampler)
    }
}

impl<T: VarValue> WeakVar<T> for WeakArcVar<T> {
    type Upgrade = ArcVar<T>;

    fn upgrade(&self) -> Option<ArcVar<T>> {
        self.0.upgrade().map(|rc| ArcVar(rc, PhantomData))
    }
}

/// Variable for state properties (`is_*`, `has_*`).
///
/// State variables are `bool` probes that are set by the property, they are created automatically
/// by the property default when used in `when` expressions, but can be created manually.
pub fn state_var() -> ArcVar<bool> {
    var(false)
}

/// Variable for getter properties (`get_*`, `actual_*`).
///
/// Getter variables are inited with a default value that is overridden by the property on node init and updated
/// by the property when the internal state they track changes. They are created automatically by the property
/// default when used in `when` expressions, but can be created manually.
pub fn getter_var<T: VarValue + Default>() -> ArcVar<T> {
    var(T::default())
}
