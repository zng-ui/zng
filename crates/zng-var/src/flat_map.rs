use std::{
    marker::PhantomData,
    sync::{Arc, Weak},
};

use parking_lot::RwLock;

use super::*;

struct Data<T, V> {
    _t: PhantomData<T>,
    _source: BoxedAnyVar,
    var: V,
    source_handle: VarHandle,
    last_update: VarUpdateId,
    var_handle: VarHandle,
    hooks: Vec<VarHook>,
}

/// See [`Var::flat_map`].
pub struct ArcFlatMapVar<T, V>(Arc<RwLock<Data<T, V>>>);

/// Weak reference to a [`ArcFlatMapVar<T, V>`].
pub struct WeakFlatMapVar<T, V>(Weak<RwLock<Data<T, V>>>);

impl<T, V> ArcFlatMapVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
    /// New.
    pub fn new<I: VarValue>(source: &impl Var<I>, mut map: impl FnMut(&I) -> V + Send + 'static) -> Self {
        let flat = Arc::new(RwLock::new(Data {
            _t: PhantomData,
            _source: source.clone_any(),
            var: source.with(&mut map),
            last_update: VarUpdateId::never(),
            source_handle: VarHandle::dummy(),
            var_handle: VarHandle::dummy(),
            hooks: vec![],
        }));

        {
            let mut data = flat.write();
            let weak_flat = Arc::downgrade(&flat);
            let map = Mutex::new(map);
            data.var_handle = data.var.hook_any(ArcFlatMapVar::on_var_hook(weak_flat.clone()));
            data.source_handle = source.hook_any(Box::new(move |args| {
                if let Some(flat) = weak_flat.upgrade() {
                    if let Some(value) = args.downcast_value() {
                        let mut data = flat.write();
                        let data = &mut *data;
                        data.var = map.lock()(value);
                        data.var_handle = data.var.hook_any(ArcFlatMapVar::on_var_hook(weak_flat.clone()));
                        data.last_update = VARS.update_id();
                        data.var.with(|value| {
                            let args = AnyVarHookArgs::new(value, args.update(), args.tags());
                            data.hooks.retain(|h| h.call(&args));
                        });
                    }
                    true
                } else {
                    false
                }
            }));
        }

        Self(flat)
    }

    fn on_var_hook(weak_flat: Weak<RwLock<Data<T, V>>>) -> Box<dyn Fn(&AnyVarHookArgs) -> bool + Send + Sync> {
        Box::new(move |args| {
            if let Some(flat) = weak_flat.upgrade() {
                let mut data = flat.write();
                data.last_update = VARS.update_id();
                data.hooks.retain(|h| h.call(args));
                true
            } else {
                false
            }
        })
    }
}

impl<T, V> Clone for ArcFlatMapVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T, V> Clone for WeakFlatMapVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T, V> crate::private::Sealed for ArcFlatMapVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
}

impl<T, V> crate::private::Sealed for WeakFlatMapVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
}

impl<T, V> AnyVar for ArcFlatMapVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
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
        self.0.read().var.with_any(read)
    }

    fn with_new_any(&self, read: &mut dyn FnMut(&dyn AnyVarValue)) -> bool {
        self.0.read().var.with_new_any(read)
    }

    fn set_any(&self, value: Box<dyn AnyVarValue>) -> Result<(), VarIsReadOnlyError> {
        self.modify(var_set_any(value))
    }

    fn last_update(&self) -> VarUpdateId {
        self.0.read().last_update
    }

    fn is_contextual(&self) -> bool {
        self.0.read().var.is_contextual()
    }

    fn capabilities(&self) -> VarCapability {
        self.0.read().var.capabilities() | VarCapability::CAPS_CHANGE
    }

    fn hook_any(&self, pos_modify_action: Box<dyn Fn(&AnyVarHookArgs) -> bool + Send + Sync>) -> VarHandle {
        let (handle, weak_handle) = VarHandle::new(pos_modify_action);
        self.0.write().hooks.push(weak_handle);
        handle
    }

    fn hook_animation_stop(&self, handler: Box<dyn FnOnce() + Send>) -> Result<(), Box<dyn FnOnce() + Send>> {
        self.0.read().var.hook_animation_stop(handler)
    }

    fn strong_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }

    fn weak_count(&self) -> usize {
        Arc::weak_count(&self.0)
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        self.clone_any()
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        Box::new(self.downgrade())
    }

    fn is_animating(&self) -> bool {
        self.0.read().var.is_animating()
    }

    fn modify_importance(&self) -> usize {
        self.0.read().var.modify_importance()
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

impl<T, V> AnyWeakVar for WeakFlatMapVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
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
        self.0.upgrade().map(|rc| Box::new(ArcFlatMapVar(rc)) as _)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<T, V> IntoVar<T> for ArcFlatMapVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T, V> Var<T> for ArcFlatMapVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
    type ReadOnly = types::ReadOnlyVar<T, Self>;

    type ActualVar = Self;

    type Downgrade = WeakFlatMapVar<T, V>;

    type Map<O: VarValue> = ReadOnlyArcVar<O>;
    type MapBidi<O: VarValue> = ArcVar<O>;

    type FlatMap<O: VarValue, VF: Var<O>> = types::ArcFlatMapVar<O, VF>;

    type FilterMap<O: VarValue> = ReadOnlyArcVar<O>;
    type FilterMapBidi<O: VarValue> = ArcVar<O>;

    type MapRef<O: VarValue> = types::MapRef<T, O, Self>;
    type MapRefBidi<O: VarValue> = types::MapRefBidi<T, O, Self>;

    type Easing = ReadOnlyArcVar<T>;

    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.0.read_recursive().var.with(read)
    }

    fn modify<F>(&self, modify: F) -> Result<(), VarIsReadOnlyError>
    where
        F: FnOnce(&mut VarModify<T>) + Send + 'static,
    {
        self.0.read_recursive().var.modify(modify)
    }

    fn actual_var(self) -> Self {
        self
    }

    fn downgrade(&self) -> Self::Downgrade {
        WeakFlatMapVar(Arc::downgrade(&self.0))
    }

    fn into_value(self) -> T {
        match Arc::try_unwrap(self.0) {
            Ok(state) => state.into_inner().var.into_value(),
            Err(rc) => Self(rc).get(),
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

    fn flat_map<O, VF, M>(&self, map: M) -> Self::FlatMap<O, VF>
    where
        O: VarValue,
        VF: Var<O>,
        M: FnMut(&T) -> VF + Send + 'static,
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

impl<T, V> WeakVar<T> for WeakFlatMapVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
    type Upgrade = ArcFlatMapVar<T, V>;

    fn upgrade(&self) -> Option<Self::Upgrade> {
        self.0.upgrade().map(|rc| ArcFlatMapVar(rc))
    }
}
