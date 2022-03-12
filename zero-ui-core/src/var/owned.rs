use super::*;

/// A [`Var`] that owns the value and keeps it locally.
#[derive(Clone)]
pub struct OwnedVar<T: VarValue>(pub T);
impl<T: VarValue> crate::private::Sealed for OwnedVar<T> {}
impl<T: VarValue> Var<T> for OwnedVar<T> {
    type AsReadOnly = Self;

    #[inline]
    fn get<'a, Vr: AsRef<VarsRead>>(&'a self, _: &'a Vr) -> &'a T {
        &self.0
    }

    #[inline]
    fn get_new<'a, Vw: AsRef<Vars>>(&'a self, _: &'a Vw) -> Option<&'a T> {
        None
    }

    #[inline]
    fn is_new<Vw: WithVars>(&self, _: &Vw) -> bool {
        false
    }

    #[inline]
    fn into_value<Vr: WithVarsRead>(self, _: &Vr) -> T {
        self.0
    }

    #[inline]
    fn version<Vr: WithVarsRead>(&self, _: &Vr) -> u32 {
        0
    }

    #[inline]
    fn is_read_only<Vw: WithVars>(&self, _: &Vw) -> bool {
        true
    }

    #[inline]
    fn strong_count(&self) -> usize {
        0
    }

    #[inline]
    fn always_read_only(&self) -> bool {
        true
    }

    #[inline]
    fn can_update(&self) -> bool {
        false
    }

    #[inline]
    fn is_contextual(&self) -> bool {
        false
    }

    #[inline]
    fn modify<Vw, M>(&self, _: &Vw, _: M) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        M: FnOnce(&mut VarModify<T>) + 'static,
    {
        Err(VarIsReadOnly)
    }

    #[inline]
    fn set<Vw, N>(&self, _: &Vw, _: N) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<T>,
    {
        Err(VarIsReadOnly)
    }

    #[inline]
    fn set_ne<Vw, N>(&self, _: &Vw, _: N) -> Result<bool, VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<T>,
        T: PartialEq,
    {
        Err(VarIsReadOnly)
    }

    #[inline]
    fn into_read_only(self) -> Self::AsReadOnly {
        self
    }

    #[inline]
    fn update_mask<Vr: WithVarsRead>(&self, _: &Vr) -> UpdateMask {
        UpdateMask::none()
    }
}
impl<T: VarValue + Default> Default for OwnedVar<T> {
    fn default() -> Self {
        OwnedVar(T::default())
    }
}
impl<T: VarValue> IntoVar<T> for OwnedVar<T> {
    type Var = Self;

    #[inline]
    fn into_var(self) -> Self::Var {
        self
    }
}
impl<T: VarValue> IntoVar<T> for T {
    type Var = OwnedVar<T>;

    fn into_var(self) -> Self::Var {
        OwnedVar(self)
    }
}
