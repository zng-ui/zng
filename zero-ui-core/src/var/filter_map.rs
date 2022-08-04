use std::{
    cell::{Cell, RefCell, UnsafeCell},
    marker::PhantomData,
    mem,
    rc::{Rc, Weak},
};

use super::*;

/// A weak reference to a [`RcFilterMapVar`].
pub struct WeakRcFilterMapVar<A, B, I, M, S>(Weak<FilterMapData<A, B, I, M, S>>);
impl<A, B, M, I, S> crate::private::Sealed for WeakRcFilterMapVar<A, B, I, M, S>
where
    A: VarValue,
    B: VarValue,
    I: FnOnce(&A) -> B + 'static,
    M: FnMut(&A) -> Option<B> + 'static,
    S: Var<A>,
{
}
impl<A, B, M, I, S> Clone for WeakRcFilterMapVar<A, B, I, M, S>
where
    A: VarValue,
    B: VarValue,
    I: FnOnce(&A) -> B + 'static,
    M: FnMut(&A) -> Option<B> + 'static,
    S: Var<A>,
{
    fn clone(&self) -> Self {
        WeakRcFilterMapVar(self.0.clone())
    }
}
impl<A, B, I, M, S> any::AnyWeakVar for WeakRcFilterMapVar<A, B, I, M, S>
where
    A: VarValue,
    B: VarValue,
    I: FnOnce(&A) -> B + 'static,
    M: FnMut(&A) -> Option<B> + 'static,
    S: Var<A>,
{
    any_var_impls!(WeakVar);
}
impl<A, B, M, I, S> WeakVar<B> for WeakRcFilterMapVar<A, B, I, M, S>
where
    A: VarValue,
    B: VarValue,
    I: FnOnce(&A) -> B + 'static,
    M: FnMut(&A) -> Option<B> + 'static,
    S: Var<A>,
{
    type Strong = RcFilterMapVar<A, B, I, M, S>;

    fn upgrade(&self) -> Option<Self::Strong> {
        self.0.upgrade().map(RcFilterMapVar)
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
    map: Rc<RefCell<M>>,

    value: UnsafeCell<FilterMapValue<I, B>>,
    version_checked: VarVersionCell,
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
    /// Prefer using the [`Var::filter_map`] method.
    pub fn new(source: S, fallback_init: I, map: M) -> Self {
        RcFilterMapVar(Rc::new(FilterMapData {
            _a: PhantomData,
            source,
            map: Rc::new(RefCell::new(map)),
            value: UnsafeCell::new(FilterMapValue::Uninited(fallback_init)),
            version_checked: VarVersionCell::new(0),
            version: Cell::new(0),
            last_update_id: Cell::new(0),
        }))
    }

    /// Create a weak reference to the variable.
    pub fn downgrade(&self) -> WeakRcFilterMapVar<A, B, I, M, S> {
        WeakRcFilterMapVar(Rc::downgrade(&self.0))
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

    fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a B {
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

    fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a B> {
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

    fn into_value<Vr: WithVarsRead>(self, vars: &Vr) -> B {
        self.get_clone(vars)
    }

    fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
        vars.with_vars(|vars| self.get_new(vars).is_some())
    }

    fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> VarVersion {
        vars.with_vars_read(|vars| {
            let _ = self.get(vars);
            VarVersion::normal(self.0.version.get())
        })
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
        if self.is_contextual() {
            vars.with_vars(|vars| {
                let value = self.get_clone(vars);
                let var = RcFilterMapVar::<_, _, I, _, _>(Rc::new(FilterMapData {
                    _a: PhantomData,
                    source: self.0.source.actual_var(vars),
                    map: self.0.map.clone(),
                    value: UnsafeCell::new(FilterMapValue::Value(value)),
                    version_checked: self.0.version_checked.clone(),
                    version: self.0.version.clone(),
                    last_update_id: self.0.last_update_id.clone(),
                }));
                var.boxed()
            })
        } else {
            self.clone().boxed()
        }
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

    fn strong_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }

    fn into_read_only(self) -> Self::AsReadOnly {
        self
    }

    fn update_mask<Vr: WithVarsRead>(&self, vars: &Vr) -> UpdateMask {
        self.0.source.update_mask(vars)
    }

    type Weak = WeakRcFilterMapVar<A, B, I, M, S>;

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
impl<A, B, I, M, S> IntoVar<B> for RcFilterMapVar<A, B, I, M, S>
where
    A: VarValue,
    B: VarValue,
    I: FnOnce(&A) -> B + 'static,
    M: FnMut(&A) -> Option<B> + 'static,
    S: Var<A>,
{
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}
impl<A, B, I, M, S> any::AnyVar for RcFilterMapVar<A, B, I, M, S>
where
    A: VarValue,
    B: VarValue,
    I: FnOnce(&A) -> B + 'static,
    M: FnMut(&A) -> Option<B> + 'static,
    S: Var<A>,
{
    any_var_impls!(Var);
}

/// A weak reference to a [`RcFilterMapBidiVar`].
pub struct WeakRcFilterMapBidiVar<A, B, I, M, N, S>(Weak<FilterMapBidiData<A, B, I, M, N, S>>);
impl<A, B, I, M, N, S> crate::private::Sealed for WeakRcFilterMapBidiVar<A, B, I, M, N, S>
where
    A: VarValue,
    B: VarValue,
    I: FnOnce(&A) -> B + 'static,
    M: FnMut(&A) -> Option<B> + 'static,
    N: FnMut(B) -> Option<A> + 'static,
    S: Var<A>,
{
}

impl<A, B, I, M, N, S> Clone for WeakRcFilterMapBidiVar<A, B, I, M, N, S>
where
    A: VarValue,
    B: VarValue,
    I: FnOnce(&A) -> B + 'static,
    M: FnMut(&A) -> Option<B> + 'static,
    N: FnMut(B) -> Option<A> + 'static,
    S: Var<A>,
{
    fn clone(&self) -> Self {
        WeakRcFilterMapBidiVar(self.0.clone())
    }
}
impl<A, B, I, M, N, S> any::AnyWeakVar for WeakRcFilterMapBidiVar<A, B, I, M, N, S>
where
    A: VarValue,
    B: VarValue,
    I: FnOnce(&A) -> B + 'static,
    M: FnMut(&A) -> Option<B> + 'static,
    N: FnMut(B) -> Option<A> + 'static,
    S: Var<A>,
{
    any_var_impls!(WeakVar);
}

impl<A, B, I, M, N, S> WeakVar<B> for WeakRcFilterMapBidiVar<A, B, I, M, N, S>
where
    A: VarValue,
    B: VarValue,
    I: FnOnce(&A) -> B + 'static,
    M: FnMut(&A) -> Option<B> + 'static,
    N: FnMut(B) -> Option<A> + 'static,
    S: Var<A>,
{
    type Strong = RcFilterMapBidiVar<A, B, I, M, N, S>;

    fn upgrade(&self) -> Option<Self::Strong> {
        self.0.upgrade().map(RcFilterMapBidiVar)
    }

    fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.0.weak_count()
    }

    fn as_ptr(&self) -> *const () {
        self.0.as_ptr() as *const ()
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
    map: Rc<RefCell<M>>,
    map_back: Rc<RefCell<N>>,

    value: UnsafeCell<FilterMapValue<I, B>>,
    version_checked: VarVersionCell,
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
    /// Prefer using the [`Var::filter_map_bidi`] method.
    pub fn new(source: S, fallback_init: I, map: M, map_back: N) -> Self {
        RcFilterMapBidiVar(Rc::new(FilterMapBidiData {
            _a: PhantomData,
            source,
            map: Rc::new(RefCell::new(map)),
            map_back: Rc::new(RefCell::new(map_back)),

            value: UnsafeCell::new(FilterMapValue::Uninited(fallback_init)),
            version_checked: VarVersionCell::new(0),
            version: Cell::new(0),
            last_update_id: Cell::new(0),
        }))
    }

    /// Convert to a [`RcFilterMapVar`], a deep clone is made if `self` is not the only reference.
    pub fn into_filter_map<Vr: WithVarsRead>(self, vars: &Vr) -> RcFilterMapVar<A, B, I, M, S> {
        match Rc::try_unwrap(self.0) {
            Ok(data) => RcFilterMapVar(Rc::new(FilterMapData {
                _a: PhantomData,
                source: data.source,
                map: data.map,
                value: data.value,
                version_checked: data.version_checked,
                version: data.version,
                last_update_id: data.last_update_id,
            })),
            Err(rc) => vars.with_vars_read(|vars| {
                let self_ = Self(rc);
                let value = self_.get_clone(vars);
                RcFilterMapVar(Rc::new(FilterMapData {
                    _a: PhantomData,
                    source: self_.0.source.clone(),
                    map: self_.0.map.clone(),
                    value: UnsafeCell::new(FilterMapValue::Value(value)),
                    version_checked: self_.0.version_checked.clone(),
                    version: self_.0.version.clone(),
                    last_update_id: self_.0.last_update_id.clone(),
                }))
            }),
        }
    }

    /// Create a weak reference to the variable.
    pub fn downgrade(&self) -> WeakRcFilterMapBidiVar<A, B, I, M, N, S> {
        WeakRcFilterMapBidiVar(Rc::downgrade(&self.0))
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
    type AsReadOnly = types::ReadOnlyVar<B, Self>;

    fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a B {
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

    fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a B> {
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

    fn into_value<Vr: WithVarsRead>(self, vars: &Vr) -> B {
        self.get_clone(vars)
    }

    fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
        vars.with_vars(|vars| self.get_new(vars).is_some())
    }

    fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> VarVersion {
        vars.with_vars_read(|vars| {
            let _ = self.get(vars);
            VarVersion::normal(self.0.version.get())
        })
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

    fn is_contextual(&self) -> bool {
        self.0.source.is_contextual()
    }

    fn actual_var<Vw: WithVars>(&self, vars: &Vw) -> BoxedVar<B> {
        if self.is_contextual() {
            vars.with_vars(|vars| {
                let value = self.get_clone(vars);
                let var = RcFilterMapBidiVar::<_, _, I, _, _, _>(Rc::new(FilterMapBidiData {
                    _a: PhantomData,
                    source: self.0.source.actual_var(vars),
                    map: self.0.map.clone(),
                    map_back: self.0.map_back.clone(),
                    value: UnsafeCell::new(FilterMapValue::Value(value)),
                    version_checked: self.0.version_checked.clone(),
                    version: self.0.version.clone(),
                    last_update_id: self.0.last_update_id.clone(),
                }));
                var.boxed()
            })
        } else {
            self.clone().boxed()
        }
    }

    fn can_update(&self) -> bool {
        self.0.source.can_update()
    }

    fn modify<Vw, Mo>(&self, vars: &Vw, modify: Mo) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        Mo: FnOnce(VarModify<B>) + 'static,
    {
        let self_ = self.clone();
        self.0.source.modify(vars, move |mut source_value| {
            if let Some(mut mapped_value) = self_.0.map.borrow_mut()(&source_value) {
                let mut touched = false;
                modify(VarModify::new(&mut mapped_value, &mut touched));
                if touched {
                    if let Some(new_value) = self_.0.map_back.borrow_mut()(mapped_value) {
                        *source_value = new_value;
                    }
                }
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
        } else if let Some(new_value) = self.0.map_back.borrow_mut()(new_value.into()) {
            self.0.source.set(vars, new_value)
        } else {
            Ok(())
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

    fn strong_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }

    fn into_read_only(self) -> Self::AsReadOnly {
        types::ReadOnlyVar::new(self)
    }

    fn update_mask<Vr: WithVarsRead>(&self, vars: &Vr) -> UpdateMask {
        self.0.source.update_mask(vars)
    }

    type Weak = WeakRcFilterMapBidiVar<A, B, I, M, N, S>;

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

    fn into_var(self) -> Self::Var {
        self
    }
}
impl<A, B, I, M, N, S> any::AnyVar for RcFilterMapBidiVar<A, B, I, M, N, S>
where
    A: VarValue,
    B: VarValue,
    I: FnOnce(&A) -> B + 'static,
    M: FnMut(&A) -> Option<B> + 'static,
    N: FnMut(B) -> Option<A> + 'static,
    S: Var<A>,
{
    any_var_impls!(Var);
}
