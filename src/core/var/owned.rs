use super::{protected, IntoVar, LocalVar, MapVar, MapVarBiDi, MapVarBiDiInner, MapVarInner, ObjVar, Var, VarValue};
use crate::core::context::Vars;
use std::rc::Rc;

/// [`Var`](Var) implementer that owns the value.
#[derive(Clone)]
pub struct OwnedVar<T: VarValue>(pub T);

impl<T: VarValue> protected::Var<T> for OwnedVar<T> {
    fn bind_info<'a, 'b>(&'a self, _: &'b Vars) -> protected::BindInfo<'a, T> {
        protected::BindInfo::Var(&self.0, false, 0)
    }
}

impl<T: VarValue> ObjVar<T> for OwnedVar<T> {
    fn get(&self, _: &Vars) -> &T {
        &self.0
    }

    fn update<'a>(&'a self, _: &'a Vars) -> Option<&'a T> {
        None
    }

    fn is_new(&self, _: &Vars) -> bool {
        false
    }

    fn version(&self, _: &Vars) -> u32 {
        0
    }
}

impl<T: VarValue> Var<T> for OwnedVar<T> {
    type AsReadOnly = Self;
    type AsLocal = Self;

    fn map<O, M>(&self, mut map: M) -> MapVar<T, Self, O, M>
    where
        M: FnMut(&T) -> O + 'static,
        O: VarValue,
    {
        MapVar::new(MapVarInner::Owned(Rc::new(OwnedVar(map(&self.0)))))
    }

    fn into_map<O, M>(self, map: M) -> MapVar<T, Self, O, M>
    where
        M: FnMut(&T) -> O + 'static,
        O: VarValue,
    {
        self.map(map)
    }

    fn map_bidi<O, M, N>(&self, mut map: M, _: N) -> MapVarBiDi<T, Self, O, M, N>
    where
        M: FnMut(&T) -> O + 'static,
        N: FnMut(&O) -> T + 'static,
        O: VarValue,
    {
        MapVarBiDi::new(MapVarBiDiInner::Owned(Rc::new(OwnedVar(map(&self.0)))))
    }

    fn as_read_only(self) -> Self {
        self
    }

    fn as_local(self) -> Self {
        self
    }
}

impl<T: VarValue> LocalVar<T> for OwnedVar<T> {
    fn get_local(&self) -> &T {
        &self.0
    }

    fn get_local_step(&self) -> &T {
        &self.0
    }

    fn init_local<'a, 'b>(&'a mut self, _: &'b Vars) -> &'a T {
        &self.0
    }

    fn update_local<'a, 'b>(&'a mut self, _: &'b Vars) -> Option<&'a T> {
        None
    }

    fn update_local_step<'a, 'b>(&'a mut self, _: &'b Vars) -> Option<&'a T> {
        None
    }
}

impl<T: VarValue> IntoVar<T> for OwnedVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

/// Wraps the value in an [`OwnedVar`](OwnedVar) value.
impl<T: VarValue> IntoVar<T> for T {
    type Var = OwnedVar<T>;

    fn into_var(self) -> OwnedVar<T> {
        OwnedVar(self)
    }
}
