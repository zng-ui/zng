use std::{
    cell::{Cell, RefCell, UnsafeCell},
    marker::PhantomData,
    rc::{Rc, Weak},
};

use super::*;

/// A [`Var`] that is a [`Rc`] pointer to its value.
pub struct RcVar<T: VarValue>(Rc<Data<T>>);
struct Data<T> {
    value: UnsafeCell<T>,
    last_update_id: Cell<u32>,
    version: Cell<u32>,
}
impl<T: VarValue> RcVar<T> {
    /// New [`RcVar`].
    ///
    /// You can also use the [`var`] function to initialize.
    pub fn new(initial_value: T) -> Self {
        RcVar(Rc::new(Data {
            value: UnsafeCell::new(initial_value),
            last_update_id: Cell::new(0),
            version: Cell::new(0),
        }))
    }

    /// Reference the current value.
    #[inline]
    pub fn get<'a>(&'a self, vars: &'a VarsRead) -> &'a T {
        let _ = vars;
        // SAFETY: this is safe because we are tying the `Vars` lifetime to the value
        // and we require `&mut Vars` to modify the value.
        unsafe { &*self.0.value.get() }
    }

    /// Reference the current value if it [is new](Self::is_new).
    #[inline]
    pub fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        let _ = vars;
        if self.0.last_update_id.get() == vars.update_id() {
            Some(self.get(vars))
        } else {
            None
        }
    }

    /// If the current value changed in the last update.
    #[inline]
    pub fn is_new(&self, vars: &Vars) -> bool {
        self.0.last_update_id.get() == vars.update_id()
    }

    /// Gets the current value version.
    #[inline]
    pub fn version(&self, vars: &VarsRead) -> u32 {
        let _ = vars;
        self.0.version.get()
    }

    /// Schedule a value modification for this variable.
    #[inline]
    pub fn modify<M>(&self, vars: &Vars, modify: M)
    where
        M: FnOnce(&mut VarModify<T>),
    {
        todo!()
    }

    /// Schedule a new value for this variable.
    #[inline]
    pub fn set(&self, vars: &Vars, new_value: T) {
        self.modify(vars, move |v| **v = new_value)
    }

    /// Schedule a new value for this variable, the variable will only be set if
    /// the value is not equal to `new_value`.
    #[inline]
    pub fn set_ne(&self, vars: &Vars, new_value: T)
    where
        T: PartialEq,
    {
        self.modify(vars, move |v| {
            if v.eq(&new_value) {
                **v = new_value;
            }
        })
    }

    /// Gets the number of [`RcVar`] that point to this same variable.
    #[inline]
    pub fn strong_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }

    ///Gets the number of [`WeakVar`] that point to this variable.
    #[inline]
    pub fn weak_count(&self) -> usize {
        Rc::weak_count(&self.0)
    }

    /// Returns `true` if `self` and `other` are the same variable.
    #[inline]
    pub fn ptr_eq(&self, other: &RcVar<T>) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    /// Creates a new [`WeakVar`] that points to this variable.
    #[inline]
    pub fn downgrade(&self) -> WeakVar<T> {
        WeakVar(Rc::downgrade(&self.0))
    }
}
impl<T: VarValue> Clone for RcVar<T> {
    fn clone(&self) -> Self {
        RcVar(Rc::clone(&self.0))
    }
}

/// New [`RcVar`].
#[inline]
pub fn var<T: VarValue>(value: T) -> RcVar<T> {
    RcVar::new(value)
}

/// New [`RcVar`] from any value that converts to `T`.
#[inline]
pub fn var_from<T: VarValue, I: Into<T>>(value: I) -> RcVar<T> {
    RcVar::new(value.into())
}

/// A weak reference to a [`RcVar`].
pub struct WeakVar<T: VarValue>(Weak<Data<T>>);

impl<T: VarValue> Clone for WeakVar<T> {
    fn clone(&self) -> Self {
        WeakVar(self.0.clone())
    }
}

impl<T: VarValue> WeakVar<T> {
    /// Attempts to upgrade to an [`RcVar`], returns `None` if the variable no longer exists.
    pub fn updgrade(&self) -> Option<RcVar<T>> {
        self.0.upgrade().map(RcVar)
    }
}

impl<T: VarValue> Var<T> for RcVar<T> {
    type AsReadOnly = ForceReadOnlyVar<T, Self>;

    type AsLocal = CloningLocalVar<T, Self>;

