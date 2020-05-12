use super::{
    protected, CloningLocalVar, IntoVar, MapBiDiSharedVar, MapSharedVar, MapVar, MapVarBiDi, MapVarBiDiInner, MapVarInner, ObjVar, Var,
    VarValue,
};
use crate::core::context::Vars;
use std::marker::PhantomData;

/// A variable that is [`always_read_only`](ObjVar::always_read_only).
///
/// This `struct` is created by the [`as_read_only`](Var::as_read_only) method in variables
/// that are not `always_read_only`.
pub struct ReadOnlyVar<T: VarValue, V: Var<T> + Clone> {
    _t: PhantomData<T>,
    var: V,
}

impl<T: VarValue, V: Var<T> + Clone> ReadOnlyVar<T, V> {
    pub(crate) fn new(var: V) -> Self {
        ReadOnlyVar { _t: PhantomData, var }
    }
}

impl<T: VarValue, V: Var<T> + Clone> protected::Var<T> for ReadOnlyVar<T, V> {
    fn bind_info<'a>(&'a self, vars: &'a Vars) -> protected::BindInfo<'a, T> {
        self.var.bind_info(vars)
    }
}

impl<T: VarValue, V: Var<T> + Clone> ObjVar<T> for ReadOnlyVar<T, V> {
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a T {
        self.var.get(vars)
    }

    /// [`get`](ObjVar::get) if [`is_new`](ObjVar::is_new) or none.
    fn update<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        self.var.update(vars)
    }

    /// If the value changed this update.
    fn is_new(&self, vars: &Vars) -> bool {
        self.var.is_new(vars)
    }

    /// Current value version. Version changes every time the value changes.
    fn version(&self, vars: &Vars) -> u32 {
        self.var.version(vars)
    }
}

impl<T: VarValue, V: Var<T>> Clone for ReadOnlyVar<T, V> {
    fn clone(&self) -> Self {
        ReadOnlyVar {
            _t: PhantomData,
            var: self.var.clone(),
        }
    }
}

impl<T: VarValue, V: Var<T>> Var<T> for ReadOnlyVar<T, V> {
    type AsReadOnly = Self;
    type AsLocal = CloningLocalVar<T, Self>;

    fn map<O, M>(&self, map: M) -> MapVar<T, Self, O, M>
    where
        M: FnMut(&T) -> O + 'static,
        O: VarValue,
    {
        self.clone().into_map(map)
    }

    fn into_map<O, M>(self, map: M) -> MapVar<T, Self, O, M>
    where
        M: FnMut(&T) -> O + 'static,
        O: VarValue,
    {
        let prev_version = self.var.read_only_prev_version();
        MapVar::new(MapVarInner::Shared(MapSharedVar::new(self, map, prev_version)))
    }

    fn map_bidi<O, M, N>(&self, map: M, map_back: N) -> MapVarBiDi<T, Self, O, M, N>
    where
        M: FnMut(&T) -> O + 'static,
        N: FnMut(&O) -> T + 'static,
        O: VarValue,
    {
        MapVarBiDi::new(MapVarBiDiInner::Shared(MapBiDiSharedVar::new(
            self.clone(),
            map,
            map_back,
            self.var.read_only_prev_version(),
        )))
    }

    fn as_read_only(self) -> Self {
        self
    }

    fn as_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }
}

impl<T: VarValue, V: Var<T>> IntoVar<T> for ReadOnlyVar<T, V> {
    type Var = Self;
    #[inline]
    fn into_var(self) -> Self::Var {
        self
    }
}
