use std::marker::PhantomData;

use super::*;

/// A [`Var`] wrapper that makes it [`always_read_only`](Var::always_read_only).
pub struct ForceReadOnlyVar<T: VarValue, V: Var<T>>(V, PhantomData<T>);

impl<T, V> ForceReadOnlyVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
    /// Wrap var.
    pub fn new(var: V) -> Self {
        ForceReadOnlyVar(var, PhantomData)
    }
}

impl<T, V> Clone for ForceReadOnlyVar<T, V>
where
    T: VarValue,
    V: Var<T> + Clone,
{
    fn clone(&self) -> Self {
        ForceReadOnlyVar(self.0.clone(), PhantomData)
    }
}

impl<T, V> Var<T> for ForceReadOnlyVar<T, V>
where
    T: VarValue,
    V: Var<T>,
{
    type AsReadOnly = Self;

    type AsLocal = ForceReadOnlyVar<T, V::AsLocal>;

    fn get<'a>(&'a self, vars: &'a VarsRead) -> &'a T {
        self.0.get(vars)
    }

    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        self.0.get_new(vars)
    }

    fn version(&self, vars: &VarsRead) -> u32 {
        self.0.version(vars)
    }

    fn is_read_only(&self, _: &VarsRead) -> bool {
        true
    }

    fn always_read_only(&self) -> bool {
        true
    }

    fn can_update(&self) -> bool {
        self.0.can_update()
    }

    fn modify<M>(&self, _: &Vars, _: M) -> Result<(), VarIsReadOnly>
    where
        M: FnOnce(&mut VarModify<T>) + 'static,
    {
        Err(VarIsReadOnly)
    }

    fn set(&self, _: &Vars, _: T) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    fn set_ne(&self, _: &Vars, _: T) -> Result<(), VarIsReadOnly>
    where
        T: PartialEq,
    {
        Err(VarIsReadOnly)
    }

    fn into_read_only(self) -> Self::AsReadOnly {
        self
    }

    fn into_local(self) -> Self::AsLocal {
        ForceReadOnlyVar::new(self.0.into_local())
    }
}

impl<T, V> VarLocal<T> for ForceReadOnlyVar<T, V>
where
    T: VarValue,
    V: Var<T> + VarLocal<T>,
{
    fn get_local(&self) -> &T {
        self.0.get_local()
    }

    fn init_local<'a>(&'a mut self, vars: &'a Vars) -> &'a T {
        self.0.init_local(vars)
    }

    fn update_local<'a>(&'a mut self, vars: &'a Vars) -> Option<&'a T> {
        self.0.update_local(vars)
    }
}

impl<A, B, M, V> VarMap<A, B, M> for ForceReadOnlyVar<A, V>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    V: Var<A> + VarMap<A, B, M>,
{
    type MapVar = V::MapVar;

    fn map_impl(&self, map: M) -> Self::MapVar {
        self.0.map(map)
    }

    fn into_map_impl(self, map: M) -> Self::MapVar {
        self.0.into_map(map)
    }
}

impl<A, B, M, N, V> VarMapBidi<A, B, M, N> for ForceReadOnlyVar<A, V>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    N: FnMut(&B) -> A + 'static,
    V: Var<A> + VarMapBidi<A, B, M, N>,
{
    type MapBidiVar = ForceReadOnlyVar<B, V::MapBidiVar>;

    fn map_bidi_impl(&self, map: M, map_back: N) -> Self::MapBidiVar {
        ForceReadOnlyVar::new(self.0.map_bidi(map, map_back))
    }

    fn into_map_bidi_impl(self, map: M, map_back: N) -> Self::MapBidiVar {
        ForceReadOnlyVar::new(self.0.into_map_bidi(map, map_back))
    }
}
