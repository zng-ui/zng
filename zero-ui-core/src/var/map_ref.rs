use std::{
    marker::PhantomData,
    rc::{Rc, Weak},
};

use super::*;

struct MapRefData<A, B, M, S> {
    _ab: PhantomData<(A, B)>,
    source: S,
    map: Rc<M>,
}

/// A weak reference to a [`MapRefVar`].
pub struct WeakMapRefVar<A, B, M, S>(Weak<MapRefData<A, B, M, S>>);
impl<A, B, M, S> crate::private::Sealed for WeakMapRefVar<A, B, M, S>
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + 'static,
    S: Var<A>,
{
}
impl<A, B, M, S> Clone for WeakMapRefVar<A, B, M, S>
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + 'static,
    S: Var<A>,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<A, B, M, S> WeakVar<B> for WeakMapRefVar<A, B, M, S>
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + 'static,
    S: Var<A>,
{
    type Strong = MapRefVar<A, B, M, S>;

    fn upgrade(&self) -> Option<Self::Strong> {
        self.0.upgrade().map(MapRefVar)
    }

    fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.0.weak_count()
    }

    fn as_ptr(&self) -> *const () {
        self.0.as_ptr() as _
    }
}

/// A [`Var`] that maps a reference from the value of another variable.
pub struct MapRefVar<A, B, M, S>(Rc<MapRefData<A, B, M, S>>)
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + 'static,
    S: Var<A>;

impl<A, B, M, S> MapRefVar<A, B, M, S>
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + 'static,
    S: Var<A>,
{
    /// New reference mapping var.
    ///
    /// Prefer using the [`Var::map_ref`] method.
    pub fn new(source: S, map: M) -> Self {
        Self(Rc::new(MapRefData {
            _ab: PhantomData,
            map: Rc::new(map),
            source,
        }))
    }

    /// New weak reference to the variable.
    pub fn downgrade(&self) -> WeakMapRefVar<A, B, M, S> {
        WeakMapRefVar(Rc::downgrade(&self.0))
    }
}
impl<A, B, M, S> crate::private::Sealed for MapRefVar<A, B, M, S>
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + 'static,
    S: Var<A>,
{
}
impl<A, B, M, S> Clone for MapRefVar<A, B, M, S>
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + 'static,
    S: Var<A>,
{
    fn clone(&self) -> Self {
        MapRefVar(Rc::clone(&self.0))
    }
}
impl<A, B, M, S> Var<B> for MapRefVar<A, B, M, S>
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + 'static,
    S: Var<A>,
{
    type AsReadOnly = Self;

    #[inline]
    fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a B {
        (self.0.map)(self.0.source.get(vars))
    }

    #[inline]
    fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a B> {
        self.0.source.get_new(vars).map(|v| (self.0.map)(v))
    }

    fn into_value<Vr: WithVarsRead>(self, vars: &Vr) -> B {
        self.get_clone(vars)
    }

    #[inline]
    fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
        self.0.source.is_new(vars)
    }

    #[inline]
    fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> VarVersion {
        self.0.source.version(vars)
    }

    #[inline]
    fn is_read_only<Vw: WithVars>(&self, _: &Vw) -> bool {
        true
    }

    #[inline]
    fn is_animating<Vr: WithVarsRead>(&self, vars: &Vr) -> bool {
        self.0.source.is_animating(vars)
    }

    #[inline]
    fn always_read_only(&self) -> bool {
        true
    }

    #[inline]
    fn is_contextual(&self) -> bool {
        self.0.source.is_contextual()
    }

    #[inline]
    fn can_update(&self) -> bool {
        self.0.source.can_update()
    }

    #[inline]
    fn modify<Vw, Mo>(&self, _: &Vw, _: Mo) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        Mo: FnOnce(VarModify<B>) + 'static,
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
    fn strong_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }

    #[inline]
    fn into_read_only(self) -> Self::AsReadOnly {
        self
    }

    #[inline]
    fn update_mask<Vr: WithVarsRead>(&self, vars: &Vr) -> UpdateMask {
        self.0.source.update_mask(vars)
    }

    type Weak = WeakMapRefVar<A, B, M, S>;

    #[inline]
    fn is_rc(&self) -> bool {
        true
    }

    #[inline]
    fn downgrade(&self) -> Option<Self::Weak> {
        Some(self.downgrade())
    }

    #[inline]
    fn weak_count(&self) -> usize {
        Rc::weak_count(&self.0)
    }

    #[inline]
    fn as_ptr(&self) -> *const () {
        Rc::as_ptr(&self.0) as _
    }
}

