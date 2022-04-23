use super::*;

/// A dynamic [`Var<T>`] in a box.
pub type BoxedVar<T> = Box<dyn VarBoxed<T>>;

/// A dynamic [`WeakVar<T>`] in a box.
pub type BoxedWeakVar<T> = Box<dyn WeakVarBoxed<T>>;

#[doc(hidden)]
pub trait WeakVarBoxed<T: VarValue>: crate::private::Sealed {
    fn upgrade_boxed(&self) -> Option<BoxedVar<T>>;
    fn strong_count_boxed(&self) -> usize;
    fn weak_count_boxed(&self) -> usize;
    fn as_ptr_boxed(&self) -> *const ();
    fn clone_boxed(&self) -> BoxedWeakVar<T>;
}
impl<T: VarValue, W: WeakVar<T>> WeakVarBoxed<T> for W {
    #[inline]
    fn upgrade_boxed(&self) -> Option<BoxedVar<T>> {
        self.upgrade().map(|w| w.boxed())
    }
    #[inline]
    fn strong_count_boxed(&self) -> usize {
        self.strong_count()
    }
    #[inline]
    fn weak_count_boxed(&self) -> usize {
        self.weak_count()
    }
    #[inline]
    fn as_ptr_boxed(&self) -> *const () {
        self.as_ptr()
    }
    #[inline]
    fn clone_boxed(&self) -> BoxedWeakVar<T> {
        self.clone().boxed()
    }
}
impl<T: VarValue> Clone for BoxedWeakVar<T> {
    fn clone(&self) -> Self {
        self.as_ref().clone_boxed()
    }
}
impl<T: VarValue> crate::private::Sealed for BoxedWeakVar<T> {}
impl<T: VarValue> WeakVar<T> for BoxedWeakVar<T> {
    type Strong = BoxedVar<T>;

    #[inline]
    fn boxed(self) -> BoxedWeakVar<T>
    where
        Self: WeakVarBoxed<T> + Sized,
    {
        self
    }

    #[inline]
    fn upgrade(&self) -> Option<Self::Strong> {
        self.as_ref().upgrade_boxed()
    }
    #[inline]
    fn strong_count(&self) -> usize {
        self.as_ref().strong_count_boxed()
    }
    #[inline]
    fn weak_count(&self) -> usize {
        self.as_ref().weak_count_boxed()
    }
    #[inline]
    fn as_ptr(&self) -> *const () {
        self.as_ref().as_ptr_boxed()
    }
}

#[doc(hidden)]
pub trait VarBoxed<T: VarValue>: crate::private::Sealed {
    fn get_boxed<'a>(&'a self, vars: &'a VarsRead) -> &'a T;
    fn get_new_boxed<'a>(&'a self, vars: &'a Vars) -> Option<&'a T>;
    fn is_new_boxed(&self, vars: &Vars) -> bool;
    fn version_boxed<'a>(&'a self, vars: &'a VarsRead) -> VarVersion;
    fn is_read_only_boxed(&self, vars: &Vars) -> bool;
    fn is_animating_boxed(&self, vars: &VarsRead) -> bool;
    fn into_value_boxed(self: Box<Self>, vars: &VarsRead) -> T;
    fn always_read_only_boxed(&self) -> bool;
    fn is_contextual_boxed(&self) -> bool;
    fn can_update_boxed(&self) -> bool;
    fn modify_boxed(&self, vars: &Vars, modify: Box<dyn FnOnce(VarModify<T>)>) -> Result<(), VarIsReadOnly>;
    fn set_boxed(&self, vars: &Vars, new_value: T) -> Result<(), VarIsReadOnly>;
    fn clone_boxed(&self) -> BoxedVar<T>;
    fn strong_count_boxed(&self) -> usize;
    fn update_mask_boxed(&self, vars: &VarsRead) -> UpdateMask;
    fn is_rc_boxed(&self) -> bool;
    fn downgrade_boxed(&self) -> Option<BoxedWeakVar<T>>;
    fn weak_count_boxed(&self) -> usize;
    fn as_ptr_boxed(&self) -> *const ();
}
impl<T: VarValue, V: Var<T>> VarBoxed<T> for V {
    #[inline]
    fn get_boxed<'a>(&'a self, vars: &'a VarsRead) -> &'a T {
        self.get(vars)
    }

