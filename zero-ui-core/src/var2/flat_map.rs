use std::{
    marker::PhantomData,
    rc::{Rc, Weak},
};

use super::*;

struct Data<T, V> {
    _t: PhantomData<T>,
    var: V,
    source_handle: VarHandle,
    last_update: VarUpdateId,
    var_handle: VarHandle,
    hooks: Vec<VarHook>,
}

/// See [`Var::flat_map`].
pub struct RcFlatMapVar<T, V>(Rc<RefCell<Data<T, V>>>);

/// Weak reference to a [`RcFlatMapVar<T, V>`].
pub struct WeakFlatMapVar<T, V>(Weak<RefCell<Data<T, V>>>);

impl<T, V> RcFlatMapVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
    /// New.
    pub fn new<I: VarValue>(source: &impl Var<I>, mut map: impl FnMut(&I) -> V + 'static) -> Self {
        let flat = Rc::new(RefCell::new(Data {
            _t: PhantomData,
            var: source.with(&mut map),
            last_update: VarUpdateId::never(),
            source_handle: VarHandle::dummy(),
            var_handle: VarHandle::dummy(),
            hooks: vec![],
        }));

        {
            let mut data = flat.borrow_mut();
            let weak_flat = Rc::downgrade(&flat);
            let map = RefCell::new(map);
            data.var_handle = data.var.hook(RcFlatMapVar::on_var_hook(weak_flat.clone()));
            data.source_handle = source.hook(Box::new(move |vars, updates, value| {
                if let Some(flat) = weak_flat.upgrade() {
                    if let Some(value) = value.as_any().downcast_ref() {
                        let mut data = flat.borrow_mut();
                        let data = &mut *data;
                        data.var = map.borrow_mut()(value);
                        data.var_handle = data.var.hook(RcFlatMapVar::on_var_hook(weak_flat.clone()));
                        data.last_update = vars.update_id();
                        data.var.with(|value| {
                            data.hooks.retain(|h| h.call(vars, updates, value));
                        });
                    }
                    true
                } else {
                    false
                }
            }));
        }

        Self(flat)
    }

    fn on_var_hook(weak_flat: Weak<RefCell<Data<T, V>>>) -> Box<dyn Fn(&Vars, &mut Updates, &dyn AnyVarValue) -> bool> {
        Box::new(move |vars, updates, value| {
            if let Some(flat) = weak_flat.upgrade() {
                let mut data = flat.borrow_mut();
                data.last_update = vars.update_id();
                data.hooks.retain(|h| h.call(vars, updates, value));
                true
            } else {
                false
            }
        })
    }
}

impl<T, V> Clone for RcFlatMapVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T, V> Clone for WeakFlatMapVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T, V> crate::private::Sealed for RcFlatMapVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
}

impl<T, V> crate::private::Sealed for WeakFlatMapVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
}

impl<T, V> AnyVar for RcFlatMapVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
    fn clone_any(&self) -> BoxedAnyVar {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_boxed_any(self: Box<Self>) -> Box<dyn Any> {
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
        self.0.borrow().var.capabilities() | VarCapabilities::CAP_CHANGE
    }

    fn hook(&self, pos_modify_action: Box<dyn Fn(&Vars, &mut Updates, &dyn AnyVarValue) -> bool>) -> VarHandle {
        let (handle, weak_handle) = VarHandle::new(pos_modify_action);
        self.0.borrow_mut().hooks.push(weak_handle);
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
        Box::new(self.downgrade())
    }
}

impl<T, V> AnyWeakVar for WeakFlatMapVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
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
        self.0.upgrade().map(|rc| Box::new(RcFlatMapVar(rc)) as _)
    }
}

impl<T, V> IntoVar<T> for RcFlatMapVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T, V> Var<T> for RcFlatMapVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
    type ReadOnly = types::ReadOnlyVar<T, Self>;

    type ActualVar = Self;

    type Downgrade = WeakFlatMapVar<T, V>;

    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.0.borrow().var.with(read)
    }

    fn modify<V2, F>(&self, vars: &V2, modify: F) -> Result<(), VarIsReadOnlyError>
    where
        V2: WithVars,
        F: FnOnce(&mut VarModifyValue<T>) + 'static,
    {
        self.0.borrow().var.modify(vars, modify)
    }

    fn actual_var(&self) -> Self {
        self.clone()
    }

    fn downgrade(&self) -> Self::Downgrade {
        WeakFlatMapVar(Rc::downgrade(&self.0))
    }

    fn into_value(self) -> T {
        match Rc::try_unwrap(self.0) {
            Ok(state) => state.into_inner().var.into_value(),
            Err(rc) => Self(rc).get(),
        }
    }

    fn read_only(&self) -> Self::ReadOnly {
        types::ReadOnlyVar::new(self.clone())
    }
}

impl<T, V> WeakVar<T> for WeakFlatMapVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
    type Upgrade = RcFlatMapVar<T, V>;

    fn upgrade(&self) -> Option<Self::Upgrade> {
        self.0.upgrade().map(|rc| RcFlatMapVar(rc))
    }
}