impl<A, B, M, S> IntoVar<B> for MapRefVar<A, B, M, S>
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + 'static,
    S: Var<A>,
{
    type Var = Self;

    #[inline]
    fn into_var(self) -> Self::Var {
        self
    }
}

struct MapBidiRefData<A, B, M, N, S> {
    _ab: PhantomData<(A, B)>,
    source: S,
    map: Rc<M>,
    map_mut: Rc<N>,
}

/// A weak reference to a [`MapBidiRefVar`].
pub struct WeakMapBidiRefVar<A, B, M, N, S>(Weak<MapBidiRefData<A, B, M, N, S>>);
impl<A, B, M, N, S> crate::private::Sealed for WeakMapBidiRefVar<A, B, M, N, S>
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + 'static,
    N: Fn(&mut A) -> &mut B + 'static,
    S: Var<A>,
{
}
impl<A, B, M, N, S> Clone for WeakMapBidiRefVar<A, B, M, N, S>
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + 'static,
    N: Fn(&mut A) -> &mut B + 'static,
    S: Var<A>,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<A, B, M, N, S> WeakVar<B> for WeakMapBidiRefVar<A, B, M, N, S>
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + 'static,
    N: Fn(&mut A) -> &mut B + 'static,
    S: Var<A>,
{
    type Strong = MapBidiRefVar<A, B, M, N, S>;

    fn upgrade(&self) -> Option<Self::Strong> {
        self.0.upgrade().map(MapBidiRefVar)
    }

    fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.0.weak_count()
    }

    fn as_ptr(&self) -> *const () {
        self.0.as_ptr() as _
    }
}

/// A [`Var`] that maps a mutable reference from the value of another variable.
pub struct MapBidiRefVar<A, B, M, N, S>(Rc<MapBidiRefData<A, B, M, N, S>>)
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + 'static,
    N: Fn(&mut A) -> &mut B + 'static,
    S: Var<A>;

impl<A, B, M, N, S> MapBidiRefVar<A, B, M, N, S>
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + 'static,
    N: Fn(&mut A) -> &mut B + 'static,
    S: Var<A>,
{
    /// New bidirectional reference mapping variable.
    ///
    /// Prefer using the [`Var::map_ref_bidi`] method.
    pub fn new(source: S, map: M, map_mut: N) -> Self {
        Self(Rc::new(MapBidiRefData {
            _ab: PhantomData,
            source,
            map: Rc::new(map),
            map_mut: Rc::new(map_mut),
        }))
    }

    /// Convert this variable into a [`MapRefVar`].
    #[inline]
    pub fn into_map(self) -> MapRefVar<A, B, M, S> {
        match Rc::try_unwrap(self.0) {
            Ok(data) => MapRefVar(Rc::new(MapRefData {
                _ab: PhantomData,
                source: data.source,
                map: data.map,
            })),
            Err(rc) => MapRefVar(Rc::new(MapRefData {
                _ab: PhantomData,
                source: rc.source.clone(),
                map: rc.map.clone(),
            })),
        }
    }

    /// New weak reference to the variable.
    pub fn downgrade(&self) -> WeakMapBidiRefVar<A, B, M, N, S> {
        WeakMapBidiRefVar(Rc::downgrade(&self.0))
    }
}

