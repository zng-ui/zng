use std::{
    any::TypeId,
    cell::RefCell,
    cell::{Cell, UnsafeCell},
    fmt::Debug,
    marker::PhantomData,
    mem::MaybeUninit,
    rc::Rc,
};

use fnv::FnvHashMap;

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
    fn default_value() -> &'static Self::Type;

    /// Gets the variable.
    fn var() -> ContextVarProxy<Self> {
        ContextVarProxy::default()
    }
}

/// Error when trying to set or modify a read-only variable.
#[derive(Debug, Hash, PartialEq, Eq)]
pub struct VarIsReadOnly;
impl std::fmt::Display for VarIsReadOnly {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "cannot set or modify read-only variable")
    }
}

mod protected {
    /// Ensures that only `zero-ui` can implement var types.
    pub trait Var {}
}

/// Part of [`Var`] that can be boxed.
pub trait VarObj<T: VarValue>: protected::Var + 'static {
    /// References the current value.
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a T;

    /// References the current value if it is new.
    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a T>;

    /// If [`set`](Self::set) or [`modify`](Var::modify) was called in the previous update.
    ///
    /// When you set the variable, the new value is only applied after the UI tree finishes
    /// the current update. The value is then applied causing a new update to happen, in the new
    /// update this method returns `true`. After the new update it returns `false` again.
    fn is_new(&self, vars: &Vars) -> bool;

    /// Version of the current value.
    ///
    /// The version number changes every update where [`set`](Self::set) or [`modify`](Var::modify) are called.
    fn version(&self, vars: &Vars) -> u32;

    /// If the variable cannot be set.
    ///
    /// Variables can still change if [`can_update`](Self::can_update) is `true`.
    ///
    /// Some variables can stop being read-only after an update, see also [`always_read_only`](Self::always_read_only).
    fn is_read_only(&self, vars: &Vars) -> bool;

    /// If the variable type is read-only, unlike [`is_read_only`](Self::is_read_only) this never changes.
    fn always_read_only(&self) -> bool;

    /// If the variable type allows the value to change.
    ///
    /// Some variables can change even if they are read-only, for example mapping variables.
    fn can_update(&self) -> bool;

    /// Schedules an assign for after the current update.
    ///
    /// Variables are not changed immediately, the full UI tree gets a chance to see the current value,
    /// after the current UI update, the values set here are applied.
    ///
    /// ### Error
    ///
    /// Returns [`VarIsReadOnly`] if [`is_read_only`](Self::is_read_only) is `true`.
    fn set(&self, vars: &Vars, new_value: T) -> Result<(), VarIsReadOnly>;

    /// Boxed version of the [`modify`](Var::modify) method.
    fn modify_boxed(&self, vars: &Vars, change: Box<dyn FnOnce(&mut T)>) -> Result<(), VarIsReadOnly>;

    /// Boxes `self`.
    ///
    /// A boxed var is also a var, that implementation just returns `self`.
    fn boxed(self) -> Box<dyn VarObj<T>>
    where
        Self: Sized,
    {
        Box::new(self)
    }
}

/// Represents a variable that has a value that can be accessed directly.
///
/// For the normal variables you need a reference to [`Vars`] to access the value,
/// this reference is not available in all [`UiNode`](crate::core::UiNode) methods.
///
/// Some variable types are safe to reference the inner value at any moment, other variables
/// can be wrapped in a type that makes a local clone of the current value. You can get any
/// variable as a local variable by calling [`Var::as_local`].
pub trait VarLocal<T: VarValue>: VarObj<T> {
    /// Reference the value.
    fn get_local(&self) -> &T;

    /// Initializes local clone of the value, if needed.
    ///
    /// This must be called in the [`UiNode::init`](crate::core::UiNode::init) method.
    ///
    /// Returns a reference to the local value for convenience.
    fn init_local(&mut self, vars: &Vars) -> &T;

