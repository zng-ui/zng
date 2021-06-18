use super::*;

/// A [`Var`] in a box.
///
/// This type uses dynamic dispatch to access the generic methods of [`Var`], in exchange
/// it can store any type of var.
pub type BoxedVar<T> = Box<dyn VarBoxed<T>>;

#[doc(hidden)]
pub trait VarBoxed<T: VarValue> {
    fn get_boxed<'a>(&'a self, vars: &'a VarsRead) -> &'a T;
    fn get_new_boxed<'a>(&'a self, vars: &'a Vars) -> Option<&'a T>;
    fn is_new_boxed(&self, vars: &Vars) -> bool;
    fn version_boxed<'a>(&'a self, vars: &'a VarsRead) -> u32;
    fn is_read_only_boxed(&self, vars: &Vars) -> bool;
    fn into_value_boxed(self: Box<Self>, vars: &VarsRead) -> T;
    fn always_read_only_boxed(&self) -> bool;
    fn can_update_boxed(&self) -> bool;
    fn modify_boxed(&self, vars: &Vars, modify: Box<dyn FnOnce(&mut VarModify<T>)>) -> Result<(), VarIsReadOnly>;
    fn set_boxed(&self, vars: &Vars, new_value: T) -> Result<(), VarIsReadOnly>;
    fn clone_boxed(&self) -> BoxedVar<T>;
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
    fn version_boxed<'a>(&'a self, vars: &'a VarsRead) -> u32 {
        self.version(vars)
    }

    #[inline]
    fn is_read_only_boxed(&self, vars: &Vars) -> bool {
        self.is_read_only(vars)
    }

    #[inline]
    fn always_read_only_boxed(&self) -> bool {
        self.always_read_only()
    }

    #[inline]
    fn can_update_boxed(&self) -> bool {
        self.can_update()
    }

    #[inline]
    fn modify_boxed(&self, vars: &Vars, modify: Box<dyn FnOnce(&mut VarModify<T>)>) -> Result<(), VarIsReadOnly> {
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
}
impl<T: VarValue> Clone for BoxedVar<T> {
    fn clone(&self) -> Self {
        self.clone_boxed()
    }
}
impl<T: VarValue> Var<T> for BoxedVar<T> {
    type AsReadOnly = BoxedVar<T>;

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
        vars.with(|vars| self.as_ref().is_new_boxed(vars))
    }

    #[inline]
    fn into_value<Vr: WithVarsRead>(self, vars: &Vr) -> T {
        vars.with(|vars| self.into_value_boxed(vars))
    }

    #[inline]
    fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> u32 {
        vars.with(|vars| self.as_ref().version_boxed(vars))
    }

    #[inline]
    fn is_read_only<Vw: WithVars>(&self, vars: &Vw) -> bool {
        vars.with(|vars| self.as_ref().is_read_only_boxed(vars))
    }

    #[inline]
    fn always_read_only(&self) -> bool {
        self.as_ref().always_read_only_boxed()
    }

    #[inline]
    fn can_update(&self) -> bool {
        self.as_ref().can_update_boxed()
    }

    #[inline]
    fn modify<Vw, M>(&self, vars: &Vw, modify: M) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        M: FnOnce(&mut VarModify<T>) + 'static,
    {
        vars.with(|vars| self.as_ref().modify_boxed(vars, Box::new(modify)))
    }

    #[inline]
    fn set<Vw, N>(&self, vars: &Vw, new_value: N) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<T>,
    {
        vars.with(|vars| self.as_ref().set_boxed(vars, new_value.into()))
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
            vars.with(|vars| {
                if self.get(vars) != &new_value {
                    let _ = self.set(vars, new_value);
                    Ok(true)
                } else {
                    Ok(false)
                }
            })
        }
    }

    #[inline]
    fn into_read_only(self) -> Self::AsReadOnly {
        if self.always_read_only() {
            self
        } else {
            ReadOnlyVar::new(self).boxed()
        }
    }
}
impl<T: VarValue> IntoVar<T> for BoxedVar<T> {
    type Var = Self;

    #[inline]
    fn into_var(self) -> Self::Var {
        self
    }
}
