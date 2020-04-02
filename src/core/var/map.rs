use super::{
    context::MapContextVar, protected, CloningLocalVar, IntoVar, MapBiDiSharedVar, MapSharedVar, ObjVar, OwnedVar, ReadOnlyVar, Var,
    VarIsReadOnly, VarValue,
};
use crate::core::context::{Updates, Vars};
use std::rc::Rc;

pub(crate) enum MapVarInner<T, S, O, M>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
{
    Owned(Rc<OwnedVar<O>>),
    Shared(MapSharedVar<T, S, O, M>),
    Context(MapContextVar<T, S, O, M>),
}

pub(crate) enum MapVarBiDiInner<T, S, O, M, N>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
    N: FnMut(&O) -> T + 'static,
{
    Owned(Rc<OwnedVar<O>>),
    Shared(MapBiDiSharedVar<T, S, O, M, N>),
    Context(MapContextVar<T, S, O, M>),
}

/// A variable that maps the value of another variable.
///
/// This `struct` is created by the [map](Var::map) method and is a temporary adapter until
/// [GATs](https://github.com/rust-lang/rust/issues/44265) are stable.
pub struct MapVar<T, S, O, M>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
{
    r: MapVarInner<T, S, O, M>,
}

/// A variable that maps from and to another variable.
///
/// This `struct` is created by the [map_bidi](Var::map_bidi) method and is a temporary adapter until
/// [GATs](https://github.com/rust-lang/rust/issues/44265) are stable.
pub struct MapVarBiDi<T, S, O, M, N>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
    N: FnMut(&O) -> T + 'static,
{
    r: MapVarBiDiInner<T, S, O, M, N>,
}

impl<T, S, O, M> MapVar<T, S, O, M>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
{
    pub(crate) fn new(inner: MapVarInner<T, S, O, M>) -> Self {
        MapVar { r: inner }
    }
}

impl<T, S, O, M, N> MapVarBiDi<T, S, O, M, N>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
    N: FnMut(&O) -> T + 'static,
{
    pub(crate) fn new(inner: MapVarBiDiInner<T, S, O, M, N>) -> Self {
        MapVarBiDi { r: inner }
    }
}

impl<T, S, O, M> protected::Var<O> for MapVar<T, S, O, M>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
{
    fn bind_info<'a>(&'a self, vars: &'a Vars) -> protected::BindInfo<'a, O> {
        match &self.r {
            MapVarInner::Owned(o) => o.bind_info(vars),
            MapVarInner::Shared(s) => s.bind_info(vars),
            MapVarInner::Context(c) => c.bind_info(vars),
        }
    }
}

impl<T, S, O, M, N> protected::Var<O> for MapVarBiDi<T, S, O, M, N>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
    N: FnMut(&O) -> T + 'static,
{
    fn bind_info<'a>(&'a self, vars: &'a Vars) -> protected::BindInfo<'a, O> {
        match &self.r {
            MapVarBiDiInner::Owned(o) => o.bind_info(vars),
            MapVarBiDiInner::Shared(s) => s.bind_info(vars),
            MapVarBiDiInner::Context(c) => c.bind_info(vars),
        }
    }

    fn read_only_prev_version(&self) -> u32 {
        todo!()
    }
}

impl<T, S, O, M> ObjVar<O> for MapVar<T, S, O, M>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
{
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a O {
        match &self.r {
            MapVarInner::Owned(o) => o.get(vars),
            MapVarInner::Shared(s) => s.get(vars),
            MapVarInner::Context(c) => c.get(vars),
        }
    }

    fn update<'a>(&'a self, vars: &'a Vars) -> Option<&'a O> {
        match &self.r {
            MapVarInner::Owned(o) => o.update(vars),
            MapVarInner::Shared(s) => s.update(vars),
            MapVarInner::Context(c) => c.update(vars),
        }
    }

    fn is_new(&self, vars: &Vars) -> bool {
        match &self.r {
            MapVarInner::Owned(o) => o.is_new(vars),
            MapVarInner::Shared(s) => s.is_new(vars),
            MapVarInner::Context(c) => c.is_new(vars),
        }
    }

    fn version(&self, vars: &Vars) -> u32 {
        match &self.r {
            MapVarInner::Owned(o) => o.version(vars),
            MapVarInner::Shared(s) => s.version(vars),
            MapVarInner::Context(c) => c.version(vars),
        }
    }
}

