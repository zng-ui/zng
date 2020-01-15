use super::{AppContext, AppContextId, AppContextOwnership, WidgetId};
use fnv::FnvHashMap;
use std::any::type_name;
use std::cell::{Cell, RefCell, UnsafeCell};
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::rc::Rc;

// #region Traits

/// A variable value that is set by the ancestors of an UiNode.
pub trait ContextVar: Clone + Copy + 'static {
    /// The variable type.
    type Type: 'static;

    /// Default value, used when the variable is not set in the context.
    fn default() -> &'static Self::Type;
}

/// A variable value that is set by the previously visited UiNodes during the call.
pub trait VisitedVar: 'static {
    /// The variable type.
    type Type: 'static;
}

pub(crate) mod protected {
    use super::AppContext;
    use std::any::TypeId;

    /// Info for context var binding.
    pub enum BindInfo<'a, T: 'static> {
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
    pub trait Var<T: 'static> {
        fn bind_info<'a, 'b>(&'a self, ctx: &'b AppContext) -> BindInfo<'a, T>;

        fn is_context_var(&self) -> bool {
            false
        }

        fn read_only_prev_version(&self) -> u32 {
            0
        }
    }

    /// pub(crate) part of `SwitchVar`.
    pub trait SwitchVar<T: 'static>: Var<T> {
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
pub trait ObjVar<T: 'static>: protected::Var<T> + 'static {
    /// The current value.
    fn get<'a>(&'a self, ctx: &'a AppContext) -> &'a T;

    /// [get] if [is_new] or none.
    fn update<'a>(&'a self, ctx: &'a AppContext) -> Option<&'a T>;

    /// If the value changed this update.
    fn is_new(&self, ctx: &AppContext) -> bool;

    /// Current value version. Version changes every time the value changes.
    fn version(&self, ctx: &AppContext) -> u32;

    /// Gets if the variable is currently read-only.
    fn read_only(&self) -> bool {
        true
    }

    /// Gets if the variable is always read-only.
    fn always_read_only(&self) -> bool {
        true
    }

    /// Schedules a variable change for the next update if the variable is not [read_only].
    fn push_set(&self, _new_value: T, _ctx: &mut AppContext) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    /// Schedules a variable modification for the next update using a boxed closure.
    fn push_modify_boxed(
        &self,
        _modify: Box<dyn FnOnce(&mut T) + 'static>,
        _ctx: &mut AppContext,
    ) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    /// Box the variable. This disables mapping.
    fn into_box(self) -> BoxVar<T>
    where
        Self: std::marker::Sized,
    {
        Box::new(self)
    }
}

/// Boxed [ObjVar].
pub type BoxVar<T> = Box<dyn ObjVar<T>>;

/// A value that can change. Can [own the value](OwnedVar) or be a [reference](SharedVar).
///
/// This is the complete generic trait, the non-generic methods are defined in [ObjVar]
/// to support boxing.
///
/// Cannot be implemented outside of zero-ui crate. Use this together with [IntoVar] to
/// support dinamic values in property definitions.
pub trait Var<T: 'static>: ObjVar<T> {
    /// Return type of [as_read_only].
    type AsReadOnly: Var<T>;

    /// Schedules a variable modification for the next update.
    fn push_modify(&self, _modify: impl FnOnce(&mut T) + 'static, _ctx: &mut AppContext) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    /// Returns a read-only `Var<O>` that uses a closure to generate its value from this `Var<T>` every time it changes.
    fn map<O: 'static, M: FnMut(&T) -> O + 'static>(&self, map: M) -> MapVar<T, Self, O, M>
    where
        Self: Sized;

    /// Bidirectional map. Returns a `Var<O>` that uses two closures to convert to and from this `Var<T>`.
    ///
    /// Unlike [map](Var::map) the returned variable is read-write when this variable is read-write.
    fn map_bidi<O: 'static, M: FnMut(&T) -> O + 'static, N: FnMut(&O) -> T + 'static>(
        &self,
        map: M,
        map_back: N,
    ) -> MapVarBiDi<T, Self, O, M, N>
    where
        Self: Sized;

    /// Ensures this variable is [always_read_only].
    fn as_read_only(self) -> Self::AsReadOnly;
}

/// A value-to-[var](Var) conversion that consumes the value.
pub trait IntoVar<T: 'static> {
    type Var: Var<T> + 'static;

    fn into_var(self) -> Self::Var;
}

/// A variable that can be one of many variables at a time, determined by
/// a its index.
#[allow(clippy::len_without_is_empty)]
pub trait SwitchVar<T: 'static>: Var<T> + protected::SwitchVar<T> {
    /// Current variable index.
    fn index(&self) -> usize;

    /// Number of variables that can be indexed.
    fn len(&self) -> usize;
}

// #endregion Traits

// #region Var<T> for ContextVar<Type=T>

impl<T: 'static, V: ContextVar<Type = T>> protected::Var<T> for V {
    fn bind_info<'a, 'b>(&'a self, _: &'b AppContext) -> protected::BindInfo<'a, T> {
        protected::BindInfo::ContextVar(std::any::TypeId::of::<V>(), V::default(), None)
    }

    fn is_context_var(&self) -> bool {
        true
    }
}

impl<T: 'static, V: ContextVar<Type = T>> ObjVar<T> for V {
    fn get<'a>(&'a self, ctx: &'a AppContext) -> &'a T {
        ctx.get::<V>()
    }

    fn update<'a>(&'a self, ctx: &'a AppContext) -> Option<&'a T> {
        ctx.get_new::<V>()
    }

    fn is_new(&self, ctx: &AppContext) -> bool {
        ctx.get_is_new::<V>()
    }

    fn version(&self, ctx: &AppContext) -> u32 {
        ctx.get_version::<V>()
    }
}

impl<T: 'static, V: ContextVar<Type = T>> Var<T> for V {
    type AsReadOnly = Self;

    fn map<O: 'static, M: FnMut(&T) -> O + 'static>(&self, map: M) -> MapVar<T, Self, O, M> {
        MapVar::new(MapVarInner::Context(MapContextVar::new(*self, map)))
    }

    fn map_bidi<O: 'static, M: FnMut(&T) -> O + 'static, N: FnMut(&O) -> T + 'static>(
        &self,
        map: M,
        _: N,
    ) -> MapVarBiDi<T, Self, O, M, N> {
        MapVarBiDi::new(MapVarBiDiInner::Context(MapContextVar::new(*self, map)))
    }

    fn as_read_only(self) -> Self {
        self
    }
}

// #endregion Var<T> for ContextVar<Type=T>

// #region OwnedVar<T>

/// [Var] implementer that owns the value.
pub struct OwnedVar<T: 'static>(pub T);

impl<T: 'static> protected::Var<T> for OwnedVar<T> {
    fn bind_info<'a, 'b>(&'a self, _: &'b AppContext) -> protected::BindInfo<'a, T> {
        protected::BindInfo::Var(&self.0, false, 0)
    }
}

impl<T: 'static> ObjVar<T> for OwnedVar<T> {
    fn get(&self, _: &AppContext) -> &T {
        &self.0
    }

    fn update<'a>(&'a self, _: &'a AppContext) -> Option<&'a T> {
        None
    }

    fn is_new(&self, _: &AppContext) -> bool {
        false
    }

    fn version(&self, _: &AppContext) -> u32 {
        0
    }
}

