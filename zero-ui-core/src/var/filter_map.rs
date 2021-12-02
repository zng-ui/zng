use std::{
    cell::{Cell, RefCell, UnsafeCell},
    marker::PhantomData,
    mem,
    rc::Rc,
};

use super::*;

/// A [`Var`] that maps from another var and is a [`Rc`] pointer to its value. The value updates only for
/// source values approved by the mapping function.
pub struct RcFilterMapVar<A, B, I, M, S>(Rc<FilterMapData<A, B, I, M, S>>)
where
    A: VarValue,
    B: VarValue,
    I: FnOnce(&A) -> B + 'static,
    M: FnMut(&A) -> Option<B> + 'static,
    S: Var<A>;

struct FilterMapData<A, B, I, M, S> {
    _a: PhantomData<A>,

    source: S,
    map: RefCell<M>,

    value: UnsafeCell<FilterMapValue<I, B>>,
    version_checked: Cell<u32>,
    version: Cell<u32>,
    last_update_id: Cell<u32>,
}

enum FilterMapValue<I, B> {
    Uninited(I),
    Initializing,
    Value(B),
}
impl<I, B> FilterMapValue<I, B> {
    fn unwrap_init(&mut self) -> I {
        if let FilterMapValue::Uninited(i) = mem::replace(self, FilterMapValue::Initializing) {
            i
        } else {
            panic!("value initialized")
        }
    }

