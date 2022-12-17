use std::sync::{Arc, Weak};

use super::{util::VarData, *};

/// Reference counted read/write variable.
///
/// This is the primary variable type, it can be instantiated using the [`var`] and [`var_from`] functions.
#[derive(Clone)]
pub struct ArcVar<T: VarValue>(Arc<VarData<T>>);

/// Weak reference to a [`ArcVar<T>`].
#[derive(Clone)]
pub struct WeakArcVar<T: VarValue>(Weak<VarData<T>>);

/// New ref counted read/write variable with initial `value`.
pub fn var<T: VarValue>(value: T) -> ArcVar<T> {
    ArcVar(Arc::new(VarData::new(value)))
}

/// New ref counted read/write variable with initial value converted from `source`.
pub fn var_from<T: VarValue, U: Into<T>>(source: U) -> ArcVar<T> {
    var(source.into())
}

/// New ref counted read/write variable with default initial value.
pub fn var_default<T: VarValue + Default>() -> ArcVar<T> {
    var(T::default())
}

impl<T: VarValue> WeakArcVar<T> {
    /// New reference to nothing.
    pub fn new() -> Self {
        Self(Weak::new())
    }
}

impl<T: VarValue> Default for WeakArcVar<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: VarValue> crate::private::Sealed for ArcVar<T> {}

impl<T: VarValue> crate::private::Sealed for WeakArcVar<T> {}

impl<T: VarValue> AnyVar for ArcVar<T> {
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
        self.modify(vars, var_set_any(value));
        Ok(())
    }

    fn last_update(&self) -> VarUpdateId {
        self.0.last_update()
    }

    fn capabilities(&self) -> VarCapabilities {
        VarCapabilities::MODIFY
    }

    fn hook(&self, pos_modify_action: Box<dyn Fn(&Vars, &mut Updates, &dyn AnyVarValue) -> bool + Send + Sync>) -> VarHandle {
        self.0.push_hook(pos_modify_action)
    }

    fn strong_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }

    fn weak_count(&self) -> usize {
        Arc::weak_count(&self.0)
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        Box::new(self.clone())
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        Box::new(WeakArcVar(Arc::downgrade(&self.0)))
    }

    fn is_animating(&self) -> bool {
        self.0.is_animating()
    }

    fn modify_importance(&self) -> usize {
        self.0.modify_importance()
    }

    fn var_ptr(&self) -> VarPtr {
        VarPtr::new_arc(&self.0)
    }
}

impl<T: VarValue> AnyWeakVar for WeakArcVar<T> {
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
        self.0.upgrade().map(|rc| Box::new(ArcVar(rc)) as _)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<T: VarValue> IntoVar<T> for ArcVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T: VarValue> ArcVar<T> {
    fn modify_impl(&self, vars: &Vars, modify: impl FnOnce(&mut Cow<T>) + 'static) -> Result<(), VarIsReadOnlyError> {
        let me = self.clone();
        vars.schedule_update(Box::new(move |vars, updates| {
            me.0.apply_modify(vars, updates, modify);
        }));
        Ok(())
    }

    impl_infallible_write! {
        for<T>
    }
}

impl<T: VarValue> Var<T> for ArcVar<T> {
    type ReadOnly = types::ReadOnlyVar<T, Self>;

    type ActualVar = Self;

    type Downgrade = WeakArcVar<T>;

    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.0.with(read)
    }

    fn modify<V, F>(&self, vars: &V, modify: F) -> Result<(), VarIsReadOnlyError>
    where
        V: WithVars,
        F: FnOnce(&mut Cow<T>) + 'static,
    {
        vars.with_vars(move |vars| self.modify_impl(vars, modify))
    }

    fn actual_var(self) -> Self {
        self
    }

    fn downgrade(&self) -> WeakArcVar<T> {
        WeakArcVar(Arc::downgrade(&self.0))
    }

    fn into_value(self) -> T {
        match Arc::try_unwrap(self.0) {
            Ok(data) => data.into_value(),
            Err(rc) => Self(rc).get(),
        }
    }

    fn read_only(&self) -> Self::ReadOnly {
        types::ReadOnlyVar::new(self.clone())
    }
}

impl<T: VarValue> WeakVar<T> for WeakArcVar<T> {
    type Upgrade = ArcVar<T>;

    fn upgrade(&self) -> Option<ArcVar<T>> {
        self.0.upgrade().map(|rc| ArcVar(rc))
    }
}