impl<T: 'static> Var<T> for OwnedVar<T> {
    type AsReadOnly = Self;

    fn map<O: 'static, M: FnMut(&T) -> O + 'static>(&self, mut map: M) -> MapVar<T, Self, O, M> {
        MapVar::new(MapVarInner::Owned(OwnedVar(map(&self.0))))
    }

    fn map_bidi<O: 'static, M: FnMut(&T) -> O + 'static, N: FnMut(&O) -> T + 'static>(
        &self,
        map: M,
        _: N,
    ) -> MapVarBiDi<T, Self, O, M, N> {
        MapVarBiDi::new(MapVarBiDiInner::Owned(OwnedVar(map(&self.0))))
    }

    fn as_read_only(self) -> Self {
        self
    }
}

impl<T: 'static> IntoVar<T> for OwnedVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

/// Wraps the value in an `[Owned]<T>` value.
impl<T: 'static> IntoVar<T> for T {
    type Var = OwnedVar<T>;

    fn into_var(self) -> OwnedVar<T> {
        OwnedVar(self)
    }
}

// #endregion OwnedVar<T>

// #region SharedVar<T>

struct SharedVarInner<T> {
    data: UnsafeCell<T>,
    context: AppContextOwnership,
    is_new: Cell<bool>,
    version: Cell<u32>,
}

/// A reference-counting [Var].
pub struct SharedVar<T: 'static> {
    r: Rc<SharedVarInner<T>>,
}

impl<T: 'static> SharedVar<T> {
    pub fn new(initial_value: T) -> Self {
        SharedVar {
            r: Rc::new(SharedVarInner {
                data: UnsafeCell::new(initial_value),
                context: AppContextOwnership::default(),
                is_new: Cell::new(false),
                version: Cell::new(0),
            }),
        }
    }

    pub(crate) fn modify(
        self,
        mut_ctx_id: AppContextId,
        modify: impl FnOnce(&mut T) + 'static,
        cleanup: &mut Vec<Box<dyn FnOnce()>>,
    ) {
        self.r.context.check(mut_ctx_id, || {
            format!(
                "cannot set `SharedVar<{}>` because it is bound to a different `AppContext`",
                type_name::<T>()
            )
        });

        // SAFETY: This is safe because borrows are bound to a context that
        // is the only place where the value can be changed and this change is
        // only applied when the context is mut.
        modify(unsafe { &mut *self.r.data.get() });
        self.r.version.set(self.next_version());

        cleanup.push(Box::new(move || self.r.is_new.set(false)));
    }

    fn borrow(&self, ctx_id: AppContextId) -> &T {
        self.r.context.check(ctx_id, || {
            format!(
                "cannot borrow `SharedVar<{}>` because it is bound to a different `AppContext`",
                type_name::<T>()
            )
        });

        // SAFETY: This is safe because borrows are bound to a context that
        // is the only place where the value can be changed and this change is
        // only applied when the context is mut.
        unsafe { &*self.r.data.get() }
    }

    /// Gets the [version] this variable will be in the next update if set in this update.
    pub fn next_version(&self) -> u32 {
        self.r.version.get().wrapping_add(1)
    }
}

impl<T: 'static> Clone for SharedVar<T> {
    fn clone(&self) -> Self {
        SharedVar { r: Rc::clone(&self.r) }
    }
}

impl<T: 'static> protected::Var<T> for SharedVar<T> {
    fn bind_info<'a, 'b>(&'a self, ctx: &'b AppContext) -> protected::BindInfo<'a, T> {
        protected::BindInfo::Var(self.borrow(ctx.id()), self.r.is_new.get(), self.r.version.get())
    }

    fn read_only_prev_version(&self) -> u32 {
        self.r.version.get().wrapping_sub(1)
    }
}

impl<T: 'static> ObjVar<T> for SharedVar<T> {
    fn get(&self, ctx: &AppContext) -> &T {
        self.borrow(ctx.id())
    }

    fn update<'a>(&'a self, ctx: &'a AppContext) -> Option<&'a T> {
        if self.r.is_new.get() {
            Some(self.get(ctx))
        } else {
            None
        }
    }

    fn is_new(&self, _: &AppContext) -> bool {
        self.r.is_new.get()
    }

    fn version(&self, _: &AppContext) -> u32 {
        self.r.version.get()
    }

    fn read_only(&self) -> bool {
        false
    }

    fn always_read_only(&self) -> bool {
        false
    }

    fn push_set(&self, new_value: T, ctx: &mut AppContext) -> Result<(), VarIsReadOnly> {
        let var = self.clone();
        let ctx_id = ctx.id();
        ctx.push_modify_impl(move |cleanup| {
            var.modify(ctx_id, move |v| *v = new_value, cleanup);
        });
        Ok(())
    }

    fn push_modify_boxed(
        &self,
        modify: Box<dyn FnOnce(&mut T) + 'static>,
        ctx: &mut AppContext,
    ) -> Result<(), VarIsReadOnly> {
        let var = self.clone();
        let ctx_id = ctx.id();
        ctx.push_modify_impl(move |cleanup| {
            var.modify(ctx_id, |v| modify(v), cleanup);
        });
        Ok(())
    }
}

impl<T: 'static> Var<T> for SharedVar<T> {
    type AsReadOnly = ReadOnlyVar<T, Self>;

    fn push_modify(&self, modify: impl FnOnce(&mut T) + 'static, ctx: &mut AppContext) -> Result<(), VarIsReadOnly> {
        let var = self.clone();
        let ctx_id = ctx.id();
        ctx.push_modify_impl(move |cleanup| {
            var.modify(ctx_id, modify, cleanup);
        });
        Ok(())
    }

    fn map<O: 'static, M: FnMut(&T) -> O + 'static>(&self, map: M) -> MapVar<T, Self, O, M> {
        MapVar::new(MapVarInner::Shared(MapSharedVar::new(
            self.clone(),
            map,
            self.r.version.get().wrapping_sub(1),
        )))
    }

    fn map_bidi<O: 'static, M: FnMut(&T) -> O + 'static, N: FnMut(&O) -> T + 'static>(
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
}

impl<T: 'static> IntoVar<T> for SharedVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

// #endregion SharedVar<T>

// #region ReadOnlyVar<T>

/// A variable that is [always_read_only](Var::always_read_only).
///
/// This `struct` is created by the [as_read_only](Var::as_read_only) method in variables
/// that are not `always_read_only`.
pub struct ReadOnlyVar<T: 'static, V: Var<T> + Clone> {
    _t: PhantomData<T>,
    var: V,
}

impl<T: 'static, V: Var<T> + Clone> ReadOnlyVar<T, V> {
    fn new(var: V) -> Self {
        ReadOnlyVar { _t: PhantomData, var }
    }
}

impl<T: 'static, V: Var<T> + Clone> protected::Var<T> for ReadOnlyVar<T, V> {
    fn bind_info<'a, 'b>(&'a self, ctx: &'b AppContext) -> protected::BindInfo<'a, T> {
        self.var.bind_info(ctx)
    }
}