impl<T, S, O, M, N> ObjVar<O> for MapVarBiDi<T, S, O, M, N>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
    N: FnMut(&O) -> T + 'static,
{
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a O {
        match &self.r {
            MapVarBiDiInner::Owned(o) => o.get(vars),
            MapVarBiDiInner::Shared(s) => s.get(vars),
            MapVarBiDiInner::Context(c) => c.get(vars),
        }
    }

    fn update<'a>(&'a self, vars: &'a Vars) -> Option<&'a O> {
        match &self.r {
            MapVarBiDiInner::Owned(o) => o.update(vars),
            MapVarBiDiInner::Shared(s) => s.update(vars),
            MapVarBiDiInner::Context(c) => c.update(vars),
        }
    }

    fn is_new(&self, vars: &Vars) -> bool {
        match &self.r {
            MapVarBiDiInner::Owned(o) => o.is_new(vars),
            MapVarBiDiInner::Shared(s) => s.is_new(vars),
            MapVarBiDiInner::Context(c) => c.is_new(vars),
        }
    }

    fn version(&self, vars: &Vars) -> u32 {
        match &self.r {
            MapVarBiDiInner::Owned(o) => o.version(vars),
            MapVarBiDiInner::Shared(s) => s.version(vars),
            MapVarBiDiInner::Context(c) => c.version(vars),
        }
    }

    fn read_only(&self, vars: &Vars) -> bool {
        match &self.r {
            MapVarBiDiInner::Owned(o) => o.read_only(vars),
            MapVarBiDiInner::Shared(s) => s.read_only(vars),
            MapVarBiDiInner::Context(c) => c.read_only(vars),
        }
    }

    fn always_read_only(&self, vars: &Vars) -> bool {
        match &self.r {
            MapVarBiDiInner::Owned(o) => o.always_read_only(vars),
            MapVarBiDiInner::Shared(s) => s.always_read_only(vars),
            MapVarBiDiInner::Context(c) => c.always_read_only(vars),
        }
    }

    fn push_set(&self, new_value: O, vars: &Vars, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
        match &self.r {
            MapVarBiDiInner::Owned(o) => o.push_set(new_value, vars, updates),
            MapVarBiDiInner::Shared(s) => s.push_set(new_value, vars, updates),
            MapVarBiDiInner::Context(c) => c.push_set(new_value, vars, updates),
        }
    }

    fn push_modify_boxed(
        &self,
        modify: Box<dyn FnOnce(&mut O) + 'static>,
        vars: &Vars,
        updates: &mut Updates,
    ) -> Result<(), VarIsReadOnly> {
        match &self.r {
            MapVarBiDiInner::Owned(o) => o.push_modify_boxed(modify, vars, updates),
            MapVarBiDiInner::Shared(s) => s.push_modify_boxed(modify, vars, updates),
            MapVarBiDiInner::Context(c) => c.push_modify_boxed(modify, vars, updates),
        }
    }
}

impl<T, S, O, M> Clone for MapVar<T, S, O, M>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
{
    fn clone(&self) -> Self {
        MapVar {
            r: match &self.r {
                MapVarInner::Owned(o) => MapVarInner::Owned(Rc::clone(&o)),
                MapVarInner::Shared(s) => MapVarInner::Shared(s.clone()),
                MapVarInner::Context(c) => MapVarInner::Context(c.clone()),
            },
        }
    }
}

impl<T, S, O, M, N> Clone for MapVarBiDi<T, S, O, M, N>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
    N: FnMut(&O) -> T + 'static,
{
    fn clone(&self) -> Self {
        MapVarBiDi {
            r: match &self.r {
                MapVarBiDiInner::Owned(o) => MapVarBiDiInner::Owned(Rc::clone(&o)),
                MapVarBiDiInner::Shared(s) => MapVarBiDiInner::Shared(s.clone()),
                MapVarBiDiInner::Context(c) => MapVarBiDiInner::Context(c.clone()),
            },
        }
    }
}

impl<T, S, O, M> Var<O> for MapVar<T, S, O, M>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
{
    type AsReadOnly = Self;
    type AsLocal = CloningLocalVar<O, Self>;

    fn map<O2, M2>(&self, map: M2) -> MapVar<O, Self, O2, M2>
    where
        O2: VarValue,
        M2: FnMut(&O) -> O2,
    {
        MapVar::new(MapVarInner::Shared(MapSharedVar::new(self.clone(), map, 0)))
        // TODO prev_version?
    }

    fn map_bidi<O2, M2, N2>(&self, map: M2, map_back: N2) -> MapVarBiDi<O, Self, O2, M2, N2>
    where
        O2: VarValue,
        M2: FnMut(&O) -> O2,
        N2: FnMut(&O2) -> O,
    {
        MapVarBiDi::new(MapVarBiDiInner::Shared(MapBiDiSharedVar::new(self.clone(), map, map_back, 0)))
    }

    fn as_read_only(self) -> Self {
        self
    }

    fn as_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }
}

impl<T, S, O, M, N> Var<O> for MapVarBiDi<T, S, O, M, N>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
    N: FnMut(&O) -> T + 'static,
{
    type AsReadOnly = ReadOnlyVar<O, Self>;
    type AsLocal = CloningLocalVar<O, Self>;

    fn push_modify(&self, modify: impl FnOnce(&mut O) + 'static, vars: &Vars, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
        match &self.r {
            MapVarBiDiInner::Owned(o) => o.push_modify(modify, vars, updates),
            MapVarBiDiInner::Shared(s) => s.push_modify(modify, vars, updates),
            MapVarBiDiInner::Context(c) => c.push_modify(modify, vars, updates),
        }
    }

    fn map<O2, M2>(&self, map: M2) -> MapVar<O, Self, O2, M2>
    where
        O2: VarValue,
        M2: FnMut(&O) -> O2,
    {
        MapVar::new(MapVarInner::Shared(MapSharedVar::new(self.clone(), map, 0)))
        // TODO prev_version?
    }

    fn map_bidi<O2, M2, N2>(&self, map: M2, map_back: N2) -> MapVarBiDi<O, Self, O2, M2, N2>
    where
        O2: VarValue,
        M2: FnMut(&O) -> O2,
        N2: FnMut(&O2) -> O,
    {
        MapVarBiDi::new(MapVarBiDiInner::Shared(MapBiDiSharedVar::new(self.clone(), map, map_back, 0)))
    }

    fn as_read_only(self) -> Self::AsReadOnly {
        ReadOnlyVar::new(self)
    }

    fn as_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }
}

impl<T, S, O, M, N> IntoVar<O> for MapVarBiDi<T, S, O, M, N>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
    N: FnMut(&O) -> T + 'static,
{
    type Var = Self;

    #[inline]
    fn into_var(self) -> Self::Var {
        self
    }
}
