use super::{AppContext, AppContextId};
use std::any::type_name;
use std::cell::{Cell, UnsafeCell};
use std::rc::Rc;

/// A variable value that is set by the ancestors of an UiNode.
pub trait ContextVar: 'static {
    /// The variable type.
    type Type: 'static;
}

/// A variable value that is set by the previously visited UiNodes during the call.
pub trait VisitedVar: 'static {
    /// The variable type.
    type Type: 'static;
}

/// Abstraction over [ContextVar], [SharedVar] or [OwnedVar].
pub trait Var<T: 'static> {
    /// The current value.
    fn get<'a>(&'a self, ctx: &'a AppContext) -> &'a T;

    /// If the value changed this update.
    fn is_new(&self, ctx: &AppContext) -> bool;
}

impl<T: 'static, V: ContextVar<Type = T>> Var<T> for V {
    fn get<'a>(&'a self, ctx: &'a AppContext) -> &'a T {
        ctx.get::<V>()
    }

    fn is_new(&self, ctx: &AppContext) -> bool {
        ctx.get_is_new::<V>()
    }
}

/// [Var] implementer that owns the value.
pub struct OwnedVar<T: 'static>(pub T);

impl<T: 'static> Var<T> for OwnedVar<T> {
    fn get(&self, _: &AppContext) -> &T {
        &self.0
    }

    fn is_new(&self, _: &AppContext) -> bool {
        false
    }
}

struct SharedVarData<T> {
    data: UnsafeCell<T>,
    borrowed: Cell<Option<AppContextId>>,
    is_new: Cell<bool>,
}

/// [Var] Rc implementer.
pub struct SharedVar<T: 'static> {
    r: Rc<SharedVarData<T>>,
}

impl<T: 'static> SharedVar<T> {
    pub(crate) fn modify(
        self,
        mut_ctx_id: AppContextId,
        modify: impl FnOnce(&mut T) + 'static,
        cleanup: &mut Vec<Box<dyn FnOnce()>>,
    ) {
        if let Some(ctx_id) = self.r.borrowed.get() {
            if ctx_id != mut_ctx_id {
                panic!(
                    "cannot set `Var<{}>` because it is borrowed in a different context",
                    type_name::<T>()
                )
            }
            self.r.borrowed.set(None);
        }

        // SAFETY: This is safe because borrows are bound to a context that
        // is the only place where the value can be changed and this change is
        // only applied when the context is mut.
        modify(unsafe { &mut *self.r.data.get() });

        cleanup.push(Box::new(move || self.r.is_new.set(false)));
    }
}

impl<T: 'static> Clone for SharedVar<T> {
    fn clone(&self) -> Self {
        SharedVar { r: Rc::clone(&self.r) }
    }
}

impl<T: 'static> Var<T> for SharedVar<T> {
    fn get(&self, ctx: &AppContext) -> &T {
        let id = ctx.id();
        if let Some(ctx_id) = self.r.borrowed.get() {
            if ctx_id != id {
                panic!(
                    "`SharedVar<{}>` is already borrowed in a different `AppContext`",
                    type_name::<T>()
                )
            }
        } else {
            self.r.borrowed.set(Some(id));
        }

        // SAFETY: This is safe because borrows are bound to a context that
        // is the only place where the value can be changed and this change is
        // only applied when the context is mut.
        unsafe { &*self.r.data.get() }
    }

    fn is_new(&self, _: &AppContext) -> bool {
        self.r.is_new.get()
    }
}

pub trait IntoVar<T: 'static> {
    type Var: Var<T> + 'static;

    fn into_var(self) -> Self::Var;
}

/// Does nothing. `[Var]<T>` already implements `Value<T>`.
impl<T: 'static> IntoVar<T> for SharedVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

/// Wraps the value in an `[Owned]<T>` value.
impl<T: 'static> IntoVar<T> for T {
    type Var = OwnedVar<T>;

    fn into_var(self) -> OwnedVar<T> {
        OwnedVar(self)
    }
}
