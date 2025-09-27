//! Read-write variable that stores values in a shared storage to save space.

use std::{
    mem,
    sync::{Arc, Weak},
};

use parking_lot::{Mutex, RwLock};
use smallvec::SmallVec;

use crate::{AnyVar, VARS, Var, VarUpdateId, VarValue, animation::ModifyInfo};

use super::*;

/// New read/write shared reference variable.
pub fn var<T: VarValue>(initial_value: T) -> Var<T> {
    Var::new_any(any_var(BoxAnyVarValue::new(initial_value)))
}

/// New read/write shared reference type-erased variable that has initial value derived from `source`.
///
/// This function is useful for creating custom mapping outputs, the new variable
/// starts with the same [`AnyVar::last_update`] and animation handle as the `source`.
pub fn var_derived<T: VarValue>(initial_value: T, source: &AnyVar) -> Var<T> {
    Var::new_any(any_var_derived(BoxAnyVarValue::new(initial_value), source))
}

/// New read/write shared reference type-erased variable.
pub fn any_var(initial_value: BoxAnyVarValue) -> AnyVar {
    AnyVar(DynAnyVar::Shared(SharedVar::new(
        initial_value,
        VarUpdateId::never(),
        ModifyInfo::never(),
    )))
}

/// New read/write shared reference type-erased variable that has initial value derived from `source`.
///
/// This function is useful for creating custom mapping outputs, the new variable
/// starts with the same [`AnyVar::last_update`] and animation handle as the `source`.
pub fn any_var_derived(initial_value: BoxAnyVarValue, source: &AnyVar) -> AnyVar {
    AnyVar(DynAnyVar::Shared(SharedVar::new(
        initial_value,
        source.0.last_update(),
        source.0.modify_info(),
    )))
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

pub(super) struct VarData {
    pub(super) value: RwLock<(BoxAnyVarValue, VarUpdateId, ModifyInfo)>,
    hooks: MutexHooks,
}

#[derive(Clone)]
pub(crate) struct SharedVar(pub(super) Arc<VarData>);
impl fmt::Debug for SharedVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut b = f.debug_struct("SharedVar");
        b.field("var_instance_tag()", &Arc::as_ptr(&self.0));
        b.field("strong_count()", &self.strong_count());

        if let Some(value) = self.0.value.try_read() {
            b.field("value", &value.0.detailed_debug());
            b.field("last_update", &value.1);
            b.field("modify_info", &value.2);
        } else {
            b.field("value", &"<locked>");
        }

        b.field("hooks", &self.0.hooks);

        b.finish()
    }
}
impl SharedVar {
    pub(crate) fn new(value: BoxAnyVarValue, last_update: VarUpdateId, modify_info: ModifyInfo) -> Self {
        Self(Arc::new(VarData {
            value: RwLock::new((value, last_update, modify_info)),
            hooks: MutexHooks::default(),
        }))
    }

    pub(super) fn downgrade_typed(&self) -> WeakSharedVar {
        WeakSharedVar(Arc::downgrade(&self.0))
    }
}
impl PartialEq for SharedVar {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl VarImpl for SharedVar {
    fn clone_dyn(&self) -> DynAnyVar {
        DynAnyVar::Shared(self.clone())
    }

    fn current_context(&self) -> DynAnyVar {
        self.clone_dyn()
    }

    fn value_type(&self) -> TypeId {
        self.0.value.read().0.type_id()
    }

    #[cfg(feature = "type_names")]
    fn value_type_name(&self) -> &'static str {
        let value = self.0.value.read();
        value.0.type_name()
    }