    /// Updates the local clone of the value, if needed.
    ///
    /// This must be called in the [`UiNode::update`](crate::core::UiNode::update) method.
    ///
    /// Returns a reference to the local value if the value is new.
    fn update_local(&mut self, vars: &Vars) -> Option<&T>;

    /// Boxes `self`.
    fn boxed_local(self) -> Box<dyn VarLocal<T>>
    where
        Self: Sized,
    {
        Box::new(self)
    }
}

/// Represents a variable.
///
/// Most of the methods are declared in the [`VarObj`] trait to support boxing.
pub trait Var<T: VarValue>: VarObj<T> + Clone {
    /// Return type of [`as_read_only`](Var::as_read_only).
    type AsReadOnly: Var<T>;
    /// Return type of [`as_local`](Var::as_local).
    type AsLocal: VarLocal<T>;

    /// Schedules a closure to modify the value after the current update.
    ///
    /// This is a variation of the [`set`](VarObj::set) method that does not require
    /// an entire new value to be instantiated.
    fn modify<F: FnOnce(&mut T) + 'static>(&self, vars: &Vars, change: F) -> Result<(), VarIsReadOnly>;

    /// Returns the variable as a type that is [`always_read_only`](ObjVar::always_read_only).
    fn as_read_only(self) -> Self::AsReadOnly;

    /// Returns the variable as a type that implements [`VarLocal`].
    fn as_local(self) -> Self::AsLocal;

    /// Returns a variable whos value is mapped from `self`.
    ///
    /// The value is new when the `self` value is new, `map` is only called once per new value.
    ///
    /// The variable is read-only, use [`map_bidi`](Self::map_bidi) to propagate changes back to `self`.
    fn map<O: VarValue, F: FnMut(&T) -> O + 'static>(&self, map: F) -> RcMapVar<T, O, Self, F>;

    /// Returns a variable whos value is mapped to and from `self`.
    ///
    /// The value is new when the `self` value is new, `map` is only called once per new value.
    ///
    /// The variable can be set if `self` is not read-only, when set `map_back` is called to generate
    /// a value
    fn map_bidi<O: VarValue, F: FnMut(&T) -> O + 'static, G: FnMut(O) -> T + 'static>(
        &self,
        map: F,
        map_back: G,
    ) -> RcMapBidiVar<T, O, Self, F, G>;
}

/// A [`Var`] that locally owns the value.
///
/// This is [`always read-only`](VarObj::always_read_only), [cannot update](VarObj::can_update) and
/// is a [`VarLocal`].
#[derive(Clone, Default)]
pub struct OwnedVar<T: VarValue>(pub T);
impl<T: VarValue> protected::Var for OwnedVar<T> {}
impl<T: VarValue> VarObj<T> for OwnedVar<T> {
    fn get<'a>(&'a self, _: &'a Vars) -> &'a T {
        &self.0
    }

    fn get_new<'a>(&'a self, _: &'a Vars) -> Option<&'a T> {
        None
    }

    fn is_new(&self, _: &Vars) -> bool {
        false
    }

    fn version(&self, _: &Vars) -> u32 {
        0
    }

    fn is_read_only(&self, _: &Vars) -> bool {
        true
    }

    fn always_read_only(&self) -> bool {
        true
    }

    fn can_update(&self) -> bool {
        false
    }

    fn set(&self, _: &Vars, _: T) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    fn modify_boxed(&self, _: &Vars, _: Box<dyn FnOnce(&mut T)>) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }
}
impl<T: VarValue> VarLocal<T> for OwnedVar<T> {
    fn get_local(&self) -> &T {
        &self.0
    }
    fn init_local(&mut self, _: &Vars) -> &T {
        &self.0
    }

    fn update_local(&mut self, _: &Vars) -> Option<&T> {
        None
    }
}
impl<T: VarValue> Var<T> for OwnedVar<T> {
    type AsReadOnly = Self;
    type AsLocal = Self;

