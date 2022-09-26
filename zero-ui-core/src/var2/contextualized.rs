use std::{cell::Ref, marker::PhantomData, rc::Weak};

use super::*;

/// Represents a variable that delays initialization until the first usage.
///
/// Usage that initializes are all [`AnyVar`] and [`Var<T>`] methods except `read_only`, `downgrade` and `boxed`.
///
/// Clones of this variable are always not initialized and re-init on first usage.
pub struct ContextualizedVar<T, S> {
    _type: PhantomData<T>,
    init: Rc<dyn Fn() -> S>,
    actual: RefCell<Option<S>>,
}

impl<T: VarValue, S: Var<T>> ContextualizedVar<T, S> {
    /// New with initialization function.
    pub fn new(init: Rc<dyn Fn() -> S>) -> Self {
        Self {
            _type: PhantomData,
            init,
            actual: RefCell::new(None),
        }
    }

    /// Borrow/initialize the actual var.
    pub fn borrow_init(&self) -> Ref<S> {
        let act = self.actual.borrow();
        if act.is_some() {
            return Ref::map(act, |opt| opt.as_ref().unwrap());
        }

        drop(act);
        *self.actual.borrow_mut() = Some((self.init)());

        let act = self.actual.borrow();
        Ref::map(act, |opt| opt.as_ref().unwrap())
    }
}

/// Weak var that upgrades to an uninitialized [`ContextualizedVar<T, S>`].
pub struct WeakContextualizedVar<T, S> {
    _type: PhantomData<T>,
    init: Weak<dyn Fn() -> S>,
}
impl<T: VarValue, S: Var<T>> WeakContextualizedVar<T, S> {
    /// New with weak init function.
    pub fn new(init: Weak<dyn Fn() -> S>) -> Self {
        Self { _type: PhantomData, init }
    }
}

impl<T: VarValue, S: Var<T>> Clone for ContextualizedVar<T, S> {
    fn clone(&self) -> Self {
        Self {
            _type: PhantomData,
            init: self.init.clone(),
            actual: RefCell::new(None),
        }
    }
}
impl<T: VarValue, S: Var<T>> Clone for WeakContextualizedVar<T, S> {
    fn clone(&self) -> Self {
        Self {
            _type: PhantomData,
            init: self.init.clone(),
        }
    }
}

impl<T: VarValue, S: Var<T>> crate::private::Sealed for ContextualizedVar<T, S> {}
impl<T: VarValue, S: Var<T>> crate::private::Sealed for WeakContextualizedVar<T, S> {}

impl<T: VarValue, S: Var<T>> AnyVar for ContextualizedVar<T, S> {
    fn clone_any(&self) -> BoxedAnyVar {
        Box::new(self.clone())
    }

    fn var_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn get_any(&self) -> Box<dyn AnyVarValue> {
        Box::new(self.get())
    }

    fn set_any(&self, vars: &Vars, value: Box<dyn AnyVarValue>) -> Result<(), VarIsReadOnlyError> {
        self.modify(vars, var_set_any(value))
    }

    fn last_update(&self) -> VarUpdateId {
        self.borrow_init().last_update()
    }

    fn capabilities(&self) -> VarCapabilities {
        self.borrow_init().capabilities()
    }

    fn hook(&self, pos_modify_action: Box<dyn Fn(&Vars, &mut Updates, &dyn AnyVarValue) -> bool>) -> VarHandle {
        self.borrow_init().hook(pos_modify_action)
    }

    fn strong_count(&self) -> usize {
        Rc::strong_count(&self.init)
    }

    fn weak_count(&self) -> usize {
        Rc::weak_count(&self.init)
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        self.borrow_init().actual_var_any()
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        Box::new(self.downgrade())
    }
}
impl<T: VarValue, S: Var<T>> AnyWeakVar for WeakContextualizedVar<T, S> {
    fn clone_any(&self) -> BoxedAnyWeakVar {
        Box::new(self.clone())
    }

    fn strong_count(&self) -> usize {
        self.init.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.init.weak_count()
    }

    fn upgrade_any(&self) -> Option<BoxedAnyVar> {
        self.upgrade().map(|c| Box::new(c) as _)
    }
}

impl<T: VarValue, S: Var<T>> IntoVar<T> for ContextualizedVar<T, S> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T: VarValue, S: Var<T>> Var<T> for ContextualizedVar<T, S> {
    type ReadOnly = types::ReadOnlyVar<T, Self>;

    type ActualVar = S::ActualVar;

    type Downgrade = WeakContextualizedVar<T, S>;

    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.borrow_init().with(read)
    }

    fn modify<V, F>(&self, vars: &V, modify: F) -> Result<(), VarIsReadOnlyError>
    where
        V: WithVars,
        F: FnOnce(&mut VarModifyValue<T>) + 'static,
    {
        self.borrow_init().modify(vars, modify)
    }

    fn actual_var(&self) -> Self::ActualVar {
        self.borrow_init().actual_var()
    }

    fn downgrade(&self) -> Self::Downgrade {
        WeakContextualizedVar::new(Rc::downgrade(&self.init))
    }

    fn into_value(self) -> T {
        match self.actual.into_inner() {
            Some(act) => act.into_value(),
            None => (self.init)().into_value(),
        }
    }

    fn read_only(&self) -> Self::ReadOnly {
        types::ReadOnlyVar::new(self.clone())
    }
}
impl<T: VarValue, S: Var<T>> WeakVar<T> for WeakContextualizedVar<T, S> {
    type Upgrade = ContextualizedVar<T, S>;

    fn upgrade(&self) -> Option<Self::Upgrade> {
        self.init.upgrade().map(ContextualizedVar::new)
    }
}
