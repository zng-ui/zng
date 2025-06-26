use std::{
    mem,
    sync::{Arc, Weak},
};

use parking_lot::RwLock;

use super::{animation::ModifyInfo, *};

enum Data<T: VarValue, S> {
    Source {
        source: S,
        source_handle: VarHandle,
        hooks: Vec<VarHook>,
    },
    Owned {
        value: T,
        last_update: VarUpdateId,
        hooks: Vec<VarHook>,
        animation: ModifyInfo,
    },
}

/// See [`Var::cow`].
pub struct ArcCowVar<T: VarValue, S>(Arc<RwLock<Data<T, S>>>);

/// Weak reference to a [`ArcCowVar<T>`].
pub struct WeakCowVar<T: VarValue, S>(Weak<RwLock<Data<T, S>>>);

impl<T: VarValue, S: Var<T>> ArcCowVar<T, S> {
    pub(super) fn new(source: S) -> Self {
        let cow = Arc::new(RwLock::new(Data::Source {
            source,
            source_handle: VarHandle::dummy(),
            hooks: vec![],
        }));
        {
            let mut data = cow.write();
            if let Data::Source { source, source_handle, .. } = &mut *data {
                let weak_cow = Arc::downgrade(&cow);
                *source_handle = source.hook_any(Box::new(move |value| {
                    if let Some(cow) = weak_cow.upgrade() {
                        match &mut *cow.write() {
                            Data::Source { hooks, .. } => {
                                hooks.retain(|h| h.call(value));
                                true
                            }
                            Data::Owned { .. } => false,
                        }
                    } else {
                        false
                    }
                }));
            }
        }
        Self(cow)
    }

    fn modify_impl(&self, modify: impl FnOnce(&mut VarModify<T>) + Send + 'static) -> Result<(), VarIsReadOnlyError> {
        let me = self.clone();
        VARS.schedule_update(
            Box::new(move || {
                let mut data = me.0.write();
                let data = &mut *data;

                match data {
                    Data::Source { source, hooks, .. } => {
                        let (notify, new_value, update, tags, custom_importance) = source.with(|val| {
                            let mut vm = VarModify::new(val);
                            modify(&mut vm);
                            vm.finish()
                        });
                        let value = new_value.unwrap_or_else(|| source.get());
                        if notify {
                            let hook_args = AnyVarHookArgs::new(&value, update, &tags);
                            hooks.retain(|h| h.call(&hook_args));
                            VARS.wake_app();
                        }
                        let mut modify = VARS.current_modify();
                        if let Some(i) = custom_importance {
                            modify.importance = i;
                        }
                        *data = Data::Owned {
                            value,
                            last_update: if notify { VARS.update_id() } else { source.last_update() },
                            hooks: mem::take(hooks),
                            animation: modify,
                        };
                    }
                    Data::Owned {
                        value,
                        last_update,
                        hooks,
                        animation,
                    } => {
                        {
                            let cur_anim = VARS.current_modify();
                            if cur_anim.importance() < animation.importance() {
                                return;
                            }
                            *animation = cur_anim;
                        }

                        let (notify, new_value, update, tags, custom_importance) = {
                            let mut vm = VarModify::new(value);
                            modify(&mut vm);
                            vm.finish()
                        };

                        if let Some(i) = custom_importance {
                            animation.importance = i;
                        }

                        if notify {
                            if let Some(nv) = new_value {
                                *value = nv;
                            }
                            *last_update = VARS.update_id();
                            let hook_args = AnyVarHookArgs::new(value, update, &tags);
                            hooks.retain(|h| h.call(&hook_args));
                            VARS.wake_app();
                        }
                    }
                }
            }),
            std::any::type_name::<T>(),
        );
        Ok(())
    }

    impl_infallible_write! {
        for<T>
    }
}

impl<T: VarValue, S> Clone for ArcCowVar<T, S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T: VarValue, S> Clone for WeakCowVar<T, S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: VarValue, S: Var<T>> crate::private::Sealed for ArcCowVar<T, S> {}
impl<T: VarValue, S: Var<T>> crate::private::Sealed for WeakCowVar<T, S> {}

