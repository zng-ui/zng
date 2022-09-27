use std::marker::PhantomData;

use super::*;

/// See [`Var::read_only`].
pub struct ReadOnlyVar<T, V>(PhantomData<T>, V);

/// Weak [`ReadOnlyVar<T>`].
pub struct WeakReadOnlyVar<T, V>(PhantomData<T>, V);

impl<T: VarValue, V: Var<T>> ReadOnlyVar<T, V> {
    pub(super) fn new(var: V) -> Self {
        ReadOnlyVar(PhantomData, var)
    }
}

impl<T: VarValue, V: Var<T>> Clone for ReadOnlyVar<T, V> {
    fn clone(&self) -> Self {
        Self(PhantomData, self.1.clone())
    }
}

impl<T: VarValue, V: WeakVar<T>> Clone for WeakReadOnlyVar<T, V> {
    fn clone(&self) -> Self {
        Self(PhantomData, self.1.clone())
    }
}

impl<T: VarValue, V: Var<T>> crate::private::Sealed for ReadOnlyVar<T, V> {}
impl<T: VarValue, V: WeakVar<T>> crate::private::Sealed for WeakReadOnlyVar<T, V> {}

impl<T: VarValue, V: Var<T>> AnyVar for ReadOnlyVar<T, V> {
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
        self.1.var_type_id()
    }

    fn get_any(&self) -> Box<dyn AnyVarValue> {
        self.1.get_any()
    }

    fn set_any(&self, _: &Vars, _: Box<dyn AnyVarValue>) -> Result<(), VarIsReadOnlyError> {
        Err(VarIsReadOnlyError {
            capabilities: self.capabilities(),
        })
    }

    fn last_update(&self) -> VarUpdateId {
        self.1.last_update()
    }

    fn capabilities(&self) -> VarCapabilities {
        self.1.capabilities().as_read_only()
    }

    fn hook(&self, pos_modify_action: Box<dyn Fn(&Vars, &mut Updates, &dyn AnyVarValue) -> bool>) -> VarHandle {
        self.1.hook(pos_modify_action)
    }

    fn subscribe(&self, widget_id: WidgetId) -> VarHandle {
        self.1.subscribe(widget_id)
    }

    fn strong_count(&self) -> usize {
        self.1.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.1.weak_count()
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        Box::new(self.clone())
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        Box::new(WeakReadOnlyVar(PhantomData, self.1.downgrade()))
    }

    fn is_animating(&self) -> bool {
        self.1.is_animating()
    }
}

impl<T: VarValue, V: WeakVar<T>> AnyWeakVar for WeakReadOnlyVar<T, V> {
    fn clone_any(&self) -> BoxedAnyWeakVar {
        Box::new(self.clone())
    }

    fn strong_count(&self) -> usize {
        self.1.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.1.weak_count()
    }

    fn upgrade_any(&self) -> Option<BoxedAnyVar> {
        self.1.upgrade().map(|inner| Box::new(inner.read_only()) as _)
    }
}

impl<T: VarValue, V: Var<T>> IntoVar<T> for ReadOnlyVar<T, V> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T: VarValue, V: Var<T>> Var<T> for ReadOnlyVar<T, V> {
    type ReadOnly = Self;

    type ActualVar = Self;

    type Downgrade = WeakReadOnlyVar<T, V::Downgrade>;

    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.1.with(read)
    }

    fn modify<V2, F>(&self, _: &V2, _: F) -> Result<(), VarIsReadOnlyError>
    where
        V2: WithVars,
        F: FnOnce(&mut VarModifyValue<T>) + 'static,
    {
        Err(VarIsReadOnlyError {
            capabilities: self.capabilities(),
        })
    }

    fn boxed(self) -> BoxedVar<T>
    where
        Self: Sized,
    {
        Box::new(self)
    }

    fn actual_var(&self) -> Self {
        self.clone()
    }

    fn downgrade(&self) -> Self::Downgrade {
        WeakReadOnlyVar(PhantomData, self.1.downgrade())
    }

    fn into_value(self) -> T {
        self.1.into_value()
    }

    fn read_only(&self) -> Self::ReadOnly {
        self.clone()
    }
}

impl<T: VarValue, V: WeakVar<T>> WeakVar<T> for WeakReadOnlyVar<T, V> {
    type Upgrade = <V::Upgrade as Var<T>>::ReadOnly;

    fn upgrade(&self) -> Option<Self::Upgrade> {
        self.1.upgrade().map(|inner| inner.read_only())
    }
}

/// Read-only [`RcVar<T>`].
pub type ReadOnlyRcVar<T> = ReadOnlyVar<T, RcVar<T>>;
