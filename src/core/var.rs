use super::context::{ContextVarStageId, Updates, Vars};
use fnv::FnvHashMap;
use std::cell::{Cell, RefCell, UnsafeCell};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::rc::Rc;

// #region Traits

/// A type that can be a [`Var`](crate::core::var::Var) value.
///
/// # Trait Alias
///
/// This trait is used like a type alias for traits and is
/// already implemented for all types it applies to.
pub trait VarValue: Debug + Clone + 'static {}
impl<T: Debug + Clone + 'static> VarValue for T {}

/// A variable value that is set by the ancestors of an UiNode.
pub trait ContextVar: Clone + Copy + 'static {
    /// The variable type.
    type Type: VarValue;

    /// Default value, used when the variable is not set in the context.
    fn default() -> &'static Self::Type;
}

pub(crate) mod protected {
    use super::{VarValue, Vars};
    use std::any::TypeId;

    /// Info for context var binding.
    pub enum BindInfo<'a, T: VarValue> {
        /// Owned or SharedVar.
        ///
        /// * `&'a T` is a reference to the value borrowed in the context.
        /// * `bool` is the is_new flag.
        Var(&'a T, bool, u32),
        /// ContextVar.
        ///
        /// * `TypeId` of self.
        /// * `&'static T` is the ContextVar::default value of self.
        /// * `Option<(bool, u32)>` optional is_new and version override.
        ContextVar(TypeId, &'static T, Option<(bool, u32)>),
    }

    /// pub(crate) part of `ObjVar`.
    pub trait Var<T: VarValue>: 'static {
        fn bind_info<'a>(&'a self, vars: &'a Vars) -> BindInfo<'a, T>;

        fn is_context_var(&self) -> bool {
            false
        }

        fn read_only_prev_version(&self) -> u32 {
            0
        }
    }

    /// pub(crate) part of `SwitchVar`.
    pub trait SwitchVar<T: VarValue>: Var<T> {
        fn modify(self, new_index: usize, cleanup: &mut Vec<Box<dyn FnOnce()>>);
    }
}

/// Error when trying to set or motify a read-only variable.
#[derive(Debug, Hash, PartialEq, Eq)]
pub struct VarIsReadOnly;

impl std::fmt::Display for VarIsReadOnly {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "cannot set or modify read-only variable")
    }
}

impl std::error::Error for VarIsReadOnly {}

/// Part of [Var] that can be boxed (object safe).
pub trait ObjVar<T: VarValue>: protected::Var<T> {
    /// The current value.
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a T;

    /// [get](ObjVar::get) if [is_new](ObjVar::is_new) or none.
    fn update<'a>(&'a self, vars: &'a Vars) -> Option<&'a T>;

    /// If the value changed this update.
    fn is_new(&self, vars: &Vars) -> bool;

    /// Current value version. Version changes every time the value changes.
    fn version(&self, vars: &Vars) -> u32;

    /// Gets if the variable is currently read-only.
    fn read_only(&self) -> bool {
        true
    }

    /// Gets if the variable is always read-only.
    fn always_read_only(&self) -> bool {
        true
    }

    /// Schedules a variable change for the next update if the variable is not [read_only](ObjVar::read_only).
    fn push_set(&self, _new_value: T, _vars: &mut Updates) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    /// Schedules a variable modification for the next update using a boxed closure.
    fn push_modify_boxed(&self, _modify: Box<dyn FnOnce(&mut T) + 'static>, _vars: &mut Updates) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    /// Box the variable. This disables mapping.
    fn boxed(self) -> BoxVar<T>
    where
        Self: std::marker::Sized,
    {
        Box::new(self)
    }
}

/// Boxed [ObjVar].
pub type BoxVar<T> = Box<dyn ObjVar<T>>;

/// Boxed [LocalVar].
pub type BoxLocalVar<T> = Box<dyn LocalVar<T>>;

/// A value that can change. Can [own the value](OwnedVar) or be a [reference](SharedVar).
///
/// This is the complete generic trait, the non-generic methods are defined in [ObjVar]
/// to support boxing.
///
/// Cannot be implemented outside of zero-ui crate. Use this together with [IntoVar] to
/// support dinamic values in property definitions.
pub trait Var<T: VarValue>: ObjVar<T> {
    /// Return type of [as_read_only](Var::as_read_only).
    type AsReadOnly: Var<T>;
    /// Return type of [as_local](Var::as_local).
    type AsLocal: LocalVar<T>;

    /// Schedules a variable modification for the next update.
    fn push_modify(&self, _modify: impl FnOnce(&mut T) + 'static, _vars: &mut Updates) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    /// Returns a read-only `Var<O>` that uses a closure to generate its value from this `Var<T>` every time it changes.
    fn map<O, M>(&self, map: M) -> MapVar<T, Self, O, M>
    where
        Self: Sized,
        M: FnMut(&T) -> O + 'static,
        O: VarValue;

    /// Bidirectional map. Returns a `Var<O>` that uses two closures to convert to and from this `Var<T>`.
    ///
    /// Unlike [map](Var::map) the returned variable is read-write when this variable is read-write.
    fn map_bidi<O, M, N>(&self, map: M, map_back: N) -> MapVarBiDi<T, Self, O, M, N>
    where
        Self: Sized,
        O: VarValue,
        M: FnMut(&T) -> O + 'static,
        N: FnMut(&O) -> T + 'static;

    /// Ensures this variable is [always_read_only](ObjVar::always_read_only).
    fn as_read_only(self) -> Self::AsReadOnly;

    /// Returns a [variable](LocalVar) that keeps the current value locally so
    /// it can be read without a [context](Vars).
    fn as_local(self) -> Self::AsLocal;
}

/// A value-to-[var](Var) conversion that consumes the value.
pub trait IntoVar<T: VarValue> {
    type Var: Var<T> + 'static;

    /// Converts the source value into a var.
    fn into_var(self) -> Self::Var;

    /// Shortcut call `self.into_var().as_local()`.
    #[inline]
    fn into_local(self) -> <<Self as IntoVar<T>>::Var as Var<T>>::AsLocal
    where
        Self: Sized,
    {
        self.into_var().as_local()
    }
}

/// A variable that can be one of many variables at a time, determined by its index.
#[allow(clippy::len_without_is_empty)]
pub trait SwitchVar<T: VarValue>: Var<T> + protected::SwitchVar<T> {
    /// Current variable index.
    fn index(&self) -> usize;

    /// Number of variables that can be indexed.
    fn len(&self) -> usize;
}

/// A variable that can be read without [context](Vars).
pub trait LocalVar<T: VarValue>: ObjVar<T> {
    /// Gets the local copy of the value.
    fn get_local(&self) -> &T;

    /// Initializes the local copy of the value. Mut be called on [init](crate::core::UiNode::init).
    fn init_local<'a, 'b>(&'a mut self, vars: &'b Vars) -> &'a T;

    /// Update the local copy of the value. Must be called every [update](crate::core::UiNode::update).
    fn update_local<'a, 'b>(&'a mut self, vars: &'b Vars) -> Option<&'a T>;
}

// #endregion Traits

//# region CloningLocalVar<T>

/// Variable that keeps a local clone of the current value.
pub struct CloningLocalVar<T: VarValue, V: Var<T>> {
    var: V,
    local: Option<T>,
}

impl<T: VarValue, V: Var<T>> CloningLocalVar<T, V> {
    fn new(var: V) -> Self {
        CloningLocalVar { var, local: None }
    }
}

impl<T: VarValue, V: Var<T>> protected::Var<T> for CloningLocalVar<T, V> {
    fn bind_info<'a>(&'a self, vars: &'a Vars) -> protected::BindInfo<'a, T> {
        self.var.bind_info(vars)
    }