impl<T: 'static, V: Var<T> + Clone> ObjVar<T> for ReadOnlyVar<T, V> {
    fn get<'a>(&'a self, ctx: &'a AppContext) -> &'a T {
        self.var.get(ctx)
    }

    /// [get] if [is_new] or none.
    fn update<'a>(&'a self, ctx: &'a AppContext) -> Option<&'a T> {
        self.var.update(ctx)
    }

    /// If the value changed this update.
    fn is_new(&self, ctx: &AppContext) -> bool {
        self.var.is_new(ctx)
    }

    /// Current value version. Version changes every time the value changes.
    fn version(&self, ctx: &AppContext) -> u32 {
        self.var.version(ctx)
    }
}

impl<T: 'static, V: Var<T> + Clone> Clone for ReadOnlyVar<T, V> {
    fn clone(&self) -> Self {
        ReadOnlyVar {
            _t: PhantomData,
            var: self.var.clone(),
        }
    }
}

impl<T: 'static, V: Var<T> + Clone> Var<T> for ReadOnlyVar<T, V> {
    type AsReadOnly = Self;

    fn map<O: 'static, M: FnMut(&T) -> O + 'static>(&self, map: M) -> MapVar<T, Self, O, M> {
        MapVar::new(MapVarInner::Shared(MapSharedVar::new(
            self.clone(),
            map,
            self.var.read_only_prev_version(),
        )))
    }

    fn map_bidi<O: 'static, M: FnMut(&T) -> O + 'static, N: FnMut(&O) -> T + 'static>(
        &self,
        map: M,
        map_back: N,
    ) -> MapVarBiDi<T, Self, O, M, N> {
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
}

// #endregion ReadOnlyVar<T>

// #region MapSharedVar<T> and MapBiDiSharedVar<T>

/// A read-only variable that maps the value of another variable.
struct MapSharedVar<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O> {
    _t: PhantomData<T>,
    source: S,
    map: RefCell<M>,
    output: UnsafeCell<MaybeUninit<O>>,
    output_version: Cell<u32>,
    context: AppContextOwnership,
}

/// A variable that maps the value of another variable.
struct MapBiDiSharedVar<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O, N: FnMut(&O) -> T> {
    _t: PhantomData<T>,
    source: S,
    map: RefCell<M>,
    map_back: RefCell<N>,
    output: UnsafeCell<MaybeUninit<O>>,
    output_version: Cell<u32>,
    context: AppContextOwnership,
}

impl<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O> MapSharedVar<T, S, O, M> {
    fn new(source: S, map: M, prev_version: u32) -> Self {
        MapSharedVar {
            _t: PhantomData,
            source,
            map: RefCell::new(map),
            output: UnsafeCell::new(MaybeUninit::uninit()),
            output_version: Cell::new(prev_version),
            context: AppContextOwnership::default(),
        }
    }

    fn borrow(&self, ctx: &AppContext) -> &O {
        self.context.check(ctx.id(), || {
            format!(
                "cannot borrow `MapVar<{} -> {}>` because it is already bound to a different `AppContext`",
                type_name::<T>(),
                type_name::<O>()
            )
        });

        let source_version = self.source.version(ctx);
        if self.output_version.get() != source_version {
            let value = (self.map.borrow_mut())(self.source.get(ctx));
            // SAFETY: This is safe because it only happens before the first borrow
            // of this update, and borrows cannot exist across updates because source
            // vars require a &mut AppContext for changing version.
            unsafe {
                let m_uninit = &mut *self.output.get();
                m_uninit.as_mut_ptr().write(value);
            }
            self.output_version.set(source_version);
        }

        // SAFETY:
        // borrow validation was done at the start of the method.
        // memory is initialized here because we start from the prev_version.
        unsafe {
            let inited = &*self.output.get();
            &*inited.as_ptr()
        }
    }
}

impl<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O, N: FnMut(&O) -> T> MapBiDiSharedVar<T, S, O, M, N> {
    fn new(source: S, map: M, map_back: N, prev_version: u32) -> Self {
        MapBiDiSharedVar {
            _t: PhantomData,
            source,
            map: RefCell::new(map),
            map_back: RefCell::new(map_back),
            output: UnsafeCell::new(MaybeUninit::uninit()),
            output_version: Cell::new(prev_version),
            context: AppContextOwnership::default(),
        }
    }

    fn borrow(&self, ctx: &AppContext) -> &O {
        self.context.check(ctx.id(), || {
            format!(
                "cannot borrow `MapVarBiDi<{} <-> {}>` because it is already bound to a different `AppContext`",
                type_name::<T>(),
                type_name::<O>()
            )
        });

        let source_version = self.source.version(ctx);
        if self.output_version.get() != source_version {
            let value = (self.map.borrow_mut())(self.source.get(ctx));
            // SAFETY: This is safe because it only happens before the first borrow
            // of this update, and borrows cannot exist across updates because source
            // vars require a &mut AppContext for changing version.
            unsafe {
                let m_uninit = &mut *self.output.get();
                m_uninit.as_mut_ptr().write(value);
            }
            self.output_version.set(source_version);
        }

        // SAFETY:
        // borrow validation was done at the start of the method.
        // memory is initialized here because we start from the prev_version.
        unsafe {
            let inited = &*self.output.get();
            &*inited.as_ptr()
        }
    }
}

impl<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O + 'static> protected::Var<O> for MapSharedVar<T, S, O, M> {
    fn bind_info<'a, 'b>(&'a self, ctx: &'b AppContext) -> protected::BindInfo<'a, O> {
        protected::BindInfo::Var(self.borrow(ctx), self.is_new(ctx), self.version(ctx))
    }
}

impl<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O + 'static, N: FnMut(&O) -> T + 'static> protected::Var<O>
    for MapBiDiSharedVar<T, S, O, M, N>
{
    fn bind_info<'a, 'b>(&'a self, ctx: &'b AppContext) -> protected::BindInfo<'a, O> {
        protected::BindInfo::Var(self.borrow(ctx), self.is_new(ctx), self.version(ctx))
    }

    fn read_only_prev_version(&self) -> u32 {
        self.output_version.get().wrapping_sub(1)
    }
}

impl<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O + 'static> ObjVar<O> for MapSharedVar<T, S, O, M> {
    fn get<'a>(&'a self, ctx: &'a AppContext) -> &'a O {
        self.borrow(ctx)
    }

    fn update<'a>(&'a self, ctx: &'a AppContext) -> Option<&'a O> {
        if self.is_new(ctx) {
            Some(self.borrow(ctx))
        } else {
            None
        }
    }

    fn is_new(&self, ctx: &AppContext) -> bool {
        self.source.is_new(ctx)
    }

    fn version(&self, ctx: &AppContext) -> u32 {
        self.source.version(ctx)
    }
}

