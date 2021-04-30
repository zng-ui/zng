use super::*;

/// A bidirectional referencing mapping variable.
///
/// The variable is read-write, the value is
/// referenced from the value of another variable.
pub struct MapBidiRefVar<I, O, V, F, G>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: Fn(&I) -> &O + Clone + 'static,
    G: Fn(&mut I) -> &mut O + Clone + 'static,
{
    _p: PhantomData<(I, O)>,
    var: V,
    map: F,
    map_mut: G,
}
impl<I, O, V, F, G> Clone for MapBidiRefVar<I, O, V, F, G>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: Fn(&I) -> &O + Clone + 'static,
    G: Fn(&mut I) -> &mut O + Clone + 'static,
{
    fn clone(&self) -> Self {
        Self {
            _p: PhantomData,
            var: self.var.clone(),
            map: self.map.clone(),
            map_mut: self.map_mut.clone(),
        }
    }
}
impl<I, O, V, F, G> MapBidiRefVar<I, O, V, F, G>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: Fn(&I) -> &O + Clone + 'static,
    G: Fn(&mut I) -> &mut O + Clone + 'static,
{
    pub(super) fn new(var: V, map: F, map_mut: G) -> Self {
        Self {
            _p: PhantomData,
            var,
            map,
            map_mut,
        }
    }
}
impl<I, O, V, F, G> protected::Var for MapBidiRefVar<I, O, V, F, G>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: Fn(&I) -> &O + Clone + 'static,
    G: Fn(&mut I) -> &mut O + Clone + 'static,
{
}
impl<I, O, V, F, G> VarObj<O> for MapBidiRefVar<I, O, V, F, G>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: Fn(&I) -> &O + Clone + 'static,
    G: Fn(&mut I) -> &mut O + Clone + 'static,
{
    fn get<'a>(&'a self, vars: &'a VarsRead) -> &'a O {
        (self.map)(self.var.get(vars))
    }

    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a O> {
        self.var.get_new(vars).map(|v| (self.map)(v))
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.var.is_new(vars)
    }

    fn version(&self, vars: &VarsRead) -> u32 {
        self.var.version(vars)
    }

    fn is_read_only(&self, vars: &Vars) -> bool {
        self.var.is_read_only(vars)
    }

    fn always_read_only(&self) -> bool {
        self.var.always_read_only()
    }

    fn can_update(&self) -> bool {
        self.var.can_update()
    }

    fn set(&self, vars: &Vars, new_value: O) -> Result<(), VarIsReadOnly> {
        self.modify(vars, move |m| *m = new_value)
    }

    fn modify_boxed(&self, vars: &Vars, change: Box<dyn FnOnce(&mut O)>) -> Result<(), VarIsReadOnly> {
        self.modify(vars, move |m| change(m))
    }
}
impl<I, O, V, F, G> Var<O> for MapBidiRefVar<I, O, V, F, G>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: Fn(&I) -> &O + Clone + 'static,
    G: Fn(&mut I) -> &mut O + Clone + 'static,
{
    type AsReadOnly = MapRefVar<I, O, V, F>;

    type AsLocal = CloningLocalVar<O, Self>;
    fn into_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }

    fn modify<F2: FnOnce(&mut O) + 'static>(&self, vars: &Vars, change: F2) -> Result<(), VarIsReadOnly> {
        let mut_ = self.map_mut.clone();
        self.var.modify(vars, move |v| change(mut_(v)))
    }

    fn into_read_only(self) -> Self::AsReadOnly {
        MapRefVar::new(self.var, self.map)
    }

    fn map<O2: VarValue, F2: FnMut(&O) -> O2 + 'static>(&self, map: F2) -> RcMapVar<O, O2, Self, F2> {
        self.clone().into_map(map)
    }

    fn map_ref<O2: VarValue, F2: Fn(&O) -> &O2 + Clone + 'static>(&self, map: F2) -> MapRefVar<O, O2, Self, F2> {
        self.clone().into_map_ref(map)
    }

    fn map_bidi<O2: VarValue, F2: FnMut(&O) -> O2 + 'static, G2: FnMut(O2) -> O + 'static>(
        &self,
        map: F2,
        map_back: G2,
    ) -> RcMapBidiVar<O, O2, Self, F2, G2> {
        self.clone().into_map_bidi(map, map_back)
    }

    fn map_bidi_ref<O2: VarValue, F2: Fn(&O) -> &O2 + Clone + 'static, G2: Fn(&mut O) -> &mut O2 + Clone + 'static>(
        &self,
        map: F2,
        map_mut: G2,
    ) -> MapBidiRefVar<O, O2, Self, F2, G2> {
        self.clone().into_map_bidi_ref(map, map_mut)
    }

    fn into_map<O2: VarValue, F2: FnMut(&O) -> O2 + 'static>(self, map: F2) -> RcMapVar<O, O2, Self, F2> {
        RcMapVar::new(self, map)
    }

    fn into_map_ref<O2: VarValue, F2: Fn(&O) -> &O2 + Clone + 'static>(self, map: F2) -> MapRefVar<O, O2, Self, F2> {
        MapRefVar::new(self, map)
    }

    fn into_map_bidi<O2: VarValue, F2: FnMut(&O) -> O2 + 'static, G2: FnMut(O2) -> O + 'static>(
        self,
        map: F2,
        map_back: G2,
    ) -> RcMapBidiVar<O, O2, Self, F2, G2> {
        RcMapBidiVar::new(self, map, map_back)
    }

    fn into_map_bidi_ref<O2: VarValue, F2: Fn(&O) -> &O2 + Clone + 'static, G2: Fn(&mut O) -> &mut O2 + Clone + 'static>(
        self,
        map: F2,
        map_mut: G2,
    ) -> MapBidiRefVar<O, O2, Self, F2, G2> {
        MapBidiRefVar::new(self, map, map_mut)
    }
}

impl<I, O, V, F, G> IntoVar<O> for MapBidiRefVar<I, O, V, F, G>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: Fn(&I) -> &O + Clone + 'static,
    G: Fn(&mut I) -> &mut O + Clone + 'static,
{
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}
