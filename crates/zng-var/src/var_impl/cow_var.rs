//! Clone on write var, represents another variable until the first modify,
//! them the value is cloned and modified in a new variable.

use std::mem;

use crate::AnyVar;

use super::{shared_var::SharedVar, *};

/// source var is stored as the value of the `write` SharedVar
#[derive(Clone)]
struct CowVarSource {
    source: AnyVar,
    _source_hook: VarHandle,
}
impl fmt::Debug for CowVarSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CowVarSource").finish_non_exhaustive()
    }
}
impl PartialEq for CowVarSource {
    fn eq(&self, other: &Self) -> bool {
        self.source.var_eq(&other.source)
    }
}

pub(crate) struct CowVar(SharedVar);
impl fmt::Debug for CowVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut b = f.debug_struct("CowVar");

        b.finish()
    }
}
impl CowVar {
    pub(crate) fn new(source: AnyVar) -> Self {
        let me = SharedVar::new(BoxAnyVarValue::new(()), source.0.last_update(), source.0.modify_info());
        let weak_me = me.downgrade_typed();

        // update CowVar on source update
        let _source_hook = source.hook(move |_| match weak_me.upgrade_typed() {
            Some(me) => {
                me.update();
                true
            }
            None => false,
        });
        me.0.value.write().0 = BoxAnyVarValue::new(CowVarSource { source, _source_hook });

        Self(me)
    }
}
impl PartialEq for CowVar {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0.0, &other.0.0)
    }
}
impl VarImpl for CowVar {
    fn clone_dyn(&self) -> DynAnyVar {
        DynAnyVar::Cow(Self(self.0.clone()))
    }

    fn current_context(&self) -> DynAnyVar {
        self.clone_dyn()
    }

    fn value_type(&self) -> TypeId {
        let mut output = None;
        self.0.with(&mut |v| {
            if let Some(source) = v.downcast_ref::<CowVarSource>() {
                output = Some(source.source.value_type());
            } else {
                output = Some(v.type_id());
            }
        });
        output.unwrap()
    }