impl<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O + 'static, N: FnMut(&O) -> T + 'static> ObjVar<O>
    for MapBiDiSharedVar<T, S, O, M, N>
{
    fn get<'a>(&'a self, ctx: &'a AppContext) -> &'a O {
        self.borrow(ctx)
    }

    fn update<'a>(&'a self, ctx: &'a AppContext) -> Option<&'a O> {
        if self.is_new(ctx) {
            Some(self.borrow(ctx))
        } else {
            None
        }
    }

    fn is_new(&self, ctx: &AppContext) -> bool {
        self.source.is_new(ctx)
    }

    fn version(&self, ctx: &AppContext) -> u32 {
        self.source.version(ctx)
    }

    fn read_only(&self) -> bool {
        self.source.read_only()
    }

    fn always_read_only(&self) -> bool {
        self.source.always_read_only()
    }

    fn push_set(&self, new_value: O, ctx: &mut AppContext) -> Result<(), VarIsReadOnly> {
        self.source.push_set((self.map_back.borrow_mut())(&new_value), ctx);
        todo!()
    }

    fn push_modify_boxed(
        &self,
        _modify: Box<dyn FnOnce(&mut O) + 'static>,
        _ctx: &mut AppContext,
    ) -> Result<(), VarIsReadOnly> {
        todo!()
    }
}

impl<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O + 'static> Clone for MapSharedVar<T, S, O, M> {
    fn clone(&self) -> Self {
        todo!("when GATs are stable")
    }
}

impl<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O + 'static, N: FnMut(&O) -> T + 'static> Clone
    for MapBiDiSharedVar<T, S, O, M, N>
{
    fn clone(&self) -> Self {
        todo!("when GATs are stable")
    }
}

impl<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O + 'static> Var<O> for MapSharedVar<T, S, O, M> {
    type AsReadOnly = Self;

    fn map<O2: 'static, M2: FnMut(&O) -> O2 + 'static>(&self, map: M2) -> MapVar<O, Self, O2, M2> {
        MapVar::new(MapVarInner::Shared(MapSharedVar::new(
            self.clone(),
            map,
            self.output_version.get().wrapping_sub(1),
        )))
    }

    fn map_bidi<O2: 'static, M2: FnMut(&O) -> O2 + 'static, N2: FnMut(&O2) -> O + 'static>(
        &self,
        map: M2,
        map_back: N2,
    ) -> MapVarBiDi<O, Self, O2, M2, N2> {
        MapVarBiDi::new(MapVarBiDiInner::Shared(MapBiDiSharedVar::new(
            self.clone(),
            map,
            map_back,
            self.output_version.get().wrapping_sub(1),
        )))
    }

    fn as_read_only(self) -> Self {
        self
    }
}

impl<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O + 'static, N: FnMut(&O) -> T + 'static> Var<O>
    for MapBiDiSharedVar<T, S, O, M, N>
{
    type AsReadOnly = ReadOnlyVar<O, Self>;

    fn push_modify(&self, _modify: impl FnOnce(&mut O) + 'static, _ctx: &mut AppContext) -> Result<(), VarIsReadOnly> {
        todo!()
    }

    fn map<O2: 'static, M2: FnMut(&O) -> O2 + 'static>(&self, map: M2) -> MapVar<O, Self, O2, M2> {
        MapVar::new(MapVarInner::Shared(MapSharedVar::new(
            self.clone(),
            map,
            self.output_version.get().wrapping_sub(1),
        )))
    }

    fn map_bidi<O2: 'static, M2: FnMut(&O) -> O2 + 'static, N2: FnMut(&O2) -> O + 'static>(
        &self,
        map: M2,
        map_back: N2,
    ) -> MapVarBiDi<O, Self, O2, M2, N2> {
        MapVarBiDi::new(MapVarBiDiInner::Shared(MapBiDiSharedVar::new(
            self.clone(),
            map,
            map_back,
            self.output_version.get().wrapping_sub(1),
        )))
    }

    fn as_read_only(self) -> Self::AsReadOnly {
        ReadOnlyVar::new(self)
    }
}

impl<T: 'static, O: 'static, M: FnMut(&T) -> O + 'static, S: ObjVar<T>> IntoVar<O> for MapSharedVar<T, S, O, M> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T: 'static, O: 'static, S: ObjVar<T>, M: FnMut(&T) -> O + 'static, N: FnMut(&O) -> T + 'static> IntoVar<O>
    for MapBiDiSharedVar<T, S, O, M, N>
{
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

// #endregion MapSharedVar<T> and MapBiDiSharedVar<T>

// #region MapContextVar<T>

type MapContextVarOutputs<O> = FnvHashMap<Option<WidgetId>, (UnsafeCell<O>, u32)>;

/// A variable that maps the value of a context variable.
struct MapContextVar<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O> {
    _t: PhantomData<T>,
    source: S,
    map: RefCell<M>,
    outputs: RefCell<MapContextVarOutputs<O>>,
    context: AppContextOwnership,
}

impl<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O> MapContextVar<T, S, O, M> {
    fn new(source: S, map: M) -> Self {
        MapContextVar {
            _t: PhantomData,
            source,
            map: RefCell::new(map),
            outputs: RefCell::default(),
            context: AppContextOwnership::default(),
        }
    }

    fn borrow(&self, ctx: &AppContext) -> &O {
        self.context.check(ctx.id(), || {
            format!(
                "cannot borrow `MapVar<{}>` because it is already bound to a different `AppContext`",
                type_name::<T>()
            )
        });

        use std::collections::hash_map::Entry::{Occupied, Vacant};
        let mut outputs = self.outputs.borrow_mut();
        let widget_id = ctx.try_widget_id();
        let source_version = self.source.version(ctx);

        let output = match outputs.entry(widget_id) {
            Occupied(entry) => {
                let (output, output_version) = entry.into_mut();
                if *output_version != source_version {
                    let value = (self.map.borrow_mut())(self.source.get(ctx));
                    // TODO UNSAFE: Same context var can be set twice in same widget.

                    // SAFETY: This is safe because it only happens before the first borrow
                    // of this update.
                    unsafe { *output.get() = value }
                    *output_version = source_version;
                }
                output
            }
            Vacant(entry) => {
                let value = (self.map.borrow_mut())(self.source.get(ctx));
                let (output, _) = entry.insert((UnsafeCell::new(value), source_version));
                output
            }
        };

        // SAFETY:
        // borrow validation was done at the start of the method.
        unsafe { &*output.get() }
    }
}

impl<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O> protected::Var<O> for MapContextVar<T, S, O, M> {
    fn bind_info<'a, 'b>(&'a self, ctx: &'b AppContext) -> protected::BindInfo<'a, O> {
        protected::BindInfo::Var(self.borrow(ctx), self.source.is_new(ctx), self.source.version(ctx))
    }
}

impl<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O + 'static> ObjVar<O> for MapContextVar<T, S, O, M> {
    fn get<'a>(&'a self, ctx: &'a AppContext) -> &'a O {
        self.borrow(ctx)
    }

    fn update<'a>(&'a self, ctx: &'a AppContext) -> Option<&'a O> {
        if self.is_new(ctx) {
            Some(self.borrow(ctx))
        } else {
            None
        }
    }

    fn is_new(&self, ctx: &AppContext) -> bool {
        self.source.is_new(ctx)
    }

    fn version(&self, ctx: &AppContext) -> u32 {
        self.source.version(ctx)
    }
}

