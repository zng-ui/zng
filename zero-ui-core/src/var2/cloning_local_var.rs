use super::*;

/// A [`VarLocal`] that keeps a clone of the value locally.
pub struct CloningLocalVar<T: VarValue, V: Var<T>> {
    source: V,
    local_version: u32,
    local: Option<T>,
}
impl<T: VarValue, V: Var<T>> CloningLocalVar<T, V> {
    /// New uninitialized.
    pub fn new(source: V) -> Self {
        Self {
            source,
            local_version: 0,
            local: None,
        }
    }
}
impl<T: VarValue, V: Var<T> + Clone> Clone for CloningLocalVar<T, V> {
    fn clone(&self) -> Self {
        CloningLocalVar {
            source: self.source.clone(),
            local_version: self.local_version,
            local: self.local.clone(),
        }
    }
}
impl<T: VarValue, V: Var<T>> Var<T> for CloningLocalVar<T, V> {
    type AsReadOnly = V::AsReadOnly;

    type AsLocal = Self;

    fn get<'a>(&'a self, vars: &'a VarsRead) -> &'a T {
        self.source.get(vars)
    }

    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        self.source.get_new(vars)
    }

    fn version(&self, vars: &VarsRead) -> u32 {
        self.source.version(vars)
    }

    fn is_read_only(&self, vars: &VarsRead) -> bool {
        self.source.is_read_only(vars)
    }

    fn always_read_only(&self) -> bool {
        self.source.always_read_only()
    }

    fn can_update(&self) -> bool {
        self.source.can_update()
    }

    fn modify<M>(&self, vars: &Vars, modify: M) -> Result<(), VarIsReadOnly>
    where
        M: FnOnce(&mut VarModify<T>) + 'static,
    {
        self.source.modify(vars, modify)
    }

    fn set(&self, vars: &Vars, new_value: T) -> Result<(), VarIsReadOnly> {
        self.source.set(vars, new_value)
    }

    fn set_ne(&self, vars: &Vars, new_value: T) -> Result<(), VarIsReadOnly>
    where
        T: PartialEq,
    {
        self.source.set_ne(vars, new_value)
    }

    fn into_read_only(self) -> Self::AsReadOnly {
        self.source.into_read_only()
    }

    fn into_local(self) -> Self::AsLocal {
        self
    }
}
impl<T: VarValue, V: Var<T>> VarLocal<T> for CloningLocalVar<T, V> {
    fn get_local(&self) -> &T {
        self.local.as_ref().expect("local var not initialized")
    }

    fn init_local<'a>(&'a mut self, vars: &'a Vars) -> &'a T {
        let version = self.source.version(vars);
        let value = self.source.get(vars);
        if self.local_version != version || self.local.is_none() {
            self.local = Some(value.clone());
            self.local_version = version;
        }
        value
    }

    fn update_local<'a>(&'a mut self, vars: &'a Vars) -> Option<&'a T> {
        if let Some(new_value) = self.source.get_new(vars) {
            let version = self.source.version(vars);
            if version != self.local_version {
                self.local = Some(new_value.clone());
                self.local_version = version;
            }
            Some(new_value)
        } else {
            None
        }
    }
}

impl<A, B, M, V> VarMap<A, B, M> for CloningLocalVar<A, V>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    V: Var<A> + VarMap<A, B, M>,
{
    type MapVar = V::MapVar;

    fn map_impl(&self, map: M) -> Self::MapVar {
        self.source.map(map)
    }

    fn into_map_impl(self, map: M) -> Self::MapVar {
        self.source.into_map(map)
    }
}

impl<A, B, M, N, V> VarMapBidi<A, B, M, N> for CloningLocalVar<A, V>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    N: FnMut(&B) -> A + 'static,
    V: Var<A> + VarMap<A, B, M>,
{
    type MapBidiVar = V::MapVar;

    fn map_bidi_impl(&self, map: M, _: N) -> Self::MapBidiVar {
        self.source.map(map)
    }

    fn into_map_bidi_impl(self, map: M, _: N) -> Self::MapBidiVar {
        self.source.into_map(map)
    }
}
