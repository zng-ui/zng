use super::{
    protected, CloningLocalVar, IntoVar, MapVar, MapVarBiDi, MapVarBiDiInner, MapVarInner, ObjVar, ReadOnlyVar, Var, VarIsReadOnly,
    VarValue,
};
use crate::core::context::{Updates, Vars};
use std::{
    cell::{Cell, RefCell, UnsafeCell},
    marker::PhantomData,
    mem::MaybeUninit,
    rc::Rc,
};

struct SharedVarInner<T> {
    data: UnsafeCell<T>,
    is_new: Cell<bool>,
    version: Cell<u32>,
}

/// A reference-counting [`Var`](Var).
pub struct SharedVar<T: VarValue> {
    r: Rc<SharedVarInner<T>>,
}

impl<T: VarValue> SharedVar<T> {
    pub fn new(initial_value: T) -> Self {
        SharedVar {
            r: Rc::new(SharedVarInner {
                data: UnsafeCell::new(initial_value),
                is_new: Cell::new(false),
                version: Cell::new(0),
            }),
        }
    }

    pub(crate) fn modify(
        self,
        modify: impl FnOnce(&mut T) + 'static,
        _assert_vars_not_borrowed: &mut Vars,
        cleanup: &mut Vec<Box<dyn FnOnce()>>,
    ) {
        // SAFETY: This is safe because borrows are bound to the `Vars` instance
        // so if we have a mutable reference to it no event value is borrowed.
        modify(unsafe { &mut *self.r.data.get() });
        self.r.is_new.set(true);
        self.r.version.set(self.next_version());
        cleanup.push(Box::new(move || self.r.is_new.set(false)));
    }

    fn borrow<'a>(&'a self, _assert: &'a Vars) -> &'a T {
        // SAFETY: This is safe because we are bounding the value lifetime with
        // the `Vars` lifetime and we require a mutable reference to `Vars` to
        // modify the value.
        unsafe { &*self.r.data.get() }
    }

    /// Gets the [version](ObjVar::version) this variable will be in the next update if set in this update.
    pub fn next_version(&self) -> u32 {
        self.r.version.get().wrapping_add(1)
    }
}

impl<T: VarValue> Clone for SharedVar<T> {
    fn clone(&self) -> Self {
        SharedVar { r: Rc::clone(&self.r) }
    }
}

impl<T: VarValue> protected::Var<T> for SharedVar<T> {
    fn bind_info<'a>(&'a self, vars: &'a Vars) -> protected::BindInfo<'a, T> {
        protected::BindInfo::Var(self.borrow(vars), self.r.is_new.get(), self.r.version.get())
    }

    fn read_only_prev_version(&self) -> u32 {
        self.r.version.get().wrapping_sub(1)
    }
}

impl<T: VarValue> ObjVar<T> for SharedVar<T> {
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a T {
        self.borrow(vars)
    }

    fn update<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        if self.r.is_new.get() {
            Some(self.borrow(vars))
        } else {
            None
        }
    }

    fn is_new(&self, _: &Vars) -> bool {
        self.r.is_new.get()
    }

    fn version(&self, _: &Vars) -> u32 {
        self.r.version.get()
    }

    fn read_only(&self, _: &Vars) -> bool {
        false
    }

    fn always_read_only(&self, _: &Vars) -> bool {
        false
    }

    fn push_set(&self, new_value: T, _: &Vars, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
        let var = self.clone();
        updates.push_modify_impl(move |assert, cleanup| {
            var.modify(move |v: &mut T| *v = new_value, assert, cleanup);
        });
        Ok(())
    }

    fn push_modify_boxed(&self, modify: Box<dyn FnOnce(&mut T) + 'static>, _: &Vars, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
        let var = self.clone();
        updates.push_modify_impl(move |assert, cleanup| {
            var.modify(|v: &mut T| modify(v), assert, cleanup);
        });
        Ok(())
    }
}