impl<T: VarValue, S: Var<T>> AnyVar for ArcCowVar<T, S> {
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
        match &*self.0.read_recursive() {
            Data::Source { source, .. } => source.last_update(),
            Data::Owned { last_update, .. } => *last_update,
        }
    }

    fn is_contextual(&self) -> bool {
        match &*self.0.read_recursive() {
            Data::Source { source, .. } => source.is_contextual(),
            Data::Owned { .. } => false,
        }
    }

    fn capabilities(&self) -> VarCapability {
        VarCapability::MODIFY
    }

    fn hook_any(&self, pos_modify_action: Box<dyn Fn(&AnyVarHookArgs) -> bool + Send + Sync>) -> VarHandle {
        let mut data = self.0.write();
        match &mut *data {
            Data::Source { hooks, .. } => {
                let (hook, weak) = VarHandle::new(pos_modify_action);
                hooks.push(weak);
                hook
            }
            Data::Owned { hooks, .. } => {
                let (hook, weak) = VarHandle::new(pos_modify_action);
                hooks.push(weak);
                hook
            }
        }
    }

    fn hook_animation_stop(&self, handler: Box<dyn FnOnce() + Send>) -> Result<(), Box<dyn FnOnce() + Send>> {
        match &*self.0.read_recursive() {
            Data::Source { source, .. } => source.hook_animation_stop(handler),
            Data::Owned { animation, .. } => animation.hook_animation_stop(handler),
        }
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
        Box::new(WeakCowVar(Arc::downgrade(&self.0)))
    }

    fn is_animating(&self) -> bool {
        match &*self.0.read_recursive() {
            Data::Source { source, .. } => source.is_animating(),
            Data::Owned { animation, .. } => animation.is_animating(),
        }
    }

    fn modify_importance(&self) -> usize {
        match &*self.0.read_recursive() {
            Data::Source { source, .. } => source.modify_importance(),
            Data::Owned { animation, .. } => animation.importance(),
        }
    }

    fn var_ptr(&self) -> VarPtr<'_> {
        VarPtr::new_arc(&self.0)
    }

    fn get_debug(&self) -> crate::Txt {
        self.with(var_debug)
    }

    fn update(&self) -> Result<(), VarIsReadOnlyError> {
        Var::modify(self, var_update)
    }

    fn map_debug(&self) -> BoxedVar<Txt> {
        Var::map(self, var_debug).boxed()
    }
}

impl<T: VarValue, S: Var<T>> AnyWeakVar for WeakCowVar<T, S> {
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
        self.0.upgrade().map(|rc| Box::new(ArcCowVar(rc)) as _)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<T: VarValue, S: Var<T>> IntoVar<T> for ArcCowVar<T, S> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T: VarValue, S: Var<T>> Var<T> for ArcCowVar<T, S> {
    type ReadOnly = types::ReadOnlyVar<T, Self>;

    type ActualVar = Self;

    type Downgrade = WeakCowVar<T, S>;

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
        match &*self.0.read_recursive() {
            Data::Source { source, .. } => source.with(read),
            Data::Owned { value, .. } => read(value),
        }
    }

    fn modify<F>(&self, modify: F) -> Result<(), VarIsReadOnlyError>
    where
        F: FnOnce(&mut VarModify<T>) + Send + 'static,
    {
        self.modify_impl(modify)
    }

    fn actual_var(self) -> Self {
        self
    }

    fn downgrade(&self) -> Self::Downgrade {
        WeakCowVar(Arc::downgrade(&self.0))
    }

    fn into_value(self) -> T {
        match Arc::try_unwrap(self.0) {
            Ok(state) => match state.into_inner() {
                Data::Source { source, .. } => source.into_value(),
                Data::Owned { value, .. } => value,
            },
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

    fn easing_with<F, SE>(&self, duration: Duration, easing: F, sampler: SE) -> Self::Easing
    where
        T: Transitionable,
        F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
        SE: Fn(&animation::Transition<T>, EasingStep) -> T + Send + Sync + 'static,
    {
        var_easing_with(self, duration, easing, sampler)
    }
}

impl<T: VarValue, S: Var<T>> WeakVar<T> for WeakCowVar<T, S> {
    type Upgrade = ArcCowVar<T, S>;

    fn upgrade(&self) -> Option<Self::Upgrade> {
        self.0.upgrade().map(|rc| ArcCowVar(rc))
    }
}
