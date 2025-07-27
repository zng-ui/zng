//! Read-write variable that stores values in a shared storage to save space.

use std::{
    mem,
    sync::{Arc, Weak, atomic::AtomicBool},
};

use parking_lot::{Mutex, RwLock};
use smallvec::SmallVec;

use crate::{VARS, Var, VarAny, VarUpdateId, VarValue, animation::ModifyInfo};

use super::*;

/// New read/write shared reference variable.
pub fn var<T: VarValue>(initial_value: T) -> Var<T> {
    Var::new_any(VarAny(smallbox!(SharedVar::new(smallbox!(initial_value)))))
}

/// New read/write shared reference type-erased variable.
pub fn var_any(initial_value: BoxedVarValueAny) -> VarAny {
    VarAny(smallbox!(SharedVar::new(initial_value)))
}

/// Variable for state properties (`is_*`, `has_*`).
///
/// State variables are `bool` probes that are set by the property, they are created automatically
/// by the property default when used in `when` expressions, but can be created manually.
pub fn var_state() -> Var<bool> {
    var(false)
}

/// Variable for getter properties (`get_*`, `actual_*`).
///
/// Getter variables are inited with a default value that is overridden by the property on node init and updated
/// by the property when the internal state they track changes. They are created automatically by the property
/// default when used in `when` expressions, but can be created manually.
pub fn var_getter<T: VarValue + Default>() -> Var<T> {
    var(T::default())
}

struct VarData {
    value: RwLock<(BoxedVarValueAny, VarUpdateId, ModifyInfo)>,
    hooks: MutexHooks,
}

#[derive(Clone)]
pub(crate) struct SharedVar(Arc<VarData>);
impl SharedVar {
    pub(crate) fn new(value: BoxedVarValueAny) -> Self {
        Self(Arc::new(VarData {
            value: RwLock::new((value, VarUpdateId::never(), ModifyInfo::never())),
            hooks: MutexHooks::default(),
        }))
    }

    pub(super) fn downgrade_typed(&self) -> WeakSharedVar {
        WeakSharedVar(Arc::downgrade(&self.0))
    }
}
impl VarImpl for SharedVar {
    fn clone_boxed(&self) -> SmallBox<dyn VarImpl, smallbox::space::S2> {
        smallbox!(self.clone())
    }

    fn current_context(&self) -> SmallBox<dyn VarImpl, smallbox::space::S2> {
        self.clone_boxed()
    }

    fn value_type(&self) -> TypeId {
        let value = self.0.value.read();
        let value: &dyn Any = &*value;
        value.type_id()
    }

    #[cfg(feature = "value_type_name")]
    fn value_type_name(&self) -> &'static str {
        let value = self.0.value.read();
        value.0.type_name()
    }

    fn strong_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }

    fn var_eq(&self, other: &dyn Any) -> bool {
        match other.downcast_ref::<SharedVar>() {
            Some(v) => Arc::ptr_eq(&self.0, &v.0),
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
        VarCapability::NEW | VarCapability::MODIFY | VarCapability::SHARE
    }

    fn with(&self, visitor: &mut dyn FnMut(&dyn VarValueAny)) {
        let value = self.0.value.read();
        visitor(&*value.0);
    }

    fn get(&self) -> BoxedVarValueAny {
        self.0.value.read().0.clone_boxed()
    }

    fn set(&self, new_value: BoxedVarValueAny) -> bool {
        self.modify_impl(ValueOrModify::Value(new_value));
        true
    }

    fn update(&self) -> bool {
        self.modify(smallbox!(|v: &mut VarModifyAny| {
            v.update();
        }))
    }

    fn modify(&self, modify: SmallBox<dyn FnMut(&mut VarModifyAny) + Send + 'static, smallbox::space::S4>) -> bool {
        self.modify_impl(ValueOrModify::Modify(modify));
        true
    }

    fn hook(&self, on_new: HookFn) -> VarHandle {
        self.0.hooks.push(on_new)
    }

    fn last_update(&self) -> VarUpdateId {
        self.0.value.read().1
    }

    fn modify_importance(&self) -> usize {
        self.0.value.read().2.importance()
    }

    fn is_animating(&self) -> bool {
        self.0.value.read().2.is_animating()
    }

    fn hook_animation_stop(&self, handler: AnimationStopFn) -> Result<(), AnimationStopFn> {
        self.0.value.read().2.hook_animation_stop(handler)
    }
}
impl SharedVar {
    fn modify_impl(&self, value_or_modify: ValueOrModify) {
        let weak = self.downgrade_typed();
        let name = value_type_name(self);
        VARS.schedule_update(name, move || {
            if let Some(var) = weak.upgrade_typed() {
                let mut value = var.0.value.write();

                // verify if contextual animation can still set
                let current_modify = VARS.current_modify();
                if current_modify.importance() < value.2.importance() {
                    return;
                }
                value.2 = current_modify;

                // modify
                let mut m = VarModifyAny {
                    value: VarModifyAnyValue::Boxed(&mut value.0),
                    update: VarModifyUpdate::empty(),
                    tags: vec![],
                    custom_importance: None,
                };
                match value_or_modify {
                    ValueOrModify::Value(v) => {
                        m.set(v);
                    }
                    ValueOrModify::Modify(mut f) => (f)(&mut m),
                }

                let VarModifyAny {
                    update,
                    tags,
                    custom_importance,
                    ..
                } = m;

                if let Some(i) = custom_importance {
                    value.2.importance = i;
                }

                if update.contains(VarModifyUpdate::UPDATE) {
                    value.1 = VARS.update_id();

                    let value = parking_lot::RwLockWriteGuard::downgrade(value);
                    let args = VarAnyHookArgs::new(&*value.0, update.contains(VarModifyUpdate::REQUESTED), &tags);
                    var.0.hooks.notify(&args);
                }
            }
        });
    }
}
// both boxes are space::S4, so can't implement `set` as `modify` without alloc
// this type and `modify_impl` work around that
enum ValueOrModify {
    Value(BoxedVarValueAny),
    Modify(SmallBox<dyn FnMut(&mut VarModifyAny) + Send + 'static, smallbox::space::S4>),
}

#[derive(Clone)]
pub(super) struct WeakSharedVar(Weak<VarData>);
impl WeakSharedVar {
    pub(super) fn upgrade_typed(&self) -> Option<SharedVar> {
        self.0.upgrade().map(SharedVar)
    }
}
impl WeakVarImpl for WeakSharedVar {
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

#[derive(Default)]
pub(super) struct MutexHooks {
    h: Mutex<SmallVec<[(HookFn, Arc<AtomicBool>); 1]>>,
}
impl MutexHooks {
    pub fn push(&self, on_new: HookFn) -> VarHandle {
        let handle = Arc::new(AtomicBool::new(false));
        self.h.lock().push((on_new, handle.clone()));
        VarHandle::new(handle)
    }

    pub fn notify(&self, args: &VarAnyHookArgs) {
        let mut hooks = mem::take(&mut *self.h.lock());

        hooks.retain(|(f, handle)| {
            if Arc::strong_count(handle) == 1 && !handle.load(std::sync::atomic::Ordering::Relaxed) {
                return false; // handle dropped
            }
            f(args)
        });

        if !hooks.is_empty() {
            let mut hs = self.h.lock();
            if hs.capacity() > hooks.capacity() {
                hs.append(&mut hooks);
            } else {
                hooks.append(&mut *hs);
                *hs = hooks;
            }
        }
    }
}

pub(super) type HookFn = SmallBox<dyn FnMut(&VarAnyHookArgs) -> bool + Send + 'static, smallbox::space::S4>;
