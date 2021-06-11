use std::marker::PhantomData;

use super::*;

/// A [`Var`] that maps a reference from the value of another variable.
pub struct MapRefVar<A, B, M, S>
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + Clone + 'static,
    S: Var<A>,
{
    _ab: PhantomData<(A, B)>,
    source: S,
    map: M,
}

impl<A, B, M, S> MapRefVar<A, B, M, S>
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + Clone + 'static,
    S: Var<A>,
{
    /// New reference mapping var.
    pub fn new(source: S, map: M) -> Self {
        MapRefVar {
            _ab: PhantomData,
            map,
            source,
        }
    }

    /// Gets the mapped reference.
    #[inline]
    pub fn get<'a>(&'a self, vars: &'a VarsRead) -> &'a B {
        (self.map)(self.source.get(vars))
    }

    /// Gets the mapped reference if the value of the source variable is new.
    #[inline]
    pub fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a B> {
        self.source.get_new(vars).map(|v| (self.map)(v))
    }

    /// Gets if the value of the source variable is new.
    #[inline]
    pub fn is_new(&self, vars: &Vars) -> bool {
        self.source.is_new(vars)
    }

    /// Gets the version of the source variable value.
    #[inline]
    pub fn version(&self, vars: &VarsRead) -> u32 {
        self.source.version(vars)
    }

    /// Gets if the source value can update.
    #[inline]
    pub fn can_update(&self) -> bool {
        self.source.can_update()
    }
}

impl<A, B, M, S> Clone for MapRefVar<A, B, M, S>
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + Clone + 'static,
    S: Var<A>,
{
    fn clone(&self) -> Self {
        MapRefVar {
            _ab: PhantomData,
            source: self.source.clone(),
            map: self.map.clone(),
        }
    }
}

impl<A, B, M, S> Var<B> for MapRefVar<A, B, M, S>
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + Clone + 'static,
    S: Var<A>,
{
    type AsReadOnly = Self;

    type AsLocal = CloningLocalVar<B, Self>;

    fn get<'a>(&'a self, vars: &'a VarsRead) -> &'a B {
        self.get(vars)
    }

    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a B> {
        self.get_new(vars)
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.is_new(vars)
    }

    fn version(&self, vars: &VarsRead) -> u32 {
        self.version(vars)
    }

    fn is_read_only(&self, _: &Vars) -> bool {
        true
    }

    fn always_read_only(&self) -> bool {
        true
    }

    fn can_update(&self) -> bool {
        self.can_update()
    }

    fn modify<Mo>(&self, _: &Vars, _: Mo) -> Result<(), VarIsReadOnly>
    where
        Mo: FnOnce(&mut VarModify<B>) + 'static,
    {
        Err(VarIsReadOnly)
    }

    fn set(&self, _: &Vars, _: B) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    fn set_ne(&self, _: &Vars, _: B) -> Result<(), VarIsReadOnly>
    where
        B: PartialEq,
    {
        Err(VarIsReadOnly)
    }

    fn into_read_only(self) -> Self::AsReadOnly {
        self
    }

    fn into_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }
}

