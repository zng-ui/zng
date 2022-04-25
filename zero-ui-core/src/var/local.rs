use super::*;

/// A [`Var`] is a fixed value that is stored locally.
///
/// Cloning this variable clones the value.
#[derive(Clone)]
pub struct LocalVar<T: VarValue>(pub T);
impl<T: VarValue> crate::private::Sealed for LocalVar<T> {}
impl<T: VarValue> Var<T> for LocalVar<T> {
    type AsReadOnly = Self;
    type Weak = NoneWeakVar<T>;

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
    fn version<Vr: WithVarsRead>(&self, _: &Vr) -> VarVersion {
        VarVersion::normal(0)
    }

    #[inline]
    fn is_read_only<Vw: WithVars>(&self, _: &Vw) -> bool {
        true
    }

    #[inline]
    fn is_animating<Vr: WithVarsRead>(&self, _: &Vr) -> bool {
        false
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
        M: FnOnce(VarModify<T>) + 'static,
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

    #[inline]
    fn is_rc(&self) -> bool {
        false
    }

    #[inline]
    fn downgrade(&self) -> Option<Self::Weak> {
        None
    }
    #[inline]
    fn weak_count(&self) -> usize {
        0
    }
    #[inline]
    fn as_ptr(&self) -> *const () {
        std::ptr::null()
    }
}
impl<T: VarValue + Default> Default for LocalVar<T> {
    fn default() -> Self {
        LocalVar(T::default())
    }
}
impl<T: VarValue> IntoVar<T> for LocalVar<T> {
    type Var = Self;

    #[inline]
    fn into_var(self) -> Self::Var {
        self
    }
}
impl<T: VarValue> IntoVar<T> for T {
    type Var = LocalVar<T>;

    fn into_var(self) -> Self::Var {
        LocalVar(self)
    }
}