impl<T: VarValue> Var<T> for SharedVar<T> {
    type AsReadOnly = ReadOnlyVar<T, Self>;
    type AsLocal = CloningLocalVar<T, Self>;

    fn push_modify(&self, modify: impl FnOnce(&mut T) + 'static, _: &Vars, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
        let var = self.clone();
        updates.push_modify_impl(move |assert, cleanup| {
            var.modify(modify, assert, cleanup);
        });
        Ok(())
    }

    fn map<O, M>(&self, map: M) -> MapVar<T, Self, O, M>
    where
        M: FnMut(&T) -> O + 'static,
        O: VarValue,
    {
        MapVar::new(MapVarInner::Shared(MapSharedVar::new(
            self.clone(),
            map,
            self.r.version.get().wrapping_sub(1),
        )))
    }

    fn map_bidi<O, M, N>(&self, map: M, map_back: N) -> MapVarBiDi<T, Self, O, M, N>
    where
        M: FnMut(&T) -> O + 'static,
        N: FnMut(&O) -> T + 'static,
        O: VarValue,
    {
        MapVarBiDi::new(MapVarBiDiInner::Shared(MapBiDiSharedVar::new(
            self.clone(),
            map,
            map_back,
            self.r.version.get().wrapping_sub(1),
        )))
    }

    fn as_read_only(self) -> Self::AsReadOnly {
        ReadOnlyVar::new(self)
    }

    fn as_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }
}

impl<T: VarValue> IntoVar<T> for SharedVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

/* MAP */

struct MapSharedVarInner<T: VarValue, S: ObjVar<T>, O: VarValue, M: FnMut(&T) -> O> {
    _t: PhantomData<T>,
    source: S,
    map: RefCell<M>,
    output: UnsafeCell<MaybeUninit<O>>,
    output_version: Cell<u32>,
}

/// A read-only variable that maps the value of another variable.
pub(crate) struct MapSharedVar<T: VarValue, S: ObjVar<T>, O: VarValue, M: FnMut(&T) -> O> {
    r: Rc<MapSharedVarInner<T, S, O, M>>,
}

struct MapBiDiSharedVarInner<T: VarValue, S: ObjVar<T>, O: VarValue, M: FnMut(&T) -> O + 'static, N: FnMut(&O) -> T> {
    _t: PhantomData<T>,
    source: S,
    map: RefCell<M>,
    map_back: RefCell<N>,
    output: UnsafeCell<MaybeUninit<O>>,
    output_version: Cell<u32>,
}

/// A variable that maps the value of another variable.
pub(crate) struct MapBiDiSharedVar<T: VarValue, S: ObjVar<T>, O: VarValue, M: FnMut(&T) -> O + 'static, N: FnMut(&O) -> T> {
    r: Rc<MapBiDiSharedVarInner<T, S, O, M, N>>,
}

impl<T: VarValue, S: ObjVar<T>, O: VarValue, M: FnMut(&T) -> O> MapSharedVar<T, S, O, M> {
    pub(crate) fn new(source: S, map: M, prev_version: u32) -> Self {
        MapSharedVar {
            r: Rc::new(MapSharedVarInner {
                _t: PhantomData,
                source,
                map: RefCell::new(map),
                output: UnsafeCell::new(MaybeUninit::uninit()),
                output_version: Cell::new(prev_version),
            }),
        }
    }

    fn borrow<'a>(&'a self, vars: &'a Vars) -> &'a O {
        let source_version = self.r.source.version(vars);
        if self.r.output_version.get() != source_version {
            let value = (&mut *self.r.map.borrow_mut())(self.r.source.get(vars));
            // SAFETY: This is safe because it only happens before the first borrow
            // of this update, and borrows cannot exist across updates because source
            // vars require a &mut Vars for changing version.
            unsafe {
                let m_uninit = &mut *self.r.output.get();
                m_uninit.as_mut_ptr().write(value);
            }
            self.r.output_version.set(source_version);
        }

        // SAFETY:
        // This is safe because source require &mut Vars for updating.
        unsafe {
            let inited = &*self.r.output.get();
            &*inited.as_ptr()
        }
    }
}

