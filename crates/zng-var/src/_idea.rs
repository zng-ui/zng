/*
!!: IDEA

* Have all variables have a type erased core.
* Var<T>: Deref<AnyVar> (the core).
    - Deref so we don't endup with huge dyn tables for all Var<T>: AnyVar.
* Implement everything type erased, map, animation.
* Var<T> is just a PhantomData<T> wrapper that casts for the user.

* LocalVar<T> could implement AnyVar directly, but when boxed have a LocalVarAny core too?
* Is there any actual bloat from dyn tables? Maybe try just the Any core for now, add a modify_any (internal for this release) and profile.

*/

use std::{marker::PhantomData, sync::Arc};

use parking_lot::RwLock;
use zng_txt::formatx;

use crate::{animation::ModifyInfo, private, AnyVar, Var, VarCapability, VarHandle, VarHook, VarPtr, VarUpdateId, VarValue};

pub struct ArcVar<T: VarValue>(Arc<RwLock<ArcVarInner>>, PhantomData<T>);

struct ArcVarInner {
    value: Box<dyn crate::AnyVarValue>,
    last_update: VarUpdateId,
    hooks: Vec<VarHook>,
    animation: ModifyInfo,
}

impl<T: VarValue> ArcVar<T> {
    pub fn new_data(value: Box<dyn crate::AnyVarValue>) -> Arc<RwLock<ArcVarInner>> {
        Arc::new(RwLock::new(ArcVarInner {
            value,
            last_update: VarUpdateId::never(),
                hooks: vec![],
                animation: ModifyInfo::never(),
        }))
    }
}

impl<T: VarValue> private::Sealed for ArcVar<T> { }
impl<T: VarValue> Clone for ArcVar<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<T: VarValue> AnyVar for ArcVar<T> {
    fn clone_any(&self) -> crate::BoxedAnyVar {
        Box::new(self.clone())
    }

    // Limitation, can't cast back to ArcVar<T>
    // 
    // * Maybe just this methods can be a separate trait
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_unboxed_any(&self) -> &dyn std::any::Any {
        self
    }

    fn double_boxed_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }

    fn var_type_id(&self) -> std::any::TypeId {
        self.0.read().value.type_id()
    }

    fn get_any(&self) -> Box<dyn crate::AnyVarValue> {
        self.0.read().value.clone_boxed()
    }

    fn with_any(&self, read: &mut dyn FnMut(&dyn crate::AnyVarValue)) {
        read(&*self.0.read().value)
    }

    fn with_new_any(&self, read: &mut dyn FnMut(&dyn crate::AnyVarValue)) -> bool {
        if self.is_new() {
            read(&*self.0.read().value);
            return true;
        }
        false
    }

    fn set_any(&self, value: Box<dyn crate::AnyVarValue>) -> Result<(), crate::VarIsReadOnlyError> {
        todo!()
    }

    fn last_update(&self) -> VarUpdateId {
        self.0.read().last_update
    }

    fn is_contextual(&self) -> bool {
        false
    }

    fn capabilities(&self) -> VarCapability {
        VarCapability::MODIFY
    }

    fn is_animating(&self) -> bool {
        self.0.read().animation.is_animating()
    }

    fn modify_importance(&self) -> usize {
        self.0.read().animation.importance()
    }

    fn hook_any(&self, pos_modify_action: Box<dyn Fn(&crate::AnyVarHookArgs) -> bool + Send + Sync>) -> VarHandle {
        let (handle, hook) = VarHandle::new(pos_modify_action);
        self.0.write().hooks.push(hook);
        handle
    }

    fn hook_animation_stop(&self, handler: Box<dyn FnOnce() + Send>) -> Result<(), Box<dyn FnOnce() + Send>> {
        self.0.read().animation.hook_animation_stop(handler)
    }

    fn strong_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }

    fn weak_count(&self) -> usize {
        Arc::weak_count(&self.0)
    }

    fn actual_var_any(&self) -> crate::BoxedAnyVar {
        self.clone_any()
    }

    fn downgrade_any(&self) -> crate::BoxedAnyWeakVar {
        todo!()
    }

    fn var_ptr(&self) -> VarPtr {
        VarPtr::new_arc(&self.0)
    }

    fn get_debug(&self) -> zng_txt::Txt {
        formatx!("{:?}", self.0.read().value)
    }

    fn update(&self) -> Result<(), crate::VarIsReadOnlyError> {
        todo!()
    }

    fn map_debug(&self) -> crate::BoxedVar<zng_txt::Txt> {
        todo!()
    }
    
    fn is_new(&self) -> bool {
        crate::VARS.update_id() == self.last_update()
    }
    
    fn perm(&self) {
        crate::VARS.perm(self.clone_any());
    }
    
    fn hold_any(&self, value: Box<dyn std::any::Any + Send + Sync>) -> VarHandle {
        self.hook_any(Box::new(move |_| {
            let _hold = &value;
            true
        }))
    }
}

