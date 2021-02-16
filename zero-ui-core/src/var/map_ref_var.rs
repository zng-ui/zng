use super::*;

/// A referencing mapping variable.
///
/// The variable is read-only, the value is
/// referenced from the value of another variable.
#[doc(hidden)]
pub struct MapRefVar<I: VarValue, O: VarValue, V: Var<I>, F: Fn(&I) -> &O + Clone + 'static> {
    _p: PhantomData<(I, O)>,
    var: V,
    f: F,
}
impl<I, O, V, F> MapRefVar<I, O, V, F>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: Fn(&I) -> &O + Clone + 'static,
{
    pub fn new(var: V, f: F) -> Self {
        MapRefVar { _p: PhantomData, var, f }
    }
}
impl<I, O, V, F> Clone for MapRefVar<I, O, V, F>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: Fn(&I) -> &O + Clone + 'static,
{
    fn clone(&self) -> Self {
        Self {
            _p: PhantomData,
            var: self.var.clone(),
            f: self.f.clone(),
        }
    }
}
impl<I, O, V, F> protected::Var for MapRefVar<I, O, V, F>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: Fn(&I) -> &O + Clone + 'static,
{
}
impl<I, O, V, F> VarObj<O> for MapRefVar<I, O, V, F>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: Fn(&I) -> &O + Clone + 'static,
{
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a O {
        (self.f)(self.var.get(vars))
    }

    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a O> {
        self.var.get_new(vars).map(|v| (self.f)(v))
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.var.is_new(vars)
    }

    fn version(&self, vars: &Vars) -> u32 {
        self.var.version(vars)
    }

    fn is_read_only(&self, _: &Vars) -> bool {
        true
    }

    fn always_read_only(&self) -> bool {
        true
    }

    fn can_update(&self) -> bool {
        self.var.can_update()
    }

    fn set(&self, _: &Vars, _: O) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    fn modify_boxed(&self, _: &Vars, _: Box<dyn FnOnce(&mut O)>) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }
}
impl<I, O, V, F> Var<O> for MapRefVar<I, O, V, F>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: Fn(&I) -> &O + Clone + 'static,
{
    type AsReadOnly = Self;

    type AsLocal = CloningLocalVar<O, Self>;

    fn modify<F2: FnOnce(&mut O) + 'static>(&self, _: &Vars, _: F2) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    fn into_read_only(self) -> Self::AsReadOnly {
        self
    }

    fn into_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }

    fn map<O2: VarValue, F2: FnMut(&O) -> O2 + 'static>(&self, map: F2) -> RcMapVar<O, O2, Self, F2> {
        self.clone().into_map(map)
    }

    fn map_ref<O2: VarValue, F2: Fn(&O) -> &O2 + Clone + 'static>(&self, map: F2) -> MapRefVar<O, O2, Self, F2> {
        self.clone().into_map_ref(map)
    }

    fn map_bidi<O2: VarValue, F2: FnMut(&O) -> O2 + 'static, G: FnMut(O2) -> O + 'static>(
        &self,
        map: F2,
        map_back: G,
    ) -> RcMapBidiVar<O, O2, Self, F2, G> {
        self.clone().into_map_bidi(map, map_back)
    }

    fn into_map<O2: VarValue, F2: FnMut(&O) -> O2 + 'static>(self, map: F2) -> RcMapVar<O, O2, Self, F2> {
        RcMapVar::new(self, map)
    }

    fn into_map_ref<O2: VarValue, F2: Fn(&O) -> &O2 + Clone + 'static>(self, map: F2) -> MapRefVar<O, O2, Self, F2> {
        MapRefVar::new(self, map)
    }

    fn into_map_bidi<O2: VarValue, F2: FnMut(&O) -> O2 + 'static, G: FnMut(O2) -> O + 'static>(
        self,
        map: F2,
        map_back: G,
    ) -> RcMapBidiVar<O, O2, Self, F2, G> {
        RcMapBidiVar::new(self, map, map_back)
    }

    fn map_bidi_ref<O2: VarValue, F2: Fn(&O) -> &O2 + Clone + 'static, G2: Fn(&mut O) -> &mut O2 + Clone + 'static>(
        &self,
        map: F2,
        map_mut: G2,
    ) -> MapBidiRefVar<O, O2, Self, F2, G2> {
        self.clone().into_map_bidi_ref(map, map_mut)
    }

    fn into_map_bidi_ref<O2: VarValue, F2: Fn(&O) -> &O2 + Clone + 'static, G2: Fn(&mut O) -> &mut O2 + Clone + 'static>(
        self,
        map: F2,
        map_mut: G2,
    ) -> MapBidiRefVar<O, O2, Self, F2, G2> {
        MapBidiRefVar::new(self, map, map_mut)
    }
}

impl<I, O, V, F> IntoVar<O> for MapRefVar<I, O, V, F>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: Fn(&I) -> &O + Clone + 'static,
{
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}