    fn as_read_only(self) -> Self::AsReadOnly {
        self
    }

    fn as_local(self) -> Self::AsLocal {
        self
    }

    fn modify<F: FnOnce(&mut T) + 'static>(&self, _: &Vars, _: F) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    fn map<O: VarValue, F: FnMut(&T) -> O + 'static>(&self, map: F) -> RcMapVar<T, O, Self, F> {
        RcMapVar::new(self.clone(), map)
    }

    fn map_bidi<O: VarValue, F: FnMut(&T) -> O + 'static, G: FnMut(O) -> T + 'static>(
        &self,
        map: F,
        map_back: G,
    ) -> RcMapBidiVar<T, O, Self, F, G> {
        RcMapBidiVar::new(self.clone(), map, map_back)
    }
}

struct RcVarData<T> {
    data: UnsafeCell<T>,
    last_updated: Cell<u32>,
    version: Cell<u32>,
}
/// A reference counted [`Var`].
pub struct RcVar<T: VarValue>(Rc<RcVarData<T>>);
impl<T: VarValue> protected::Var for RcVar<T> {}
impl<T: VarValue> RcVar<T> {
    pub fn new(value: T) -> Self {
        RcVar(Rc::new(RcVarData {
            data: UnsafeCell::new(value),
            last_updated: Cell::new(0),
            version: Cell::new(0),
        }))
    }
}
impl<T: VarValue> Clone for RcVar<T> {
    fn clone(&self) -> Self {
        RcVar(Rc::clone(&self.0))
    }
}
impl<T: VarValue> VarObj<T> for RcVar<T> {
    fn get<'a>(&'a self, _: &'a Vars) -> &'a T {
        // SAFETY: This is safe because we are bounding the value lifetime with
        // the `Vars` lifetime and we require a mutable reference to `Vars` to
        // modify the value.
        unsafe { &*self.0.data.get() }
    }

    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        if self.is_new(vars) {
            Some(self.get(vars))
        } else {
            None
        }
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.0.last_updated.get() == vars.update_id()
    }

    fn version(&self, _: &Vars) -> u32 {
        self.0.version.get()
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

    fn set(&self, vars: &Vars, new_value: T) -> Result<(), VarIsReadOnly> {
        let self2 = self.clone();
        vars.push_change(Box::new(move |update_id: u32| {
            // SAFETY: this is safe because Vars requires a mutable reference to apply changes.
            unsafe {
                *self2.0.data.get() = new_value;
            }
            self2.0.last_updated.set(update_id);
            self2.0.version.set(self2.0.version.get().wrapping_add(1));
        }));
        Ok(())
    }

    fn modify_boxed(&self, vars: &Vars, change: Box<dyn FnOnce(&mut T)>) -> Result<(), VarIsReadOnly> {
        let self2 = self.clone();
        vars.push_change(Box::new(move |update_id| {
            // SAFETY: this is safe because Vars requires a mutable reference to apply changes.
            change(unsafe { &mut *self2.0.data.get() });
            self2.0.last_updated.set(update_id);
            self2.0.version.set(self2.0.version.get().wrapping_add(1));
        }));
        Ok(())
    }
}
impl<T: VarValue> Var<T> for RcVar<T> {
    type AsReadOnly = ForceReadOnlyVar<T, Self>;
    type AsLocal = CloningLocalVar<T, Self>;

    fn as_read_only(self) -> Self::AsReadOnly {
        ForceReadOnlyVar::new(self)
    }

