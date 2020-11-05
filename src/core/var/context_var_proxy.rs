use super::*;

/// A [`Var`] that represents a [`ContextVar`].
///
/// `PhantomData` is public here because we can't implement a `const fn new()` on stable.
/// We need to generate a const value to implement `ContextVar::var()`.
#[derive(Clone)]
pub struct ContextVarProxy<C: ContextVar>(pub PhantomData<C>);
impl<C: ContextVar> ContextVarProxy<C> {
    /// References the value in the current `vars` context.
    pub fn get<'a>(&'a self, vars: &'a Vars) -> &'a C::Type {
        <Self as VarObj<C::Type>>::get(self, vars)
    }

    /// References the value in the current `vars` context if it is marked as new.
    pub fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a C::Type> {
        <Self as VarObj<C::Type>>::get_new(self, vars)
    }

    /// If the value in the current `vars` context is marked as new.
    pub fn is_new(&self, vars: &Vars) -> bool {
        <Self as VarObj<C::Type>>::is_new(self, vars)
    }

    /// Gets the version of the value in the current `vars` context.
    pub fn version(&self, vars: &Vars) -> u32 {
        <Self as VarObj<C::Type>>::version(self, vars)
    }
}
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
        self.clone().into_map(map)
    }

    fn map_ref<O: VarValue, F: Fn(&C::Type) -> &O + Clone + 'static>(&self, map: F) -> MapRefVar<C::Type, O, Self, F> {
        self.clone().into_map_ref(map)
    }

    fn map_bidi<O: VarValue, F: FnMut(&C::Type) -> O + 'static, G: FnMut(O) -> C::Type + 'static>(
        &self,
        map: F,
        map_back: G,
    ) -> RcMapBidiVar<C::Type, O, Self, F, G> {
        self.clone().into_map_bidi(map, map_back)
    }

    fn into_map<O: VarValue, F: FnMut(&C::Type) -> O>(self, map: F) -> RcMapVar<C::Type, O, Self, F> {
        RcMapVar::new(self, map)
    }

    fn into_map_bidi<O: VarValue, F: FnMut(&C::Type) -> O + 'static, G: FnMut(O) -> C::Type + 'static>(
        self,
        map: F,
        map_back: G,
    ) -> RcMapBidiVar<C::Type, O, Self, F, G> {
        RcMapBidiVar::new(self, map, map_back)
    }

    fn into_map_ref<O: VarValue, F: Fn(&C::Type) -> &O + Clone + 'static>(self, map: F) -> MapRefVar<C::Type, O, Self, F> {
        MapRefVar::new(self, map)
    }

    fn map_bidi_ref<O: VarValue, F: Fn(&C::Type) -> &O + Clone + 'static, G: Fn(&mut C::Type) -> &mut O + Clone + 'static>(
        &self,
        map: F,
        map_mut: G,
    ) -> MapBidiRefVar<C::Type, O, Self, F, G> {
        self.clone().into_map_bidi_ref(map, map_mut)
    }

    fn into_map_bidi_ref<O: VarValue, F: Fn(&C::Type) -> &O + Clone + 'static, G: Fn(&mut C::Type) -> &mut O + Clone + 'static>(
        self,
        map: F,
        map_mut: G,
    ) -> MapBidiRefVar<C::Type, O, Self, F, G> {
        MapBidiRefVar::new(self, map, map_mut)
    }
}

impl<C: ContextVar> IntoVar<C::Type> for ContextVarProxy<C> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}