    fn is_context_var(&self) -> bool {
        self.var.is_context_var()
    }

    fn read_only_prev_version(&self) -> u32 {
        self.var.read_only_prev_version()
    }
}

impl<T: VarValue, V: Var<T>> ObjVar<T> for CloningLocalVar<T, V> {
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a T {
        self.var.get(vars)
    }

    fn update<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        self.var.update(vars)
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.var.is_new(vars)
    }

    fn version(&self, vars: &Vars) -> u32 {
        self.var.version(vars)
    }

    fn read_only(&self) -> bool {
        self.var.read_only()
    }

    fn always_read_only(&self) -> bool {
        self.var.always_read_only()
    }

    fn push_set(&self, new_value: T, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
        self.var.push_set(new_value, updates)
    }

    fn push_modify_boxed(&self, modify: Box<dyn FnOnce(&mut T) + 'static>, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
        self.var.push_modify_boxed(modify, updates)
    }
}

impl<T: VarValue, V: Var<T>> LocalVar<T> for CloningLocalVar<T, V> {
    fn get_local(&self) -> &T {
        self.local.as_ref().expect("`init_local` was never called")
    }

    fn init_local<'a, 'b>(&'a mut self, vars: &'b Vars) -> &'a T {
        self.local = Some(self.var.get(vars).clone());
        self.get_local()
    }

    fn update_local<'a, 'b>(&'a mut self, vars: &'b Vars) -> Option<&'a T> {
        match self.var.update(vars) {
            Some(update) => {
                self.local = Some(update.clone());
                Some(self.get_local())
            }
            None => None,
        }
    }
}

//# endregion CloningLocalVar<T>

// #region Var<T> for ContextVar<Type=T>

impl<T: VarValue, V: ContextVar<Type = T>> protected::Var<T> for V {
    fn bind_info<'a, 'b>(&'a self, _: &'b Vars) -> protected::BindInfo<'a, T> {
        protected::BindInfo::ContextVar(std::any::TypeId::of::<V>(), V::default(), None)
    }

    fn is_context_var(&self) -> bool {
        true
    }
}

impl<T: VarValue, V: ContextVar<Type = T>> ObjVar<T> for V {
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a T {
        vars.context::<V>()
    }

    fn update<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        vars.context_update::<V>()
    }

    fn is_new(&self, vars: &Vars) -> bool {
        vars.context_is_new::<V>()
    }

    fn version(&self, vars: &Vars) -> u32 {
        vars.context_version::<V>()
    }
}

impl<T: VarValue, V: ContextVar<Type = T>> Var<T> for V {
    type AsReadOnly = Self;
    type AsLocal = CloningLocalVar<T, Self>;

    fn map<O, M>(&self, map: M) -> MapVar<T, Self, O, M>
    where
        M: FnMut(&T) -> O + 'static,
        O: VarValue,
    {
        MapVar::new(MapVarInner::Context(MapContextVar::new(*self, map)))
    }

    fn map_bidi<O, M, N>(&self, map: M, _: N) -> MapVarBiDi<T, Self, O, M, N>
    where
        M: FnMut(&T) -> O + 'static,
        N: FnMut(&O) -> T + 'static,
        O: VarValue,
    {
        MapVarBiDi::new(MapVarBiDiInner::Context(MapContextVar::new(*self, map)))
    }

    fn as_read_only(self) -> Self {
        self
    }

    fn as_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }
}

// #endregion Var<T> for ContextVar<Type=T>

// #region OwnedVar<T>

/// [Var] implementer that owns the value.
pub struct OwnedVar<T: VarValue>(pub T);

impl<T: VarValue> protected::Var<T> for OwnedVar<T> {
    fn bind_info<'a, 'b>(&'a self, _: &'b Vars) -> protected::BindInfo<'a, T> {
        protected::BindInfo::Var(&self.0, false, 0)
    }
}

impl<T: VarValue> ObjVar<T> for OwnedVar<T> {
    fn get(&self, _: &Vars) -> &T {
        &self.0
    }

    fn update<'a>(&'a self, _: &'a Vars) -> Option<&'a T> {
        None
    }

    fn is_new(&self, _: &Vars) -> bool {
        false
    }

    fn version(&self, _: &Vars) -> u32 {
        0
    }
}

impl<T: VarValue> Var<T> for OwnedVar<T> {
    type AsReadOnly = Self;
    type AsLocal = Self;

    fn map<O, M>(&self, mut map: M) -> MapVar<T, Self, O, M>
    where
        M: FnMut(&T) -> O + 'static,
        O: VarValue,
    {
        MapVar::new(MapVarInner::Owned(Rc::new(OwnedVar(map(&self.0)))))
    }

    fn map_bidi<O, M, N>(&self, mut map: M, _: N) -> MapVarBiDi<T, Self, O, M, N>
    where
        M: FnMut(&T) -> O + 'static,
        N: FnMut(&O) -> T + 'static,
        O: VarValue,
    {
        MapVarBiDi::new(MapVarBiDiInner::Owned(Rc::new(OwnedVar(map(&self.0)))))
    }

    fn as_read_only(self) -> Self {
        self
    }

    fn as_local(self) -> Self {
        self
    }
}

impl<T: VarValue> LocalVar<T> for OwnedVar<T> {
    fn get_local(&self) -> &T {
        &self.0
    }

    fn init_local<'a, 'b>(&'a mut self, _: &'b Vars) -> &'a T {
        &self.0
    }

    fn update_local<'a, 'b>(&'a mut self, _: &'b Vars) -> Option<&'a T> {
        None
    }
}

impl<T: VarValue> IntoVar<T> for OwnedVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

/// Wraps the value in an `[Owned]<T>` value.
impl<T: VarValue> IntoVar<T> for T {
    type Var = OwnedVar<T>;

    fn into_var(self) -> OwnedVar<T> {
        OwnedVar(self)
    }
}

// #endregion OwnedVar<T>

// #region SharedVar<T>

struct SharedVarInner<T> {
    data: UnsafeCell<T>,
    is_new: Cell<bool>,
    version: Cell<u32>,
}

