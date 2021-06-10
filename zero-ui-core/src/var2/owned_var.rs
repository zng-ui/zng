use super::*;

/// A [`Var`] that owns the value and keeps it locally.
#[derive(Clone)]
pub struct OwnedVar<T: VarValue>(pub T);
impl<T: VarValue> Var<T> for OwnedVar<T> {
    type AsReadOnly = Self;

    type AsLocal = Self;

    fn get<'a>(&'a self, _: &'a VarsRead) -> &'a T {
        &self.0
    }

    fn get_new<'a>(&'a self, _: &'a Vars) -> Option<&'a T> {
        None
    }

    fn version(&self, _: &VarsRead) -> u32 {
        0
    }

    fn is_read_only(&self, _: &VarsRead) -> bool {
        true
    }

    fn always_read_only(&self) -> bool {
        true
    }

    fn can_update(&self) -> bool {
        false
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
        self
    }
}
impl<T: VarValue> VarLocal<T> for OwnedVar<T> {
    fn get_local(&self) -> &T {
        &self.0
    }

    fn init_local<'a>(&'a mut self, _: &'a Vars) -> &'a T {
        &self.0
    }

    fn update_local<'a>(&'a mut self, _: &'a Vars) -> Option<&'a T> {
        None
    }
}

impl<A, B, M> VarMap<A, B, M> for OwnedVar<A>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
{
    type MapVar = OwnedVar<B>;

    fn map_impl(&self, mut map: M) -> Self::MapVar {
        OwnedVar(map(&self.0))
    }

    fn into_map_impl(self, map: M) -> Self::MapVar {
        self.map_impl(map)
    }
}

impl<A, B, M, N> VarMapBidi<A, B, M, N> for OwnedVar<A>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    N: FnMut(&B) -> A + 'static,
{
    type MapBidiVar = OwnedVar<B>;

    fn map_bidi_impl(&self, map: M, _: N) -> Self::MapBidiVar {
        self.map_impl(map)
    }

    fn into_map_bidi_impl(self, map: M, _: N) -> Self::MapBidiVar {
        self.map_impl(map)
    }
}
