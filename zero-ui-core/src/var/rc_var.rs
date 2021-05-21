use super::*;

/// A [`Var`] that can be shared.
///
/// This type is a reference-counted pointer ([`Rc`]),
/// it implements the full [`Var`] read and write methods.
///
/// This is the variable type to use for binding two properties to the same value,
/// or changing a property value during runtime.
///
/// # New and Share
///
/// Use [`var`] to create a new variable, use `RcVar::clone` to create another reference
/// to the same variable.
///
/// Use the [`Sync`](crate::sync::Sync) variable methods to access the variable from other threads.
pub struct RcVar<T: VarValue>(Rc<RcVarData<T>>);
struct RcVarData<T> {
    data: UnsafeCell<T>,
    last_updated: Cell<u32>,
    version: Cell<u32>,
}
impl<T: VarValue> protected::Var for RcVar<T> {}
impl<T: VarValue> RcVar<T> {
    pub(super) fn new(value: T) -> Self {
        RcVar(Rc::new(RcVarData {
            data: UnsafeCell::new(value),
            last_updated: Cell::new(0),
            version: Cell::new(0),
        }))
    }

    /// References the current value.
    pub fn get<'a>(&'a self, vars: &'a VarsRead) -> &'a T {
        <Self as VarObj<T>>::get(self, vars)
    }

    /// References the current value if it is new.
    pub fn get_new<'a>(&'a self, vars: &'a Vars) -> Option<&'a T> {
        <Self as VarObj<T>>::get_new(self, vars)
    }

    /// If [`set`](Self::set) or [`modify`](Var::modify) was called in the previous update.
    pub fn is_new(&self, vars: &Vars) -> bool {
        <Self as VarObj<T>>::is_new(self, vars)
    }

    /// Version of the current value.
    ///
    /// The version is incremented every update
    /// that [`set`](Self::set) or [`modify`](Var::modify) are called.
    pub fn version(&self, vars: &VarsRead) -> u32 {
        <Self as VarObj<T>>::version(self, vars)
    }

    /// Schedules an assign for after the current update.
    ///
    /// The value is not changed immediately, the full UI tree gets a chance to see the current value,
    /// after the current UI update, the value is updated.
    pub fn set(&self, vars: &Vars, new_value: T) {
        let _ = <Self as VarObj<T>>::set(self, vars, new_value);
    }

    /// Does `set` if `new_value` is not equal to the current value.
    pub fn set_ne(&self, vars: &Vars, new_value: T) -> bool
    where
        T: PartialEq,
    {
        let ne = self.get(vars) != &new_value;
        if ne {
            self.set(vars, new_value);
        }
        ne
    }

