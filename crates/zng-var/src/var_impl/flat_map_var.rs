//! Unwrapping mapping var

use core::fmt;
use std::sync::{Arc, Weak};

use crate::{
    AnyVar, AnyVarHookArgs, AnyVarValue, BoxAnyVarValue, DynAnyVar, DynWeakAnyVar, VarHandle, VarImpl, VarInstanceTag, VarUpdateId,
    WeakVarImpl, shared_var::MutexHooks,
};
use parking_lot::{Mutex, RwLock};
use smallbox::SmallBox;

use super::{AnyVarModify, VarCapability};

type MapFn = SmallBox<dyn FnMut(&dyn AnyVarValue) -> AnyVar + Send + 'static, smallbox::space::S4>;

/// source var is stored as the value of the `write` SharedVar
struct FlatMapData {
    source: AnyVar,
    map: Mutex<MapFn>,
    current: RwLock<(AnyVar, VarHandle)>,
    hooks: MutexHooks,
}

#[derive(Clone)]
pub(crate) struct FlatMapVar(Arc<FlatMapData>);
impl fmt::Debug for FlatMapVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut b = f.debug_struct("FlatMapVar");
        b.field("source", &self.0.source);
        if let Some(c) = self.0.current.try_read() {
            b.field("current", &c.0);
        } else {
            b.field("current", &"<locked>");
        }
        b.field("hooks", &self.0.hooks);
        b.finish()
    }
}
impl FlatMapVar {
    pub(crate) fn new(source: AnyVar, mut map: MapFn) -> Self {
        let init = source.with(|v| map(v));

        let data = Arc::new(FlatMapData {
            source,
            map: Mutex::new(map),
            current: RwLock::new((init, VarHandle::dummy())),
            hooks: MutexHooks::default(),
        });

        hook_inner_var(&data, data.current.write());

        let weak = Arc::downgrade(&data);
        data.source
            .hook(move |args| {
                if let Some(data) = weak.upgrade() {
                    let new_inner = data.map.lock()(args.value());

                    let mut current = data.current.write();
                    if !current.0.var_eq(&new_inner) {
                        current.1 = VarHandle::dummy();
                        current.0 = new_inner;
                        hook_inner_var(&data, current);

                        // FlatMapVar will show as new because `source` just updated, so
                        // notify hooks here to match
                        data.current.read().0.with(|v| {
                            data.hooks.notify(&AnyVarHookArgs {
                                var_instance_tag: VarInstanceTag(Arc::as_ptr(&data) as _),
                                value: v,
                                update: args.update,
                                tags: args.tags,
                            });
                        });
                    }
                    // retain hook, we are still alive
                    true
                } else {
                    false
                }
            })
            .perm();

        Self(data)
    }
}

fn hook_inner_var(data: &Arc<FlatMapData>, mut current: parking_lot::RwLockWriteGuard<(AnyVar, VarHandle)>) {
    let weak = Arc::downgrade(data);
    let init_handle = current.0.hook(move |args| {
        if let Some(data) = weak.upgrade() {
            data.hooks.notify(&AnyVarHookArgs {
                var_instance_tag: VarInstanceTag(Arc::as_ptr(&data) as _),
                value: args.value,
                update: args.update,
                tags: args.tags,
            });

            // retain hook to current inner var
            true
        } else {
            false
        }
    });
    current.1 = init_handle;
}

impl VarImpl for FlatMapVar {
    fn clone_dyn(&self) -> DynAnyVar {
        DynAnyVar::FlatMap(self.clone())
    }

    fn current_context(&self) -> DynAnyVar {
        self.clone_dyn()
    }

    fn value_type(&self) -> std::any::TypeId {
        self.0.current.read().0.value_type()
    }

    #[cfg(feature = "type_names")]
    fn value_type_name(&self) -> &'static str {
        self.0.current.read().0.value_type_name()
    }

    fn strong_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }

    fn var_eq(&self, other: &DynAnyVar) -> bool {
        match other {
            DynAnyVar::FlatMap(o) => Arc::ptr_eq(&self.0, &o.0),
            _ => false,
        }
    }

    fn var_instance_tag(&self) -> VarInstanceTag {
        VarInstanceTag(Arc::as_ptr(&self.0) as _)
    }

    fn downgrade(&self) -> DynWeakAnyVar {
        DynWeakAnyVar::FlatMap(WeakFlatMapVar(Arc::downgrade(&self.0)))
    }

    fn capabilities(&self) -> VarCapability {
        self.0.current.read().0.capabilities() | VarCapability::SHARE | VarCapability::MODIFY_CHANGES | VarCapability::CONTEXT_CHANGES
    }

    fn with(&self, visitor: &mut dyn FnMut(&dyn AnyVarValue)) {
        self.0.current.read().0.0.with(visitor);
    }

    fn get(&self) -> BoxAnyVarValue {
        self.0.current.read().0.0.get()
    }

    fn set(&self, new_value: BoxAnyVarValue) -> bool {
        self.0.current.read().0.0.set(new_value)
    }

    fn update(&self) -> bool {
        self.0.current.read().0.0.update()
    }

    fn modify(&self, modify: SmallBox<dyn FnMut(&mut AnyVarModify) + Send + 'static, smallbox::space::S4>) -> bool {
        self.0.current.read().0.0.modify(modify)
    }

    fn hook(&self, on_new: SmallBox<dyn FnMut(&AnyVarHookArgs) -> bool + Send + 'static, smallbox::space::S4>) -> VarHandle {
        self.0.hooks.push(on_new)
    }

    fn last_update(&self) -> VarUpdateId {
        self.0.source.0.last_update().max(self.0.current.read().0.0.last_update())
    }

    fn modify_info(&self) -> crate::animation::ModifyInfo {
        self.0.current.read().0.0.modify_info()
    }

    fn modify_importance(&self) -> usize {
        self.0.current.read().0.0.modify_importance()
    }

    fn is_animating(&self) -> bool {
        self.0.current.read().0.0.is_animating()
    }

    fn hook_animation_stop(&self, handler: crate::animation::AnimationStopFn) -> Result<(), crate::animation::AnimationStopFn> {
        self.0.current.read().0.0.hook_animation_stop(handler)
    }
}

#[derive(Clone)]
pub(crate) struct WeakFlatMapVar(Weak<FlatMapData>);
impl fmt::Debug for WeakFlatMapVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("WeakFlatMapVar").field(&self.0.as_ptr()).finish()
    }
}
impl WeakVarImpl for WeakFlatMapVar {
    fn clone_dyn(&self) -> DynWeakAnyVar {
        DynWeakAnyVar::FlatMap(Self(self.0.clone()))
    }

    fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    fn upgrade(&self) -> Option<DynAnyVar> {
        Some(DynAnyVar::FlatMap(FlatMapVar(self.0.upgrade()?)))
    }
}
