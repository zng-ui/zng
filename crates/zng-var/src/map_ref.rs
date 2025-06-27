use super::*;

/// See [`Var::map_ref`].
pub struct MapRef<I, O, S> {
    source: S,
    map: Arc<dyn Fn(&I) -> &O + Send + Sync>,
}

/// Weak var that can upgrade to [`MapRef<I, O, S>`] if the source is not dropped.
pub struct WeakMapRef<I, O, S> {
    source: S,
    map: Arc<dyn Fn(&I) -> &O + Send + Sync>,
}

impl<I: VarValue, O: VarValue, S: Var<I>> MapRef<I, O, S> {
    pub(super) fn new(source: S, map: Arc<dyn Fn(&I) -> &O + Send + Sync>) -> Self {
        MapRef { source, map }
    }
}

impl<I: VarValue, O: VarValue, S: Var<I>> crate::private::Sealed for MapRef<I, O, S> {}
impl<I: VarValue, O: VarValue, S: WeakVar<I>> crate::private::Sealed for WeakMapRef<I, O, S> {}

impl<I: VarValue, O: VarValue, S: Var<I>> Clone for MapRef<I, O, S> {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            map: self.map.clone(),
        }
    }
}
impl<I: VarValue, O: VarValue, S: WeakVar<I>> Clone for WeakMapRef<I, O, S> {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            map: self.map.clone(),
        }
    }
}