impl<A, B, M, N, S> Clone for MapBidiRefVar<A, B, M, N, S>
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + 'static,
    N: Fn(&mut A) -> &mut B + 'static,
    S: Var<A>,
{
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}
impl<A, B, M, N, S> crate::private::Sealed for MapBidiRefVar<A, B, M, N, S>
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + 'static,
    N: Fn(&mut A) -> &mut B + 'static,
    S: Var<A>,
{
}
impl<A, B, M, N, S> Var<B> for MapBidiRefVar<A, B, M, N, S>
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + 'static,
    N: Fn(&mut A) -> &mut B + 'static,
    S: Var<A>,
{
    type AsReadOnly = types::ReadOnlyVar<B, Self>;

    #[inline]
    fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a B {
        (self.0.map)(self.0.source.get(vars))
    }

    #[inline]
    fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a B> {
        self.0.source.get_new(vars).map(|v| (self.0.map)(v))
    }

    #[inline]
    fn into_value<Vr: WithVarsRead>(self, vars: &Vr) -> B {
        self.get_clone(vars)
    }

    #[inline]
    fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
        self.0.source.is_new(vars)
    }

    #[inline]
    fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> VarVersion {
        self.0.source.version(vars)
    }

    #[inline]
    fn is_read_only<Vw: WithVars>(&self, vars: &Vw) -> bool {
        self.0.source.is_read_only(vars)
    }

    #[inline]
    fn is_animating<Vr: WithVarsRead>(&self, vars: &Vr) -> bool {
        self.0.source.is_animating(vars)
    }

    #[inline]
    fn always_read_only(&self) -> bool {
        self.0.source.always_read_only()
    }

    #[inline]
    fn can_update(&self) -> bool {
        self.0.source.can_update()
    }

    #[inline]
    fn is_contextual(&self) -> bool {
        self.0.source.is_contextual()
    }

    #[inline]
    fn modify<Vw, Mo>(&self, vars: &Vw, modify: Mo) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        Mo: FnOnce(VarModify<B>) + 'static,
    {
        let map = self.0.map_mut.clone();
        self.0.source.modify(vars, move |mut v| {
            // rust-analyzer gets confused by `v.map_ref` here.
            VarModify::map_ref(&mut v, |v| map(v), modify);
        })
    }

    #[inline]
    fn set<Vw, Nv>(&self, vars: &Vw, new_value: Nv) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        Nv: Into<B>,
    {
        let map = self.0.map_mut.clone();
        let new_value = new_value.into();
        self.0.source.modify(vars, move |mut v| {
            *map(&mut v) = new_value;
        })
    }

    #[inline]
    fn set_ne<Vw, Nv>(&self, vars: &Vw, new_value: Nv) -> Result<bool, VarIsReadOnly>
    where
        Vw: WithVars,
        Nv: Into<B>,
        B: PartialEq,
    {
        if self.is_read_only(vars) {
            Err(VarIsReadOnly)
        } else {
            vars.with_vars(|vars| {
                let new_value = new_value.into();
                if self.get(vars) != &new_value {
                    let _ = self.set(vars, new_value);
                    Ok(true)
                } else {
                    Ok(false)
                }
            })
        }
    }

    #[inline]
    fn strong_count(&self) -> usize {
        self.0.source.strong_count()
    }

    #[inline]
    fn into_read_only(self) -> Self::AsReadOnly {
        types::ReadOnlyVar::new(self)
    }

    #[inline]
    fn update_mask<Vr: WithVarsRead>(&self, vars: &Vr) -> UpdateMask {
        self.0.source.update_mask(vars)
    }

    type Weak = WeakMapBidiRefVar<A, B, M, N, S>;

    fn is_rc(&self) -> bool {
        true
    }

    fn downgrade(&self) -> Option<Self::Weak> {
        Some(self.downgrade())
    }

    fn weak_count(&self) -> usize {
        Rc::weak_count(&self.0)
    }

    fn as_ptr(&self) -> *const () {
        Rc::as_ptr(&self.0) as _
    }
}

impl<A, B, M, N, S> IntoVar<B> for MapBidiRefVar<A, B, M, N, S>
where
    A: VarValue,
    B: VarValue,
    M: Fn(&A) -> &B + 'static,
    N: Fn(&mut A) -> &mut B + 'static,
    S: Var<A>,
{
    type Var = Self;

    #[inline]
    fn into_var(self) -> Self::Var {
        self
    }
}
