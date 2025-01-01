/*
IDEA

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

use crate::{animation::ModifyInfo, private, AnyVar, VarCapability, VarHandle, VarHook, VarUpdateId, VarValue};

pub struct ArcVarAny(Arc<RwLock<ArcVarInner>>);

pub struct ArcVar<T: VarValue>(ArcVarAny, PhantomData<T>);

struct ArcVarInner {
    value: Box<dyn crate::AnyVarValue>,
    last_update: VarUpdateId,
    hooks: Vec<VarHook>,
    animation: ModifyInfo,
}

impl ArcVarAny {
    pub fn new(value: Box<dyn crate::AnyVarValue>) -> Self {
        Self(Arc::new(RwLock::new(ArcVarInner {
            value,
            last_update: VarUpdateId::never(),
                hooks: vec![],
                animation: ModifyInfo::never(),
        })))
    }
}

impl private::Sealed for ArcVarAny { }

impl AnyVar for ArcVarAny {
    fn clone_any(&self) -> crate::BoxedAnyVar {
        todo!()
    }

    // Limitation, can't cast back to ArcVar<T>
    // 
    // * Maybe just this methods can be a separate trait
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_unboxed_any(&self) -> &dyn std::any::Any {
        todo!()
    }

    fn double_boxed_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        todo!()
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
        todo!()
    }

    fn downgrade_any(&self) -> crate::BoxedAnyWeakVar {
        todo!()
    }

    fn var_ptr(&self) -> crate::VarPtr {
        todo!()
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
}