impl<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O + 'static> Var<O> for MapContextVar<T, S, O, M> {
    type AsReadOnly = Self;

    fn map<O2: 'static, M2: FnMut(&O) -> O2 + 'static>(&self, _map: M2) -> MapVar<O, Self, O2, M2> {
        todo!("when GATs are stable")
    }

    fn map_bidi<O2: 'static, M2: FnMut(&O) -> O2 + 'static, N2: FnMut(&O2) -> O + 'static>(
        &self,
        map: M2,
        map_back: N2,
    ) -> MapVarBiDi<O, Self, O2, M2, N2> {
        todo!("when GATs are stable")
    }

    fn as_read_only(self) -> Self {
        self
    }
}

impl<T: 'static, O: 'static, M: FnMut(&T) -> O + 'static, S: ObjVar<T>> IntoVar<O> for MapContextVar<T, S, O, M> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T: 'static, O: 'static, M: FnMut(&T) -> O + 'static, S: ObjVar<T>> IntoVar<O> for MapVar<T, S, O, M> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

// #endregion MapContextVar<T>

// #region MapVar<T> and MapVarBidi<T>

enum MapVarInner<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O> {
    Owned(OwnedVar<O>),
    Shared(MapSharedVar<T, S, O, M>),
    Context(MapContextVar<T, S, O, M>),
}

enum MapVarBiDiInner<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O, N: FnMut(&O) -> T> {
    Owned(OwnedVar<O>),
    Shared(MapBiDiSharedVar<T, S, O, M, N>),
    Context(MapContextVar<T, S, O, M>),
}

/// A variable that maps the value of another variable.
///
/// This `struct` is created by the [map](Var::map) method and is a temporary adapter until
/// [GATs](https://github.com/rust-lang/rust/issues/44265) are stable.
pub struct MapVar<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O> {
    r: Rc<MapVarInner<T, S, O, M>>,
}

/// A variable that maps from and to another variable.
///
/// This `struct` is created by the [map_bidi](Var::map_bidi) method and is a temporary adapter until
/// [GATs](https://github.com/rust-lang/rust/issues/44265) are stable.
pub struct MapVarBiDi<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O, N: FnMut(&O) -> T> {
    r: Rc<MapVarBiDiInner<T, S, O, M, N>>,
}

impl<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O> MapVar<T, S, O, M> {
    fn new(inner: MapVarInner<T, S, O, M>) -> Self {
        MapVar { r: Rc::new(inner) }
    }
}

impl<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O, N: FnMut(&O) -> T> MapVarBiDi<T, S, O, M, N> {
    fn new(inner: MapVarBiDiInner<T, S, O, M, N>) -> Self {
        MapVarBiDi { r: Rc::new(inner) }
    }
}

impl<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O + 'static> protected::Var<O> for MapVar<T, S, O, M> {
    fn bind_info<'a, 'b>(&'a self, ctx: &'b AppContext) -> protected::BindInfo<'a, O> {
        match &*self.r {
            MapVarInner::Owned(o) => o.bind_info(ctx),
            MapVarInner::Shared(s) => s.bind_info(ctx),
            MapVarInner::Context(c) => c.bind_info(ctx),
        }
    }
}

impl<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O + 'static, N: FnMut(&O) -> T + 'static> protected::Var<O>
    for MapVarBiDi<T, S, O, M, N>
{
    fn bind_info<'a, 'b>(&'a self, ctx: &'b AppContext) -> protected::BindInfo<'a, O> {
        match &*self.r {
            MapVarBiDiInner::Owned(o) => o.bind_info(ctx),
            MapVarBiDiInner::Shared(s) => s.bind_info(ctx),
            MapVarBiDiInner::Context(c) => c.bind_info(ctx),
        }
    }

    fn read_only_prev_version(&self) -> u32 {
        todo!()
    }
}

impl<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O + 'static> ObjVar<O> for MapVar<T, S, O, M> {
    fn get<'a>(&'a self, ctx: &'a AppContext) -> &'a O {
        match &*self.r {
            MapVarInner::Owned(o) => o.get(ctx),
            MapVarInner::Shared(s) => s.get(ctx),
            MapVarInner::Context(c) => c.get(ctx),
        }
    }

    fn update<'a>(&'a self, ctx: &'a AppContext) -> Option<&'a O> {
        match &*self.r {
            MapVarInner::Owned(o) => o.update(ctx),
            MapVarInner::Shared(s) => s.update(ctx),
            MapVarInner::Context(c) => c.update(ctx),
        }
    }

    fn is_new(&self, ctx: &AppContext) -> bool {
        match &*self.r {
            MapVarInner::Owned(o) => o.is_new(ctx),
            MapVarInner::Shared(s) => s.is_new(ctx),
            MapVarInner::Context(c) => c.is_new(ctx),
        }
    }

    fn version(&self, ctx: &AppContext) -> u32 {
        match &*self.r {
            MapVarInner::Owned(o) => o.version(ctx),
            MapVarInner::Shared(s) => s.version(ctx),
            MapVarInner::Context(c) => c.version(ctx),
        }
    }
}

impl<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O + 'static, N: FnMut(&O) -> T + 'static> ObjVar<O>
    for MapVarBiDi<T, S, O, M, N>
{
    fn get<'a>(&'a self, ctx: &'a AppContext) -> &'a O {
        match &*self.r {
            MapVarBiDiInner::Owned(o) => o.get(ctx),
            MapVarBiDiInner::Shared(s) => s.get(ctx),
            MapVarBiDiInner::Context(c) => c.get(ctx),
        }
    }

    fn update<'a>(&'a self, ctx: &'a AppContext) -> Option<&'a O> {
        match &*self.r {
            MapVarBiDiInner::Owned(o) => o.update(ctx),
            MapVarBiDiInner::Shared(s) => s.update(ctx),
            MapVarBiDiInner::Context(c) => c.update(ctx),
        }
    }

    fn is_new(&self, ctx: &AppContext) -> bool {
        match &*self.r {
            MapVarBiDiInner::Owned(o) => o.is_new(ctx),
            MapVarBiDiInner::Shared(s) => s.is_new(ctx),
            MapVarBiDiInner::Context(c) => c.is_new(ctx),
        }
    }

    fn version(&self, ctx: &AppContext) -> u32 {
        match &*self.r {
            MapVarBiDiInner::Owned(o) => o.version(ctx),
            MapVarBiDiInner::Shared(s) => s.version(ctx),
            MapVarBiDiInner::Context(c) => c.version(ctx),
        }
    }

    fn read_only(&self) -> bool {
        match &*self.r {
            MapVarBiDiInner::Owned(o) => o.read_only(),
            MapVarBiDiInner::Shared(s) => s.read_only(),
            MapVarBiDiInner::Context(c) => c.read_only(),
        }
    }

    fn always_read_only(&self) -> bool {
        match &*self.r {
            MapVarBiDiInner::Owned(o) => o.always_read_only(),
            MapVarBiDiInner::Shared(s) => s.always_read_only(),
            MapVarBiDiInner::Context(c) => c.always_read_only(),
        }
    }

    fn push_set(&self, new_value: O, ctx: &mut AppContext) -> Result<(), VarIsReadOnly> {
        match &*self.r {
            MapVarBiDiInner::Owned(o) => o.push_set(new_value, ctx),
            MapVarBiDiInner::Shared(s) => s.push_set(new_value, ctx),
            MapVarBiDiInner::Context(c) => c.push_set(new_value, ctx),
        }
    }

    fn push_modify_boxed(
        &self,
        modify: Box<dyn FnOnce(&mut O) + 'static>,
        ctx: &mut AppContext,
    ) -> Result<(), VarIsReadOnly> {
        match &*self.r {
            MapVarBiDiInner::Owned(o) => o.push_modify_boxed(modify, ctx),
            MapVarBiDiInner::Shared(s) => s.push_modify_boxed(modify, ctx),
            MapVarBiDiInner::Context(c) => c.push_modify_boxed(modify, ctx),
        }
    }
}