/// A reference-counting [Var].
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

    fn read_only(&self) -> bool {
        false
    }

    fn always_read_only(&self) -> bool {
        false
    }

    fn push_set(&self, new_value: T, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
        let var = self.clone();
        updates.push_modify_impl(move |assert, cleanup| {
            var.modify(move |v: &mut T| *v = new_value, assert, cleanup);
        });
        Ok(())
    }

    fn push_modify_boxed(&self, modify: Box<dyn FnOnce(&mut T) + 'static>, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
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

    fn push_modify(&self, modify: impl FnOnce(&mut T) + 'static, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
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

// #endregion SharedVar<T>

// #region ReadOnlyVar<T>

/// A variable that is [always_read_only](ObjVar::always_read_only).
///
/// This `struct` is created by the [as_read_only](Var::as_read_only) method in variables
/// that are not `always_read_only`.
pub struct ReadOnlyVar<T: VarValue, V: Var<T> + Clone> {
    _t: PhantomData<T>,
    var: V,
}

impl<T: VarValue, V: Var<T> + Clone> ReadOnlyVar<T, V> {
    fn new(var: V) -> Self {
        ReadOnlyVar { _t: PhantomData, var }
    }
}

impl<T: VarValue, V: Var<T> + Clone> protected::Var<T> for ReadOnlyVar<T, V> {
    fn bind_info<'a>(&'a self, vars: &'a Vars) -> protected::BindInfo<'a, T> {
        self.var.bind_info(vars)
    }
}

impl<T: VarValue, V: Var<T> + Clone> ObjVar<T> for ReadOnlyVar<T, V> {
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a T {
        self.var.get(vars)
    }

    /// [get](ObjVar::get) if [is_new](ObjVar::is_new) or none.
    fn update<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        self.var.update(vars)
    }

    /// If the value changed this update.
    fn is_new(&self, vars: &Vars) -> bool {
        self.var.is_new(vars)
    }

    /// Current value version. Version changes every time the value changes.
    fn version(&self, vars: &Vars) -> u32 {
        self.var.version(vars)
    }
}

impl<T: VarValue, V: Var<T> + Clone> Clone for ReadOnlyVar<T, V> {
    fn clone(&self) -> Self {
        ReadOnlyVar {
            _t: PhantomData,
            var: self.var.clone(),
        }
    }
}

impl<T: VarValue, V: Var<T> + Clone> Var<T> for ReadOnlyVar<T, V> {
    type AsReadOnly = Self;
    type AsLocal = CloningLocalVar<T, Self>;

    fn map<O, M>(&self, map: M) -> MapVar<T, Self, O, M>
    where
        M: FnMut(&T) -> O + 'static,
        O: VarValue,
    {
        MapVar::new(MapVarInner::Shared(MapSharedVar::new(
            self.clone(),
            map,
            self.var.read_only_prev_version(),
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
            self.var.read_only_prev_version(),
        )))
    }

    fn as_read_only(self) -> Self {
        self
    }

    fn as_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }
}

// #endregion ReadOnlyVar<T>

// #region MapSharedVar<T> and MapBiDiSharedVar<T>

struct MapSharedVarInner<T: VarValue, S: ObjVar<T>, O: VarValue, M: FnMut(&T) -> O> {
    _t: PhantomData<T>,
    source: S,
    map: RefCell<M>,
    output: UnsafeCell<MaybeUninit<O>>,
    output_version: Cell<u32>,
}

/// A read-only variable that maps the value of another variable.
struct MapSharedVar<T: VarValue, S: ObjVar<T>, O: VarValue, M: FnMut(&T) -> O> {
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
struct MapBiDiSharedVar<T: VarValue, S: ObjVar<T>, O: VarValue, M: FnMut(&T) -> O + 'static, N: FnMut(&O) -> T> {
    r: Rc<MapBiDiSharedVarInner<T, S, O, M, N>>,
}

