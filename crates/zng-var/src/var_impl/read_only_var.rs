//! Wrapper var that enforces read-only.

use crate::{AnyVar, WeakAnyVar};

use super::*;

pub(crate) struct ReadOnlyVar(pub Box<AnyVar>); // TODO generic to avoid Box?

impl fmt::Debug for ReadOnlyVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ReadOnlyVar").field(&self.0.0).finish()
    }
}
impl VarImpl for ReadOnlyVar {
    fn clone_dyn(&self) -> DynAnyVar {
        DynAnyVar::ReadOnly(Self(self.0.clone()))
    }

    fn current_context(&self) -> DynAnyVar {
        DynAnyVar::ReadOnly(Self(Box::new(self.0.current_context())))
    }

    fn value_type(&self) -> TypeId {
        self.0.value_type()
    }

    #[cfg(feature = "type_names")]
    fn value_type_name(&self) -> &'static str {
        self.0.value_type_name()
    }

    fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    fn var_eq(&self, other: &DynAnyVar) -> bool {
        match other {
            DynAnyVar::ReadOnly(v) => self.0.var_eq(&v.0),
            _ => false,
        }
    }

    fn var_instance_tag(&self) -> VarInstanceTag {
        self.0.var_instance_tag()
    }

    fn downgrade(&self) -> DynWeakAnyVar {
        DynWeakAnyVar::ReadOnly(WeakReadOnlyVar(Box::new(self.0.downgrade())))
    }

    fn capabilities(&self) -> VarCapability {
        self.0.capabilities().as_always_read_only()
    }

    fn with(&self, visitor: &mut dyn FnMut(&dyn AnyVarValue)) {
        self.0.0.with(visitor);
    }

    fn get(&self) -> BoxAnyVarValue {
        self.0.get()
    }

    fn set(&self, _: BoxAnyVarValue) -> bool {
        false
    }

    fn update(&self) -> bool {
        false
    }

    fn modify(&self, _: SmallBox<dyn FnMut(&mut AnyVarModify) + Send + 'static, smallbox::space::S4>) -> bool {
        false
    }

    fn hook(&self, on_new: SmallBox<dyn FnMut(&AnyVarHookArgs) -> bool + Send + 'static, smallbox::space::S4>) -> VarHandle {
        self.0.0.hook(on_new)
    }

    fn last_update(&self) -> VarUpdateId {
        self.0.0.last_update()
    }

    fn modify_info(&self) -> ModifyInfo {
        self.0.0.modify_info()
    }

    fn modify_importance(&self) -> usize {
        self.0.0.modify_importance()
    }

    fn is_animating(&self) -> bool {
        self.0.0.is_animating()
    }

    fn hook_animation_stop(&self, handler: AnimationStopFn) -> Result<(), AnimationStopFn> {
        self.0.0.hook_animation_stop(handler)
    }
}

#[derive(Debug)]
pub(crate) struct WeakReadOnlyVar(Box<WeakAnyVar>);

impl WeakVarImpl for WeakReadOnlyVar {
    fn clone_dyn(&self) -> DynWeakAnyVar {
        DynWeakAnyVar::ReadOnly(Self(self.0.clone()))
    }

    fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    fn upgrade(&self) -> Option<DynAnyVar> {
        Some(DynAnyVar::ReadOnly(ReadOnlyVar(Box::new(self.0.upgrade()?))))
    }
}
