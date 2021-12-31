use std::{
    cell::{Cell, RefCell, UnsafeCell},
    marker::PhantomData,
    rc::Rc,
};

use super::*;

/// A [`Var`] that maps from another var and is a [`Rc`] pointer to its value.
pub struct RcMapVar<A, B, M, S>(Rc<MapData<A, B, M, S>>)
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    S: Var<A>;
struct MapData<A, B, M, S> {
    _a: PhantomData<A>,

    source: S,
    map: RefCell<M>,

    value: UnsafeCell<Option<B>>,
    version: Cell<u32>,
}

impl<A, B, M, S> RcMapVar<A, B, M, S>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    S: Var<A>,
{
    /// New mapping var.
    ///
    /// Prefer using the [`Var::map`] method.
    #[inline]
    pub fn new(source: S, map: M) -> Self {
        RcMapVar(Rc::new(MapData {
            _a: PhantomData,
            source,
            map: RefCell::new(map),
            value: UnsafeCell::new(None),
            version: Cell::new(0),
        }))
    }

    /// Get the value, applies the mapping if the value is out of sync.
    pub fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a B {
        let vars = vars.as_ref();

        // SAFETY: access to value is safe because `source` needs a `&mut Vars` to change its version
        // and we change the value only in the first call to `get` with the new source version.

        let version = self.0.source.version(vars);
        let first = unsafe { &*self.0.value.get() }.is_none();

        if first || version != self.0.version.get() {
            let new_value = self.0.map.borrow_mut()(self.0.source.get(vars));

            unsafe {
                *self.0.value.get() = Some(new_value);
            }

            self.0.version.set(version);
        }

        unsafe { &*self.0.value.get() }.as_ref().unwrap()
    }

    /// Get the value if the source var updated in the last update.
    pub fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a B> {
        let vars = vars.as_ref();

        if self.0.source.is_new(vars) {
            Some(self.get(vars))
        } else {
            None
        }
    }

    /// Gets if the source var updated in the last update.
    #[inline]
    pub fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
        self.0.source.is_new(vars)
    }

    /// Gets the source var value version.
    #[inline]
    pub fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> u32 {
        self.0.source.version(vars)
    }

    /// Gets the number of [`RcMapBidiVar`] that point to this same variable.
    #[inline]
    pub fn strong_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }

    /// Returns `true` if `self` and `other` are the same variable.
    #[inline]
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<A, B, M, S> Clone for RcMapVar<A, B, M, S>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    S: Var<A>,
{
    fn clone(&self) -> Self {
        RcMapVar(Rc::clone(&self.0))
    }
}
impl<A, B, M, S> crate::private::Sealed for RcMapVar<A, B, M, S>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    S: Var<A>,
{
}
impl<A, B, M, S> Var<B> for RcMapVar<A, B, M, S>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    S: Var<A>,
{
    type AsReadOnly = Self;

    #[inline]
    fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a B {
        self.get(vars)
    }

    #[inline]
    fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a B> {
        self.get_new(vars)
    }

    #[inline]
    fn into_value<Vr: WithVarsRead>(self, vars: &Vr) -> B {
        self.get_clone(vars)
    }

    #[inline]
    fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
        self.is_new(vars)
    }

    #[inline]
    fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> u32 {
        self.version(vars)
    }

    #[inline]
    fn is_read_only<Vw: WithVars>(&self, _: &Vw) -> bool {
        true
    }

    #[inline]
    fn always_read_only(&self) -> bool {
        true
    }

    #[inline]
    fn can_update(&self) -> bool {
        self.0.source.can_update()
    }

    #[inline]
    fn strong_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }

    #[inline]
    fn modify<Vw, Mo>(&self, _: &Vw, _: Mo) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        Mo: FnOnce(&mut VarModify<B>) + 'static,
    {
        Err(VarIsReadOnly)
    }

    #[inline]
    fn set<Vw, N>(&self, _: &Vw, _: N) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<B>,
    {
        Err(VarIsReadOnly)
    }

    #[inline]
    fn set_ne<Vw, N>(&self, _: &Vw, _: N) -> Result<bool, VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<B>,
        B: PartialEq,
    {
        Err(VarIsReadOnly)
    }

    #[inline]
    fn into_read_only(self) -> Self::AsReadOnly {
        self
    }

    #[inline]
    fn update_mask<Vr: WithVarsRead>(&self, vars: &Vr) -> UpdateMask {
        self.0.source.update_mask(vars)
    }
}
impl<A, B, M, S> IntoVar<B> for RcMapVar<A, B, M, S>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    S: Var<A>,
{
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

/// A [`Var`] that maps from-and-to another var and is a [`Rc`] pointer to its value.
pub struct RcMapBidiVar<A, B, M, N, S>(Rc<MapBidiData<A, B, M, N, S>>)
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    N: FnMut(B) -> A + 'static,
    S: Var<A>;

struct MapBidiData<A, B, M, N, S> {
    _a: PhantomData<A>,