impl<I: VarValue, O: VarValue, S: Var<I>> AnyVar for MapRef<I, O, S> {
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
        let me: BoxedVar<O> = self;
        Box::new(me)
    }

    fn var_type_id(&self) -> TypeId {
        TypeId::of::<O>()
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

    fn set_any(&self, _: Box<dyn AnyVarValue>) -> Result<(), VarIsReadOnlyError> {
        Err(VarIsReadOnlyError {
            capabilities: self.capabilities(),
        })
    }

    fn last_update(&self) -> VarUpdateId {
        self.source.last_update()
    }

    fn is_contextual(&self) -> bool {
        self.source.is_contextual()
    }

    fn capabilities(&self) -> VarCapability {
        self.source.capabilities().as_read_only()
    }

    fn hook_any(&self, pos_modify_action: Box<dyn Fn(&AnyVarHookArgs) -> bool + Send + Sync>) -> VarHandle {
        let map = self.map.clone();
        self.source.hook_any(Box::new(move |args| {
            if let Some(value) = args.downcast_value() {
                let value = map(value);
                pos_modify_action(&AnyVarHookArgs::new(value, args.update(), args.tags()))
            } else {
                true
            }
        }))
    }

    fn hook_animation_stop(&self, handler: Box<dyn FnOnce() + Send>) -> Result<(), Box<dyn FnOnce() + Send>> {
        self.source.hook_animation_stop(handler)
    }

    fn strong_count(&self) -> usize {
        self.source.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.source.weak_count()
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        Box::new(self.clone().actual_var())
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        Box::new(self.downgrade())
    }

    fn is_animating(&self) -> bool {
        self.source.is_animating()
    }

    fn modify_importance(&self) -> usize {
        self.source.modify_importance()
    }

    fn var_ptr(&self) -> VarPtr<'_> {
        self.source.var_ptr()
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
impl<I: VarValue, O: VarValue, S: WeakVar<I>> AnyWeakVar for WeakMapRef<I, O, S> {
    fn clone_any(&self) -> BoxedAnyWeakVar {
        Box::new(self.clone())
    }

    fn strong_count(&self) -> usize {
        self.source.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.source.weak_count()
    }

    fn upgrade_any(&self) -> Option<BoxedAnyVar> {
        self.upgrade().map(|m| Box::new(m) as _)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<I: VarValue, O: VarValue, S: Var<I>> IntoVar<O> for MapRef<I, O, S> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<I: VarValue, O: VarValue, S: Var<I>> Var<O> for MapRef<I, O, S> {
    type ReadOnly = Self;

    type ActualVar = MapRef<I, O, S::ActualVar>;

    type Downgrade = WeakMapRef<I, O, S::Downgrade>;

    type Map<MO: VarValue> = BoxedVar<MO>;
    type MapBidi<MO: VarValue> = BoxedVar<MO>;

    type FlatMap<OF: VarValue, V: Var<OF>> = BoxedVar<OF>;

    type FilterMap<OF: VarValue> = BoxedVar<OF>;
    type FilterMapBidi<OF: VarValue> = BoxedVar<OF>;

    type MapRef<OM: VarValue> = types::MapRef<O, OM, Self>;
    type MapRefBidi<OM: VarValue> = types::MapRef<O, OM, Self>;

    type Easing = BoxedVar<O>;

    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&O) -> R,
    {
        self.source.with(|val| {
            let val = (self.map)(val);
            read(val)
        })
    }

    fn modify<F>(&self, _: F) -> Result<(), VarIsReadOnlyError>
    where
        F: FnOnce(&mut VarModify<O>) + 'static,
    {
        Err(VarIsReadOnlyError {
            capabilities: self.capabilities(),
        })
    }

    fn actual_var(self) -> Self::ActualVar {
        MapRef {
            source: self.source.actual_var(),
            map: self.map,
        }
    }

    fn downgrade(&self) -> Self::Downgrade {
        WeakMapRef {
            source: self.source.downgrade(),
            map: self.map.clone(),
        }
    }

    fn into_value(self) -> O {
        self.get()
    }

    fn read_only(&self) -> Self::ReadOnly {
        self.clone()
    }

    fn map<MO, M>(&self, map: M) -> Self::Map<MO>
    where
        MO: VarValue,
        M: FnMut(&O) -> MO + Send + 'static,
    {
        var_map_mixed(self, map)
    }

    fn map_bidi<MO, M, B>(&self, map: M, map_back: B) -> Self::MapBidi<MO>
    where
        MO: VarValue,
        M: FnMut(&O) -> MO + Send + 'static,
        B: FnMut(&MO) -> O + Send + 'static,
    {
        var_map_bidi_mixed(self, map, map_back)
    }

    fn flat_map<OF, V, M>(&self, map: M) -> Self::FlatMap<OF, V>
    where
        OF: VarValue,
        V: Var<OF>,
        M: FnMut(&O) -> V + Send + 'static,
    {
        var_flat_map_mixed(self, map)
    }

    fn filter_map<OF, M, IF>(&self, map: M, fallback: IF) -> Self::FilterMap<OF>
    where
        OF: VarValue,
        M: FnMut(&O) -> Option<OF> + Send + 'static,
        IF: Fn() -> OF + Send + Sync + 'static,
    {
        var_filter_map_mixed(self, map, fallback)
    }

    fn filter_map_bidi<OF, M, B, IF>(&self, map: M, map_back: B, fallback: IF) -> Self::FilterMapBidi<OF>
    where
        OF: VarValue,
        M: FnMut(&O) -> Option<OF> + Send + 'static,
        B: FnMut(&OF) -> Option<O> + Send + 'static,
        IF: Fn() -> OF + Send + Sync + 'static,
    {
        var_filter_map_bidi_mixed(self, map, map_back, fallback)
    }

    fn map_ref<OM, M>(&self, map: M) -> Self::MapRef<OM>
    where
        OM: VarValue,
        M: Fn(&O) -> &OM + Send + Sync + 'static,
    {
        var_map_ref(self, map)
    }

    fn map_ref_bidi<OM, M, B>(&self, map: M, _: B) -> Self::MapRefBidi<OM>
    where
        OM: VarValue,
        M: Fn(&O) -> &OM + Send + Sync + 'static,
        B: Fn(&mut O) -> &mut OM + Send + Sync + 'static,
    {
        var_map_ref(self, map)
    }

    fn easing<F>(&self, duration: Duration, easing: F) -> Self::Easing
    where
        O: Transitionable,
        F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
    {
        var_easing_mixed(self, duration, easing)
    }

    fn easing_with<F, SE>(&self, duration: Duration, easing: F, sampler: SE) -> Self::Easing
    where
        O: Transitionable,
        F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
        SE: Fn(&animation::Transition<O>, EasingStep) -> O + Send + Sync + 'static,
    {
        var_easing_with_mixed(self, duration, easing, sampler)
    }
}

impl<I: VarValue, O: VarValue, S: WeakVar<I>> WeakVar<O> for WeakMapRef<I, O, S> {
    type Upgrade = MapRef<I, O, S::Upgrade>;

    fn upgrade(&self) -> Option<Self::Upgrade> {
        self.source.upgrade().map(|s| MapRef {
            source: s,
            map: self.map.clone(),
        })
    }
}

/// See [`Var::map_ref_bidi`].
pub struct MapRefBidi<I, O, S> {
    source: S,
    map: Arc<dyn Fn(&I) -> &O + Send + Sync>,
    map_mut: Arc<dyn Fn(&mut I) -> &mut O + Send + Sync>,
}

/// Weak var that can upgrade to [`MapRefBidi<I, O, S>`] if the source is not dropped.
pub struct WeakMapRefBidi<I, O, S> {
    source: S,
    map: Arc<dyn Fn(&I) -> &O + Send + Sync>,
    map_mut: Arc<dyn Fn(&mut I) -> &mut O + Send + Sync>,
}

impl<I: VarValue, O: VarValue, S: Var<I>> MapRefBidi<I, O, S> {
    pub(super) fn new(source: S, map: Arc<dyn Fn(&I) -> &O + Send + Sync>, map_mut: Arc<dyn Fn(&mut I) -> &mut O + Send + Sync>) -> Self {
        MapRefBidi { source, map, map_mut }
    }
}

impl<I: VarValue, O: VarValue, S: Var<I>> crate::private::Sealed for MapRefBidi<I, O, S> {}
impl<I: VarValue, O: VarValue, S: WeakVar<I>> crate::private::Sealed for WeakMapRefBidi<I, O, S> {}

impl<I: VarValue, O: VarValue, S: Var<I>> Clone for MapRefBidi<I, O, S> {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            map: self.map.clone(),
            map_mut: self.map_mut.clone(),
        }
    }
}
impl<I: VarValue, O: VarValue, S: WeakVar<I>> Clone for WeakMapRefBidi<I, O, S> {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            map: self.map.clone(),
            map_mut: self.map_mut.clone(),
        }
    }
}

