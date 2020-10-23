use super::*;

/// A [`Var`] wrapper that forces read-only.
#[doc(hidden)]
pub struct ForceReadOnlyVar<T: VarValue, V: Var<T>>(V, PhantomData<T>);
impl<T: VarValue, V: Var<T>> protected::Var for ForceReadOnlyVar<T, V> {}
impl<T: VarValue, V: Var<T>> ForceReadOnlyVar<T, V> {
    pub(super) fn new(var: V) -> Self {
        ForceReadOnlyVar(var, PhantomData)
    }
}
impl<T: VarValue, V: Var<T>> Clone for ForceReadOnlyVar<T, V> {
    fn clone(&self) -> Self {
        ForceReadOnlyVar(self.0.clone(), PhantomData)
    }
}
impl<T: VarValue, V: Var<T>> VarObj<T> for ForceReadOnlyVar<T, V> {
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a T {
        self.0.get(vars)
    }

    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        self.0.get_new(vars)
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.0.is_new(vars)
    }

    fn version(&self, vars: &Vars) -> u32 {
        self.0.version(vars)
    }

    fn is_read_only(&self, _: &Vars) -> bool {
        true
    }

    fn always_read_only(&self) -> bool {
        true
    }

    fn can_update(&self) -> bool {
        self.0.can_update()
    }

    fn set(&self, _: &Vars, _: T) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    fn modify_boxed(&self, _: &Vars, _: Box<dyn FnOnce(&mut T)>) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }
}
impl<T: VarValue, V: Var<T>> Var<T> for ForceReadOnlyVar<T, V> {
    type AsReadOnly = Self;
    type AsLocal = CloningLocalVar<T, Self>;

    fn as_read_only(self) -> Self::AsReadOnly {
        self
    }

    fn as_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }

    fn modify<F: FnOnce(&mut T) + 'static>(&self, _: &Vars, _: F) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    fn map<O: VarValue, F: FnMut(&T) -> O + 'static>(&self, map: F) -> RcMapVar<T, O, Self, F> {
        RcMapVar::new(self.clone(), map)
    }

    fn map_bidi<O: VarValue, F: FnMut(&T) -> O + 'static, G: FnMut(O) -> T + 'static>(
        &self,
        map: F,
        map_back: G,
    ) -> RcMapBidiVar<T, O, Self, F, G> {
        RcMapBidiVar::new(self.clone(), map, map_back)
    }
}
