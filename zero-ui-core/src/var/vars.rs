use super::*;
use crate::context::Updates;

singleton_assert!(SingletonVars);

/// Access to application variables.
///
/// Only a single instance of this type exists at a time.
pub struct Vars {
    _singleton: SingletonVars,
    update_id: u32,
    #[allow(clippy::type_complexity)]
    pending: RefCell<Vec<Box<dyn FnOnce(u32)>>>,
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
        }
    }

    pub(super) fn update_id(&self) -> u32 {
        self.update_id
    }

    /// Gets a var at the context level.
    pub(super) fn context_var<C: ContextVar>(&self) -> (&C::Type, bool, u32) {
        let (value, is_new, version) = C::thread_local_value().get();

        (
            // SAFETY: this is safe as long we are the only one to call `C::thread_local_value().get()` in
            // `Self::with_context_var`.
            //
            // The reference is held for as long as it is accessible in here, at least:
            //
            // * The initial reference is actually the `static` default value.
            // * Other references are held by `Self::with_context_var` for the duration
            //   they can appear here.
            unsafe { &*value },
            is_new,
            version,
        )
    }

    /// Calls `f` with the context var value.
    pub fn with_context_var<C: ContextVar, F: FnOnce()>(&self, context_var: C, value: &C::Type, is_new: bool, version: u32, f: F) {
        // SAFETY: `Self::context_var` makes safety assumptions about this code
        // don't change before studying it.

        let _prev = RestoreOnDrop {
            prev: C::thread_local_value().replace((value as _, is_new, version)),
            _c: context_var,
        };

        f();

        // _prev restores the parent reference here on drop
    }

    /// Calls `f` with the `context_var` set from the `other_var`.
    pub fn with_context_bind<C: ContextVar, F: FnOnce(), V: VarObj<C::Type>>(&self, context_var: C, other_var: &V, f: F) {
        self.with_context_var(context_var, other_var.get(self), other_var.is_new(self), other_var.version(self), f)
    }

    pub(super) fn push_change(&self, change: Box<dyn FnOnce(u32)>) {
        self.pending.borrow_mut().push(change);
    }

    pub(crate) fn apply(&mut self, updates: &mut Updates) {
        self.update_id = self.update_id.wrapping_add(1);

        let pending = self.pending.get_mut();
        if !pending.is_empty() {
            for f in pending.drain(..) {
                f(self.update_id);
            }
            updates.update();
        }
    }
}

struct RestoreOnDrop<C: ContextVar> {
    prev: (*const C::Type, bool, u32),
    _c: C,
}
impl<C: ContextVar> Drop for RestoreOnDrop<C> {
    fn drop(&mut self) {
        C::thread_local_value().set(self.prev);
    }
}
