use std::sync::{Arc, Weak};

use crate::AnyVar;

use super::*;

type DerefFn = SmallBox<dyn for<'a> Fn(&'a dyn AnyVarValue) -> &'a (dyn AnyVarValue) + Send + Sync + 'static, smallbox::space::S4>;

struct VarData {
    source: AnyVar,
    deref: DerefFn,
    value_type: TypeId,
}

#[derive(Clone)]
pub(crate) struct MapRefVar(Arc<VarData>);
impl fmt::Debug for MapRefVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut b = f.debug_struct("MapRefVar");
        b.field("var_instance_tag()", &self.var_instance_tag());
        b.field("source", &self.0.source);
        b.finish()
    }
}
impl MapRefVar {
    pub(crate) fn new(source: AnyVar, deref: DerefFn, value_type: TypeId) -> Self {
        Self(Arc::new(VarData { source, deref, value_type }))
    }
}
impl VarImpl for MapRefVar {
    fn clone_boxed(&self) -> SmallBox<dyn VarImpl, smallbox::space::S2> {
        smallbox!(self.clone())
    }

    fn current_context(&self) -> SmallBox<dyn VarImpl, smallbox::space::S2> {
        self.clone_boxed()
    }

    fn value_type(&self) -> TypeId {
        self.0.value_type
    }

    #[cfg(feature = "type_names")]
    fn value_type_name(&self) -> &'static str {
        let mut r = "";
        self.with(&mut |v| r = v.type_name());
        r
    }

    fn strong_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }

    fn var_eq(&self, other: &dyn Any) -> bool {
        match other.downcast_ref::<Self>() {
            Some(o) => Arc::ptr_eq(&self.0, &o.0),
            None => false,
        }
    }

    fn var_instance_tag(&self) -> VarInstanceTag {
        VarInstanceTag(Arc::as_ptr(&self.0) as usize)
    }

    fn downgrade(&self) -> SmallBox<dyn WeakVarImpl, smallbox::space::S2> {
        smallbox!(WeakMapRefVar(Arc::downgrade(&self.0)))
    }

    fn capabilities(&self) -> VarCapability {
        self.0.source.capabilities().as_always_read_only() | VarCapability::SHARE
    }

    fn with(&self, visitor: &mut dyn FnMut(&dyn AnyVarValue)) {
        let deref = &*self.0.deref;
        self.0.source.with(&mut move |value: &dyn AnyVarValue| {
            let value = (deref)(value);
            debug_assert_eq!(self.0.value_type, value.type_id(), "map_ref_any value type does not match");
            visitor(value)
        });
    }

    fn get(&self) -> BoxAnyVarValue {
        let mut out = None;
        let deref = &*self.0.deref;
        self.0
            .source
            .with(&mut |value: &dyn AnyVarValue| out = Some((deref)(value).clone_boxed()));
        let out = out.unwrap();
        debug_assert_eq!(self.0.value_type, out.type_id(), "map_ref_any value type does not match");
        out
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

    fn hook(&self, mut on_new: SmallBox<dyn FnMut(&AnyVarHookArgs) -> bool + Send + 'static, smallbox::space::S4>) -> VarHandle {
        let weak = Arc::downgrade(&self.0);
        self.0.source.hook(move |args: &AnyVarHookArgs| {
            if let Some(s) = weak.upgrade() {
                on_new(&AnyVarHookArgs {
                    value: (s.deref)(args.value),
                    update: args.update,
                    tags: args.tags,
                });
                true
            } else {
                false
            }
        })
    }

    fn last_update(&self) -> VarUpdateId {
        self.0.source.last_update()
    }

    fn modify_info(&self) -> ModifyInfo {
        self.0.source.0.modify_info()
    }

    fn modify_importance(&self) -> usize {
        self.0.source.modify_importance()
    }

    fn is_animating(&self) -> bool {
        self.0.source.is_animating()
    }

    fn hook_animation_stop(&self, handler: AnimationStopFn) -> Result<(), AnimationStopFn> {
        self.0.source.0.hook_animation_stop(handler)
    }
}

#[derive(Clone)]
struct WeakMapRefVar(Weak<VarData>);
impl fmt::Debug for WeakMapRefVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("WeakMapRefVar").field(&self.0.as_ptr()).finish()
    }
}
impl WeakMapRefVar {
    pub(super) fn upgrade_typed(&self) -> Option<MapRefVar> {
        self.0.upgrade().map(MapRefVar)
    }
}
impl WeakVarImpl for WeakMapRefVar {
    fn clone_boxed(&self) -> SmallBox<dyn WeakVarImpl, smallbox::space::S2> {
        smallbox!(self.clone())
    }

    fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    fn upgrade(&self) -> Option<SmallBox<dyn VarImpl, smallbox::space::S2>> {
        match self.upgrade_typed() {
            Some(v) => Some(smallbox!(v)),
            None => None,
        }
    }
}
