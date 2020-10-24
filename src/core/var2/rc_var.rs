use super::*;

/// A reference counted [`Var`].
pub struct RcVar<T: VarValue>(Rc<RcVarData<T>>);
struct RcVarData<T> {
    data: UnsafeCell<T>,
    last_updated: Cell<u32>,
    version: Cell<u32>,
}
impl<T: VarValue> protected::Var for RcVar<T> {}
impl<T: VarValue> RcVar<T> {
    pub fn new(value: T) -> Self {
        RcVar(Rc::new(RcVarData {
            data: UnsafeCell::new(value),
            last_updated: Cell::new(0),
            version: Cell::new(0),
        }))
    }

    /// References the current value.
    pub fn get<'a>(&'a self, vars: &'a Vars) -> &'a T {
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
    pub fn version(&self, vars: &Vars) -> u32 {
        <Self as VarObj<T>>::version(self, vars)
    }

    /// Schedules an assign for after the current update.
    ///
    /// The value is not changed immediately, the full UI tree gets a chance to see the current value,
    /// after the current UI update, the value is updated.
    pub fn set(&self, vars: &Vars, new_value: T) {
        let _ = <Self as VarObj<T>>::set(self, vars, new_value);
    }

    /// Schedules a closure to modify the value after the current update.
    ///
    /// This is a variation of the [`set`](Self::set) method that does not require
    /// an entire new value to be instantiated.
    pub fn modify<F: FnOnce(&mut T) + 'static>(&self, vars: &Vars, change: F) {
        let _ = <Self as Var<T>>::modify(self, vars, change);
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

impl<T: VarValue> IntoVar<T> for RcVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

/// New [`RcVar`].
pub fn var<V: VarValue, I: Into<V>>(value: I) -> RcVar<V> {
    RcVar::new(value.into())
}

/// Initializes a new [`StateVar`].
pub fn state_var() -> StateVar {
    var(false)
}

/// State properties (`is_*`) variable type.
pub type StateVar = RcVar<bool>;