impl<T: VarValue, S: ObjVar<T>, O: VarValue, M: FnMut(&T) -> O> MapSharedVar<T, S, O, M> {
    fn new(source: S, map: M, prev_version: u32) -> Self {
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
    fn new(source: S, map: M, map_back: N, prev_version: u32) -> Self {
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

    fn read_only(&self) -> bool {
        self.r.source.read_only()
    }

    fn always_read_only(&self) -> bool {
        self.r.source.always_read_only()
    }

    fn push_set(&self, new_value: O, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
        self.r.source.push_set((&mut *self.r.map_back.borrow_mut())(&new_value), updates)
    }

    fn push_modify_boxed(&self, modify: Box<dyn FnOnce(&mut O) + 'static>, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
        let r = Rc::clone(&self.r);
        self.r.source.push_modify_boxed(
            Box::new(move |input| {
                let mut value = (&mut *r.map.borrow_mut())(input);
                modify(&mut value);
                let output = (&mut *r.map_back.borrow_mut())(&value);
                *input = output;
            }),
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

    fn push_modify(&self, modify: impl FnOnce(&mut O) + 'static, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
        let r = Rc::clone(&self.r);
        self.r.source.push_modify_boxed(
            Box::new(move |input| {
                let mut value = (&mut *r.map.borrow_mut())(input);
                modify(&mut value);
                let output = (&mut *r.map_back.borrow_mut())(&value);
                *input = output;
            }),
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

// #endregion MapSharedVar<T> and MapBiDiSharedVar<T>

// #region MapContextVar<T>

type MapContextVarOutputs<O> = FnvHashMap<ContextVarStageId, (UnsafeCell<O>, u32)>;

struct MapContextVarInner<T: VarValue, S: ObjVar<T>, O: VarValue, M: FnMut(&T) -> O> {
    _t: PhantomData<T>,
    source: S,
    map: RefCell<M>,
    outputs: RefCell<MapContextVarOutputs<O>>,
}

/// A variable that maps the value of a context variable.
struct MapContextVar<T: VarValue, S: ObjVar<T>, O: VarValue, M: FnMut(&T) -> O> {
    r: Rc<MapContextVarInner<T, S, O, M>>,
}

impl<T: VarValue, S: ObjVar<T>, O: VarValue, M: FnMut(&T) -> O> MapContextVar<T, S, O, M> {
    fn new(source: S, map: M) -> Self {
        MapContextVar {
            r: Rc::new(MapContextVarInner {
                _t: PhantomData,
                source,
                map: RefCell::new(map),
                outputs: RefCell::default(),
            }),
        }
    }

    fn borrow<'a>(&'a self, vars: &'a Vars) -> &'a O {
        use std::collections::hash_map::Entry::{Occupied, Vacant};
        let mut outputs = self.r.outputs.borrow_mut();
        let context_id = vars.context_id();
        let source_version = self.r.source.version(vars);

        let output = match outputs.entry(context_id) {
            Occupied(entry) => {
                let (output, output_version) = entry.into_mut();
                if *output_version != source_version {
                    let value = (&mut *self.r.map.borrow_mut())(self.r.source.get(vars));
                    // SAFETY: This is safe because it only happens before the first borrow
                    // of this update.
                    unsafe { *output.get() = value }
                    *output_version = source_version;
                }
                output
            }
            Vacant(entry) => {
                let value = (&mut *self.r.map.borrow_mut())(self.r.source.get(vars));
                let (output, _) = entry.insert((UnsafeCell::new(value), source_version));
                output
            }
        };

        // SAFETY:
        // This is safe because a mutable reference to `Vars` is required
        // for changing values.
        unsafe { &*output.get() }
    }
}

impl<T, S, O, M> protected::Var<O> for MapContextVar<T, S, O, M>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
{
    fn bind_info<'a>(&'a self, vars: &'a Vars) -> protected::BindInfo<'a, O> {
        protected::BindInfo::Var(self.borrow(vars), self.r.source.is_new(vars), self.r.source.version(vars))
    }
}

impl<T, S, O, M> ObjVar<O> for MapContextVar<T, S, O, M>
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

impl<T, S, O, M> Clone for MapContextVar<T, S, O, M>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
{
    fn clone(&self) -> Self {
        MapContextVar { r: Rc::clone(&self.r) }
    }
}

impl<T, S, O, M> Var<O> for MapContextVar<T, S, O, M>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
{
    type AsReadOnly = Self;
    type AsLocal = CloningLocalVar<O, Self>;

    fn map<O2, M2>(&self, _map: M2) -> MapVar<O, Self, O2, M2>
    where
        O2: VarValue,
        M2: FnMut(&O) -> O2,
    {
        todo!("when GATs are stable")
    }

    fn map_bidi<O2, M2, N2>(&self, _map: M2, _map_back: N2) -> MapVarBiDi<O, Self, O2, M2, N2>
    where
        O2: VarValue,
        M2: FnMut(&O) -> O2,
        N2: FnMut(&O2) -> O,
    {
        todo!("when GATs are stable")
    }

    fn as_read_only(self) -> Self {
        self
    }

    fn as_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }
}

impl<T, S, O, M> IntoVar<O> for MapContextVar<T, S, O, M>
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

impl<T, S, O, M> IntoVar<O> for MapVar<T, S, O, M>
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

// #endregion MapContextVar<T>

// #region MapVar<T> and MapVarBidi<T>

enum MapVarInner<T, S, O, M>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
{
    Owned(Rc<OwnedVar<O>>),
    Shared(MapSharedVar<T, S, O, M>),
    Context(MapContextVar<T, S, O, M>),
}

enum MapVarBiDiInner<T, S, O, M, N>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
    N: FnMut(&O) -> T + 'static,
{
    Owned(Rc<OwnedVar<O>>),
    Shared(MapBiDiSharedVar<T, S, O, M, N>),
    Context(MapContextVar<T, S, O, M>),
}

/// A variable that maps the value of another variable.
///
/// This `struct` is created by the [map](Var::map) method and is a temporary adapter until
/// [GATs](https://github.com/rust-lang/rust/issues/44265) are stable.
pub struct MapVar<T, S, O, M>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
{
    r: MapVarInner<T, S, O, M>,
}

/// A variable that maps from and to another variable.
///
/// This `struct` is created by the [map_bidi](Var::map_bidi) method and is a temporary adapter until
/// [GATs](https://github.com/rust-lang/rust/issues/44265) are stable.
pub struct MapVarBiDi<T, S, O, M, N>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
    N: FnMut(&O) -> T + 'static,
{
    r: MapVarBiDiInner<T, S, O, M, N>,
}

impl<T, S, O, M> MapVar<T, S, O, M>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
{
    fn new(inner: MapVarInner<T, S, O, M>) -> Self {
        MapVar { r: inner }
    }
}

impl<T, S, O, M, N> MapVarBiDi<T, S, O, M, N>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
    N: FnMut(&O) -> T + 'static,
{
    fn new(inner: MapVarBiDiInner<T, S, O, M, N>) -> Self {
        MapVarBiDi { r: inner }
    }
}

impl<T, S, O, M> protected::Var<O> for MapVar<T, S, O, M>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
{
    fn bind_info<'a>(&'a self, vars: &'a Vars) -> protected::BindInfo<'a, O> {
        match &self.r {
            MapVarInner::Owned(o) => o.bind_info(vars),
            MapVarInner::Shared(s) => s.bind_info(vars),
            MapVarInner::Context(c) => c.bind_info(vars),
        }
    }
}

impl<T, S, O, M, N> protected::Var<O> for MapVarBiDi<T, S, O, M, N>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
    N: FnMut(&O) -> T + 'static,
{
    fn bind_info<'a>(&'a self, vars: &'a Vars) -> protected::BindInfo<'a, O> {
        match &self.r {
            MapVarBiDiInner::Owned(o) => o.bind_info(vars),
            MapVarBiDiInner::Shared(s) => s.bind_info(vars),
            MapVarBiDiInner::Context(c) => c.bind_info(vars),
        }
    }

    fn read_only_prev_version(&self) -> u32 {
        todo!()
    }
}

impl<T, S, O, M> ObjVar<O> for MapVar<T, S, O, M>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
{
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a O {
        match &self.r {
            MapVarInner::Owned(o) => o.get(vars),
            MapVarInner::Shared(s) => s.get(vars),
            MapVarInner::Context(c) => c.get(vars),
        }
    }

    fn update<'a>(&'a self, vars: &'a Vars) -> Option<&'a O> {
        match &self.r {
            MapVarInner::Owned(o) => o.update(vars),
            MapVarInner::Shared(s) => s.update(vars),
            MapVarInner::Context(c) => c.update(vars),
        }
    }

    fn is_new(&self, vars: &Vars) -> bool {
        match &self.r {
            MapVarInner::Owned(o) => o.is_new(vars),
            MapVarInner::Shared(s) => s.is_new(vars),
            MapVarInner::Context(c) => c.is_new(vars),
        }
    }

    fn version(&self, vars: &Vars) -> u32 {
        match &self.r {
            MapVarInner::Owned(o) => o.version(vars),
            MapVarInner::Shared(s) => s.version(vars),
            MapVarInner::Context(c) => c.version(vars),
        }
    }
}

impl<T, S, O, M, N> ObjVar<O> for MapVarBiDi<T, S, O, M, N>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
    N: FnMut(&O) -> T + 'static,
{
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a O {
        match &self.r {
            MapVarBiDiInner::Owned(o) => o.get(vars),
            MapVarBiDiInner::Shared(s) => s.get(vars),
            MapVarBiDiInner::Context(c) => c.get(vars),
        }
    }

    fn update<'a>(&'a self, vars: &'a Vars) -> Option<&'a O> {
        match &self.r {
            MapVarBiDiInner::Owned(o) => o.update(vars),
            MapVarBiDiInner::Shared(s) => s.update(vars),
            MapVarBiDiInner::Context(c) => c.update(vars),
        }
    }

    fn is_new(&self, vars: &Vars) -> bool {
        match &self.r {
            MapVarBiDiInner::Owned(o) => o.is_new(vars),
            MapVarBiDiInner::Shared(s) => s.is_new(vars),
            MapVarBiDiInner::Context(c) => c.is_new(vars),
        }
    }

    fn version(&self, vars: &Vars) -> u32 {
        match &self.r {
            MapVarBiDiInner::Owned(o) => o.version(vars),
            MapVarBiDiInner::Shared(s) => s.version(vars),
            MapVarBiDiInner::Context(c) => c.version(vars),
        }
    }

    fn read_only(&self) -> bool {
        match &self.r {
            MapVarBiDiInner::Owned(o) => o.read_only(),
            MapVarBiDiInner::Shared(s) => s.read_only(),
            MapVarBiDiInner::Context(c) => c.read_only(),
        }
    }

    fn always_read_only(&self) -> bool {
        match &self.r {
            MapVarBiDiInner::Owned(o) => o.always_read_only(),
            MapVarBiDiInner::Shared(s) => s.always_read_only(),
            MapVarBiDiInner::Context(c) => c.always_read_only(),
        }
    }

    fn push_set(&self, new_value: O, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
        match &self.r {
            MapVarBiDiInner::Owned(o) => o.push_set(new_value, updates),
            MapVarBiDiInner::Shared(s) => s.push_set(new_value, updates),
            MapVarBiDiInner::Context(c) => c.push_set(new_value, updates),
        }
    }

    fn push_modify_boxed(&self, modify: Box<dyn FnOnce(&mut O) + 'static>, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
        match &self.r {
            MapVarBiDiInner::Owned(o) => o.push_modify_boxed(modify, updates),
            MapVarBiDiInner::Shared(s) => s.push_modify_boxed(modify, updates),
            MapVarBiDiInner::Context(c) => c.push_modify_boxed(modify, updates),
        }
    }
}

impl<T, S, O, M> Clone for MapVar<T, S, O, M>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
{
    fn clone(&self) -> Self {
        MapVar {
            r: match &self.r {
                MapVarInner::Owned(o) => MapVarInner::Owned(Rc::clone(&o)),
                MapVarInner::Shared(s) => MapVarInner::Shared(s.clone()),
                MapVarInner::Context(c) => MapVarInner::Context(c.clone()),
            },
        }
    }
}

impl<T, S, O, M, N> Clone for MapVarBiDi<T, S, O, M, N>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
    N: FnMut(&O) -> T + 'static,
{
    fn clone(&self) -> Self {
        MapVarBiDi {
            r: match &self.r {
                MapVarBiDiInner::Owned(o) => MapVarBiDiInner::Owned(Rc::clone(&o)),
                MapVarBiDiInner::Shared(s) => MapVarBiDiInner::Shared(s.clone()),
                MapVarBiDiInner::Context(c) => MapVarBiDiInner::Context(c.clone()),
            },
        }
    }
}

impl<T, S, O, M> Var<O> for MapVar<T, S, O, M>
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
        MapVar::new(MapVarInner::Shared(MapSharedVar::new(self.clone(), map, 0)))
        // TODO prev_version?
    }

    fn map_bidi<O2, M2, N2>(&self, map: M2, map_back: N2) -> MapVarBiDi<O, Self, O2, M2, N2>
    where
        O2: VarValue,
        M2: FnMut(&O) -> O2,
        N2: FnMut(&O2) -> O,
    {
        MapVarBiDi::new(MapVarBiDiInner::Shared(MapBiDiSharedVar::new(self.clone(), map, map_back, 0)))
    }

    fn as_read_only(self) -> Self {
        self
    }

    fn as_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }
}

impl<T, S, O, M, N> Var<O> for MapVarBiDi<T, S, O, M, N>
where
    T: VarValue,
    S: ObjVar<T>,
    O: VarValue,
    M: FnMut(&T) -> O + 'static,
    N: FnMut(&O) -> T + 'static,
{
    type AsReadOnly = ReadOnlyVar<O, Self>;
    type AsLocal = CloningLocalVar<O, Self>;

    fn push_modify(&self, modify: impl FnOnce(&mut O) + 'static, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
        match &self.r {
            MapVarBiDiInner::Owned(o) => o.push_modify(modify, updates),
            MapVarBiDiInner::Shared(s) => s.push_modify(modify, updates),
            MapVarBiDiInner::Context(c) => c.push_modify(modify, updates),
        }
    }

    fn map<O2, M2>(&self, map: M2) -> MapVar<O, Self, O2, M2>
    where
        O2: VarValue,
        M2: FnMut(&O) -> O2,
    {
        MapVar::new(MapVarInner::Shared(MapSharedVar::new(self.clone(), map, 0)))
        // TODO prev_version?
    }

    fn map_bidi<O2, M2, N2>(&self, map: M2, map_back: N2) -> MapVarBiDi<O, Self, O2, M2, N2>
    where
        O2: VarValue,
        M2: FnMut(&O) -> O2,
        N2: FnMut(&O2) -> O,
    {
        MapVarBiDi::new(MapVarBiDiInner::Shared(MapBiDiSharedVar::new(self.clone(), map, map_back, 0)))
    }

    fn as_read_only(self) -> Self::AsReadOnly {
        ReadOnlyVar::new(self)
    }

    fn as_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }
}

// #endregion MapVar<T> and MapVarBidi<T>

// #region SwitchVar2<T>..SwitchVar8<T>

macro_rules! impl_switch_vars {
    ($($SwitchVar:ident<$N:expr,$($VN:ident),+> {
        $SwitchVarInner:ident {
            $($n:expr => $vn:ident, $version: ident;)+
        }
    })+) => {$(
        struct $SwitchVarInner<T: VarValue, $($VN: Var<T>),+> {
            _t: PhantomData<T>,
            $($vn: $VN,)+

            index: Cell<u8>,

            $($version: Cell<u32>,)+

            version: Cell<u32>,
            is_new: Cell<bool>
        }

        #[doc(hidden)]
        pub struct $SwitchVar<T: VarValue, $($VN: Var<T>),+> {
            r: Rc<$SwitchVarInner<T, $($VN),+>>,
        }

        impl<T: VarValue, $($VN: Var<T>),+> $SwitchVar<T, $($VN),+> {
            #[allow(clippy::too_many_arguments)]
            pub fn new(index: u8, $($vn: impl IntoVar<T, Var=$VN>),+) -> Self {
                assert!(index < $N);
                $SwitchVar {
                    r: Rc::new($SwitchVarInner {
                        _t: PhantomData,
                        index: Cell::new(index),
                        $($version: Cell::new(0),)+
                        version: Cell::new(0),
                        is_new: Cell::new(false),
                        $($vn: $vn.into_var(),)+
                    })
                }
            }
        }

        impl<T: VarValue, $($VN: Var<T>),+> protected::Var<T> for $SwitchVar<T, $($VN),+> {
            fn bind_info<'a>(&'a self, vars: &'a Vars) -> protected::BindInfo<'a, T> {
                let is_new = self.is_new(vars);
                let version = self.version(vars);
                let inner_info = match self.r.index.get() {
                    $($n => self.r.$vn.bind_info(vars),)+
                    _ => unreachable!(),
                };

                match inner_info {
                    protected::BindInfo::Var(value, _, _) => protected::BindInfo::Var(value, is_new, version),
                    protected::BindInfo::ContextVar(var_id, default, _) => {
                        protected::BindInfo::ContextVar(var_id, default, Some((is_new, version)))
                    }
                }
            }

            fn read_only_prev_version(&self) -> u32 {
                self.r.version.get().wrapping_sub(1)
            }
        }

        impl<T: VarValue, $($VN: Var<T>),+> ObjVar<T> for $SwitchVar<T, $($VN),+> {
            fn get<'a>(&'a self, vars: &'a Vars) -> &'a T {
                match self.r.index.get() {
                    $($n => self.r.$vn.get(vars),)+
                    _ => unreachable!(),
                }
            }

            fn update<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
                if self.r.is_new.get() {
                    Some(self.get(vars))
                } else {
                    match self.r.index.get() {
                        $($n => self.r.$vn.update(vars),)+
                        _ => unreachable!(),
                    }
                }
            }

            fn is_new(&self, vars: &Vars) -> bool {
                self.r.is_new.get()
                    || match self.r.index.get() {
                        $($n => self.r.$vn.is_new(vars),)+
                        _ => unreachable!(),
                    }
            }

            fn version(&self, vars: &Vars) -> u32 {
                match self.r.index.get() {
                    $($n => {
                        let $version = self.r.$vn.version(vars);
                        if $version != self.r.$version.get() {
                            self.r.$version.set($version);
                            self.r.version.set(self.r.version.get().wrapping_add(1));
                        }
                    },)+
                    _ => unreachable!(),
                }
                self.r.version.get()
            }

            fn read_only(&self) -> bool {
                match self.r.index.get() {
                    $($n => self.r.$vn.read_only(),)+
                    _ => unreachable!(),
                }
            }

            fn always_read_only(&self) -> bool {
                $(self.r.$vn.always_read_only()) && +
            }

            fn push_set(&self, new_value: T, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
                match self.r.index.get() {
                    $($n => self.r.$vn.push_set(new_value, updates),)+
                    _ => unreachable!(),
                }
            }

            fn push_modify_boxed(&self, modify: Box<dyn FnOnce(&mut T) + 'static>, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
                match self.r.index.get() {
                    $($n => self.r.$vn.push_modify_boxed(modify, updates),)+
                    _ => unreachable!(),
                }
            }
        }

        impl<T: VarValue, $($VN: Var<T>),+> Clone for $SwitchVar<T, $($VN),+> {
            fn clone(&self) -> Self {
                $SwitchVar { r: Rc::clone(&self.r) }
            }
        }

        impl<T: VarValue, $($VN: Var<T>),+> Var<T> for $SwitchVar<T, $($VN),+> {
            type AsReadOnly = ReadOnlyVar<T, Self>;
            type AsLocal = CloningLocalVar<T, Self>;

            fn push_modify(&self, modify: impl FnOnce(&mut T) + 'static, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
                match self.r.index.get() {
                    $($n => self.r.$vn.push_modify(modify, updates),)+
                    _ => unreachable!(),
                }
            }

            fn map<O, M>(&self, map: M) -> MapVar<T, Self, O, M>  where
        M: FnMut(&T) -> O + 'static,
        O: VarValue,{
                MapVar::new(MapVarInner::Shared(MapSharedVar::new(
                    self.clone(),
                    map,
                    self.r.version.get().wrapping_sub(1),
                )))
            }

            fn map_bidi<O: VarValue, M: FnMut(&T) -> O + 'static, N: FnMut(&O) -> T>(
                &self,
                map: M,
                map_back: N,
            ) -> MapVarBiDi<T, Self, O, M, N> {
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

        impl<T: VarValue, $($VN: Var<T>),+> protected::SwitchVar<T> for $SwitchVar<T, $($VN),+> {
            fn modify(self, new_index: usize, cleanup: &mut Vec<Box<dyn FnOnce()>>) {
                debug_assert!(new_index < $N);
                let new_index = new_index as u8;

                if new_index != self.r.index.get() {
                    self.r.index.set(new_index as u8);
                    self.r.is_new.set(true);
                    self.r.version.set(self.r.version.get().wrapping_add(1));

                    cleanup.push(Box::new(move || self.r.is_new.set(false)));
                }
            }
        }

        impl<T: VarValue, $($VN: Var<T>),+> SwitchVar<T> for $SwitchVar<T, $($VN),+> {
            fn index(&self) -> usize {
                self.r.index.get() as usize
            }

            fn len(&self) -> usize {
                $N
            }
        }

        impl<T: VarValue, $($VN: Var<T>),+> IntoVar<T> for $SwitchVar<T, $($VN),+> {
            type Var = Self;

            fn into_var(self) -> Self::Var {
                self
            }
        }
    )+};
}

impl_switch_vars! {
    SwitchVar2<2, V0, V1> {
        SwitchVar2Inner {
            0 => v0, v0_version;
            1 => v1, v1_version;
        }
    }
    SwitchVar3<3, V0, V1, V2> {
        SwitchVar3Inner {
            0 => v0, v0_version;
            1 => v1, v1_version;
            2 => v2, v2_version;
        }
    }
    SwitchVar4<4, V0, V1, V2, V3> {
        SwitchVar4Inner {
            0 => v0, v0_version;
            1 => v1, v1_version;
            2 => v2, v2_version;
            3 => v3, v3_version;
        }
    }
    SwitchVar5<5, V0, V1, V2, V3, V4> {
        SwitchVar5Inner {
            0 => v0, v0_version;
            1 => v1, v1_version;
            2 => v2, v2_version;
            3 => v3, v3_version;
            4 => v4, v4_version;
        }
    }
    SwitchVar6<6, V0, V1, V2, V3, V4, V5> {
        SwitchVar6Inner {
            0 => v0, v0_version;
            1 => v1, v1_version;
            2 => v2, v2_version;
            3 => v3, v3_version;
            4 => v4, v4_version;
            5 => v5, v5_version;
        }
    }
    SwitchVar7<7, V0, V1, V2, V3, V4, V5, V6> {
        SwitchVar7Inner {
            0 => v0, v0_version;
            1 => v1, v1_version;
            2 => v2, v2_version;
            3 => v3, v3_version;
            4 => v4, v4_version;
            5 => v5, v5_version;
            6 => v6, v6_version;
        }
    }
    SwitchVar8<8, V0, V1, V2, V3, V4, V5, V6, V7> {
        SwitchVar8Inner {
            0 => v0, v0_version;
            1 => v1, v1_version;
            2 => v2, v2_version;
            3 => v3, v3_version;
            4 => v4, v4_version;
            5 => v5, v5_version;
            6 => v6, v6_version;
            7 => v7, v7_version;
        }
    }
}

// #endregion SwitchVar2<T>..SwitchVar8<T>

// #region SwitchVarDyn<T>

struct SwitchVarDynInner<T: 'static> {
    _t: PhantomData<T>,
    vars: Vec<Box<dyn ObjVar<T>>>,
    versions: Vec<Cell<u32>>,

    index: Cell<usize>,

    version: Cell<u32>,
    is_new: Cell<bool>,
}

/// A dynamically-sized set of variables that can be switched on. See [switch_var!] for
/// the full documentation.
pub struct SwitchVarDyn<T: VarValue> {
    r: Rc<SwitchVarDynInner<T>>,
}

impl<T: VarValue> SwitchVarDyn<T> {
    pub fn new(index: usize, vars: Vec<Box<dyn ObjVar<T>>>) -> Self {
        assert!(!vars.is_empty());
        assert!(index < vars.len());

        SwitchVarDyn {
            r: Rc::new(SwitchVarDynInner {
                _t: PhantomData,
                index: Cell::new(index),
                versions: vec![Cell::new(0); vars.len()],
                version: Cell::new(0),
                is_new: Cell::new(false),
                vars,
            }),
        }
    }
}

impl<T: VarValue> protected::Var<T> for SwitchVarDyn<T> {
    fn bind_info<'a>(&'a self, vars: &'a Vars) -> protected::BindInfo<'a, T> {
        let is_new = self.is_new(vars);
        let version = self.version(vars);
        let inner_info = self.r.vars[self.r.index.get()].bind_info(vars);

        match inner_info {
            protected::BindInfo::Var(value, _, _) => protected::BindInfo::Var(value, is_new, version),
            protected::BindInfo::ContextVar(var_id, default, _) => {
                protected::BindInfo::ContextVar(var_id, default, Some((is_new, version)))
            }
        }
    }