    #[cfg(feature = "type_names")]
    fn value_type_name(&self) -> &'static str {
        let mut output = "";
        self.0.with(&mut |v| {
            if let Some(source) = v.downcast_ref::<CowVarSource>() {
                output = source.source.value_type_name();
            } else {
                output = v.type_name();
            }
        });
        output
    }

    fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    fn var_eq(&self, other: &DynAnyVar) -> bool {
        match other {
            DynAnyVar::Cow(v) => self == v,
            _ => false,
        }
    }

    fn var_instance_tag(&self) -> VarInstanceTag {
        self.0.var_instance_tag()
    }

    fn downgrade(&self) -> DynWeakAnyVar {
        DynWeakAnyVar::Cow(WeakCowVar(self.0.downgrade_typed()))
    }

    fn capabilities(&self) -> VarCapability {
        let mut caps = VarCapability::NEW | VarCapability::MODIFY | VarCapability::SHARE;
        self.0.with(&mut |v| {
            if let Some(s) = v.downcast_ref::<CowVarSource>() {
                let mut source_caps = s.source.capabilities();
                source_caps.remove(VarCapability::MODIFY_CHANGES);
                if source_caps.contains(VarCapability::CONTEXT) {
                    source_caps |= VarCapability::CONTEXT_CHANGES;
                }
                caps |= source_caps;
            }
        });
        caps
    }

    fn with(&self, visitor: &mut dyn FnMut(&dyn AnyVarValue)) {
        self.0.with(&mut move |v| {
            if let Some(source) = v.downcast_ref::<CowVarSource>() {
                source.source.with(&mut *visitor);
            } else {
                visitor(v);
            }
        });
    }

    fn get(&self) -> BoxAnyVarValue {
        let mut output = None;
        self.0.with(&mut |v| {
            if let Some(source) = v.downcast_ref::<CowVarSource>() {
                output = Some(source.source.get());
            } else {
                output = Some(v.clone_boxed());
            }
        });
        output.unwrap()
    }

    fn set(&self, new_value: BoxAnyVarValue) -> bool {
        let mut new_value = Some(new_value);
        self.0.modify(smallbox!(move |value: &mut AnyVarModify| {
            let new_value = new_value.take().unwrap();
            if value.is::<CowVarSource>() {
                *value.value = new_value;
                value.update |= VarModifyUpdate::TOUCHED;
            } else {
                value.set(new_value);
            }
        }))
    }

    fn update(&self) -> bool {
        self.0.modify(smallbox!(|value: &mut AnyVarModify| {
            if let Some(read) = value.downcast_ref::<CowVarSource>() {
                // clone on write
                let new_value = read.source.get();
                *value.value = new_value;
                value.update |= VarModifyUpdate::TOUCHED;
            } else {
                value.update();
            }
        }));
        true
    }

    fn modify(&self, mut modify: SmallBox<dyn FnMut(&mut AnyVarModify) + Send + 'static, smallbox::space::S4>) -> bool {
        self.0.modify(smallbox!(move |value: &mut AnyVarModify| {
            if let Some(source) = value.downcast_ref::<CowVarSource>() {
                // clone on write
                let mut source_value = source.source.get();
                let mut vm = AnyVarModify {
                    value: &mut source_value,
                    update: VarModifyUpdate::empty(),
                    tags: mem::take(&mut value.tags),
                    custom_importance: value.custom_importance,
                };
                modify(&mut vm);

                value.tags = vm.tags;
                value.custom_importance = vm.custom_importance;
                value.update |= vm.update;

                if vm.update.contains(VarModifyUpdate::TOUCHED) {
                    *value.value = source_value;
                }
            } else {
                modify(value);
            }
        }))
    }

    fn hook(&self, mut on_new: SmallBox<dyn FnMut(&AnyVarHookArgs) -> bool + Send + 'static, smallbox::space::S4>) -> VarHandle {
        self.0.hook(smallbox!(move |args: &AnyVarHookArgs| {
            if let Some(read) = args.value.downcast_ref::<CowVarSource>() {
                let mut retain = false;
                read.source.with(&mut |value: &dyn AnyVarValue| {
                    retain = on_new(&AnyVarHookArgs {
                        var_instance_tag: args.var_instance_tag,
                        value,
                        update: args.update,
                        tags: args.tags,
                    })
                });
                retain
            } else {
                on_new(args)
            }
        }))
    }

    fn last_update(&self) -> VarUpdateId {
        let mut id = self.0.last_update();
        self.0.with(&mut |v| {
            if let Some(s) = v.downcast_ref::<CowVarSource>() {
                id = s.source.last_update();
            }
        });
        id
    }

    fn modify_info(&self) -> ModifyInfo {
        let mut info = self.0.modify_info();
        self.0.with(&mut |v| {
            if let Some(s) = v.downcast_ref::<CowVarSource>() {
                info = s.source.0.modify_info();
            }
        });
        info
    }

    fn modify_importance(&self) -> usize {
        let mut imp = self.0.modify_importance();
        self.0.with(&mut |v| {
            if let Some(s) = v.downcast_ref::<CowVarSource>() {
                imp = s.source.modify_importance();
            }
        });
        imp
    }

    fn is_animating(&self) -> bool {
        let mut is_anim = self.0.is_animating();
        self.0.with(&mut |v| {
            if let Some(s) = v.downcast_ref::<CowVarSource>() {
                is_anim = s.source.is_animating();
            }
        });
        is_anim
    }

    fn hook_animation_stop(&self, handler: AnimationStopFn) -> VarHandle {
        let mut result = VarHandle::dummy();
        let mut handler = Some(handler);
        self.0.with(&mut |v| {
            if let Some(s) = v.downcast_ref::<CowVarSource>() {
                result = s.source.0.hook_animation_stop(handler.take().unwrap());
            }
        });
        match handler {
            Some(handler) => self.0.hook_animation_stop(handler),
            None => result,
        }
    }
}

pub(crate) struct WeakCowVar(super::shared_var::WeakSharedVar);
impl fmt::Debug for WeakCowVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("WeakCowVar").field(&self.0).finish()
    }
}
impl WeakVarImpl for WeakCowVar {
    fn clone_dyn(&self) -> DynWeakAnyVar {
        DynWeakAnyVar::Cow(Self(self.0.clone()))
    }

    fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    fn upgrade(&self) -> Option<DynAnyVar> {
        Some(DynAnyVar::Cow(CowVar(self.0.upgrade_typed()?)))
    }
}
