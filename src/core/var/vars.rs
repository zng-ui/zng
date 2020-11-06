use super::*;
use crate::core::context::Updates;
use fnv::FnvHashMap;
use std::any::*;

singleton_assert!(SingletonVars);

enum AnyRef {}
impl AnyRef {
    fn pack<T>(r: &T) -> *const AnyRef {
        (r as *const T) as *const AnyRef
    }

    unsafe fn unpack<'a, T>(pointer: *const Self) -> &'a T {
        &*(pointer as *const T)
    }
}

/// Access to application variables.
///
/// Only a single instance of this type exists at a time.
pub struct Vars {
    _singleton: SingletonVars,
    update_id: u32,
    #[allow(clippy::type_complexity)]
    pending: RefCell<Vec<Box<dyn FnOnce(u32)>>>,
    context_vars: RefCell<FnvHashMap<TypeId, (*const AnyRef, bool, u32)>>,
}
impl Vars {
    /// Produces the instance of `Vars`. Only a single
    /// instance can exist at a time, panics if called
    /// again before dropping the previous instance.
    pub fn instance() -> Self {
        Vars {
            _singleton: SingletonVars::assert_new(),
            update_id: 0,
            pending: Default::default(),
            context_vars: Default::default(),
        }
    }

    pub(super) fn update_id(&self) -> u32 {
        self.update_id
    }

    /// Gets a var at the context level.
    pub(super) fn context_var<C: ContextVar>(&self) -> (&C::Type, bool, u32) {
        let vars = self.context_vars.borrow();
        if let Some((any_ref, is_new, version)) = vars.get(&TypeId::of::<C>()) {
            // SAFETY: This is safe because `TypeId` keys are always associated
            // with the same type of reference. Also we are not leaking because the
            // source reference is borrowed in a [`with_context_var`] call.
            let value = unsafe { AnyRef::unpack(*any_ref) };
            (value, *is_new, *version)
        } else {
            (C::default_value(), false, 0)
        }
    }

    /// Calls `f` with the context var value.
    pub fn with_context_var<C: ContextVar, F: FnOnce()>(&self, _: C, value: &C::Type, is_new: bool, version: u32, f: F) {
        let var_id = TypeId::of::<C>();

        let prev = self
            .context_vars
            .borrow_mut()
            .insert(var_id, (AnyRef::pack(value), is_new, version));

        f();

        let mut vars = self.context_vars.borrow_mut();
        if let Some(prev) = prev {
            vars.insert(var_id, prev);
        } else {
            vars.remove(&var_id);
        }
    }

    /// Calls `f` with the `context_var` set from the `other_var`.
    pub fn with_context_bind<C: ContextVar, F: FnOnce(), V: VarObj<C::Type>>(&self, context_var: C, other_var: &V, f: F) {
        self.with_context_var(context_var, other_var.get(self), other_var.is_new(self), other_var.version(self), f)
    }

    pub(super) fn push_change(&self, change: Box<dyn FnOnce(u32)>) {
        self.pending.borrow_mut().push(change);
    }

    pub(in crate::core) fn apply(&mut self, updates: &mut Updates) {
        self.update_id = self.update_id.wrapping_add(1);

        let pending = self.pending.get_mut();
        if !pending.is_empty() {
            for f in pending.drain(..) {
                f(self.update_id);
            }
            updates.push_update();
        }
    }
}