    #[inline]
    fn get_new_boxed<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        self.get_new(vars)
    }

    #[inline]
    fn is_new_boxed(&self, vars: &Vars) -> bool {
        self.is_new(vars)
    }

    #[inline]
    fn into_value_boxed(self: Box<Self>, vars: &VarsRead) -> T {
        self.into_value(vars)
    }

    #[inline]
    fn version_boxed<'a>(&'a self, vars: &'a VarsRead) -> VarVersion {
        self.version(vars)
    }

    #[inline]
    fn is_read_only_boxed(&self, vars: &Vars) -> bool {
        self.is_read_only(vars)
    }

    #[inline]
    fn is_animating_boxed(&self, vars: &VarsRead) -> bool {
        self.is_animating(vars)
    }

    #[inline]
    fn always_read_only_boxed(&self) -> bool {
        self.always_read_only()
    }

    #[inline]
    fn is_contextual_boxed(&self) -> bool {
        self.is_contextual()
    }

    #[inline]
    fn can_update_boxed(&self) -> bool {
        self.can_update()
    }

    #[inline]
    fn modify_boxed(&self, vars: &Vars, modify: Box<dyn FnOnce(VarModify<T>)>) -> Result<(), VarIsReadOnly> {
        self.modify(vars, modify)
    }

    #[inline]
    fn set_boxed(&self, vars: &Vars, new_value: T) -> Result<(), VarIsReadOnly> {
        self.set(vars, new_value)
    }

    #[inline]
    fn clone_boxed(&self) -> BoxedVar<T> {
        self.clone().boxed()
    }

    #[inline]
    fn strong_count_boxed(&self) -> usize {
        self.strong_count()
    }

    #[inline]
    fn update_mask_boxed(&self, vars: &VarsRead) -> UpdateMask {
        self.update_mask(vars)
    }
    #[inline]
    fn is_rc_boxed(&self) -> bool {
        self.is_rc()
    }
    #[inline]
    fn downgrade_boxed(&self) -> Option<BoxedWeakVar<T>> {
        self.downgrade().map(|w| w.boxed())
    }
    #[inline]
    fn weak_count_boxed(&self) -> usize {
        self.weak_count()
    }
    #[inline]
    fn as_ptr_boxed(&self) -> *const () {
        self.as_ptr()
    }
}
impl<T: VarValue> Clone for BoxedVar<T> {
    fn clone(&self) -> Self {
        self.as_ref().clone_boxed()
    }
}
impl<T: VarValue> crate::private::Sealed for BoxedVar<T> {}
impl<T: VarValue> Var<T> for BoxedVar<T> {
    type AsReadOnly = BoxedVar<T>;
    type Weak = BoxedWeakVar<T>;

    #[inline]
    fn boxed(self) -> BoxedVar<T>
    where
        Self: VarBoxed<T> + Sized,
    {
        self
    }

    #[inline]
    fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a T {
        self.as_ref().get_boxed(vars.as_ref())
    }

    #[inline]
    fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a T> {
        self.as_ref().get_new_boxed(vars.as_ref())
    }

    #[inline]
    fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
        vars.with_vars(|vars| self.as_ref().is_new_boxed(vars))
    }

    #[inline]
    fn into_value<Vr: WithVarsRead>(self, vars: &Vr) -> T {
        vars.with_vars_read(|vars| self.into_value_boxed(vars))
    }

    #[inline]
    fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> VarVersion {
        vars.with_vars_read(|vars| self.as_ref().version_boxed(vars))
    }

    #[inline]
    fn is_read_only<Vw: WithVars>(&self, vars: &Vw) -> bool {
        vars.with_vars(|vars| self.as_ref().is_read_only_boxed(vars))
    }

    #[inline]
    fn is_animating<Vr: WithVarsRead>(&self, vars: &Vr) -> bool {
        vars.with_vars_read(|vars| self.as_ref().is_animating_boxed(vars))
    }

    #[inline]
    fn always_read_only(&self) -> bool {
        self.as_ref().always_read_only_boxed()
    }

    #[inline]
    fn is_contextual(&self) -> bool {
        self.as_ref().is_contextual_boxed()
    }

    #[inline]
    fn can_update(&self) -> bool {
        self.as_ref().can_update_boxed()
    }

    #[inline]
    fn modify<Vw, M>(&self, vars: &Vw, modify: M) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        M: FnOnce(VarModify<T>) + 'static,
    {
        vars.with_vars(|vars| self.as_ref().modify_boxed(vars, Box::new(modify)))
    }

    #[inline]
    fn set<Vw, N>(&self, vars: &Vw, new_value: N) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<T>,
    {
        vars.with_vars(|vars| self.as_ref().set_boxed(vars, new_value.into()))
    }

    fn set_ne<Vw, N>(&self, vars: &Vw, new_value: N) -> Result<bool, VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<T>,
        T: PartialEq,
    {
        if self.is_read_only(vars) {
            Err(VarIsReadOnly)
        } else {
            let new_value = new_value.into();
            vars.with_vars(|vars| {
                if self.get(vars) != &new_value {
                    let _ = self.set(vars, new_value);
                    Ok(true)
                } else {
                    Ok(false)
                }
            })
        }
    }

    fn strong_count(&self) -> usize {
        self.as_ref().strong_count_boxed()
    }

    #[inline]
    fn into_read_only(self) -> Self::AsReadOnly {
        if self.always_read_only() {
            self
        } else {
            ReadOnlyVar::new(self).boxed()
        }
    }

    fn update_mask<Vr: WithVarsRead>(&self, vars: &Vr) -> UpdateMask {
        vars.with_vars_read(|vars| self.as_ref().update_mask_boxed(vars.as_ref()))
    }

    #[inline]
    fn is_rc(&self) -> bool {
        self.as_ref().is_rc_boxed()
    }

    #[inline]
    fn downgrade(&self) -> Option<Self::Weak> {
        self.as_ref().downgrade_boxed()
    }

    #[inline]
    fn weak_count(&self) -> usize {
        self.as_ref().weak_count_boxed()
    }

    #[inline]
    fn as_ptr(&self) -> *const () {
        self.as_ref().as_ptr_boxed()
    }
}
impl<T: VarValue> IntoVar<T> for BoxedVar<T> {
    type Var = Self;

    #[inline]
    fn into_var(self) -> Self::Var {
        self
    }
}