    fn unwrap(&self) -> &B {
        if let FilterMapValue::Value(v) = self {
            v
        } else {
            panic!("value uninitialized")
        }
    }
}
impl<A, B, M, I, S> RcFilterMapVar<A, B, I, M, S>
where
    A: VarValue,
    B: VarValue,
    I: FnOnce(&A) -> B + 'static,
    M: FnMut(&A) -> Option<B> + 'static,
    S: Var<A>,
{
    /// New filter mapping var.
    ///
    /// Only use this directly if you are implementing [`Var`]. For existing variables use
    /// the [`Var::filter_map`] method.
    pub fn new(source: S, fallback_init: I, map: M) -> Self {
        RcFilterMapVar(Rc::new(FilterMapData {
            _a: PhantomData,
            source,
            map: RefCell::new(map),
            value: UnsafeCell::new(FilterMapValue::Uninited(fallback_init)),
            version_checked: Cell::new(0),
            version: Cell::new(0),
            last_update_id: Cell::new(0),
        }))
    }

    /// Get the value, applies the mapping if the value is out of sync.
    pub fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a B {
        let vars = vars.as_ref();

        // SAFETY: access to value is safe because `source` needs a `&mut Vars` to change its version
        // and we change the value only in the first call to `get` with the new source version.

        let source_version = self.0.source.version(vars);

        if self.0.version.get() == 0 {
            let source_value = self.0.source.get(vars);
            let new_value = self.0.map.borrow_mut()(source_value).unwrap_or_else(|| {
                let init = unsafe { &mut *self.0.value.get() }.unwrap_init();
                init(source_value)
            });

            unsafe {
                *self.0.value.get() = FilterMapValue::Value(new_value);
            }

            self.0.version.set(1);
            self.0.version_checked.set(source_version);
            self.0.last_update_id.set(vars.update_id());
        } else if source_version != self.0.version_checked.get() {
            if let Some(new_value) = self.0.map.borrow_mut()(self.0.source.get(vars)) {
                unsafe {
                    *self.0.value.get() = FilterMapValue::Value(new_value);
                }
                self.0.version.set(self.0.version.get().wrapping_add(1));
                self.0.last_update_id.set(vars.update_id());
            }
            self.0.version_checked.set(source_version);
        }
        unsafe { &*self.0.value.get() }.unwrap()
    }

    /// Gets the value if [`is_new`](Self::is_new).
    pub fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a B> {
        let vars = vars.as_ref();

        if self.0.source.is_new(vars) {
            let value = self.get(vars);
            if self.0.last_update_id.get() == vars.update_id() {
                Some(value)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Gets if the value updated in the last update.
    ///
    /// Returns `true` if the source var is new and the new value was approved by the filter.
    #[inline]
    pub fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
        vars.with_vars(|vars| self.get_new(vars).is_some())
    }

    /// Gets the up-to-date value version.
    #[inline]
    pub fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> u32 {
        vars.with_vars_read(|vars| {
            let _ = self.get(vars);
            self.0.source.version(vars)
        })
    }

    /// Gets the number of [`RcFilterMapVar`] that point to this same variable.
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
impl<A, B, M, I, S> Clone for RcFilterMapVar<A, B, I, M, S>
where
    A: VarValue,
    B: VarValue,
    I: FnOnce(&A) -> B + 'static,
    M: FnMut(&A) -> Option<B> + 'static,
    S: Var<A>,
{
    fn clone(&self) -> Self {
        RcFilterMapVar(Rc::clone(&self.0))
    }
}
impl<A, B, I, M, S> crate::private::Sealed for RcFilterMapVar<A, B, I, M, S>
where
    A: VarValue,
    B: VarValue,
    I: FnOnce(&A) -> B + 'static,
    M: FnMut(&A) -> Option<B> + 'static,
    S: Var<A>,
{
}
impl<A, B, I, M, S> Var<B> for RcFilterMapVar<A, B, I, M, S>
where
    A: VarValue,
    B: VarValue,
    I: FnOnce(&A) -> B + 'static,
    M: FnMut(&A) -> Option<B> + 'static,
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
    fn is_read_only<Vr: WithVars>(&self, _: &Vr) -> bool {
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
    fn strong_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }

    #[inline]
    fn into_read_only(self) -> Self::AsReadOnly {
        self
    }

    #[inline]
    fn update_mask(&self) -> UpdateMask {
        self.0.source.update_mask()
    }
}
impl<A, B, I, M, S> IntoVar<B> for RcFilterMapVar<A, B, I, M, S>
where
    A: VarValue,
    B: VarValue,
    I: FnOnce(&A) -> B + 'static,
    M: FnMut(&A) -> Option<B> + 'static,
    S: Var<A>,
{
    type Var = Self;

    #[inline]
    fn into_var(self) -> Self::Var {
        self
    }
}

/// A [`Var`] that maps from another var and is a [`Rc`] pointer to its value. The value updates only for
/// source values approved by the mapping function.
pub struct RcFilterMapBidiVar<A, B, I, M, N, S>(Rc<FilterMapBidiData<A, B, I, M, N, S>>)
where
    A: VarValue,
    B: VarValue,
    I: FnOnce(&A) -> B + 'static,
    M: FnMut(&A) -> Option<B> + 'static,
    N: FnMut(B) -> Option<A> + 'static,
    S: Var<A>;

struct FilterMapBidiData<A, B, I, M, N, S> {
    _a: PhantomData<A>,

    source: S,
    map: RefCell<M>,
    map_back: RefCell<N>,

    value: UnsafeCell<FilterMapValue<I, B>>,
    version_checked: Cell<u32>,
    version: Cell<u32>,
    last_update_id: Cell<u32>,
}
impl<A, B, I, M, N, S> RcFilterMapBidiVar<A, B, I, M, N, S>
where
    A: VarValue,
    B: VarValue,
    I: FnOnce(&A) -> B + 'static,
    M: FnMut(&A) -> Option<B> + 'static,
    N: FnMut(B) -> Option<A> + 'static,
    S: Var<A>,
{
    /// New bidirectional filtered mapping var.
    ///
    /// Only use this directly if you are implementing [`Var`]. For existing variables use
    /// the [`Var::filter_map_bidi`] method.
    pub fn new(source: S, fallback_init: I, map: M, map_back: N) -> Self {
        RcFilterMapBidiVar(Rc::new(FilterMapBidiData {
            _a: PhantomData,
            source,
            map: RefCell::new(map),
            map_back: RefCell::new(map_back),

            value: UnsafeCell::new(FilterMapValue::Uninited(fallback_init)),
            version_checked: Cell::new(0),
            version: Cell::new(0),
            last_update_id: Cell::new(0),
        }))
    }

    /// Get the value, applies the mapping if the value is out of sync.
    pub fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a B {
        let vars = vars.as_ref();
        // SAFETY: access to value is safe because `source` needs a `&mut Vars` to change its version
        // and we change the value only in the first call to `get` with the new source version.

        let source_version = self.0.source.version(vars);

        if self.0.version.get() == 0 {
            let source_value = self.0.source.get(vars);
            let new_value = self.0.map.borrow_mut()(source_value).unwrap_or_else(|| {
                let init = unsafe { &mut *self.0.value.get() }.unwrap_init();
                init(source_value)
            });

            unsafe {
                *self.0.value.get() = FilterMapValue::Value(new_value);
            }

            self.0.version.set(1);
            self.0.version_checked.set(source_version);
            self.0.last_update_id.set(vars.update_id());
        } else if source_version != self.0.version_checked.get() {
            if let Some(new_value) = self.0.map.borrow_mut()(self.0.source.get(vars)) {
                unsafe {
                    *self.0.value.get() = FilterMapValue::Value(new_value);
                }
                self.0.version.set(self.0.version.get().wrapping_add(1));
                self.0.last_update_id.set(vars.update_id());
            }
            self.0.version_checked.set(source_version);
        }
        unsafe { &*self.0.value.get() }.unwrap()
    }

    /// Gets the value if [`is_new`](Self::is_new).
    pub fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a B> {
        let vars = vars.as_ref();

        if self.0.source.is_new(vars) {
            let value = self.get(vars);
            if self.0.last_update_id.get() == vars.update_id() {
                Some(value)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Gets if the value updated in the last update.
    ///
    /// Returns `true` if the source var is new and the new value was approved by the filter.
    #[inline]
    pub fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
        vars.with_vars(|vars| self.get_new(vars).is_some())
    }

    /// Gets the up-to-date value version.
    #[inline]
    pub fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> u32 {
        vars.with_vars_read(|vars| {
            let _ = self.get(vars);
            self.0.source.version(vars)
        })
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
            if let Some(mut mapped_value) = self_.0.map.borrow_mut()(source_value) {
                let mut guard = VarModify::new(&mut mapped_value);
                modify(&mut guard);
                if guard.touched() {
                    if let Some(new_value) = self_.0.map_back.borrow_mut()(mapped_value) {
                        **source_value = new_value;
                    }
                }
            }
        })
    }

    /// Map back the value and schedules a `set` in the source variable if the map-back function returned a value.
    ///
    /// Returns `Err(VarIsReadOnly)` if the source variable is currently read-only. Returns `Ok(bool)` where the `bool`
    /// indicates if the map-back function produced some value.
    fn set<Vw, Nv>(&self, vars: &Vw, new_value: Nv) -> Result<bool, VarIsReadOnly>
    where
        Vw: WithVars,
        Nv: Into<B>,
    {
        if self.0.source.is_read_only(vars) {
            Err(VarIsReadOnly)
        } else if let Some(new_value) = self.0.map_back.borrow_mut()(new_value.into()) {
            self.0.source.set(vars, new_value).map(|_| true)
        } else {
            Ok(false)
        }
    }

    /// If the current value is not equal to `new_value` maps back the value and schedules a `set` in the source variable
    /// if the map-back function returned a value.
    ///
    /// Returns `Err(VarIsReadOnly)` if the source variable is currently read-only. Returns `Ok(bool)` where the `bool`
    /// indicates if the source variable will update.
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
                    if let Some(new_value) = self.0.map_back.borrow_mut()(new_value) {
                        let _ = self.0.source.set(vars, new_value);
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                } else {
                    Ok(false)
                }
            })
        }
    }

    /// Convert to a [`RcFilterMapVar`] if `self` is the only reference.
    #[inline]
    pub fn into_filter_map(self) -> Result<RcFilterMapVar<A, B, I, M, S>, Self> {
        match Rc::try_unwrap(self.0) {
            Ok(data) => Ok(RcFilterMapVar(Rc::new(FilterMapData {
                _a: PhantomData,
                source: data.source,
                map: data.map,
                value: data.value,
                version_checked: data.version_checked,
                version: data.version,
                last_update_id: data.last_update_id,
            }))),
            Err(rc) => Err(Self(rc)),
        }
    }

    /// Gets the number of [`RcFilterMapVar`] that point to this same variable.
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
impl<A, B, I, M, N, S> Clone for RcFilterMapBidiVar<A, B, I, M, N, S>
where
    A: VarValue,
    B: VarValue,
    I: FnOnce(&A) -> B + 'static,
    M: FnMut(&A) -> Option<B> + 'static,
    N: FnMut(B) -> Option<A> + 'static,
    S: Var<A>,
{
    fn clone(&self) -> Self {
        RcFilterMapBidiVar(Rc::clone(&self.0))
    }
}
impl<A, B, I, M, N, S> crate::private::Sealed for RcFilterMapBidiVar<A, B, I, M, N, S>
where
    A: VarValue,
    B: VarValue,
    I: FnOnce(&A) -> B + 'static,
    M: FnMut(&A) -> Option<B> + 'static,
    N: FnMut(B) -> Option<A> + 'static,
    S: Var<A>,
{
}
impl<A, B, I, M, N, S> Var<B> for RcFilterMapBidiVar<A, B, I, M, N, S>
where
    A: VarValue,
    B: VarValue,
    I: FnOnce(&A) -> B + 'static,
    M: FnMut(&A) -> Option<B> + 'static,
    N: FnMut(B) -> Option<A> + 'static,
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
        self.set(vars, new_value).map(|_| ())
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
    fn strong_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }

    #[inline]
    fn into_read_only(self) -> Self::AsReadOnly {
        ReadOnlyVar::new(self)
    }

    fn update_mask(&self) -> UpdateMask {
        self.0.source.update_mask()
    }
}
impl<A, B, I, M, N, S> IntoVar<B> for RcFilterMapBidiVar<A, B, I, M, N, S>
where
    A: VarValue,
    B: VarValue,
    I: FnOnce(&A) -> B + 'static,
    M: FnMut(&A) -> Option<B> + 'static,
    N: FnMut(B) -> Option<A> + 'static,
    S: Var<A>,
{
    type Var = Self;

    #[inline]
    fn into_var(self) -> Self::Var {
        self
    }
}