    fn as_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }

    fn modify<F: FnOnce(&mut T) + 'static>(&self, vars: &Vars, change: F) -> Result<(), VarIsReadOnly> {
        let me = self.clone();
        vars.push_change(Box::new(move |update_id: u32| {
            // SAFETY: this is safe because Vars requires a mutable reference to apply changes.
            change(unsafe { &mut *me.0.data.get() });
            me.0.last_updated.set(update_id);
            me.0.version.set(me.0.version.get().wrapping_add(1));
        }));
        Ok(())
    }

    fn map<O: VarValue, F: FnMut(&T) -> O>(&self, map: F) -> RcMapVar<T, O, Self, F> {
        RcMapVar::new(self.clone(), map)
    }

    fn map_bidi<O: VarValue, F: FnMut(&T) -> O + 'static, G: FnMut(O) -> T + 'static>(
        &self,
        map: F,
        map_back: G,
    ) -> RcMapBidiVar<T, O, Self, F, G> {
        RcMapBidiVar::new(self.clone(), map, map_back)
    }
}

#[doc(hidden)]
pub struct ForceReadOnlyVar<T: VarValue, V: Var<T>>(V, PhantomData<T>);
impl<T: VarValue, V: Var<T>> protected::Var for ForceReadOnlyVar<T, V> {}
impl<T: VarValue, V: Var<T>> ForceReadOnlyVar<T, V> {
    fn new(var: V) -> Self {
        ForceReadOnlyVar(var, PhantomData)
    }
}
impl<T: VarValue, V: Var<T>> Clone for ForceReadOnlyVar<T, V> {
    fn clone(&self) -> Self {
        ForceReadOnlyVar(self.0.clone(), PhantomData)
    }
}
impl<T: VarValue, V: Var<T>> VarObj<T> for ForceReadOnlyVar<T, V> {
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a T {
        self.0.get(vars)
    }

    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        self.0.get_new(vars)
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.0.is_new(vars)
    }

    fn version(&self, vars: &Vars) -> u32 {
        self.0.version(vars)
    }

    fn is_read_only(&self, _: &Vars) -> bool {
        true
    }

    fn always_read_only(&self) -> bool {
        true
    }

    fn can_update(&self) -> bool {
        self.0.can_update()
    }

    fn set(&self, _: &Vars, _: T) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    fn modify_boxed(&self, _: &Vars, _: Box<dyn FnOnce(&mut T)>) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }
}
impl<T: VarValue, V: Var<T>> Var<T> for ForceReadOnlyVar<T, V> {
    type AsReadOnly = Self;
    type AsLocal = CloningLocalVar<T, Self>;

    fn as_read_only(self) -> Self::AsReadOnly {
        self
    }

    fn as_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }

    fn modify<F: FnOnce(&mut T) + 'static>(&self, _: &Vars, _: F) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    fn map<O: VarValue, F: FnMut(&T) -> O + 'static>(&self, map: F) -> RcMapVar<T, O, Self, F> {
        RcMapVar::new(self.clone(), map)
    }

    fn map_bidi<O: VarValue, F: FnMut(&T) -> O + 'static, G: FnMut(O) -> T + 'static>(
        &self,
        map: F,
        map_back: G,
    ) -> RcMapBidiVar<T, O, Self, F, G> {
        RcMapBidiVar::new(self.clone(), map, map_back)
    }
}

#[doc(hidden)]
#[derive(Clone)]
pub struct CloningLocalVar<T: VarValue, V: Var<T>> {
    var: V,
    local_version: u32,
    local: Option<T>,
}
impl<T: VarValue, V: Var<T>> protected::Var for CloningLocalVar<T, V> {}
impl<T: VarValue, V: Var<T>> CloningLocalVar<T, V> {
    fn new(var: V) -> Self {
        CloningLocalVar {
            var,
            local_version: 0,
            local: None,
        }
    }
}
impl<T: VarValue, V: Var<T>> VarObj<T> for CloningLocalVar<T, V> {
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a T {
        self.var.get(vars)
    }

    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        self.var.get_new(vars)
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.var.is_new(vars)
    }

    fn version(&self, vars: &Vars) -> u32 {
        self.var.version(vars)
    }

    fn is_read_only(&self, vars: &Vars) -> bool {
        self.var.is_read_only(vars)
    }

    fn always_read_only(&self) -> bool {
        self.var.always_read_only()
    }

    fn can_update(&self) -> bool {
        self.var.can_update()
    }

    fn set(&self, vars: &Vars, new_value: T) -> Result<(), VarIsReadOnly> {
        self.var.set(vars, new_value)
    }

    fn modify_boxed(&self, vars: &Vars, change: Box<dyn FnOnce(&mut T)>) -> Result<(), VarIsReadOnly> {
        self.var.modify_boxed(vars, change)
    }
}
impl<T: VarValue, V: Var<T>> Var<T> for CloningLocalVar<T, V> {
    type AsReadOnly = ForceReadOnlyVar<T, Self>;
    type AsLocal = Self;