impl<T: VarValue> Var<T> for ArcVar<T> {
    type ReadOnly;

    type ActualVar;

    type Downgrade;

    type Map<O: VarValue>;

    type MapBidi<O: VarValue>;

    type FlatMap<O: VarValue, V: Var<O>>;

    type FilterMap<O: VarValue>;

    type FilterMapBidi<O: VarValue>;

    type MapRef<O: VarValue>;

    type MapRefBidi<O: VarValue>;

    type Easing;

    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&T) -> R {
        todo!()
    }

    fn modify<F>(&self, modify: F) -> Result<(), crate::VarIsReadOnlyError>
    where
        F: FnOnce(&mut crate::VarModify<T>) + Send + 'static {
        todo!()
    }

    fn actual_var(self) -> Self::ActualVar {
        todo!()
    }

    fn downgrade(&self) -> Self::Downgrade {
        todo!()
    }

    fn into_value(self) -> T {
        todo!()
    }

    fn read_only(&self) -> Self::ReadOnly {
        todo!()
    }

    fn map<O, M>(&self, map: M) -> Self::Map<O>
    where
        O: VarValue,
        M: FnMut(&T) -> O + Send + 'static {
        todo!()
    }

    fn map_bidi<O, M, B>(&self, map: M, map_back: B) -> Self::MapBidi<O>
    where
        O: VarValue,
        M: FnMut(&T) -> O + Send + 'static,
        B: FnMut(&O) -> T + Send + 'static {
        todo!()
    }

    fn flat_map<O, V, M>(&self, map: M) -> Self::FlatMap<O, V>
    where
        O: VarValue,
        V: Var<O>,
        M: FnMut(&T) -> V + Send + 'static {
        todo!()
    }

    fn filter_map<O, M, I>(&self, map: M, fallback: I) -> Self::FilterMap<O>
    where
        O: VarValue,
        M: FnMut(&T) -> Option<O> + Send + 'static,
        I: Fn() -> O + Send + Sync + 'static {
        todo!()
    }

    fn filter_map_bidi<O, M, B, I>(&self, map: M, map_back: B, fallback: I) -> Self::FilterMapBidi<O>
    where
        O: VarValue,
        M: FnMut(&T) -> Option<O> + Send + 'static,
        B: FnMut(&O) -> Option<T> + Send + 'static,
        I: Fn() -> O + Send + Sync + 'static {
        todo!()
    }

    fn map_ref<O, M>(&self, map: M) -> Self::MapRef<O>
    where
        O: VarValue,
        M: Fn(&T) -> &O + Send + Sync + 'static {
        todo!()
    }

    fn map_ref_bidi<O, M, B>(&self, map: M, map_mut: B) -> Self::MapRefBidi<O>
    where
        O: VarValue,
        M: Fn(&T) -> &O + Send + Sync + 'static,
        B: Fn(&mut T) -> &mut O + Send + Sync + 'static {
        todo!()
    }

    fn easing<F>(&self, duration: std::time::Duration, easing: F) -> Self::Easing
    where
        T: crate::animation::Transitionable,
        F: Fn(crate::animation::easing::EasingTime) -> crate::animation::easing::EasingStep + Send + Sync + 'static {
        todo!()
    }

    fn easing_with<F, S>(&self, duration: std::time::Duration, easing: F, sampler: S) -> Self::Easing
    where
        T: crate::animation::Transitionable,
        F: Fn(crate::animation::easing::EasingTime) -> crate::animation::easing::EasingStep + Send + Sync + 'static,
        S: Fn(&crate::animation::Transition<T>, crate::animation::easing::EasingStep) -> T + Send + Sync + 'static {
        todo!()
    }
}