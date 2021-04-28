use super::*;
use crate::context::Updates;
use std::{fmt, ops::Deref};

singleton_assert!(SingletonVars);

/// Read-only access to [`Vars`].
///
/// In some contexts variables can be set, so a full [`Vars`] reference if given, in other contexts
/// variables can only be read, so a [`VarsRead`] reference is given.
///
/// [`Vars`] auto-dereferences to to this type
///
/// # Examples
///
/// You can [`get`](VarsObj::get) a value using a [`VarsRead`] reference.
///
/// ```
/// # use crate::var::{VarObj, VarsRead};
/// fn read_only(var: &impl VarObj<bool>, vars: &VarsRead) -> bool {
///     *var.get(vars)
/// }
/// ```
///
/// And because of auto-dereference you can can the same method using a full [`Vars`] reference.
///
/// ```
/// # use crate::var::{VarObj, Vars};
/// fn read_write(var: &impl VarObj<bool>, vars: &Vars) -> bool {
///     *var.get(vars)
/// }
/// ```
pub struct VarsRead {
    _singleton: SingletonVars,
    update_id: u32,
    #[allow(clippy::type_complexity)]
    widget_clear: RefCell<Vec<Box<dyn Fn(bool)>>>,
}
impl fmt::Debug for VarsRead {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VarsRead {{ .. }}")
    }
}
impl VarsRead {
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
    ///
    /// The value is visible for the duration of `f`, unless `f` recursive overwrites it again.
    pub fn with_context_var<C: ContextVar, F: FnOnce()>(&self, context_var: C, value: &C::Type, version: u32, f: F) {
        self.with_context_var_impl(context_var, value, false, version, f)
    }
    fn with_context_var_impl<C: ContextVar, F: FnOnce()>(&self, context_var: C, value: &C::Type, is_new: bool, version: u32, f: F) {
        // SAFETY: `Self::context_var` makes safety assumptions about this code
        // don't change before studying it.

        let _ = context_var;
        let prev = C::thread_local_value().replace((value as _, is_new, version));
        let _restore = RunOnDrop::new(move || {
            C::thread_local_value().set(prev);
        });

        f();

        // _prev restores the parent reference here on drop
    }

    /// Calls `f` with the context var value.
    ///
    /// The value is visible for the duration of `f` and only for the parts of it that are inside the current widget context.
    ///
    /// The value can be overwritten by a recursive call to [`with_context_var`](Vars::with_context_var) or
    /// this method, subsequent values from this same widget context are not visible in inner widget contexts.
    pub fn with_context_var_wgt_only<C: ContextVar, F: FnOnce()>(&self, context_var: C, value: &C::Type, version: u32, f: F) {
        self.with_context_var_wgt_only_impl(context_var, value, false, version, f)
    }
    fn with_context_var_wgt_only_impl<C: ContextVar, F: FnOnce()>(
        &self,
        context_var: C,
        value: &C::Type,
        is_new: bool,
        version: u32,
        f: F,
    ) {
        // SAFETY: `Self::context_var` makes safety assumptions about this code
        // don't change before studying it.

        let _ = context_var;

        let new = (value as _, is_new, version);
        let prev = C::thread_local_value().replace(new);

        self.widget_clear.borrow_mut().push(Box::new(move |undo| {
            if undo {
                C::thread_local_value().set(prev);
            } else {
                C::thread_local_value().set(new);
            }
        }));

        let _restore = RunOnDrop::new(move || {
            C::thread_local_value().set(prev);
        });

        f();
    }

    /// Calls [`with_context_var`](Vars::with_context_var) with values from `other_var`.
    pub fn with_context_bind<C: ContextVar, F: FnOnce(), V: VarObj<C::Type>>(&self, context_var: C, other_var: &V, f: F) {
        self.with_context_var_impl(context_var, other_var.get(self), false, other_var.version(self), f)
    }

    /// Calls [`with_context_var_wgt_only`](Vars::with_context_var_wgt_only) with values from `other_var`.
    pub fn with_context_bind_wgt_only<C: ContextVar, F: FnOnce(), V: VarObj<C::Type>>(&self, context_var: C, other_var: &V, f: F) {
        self.with_context_var_wgt_only_impl(context_var, other_var.get(self), false, other_var.version(self), f)
    }