impl<I: VarValue, O: VarValue, S: Var<I>> AnyVar for MapRefBidi<I, O, S> {
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
        let me: BoxedVar<O> = self;
        Box::new(me)
    }

    fn var_type_id(&self) -> TypeId {
        TypeId::of::<O>()
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
        self.modify(var_set_any(value))
    }

    fn last_update(&self) -> VarUpdateId {
        self.source.last_update()
    }

    fn is_contextual(&self) -> bool {
        self.source.is_contextual()
    }

    fn capabilities(&self) -> VarCapability {
        self.source.capabilities()
    }

    fn hook_any(&self, pos_modify_action: Box<dyn Fn(&AnyVarHookArgs) -> bool + Send + Sync>) -> VarHandle {
        let map = self.map.clone();
        self.source.hook_any(Box::new(move |args| {
            if let Some(value) = args.downcast_value() {
                let value = map(value);
                pos_modify_action(&AnyVarHookArgs::new(value, args.update(), args.tags()))
            } else {
                true
            }
        }))
    }

    fn hook_animation_stop(&self, handler: Box<dyn FnOnce() + Send>) -> Result<(), Box<dyn FnOnce() + Send>> {
        self.source.hook_animation_stop(handler)
    }

    fn strong_count(&self) -> usize {
        self.source.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.source.weak_count()
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        Box::new(self.clone().actual_var())
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        Box::new(self.downgrade())
    }

    fn is_animating(&self) -> bool {
        self.source.is_animating()
    }

    fn modify_importance(&self) -> usize {
        self.source.modify_importance()
    }

    fn var_ptr(&self) -> VarPtr<'_> {
        self.source.var_ptr()
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
impl<I: VarValue, O: VarValue, S: WeakVar<I>> AnyWeakVar for WeakMapRefBidi<I, O, S> {
    fn clone_any(&self) -> BoxedAnyWeakVar {
        Box::new(self.clone())
    }

    fn strong_count(&self) -> usize {
        self.source.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.source.weak_count()
    }