impl<A, B, M, S> IntoVar<B> for MapRefVar<A, B, M, S>
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + Clone + 'static,
    S: Var<A>,
{
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

/// A [`Var`] that maps a mutable reference from the value of another variable.
pub struct MapBidiRefVar<A, B, M, N, S>
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + Clone + 'static,
    N: Fn(&mut A) -> &mut B + Clone + 'static,
    S: Var<A>,
{
    _ab: PhantomData<(A, B)>,
    source: S,
    map: M,
    map_mut: N,
}

impl<A, B, M, N, S> MapBidiRefVar<A, B, M, N, S>
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + Clone + 'static,
    N: Fn(&mut A) -> &mut B + Clone + 'static,
    S: Var<A>,
{
    /// New bidirectional reference mapping variable.
    pub fn new(source: S, map: M, map_mut: N) -> Self {
        MapBidiRefVar {
            _ab: PhantomData,
            source,
            map,
            map_mut,
        }
    }

    /// Gets the mapped reference.
    #[inline]
    pub fn get<'a>(&'a self, vars: &'a VarsRead) -> &'a B {
        (self.map)(self.source.get(vars))
    }

    /// Gets the mapped reference if the value of the source variable is new.
    #[inline]
    pub fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a B> {
        self.source.get_new(vars).map(|v| (self.map)(v))
    }

    /// Gets if the value of the source variable is new.
    #[inline]
    pub fn is_new(&self, vars: &Vars) -> bool {
        self.source.is_new(vars)
    }

    /// Gets the version of the source variable value.
    #[inline]
    pub fn version(&self, vars: &VarsRead) -> u32 {
        self.source.version(vars)
    }

    /// Gets if the source value can update.
    #[inline]
    pub fn can_update(&self) -> bool {
        self.source.can_update()
    }

    /// Gets if the source is currently read-only.
    #[inline]
    pub fn is_read_only(&self, vars: &Vars) -> bool {
        self.source.is_read_only(vars)
    }

    /// Gets if the source is always read-only. If `true` you can assign or modify the value so this variable
    /// is equivalent to a [`MapRefVar`].
    #[inline]
    pub fn always_read_only(&self) -> bool {
        self.source.always_read_only()
    }

    /// Schedules a modification using the mapped mutable reference.
    pub fn modify<Mo>(&self, vars: &Vars, modify: Mo) -> Result<(), VarIsReadOnly>
    where
        Mo: FnOnce(&mut VarModify<B>) + 'static,
    {
        let map = self.map_mut.clone();
        self.source.modify(vars, |v| {
            v.map_ref(map, modify);
        })
    }

    /// Schedules an assign to the mapped mutable reference.
    pub fn set(&self, vars: &Vars, new_value: B) -> Result<(), VarIsReadOnly> {
        let map = self.map_mut.clone();
        self.source.modify(vars, move |v| {
            *map(v) = new_value;
        })
    }

    /// Schedules an assign to the mapped mutable reference, but only if the value is not equal.
    pub fn set_ne(&self, vars: &Vars, new_value: B) -> Result<(), VarIsReadOnly>
    where
        B: PartialEq,
    {
        let map = self.map_mut.clone();
        self.source.modify(vars, |v| {
            v.map_ref(map, |v| {
                if !v.eq(&new_value) {
                    **v = new_value;
                }
            })
        })
    }

    /// Convert this variable into a [`MapRefVar`].
    #[inline]
    pub fn into_map(self) -> MapRefVar<A, B, M, S> {
        MapRefVar {
            _ab: PhantomData,
            source: self.source,
            map: self.map,
        }
    }
}

impl<A, B, M, N, S> Clone for MapBidiRefVar<A, B, M, N, S>
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + Clone + 'static,
    N: Fn(&mut A) -> &mut B + Clone + 'static,
    S: Var<A>,
{
    fn clone(&self) -> Self {
        MapBidiRefVar {
            _ab: PhantomData,
            source: self.source.clone(),
            map: self.map.clone(),
            map_mut: self.map_mut.clone(),
        }
    }
}

impl<A, B, M, N, S> Var<B> for MapBidiRefVar<A, B, M, N, S>
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + Clone + 'static,
    N: Fn(&mut A) -> &mut B + Clone + 'static,
    S: Var<A>,
{
    type AsReadOnly = ReadOnlyVar<B, Self>;

    type AsLocal = CloningLocalVar<B, Self>;

    fn get<'a>(&'a self, vars: &'a VarsRead) -> &'a B {
        self.get(vars)
    }

    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a B> {
        self.get_new(vars)
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.is_new(vars)
    }

    fn version(&self, vars: &VarsRead) -> u32 {
        self.version(vars)
    }

    fn is_read_only(&self, vars: &Vars) -> bool {
        self.is_read_only(vars)
    }

    fn always_read_only(&self) -> bool {
        self.always_read_only()
    }

    fn can_update(&self) -> bool {
        self.can_update()
    }

    fn modify<Mo>(&self, vars: &Vars, modify: Mo) -> Result<(), VarIsReadOnly>
    where
        Mo: FnOnce(&mut VarModify<B>) + 'static,
    {
        self.modify(vars, modify)
    }

    fn set(&self, vars: &Vars, new_value: B) -> Result<(), VarIsReadOnly> {
        self.set(vars, new_value)
    }

    fn set_ne(&self, vars: &Vars, new_value: B) -> Result<(), VarIsReadOnly>
    where
        B: PartialEq,
    {
        self.set_ne(vars, new_value)
    }

    fn into_read_only(self) -> Self::AsReadOnly {
        ReadOnlyVar::new(self)
    }

    fn into_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }
}

impl<A, B, M, N, S> IntoVar<B> for MapBidiRefVar<A, B, M, N, S>
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + Clone + 'static,
    N: Fn(&mut A) -> &mut B + Clone + 'static,
    S: Var<A>,
{
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}