    fn get<'a>(&'a self, vars: &'a VarsRead) -> &'a T {
        self.get(vars)
    }

    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        self.get_new(vars)
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.is_new(vars)
    }

    fn version(&self, vars: &VarsRead) -> u32 {
        self.version(vars)
    }

    fn is_read_only(&self, _: &VarsRead) -> bool {
        false
    }

    fn always_read_only(&self) -> bool {
        false
    }

    fn can_update(&self) -> bool {
        true
    }

    fn modify<M>(&self, vars: &Vars, modify: M) -> Result<(), VarIsReadOnly>
    where
        M: FnOnce(&mut VarModify<T>) + 'static,
    {
        self.modify(vars, modify);
        Ok(())
    }

    fn set(&self, vars: &Vars, new_value: T) -> Result<(), VarIsReadOnly> {
        self.set(vars, new_value);
        Ok(())
    }

    fn into_read_only(self) -> Self::AsReadOnly {
        ForceReadOnlyVar::new(self)
    }

    fn into_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }
}

impl<A, B, M> VarMap<A, B, M> for RcVar<A>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
{
    type MapVar = RcMapVar<A, B, M, Self>;

    fn map_impl(&self, map: M) -> Self::MapVar {
        self.clone().into_map_impl(map)
    }

    fn into_map_impl(self, map: M) -> Self::MapVar {
        RcMapVar::new(self, map)
    }
}

/// A [`Var`] that maps from another var and is a [`Rc`] pointer to its value.
pub struct RcMapVar<A, B, M, S>(Rc<MapData<A, B, M, S>>)
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    S: Var<A>;
struct MapData<A, B, M: FnMut(&A) -> B, S> {
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
    pub fn get<'a>(&'a self, vars: &'a VarsRead) -> &'a B {
        // SAFETY: access to value is safe because `source` needs a `&mut Vars` to change its version
        // and we change the value only in the first call to `get` with the new source version.

        let version = self.0.source.version(vars);
        let first = unsafe { &*self.0.value.get() }.is_none();

        if first || version != self.0.version.get() {
            let new_value = self.0.map.borrow_mut()(self.0.source.get(vars));

            // SAFETY: see return value.
            unsafe {
                *self.0.value.get_mut() = Some(new_value);
            }

            self.0.version.set(version);
        }

        unsafe { &*self.0.value.get() }.as_ref().unwrap()
    }

    /// Get the value if the source var updated in the last update.
    pub fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a B> {
        if self.0.source.is_new(vars) {
            Some(self.get(vars))
        } else {
            None
        }
    }

    /// Gets if the source var updated in the last update.
    #[inline]
    pub fn is_new(&self, vars: &Vars) -> bool {
        self.0.source.is_new(vars)
    }

    /// Gets the source var value version.
    #[inline]
    pub fn version(&self, vars: &VarsRead) -> u32 {
        self.0.source.version(vars)
    }
}

impl<A, B, M, S> Var<B> for RcMapVar<A, B, M, S>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
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

    fn version(&self, vars: &VarsRead) -> u32 {
        self.version(vars)
    }

    fn is_read_only(&self, vars: &VarsRead) -> bool {
        true
    }

    fn always_read_only(&self) -> bool {
        true
    }

    fn can_update(&self) -> bool {
        self.0.source.can_update()
    }

    fn modify<Mo>(&self, _: &Vars, _: Mo) -> Result<(), VarIsReadOnly>
    where
        Mo: FnOnce(&mut VarModify<B>) + 'static,
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

impl<A, B, M, S, C, M2> VarMap<B, C, M2> for RcMapVar<A, B, M, S>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    S: Var<A>,
    // --
    C: VarValue,
    M2: FnMut(&B) -> C + 'static,
{
    type MapVar = RcMapVar<B, C, M2, Self>;

    fn map_impl(&self, map: M2) -> Self::MapVar {
        self.clone().into_map(map)
    }

    fn into_map_impl(self, map: M2) -> Self::MapVar {
        RcMapVar::new(self, map)
    }
}

impl<A, B, M, S, C, M2, N2> VarMapBidi<B, C, M2, N2> for RcMapVar<A, B, M, S>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    S: Var<A>,
    // --
    C: VarValue,
    M2: FnMut(&B) -> C + 'static,
    N2: FnMut(&C) -> B + 'static,
{
    type MapBidiVar = RcMapVar<B, C, M2, Self>;

    fn map_bidi_impl(&self, map: M2, _: N2) -> Self::MapBidiVar {
        self.clone().into_map(map)
    }

    fn into_map_bidi_impl(self, map: M2, _: N2) -> Self::MapBidiVar {
        RcMapVar::new(self, map)
    }
}
