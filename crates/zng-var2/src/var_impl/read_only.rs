//! Wrapper var that enforces read-only.

use crate::{VarAny, WeakVarAny};

use super::*;

pub(crate) struct ReadOnlyVar(pub VarAny);
impl VarImpl for ReadOnlyVar {
    fn clone_boxed(&self) -> SmallBox<dyn VarImpl, smallbox::space::S2> {
        smallbox!(Self(self.0.clone()))
    }

    fn current_context(&self) -> SmallBox<dyn VarImpl, smallbox::space::S2> {
        self.clone_boxed()
    }

    fn value_type(&self) -> TypeId {
        self.0.value_type()
    }

    #[cfg(feature = "value_type_name")]
    fn value_type_name(&self) -> &'static str {
        self.0.value_type_name()
    }

    fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    fn var_eq(&self, other: &dyn Any) -> bool {
        match other.downcast_ref::<ReadOnlyVar>() {
            Some(v) => self.0.var_eq(&v.0),
            None => false,
        }
    }

    fn var_instance_tag(&self) -> VarInstanceTag {
        self.0.var_instance_tag()
    }

    fn downgrade(&self) -> SmallBox<dyn WeakVarImpl, smallbox::space::S2> {
        smallbox!(WeakReadOnlyVar(self.0.downgrade()))
    }

    fn capabilities(&self) -> VarCapability {
        self.0.capabilities().as_read_only()
    }

    fn with(&self, visitor: &mut dyn FnMut(&dyn VarValueAny)) {
        self.0.0.with(visitor);
    }

    fn get(&self) -> BoxedVarValueAny {
        self.0.get()
    }

    fn set(&self, _: BoxedVarValueAny) -> bool {
        false
    }

    fn update(&self) -> bool {
        false
    }

    fn modify(&self, _: SmallBox<dyn FnMut(&mut VarModifyAny) + Send + 'static, smallbox::space::S4>) -> bool {
        false
    }

    fn hook(&self, on_new: SmallBox<dyn FnMut(&VarAnyHookArgs) -> bool + Send + 'static, smallbox::space::S4>) -> VarHandle {
        self.0.0.hook(on_new)
    }

    fn last_update(&self) -> VarUpdateId {
        self.0.0.last_update()
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

struct WeakReadOnlyVar(WeakVarAny);

impl WeakVarImpl for WeakReadOnlyVar {
    fn clone_boxed(&self) -> SmallBox<dyn WeakVarImpl, smallbox::space::S2> {
        smallbox!(Self(self.0.clone()))
    }

    fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    fn upgrade(&self) -> Option<SmallBox<dyn VarImpl, smallbox::space::S2>> {
        self.0.upgrade().map(|v| {
            let r: SmallBox<dyn VarImpl, smallbox::space::S2> = smallbox!(ReadOnlyVar(v));
            r
        })
    }
}