    fn upgrade_any(&self) -> Option<BoxedAnyVar> {
        self.upgrade().map(|m| Box::new(m) as _)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<I: VarValue, O: VarValue, S: Var<I>> IntoVar<O> for MapRefBidi<I, O, S> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<I: VarValue, O: VarValue, S: Var<I>> Var<O> for MapRefBidi<I, O, S> {
    type ReadOnly = Self;

    type ActualVar = MapRefBidi<I, O, S::ActualVar>;

    type Downgrade = WeakMapRefBidi<I, O, S::Downgrade>;

    type Map<MO: VarValue> = contextualized::ContextualizedVar<MO>;
    type MapBidi<MO: VarValue> = contextualized::ContextualizedVar<MO>;

    type FlatMap<OF: VarValue, V: Var<OF>> = contextualized::ContextualizedVar<OF>;

    type FilterMap<OF: VarValue> = contextualized::ContextualizedVar<OF>;
    type FilterMapBidi<OF: VarValue> = contextualized::ContextualizedVar<OF>;

    type MapRef<OM: VarValue> = types::MapRef<O, OM, Self>;
    type MapRefBidi<OM: VarValue> = types::MapRefBidi<O, OM, Self>;

    type Easing = types::ContextualizedVar<O>;

    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&O) -> R,
    {
        self.source.with(|val| {
            let val = (self.map)(val);
            read(val)
        })
    }

    fn modify<F>(&self, modify: F) -> Result<(), VarIsReadOnlyError>
    where
        F: FnOnce(&mut VarModify<O>) + Send + 'static,
    {
        let map = self.map.clone();
        let map_mut = self.map_mut.clone();
        self.source.modify(move |vm| {
            let (notify, new_value, update, tags, custom_importance) = {
                let mut vm = VarModify::new(map(vm.as_ref()));
                modify(&mut vm);
                vm.finish()
            };
            if let Some(i) = custom_importance {
                vm.set_modify_importance(i);
            }
            if notify {
                if update {
                    vm.update();
                }
                if let Some(nv) = new_value {
                    *map_mut(vm.to_mut()) = nv;
                }
                vm.push_tags(tags);
            }
        })
    }

    fn actual_var(self) -> Self::ActualVar {
        MapRefBidi {
            source: self.source.actual_var(),
            map: self.map,
            map_mut: self.map_mut,
        }
    }

    fn downgrade(&self) -> Self::Downgrade {
        WeakMapRefBidi {
            source: self.source.downgrade(),
            map: self.map.clone(),
            map_mut: self.map_mut.clone(),
        }
    }

    fn into_value(self) -> O {
        self.get()
    }

    fn read_only(&self) -> Self::ReadOnly {
        self.clone()
    }

    fn map<MO, M>(&self, map: M) -> Self::Map<MO>
    where
        MO: VarValue,
        M: FnMut(&O) -> MO + Send + 'static,
    {
        var_map_ctx(self, map)
    }

    fn map_bidi<MO, M, B>(&self, map: M, map_back: B) -> Self::MapBidi<MO>
    where
        MO: VarValue,
        M: FnMut(&O) -> MO + Send + 'static,
        B: FnMut(&MO) -> O + Send + 'static,
    {
        var_map_bidi_ctx(self, map, map_back)
    }

    fn flat_map<OF, V, M>(&self, map: M) -> Self::FlatMap<OF, V>
    where
        OF: VarValue,
        V: Var<OF>,
        M: FnMut(&O) -> V + Send + 'static,
    {
        var_flat_map_ctx(self, map)
    }

    fn filter_map<OF, M, IF>(&self, map: M, fallback: IF) -> Self::FilterMap<OF>
    where
        OF: VarValue,
        M: FnMut(&O) -> Option<OF> + Send + 'static,
        IF: Fn() -> OF + Send + Sync + 'static,
    {
        var_filter_map_ctx(self, map, fallback)
    }

    fn filter_map_bidi<OF, M, B, IF>(&self, map: M, map_back: B, fallback: IF) -> Self::FilterMapBidi<OF>
    where
        OF: VarValue,
        M: FnMut(&O) -> Option<OF> + Send + 'static,
        B: FnMut(&OF) -> Option<O> + Send + 'static,
        IF: Fn() -> OF + Send + Sync + 'static,
    {
        var_filter_map_bidi_ctx(self, map, map_back, fallback)
    }

    fn map_ref<OM, M>(&self, map: M) -> Self::MapRef<OM>
    where
        OM: VarValue,
        M: Fn(&O) -> &OM + Send + Sync + 'static,
    {
        var_map_ref(self, map)
    }

    fn map_ref_bidi<OM, M, B>(&self, map: M, map_mut: B) -> Self::MapRefBidi<OM>
    where
        OM: VarValue,
        M: Fn(&O) -> &OM + Send + Sync + 'static,
        B: Fn(&mut O) -> &mut OM + Send + Sync + 'static,
    {
        var_map_ref_bidi(self, map, map_mut)
    }

    fn easing<F>(&self, duration: Duration, easing: F) -> Self::Easing
    where
        O: Transitionable,
        F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
    {
        var_easing_ctx(self, duration, easing)
    }

    fn easing_with<F, SA>(&self, duration: Duration, easing: F, sampler: SA) -> Self::Easing
    where
        O: Transitionable,
        F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
        SA: Fn(&animation::Transition<O>, EasingStep) -> O + Send + Sync + 'static,
    {
        var_easing_with_ctx(self, duration, easing, sampler)
    }
}

impl<I: VarValue, O: VarValue, S: WeakVar<I>> WeakVar<O> for WeakMapRefBidi<I, O, S> {
    type Upgrade = MapRefBidi<I, O, S::Upgrade>;

    fn upgrade(&self) -> Option<Self::Upgrade> {
        self.source.upgrade().map(|s| MapRefBidi {
            source: s,
            map: self.map.clone(),
            map_mut: self.map_mut.clone(),
        })
    }
}
