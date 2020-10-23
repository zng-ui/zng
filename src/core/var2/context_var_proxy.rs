use super::*;

/// A [`Var`] that represents a [`ContextVar`].
#[doc(hidden)]
#[derive(Clone)]
pub struct ContextVarProxy<C: ContextVar>(PhantomData<C>);
impl<C: ContextVar> protected::Var for ContextVarProxy<C> {}
impl<C: ContextVar> Default for ContextVarProxy<C> {
    fn default() -> Self {
        ContextVarProxy(PhantomData)
    }
}
impl<C: ContextVar> VarObj<C::Type> for ContextVarProxy<C> {
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a C::Type {
        vars.context_var::<C>().0
    }

    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a C::Type> {
        let (value, is_new, _) = vars.context_var::<C>();
        if is_new {
            Some(value)
        } else {
            None
        }
    }

    fn is_new(&self, vars: &Vars) -> bool {
        vars.context_var::<C>().1
    }

    fn version(&self, vars: &Vars) -> u32 {
        vars.context_var::<C>().2
    }

    fn is_read_only(&self, _: &Vars) -> bool {
        true
    }

    fn always_read_only(&self) -> bool {
        true
    }

    fn can_update(&self) -> bool {
        true
    }

    fn set(&self, _: &Vars, _: C::Type) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    fn modify_boxed(&self, _: &Vars, _: Box<dyn FnOnce(&mut C::Type)>) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }
}
impl<C: ContextVar> Var<C::Type> for ContextVarProxy<C> {
    type AsReadOnly = Self;

    type AsLocal = CloningLocalVar<C::Type, Self>;

    fn as_read_only(self) -> Self::AsReadOnly {
        self
    }

    fn as_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }

    fn modify<F: FnOnce(&mut C::Type) + 'static>(&self, _: &Vars, _: F) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    fn map<O: VarValue, F: FnMut(&C::Type) -> O>(&self, map: F) -> RcMapVar<C::Type, O, Self, F> {
        RcMapVar::new(self.clone(), map)
    }

    fn map_bidi<O: VarValue, F: FnMut(&C::Type) -> O + 'static, G: FnMut(O) -> C::Type + 'static>(
        &self,
        map: F,
        map_back: G,
    ) -> RcMapBidiVar<C::Type, O, Self, F, G> {
        RcMapBidiVar::new(self.clone(), map, map_back)
    }
}

impl<C: ContextVar> IntoVar<C::Type> for ContextVarProxy<C> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}