impl<T, S, O, M, N> MapBiDiSharedVar<T, S, O, M, N>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
    N: FnMut(&O) -> T,
{
    pub(crate) fn new(source: S, map: M, map_back: N, prev_version: u32) -> Self {
        MapBiDiSharedVar {
            r: Rc::new(MapBiDiSharedVarInner {
                _t: PhantomData,
                source,
                map: RefCell::new(map),
                map_back: RefCell::new(map_back),
                output: UnsafeCell::new(MaybeUninit::uninit()),
                output_version: Cell::new(prev_version),
            }),
        }
    }

    fn borrow<'a>(&'a self, vars: &'a Vars) -> &'a O {
        let source_version = self.r.source.version(vars);
        if self.r.output_version.get() != source_version {
            let value = (&mut *self.r.map.borrow_mut())(self.r.source.get(vars));
            // SAFETY: This is safe because it only happens before the first borrow
            // of this update, and borrows cannot exist across updates because source
            // vars require a &mut Vars for changing version.
            unsafe {
                let m_uninit = &mut *self.r.output.get();
                m_uninit.as_mut_ptr().write(value);
            }
            self.r.output_version.set(source_version);
        }

        // SAFETY:
        // This is safe because we require &mut Vars for propagating updates
        // back to the source variable.
        unsafe {
            let inited = &*self.r.output.get();
            &*inited.as_ptr()
        }
    }
}

impl<T, S, O, M> protected::Var<O> for MapSharedVar<T, S, O, M>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
{
    fn bind_info<'a>(&'a self, vars: &'a Vars) -> protected::BindInfo<'a, O> {
        protected::BindInfo::Var(self.borrow(vars), self.is_new(vars), self.version(vars))
    }
}

impl<T, S, O, M, N> protected::Var<O> for MapBiDiSharedVar<T, S, O, M, N>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
    N: FnMut(&O) -> T + 'static,
{
    fn bind_info<'a>(&'a self, vars: &'a Vars) -> protected::BindInfo<'a, O> {
        protected::BindInfo::Var(self.borrow(vars), self.is_new(vars), self.version(vars))
    }

    fn read_only_prev_version(&self) -> u32 {
        self.r.output_version.get().wrapping_sub(1)
    }
}

impl<T, S, O, M> ObjVar<O> for MapSharedVar<T, S, O, M>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
{
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a O {
        self.borrow(vars)
    }

    fn update<'a>(&'a self, vars: &'a Vars) -> Option<&'a O> {
        if self.is_new(vars) {
            Some(self.borrow(vars))
        } else {
            None
        }
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.r.source.is_new(vars)
    }

    fn version(&self, vars: &Vars) -> u32 {
        self.r.source.version(vars)
    }
}

impl<T, S, O, M, N> ObjVar<O> for MapBiDiSharedVar<T, S, O, M, N>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
    N: FnMut(&O) -> T + 'static,
{
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a O {
        self.borrow(vars)
    }

    fn update<'a>(&'a self, vars: &'a Vars) -> Option<&'a O> {
        if self.is_new(vars) {
            Some(self.borrow(vars))
        } else {
            None
        }
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.r.source.is_new(vars)
    }

    fn version(&self, vars: &Vars) -> u32 {
        self.r.source.version(vars)
    }

    fn read_only(&self, vars: &Vars) -> bool {
        self.r.source.read_only(vars)
    }

    fn always_read_only(&self, vars: &Vars) -> bool {
        self.r.source.always_read_only(vars)
    }

    fn push_set(&self, new_value: O, vars: &Vars, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
        self.r
            .source
            .push_set((&mut *self.r.map_back.borrow_mut())(&new_value), vars, updates)
    }

    fn push_modify_boxed(
        &self,
        modify: Box<dyn FnOnce(&mut O) + 'static>,
        vars: &Vars,
        updates: &mut Updates,
    ) -> Result<(), VarIsReadOnly> {
        let r = Rc::clone(&self.r);
        self.r.source.push_modify_boxed(
            Box::new(move |input| {
                let mut value = (&mut *r.map.borrow_mut())(input);
                modify(&mut value);
                let output = (&mut *r.map_back.borrow_mut())(&value);
                *input = output;
            }),
            vars,
            updates,
        )
    }
}