    fn read_only_prev_version(&self) -> u32 {
        self.r.version.get().wrapping_sub(1)
    }
}

impl<T: VarValue> ObjVar<T> for SwitchVarDyn<T> {
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a T {
        self.r.vars[self.r.index.get()].get(vars)
    }

    fn update<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        if self.r.is_new.get() {
            Some(self.get(vars))
        } else {
            self.r.vars[self.r.index.get()].update(vars)
        }
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.r.is_new.get() || self.r.vars[self.r.index.get()].is_new(vars)
    }

    fn version(&self, vars: &Vars) -> u32 {
        let index = self.r.index.get();
        let version = self.r.vars[index].version(vars);
        if version != self.r.versions[index].get() {
            self.r.versions[index].set(version);
            self.r.version.set(self.r.version.get().wrapping_add(1));
        }
        self.r.version.get()
    }

    fn read_only(&self) -> bool {
        self.r.vars[self.r.index.get()].read_only()
    }

    fn always_read_only(&self) -> bool {
        self.r.vars.iter().all(|v| v.always_read_only())
    }

    fn push_set(&self, new_value: T, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
        self.r.vars[self.r.index.get()].push_set(new_value, updates)
    }

    fn push_modify_boxed(&self, modify: Box<dyn FnOnce(&mut T) + 'static>, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
        self.r.vars[self.r.index.get()].push_modify_boxed(modify, updates)
    }
}

