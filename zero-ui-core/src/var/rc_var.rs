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
        M: FnOnce(&mut VarModify<T>) + 'static,
    {
        let self_ = self.clone();
        vars.push_change(Box::new(move |update_id| {
            // SAFETY: this is safe because Vars requires a mutable reference to apply changes.
            let mut guard = VarModify::new(unsafe { &mut *self_.0.value.get() });
            modify(&mut guard);
            if guard.touched() {
                self_.0.last_update_id.set(update_id);
                self_.0.version.set(self_.0.version.get().wrapping_add(1));
            }
        }));
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
            if !v.eq(&new_value) {
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
    type AsReadOnly = ReadOnlyVar<T, Self>;

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

    fn is_read_only(&self, _: &Vars) -> bool {
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
        ReadOnlyVar::new(self)
    }

    fn into_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }
}
impl<T: VarValue> IntoVar<T> for RcVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
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
                *self.0.value.get() = Some(new_value);
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
        self.0.source.can_update()
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
    pub fn get<'a>(&'a self, vars: &'a VarsRead) -> &'a B {
        // SAFETY: access to value is safe because `source` needs a `&mut Vars` to change its version
        // and we change the value only in the first call to `get` with the new source version.

        let version = self.0.source.version(vars);
        let first = unsafe { &*self.0.value.get() }.is_none();

        if first || version != self.0.version.get() {
            let new_value = self.0.map.borrow_mut()(self.0.source.get(vars));

            // SAFETY: see return value.
            unsafe {
                *self.0.value.get() = Some(new_value);
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

    /// If the source variable is currently read-only. You can only map-back when the source is read-write.
    #[inline]
    pub fn is_read_only(&self, vars: &Vars) -> bool {
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
    fn modify<Mo>(&self, vars: &Vars, modify: Mo) -> Result<(), VarIsReadOnly>
    where
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
    fn set(&self, vars: &Vars, new_value: B) -> Result<(), VarIsReadOnly> {
        let new_value = self.0.map_back.borrow_mut()(new_value);
        self.0.source.set(vars, new_value)
    }

    /// Map back the value and schedules a `set_ne` in the source variable.
    fn set_ne(&self, vars: &Vars, new_value: B) -> Result<(), VarIsReadOnly>
    where
        B: PartialEq,
    {
        let new_value = self.0.map_back.borrow_mut()(new_value);
        self.0.source.set(vars, new_value)
    }

    /// Convert to a [`RcMapVar`] if `self` is the only reference.
    #[inline]
    pub fn into_map(self) -> Result<RcMapVar<A, B, M, S>, Self> {
        match Rc::try_unwrap(self.0) {
            Ok(data) => Ok(RcMapVar::new(data.source, data.map.into_inner())),
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

impl<A, B, M, N, S> Var<B> for RcMapBidiVar<A, B, M, N, S>
where
    A: VarValue,
    B: VarValue,
    M: FnMut(&A) -> B + 'static,
    N: FnMut(B) -> A + 'static,
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

/// New [`StateVar`].
#[inline]
pub fn state_var() -> StateVar {
    var(false)
}

/// Variable type of state properties (`is_*`).
///
/// State variables are `bool` probes that are set by the property.
///
/// Use [`state_var`] to init.
pub type StateVar = RcVar<bool>;

/// New paired [`ResponderVar`] and [`ResponseVar`] in the waiting state.
#[inline]
pub fn response_var<T: VarValue>() -> (ResponderVar<T>, ResponseVar<T>) {
    let responder = var(Response::Waiting::<T>);
    let response = responder.clone().into_read_only();
    (responder, response)
}

/// New [`ResponseVar`] in the done state.
#[inline]
pub fn response_done_var<T: VarValue>(response: T) -> ResponseVar<T> {
    var(Response::Done(response)).into_read_only()
}

/// Variable used to notify the completion of an UI operation.
///
/// Use [`response_var`] to init.
pub type ResponderVar<T> = RcVar<Response<T>>;

/// Variable used to listen to a one time signal that an UI operation has completed.
///
/// Use [`response_var`] or [`response_done_var`] to init.
pub type ResponseVar<T> = ReadOnlyVar<Response<T>, RcVar<Response<T>>>;

/// Raw value in a [`ResponseVar`] or [`ResponseSender`].
#[derive(Clone, Copy)]
pub enum Response<T: VarValue> {
    /// Responder has not set the response yet.
    Waiting,
    /// Responder has set the response.
    Done(T),
}
impl<T: VarValue> fmt::Debug for Response<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            match self {
                Response::Waiting => {
                    write!(f, "Response::Waiting")
                }
                Response::Done(v) => f.debug_tuple("Response::Done").field(v).finish(),
            }
        } else {
            match self {
                Response::Waiting => {
                    write!(f, "Waiting")
                }
                Response::Done(v) => fmt::Debug::fmt(v, f),
            }
        }
    }
}

impl<T: VarValue> ResponseVar<T> {
    /// References the response value if a response was set.
    #[inline]
    pub fn response<'a>(&'a self, vars: &'a VarsRead) -> Option<&'a T> {
        match self.get(vars) {
            Response::Waiting => None,
            Response::Done(r) => Some(r),
        }
    }

    /// References the response value if a response was set for this update.
    pub fn response_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        if let Some(new) = self.get_new(vars) {
            match new {
                Response::Waiting => None,
                Response::Done(r) => Some(r),
            }
        } else {
            None
        }
    }
}

impl<T: VarValue> ResponderVar<T> {
    /// Sets the one time response.
    ///
    /// # Panics
    ///
    /// Panics if the variable is already in the done state.
    #[inline]
    pub fn respond<'a>(&'a self, vars: &'a Vars, response: T) {
        if let Response::Done(_) = self.get(vars) {
            panic!("already responded");
        }
        self.set(vars, Response::Done(response));
    }

    /// Creates a [`ResponseVar`] linked to this responder.
    #[inline]
    pub fn response_var(&self) -> ResponseVar<T> {
        self.clone().into_read_only()
    }
}