impl<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O + 'static> Clone for MapVar<T, S, O, M> {
    fn clone(&self) -> Self {
        MapVar { r: Rc::clone(&self.r) }
    }
}

impl<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O + 'static, N: FnMut(&O) -> T + 'static> Clone
    for MapVarBiDi<T, S, O, M, N>
{
    fn clone(&self) -> Self {
        MapVarBiDi { r: Rc::clone(&self.r) }
    }
}

impl<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O + 'static> Var<O> for MapVar<T, S, O, M> {
    type AsReadOnly = Self;

    fn map<O2: 'static, M2: FnMut(&O) -> O2 + 'static>(&self, map: M2) -> MapVar<O, Self, O2, M2> {
        MapVar::new(MapVarInner::Shared(MapSharedVar::new(self.clone(), map, 0)))
        // TODO prev_version?
    }

    fn map_bidi<O2: 'static, M2: FnMut(&O) -> O2 + 'static, N2: FnMut(&O2) -> O + 'static>(
        &self,
        map: M2,
        map_back: N2,
    ) -> MapVarBiDi<O, Self, O2, M2, N2> {
        MapVarBiDi::new(MapVarBiDiInner::Shared(MapBiDiSharedVar::new(
            self.clone(),
            map,
            map_back,
            0,
        )))
    }

    fn as_read_only(self) -> Self {
        self
    }
}

impl<T: 'static, S: ObjVar<T>, O: 'static, M: FnMut(&T) -> O + 'static, N: FnMut(&O) -> T + 'static> Var<O>
    for MapVarBiDi<T, S, O, M, N>
{
    type AsReadOnly = ReadOnlyVar<O, Self>;

    fn push_modify(&self, modify: impl FnOnce(&mut O) + 'static, ctx: &mut AppContext) -> Result<(), VarIsReadOnly> {
        match &*self.r {
            MapVarBiDiInner::Owned(o) => o.push_modify(modify, ctx),
            MapVarBiDiInner::Shared(s) => s.push_modify(modify, ctx),
            MapVarBiDiInner::Context(c) => c.push_modify(modify, ctx),
        }
    }

    fn map<O2: 'static, M2: FnMut(&O) -> O2 + 'static>(&self, map: M2) -> MapVar<O, Self, O2, M2> {
        MapVar::new(MapVarInner::Shared(MapSharedVar::new(self.clone(), map, 0)))
        // TODO prev_version?
    }

    fn map_bidi<O2: 'static, M2: FnMut(&O) -> O2 + 'static, N2: FnMut(&O2) -> O + 'static>(
        &self,
        map: M2,
        map_back: N2,
    ) -> MapVarBiDi<O, Self, O2, M2, N2> {
        MapVarBiDi::new(MapVarBiDiInner::Shared(MapBiDiSharedVar::new(
            self.clone(),
            map,
            map_back,
            0,
        )))
    }

    fn as_read_only(self) -> Self::AsReadOnly {
        ReadOnlyVar::new(self)
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
        struct $SwitchVarInner<T: 'static, $($VN: Var<T>),+> {
            _t: PhantomData<T>,
            $($vn: $VN,)+

            index: Cell<u8>,

            $($version: Cell<u32>,)+

            version: Cell<u32>,
            is_new: Cell<bool>
        }

        /// A fixed-size set of variables that can be switched on. See [switch_var!] for
        /// the full documentation.
        pub struct $SwitchVar<T: 'static, $($VN: Var<T>),+> {
            r: Rc<$SwitchVarInner<T, $($VN),+>>,
        }

        impl<T: 'static, $($VN: Var<T>),+> $SwitchVar<T, $($VN),+> {
            #[allow(clippy::too_many_arguments)]
            pub fn new(index: u8, $($vn: $VN),+) -> Self {
                assert!(index < $N);
                $SwitchVar {
                    r: Rc::new($SwitchVarInner {
                        _t: PhantomData,
                        index: Cell::new(index),
                        $($version: Cell::new(0),)+
                        version: Cell::new(0),
                        is_new: Cell::new(false),
                        $($vn,)+
                    })
                }
            }
        }

        impl<T: 'static, $($VN: Var<T>),+> protected::Var<T> for $SwitchVar<T, $($VN),+> {
            fn bind_info<'a, 'b>(&'a self, ctx: &'b AppContext) -> protected::BindInfo<'a, T> {
                let is_new = self.is_new(ctx);
                let version = self.version(ctx);
                let inner_info = match self.r.index.get() {
                    $($n => self.r.$vn.bind_info(ctx),)+
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

        impl<T: 'static, $($VN: Var<T>),+> ObjVar<T> for $SwitchVar<T, $($VN),+> {
            fn get<'a>(&'a self, ctx: &'a AppContext) -> &'a T {
                match self.r.index.get() {
                    $($n => self.r.$vn.get(ctx),)+
                    _ => unreachable!(),
                }
            }

            fn update<'a>(&'a self, ctx: &'a AppContext) -> Option<&'a T> {
                if self.r.is_new.get() {
                    Some(self.get(ctx))
                } else {
                    match self.r.index.get() {
                        $($n => self.r.$vn.update(ctx),)+
                        _ => unreachable!(),
                    }
                }
            }

            fn is_new(&self, ctx: &AppContext) -> bool {
                self.r.is_new.get()
                    || match self.r.index.get() {
                        $($n => self.r.$vn.is_new(ctx),)+
                        _ => unreachable!(),
                    }
            }

            fn version(&self, ctx: &AppContext) -> u32 {
                match self.r.index.get() {
                    $($n => {
                        let $version = self.r.$vn.version(ctx);
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

            fn push_set(&self, new_value: T, ctx: &mut AppContext) -> Result<(), VarIsReadOnly> {
                match self.r.index.get() {
                    $($n => self.r.$vn.push_set(new_value, ctx),)+
                    _ => unreachable!(),
                }
            }

            fn push_modify_boxed(&self, modify: Box<dyn FnOnce(&mut T) + 'static>, ctx: &mut AppContext) -> Result<(), VarIsReadOnly> {
                match self.r.index.get() {
                    $($n => self.r.$vn.push_modify_boxed(modify, ctx),)+
                    _ => unreachable!(),
                }
            }
        }

        impl<T: 'static, $($VN: Var<T>),+> Clone for $SwitchVar<T, $($VN),+> {
            fn clone(&self) -> Self {
                $SwitchVar { r: Rc::clone(&self.r) }
            }
        }

        impl<T: 'static, $($VN: Var<T>),+> Var<T> for $SwitchVar<T, $($VN),+> {
            type AsReadOnly = ReadOnlyVar<T, Self>;

            fn push_modify(&self, modify: impl FnOnce(&mut T) + 'static, ctx: &mut AppContext) -> Result<(), VarIsReadOnly> {
                match self.r.index.get() {
                    $($n => self.r.$vn.push_modify(modify, ctx),)+
                    _ => unreachable!(),
                }
            }

            fn map<O: 'static, M: FnMut(&T) -> O + 'static>(&self, map: M) -> MapVar<T, Self, O, M> {
                MapVar::new(MapVarInner::Shared(MapSharedVar::new(
                    self.clone(),
                    map,
                    self.r.version.get().wrapping_sub(1),
                )))
            }

            fn map_bidi<O: 'static, M: FnMut(&T) -> O + 'static, N: FnMut(&O) -> T + 'static>(
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
        }

        impl<T: 'static, $($VN: Var<T>),+> protected::SwitchVar<T> for $SwitchVar<T, $($VN),+> {
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

        impl<T: 'static, $($VN: Var<T>),+> SwitchVar<T> for $SwitchVar<T, $($VN),+> {
            fn index(&self) -> usize {
                self.r.index.get() as usize
            }

            fn len(&self) -> usize {
                $N
            }
        }

        impl<T: 'static, $($VN: Var<T>),+> IntoVar<T> for $SwitchVar<T, $($VN),+> {
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
pub struct SwitchVarDyn<T: 'static> {
    r: Rc<SwitchVarDynInner<T>>,
}