impl<T: VarValue> Clone for SwitchVarDyn<T> {
    fn clone(&self) -> Self {
        SwitchVarDyn { r: Rc::clone(&self.r) }
    }
}

impl<T: VarValue> Var<T> for SwitchVarDyn<T> {
    type AsReadOnly = ReadOnlyVar<T, Self>;
    type AsLocal = CloningLocalVar<T, Self>;

    fn push_modify(&self, modify: impl FnOnce(&mut T) + 'static, updates: &mut Updates) -> Result<(), VarIsReadOnly> {
        self.push_modify_boxed(Box::new(modify), updates)
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

impl<T: VarValue> protected::SwitchVar<T> for SwitchVarDyn<T> {
    fn modify(self, new_index: usize, cleanup: &mut Vec<Box<dyn FnOnce()>>) {
        debug_assert!(new_index < self.r.vars.len());

        if new_index != self.r.index.get() {
            self.r.index.set(new_index);
            self.r.is_new.set(true);
            self.r.version.set(self.r.version.get().wrapping_add(1));

            cleanup.push(Box::new(move || self.r.is_new.set(false)));
        }
    }
}

impl<T: VarValue> SwitchVar<T> for SwitchVarDyn<T> {
    fn index(&self) -> usize {
        self.r.index.get() as usize
    }

    fn len(&self) -> usize {
        self.r.vars.len()
    }
}

impl<T: VarValue> IntoVar<T> for SwitchVarDyn<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

// #endregion SwitchVarDyn<T>

// #region MergeVar2..MergeVar8

macro_rules! impl_merge_vars {
    ($($MergeVar:ident<$($VN:ident),+> {
        $MergeVarInner:ident<$($TN:ident),+> {
            _t: $($_t: ident),+;
            v: $($vn:ident),+;
            version: $($version:ident),+;
        }
    })+) => {$(
        struct $MergeVarInner<$($TN: VarValue,)+ $($VN: Var<$TN>,)+ O: VarValue, M: FnMut($(&$TN,)+) -> O + 'static> {
            $($_t: PhantomData<$TN>,)+
            $($vn: $VN,)+
            $($version: Cell<u32>,)+
            merge: RefCell<M>,
            output: UnsafeCell<MaybeUninit<O>>,
            version: Cell<u32>
        }

        #[doc(hidden)]
        pub struct $MergeVar<$($TN: VarValue,)+ $($VN: Var<$TN>,)+ O: VarValue, M: FnMut($(&$TN,)+) -> O + 'static> {
            r: Rc<$MergeVarInner<$($TN,)+ $($VN,)+ O, M>>
        }

        impl<$($TN: VarValue,)+ $($VN: Var<$TN>,)+ O: VarValue, M: FnMut($(&$TN,)+) -> O + 'static> $MergeVar<$($TN,)+ $($VN,)+ O, M> {
            #[allow(clippy::too_many_arguments)]
            pub fn new($($vn: $VN,)+ merge: M) -> Self {
                $MergeVar {
                    r: Rc::new($MergeVarInner {
                        $($_t: PhantomData,)+
                        $($version: Cell::new(0),)+ // TODO prev_version
                        $($vn,)+
                        merge: RefCell::new(merge),
                        output: UnsafeCell::new(MaybeUninit::uninit()),
                        version: Cell::new(0)
                    })
                }
            }

            fn sync(&self, vars: &Vars) {
                let mut sync = false;

                $(
                    let version = self.r.$vn.version(vars);
                    if version != self.r.$version.get() {
                        sync = true;
                        self.r.$version.set(version);
                    }
                )+

                if sync {
                    self.r.version.set(self.r.version.get().wrapping_add(1));
                    let value = (&mut *self.r.merge.borrow_mut())($(self.r.$vn.get(vars)),+);

                    // SAFETY: This is safe because it only happens before the first borrow
                    // of this update, and borrows cannot exist across updates because source
                    // vars require a &mut Vars for changing version.
                    unsafe {
                        let m_uninit = &mut *self.r.output.get();
                        m_uninit.as_mut_ptr().write(value);
                    }
                }
            }

            fn borrow<'a>(&'a self, vars: &'a Vars) -> &'a O {
                self.sync(vars);
                // SAFETY:
                // * Value will not change here because we require a mutable reference to
                // `Vars` for changing values in source variables.
                // * Memory is initialized here because we start from the prev_version.
                unsafe {
                    let inited = &*self.r.output.get();
                    &*inited.as_ptr()
                }
            }

            fn any_is_new(&self, vars: &Vars) -> bool {
                 $(self.r.$vn.is_new(vars))||+
            }
        }

        impl<$($TN: VarValue,)+ $($VN: Var<$TN>,)+ O: VarValue, M: FnMut($(&$TN,)+) -> O + 'static> Clone
        for $MergeVar<$($TN,)+ $($VN,)+ O, M> {
            fn clone(&self) -> Self {
                $MergeVar { r: Rc::clone(&self.r) }
            }
        }

        impl<$($TN: VarValue,)+ $($VN: Var<$TN>,)+ O: VarValue, M: FnMut($(&$TN,)+) -> O + 'static> protected::Var<O>
        for $MergeVar<$($TN,)+ $($VN,)+ O, M> {
            fn bind_info<'a>(&'a self, vars: &'a Vars) -> protected::BindInfo<'a, O> {
                protected::BindInfo::Var(self.borrow(vars), self.any_is_new(vars), self.r.version.get())
            }
        }

        impl<$($TN: VarValue,)+ $($VN: Var<$TN>,)+ O: VarValue, M: FnMut($(&$TN,)+) -> O + 'static> ObjVar<O>
        for $MergeVar<$($TN,)+ $($VN,)+ O, M> {
            fn get<'a>(&'a self, vars: &'a Vars) -> &'a O {
                self.borrow(vars)
            }

            fn update<'a>(&'a self, vars: &'a Vars) -> Option<&'a O> {
                if self.any_is_new(vars) {
                    Some(self.borrow(vars))
                } else {
                    None
                }
            }

            fn is_new(&self, vars: &Vars) -> bool {
                self.any_is_new(vars)
            }

            fn version(&self, vars: &Vars) -> u32 {
                self.sync(vars);
                self.r.version.get()
            }
        }

        impl<$($TN: VarValue,)+ $($VN: Var<$TN>,)+ O: VarValue, M: FnMut($(&$TN,)+) -> O + 'static> Var<O>
        for $MergeVar<$($TN,)+ $($VN,)+ O, M> {
            type AsReadOnly = Self;
            type AsLocal = CloningLocalVar<O, Self>;

            fn map<O2: VarValue, M2: FnMut(&O) -> O2>(&self, map: M2) -> MapVar<O, Self, O2, M2> {
                MapVar::new(MapVarInner::Shared(MapSharedVar::new(
                    self.clone(),
                    map,
                    self.r.version.get().wrapping_sub(1),
                )))
            }

            fn map_bidi<O2: VarValue, M2: FnMut(&O) -> O2, N: FnMut(&O2) -> O>(
                &self,
                map: M2,
                map_back: N,
            ) -> MapVarBiDi<O, Self, O2, M2, N> {
                MapVarBiDi::new(MapVarBiDiInner::Shared(MapBiDiSharedVar::new(
                    self.clone(),
                    map,
                    map_back,
                    self.r.version.get().wrapping_sub(1),
                )))
            }

            fn as_read_only(self) -> Self {
                self
            }

            fn as_local(self) -> Self::AsLocal {
                CloningLocalVar::new(self)
            }
        }

        impl<$($TN: VarValue,)+ $($VN: Var<$TN>,)+ O: VarValue, M: FnMut($(&$TN,)+) -> O + 'static> IntoVar<O>
        for $MergeVar<$($TN,)+ $($VN,)+ O, M> {
            type Var = Self;

            fn into_var(self) -> Self::Var {
                self
            }
        }
    )+}
}

