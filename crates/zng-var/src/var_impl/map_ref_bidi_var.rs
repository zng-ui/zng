use std::{
    mem,
    sync::{Arc, Weak},
};

use crate::AnyVar;

use super::*;

type DerefFn = SmallBox<dyn for<'a> Fn(&'a dyn AnyVarValue) -> &'a (dyn AnyVarValue) + Send + Sync + 'static, smallbox::space::S4>;
type DerefMutFn =
    SmallBox<dyn for<'a> Fn(&'a mut dyn AnyVarValue) -> &'a mut (dyn AnyVarValue) + Send + Sync + 'static, smallbox::space::S4>;

struct VarData {
    source: AnyVar,
    deref: DerefFn,
    deref_mut: DerefMutFn,
}

#[derive(Clone)]
pub(crate) struct MapBidiRefVar(Arc<VarData>);

impl MapBidiRefVar {
    pub(crate) fn new(source: AnyVar, deref: DerefFn, deref_mut: DerefMutFn) -> Self {
        Self(Arc::new(VarData { source, deref, deref_mut }))
    }

    fn downgrade_typed(&self) -> WeakMapBidiRefVar {
        WeakMapBidiRefVar(Arc::downgrade(&self.0))
    }
}
impl VarImpl for MapBidiRefVar {
    fn clone_boxed(&self) -> SmallBox<dyn VarImpl, smallbox::space::S2> {
        smallbox!(self.clone())
    }

    fn current_context(&self) -> SmallBox<dyn VarImpl, smallbox::space::S2> {
        self.clone_boxed()
    }

    fn value_type(&self) -> TypeId {
        self.0.source.value_type()
    }

    #[cfg(feature = "value_type_name")]
    fn value_type_name(&self) -> &'static str {
        self.0.source.value_type_name()
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
        smallbox!(self.downgrade_typed())
    }

    fn capabilities(&self) -> VarCapability {
        self.0.source.capabilities() | VarCapability::SHARE
    }

    fn with(&self, visitor: &mut dyn FnMut(&dyn AnyVarValue)) {
        let deref = &*self.0.deref;
        self.0.source.with(&mut move |value: &dyn AnyVarValue| visitor((deref)(value)));
    }

    fn get(&self) -> BoxAnyVarValue {
        let mut out = None;
        let deref = &*self.0.deref;
        self.0
            .source
            .with(&mut |value: &dyn AnyVarValue| out = Some((deref)(value).clone_boxed()));
        out.unwrap()
    }

    fn set(&self, mut new_value: BoxAnyVarValue) -> bool {
        let weak = Arc::downgrade(&self.0);
        self.0
            .source
            .try_modify(move |value| {
                if let Some(s) = weak.upgrade() {
                    (s.deref_mut)(&mut **value).try_swap(&mut *new_value);
                }
            })
            .is_ok()
    }

    fn update(&self) -> bool {
        self.0.source.0.update()
    }

    fn modify(&self, mut modify: SmallBox<dyn FnMut(&mut VarModifyAny) + Send + 'static, smallbox::space::S4>) -> bool {
        let weak = Arc::downgrade(&self.0);
        self.0
            .source
            .try_modify(move |value| {
                if let Some(s) = weak.upgrade() {
                    let mut m = VarModifyAny {
                        update: value.update,
                        tags: mem::take(&mut value.tags),
                        custom_importance: value.custom_importance,
                        value: VarModifyAnyValue::RefOnly((s.deref_mut)(&mut **value)),
                    };

                    modify(&mut m);

                    let VarModifyAny {
                        tags, custom_importance, ..
                    } = m;

                    value.tags = tags;
                    value.custom_importance = custom_importance;
                }
            })
            .is_ok()
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
struct WeakMapBidiRefVar(Weak<VarData>);
impl WeakMapBidiRefVar {
    pub(super) fn upgrade_typed(&self) -> Option<MapBidiRefVar> {
        self.0.upgrade().map(MapBidiRefVar)
    }
}
impl WeakVarImpl for WeakMapBidiRefVar {
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