impl<T, S, O, M> Clone for MapSharedVar<T, S, O, M>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
{
    fn clone(&self) -> Self {
        MapSharedVar { r: Rc::clone(&self.r) }
    }
}

impl<T, S, O, M, N> Clone for MapBiDiSharedVar<T, S, O, M, N>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
    N: FnMut(&O) -> T + 'static,
{
    fn clone(&self) -> Self {
        MapBiDiSharedVar { r: Rc::clone(&self.r) }
    }
}

impl<T, S, O, M> Var<O> for MapSharedVar<T, S, O, M>
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
        MapVar::new(MapVarInner::Shared(MapSharedVar::new(
            self.clone(),
            map,
            self.r.output_version.get().wrapping_sub(1),
        )))
    }

    fn map_bidi<O2, M2, N2>(&self, map: M2, map_back: N2) -> MapVarBiDi<O, Self, O2, M2, N2>
    where
        O2: VarValue,
        M2: FnMut(&O) -> O2,
        N2: FnMut(&O2) -> O,
    {
        MapVarBiDi::new(MapVarBiDiInner::Shared(MapBiDiSharedVar::new(
            self.clone(),
            map,
            map_back,
            self.r.output_version.get().wrapping_sub(1),
        )))
    }

    fn as_read_only(self) -> Self {
        self
    }

    fn as_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }
}

impl<T, S, O, M, N> Var<O> for MapBiDiSharedVar<T, S, O, M, N>
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
        let r = Rc::clone(&self.r);
        self.r.source.push_modify_boxed(
            Box::new(move |input| {
                let mut value = (&mut *r.map.borrow_mut())(input);
                modify(&mut value);
                let output = (&mut *r.map_back.borrow_mut())(&value);
                *input = output;
            }),
            vars,
            updates,
        )
    }

    fn map<O2, M2>(&self, map: M2) -> MapVar<O, Self, O2, M2>
    where
        O2: VarValue,
        M2: FnMut(&O) -> O2,
    {
        MapVar::new(MapVarInner::Shared(MapSharedVar::new(
            self.clone(),
            map,
            self.r.output_version.get().wrapping_sub(1),
        )))
    }

    fn map_bidi<O2, M2, N2>(&self, map: M2, map_back: N2) -> MapVarBiDi<O, Self, O2, M2, N2>
    where
        O2: VarValue,
        M2: FnMut(&O) -> O2,
        N2: FnMut(&O2) -> O,
    {
        MapVarBiDi::new(MapVarBiDiInner::Shared(MapBiDiSharedVar::new(
            self.clone(),
            map,
            map_back,
            self.r.output_version.get().wrapping_sub(1),
        )))
    }

    fn as_read_only(self) -> Self::AsReadOnly {
        ReadOnlyVar::new(self)
    }

    fn as_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }
}

impl<T, S, O, M> IntoVar<O> for MapSharedVar<T, S, O, M>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
{
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T, S, O, M, N> IntoVar<O> for MapBiDiSharedVar<T, S, O, M, N>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
    N: FnMut(&O) -> T + 'static,
{
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

/// Initializes a new [`SharedVar`](crate::core::var::SharedVar).
pub fn var<T: VarValue>(initial_value: impl Into<T>) -> SharedVar<T> {
    SharedVar::new(initial_value.into())
}