impl_merge_vars! {
    MergeVar2<V0, V1> {
        MergeVar2Inner<T0, T1> {
            _t: _t0, _t1;
            v: v0, v1;
            version: v0_version, v1_version;
        }
    }
    MergeVar3<V0, V1, V2> {
        MergeVar3Inner<T0, T1, T2> {
            _t: _t0, _t1, _t2;
            v: v0, v1, v2;
            version: v0_version, v1_version, v2_version;
        }
    }
    MergeVar4<V0, V1, V2, V3> {
        MergeVar4Inner<T0, T1, T2, T3> {
            _t: _t0, _t1, _t2, _t3;
            v: v0, v1, v2, v3;
            version: v0_version, v1_version, v2_version, v3_version;
        }
    }
    MergeVar5<V0, V1, V2, V3, V4> {
        MergeVar5Inner<T0, T1, T2, T3, T4> {
            _t: _t0, _t1, _t2, _t3, _t4;
            v: v0, v1, v2, v3, v4;
            version: v0_version, v1_version, v2_version, v3_version, v4_version;
        }
    }
    MergeVar6<V0, V1, V2, V3, V4, V5> {
        MergeVar6Inner<T0, T1, T2, T3, T4, T5> {
            _t: _t0, _t1, _t2, _t3, _t4, _t5;
            v: v0, v1, v2, v3, v4, v5;
            version: v0_version, v1_version, v2_version, v3_version, v4_version, v5_version;
        }
    }
    MergeVar7<V0, V1, V2, V3, V4, V5, V6> {
        MergeVar7Inner<T0, T1, T2, T3, T4, T5, T6> {
            _t: _t0, _t1, _t2, _t3, _t4, _t5, _t6;
            v: v0, v1, v2, v3, v4, v5, v6;
            version: v0_version, v1_version, v2_version, v3_version, v4_version, v5_version, v6_version;
        }
    }
    MergeVar8<V0, V1, V2, V3, V4, V5, V6, V7> {
        MergeVar8Inner<T0, T1, T2, T3, T4, T5, T6, T7> {
            _t: _t0, _t1, _t2, _t3, _t4, _t5, _t6, _t7;
            v: v0, v1, v2, v3, v4, v5, v6, v7;
            version: v0_version, v1_version, v2_version, v3_version, v4_version, v5_version, v6_version, v7_version;
        }
    }
}