    fn as_read_only(self) -> Self::AsReadOnly {
        ForceReadOnlyVar::new(self)
    }

    fn as_local(self) -> Self::AsLocal {
        self
    }

    fn modify<F: FnOnce(&mut T) + 'static>(&self, vars: &Vars, change: F) -> Result<(), VarIsReadOnly> {
        self.var.modify(vars, change)
    }

    fn map<O: VarValue, F: FnMut(&T) -> O>(&self, map: F) -> RcMapVar<T, O, Self, F> {
        RcMapVar::new(self.clone(), map)
    }

    fn map_bidi<O: VarValue, F: FnMut(&T) -> O + 'static, G: FnMut(O) -> T + 'static>(
        &self,
        map: F,
        map_back: G,
    ) -> RcMapBidiVar<T, O, Self, F, G> {
        RcMapBidiVar::new(self.clone(), map, map_back)
    }
}
impl<T: VarValue, V: Var<T>> VarLocal<T> for CloningLocalVar<T, V> {
    fn get_local(&self) -> &T {
        self.local.as_ref().expect("local variable not initialized")
    }

    fn init_local(&mut self, vars: &Vars) -> &T {
        self.local_version = self.var.version(vars);
        self.local = Some(self.var.get(vars).clone());
        self.local.as_ref().unwrap()
    }

    fn update_local(&mut self, vars: &Vars) -> Option<&T> {
        let var_version = self.var.version(vars);
        if var_version != self.local_version {
            self.local_version = var_version;
            self.local = Some(self.var.get(vars).clone());
            self.local.as_ref()
        } else {
            None
        }
    }
}

struct RcMapVarData<I: VarValue, O: VarValue, V: Var<I>, F: FnMut(&I) -> O + 'static> {
    _i: PhantomData<I>,
    var: V,
    f: RefCell<F>,
    version: Cell<Option<u32>>,
    output: UnsafeCell<MaybeUninit<O>>,
}
#[doc(hidden)]
pub struct RcMapVar<I: VarValue, O: VarValue, V: Var<I>, F: FnMut(&I) -> O + 'static>(Rc<RcMapVarData<I, O, V, F>>);
impl<I, O, V, F> protected::Var for RcMapVar<I, O, V, F>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: FnMut(&I) -> O,
{
}
impl<I, O, V, F> Clone for RcMapVar<I, O, V, F>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: FnMut(&I) -> O,
{
    fn clone(&self) -> Self {
        RcMapVar(Rc::clone(&self.0))
    }
}
impl<I, O, V, F> RcMapVar<I, O, V, F>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: FnMut(&I) -> O + 'static,
{
    fn new(var: V, f: F) -> Self {
        RcMapVar(Rc::new(RcMapVarData {
            _i: PhantomData,
            var,
            f: RefCell::new(f),
            version: Cell::new(None),
            output: UnsafeCell::new(MaybeUninit::uninit()),
        }))
    }
}
impl<I, O, V, F> VarObj<O> for RcMapVar<I, O, V, F>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: FnMut(&I) -> O + 'static,
{
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a O {
        let var_version = Some(self.0.var.version(vars));
        if var_version != self.0.version.get() {
            let value = (&mut *self.0.f.borrow_mut())(self.0.var.get(vars));
            // SAFETY: This is safe because it only happens before the first borrow
            // of this update, and borrows cannot exist across updates because source
            // vars require a &mut Vars for changing version.
            unsafe {
                let m_uninit = &mut *self.0.output.get();
                m_uninit.as_mut_ptr().write(value);
            }
            self.0.version.set(var_version);
        }
        // SAFETY:
        // This is safe because source require &mut Vars for updating.
        unsafe {
            let inited = &*self.0.output.get();
            &*inited.as_ptr()
        }
    }

    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a O> {
        if self.is_new(vars) {
            Some(self.get(vars))
        } else {
            None
        }
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.0.var.is_new(vars)
    }

