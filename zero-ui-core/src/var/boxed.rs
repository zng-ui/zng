use super::*;

/// A dynamic [`Var<T>`] in a box.
pub type BoxedVar<T> = Box<dyn VarBoxed<T>>;

/// Represents a weak reference to a [`Var<T>`].
pub type BoxedWeakVar<T> = Box<dyn WeakVarBoxed<T>>;

/// Represents a type erased [`Var<T>`].
pub type BoxedAnyVar = Box<dyn AnyVar>;

/// Represents a type erased weak reference to a [`Var<T>`].
pub type BoxedAnyWeakVar = Box<dyn AnyWeakVar>;

impl<T: VarValue> Clone for BoxedWeakVar<T> {
    fn clone(&self) -> Self {
        self.clone_boxed()
    }
}

impl Clone for BoxedAnyVar {
    fn clone(&self) -> Self {
        self.clone_any()
    }
}

impl Clone for BoxedAnyWeakVar {
    fn clone(&self) -> Self {
        self.clone_any()
    }
}

#[doc(hidden)]
pub trait VarBoxed<T: VarValue>: AnyVar {
    fn clone_boxed(&self) -> BoxedVar<T>;
    fn with_boxed(&self, read: &mut dyn FnMut(&T));
    fn modify_boxed(&self, vars: &Vars, modify: Box<dyn FnOnce(&mut VarModifyValue<T>)>) -> Result<(), VarIsReadOnlyError>;
    fn actual_var_boxed(&self) -> BoxedVar<T>;
    fn downgrade_boxed(&self) -> BoxedWeakVar<T>;
    fn read_only_boxed(&self) -> BoxedVar<T>;
    fn boxed_any_boxed(self: Box<Self>) -> BoxedAnyVar;
}
impl<T: VarValue, V: Var<T>> VarBoxed<T> for V {
    fn clone_boxed(&self) -> BoxedVar<T> {
        self.clone().boxed()
    }

    fn with_boxed(&self, read: &mut dyn FnMut(&T)) {
        self.with(read)
    }

    fn modify_boxed(&self, vars: &Vars, modify: Box<dyn FnOnce(&mut VarModifyValue<T>)>) -> Result<(), VarIsReadOnlyError> {
        self.modify(vars, modify)
    }

    fn actual_var_boxed(&self) -> BoxedVar<T> {
        self.actual_var().boxed()
    }

    fn downgrade_boxed(&self) -> BoxedWeakVar<T> {
        self.downgrade().boxed()
    }

    fn read_only_boxed(&self) -> BoxedVar<T> {
        self.read_only().boxed()
    }

    fn boxed_any_boxed(self: Box<Self>) -> BoxedAnyVar {
        self
    }
}

#[doc(hidden)]
pub trait WeakVarBoxed<T: VarValue>: AnyWeakVar {
    fn clone_boxed(&self) -> BoxedWeakVar<T>;
    fn upgrade_boxed(&self) -> Option<BoxedVar<T>>;
}
impl<T: VarValue, W: WeakVar<T>> WeakVarBoxed<T> for W {
    fn clone_boxed(&self) -> BoxedWeakVar<T> {
        self.clone().boxed()
    }

    fn upgrade_boxed(&self) -> Option<BoxedVar<T>> {
        self.upgrade().map(Var::boxed)
    }
}

impl<T: VarValue> crate::private::Sealed for BoxedWeakVar<T> {}

impl<T: VarValue> AnyWeakVar for BoxedWeakVar<T> {
    fn clone_any(&self) -> BoxedAnyWeakVar {
        (**self).clone_any()
    }

    fn strong_count(&self) -> usize {
        (**self).strong_count()
    }

    fn weak_count(&self) -> usize {
        (**self).weak_count()
    }

    fn upgrade_any(&self) -> Option<BoxedAnyVar> {
        (**self).upgrade_any()
    }

    fn as_any(&self) -> &dyn Any {
        (**self).as_any()
    }
}
impl<T: VarValue> WeakVar<T> for BoxedWeakVar<T> {
    type Upgrade = BoxedVar<T>;

    fn upgrade(&self) -> Option<Self::Upgrade> {
        (**self).upgrade_boxed()
    }
}

impl<T: VarValue> crate::private::Sealed for BoxedVar<T> {}

impl<T: VarValue> Clone for BoxedVar<T> {
    fn clone(&self) -> Self {
        (**self).clone_boxed()
    }
}

impl<T: VarValue> AnyVar for BoxedVar<T> {
    fn clone_any(&self) -> BoxedAnyVar {
        (**self).clone_any()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn double_boxed_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn var_type_id(&self) -> TypeId {
        (**self).var_type_id()
    }

    fn get_any(&self) -> Box<dyn AnyVarValue> {
        (**self).get_any()
    }

    fn set_any(&self, vars: &Vars, value: Box<dyn AnyVarValue>) -> Result<(), VarIsReadOnlyError> {
        (**self).set_any(vars, value)
    }

    fn last_update(&self) -> VarUpdateId {
        (**self).last_update()
    }

    fn capabilities(&self) -> VarCapabilities {
        (**self).capabilities()
    }

    fn hook(&self, pos_modify_action: Box<dyn Fn(&Vars, &mut Updates, &dyn AnyVarValue) -> bool>) -> VarHandle {
        (**self).hook(pos_modify_action)
    }

    fn subscribe(&self, widget_id: WidgetId) -> VarHandle {
        (**self).subscribe(widget_id)
    }

    fn strong_count(&self) -> usize {
        (**self).strong_count()
    }

    fn weak_count(&self) -> usize {
        (**self).weak_count()
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        (**self).actual_var_any()
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        (**self).downgrade_any()
    }

    fn is_animating(&self) -> bool {
        (**self).is_animating()
    }

    fn var_ptr(&self) -> VarPtr {
        (**self).var_ptr()
    }
}

impl<T: VarValue> IntoVar<T> for BoxedVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T: VarValue> Var<T> for BoxedVar<T> {
    type ReadOnly = BoxedVar<T>;

    type ActualVar = BoxedVar<T>;

    type Downgrade = BoxedWeakVar<T>;

    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let mut read = Some(read);
        let mut result = None;
        (**self).with_boxed(&mut |var_value| match read.take() {
            Some(read) => {
                result = Some(read(var_value));
            }
            None => unreachable!(),
        });

        match result.take() {
            Some(r) => r,
            None => unreachable!(),
        }
    }

    fn modify<V, F>(&self, vars: &V, modify: F) -> Result<(), VarIsReadOnlyError>
    where
        V: WithVars,
        F: FnOnce(&mut VarModifyValue<T>) + 'static,
    {
        vars.with_vars(|vars| (**self).modify_boxed(vars, Box::new(modify)))
    }

    fn boxed(self) -> BoxedVar<T> {
        self
    }

    fn boxed_any(self) -> BoxedAnyVar
    where
        Self: Sized,
    {
        // fix after https://github.com/rust-lang/rust/issues/65991
        self.clone_any()
    }

    fn actual_var(&self) -> BoxedVar<T> {
        (**self).actual_var_boxed()
    }

    fn downgrade(&self) -> BoxedWeakVar<T> {
        (**self).downgrade_boxed()
    }

    fn into_value(self) -> T {
        self.get()
    }

    fn read_only(&self) -> Self::ReadOnly {
        if self.capabilities().is_always_read_only() {
            self.clone()
        } else {
            (**self).read_only_boxed()
        }
    }
}