impl<T: 'static> SwitchVarDyn<T> {
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

impl<T: 'static> protected::Var<T> for SwitchVarDyn<T> {
    fn bind_info<'a, 'b>(&'a self, ctx: &'b AppContext) -> protected::BindInfo<'a, T> {
        let is_new = self.is_new(ctx);
        let version = self.version(ctx);
        let inner_info = self.r.vars[self.r.index.get()].bind_info(ctx);

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

impl<T: 'static> ObjVar<T> for SwitchVarDyn<T> {
    fn get<'a>(&'a self, ctx: &'a AppContext) -> &'a T {
        self.r.vars[self.r.index.get()].get(ctx)
    }

    fn update<'a>(&'a self, ctx: &'a AppContext) -> Option<&'a T> {
        if self.r.is_new.get() {
            Some(self.get(ctx))
        } else {
            self.r.vars[self.r.index.get()].update(ctx)
        }
    }

    fn is_new(&self, ctx: &AppContext) -> bool {
        self.r.is_new.get() || self.r.vars[self.r.index.get()].is_new(ctx)
    }

    fn version(&self, ctx: &AppContext) -> u32 {
        let index = self.r.index.get();
        let version = self.r.vars[index].version(ctx);
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

    fn push_set(&self, new_value: T, ctx: &mut AppContext) -> Result<(), VarIsReadOnly> {
        self.r.vars[self.r.index.get()].push_set(new_value, ctx)
    }

    fn push_modify_boxed(&self, modify: Box<dyn FnOnce(&mut T)>, ctx: &mut AppContext) -> Result<(), VarIsReadOnly> {
        self.r.vars[self.r.index.get()].push_modify_boxed(modify, ctx)
    }
}

impl<T: 'static> Clone for SwitchVarDyn<T> {
    fn clone(&self) -> Self {
        SwitchVarDyn { r: Rc::clone(&self.r) }
    }
}

impl<T: 'static> Var<T> for SwitchVarDyn<T> {
    type AsReadOnly = ReadOnlyVar<T, Self>;

    fn push_modify(&self, modify: impl FnOnce(&mut T) + 'static, ctx: &mut AppContext) -> Result<(), VarIsReadOnly> {
        self.push_modify_boxed(Box::new(modify), ctx)
    }

    fn map<O: 'static, M: FnMut(&T) -> O + 'static>(&self, map: M) -> MapVar<T, Self, O, M> {
        MapVar::new(MapVarInner::Shared(MapSharedVar::new(
            self.clone(),
            map,
            self.r.version.get().wrapping_sub(1),
        )))
    }

    fn map_bidi<O: 'static, M: FnMut(&T) -> O + 'static, N: FnMut(&O) -> T + 'static>(
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
}

impl<T: 'static> protected::SwitchVar<T> for SwitchVarDyn<T> {
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

impl<T: 'static> SwitchVar<T> for SwitchVarDyn<T> {
    fn index(&self) -> usize {
        self.r.index.get() as usize
    }

    fn len(&self) -> usize {
        self.r.vars.len()
    }
}