    fn version(&self, vars: &Vars) -> u32 {
        self.0.var.version(vars)
    }

    fn is_read_only(&self, _: &Vars) -> bool {
        true
    }

    fn always_read_only(&self) -> bool {
        true
    }

    fn can_update(&self) -> bool {
        self.0.var.can_update()
    }

    fn set(&self, _: &Vars, _: O) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    fn modify_boxed(&self, _: &Vars, _: Box<dyn FnOnce(&mut O)>) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }
}
impl<I, O, V, F> Var<O> for RcMapVar<I, O, V, F>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: FnMut(&I) -> O + 'static,
{
    type AsReadOnly = Self;
    type AsLocal = CloningLocalVar<O, Self>;

    fn modify<G>(&self, _: &Vars, _: G) -> Result<(), VarIsReadOnly>
    where
        G: FnOnce(&mut O) + 'static,
    {
        Err(VarIsReadOnly)
    }

    fn as_read_only(self) -> Self::AsReadOnly {
        self
    }

    fn as_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }

    fn map<O2: VarValue, F2: FnMut(&O) -> O2 + 'static>(&self, map: F2) -> RcMapVar<O, O2, Self, F2> {
        RcMapVar::new(self.clone(), map)
    }

    fn map_bidi<O2: VarValue, F2: FnMut(&O) -> O2 + 'static, G: FnMut(O2) -> O + 'static>(
        &self,
        map: F2,
        map_back: G,
    ) -> RcMapBidiVar<O, O2, Self, F2, G> {
        RcMapBidiVar::new(self.clone(), map, map_back)
    }
}

