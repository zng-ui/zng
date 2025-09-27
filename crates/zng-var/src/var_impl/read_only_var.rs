//! Read-only wrapper.

use std::{
    any::{Any, TypeId},
    fmt,
};

use smallbox::SmallBox;

use crate::{
    AnyVarHookArgs, AnyVarModify, AnyVarValue, BoxAnyVarValue, DynAnyVar, DynWeakAnyVar, VarCapability, VarHandle, VarImpl, VarInstanceTag,
    VarUpdateId, WeakVarImpl,
    animation::{AnimationStopFn, ModifyInfo},
};

impl DynAnyVar {
    pub(crate) fn into_read_only(self) -> Self {
        match self {
            DynAnyVar::Shared(v) => DynAnyVar::ReadOnlyShared(ReadOnlyImpl(v)),
            DynAnyVar::Context(v) => DynAnyVar::ReadOnlyContext(ReadOnlyImpl(v)),
            DynAnyVar::Cow(v) => DynAnyVar::ReadOnlyCow(ReadOnlyImpl(v)),
            DynAnyVar::Contextual(v) => DynAnyVar::ReadOnlyContextual(ReadOnlyImpl(v)),
            DynAnyVar::FlatMap(v) => DynAnyVar::ReadOnlyFlatMap(ReadOnlyImpl(v)),
            r => r,
        }
    }
}
impl DynWeakAnyVar {
    fn into_read_only(self) -> Self {
        match self {
            DynWeakAnyVar::Shared(v) => DynWeakAnyVar::ReadOnlyShared(ReadOnlyImpl(v)),
            DynWeakAnyVar::Context(v) => DynWeakAnyVar::ReadOnlyContext(ReadOnlyImpl(v)),
            DynWeakAnyVar::Cow(v) => DynWeakAnyVar::ReadOnlyCow(ReadOnlyImpl(v)),
            DynWeakAnyVar::Contextual(v) => DynWeakAnyVar::ReadOnlyContextual(ReadOnlyImpl(v)),
            DynWeakAnyVar::FlatMap(v) => DynWeakAnyVar::ReadOnlyFlatMap(ReadOnlyImpl(v)),
            r => r,
        }
    }
}

pub(crate) struct ReadOnlyImpl<V>(V);
impl<V: fmt::Debug> fmt::Debug for ReadOnlyImpl<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}
impl<V: VarImpl + PartialEq + Any> VarImpl for ReadOnlyImpl<V> {
    fn clone_dyn(&self) -> DynAnyVar {
        self.0.clone_dyn().into_read_only()
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
        #[inline(always)]
        fn eq_any<V: VarImpl + PartialEq + Any>(self_: &dyn Any, other: &V) -> bool {
            match self_.downcast_ref::<V>() {
                Some(v) => v == other,
                None => false,
            }
        }
        match other {
            DynAnyVar::ReadOnlyShared(v) => eq_any(&self.0, &v.0),
            DynAnyVar::ReadOnlyFlatMap(v) => eq_any(&self.0, &v.0),
            DynAnyVar::ReadOnlyContext(v) => eq_any(&self.0, &v.0),
            DynAnyVar::ReadOnlyCow(v) => eq_any(&self.0, &v.0),
            DynAnyVar::ReadOnlyContextual(v) => eq_any(&self.0, &v.0),
            _ => false,
        }
    }

    fn var_instance_tag(&self) -> VarInstanceTag {
        self.0.var_instance_tag()
    }

    fn downgrade(&self) -> DynWeakAnyVar {
        self.0.downgrade().into_read_only()
    }

    fn capabilities(&self) -> VarCapability {
        self.0.capabilities().as_always_read_only()
    }

    fn with(&self, visitor: &mut dyn FnMut(&dyn AnyVarValue)) {
        self.0.with(visitor);
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
        self.0.hook(on_new)
    }

    fn last_update(&self) -> VarUpdateId {
        self.0.last_update()
    }

    fn modify_importance(&self) -> usize {
        self.0.modify_importance()
    }

    fn is_animating(&self) -> bool {
        self.0.is_animating()
    }

    fn hook_animation_stop(&self, handler: AnimationStopFn) -> VarHandle {
        self.0.hook_animation_stop(handler)
    }

    fn current_context(&self) -> DynAnyVar {
        self.0.current_context().into_read_only()
    }

    fn modify_info(&self) -> ModifyInfo {
        self.0.modify_info()
    }
}

impl<V: WeakVarImpl> WeakVarImpl for ReadOnlyImpl<V> {
    fn clone_dyn(&self) -> DynWeakAnyVar {
        self.0.clone_dyn().into_read_only()
    }

    fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    fn upgrade(&self) -> Option<DynAnyVar> {
        Some(self.0.upgrade()?.into_read_only())
    }
}
