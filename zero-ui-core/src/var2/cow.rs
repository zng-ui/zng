use std::{
    mem,
    rc::{Rc, Weak},
};

use super::*;

enum Data<T, S> {
    Source {
        source: S,
        source_handle: VarHandle,
        hooks: Vec<VarHook>,
    },
    Value {
        value: T,
        last_update: VarUpdateId,
        hooks: Vec<VarHook>,
    },
}

/// See [`Var::cow`].
pub struct RcCowVar<T, S>(Rc<RefCell<Data<T, S>>>);

/// Weak reference to a [`RcCowVar<T>`].
pub struct WeakCowVar<T, S>(Weak<RefCell<Data<T, S>>>);

impl<T: VarValue, S: Var<T>> RcCowVar<T, S> {
    pub(super) fn new(source: S) -> Self {
        let cow = Rc::new(RefCell::new(Data::Source {
            source,
            source_handle: VarHandle::dummy(),
            hooks: vec![],
        }));
        {
            let mut data = cow.borrow_mut();
            if let Data::Source { source, source_handle, .. } = &mut *data {
                let weak_cow = Rc::downgrade(&cow);
                *source_handle = source.hook(Box::new(move |vars, updates, value| {
                    if let Some(cow) = weak_cow.upgrade() {
                        match &mut *cow.borrow_mut() {
                            Data::Source { hooks, .. } => {
                                hooks.retain(|h| h.call(vars, updates, value));
                                true
                            }
                            Data::Value { .. } => false,
                        }
                    } else {
                        false
                    }
                }));
            }
        }
        Self(cow)
    }

    fn modify_impl(&self, vars: &Vars, modify: impl FnOnce(&mut VarModifyValue<T>) + 'static) -> Result<(), VarIsReadOnlyError> {
        let me = self.clone();
        vars.schedule_update(Box::new(move |vars, updates| {
            let mut data = me.0.borrow_mut();
            let data = &mut *data;

            match data {
                Data::Source { source, hooks, .. } => {
                    let mut value = source.get();
                    let mut mod_value = VarModifyValue {
                        update_id: vars.update_id(),
                        value: &mut value,
                        touched: false,
                    };
                    modify(&mut mod_value);
                    if mod_value.touched {
                        *data = Data::Value {
                            last_update: mod_value.update_id,
                            value,
                            hooks: mem::take(hooks),
                        }
                    }
                }
                Data::Value { value, last_update, hooks } => {
                    let mut value = VarModifyValue {
                        update_id: vars.update_id(),
                        value,
                        touched: false,
                    };
                    modify(&mut value);

                    if value.touched {
                        *last_update = value.update_id;
                        hooks.retain(|h| h.call(vars, updates, value.value))
                    }
                }
            }
        }));
        Ok(())
    }

    fn push_hook(&self, weak: VarHook) {
        let mut data = self.0.borrow_mut();
        match &mut *data {
            Data::Source { hooks, .. } => {
                hooks.push(weak);
            }
            Data::Value { hooks, .. } => {
                hooks.push(weak);
            }
        }
    }
}

impl<T, S> Clone for RcCowVar<T, S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T, S> Clone for WeakCowVar<T, S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: VarValue, S: Var<T>> crate::private::Sealed for RcCowVar<T, S> {}
impl<T: VarValue, S: Var<T>> crate::private::Sealed for WeakCowVar<T, S> {}

impl<T: VarValue, S: Var<T>> AnyVar for RcCowVar<T, S> {
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
        match &*self.0.borrow() {
            Data::Source { source, .. } => source.last_update(),
            Data::Value { last_update, .. } => *last_update,
        }
    }

    fn capabilities(&self) -> VarCapabilities {
        VarCapabilities::MODIFY
    }

    fn hook(&self, pos_modify_action: Box<dyn Fn(&Vars, &mut Updates, &dyn AnyVarValue) -> bool>) -> VarHandle {
        let (handle, weak) = VarHandle::new(pos_modify_action);
        self.push_hook(weak);
        handle
    }

    fn strong_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }

    fn weak_count(&self) -> usize {
        Rc::weak_count(&self.0)
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        self.clone_any()
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        Box::new(WeakCowVar(Rc::downgrade(&self.0)))
    }
}

impl<T: VarValue, S: Var<T>> AnyWeakVar for WeakCowVar<T, S> {
    fn clone_any(&self) -> BoxedAnyWeakVar {
        Box::new(self.clone())
    }

    fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.0.weak_count()
    }

    fn upgrade_any(&self) -> Option<BoxedAnyVar> {
        self.0.upgrade().map(|rc| Box::new(RcCowVar(rc)) as _)
    }
}

impl<T: VarValue, S: Var<T>> IntoVar<T> for RcCowVar<T, S> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T: VarValue, S: Var<T>> Var<T> for RcCowVar<T, S> {
    type ReadOnly = types::ReadOnlyVar<T, Self>;

    type ActualVar = Self;

    type Downgrade = WeakCowVar<T, S>;

    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        match &*self.0.borrow() {
            Data::Source { source, .. } => source.with(read),
            Data::Value { value, .. } => read(value),
        }
    }

    fn modify<V, F>(&self, vars: &V, modify: F) -> Result<(), VarIsReadOnlyError>
    where
        V: WithVars,
        F: FnOnce(&mut VarModifyValue<T>) + 'static,
    {
        vars.with_vars(|vars| self.modify_impl(vars, modify))
    }

    fn actual_var(&self) -> Self {
        self.clone()
    }

    fn downgrade(&self) -> Self::Downgrade {
        WeakCowVar(Rc::downgrade(&self.0))
    }

    fn into_value(self) -> T {
        match Rc::try_unwrap(self.0) {
            Ok(state) => match state.into_inner() {
                Data::Source { source, .. } => source.into_value(),
                Data::Value { value, .. } => value,
            },
            Err(rc) => Self(rc).get(),
        }
    }

    fn read_only(&self) -> Self::ReadOnly {
        types::ReadOnlyVar::new(self.clone())
    }
}

impl<T: VarValue, S: Var<T>> WeakVar<T> for WeakCowVar<T, S> {
    type Upgrade = RcCowVar<T, S>;

    fn upgrade(&self) -> Option<Self::Upgrade> {
        self.0.upgrade().map(|rc| RcCowVar(rc))
    }
}