struct RcMapBidiVarData<I: VarValue, O: VarValue, V: Var<I>, F: FnMut(&I) -> O, G: FnMut(O) -> I> {
    _i: PhantomData<I>,
    var: V,
    map: RefCell<F>,
    map_back: RefCell<G>,
    version: Cell<Option<u32>>,
    output: UnsafeCell<MaybeUninit<O>>,
}
#[doc(hidden)]
pub struct RcMapBidiVar<I: VarValue, O: VarValue, V: Var<I>, F: FnMut(&I) -> O, G: FnMut(O) -> I>(Rc<RcMapBidiVarData<I, O, V, F, G>>);
impl<I, O, V, F, G> protected::Var for RcMapBidiVar<I, O, V, F, G>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: FnMut(&I) -> O,
    G: FnMut(O) -> I,
{
}
impl<I, O, V, F, G> Clone for RcMapBidiVar<I, O, V, F, G>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: FnMut(&I) -> O,
    G: FnMut(O) -> I,
{
    fn clone(&self) -> Self {
        RcMapBidiVar(Rc::clone(&self.0))
    }
}
impl<I, O, V, F, G> RcMapBidiVar<I, O, V, F, G>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: FnMut(&I) -> O + 'static,
    G: FnMut(O) -> I + 'static,
{
    fn new(var: V, map: F, map_back: G) -> Self {
        RcMapBidiVar(Rc::new(RcMapBidiVarData {
            _i: PhantomData,
            var,
            map: RefCell::new(map),
            map_back: RefCell::new(map_back),
            version: Cell::new(None),
            output: UnsafeCell::new(MaybeUninit::uninit()),
        }))
    }
}
impl<I, O, V, F, G> VarObj<O> for RcMapBidiVar<I, O, V, F, G>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: FnMut(&I) -> O + 'static,
    G: FnMut(O) -> I + 'static,
{
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a O {
        let var_version = Some(self.0.var.version(vars));
        if var_version != self.0.version.get() {
            let value = (&mut *self.0.map.borrow_mut())(self.0.var.get(vars));
            // SAFETY: This is safe because it only happens before the first borrow
            // of this update, and borrows cannot exist across updates because source
            // vars require a &mut Vars for changing version.
            unsafe {
                let m_uninit = &mut *self.0.output.get();
                m_uninit.as_mut_ptr().write(value);
            }
            self.0.version.set(var_version);
        }
        // SAFETY:
        // This is safe because source require &mut Vars for updating.
        unsafe {
            let inited = &*self.0.output.get();
            &*inited.as_ptr()
        }
    }

    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a O> {
        if self.is_new(vars) {
            Some(self.get(vars))
        } else {
            None
        }
    }

    fn is_new(&self, vars: &Vars) -> bool {
        self.0.var.is_new(vars)
    }

    fn version(&self, vars: &Vars) -> u32 {
        self.0.var.version(vars)
    }

    fn is_read_only(&self, vars: &Vars) -> bool {
        self.0.var.is_read_only(vars)
    }

    fn always_read_only(&self) -> bool {
        self.0.var.always_read_only()
    }

    fn can_update(&self) -> bool {
        self.0.var.can_update()
    }

    fn set(&self, vars: &Vars, new_value: O) -> Result<(), VarIsReadOnly> {
        let new_value = (&mut *self.0.map_back.borrow_mut())(new_value);
        self.0.var.set(vars, new_value)
    }

    fn modify_boxed(&self, vars: &Vars, change: Box<dyn FnOnce(&mut O)>) -> Result<(), VarIsReadOnly> {
        let mut new_value = self.get(vars).clone();
        change(&mut new_value);
        self.set(vars, new_value)
    }
}
impl<I, O, V, F, G> Var<O> for RcMapBidiVar<I, O, V, F, G>
where
    I: VarValue,
    O: VarValue,
    V: Var<I>,
    F: FnMut(&I) -> O + 'static,
    G: FnMut(O) -> I + 'static,
{
    type AsReadOnly = ForceReadOnlyVar<O, Self>;
    type AsLocal = CloningLocalVar<O, Self>;

    fn modify<H>(&self, vars: &Vars, change: H) -> Result<(), VarIsReadOnly>
    where
        H: FnOnce(&mut O) + 'static,
    {
        let mut new_value = self.get(vars).clone();
        change(&mut new_value);
        self.set(vars, new_value)
    }

    fn as_read_only(self) -> Self::AsReadOnly {
        ForceReadOnlyVar::new(self)
    }

    fn as_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }

    fn map<O2: VarValue, F2: FnMut(&O) -> O2>(&self, map: F2) -> RcMapVar<O, O2, Self, F2> {
        RcMapVar::new(self.clone(), map)
    }

    fn map_bidi<O2: VarValue, F2: FnMut(&O) -> O2 + 'static, G2: FnMut(O2) -> O + 'static>(
        &self,
        map: F2,
        map_back: G2,
    ) -> RcMapBidiVar<O, O2, Self, F2, G2> {
        RcMapBidiVar::new(self.clone(), map, map_back)
    }
}

#[doc(hidden)]
#[derive(Clone)]
pub struct ContextVarProxy<C: ContextVar>(PhantomData<C>);
impl<C: ContextVar> protected::Var for ContextVarProxy<C> {}
impl<C: ContextVar> Default for ContextVarProxy<C> {
    fn default() -> Self {
        ContextVarProxy(PhantomData)
    }
}
impl<C: ContextVar> VarObj<C::Type> for ContextVarProxy<C> {
    fn get<'a>(&'a self, vars: &'a Vars) -> &'a C::Type {
        vars.context_var::<C>().0
    }

    fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a C::Type> {
        let (value, is_new, _) = vars.context_var::<C>();
        if is_new {
            Some(value)
        } else {
            None
        }
    }

    fn is_new(&self, vars: &Vars) -> bool {
        vars.context_var::<C>().1
    }

    fn version(&self, vars: &Vars) -> u32 {
        vars.context_var::<C>().2
    }

    fn is_read_only(&self, _: &Vars) -> bool {
        true
    }

    fn always_read_only(&self) -> bool {
        true
    }

    fn can_update(&self) -> bool {
        true
    }

    fn set(&self, _: &Vars, _: C::Type) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    fn modify_boxed(&self, _: &Vars, _: Box<dyn FnOnce(&mut C::Type)>) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }
}
impl<C: ContextVar> Var<C::Type> for ContextVarProxy<C> {
    type AsReadOnly = Self;

    type AsLocal = CloningLocalVar<C::Type, Self>;

    fn as_read_only(self) -> Self::AsReadOnly {
        self
    }

    fn as_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }

    fn modify<F: FnOnce(&mut C::Type) + 'static>(&self, _: &Vars, _: F) -> Result<(), VarIsReadOnly> {
        Err(VarIsReadOnly)
    }

    fn map<O: VarValue, F: FnMut(&C::Type) -> O>(&self, map: F) -> RcMapVar<C::Type, O, Self, F> {
        RcMapVar::new(self.clone(), map)
    }

    fn map_bidi<O: VarValue, F: FnMut(&C::Type) -> O + 'static, G: FnMut(O) -> C::Type + 'static>(
        &self,
        map: F,
        map_back: G,
    ) -> RcMapBidiVar<C::Type, O, Self, F, G> {
        RcMapBidiVar::new(self.clone(), map, map_back)
    }
}

pub struct Vars {
    update_id: u32,
    #[allow(clippy::type_complexity)]
    pending: RefCell<Vec<Box<dyn FnOnce(u32)>>>,
    context_vars: RefCell<FnvHashMap<TypeId, (*const AnyRef, bool, u32)>>,
}
impl Vars {
    fn update_id(&self) -> u32 {
        self.update_id
    }

    /// Gets a var at the context level.
    fn context_var<C: ContextVar>(&self) -> (&C::Type, bool, u32) {
        let vars = self.context_vars.borrow();
        if let Some((any_ref, is_new, version)) = vars.get(&TypeId::of::<C>()) {
            // SAFETY: This is safe because `TypeId` keys are always associated
            // with the same type of reference. Also we are not leaking because the
            // source reference is borrowed in a [`with_context_var`] call.
            let value = unsafe { AnyRef::unpack(*any_ref) };
            (value, *is_new, *version)
        } else {
            (C::default_value(), false, 0)
        }
    }

    /// Calls `f` with the context var value.
    pub fn with_context_var<C: ContextVar, F: FnOnce(&Vars)>(&self, value: &C::Type, is_new: bool, version: u32, f: F) {
        let prev = self
            .context_vars
            .borrow_mut()
            .insert(TypeId::of::<C>(), (AnyRef::pack(value), is_new, version));
        f(self);
        if let Some(prev) = prev {
            self.context_vars.borrow_mut().insert(TypeId::of::<C>(), prev);
        }
    }

    fn push_change(&self, change: Box<dyn FnOnce(u32)>) {
        self.pending.borrow_mut().push(change);
    }

    pub(super) fn apply(&mut self) {
        self.update_id = self.update_id.wrapping_add(1);
        for f in self.pending.get_mut().drain(..) {
            f(self.update_id);
        }
    }
}
enum AnyRef {}
impl AnyRef {
    fn pack<T>(r: &T) -> *const AnyRef {
        (r as *const T) as *const AnyRef
    }

    unsafe fn unpack<'a, T>(pointer: *const Self) -> &'a T {
        &*(pointer as *const T)
    }
}
