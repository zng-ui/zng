use super::*;

/// A [`Var`] that owns the value and keeps it locally.
#[derive(Clone)]
pub struct OwnedVar<T: VarValue>(pub T);
impl<T: VarValue> Var<T> for OwnedVar<T> {
    type AsReadOnly = Self;

    type AsLocal = Self;

    #[inline]
    fn get<'a>(&'a self, _: &'a VarsRead) -> &'a T {
        &self.0
    }

    #[inline]
    fn get_new<'a>(&'a self, _: &'a Vars) -> Option<&'a T> {
        None
    }

    #[inline]
    fn version(&self, _: &VarsRead) -> u32 {
        0
    }

    #[inline]
    fn is_read_only(&self, _: &Vars) -> bool {
        true
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
    fn modify<M>(&self, _: &Vars, _: M) -> Result<(), VarIsReadOnly>
    where
        M: FnOnce(&mut VarModify<T>) + 'static,
    {
        Err(VarIsReadOnly)
    }

    #[inline]
    fn set<N>(&self, _: &Vars, _: N) -> Result<(), VarIsReadOnly>
    where
        N: Into<T>,
    {
        Err(VarIsReadOnly)
    }

    #[inline]
    fn set_ne<N>(&self, _: &Vars, _: N) -> Result<bool, VarIsReadOnly>
    where
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
    fn into_local(self) -> Self::AsLocal {
        self
    }
}
impl<T: VarValue> IntoVar<T> for OwnedVar<T> {
    type Var = Self;

    #[inline]
    fn into_var(self) -> Self::Var {
        self
    }
}
impl<T: VarValue> VarLocal<T> for OwnedVar<T> {
    #[inline]
    fn get_local(&self) -> &T {
        &self.0
    }

    #[inline]
    fn init_local<'a>(&'a mut self, _: &'a Vars) -> &'a T {
        &self.0
    }

    #[inline]
    fn update_local<'a>(&'a mut self, _: &'a Vars) -> Option<&'a T> {
        None
    }
}

impl<T: VarValue> IntoVar<T> for T {
    type Var = OwnedVar<T>;

    fn into_var(self) -> Self::Var {
        OwnedVar(self)
    }
}