    /// Clears widget only context var values, calls `f` and restores widget only context var values.
    pub(crate) fn with_widget_clear<F: FnOnce()>(&self, f: F) {
        let wgt_clear = std::mem::take(&mut *self.widget_clear.borrow_mut());
        for clear in &wgt_clear {
            clear(true);
        }

        let _restore = RunOnDrop::new(move || {
            for clear in &wgt_clear {
                clear(false);
            }
            *self.widget_clear.borrow_mut() = wgt_clear;
        });

        f();
    }
}

/// Access to application variables.
///
/// Only a single instance of this type exists at a time.
pub struct Vars {
    read: VarsRead,
    #[allow(clippy::type_complexity)]
    pending: RefCell<Vec<Box<dyn FnOnce(u32)>>>,
}
impl fmt::Debug for Vars {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Vars {{ .. }}")
    }
}
impl Vars {
    /// Produces the instance of `Vars`. Only a single
    /// instance can exist at a time, panics if called
    /// again before dropping the previous instance.
    pub fn instance() -> Self {
        Vars {
            read: VarsRead {
                _singleton: SingletonVars::assert_new(),
                update_id: 0,
                widget_clear: Default::default(),
            },
            pending: Default::default(),
        }
    }

    /// Calls `f` with the context var value.
    ///
    /// The value is visible for the duration of `f`, unless `f` recursive overwrites it again.
    pub fn with_context_var<C: ContextVar, F: FnOnce()>(&self, context_var: C, value: &C::Type, is_new: bool, version: u32, f: F) {
        self.with_context_var_impl(context_var, value, is_new, version, f)
    }

    /// Calls `f` with the context var value.
    ///
    /// The value is visible for the duration of `f` and only for the parts of it that are inside the current widget context.
    ///
    /// The value can be overwritten by a recursive call to [`with_context_var`](Vars::with_context_var) or
    /// this method, subsequent values from this same widget context are not visible in inner widget contexts.
    pub fn with_context_var_wgt_only<C: ContextVar, F: FnOnce()>(&self, context_var: C, value: &C::Type, is_new: bool, version: u32, f: F) {
        self.with_context_var_wgt_only_impl(context_var, value, is_new, version, f)
    }

    /// Calls [`with_context_var`](Vars::with_context_var) with values from `other_var`.
    pub fn with_context_bind<C: ContextVar, F: FnOnce(), V: VarObj<C::Type>>(&self, context_var: C, other_var: &V, f: F) {
        self.with_context_var_impl(context_var, other_var.get(self), other_var.is_new(self), other_var.version(self), f)
    }

    /// Calls [`with_context_var_wgt_only`](Vars::with_context_var_wgt_only) with values from `other_var`.
    pub fn with_context_bind_wgt_only<C: ContextVar, F: FnOnce(), V: VarObj<C::Type>>(&self, context_var: C, other_var: &V, f: F) {
        self.with_context_var_wgt_only(context_var, other_var.get(self), other_var.is_new(self), other_var.version(self), f)
    }

    pub(super) fn push_change(&self, change: Box<dyn FnOnce(u32)>) {
        self.pending.borrow_mut().push(change);
    }

    pub(crate) fn apply(&mut self, updates: &mut Updates) {
        self.read.update_id = self.update_id.wrapping_add(1);

        let pending = self.pending.get_mut();
        if !pending.is_empty() {
            for f in pending.drain(..) {
                f(self.read.update_id);
            }
            updates.update();
        }
    }
}
impl Deref for Vars {
    type Target = VarsRead;

    fn deref(&self) -> &Self::Target {
        &self.read
    }
}

struct RunOnDrop<F: FnOnce()>(Option<F>);
impl<F: FnOnce()> RunOnDrop<F> {
    fn new(clean: F) -> Self {
        RunOnDrop(Some(clean))
    }
}
impl<F: FnOnce()> Drop for RunOnDrop<F> {
    fn drop(&mut self) {
        if let Some(clean) = self.0.take() {
            clean();
        }
    }
}
