use std::{
    cell::{RefCell, UnsafeCell},
    marker::PhantomData,
    rc::{Rc, Weak},
};

use super::*;

/// A weak reference to a [`RcMapVar`].
pub struct WeakRcMapVar<A, B, M, S>(Weak<MapData<A, B, M, S>>);
impl<A, B, M, S> crate::private::Sealed for WeakRcMapVar<A, B, M, S>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    S: Var<A>,
{
}
impl<A, B, M, S> Clone for WeakRcMapVar<A, B, M, S>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    S: Var<A>,
{
    fn clone(&self) -> Self {
        WeakRcMapVar(self.0.clone())
    }
}
impl<A, B, M, S> WeakVar<B> for WeakRcMapVar<A, B, M, S>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    S: Var<A>,
{
    type Strong = RcMapVar<A, B, M, S>;

    fn upgrade(&self) -> Option<Self::Strong> {
        self.0.upgrade().map(RcMapVar)
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
    map: Rc<RefCell<M>>,

    value: UnsafeCell<Option<B>>,
    version: VarVersionCell,
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

    pub fn new(source: S, map: M) -> Self {
        RcMapVar(Rc::new(MapData {
            _a: PhantomData,
            source,
            map: Rc::new(RefCell::new(map)),
            value: UnsafeCell::new(None),
            version: VarVersionCell::new(0),
        }))
    }

    /// New weak reference to this variable.
    pub fn downgrade(&self) -> WeakRcMapVar<A, B, M, S> {
        WeakRcMapVar(Rc::downgrade(&self.0))
    }

    fn get_impl(&self, vars: &VarsRead) -> &B {
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

    fn actual_var_impl(&self, vars: &Vars) -> BoxedVar<B> {
        if self.is_contextual() {
            let value = self.get_clone(vars);
            let var = RcMapVar(Rc::new(MapData {
                _a: PhantomData,
                source: self.0.source.actual_var(vars),
                map: self.0.map.clone(),
                value: UnsafeCell::new(Some(value)),
                version: self.0.version.clone(),
            }));
            var.boxed()
        } else {
            self.clone().boxed()
        }
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

    fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a B {
        self.get_impl(vars.as_ref())
    }

    fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a B> {
        let vars = vars.as_ref();

        if self.0.source.is_new(vars) {
            Some(self.get(vars))
        } else {
            None
        }
    }

    fn into_value<Vr: WithVarsRead>(self, vars: &Vr) -> B {
        self.get_clone(vars)
    }

    fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
        self.0.source.is_new(vars)
    }

    fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> VarVersion {
        self.0.source.version(vars)
    }

    fn is_read_only<Vw: WithVars>(&self, _: &Vw) -> bool {
        true
    }

    fn is_animating<Vr: WithVarsRead>(&self, vars: &Vr) -> bool {
        self.0.source.is_animating(vars)
    }

    fn always_read_only(&self) -> bool {
        true
    }

    fn can_update(&self) -> bool {
        self.0.source.can_update()
    }

    fn is_contextual(&self) -> bool {
        self.0.source.is_contextual()
    }

    fn actual_var<Vw: WithVars>(&self, vars: &Vw) -> BoxedVar<B> {
        vars.with_vars(|vars| self.actual_var_impl(vars))
    }

    fn strong_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }

    fn modify<Vw, Mo>(&self, _: &Vw, _: Mo) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        Mo: FnOnce(VarModify<B>) + 'static,
    {
        Err(VarIsReadOnly)
    }

    fn set<Vw, N>(&self, _: &Vw, _: N) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<B>,
    {
        Err(VarIsReadOnly)
    }

    fn set_ne<Vw, N>(&self, _: &Vw, _: N) -> Result<bool, VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<B>,
        B: PartialEq,
    {
        Err(VarIsReadOnly)
    }

    fn into_read_only(self) -> Self::AsReadOnly {
        self
    }

    fn update_mask<Vr: WithVarsRead>(&self, vars: &Vr) -> UpdateMask {
        self.0.source.update_mask(vars)
    }

    type Weak = WeakRcMapVar<A, B, M, S>;

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
impl<A, B, M, S> any::AnyVar for RcMapVar<A, B, M, S>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    S: Var<A>,
{
    fn into_any(self) -> Box<dyn any::AnyVar> {
        Box::new(self)
    }

    any_var_impls!();
}

/// Weak reference to a [`RcMapBidiVar`].
pub struct WeakRcMapBidiVar<A, B, M, N, S>(Weak<MapBidiData<A, B, M, N, S>>);
impl<A, B, M, N, S> crate::private::Sealed for WeakRcMapBidiVar<A, B, M, N, S>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    N: FnMut(B) -> A + 'static,
    S: Var<A>,
{
}
impl<A, B, M, N, S> Clone for WeakRcMapBidiVar<A, B, M, N, S>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    N: FnMut(B) -> A + 'static,
    S: Var<A>,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<A, B, M, N, S> WeakVar<B> for WeakRcMapBidiVar<A, B, M, N, S>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    N: FnMut(B) -> A + 'static,
    S: Var<A>,
{
    type Strong = RcMapBidiVar<A, B, M, N, S>;

    fn upgrade(&self) -> Option<Self::Strong> {
        self.0.upgrade().map(RcMapBidiVar)
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
    map: Rc<RefCell<M>>,
    map_back: Rc<RefCell<N>>,

    value: UnsafeCell<Option<B>>,
    version: VarVersionCell,
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

    pub fn new(source: S, map: M, map_back: N) -> Self {
        RcMapBidiVar(Rc::new(MapBidiData {
            _a: PhantomData,
            source,
            map: Rc::new(RefCell::new(map)),
            map_back: Rc::new(RefCell::new(map_back)),
            value: UnsafeCell::new(None),
            version: VarVersionCell::new(0),
        }))
    }

    /// New weak reference to the variable.
    pub fn downgrade(&self) -> WeakRcMapBidiVar<A, B, M, N, S> {
        WeakRcMapBidiVar(Rc::downgrade(&self.0))
    }

    /// Convert to a [`RcMapVar`], a deep clone is made if `self` is the only reference.

    pub fn into_map<Vr: WithVarsRead>(self, vars: &Vr) -> RcMapVar<A, B, M, S> {
        match Rc::try_unwrap(self.0) {
            Ok(data) => RcMapVar(Rc::new(MapData {
                _a: PhantomData,
                source: data.source,
                map: data.map,
                value: data.value,
                version: data.version,
            })),
            Err(rc) => vars.with_vars_read(|vars| {
                let self_ = Self(rc);
                let value = self_.get_clone(vars);
                RcMapVar(Rc::new(MapData {
                    _a: PhantomData,
                    source: self_.0.source.clone(),
                    map: self_.0.map.clone(),
                    value: UnsafeCell::new(Some(value)),
                    version: self_.0.version.clone(),
                }))
            }),
        }
    }

    /// Gets the number of [`RcMapBidiVar`] that point to this same variable.

    pub fn strong_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }

    /// Returns `true` if `self` and `other` are the same variable.

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
    type AsReadOnly = types::ReadOnlyVar<B, Self>;

    fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a B {
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

    fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a B> {
        let vars = vars.as_ref();

        if self.0.source.is_new(vars) {
            Some(self.get(vars))
        } else {
            None
        }
    }

    fn into_value<Vr: WithVarsRead>(self, vars: &Vr) -> B {
        self.get_clone(vars)
    }

    fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
        self.0.source.is_new(vars)
    }

    fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> VarVersion {
        self.0.source.version(vars)
    }

    fn is_read_only<Vw: WithVars>(&self, vars: &Vw) -> bool {
        self.0.source.is_read_only(vars)
    }

    fn is_animating<Vr: WithVarsRead>(&self, vars: &Vr) -> bool {
        self.0.source.is_animating(vars)
    }

    fn always_read_only(&self) -> bool {
        self.0.source.always_read_only()
    }

    fn can_update(&self) -> bool {
        self.0.source.can_update()
    }

    fn is_contextual(&self) -> bool {
        self.0.source.is_contextual()
    }

    fn actual_var<Vw: WithVars>(&self, vars: &Vw) -> BoxedVar<B> {
        if self.is_contextual() {
            vars.with_vars(|vars| {
                let value = self.get_clone(vars);
                let var = RcMapBidiVar(Rc::new(MapBidiData {
                    _a: PhantomData,
                    source: self.0.source.actual_var(vars),
                    map: self.0.map.clone(),
                    map_back: self.0.map_back.clone(),
                    value: UnsafeCell::new(Some(value)),
                    version: self.0.version.clone(),
                }));
                var.boxed()
            })
        } else {
            self.clone().boxed()
        }
    }

    fn strong_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }

    fn modify<Vw, Mo>(&self, vars: &Vw, modify: Mo) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        Mo: FnOnce(VarModify<B>) + 'static,
    {
        let self_ = self.clone();
        self.0.source.modify(vars, move |mut source_value| {
            let mut mapped_value = self_.0.map.borrow_mut()(&source_value);
            let mut touched = false;
            modify(VarModify::new(&mut mapped_value, &mut touched));
            if touched {
                *source_value = self_.0.map_back.borrow_mut()(mapped_value);
            }
        })
    }

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

    fn into_read_only(self) -> Self::AsReadOnly {
        types::ReadOnlyVar::new(self)
    }

    fn update_mask<Vr: WithVarsRead>(&self, vars: &Vr) -> UpdateMask {
        self.0.source.update_mask(vars)
    }

    type Weak = WeakRcMapBidiVar<A, B, M, N, S>;

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
impl<A, B, M, N, S> IntoVar<B> for RcMapBidiVar<A, B, M, N, S>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    N: FnMut(B) -> A + 'static,
    S: Var<A>,
{
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}
impl<A, B, M, N, S> any::AnyVar for RcMapBidiVar<A, B, M, N, S>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    N: FnMut(B) -> A + 'static,
    S: Var<A>,
{
    fn into_any(self) -> Box<dyn any::AnyVar> {
        Box::new(self)
    }

    any_var_impls!();
}