    source: S,
    map: RefCell<M>,
    map_back: RefCell<N>,

    value: UnsafeCell<Option<B>>,
    version: Cell<u32>,
}

impl<A, B, M, N, S> RcMapBidiVar<A, B, M, N, S>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    N: FnMut(B) -> A + 'static,
    S: Var<A>,
{
    /// New bidirectional mapping var.
    ///
    /// Prefer using the [`Var::map_bidi`] method.
    #[inline]
    pub fn new(source: S, map: M, map_back: N) -> Self {
        RcMapBidiVar(Rc::new(MapBidiData {
            _a: PhantomData,
            source,
            map: RefCell::new(map),
            map_back: RefCell::new(map_back),
            value: UnsafeCell::new(None),
            version: Cell::new(0),
        }))
    }

    /// Get the value, applies the mapping if the value is out of sync.
    pub fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a B {
        let vars = vars.as_ref();

        // SAFETY: access to value is safe because `source` needs a `&mut Vars` to change its version
        // and we change the value only in the first call to `get` with the new source version.

        let version = self.0.source.version(vars);
        let first = unsafe { &*self.0.value.get() }.is_none();

        if first || version != self.0.version.get() {
            let new_value = self.0.map.borrow_mut()(self.0.source.get(vars));

            unsafe {
                *self.0.value.get() = Some(new_value);
            }

            self.0.version.set(version);
        }

        unsafe { &*self.0.value.get() }.as_ref().unwrap()
    }

    /// Get the value if the source var updated in the last update.
    pub fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a B> {
        let vars = vars.as_ref();

        if self.0.source.is_new(vars) {
            Some(self.get(vars))
        } else {
            None
        }
    }

    /// Gets if the source var updated in the last update.
    #[inline]
    pub fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
        self.0.source.is_new(vars)
    }

    /// Gets the source var value version.
    #[inline]
    pub fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> u32 {
        self.0.source.version(vars)
    }

    /// If the source variable is currently read-only. You can only map-back when the source is read-write.
    #[inline]
    pub fn is_read_only<Vw: WithVars>(&self, vars: &Vw) -> bool {
        self.0.source.is_read_only(vars)
    }

    /// If the source variable is always read-only. If `true` you can never map-back a value so this variable
    /// is equivalent to a [`RcMapVar`].
    #[inline]
    pub fn always_read_only(&self) -> bool {
        self.0.source.always_read_only()
    }

    /// If the source variable value can change.
    #[inline]
    pub fn can_update(&self) -> bool {
        self.0.source.can_update()
    }

    /// Schedules a `map -> modify -> map_back -> set` chain.
    fn modify<Vw, Mo>(&self, vars: &Vw, modify: Mo) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        Mo: FnOnce(&mut VarModify<B>) + 'static,
    {
        let self_ = self.clone();
        self.0.source.modify(vars, move |source_value| {
            let mut mapped_value = self_.0.map.borrow_mut()(source_value);
            let mut guard = VarModify::new(&mut mapped_value);
            modify(&mut guard);
            if guard.touched() {
                **source_value = self_.0.map_back.borrow_mut()(mapped_value);
            }
        })
    }

    /// Map back the value and schedules a `set` in the source variable.
    fn set<Vw, Nv>(&self, vars: &Vw, new_value: Nv) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        Nv: Into<B>,
    {
        if self.0.source.is_read_only(vars) {
            Err(VarIsReadOnly)
        } else {
            let new_value = self.0.map_back.borrow_mut()(new_value.into());
            self.0.source.set(vars, new_value)
        }
    }

