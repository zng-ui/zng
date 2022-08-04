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

    fn get<'a, Vr: AsRef<VarsRead>>(&'a self, _: &'a Vr) -> &'a T {
        &self.0
    }

    fn get_new<'a, Vw: AsRef<Vars>>(&'a self, _: &'a Vw) -> Option<&'a T> {
        None
    }

    fn is_new<Vw: WithVars>(&self, _: &Vw) -> bool {
        false
    }

    fn into_value<Vr: WithVarsRead>(self, _: &Vr) -> T {
        self.0
    }

    fn version<Vr: WithVarsRead>(&self, _: &Vr) -> VarVersion {
        VarVersion::normal(0)
    }

    fn is_read_only<Vw: WithVars>(&self, _: &Vw) -> bool {
        true
    }

    fn is_animating<Vr: WithVarsRead>(&self, _: &Vr) -> bool {
        false
    }

    fn strong_count(&self) -> usize {
        0
    }

    fn always_read_only(&self) -> bool {
        true
    }

    fn can_update(&self) -> bool {
        false
    }

    fn is_contextual(&self) -> bool {
        false
    }

    fn actual_var<Vw: WithVars>(&self, _: &Vw) -> BoxedVar<T> {
        self.clone().boxed()
    }

    fn modify<Vw, M>(&self, _: &Vw, _: M) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        M: FnOnce(VarModify<T>) + 'static,
    {
        Err(VarIsReadOnly)
    }

    fn set<Vw, N>(&self, _: &Vw, _: N) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<T>,
    {
        Err(VarIsReadOnly)
    }

    fn set_ne<Vw, N>(&self, _: &Vw, _: N) -> Result<bool, VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<T>,
        T: PartialEq,
    {
        Err(VarIsReadOnly)
    }

    fn into_read_only(self) -> Self::AsReadOnly {
        self
    }

    fn update_mask<Vr: WithVarsRead>(&self, _: &Vr) -> UpdateMask {
        UpdateMask::none()
    }

    fn is_rc(&self) -> bool {
        false
    }

    fn downgrade(&self) -> Option<Self::Weak> {
        None
    }

    fn weak_count(&self) -> usize {
        0
    }

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
impl<T: VarValue> any::AnyVar for LocalVar<T> {
    fn into_any(self) -> Box<dyn any::AnyVar> {
        Box::new(self)
    }

    any_var_impls!(Var);
}
