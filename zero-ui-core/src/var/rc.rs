use std::rc::{Rc, Weak};

use super::{animation::AnimateModifyInfo, *};

struct Data<T> {
    value: T,
    last_update: VarUpdateId,
    hooks: Vec<VarHook>,
    animation: AnimateModifyInfo,
}

/// Reference counted read/write variable.
///
/// This is the primary variable type, it can be instantiated using the [`var`] and [`var_from`] functions.
#[derive(Clone)]
pub struct RcVar<T: VarValue>(Rc<RefCell<Data<T>>>);

/// Weak reference to a [`RcVar<T>`].
#[derive(Clone)]
pub struct WeakRcVar<T: VarValue>(Weak<RefCell<Data<T>>>);

/// New ref counted read/write variable with initial `value`.
pub fn var<T: VarValue>(value: T) -> RcVar<T> {
    RcVar(Rc::new(RefCell::new(Data {
        value,
        last_update: VarUpdateId::never(),
        hooks: vec![],
        animation: AnimateModifyInfo::never(),
    })))
}

/// New ref counted read/write variable with initial value converted from `source`.
pub fn var_from<T: VarValue, U: Into<T>>(source: U) -> RcVar<T> {
    var(source.into())
}

impl<T: VarValue> WeakRcVar<T> {
    /// New reference to nothing.
    pub fn new() -> Self {
        Self(Weak::new())
    }
}

impl<T: VarValue> Default for WeakRcVar<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: VarValue> crate::private::Sealed for RcVar<T> {}

impl<T: VarValue> crate::private::Sealed for WeakRcVar<T> {}

impl<T: VarValue> AnyVar for RcVar<T> {
    fn clone_any(&self) -> BoxedAnyVar {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn double_boxed_any(self: Box<Self>) -> Box<dyn Any> {
        let me: BoxedVar<T> = self;
        Box::new(me)
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
        self.0.borrow().last_update
    }

    fn capabilities(&self) -> VarCapabilities {
        VarCapabilities::MODIFY
    }

    fn hook(&self, pos_modify_action: Box<dyn Fn(&Vars, &mut Updates, &dyn AnyVarValue) -> bool>) -> VarHandle {
        let (hook, weak) = VarHandle::new(pos_modify_action);
        self.0.borrow_mut().hooks.push(weak);
        hook
    }

    fn strong_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }

    fn weak_count(&self) -> usize {
        Rc::weak_count(&self.0)
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        Box::new(self.clone())
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        Box::new(WeakRcVar(Rc::downgrade(&self.0)))
    }

    fn is_animating(&self) -> bool {
        self.0.borrow().animation.is_animating()
    }

    fn var_ptr(&self) -> VarPtr {
        VarPtr::new_rc(&self.0)
    }
}

impl<T: VarValue> AnyWeakVar for WeakRcVar<T> {
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
        self.0.upgrade().map(|rc| Box::new(RcVar(rc)) as _)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<T: VarValue> IntoVar<T> for RcVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T: VarValue> RcVar<T> {
    fn modify_impl(&self, vars: &Vars, modify: impl FnOnce(&mut VarModifyValue<T>) + 'static) -> Result<(), VarIsReadOnlyError> {
        let me = self.clone();
        vars.schedule_update(Box::new(move |vars, updates| {
            let mut data = me.0.borrow_mut();
            let data = &mut *data;

            let curr_anim = vars.current_animation();
            if curr_anim.importance() < data.animation.importance() {
                return;
            }

            data.animation = curr_anim;

            let mut value = VarModifyValue {
                update_id: vars.update_id(),
                value: &mut data.value,
                touched: false,
            };
            modify(&mut value);
            if value.touched {
                data.last_update = value.update_id;
                data.hooks.retain(|h| h.call(vars, updates, &data.value))
            }
        }));
        Ok(())
    }
}

impl<T: VarValue> Var<T> for RcVar<T> {
    type ReadOnly = types::ReadOnlyVar<T, Self>;

    type ActualVar = Self;

    type Downgrade = WeakRcVar<T>;

    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        read(&self.0.borrow().value)
    }

    fn modify<V, F>(&self, vars: &V, modify: F) -> Result<(), VarIsReadOnlyError>
    where
        V: WithVars,
        F: FnOnce(&mut VarModifyValue<T>) + 'static,
    {
        vars.with_vars(move |vars| self.modify_impl(vars, modify))
    }

    fn actual_var(&self) -> Self {
        self.clone()
    }

    fn downgrade(&self) -> WeakRcVar<T> {
        WeakRcVar(Rc::downgrade(&self.0))
    }

    fn into_value(self) -> T {
        match Rc::try_unwrap(self.0) {
            Ok(data) => data.into_inner().value,
            Err(rc) => Self(rc).get(),
        }
    }

    fn read_only(&self) -> Self::ReadOnly {
        types::ReadOnlyVar::new(self.clone())
    }
}

impl<T: VarValue> WeakVar<T> for WeakRcVar<T> {
    type Upgrade = RcVar<T>;

    fn upgrade(&self) -> Option<RcVar<T>> {
        self.0.upgrade().map(|rc| RcVar(rc))
    }
}