    /// If `new_value` is not equal to the current value maps-back and schedules an assign in the source value.
    fn set_ne<Vw, Nv>(&self, vars: &Vw, new_value: Nv) -> Result<bool, VarIsReadOnly>
    where
        Vw: WithVars,
        Nv: Into<B>,
        B: PartialEq,
    {
        if self.0.source.is_read_only(vars) {
            Err(VarIsReadOnly)
        } else {
            let new_value = new_value.into();
            vars.with_vars(|vars| {
                if self.get(vars) != &new_value {
                    let _ = self.0.source.set(vars, self.0.map_back.borrow_mut()(new_value));
                    Ok(true)
                } else {
                    Ok(false)
                }
            })
        }
    }

    /// Convert to a [`RcMapVar`] if `self` is the only reference.
    #[inline]
    pub fn into_map(self) -> Result<RcMapVar<A, B, M, S>, Self> {
        match Rc::try_unwrap(self.0) {
            Ok(data) => Ok(RcMapVar(Rc::new(MapData {
                _a: PhantomData,
                source: data.source,
                map: data.map,
                value: data.value,
                version: data.version,
            }))),
            Err(rc) => Err(Self(rc)),
        }
    }

    /// Gets the number of [`RcMapBidiVar`] that point to this same variable.
    #[inline]
    pub fn strong_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }

    /// Returns `true` if `self` and `other` are the same variable.
    #[inline]
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<A, B, M, N, S> Clone for RcMapBidiVar<A, B, M, N, S>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    N: FnMut(B) -> A + 'static,
    S: Var<A>,
{
    fn clone(&self) -> Self {
        RcMapBidiVar(Rc::clone(&self.0))
    }
}
impl<A, B, M, N, S> crate::private::Sealed for RcMapBidiVar<A, B, M, N, S>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    N: FnMut(B) -> A + 'static,
    S: Var<A>,
{
}
impl<A, B, M, N, S> Var<B> for RcMapBidiVar<A, B, M, N, S>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    N: FnMut(B) -> A + 'static,
    S: Var<A>,
{
    type AsReadOnly = ReadOnlyVar<B, Self>;

    #[inline]
    fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a B {
        self.get(vars)
    }

    #[inline]
    fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a B> {
        self.get_new(vars)
    }

    #[inline]
    fn into_value<Vr: WithVarsRead>(self, vars: &Vr) -> B {
        self.get_clone(vars)
    }

    #[inline]
    fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
        self.is_new(vars)
    }

    #[inline]
    fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> u32 {
        self.version(vars)
    }

    #[inline]
    fn is_read_only<Vw: WithVars>(&self, vars: &Vw) -> bool {
        self.is_read_only(vars)
    }

    #[inline]
    fn always_read_only(&self) -> bool {
        self.always_read_only()
    }

    #[inline]
    fn can_update(&self) -> bool {
        self.can_update()
    }

    #[inline]
    fn strong_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }

    #[inline]
    fn modify<Vw, Mo>(&self, vars: &Vw, modify: Mo) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        Mo: FnOnce(&mut VarModify<B>) + 'static,
    {
        self.modify(vars, modify)
    }

    #[inline]
    fn set<Vw, Nv>(&self, vars: &Vw, new_value: Nv) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        Nv: Into<B>,
    {
        self.set(vars, new_value)
    }

    #[inline]
    fn set_ne<Vw, Nv>(&self, vars: &Vw, new_value: Nv) -> Result<bool, VarIsReadOnly>
    where
        Vw: WithVars,
        Nv: Into<B>,
        B: PartialEq,
    {
        self.set_ne(vars, new_value)
    }

    #[inline]
    fn into_read_only(self) -> Self::AsReadOnly {
        ReadOnlyVar::new(self)
    }

    #[inline]
    fn update_mask<Vr: WithVarsRead>(&self, vars: &Vr) -> UpdateMask {
        self.0.source.update_mask(vars)
    }
}
impl<A, B, M, N, S> IntoVar<B> for RcMapBidiVar<A, B, M, N, S>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    N: FnMut(B) -> A + 'static,
    S: Var<A>,
{
    type Var = Self;

    #[inline]
    fn into_var(self) -> Self::Var {
        self
    }
}