impl<T: 'static> IntoVar<T> for SwitchVarDyn<T> {
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
        struct $MergeVarInner<$($TN: 'static,)+ $($VN: Var<$TN>,)+ O: 'static, M: FnMut($(&$TN),+) -> O> {
            $($_t: PhantomData<$TN>,)+
            $($vn: $VN,)+
            $($version: Cell<u32>,)+
            merge: RefCell<M>,
            output: UnsafeCell<MaybeUninit<O>>,
            version: Cell<u32>,
            context: AppContextOwnership
        }

        pub struct $MergeVar<$($TN: 'static,)+ $($VN: Var<$TN>,)+ O: 'static, M: FnMut($(&$TN),+) -> O> {
            r: Rc<$MergeVarInner<$($TN,)+ $($VN,)+ O, M>>
        }

        impl<$($TN: 'static,)+ $($VN: Var<$TN>,)+ O: 'static, M: FnMut($(&$TN),+) -> O> $MergeVar<$($TN,)+ $($VN,)+ O, M> {
            pub fn new($($vn: $VN,)+ merge: M) -> Self {
                $MergeVar {
                    r: Rc::new($MergeVarInner {
                        $($_t: PhantomData,)+
                        $($version: Cell::new(0),)+ // TODO prev_version
                        $($vn,)+
                        merge: RefCell::new(merge),
                        output: UnsafeCell::new(MaybeUninit::uninit()),
                        version: Cell::new(0),
                        context: AppContextOwnership::default(),
                    })
                }
            }

            fn sync(&self, ctx: &AppContext) {
                self.r.context.check(
                    ctx.id(),
                    ||format!(
                        "cannot borrow `{}<({}) -> {}>` because it is already bound to a different `AppContext`",
                        stringify!($MergeVar),
                        vec![$(type_name::<$TN>()),+].join(", "),
                        type_name::<O>(),
                    ),
                );

                let mut sync = false;

                $(
                    let version = self.r.$vn.version(ctx);
                    if version != self.r.$version.get() {
                        sync = true;
                        self.r.$version.set(version);
                    }
                )+

                if sync {
                    self.r.version.set(self.r.version.get().wrapping_add(1));
                    let value = (self.r.merge.borrow_mut())($(self.r.$vn.get(ctx)),+);

                    // SAFETY: This is safe because it only happens before the first borrow
                    // of this update, and borrows cannot exist across updates because source
                    // vars require a &mut AppContext for changing version.
                    unsafe {
                        let m_uninit = &mut *self.r.output.get();
                        m_uninit.as_mut_ptr().write(value);
                    }
                }
            }

            fn borrow(&self, ctx: &AppContext) -> &O {
                self.sync(ctx);
                // SAFETY:
                // borrow validation was done in sync.
                // memory is initialized here because we start from the prev_version.
                unsafe {
                    let inited = &*self.r.output.get();
                    &*inited.as_ptr()
                }
            }

            fn any_is_new(&self, ctx: &AppContext) -> bool {
                 $(self.r.$vn.is_new(ctx))||+
            }
        }

        impl<$($TN: 'static,)+ $($VN: Var<$TN>,)+ O: 'static, M: FnMut($(&$TN),+) -> O> Clone for $MergeVar<$($TN,)+ $($VN,)+ O, M> {
            fn clone(&self) -> Self {
                $MergeVar { r: Rc::clone(&self.r) }
            }
        }

        impl<$($TN: 'static,)+ $($VN: Var<$TN>,)+ O: 'static, M: FnMut($(&$TN),+) -> O> protected::Var<O> for $MergeVar<$($TN,)+ $($VN,)+ O, M> {
            fn bind_info<'a, 'b>(&'a self, ctx: &'b AppContext) -> protected::BindInfo<'a, O> {
                protected::BindInfo::Var(self.borrow(ctx), self.any_is_new(ctx), self.r.version.get())
            }
        }

        impl<$($TN: 'static,)+ $($VN: Var<$TN>,)+ O: 'static, M: FnMut($(&$TN),+) -> O + 'static> ObjVar<O> for $MergeVar<$($TN,)+ $($VN,)+ O, M> {
            fn get<'a>(&'a self, ctx: &'a AppContext) -> &'a O {
                self.borrow(ctx)
            }

            fn update<'a>(&'a self, ctx: &'a AppContext) -> Option<&'a O> {
                if self.any_is_new(ctx) {
                    Some(self.borrow(ctx))
                } else {
                    None
                }
            }

            fn is_new(&self, ctx: &AppContext) -> bool {
                self.any_is_new(ctx)
            }

            fn version(&self, ctx: &AppContext) -> u32 {
                self.sync(ctx);
                self.r.version.get()
            }
        }

        impl<$($TN: 'static,)+ $($VN: Var<$TN>,)+ O: 'static, M: FnMut($(&$TN),+) -> O + 'static> Var<O> for $MergeVar<$($TN,)+ $($VN,)+ O, M> {
            type AsReadOnly = Self;

            fn map<O2: 'static, M2: FnMut(&O) -> O2 + 'static>(&self, map: M2) -> MapVar<O, Self, O2, M2> {
                MapVar::new(MapVarInner::Shared(MapSharedVar::new(
                    self.clone(),
                    map,
                    self.r.version.get().wrapping_sub(1),
                )))
            }

            fn map_bidi<O2: 'static, M2: FnMut(&O) -> O2 + 'static, N: FnMut(&O2) -> O + 'static>(
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
        }

        impl<$($TN: 'static,)+ $($VN: Var<$TN>,)+ O: 'static, M: FnMut($(&$TN),+) -> O + 'static> IntoVar<O> for $MergeVar<$($TN,)+ $($VN,)+ O, M> {
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

/// Initializes a new `[SharedVar]`.
pub fn var<T>(initial_value: T) -> SharedVar<T> {
    SharedVar::new(initial_value)
}

/// Initializes a new `[SwitchVar]`.
///
/// # Arguments
///
/// All arguments are separated by comma like a function call.
///
/// * `index`: A positive integer that is the initial switch index.
/// * `var0..N`: A list of [vars](ObjVar), minimal 2, [SwitchVarDyn] is used for more then 8 variables.
///
/// # Example
/// ```
/// let var0 = var("Read-write");
/// let var1 = "Read-only";
///
/// let switch_var = switch_var!(0, var0, var1);
/// ```
#[macro_export]
macro_rules! switch_var {
    ($index: expr, $v0: expr, $v1: expr) => {
        $crate::core2::SwitchVar2::new($index, $v0, $v1)
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr) => {
        $crate::core2::SwitchVar3::new($index, $v0, $v1, $v2)
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr) => {
        $crate::core2::SwitchVar4::new($index, $v0, $v1, $v2)
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr) => {
        $crate::core2::SwitchVar5::new($index, $v0, $v1, $v2, $v4)
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr) => {
        $crate::core2::SwitchVar6::new($index, $v0, $v1, $v2, $v4, $v5)
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr) => {
        $crate::core2::SwitchVar7::new($index, $v0, $v1, $v2, $v4, $v5, $v6)
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr) => {
        $crate::core2::SwitchVar8::new($index, $v0, $v1, $v2, $v4, $v5, $v6, $v7)
    };
    ($index: expr, $($v:expr),+) => {
        $crate::core2::SwitchVarDyn::new($index, vec![$($v.into_box()),+])
    };
    ($($_:tt)*) => {
        compile_error!("this macro takes 3 or more parameters (initial_index, var0, var1, ..)")
    };
}

/// Initializes a merge var.
///
/// # Arguments
///
/// All arguments are separated by comma like a function call.
///
/// * `var0..N`: A list of [vars](Var), minimal 2.
/// * `merge`: A function that produces a new value from references to all variable values. `FnMut(&var0_T, ..) -> merge_T`
///
/// # Example
/// ```
/// let var0 = var("Hello");
/// let var1 = "World";
///
/// let merge_var = merge_var!(var0, var1, |a, b|format!("{} {}!", a, b));
///
/// assert_eq!("Hello World!", merge_var.get(ctx));
/// ```
#[macro_export]
macro_rules! merge_var {
    ($v0: expr, $v1: expr, $merge: expr) => {
        $crate::core2::MergeVar2::new($v0, $v1, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $merge: expr) => {
        $crate::core2::MergeVar3::new($v0, $v1, $v2, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $merge: expr) => {
        $crate::core2::MergeVar4::new($v0, $v1, $v2, $v3, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $merge: expr) => {
        $crate::core2::MergeVar5::new($v0, $v1, $v2, $v3, $v4, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $merge: expr) => {
        $crate::core2::MergeVar6::new($v0, $v1, $v2, $v3, $v4, $v5, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $merge: expr) => {
        $crate::core2::MergeVar7::new($v0, $v1, $v2, $v3, $v4, $v5, $v6, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $merge: expr) => {
        $crate::core2::MergeVar8::new($v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $($more_args:ident),+) => {
        compile_error!("merge_var is only implemented to a maximum of 8 variables")
    };
    ($($_:tt)*) => {
        compile_error!("this macro takes 3 or more parameters (var0, var1, .., merge_fn")
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestApp {
        ctx: AppContext,
    }

    impl TestApp {
        fn new() -> Self {
            todo!()
        }

        fn update<R>(&mut self, f: impl FnOnce(&mut AppContext) -> R) -> R {
            f(&mut self.ctx)
        }

        fn finish_update(&mut self) {
            todo!()
        }
    }

    #[test]
    fn owned_var_get() {
        let var: OwnedVar<&'static str> = "value".into_var();

        let mut test = TestApp::new();

        let value = test.update(move |ctx| *var.get(ctx));

        assert_eq!("value", value)
    }

    #[test]
    fn owned_var_set() {
        let var = OwnedVar("value");

        assert!(var.always_read_only());
        assert!(var.read_only());

        let mut test = TestApp::new();

        test.update(move |ctx| {
            assert_eq!(0, var.version(ctx));
            assert!(!var.is_new(ctx));

            assert!(var.push_set("new value", ctx).is_err());
        });
    }

    #[test]
    fn shared_var_get() {
        todo!()
    }

    #[test]
    fn shared_var_set() {
        todo!()
    }
}
