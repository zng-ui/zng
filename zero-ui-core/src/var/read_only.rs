use std::marker::PhantomData;

use super::*;

/// A [`WeakVar`] wrapper that upgrades to a [`ReadOnlyVar`].
pub struct ReadOnlyWeakVar<T: VarValue, W: WeakVar<T>>(W, PhantomData<T>);
impl<T: VarValue, W: WeakVar<T>> ReadOnlyWeakVar<T, W> {
    /// New wrapper.
    ///
    /// Prefer [`Var::into_read_only`].
    pub fn new(weak: W) -> Self {
        Self(weak, PhantomData)
    }
}
impl<T: VarValue, W: WeakVar<T>> crate::private::Sealed for ReadOnlyWeakVar<T, W> {}
impl<T: VarValue, W: WeakVar<T>> Clone for ReadOnlyWeakVar<T, W> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}
impl<T, W> any::AnyWeakVar for ReadOnlyWeakVar<T, W>
where
    T: VarValue,
    W: WeakVar<T>,
{
    fn into_any(self) -> Box<dyn any::AnyWeakVar> {
        any::AnyWeakVar::into_any(self.0)
    }
    any_var_impls!(WeakVar);
}
impl<T: VarValue, W: WeakVar<T>> WeakVar<T> for ReadOnlyWeakVar<T, W> {
    type Strong = ReadOnlyVar<T, W::Strong>;

    fn upgrade(&self) -> Option<Self::Strong> {
        self.0.upgrade().map(ReadOnlyVar::new)
    }

    fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.0.weak_count()
    }

    fn as_ptr(&self) -> *const () {
        self.0.as_ptr()
    }
}

/// A [`Var`] wrapper that makes it [`always_read_only`](Var::always_read_only).
pub struct ReadOnlyVar<T: VarValue, V: Var<T>>(V, PhantomData<T>);

impl<T, V> ReadOnlyVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
    /// Wrap var.
    pub fn new(var: V) -> Self {
        ReadOnlyVar(var, PhantomData)
    }
}

impl<T, V> Clone for ReadOnlyVar<T, V>
where
    T: VarValue,
    V: Var<T> + Clone,
{
    fn clone(&self) -> Self {
        ReadOnlyVar(self.0.clone(), PhantomData)
    }
}
impl<T, V> crate::private::Sealed for ReadOnlyVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
}
impl<T, V> Var<T> for ReadOnlyVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
    type AsReadOnly = Self;

    fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a T {
        self.0.get(vars)
    }

    fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a T> {
        self.0.get_new(vars)
    }

    fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
        self.0.is_new(vars)
    }

    fn into_value<Vr: WithVarsRead>(self, vars: &Vr) -> T {
        self.0.into_value(vars)
    }

    fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> VarVersion {
        self.0.version(vars)
    }

    fn is_read_only<Vw: WithVars>(&self, _: &Vw) -> bool {
        true
    }

    fn is_animating<Vr: WithVarsRead>(&self, vars: &Vr) -> bool {
        self.0.is_animating(vars)
    }

    fn always_read_only(&self) -> bool {
        true
    }

    fn can_update(&self) -> bool {
        self.0.can_update()
    }

    fn is_contextual(&self) -> bool {
        self.0.is_contextual()
    }

    fn actual_var<Vw: WithVars>(&self, vars: &Vw) -> BoxedVar<T> {
        if self.is_contextual() {
            self.0.actual_var(vars).into_read_only()
        } else {
            self.clone().boxed()
        }
    }

    fn strong_count(&self) -> usize {
        self.0.strong_count()
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

    fn update_mask<Vr: WithVarsRead>(&self, vars: &Vr) -> UpdateMask {
        self.0.update_mask(vars)
    }

    type Weak = ReadOnlyWeakVar<T, V::Weak>;

    fn is_rc(&self) -> bool {
        self.0.is_rc()
    }

    fn downgrade(&self) -> Option<Self::Weak> {
        self.0.downgrade().map(ReadOnlyWeakVar::new)
    }

    fn weak_count(&self) -> usize {
        self.0.weak_count()
    }

    fn as_ptr(&self) -> *const () {
        self.0.as_ptr()
    }
}
impl<T, V> IntoVar<T> for ReadOnlyVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}
impl<T, V> any::AnyVar for ReadOnlyVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
    fn into_any(self) -> Box<dyn any::AnyVar> {
        any::AnyVar::into_any(self.0)
    }
    any_var_impls!(Var);
}