    fn strong_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }

    fn var_eq(&self, other: &DynAnyVar) -> bool {
        match other {
            DynAnyVar::Shared(v) => self == v,
            _ => false,
        }
    }

    fn var_instance_tag(&self) -> VarInstanceTag {
        VarInstanceTag(Arc::as_ptr(&self.0) as usize)
    }

    fn downgrade(&self) -> DynWeakAnyVar {
        DynWeakAnyVar::Shared(self.downgrade_typed())
    }

    fn capabilities(&self) -> VarCapability {
        VarCapability::NEW | VarCapability::MODIFY | VarCapability::SHARE
    }

    fn with(&self, visitor: &mut dyn FnMut(&dyn AnyVarValue)) {
        let value = self.0.value.read();
        visitor(&*value.0);
    }

    fn get(&self) -> BoxAnyVarValue {
        self.0.value.read().0.clone_boxed()
    }

    fn set(&self, new_value: BoxAnyVarValue) -> bool {
        self.modify_impl(ValueOrModify::Value(new_value));
        true
    }

    fn update(&self) -> bool {
        self.modify(smallbox!(|v: &mut AnyVarModify| {
            v.update();
        }))
    }

    fn modify(&self, modify: SmallBox<dyn FnMut(&mut AnyVarModify) + Send + 'static, smallbox::space::S4>) -> bool {
        self.modify_impl(ValueOrModify::Modify(modify));
        true
    }

    fn hook(&self, on_new: HookFn) -> VarHandle {
        self.0.hooks.push(on_new)
    }

    fn last_update(&self) -> VarUpdateId {
        self.0.value.read().1
    }

    fn modify_info(&self) -> ModifyInfo {
        self.0.value.read().2.clone()
    }

    fn modify_importance(&self) -> usize {
        self.0.value.read().2.importance()
    }

    fn is_animating(&self) -> bool {
        self.0.value.read().2.is_animating()
    }

    fn hook_animation_stop(&self, handler: AnimationStopFn) -> VarHandle {
        self.0.value.read().2.hook_animation_stop(handler)
    }
}
impl SharedVar {
    fn modify_impl(&self, value_or_modify: ValueOrModify) {
        let name = value_type_name(self);
        let var = self.clone();
        // not weak ref here because some vars are spawned modified just to notify something and dropped
        VARS.schedule_update(name, move || {
            let mut value = var.0.value.write();

            // verify if contextual animation can still set
            let current_modify = VARS.current_modify();
            if current_modify.importance() < value.2.importance() {
                return;
            }
            value.2 = current_modify;

            // modify
            let mut m = AnyVarModify {
                value: &mut value.0,
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

            let AnyVarModify {
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
                let args = AnyVarHookArgs::new(
                    var.var_instance_tag(),
                    &*value.0,
                    update.contains(VarModifyUpdate::REQUESTED),
                    &tags,
                );
                var.0.hooks.notify(&args);
            }
        });
    }
}
// both boxes are space::S4, so can't implement `set` as `modify` without alloc
// this type and `modify_impl` work around that
enum ValueOrModify {
    Value(BoxAnyVarValue),
    Modify(SmallBox<dyn FnMut(&mut AnyVarModify) + Send + 'static, smallbox::space::S4>),
}

#[derive(Clone)]
pub(crate) struct WeakSharedVar(Weak<VarData>);
impl fmt::Debug for WeakSharedVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("WeakSharedVar").field(&self.0.as_ptr()).finish()
    }
}
impl WeakSharedVar {
    pub(super) fn upgrade_typed(&self) -> Option<SharedVar> {
        self.0.upgrade().map(SharedVar)
    }
}
impl WeakVarImpl for WeakSharedVar {
    fn clone_dyn(&self) -> DynWeakAnyVar {
        DynWeakAnyVar::Shared(self.clone())
    }

    fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    fn upgrade(&self) -> Option<DynAnyVar> {
        Some(DynAnyVar::Shared(self.upgrade_typed()?))
    }
}

#[derive(Default)]
pub(super) struct MutexHooks {
    h: Mutex<SmallVec<[(HookFn, VarHandlerOwner); 1]>>,
}
impl MutexHooks {
    pub fn push(&self, on_new: HookFn) -> VarHandle {
        let (owner, handle) = VarHandle::new();
        self.h.lock().push((on_new, owner));
        handle
    }

    pub fn notify(&self, args: &AnyVarHookArgs) {
        let mut hooks = mem::take(&mut *self.h.lock());

        hooks.retain(|(f, handle)| handle.is_alive() && f(args));

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
impl fmt::Debug for MutexHooks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(h) = self.h.try_lock() {
            let mut b = f.debug_list();
            for (_, h) in h.iter() {
                b.entry(h);
            }
            b.finish()
        } else {
            write!(f, "<locked>")
        }
    }
}

pub(super) type HookFn = SmallBox<dyn FnMut(&AnyVarHookArgs) -> bool + Send + 'static, smallbox::space::S4>;