// #endregion MergeVar2..MergeVar8

/// Initializes a new [`SharedVar`](crate::core::var::SharedVar).
pub fn var<T: VarValue>(initial_value: T) -> SharedVar<T> {
    SharedVar::new(initial_value)
}

/// Initializes a new [`SwitchVar`](crate::core::var::SwitchVar).
///
/// # Arguments
///
/// All arguments are separated by comma like a function call.
///
/// * `$index`: A positive integer that is the initial switch index.
/// * `$v0..$vn`: A list of [vars](crate::core::var::ObjVar), minimal 2,
/// [`SwitchVarDyn`](crate::core::var::SwitchVarDyn) is used for more then 8 variables.
///
/// # Example
/// ```
/// # #[macro_use] extern crate zero_ui;
/// # use zero_ui::prelude::var;
/// # fn main() {
/// let var0 = var("Read-write");
/// let var1 = "Read-only";
///
/// let switch_var = switch_var!(0, var0, var1);
/// # }
/// ```
#[macro_export]
macro_rules! switch_var {
    ($index: expr, $v0: expr, $v1: expr) => {
        $crate::core::var::SwitchVar2::new($index, $v0, $v1)
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr) => {
        $crate::core::var::SwitchVar3::new($index, $v0, $v1, $v2)
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr) => {
        $crate::core::var::SwitchVar4::new($index, $v0, $v1, $v2)
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr) => {
        $crate::core::var::SwitchVar5::new($index, $v0, $v1, $v2, $v4)
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr) => {
        $crate::core::var::SwitchVar6::new($index, $v0, $v1, $v2, $v4, $v5)
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr) => {
        $crate::core::var::SwitchVar7::new($index, $v0, $v1, $v2, $v4, $v5, $v6)
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr) => {
        $crate::core::var::SwitchVar8::new($index, $v0, $v1, $v2, $v4, $v5, $v6, $v7)
    };
    ($index: expr, $($v:expr),+) => {
        $crate::core::var::SwitchVarDyn::new($index, vec![$($v.boxed()),+])
    };
    ($($_:tt)*) => {
        compile_error!("this macro takes 3 or more parameters (initial_index, var0, var1, ..)")
    };
}

/// Initializes a new [`Var`](crate::core::var::Var) with value made
/// by merging multiple other variables.
///
/// # Arguments
///
/// All arguments are separated by comma like a function call.
///
/// * `var0..N`: A list of [vars](crate::core::var::Var), minimal 2.
/// * `merge`: A function that produces a new value from references to all variable values. `FnMut(&var0_T, ..) -> merge_T`
///
/// # Example
/// ```
/// # #[macro_use] extern crate zero_ui;
/// # use zero_ui::prelude::var;
/// # use zero_ui::core::var::IntoVar;
/// # fn main() {
/// let var0 = var("Hello");
/// let var1 = var("World");
///
/// let hello_world = merge_var!(var0, var1, |a, b|format!("{} {}!", a, b));
/// # }
/// ```
#[macro_export]
macro_rules! merge_var {
    ($v0: expr, $v1: expr, $merge: expr) => {
        $crate::core::var::MergeVar2::new($v0, $v1, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $merge: expr) => {
        $crate::core::var::MergeVar3::new($v0, $v1, $v2, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $merge: expr) => {
        $crate::core::var::MergeVar4::new($v0, $v1, $v2, $v3, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $merge: expr) => {
        $crate::core::var::MergeVar5::new($v0, $v1, $v2, $v3, $v4, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $merge: expr) => {
        $crate::core::var::MergeVar6::new($v0, $v1, $v2, $v3, $v4, $v5, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $merge: expr) => {
        $crate::core::var::MergeVar7::new($v0, $v1, $v2, $v3, $v4, $v5, $v6, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $merge: expr) => {
        $crate::core::var::MergeVar8::new($v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $($more_args:ident),+) => {
        compile_error!("merge_var is only implemented to a maximum of 8 variables")
    };
    ($($_:tt)*) => {
        compile_error!("this macro takes 3 or more parameters (var0, var1, .., merge_fn")
    };
}