    /// Schedules a closure to modify the value after the current update.
    ///
    /// This is a variation of the [`set`](Self::set) method that does not require
    /// an entire new value to be instantiated.
    pub fn modify<F: FnOnce(&mut T) + 'static>(&self, vars: &Vars, change: F) {
        let _ = <Self as Var<T>>::modify(self, vars, change);
    }

    /// Returns `true` if both are the same variable.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    /// Returns the number of pointers to this same var.
    pub fn ptr_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }

    /// Returns a variable with value generated from `self`.
    ///
    /// The value is new when the `self` value is new, `map` is only called once per new value.
    ///
    /// The variable is read-only, use [`map_bidi`](Self::map_bidi) to propagate changes back to `self`.
    ///
    /// Use [`map_ref`](Self::map_ref) if you don't need to generate a new value.
    ///
    /// Use [`into_map`](Self::into_map) if you will not use this copy of `self` anymore.
    pub fn map<O: VarValue, F: FnMut(&T) -> O + 'static>(&self, map: F) -> RcMapVar<T, O, Self, F> {
        <Self as Var<T>>::map(self, map)
    }

    /// Returns a variable with value referenced from `self`.
    ///
    /// The value is new when the `self` value is new, `map` is called every time [`get`](VarObj::get) is called.
    ///
    /// The variable is read-only.
    ///
    /// Use [`into_map_ref`](Self::into_map_ref) if you will not use this copy of `self` anymore.
    pub fn map_ref<O: VarValue, F: Fn(&T) -> &O + Clone + 'static>(&self, map: F) -> MapRefVar<T, O, Self, F> {
        <Self as Var<T>>::map_ref(self, map)
    }

    /// Returns a variable whos value is mapped to and from `self`.
    ///
    /// The value is new when the `self` value is new, `map` is only called once per new value.
    ///
    /// The variable can be set if `self` is not read-only, when set `map_back` is called to generate
    /// a new value for `self`.
    ///
    /// Use [`map_bidi_ref`](Self::map_bidi_ref) if you don't need to generate a new value.
    ///
    /// Use [`into_map_bidi`](Self::into_map_bidi) if you will not use this copy of `self` anymore.
    pub fn map_bidi<O: VarValue, F: FnMut(&T) -> O + 'static, G: FnMut(O) -> T + 'static>(
        &self,
        map: F,
        map_back: G,
    ) -> RcMapBidiVar<T, O, Self, F, G> {
        <Self as Var<T>>::map_bidi(self, map, map_back)
    }

    /// Returns a variable with value mapped to and from `self` using references.
    ///
    /// The value is new when the `self` value is new, `map` is called every time [`get`](VarObj::get) is called,
    /// `map_mut` is called every time the value is set or modified.
    ///
    /// Use [`into_map`](Self::into_map) if you will not use this copy of `self` anymore.
    pub fn map_bidi_ref<O: VarValue, F: Fn(&T) -> &O + Clone + 'static, G: Fn(&mut T) -> &mut O + Clone + 'static>(
        &self,
        map: F,
        map_mut: G,
    ) -> MapBidiRefVar<T, O, Self, F, G> {
        <Self as Var<T>>::map_bidi_ref(self, map, map_mut)
    }

    /// Taking variant of [`map`](Self::map).
    pub fn into_map<O: VarValue, F: FnMut(&T) -> O + 'static>(self, map: F) -> RcMapVar<T, O, Self, F> {
        <Self as Var<T>>::into_map(self, map)
    }

    /// Taking variant of [`map_ref`](Self::map_ref).
    pub fn into_map_ref<O: VarValue, F: Fn(&T) -> &O + Clone + 'static>(self, map: F) -> MapRefVar<T, O, Self, F> {
        <Self as Var<T>>::into_map_ref(self, map)
    }

    /// Taking variant of [`map_bidi`](Self::map_bidi).
    pub fn into_map_bidi<O: VarValue, F: FnMut(&T) -> O + 'static, G: FnMut(O) -> T + 'static>(
        self,
        map: F,
        map_back: G,
    ) -> RcMapBidiVar<T, O, Self, F, G> {
        <Self as Var<T>>::into_map_bidi(self, map, map_back)
    }

    /// Taking variant of [`map_bidi_ref`](Self::map_bidi_ref).
    pub fn into_map_bidi_ref<O: VarValue, F: Fn(&T) -> &O + Clone + 'static, G: Fn(&mut T) -> &mut O + Clone + 'static>(
        self,
        map: F,
        map_mut: G,
    ) -> MapBidiRefVar<T, O, Self, F, G> {
        <Self as Var<T>>::into_map_bidi_ref(self, map, map_mut)
    }
}
impl<T: VarValue> Clone for RcVar<T> {
    /// Clone the variable reference.
    fn clone(&self) -> Self {
        RcVar(Rc::clone(&self.0))
    }
}
impl<T: VarValue> VarObj<T> for RcVar<T> {
    fn get<'a>(&'a self, _: &'a VarsRead) -> &'a T {
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

    fn version(&self, _: &VarsRead) -> u32 {
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

    fn set_ne(&self, vars: &Vars, new_value: T) -> Result<bool, VarIsReadOnly>
    where
        T: PartialEq,
    {
        let ne = self.get(vars) != &new_value;
        if ne {
            self.set(vars, new_value);
        }
        Ok(ne)
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

    fn into_local(self) -> Self::AsLocal {
        CloningLocalVar::new(self)
    }

    fn into_read_only(self) -> Self::AsReadOnly {
        ForceReadOnlyVar::new(self)
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
        self.clone().into_map(map)
    }

    fn map_ref<O: VarValue, F: Fn(&T) -> &O + Clone + 'static>(&self, map: F) -> MapRefVar<T, O, Self, F> {
        self.clone().into_map_ref(map)
    }

    fn map_bidi<O: VarValue, F: FnMut(&T) -> O + 'static, G: FnMut(O) -> T + 'static>(
        &self,
        map: F,
        map_back: G,
    ) -> RcMapBidiVar<T, O, Self, F, G> {
        self.clone().into_map_bidi(map, map_back)
    }

    fn into_map<O: VarValue, F: FnMut(&T) -> O>(self, map: F) -> RcMapVar<T, O, Self, F> {
        RcMapVar::new(self, map)
    }

    fn into_map_bidi<O: VarValue, F: FnMut(&T) -> O + 'static, G: FnMut(O) -> T + 'static>(
        self,
        map: F,
        map_back: G,
    ) -> RcMapBidiVar<T, O, Self, F, G> {
        RcMapBidiVar::new(self, map, map_back)
    }

    fn into_map_ref<O: VarValue, F: Fn(&T) -> &O + Clone + 'static>(self, map: F) -> MapRefVar<T, O, Self, F> {
        MapRefVar::new(self, map)
    }

    fn map_bidi_ref<O: VarValue, F: Fn(&T) -> &O + Clone + 'static, G: Fn(&mut T) -> &mut O + Clone + 'static>(
        &self,
        map: F,
        map_mut: G,
    ) -> MapBidiRefVar<T, O, Self, F, G> {
        self.clone().into_map_bidi_ref(map, map_mut)
    }

    fn into_map_bidi_ref<O: VarValue, F: Fn(&T) -> &O + Clone + 'static, G: Fn(&mut T) -> &mut O + Clone + 'static>(
        self,
        map: F,
        map_mut: G,
    ) -> MapBidiRefVar<T, O, Self, F, G> {
        MapBidiRefVar::new(self, map, map_mut)
    }
}

impl<T: VarValue> IntoVar<T> for RcVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

/// New [`RcVar`].
pub fn var<V: VarValue>(value: V) -> RcVar<V> {
    RcVar::new(value)
}

/// New [`RcVar`] using conversion.
pub fn var_from<V: VarValue, I: Into<V>>(value: I) -> RcVar<V> {
    RcVar::new(value.into())
}

/// New [`StateVar`].
pub fn state_var() -> StateVar {
    var(false)
}

/// Variable type of state properties (`is_*`).
///
/// State variables are `bool` probes that are set by the property.
pub type StateVar = RcVar<bool>;
