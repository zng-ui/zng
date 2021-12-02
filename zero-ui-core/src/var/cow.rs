use std::{
    cell::{Cell, UnsafeCell},
    rc::Rc,
};

use super::*;

/// A clone-on-write variable.
///
/// This variable returns the value of another variable until it is set or modified. When
/// it is set or modified it clones the value and detaches from the source variable and
/// behaves like the [`RcVar<T>`].
///
/// You can use this variable in contexts where a value is *inherited* from a source but
/// can optionally be overridden locally.
///
/// # Examples
///
/// The example has two variables `source` and `test` at the beginning when `source` updates the
/// update is visible in `test`, but after `test` is assigned it disconnects from `source` and
/// contains its own value.
///
/// ```
/// # use zero_ui_core::{var::*, handler::*, context::*};
/// # TestWidgetContext::doc_test((),
/// async_hn!(|ctx, _| {
///     let source = var(0u8);
///     let test = RcCowVar::new(source.clone());
///
///     // update in source is visible in test var:
///     source.set(&ctx, 1);
///     ctx.update().await;
///     // both are new
///     assert_eq!(source.copy_new(&ctx).unwrap(), test.copy_new(&ctx).unwrap());
///     // test var is not cloned
///     assert!(!test.is_cloned(&ctx));
///
///     // update test var directly, disconnecting it from source:
///     test.set(&ctx, 2);
///     ctx.update().await;
///     // only test is new
///     assert!(!source.is_new(&ctx));
///     assert_eq!(Some(2), test.copy_new(&ctx));
///     // it is now cloned
///     assert!(test.is_cloned(&ctx));
///
///     // the source no longer updates the test:
///     source.set(&ctx, 3);
///     ctx.update().await;
///     assert!(!test.is_new(&ctx));
/// })
/// # );
/// ```
pub struct RcCowVar<T, V>(Rc<CowData<T, V>>);
struct CowData<T, V> {
    source: UnsafeCell<Option<V>>,
    source_always_read_only: bool,
    source_can_update: bool,
    is_pass_through: bool,
    update_mask: UpdateMask,

    value: UnsafeCell<Option<T>>,
    version: Cell<u32>,
    last_update_id: Cell<u32>,
}
impl<T: VarValue, V: Var<T>> Clone for RcCowVar<T, V> {
    fn clone(&self) -> Self {
        RcCowVar(Rc::clone(&self.0))
    }
}
impl<T: VarValue, V: Var<T>> RcCowVar<T, V> {
    /// Returns a new var that reads from `source`.
    #[inline]
    pub fn new(source: V) -> Self {
        Self::new_(source, false)
    }

    /// Returns a new [`RcCowVar`] that **is not clone-on-write**.
    ///
    /// Modifying the returned variable modifies the `source`. You can use this to
    /// avoid boxing variables in methods that can return either the source variable
    /// or an override variable.
    #[inline]
    pub fn pass_through(source: V) -> Self {
        Self::new_(source, true)
    }

    fn new_(source: V, is_pass_through: bool) -> Self {
        RcCowVar(Rc::new(CowData {
            update_mask: source.update_mask(),
            source_always_read_only: source.always_read_only(),
            source_can_update: source.can_update(),
            source: UnsafeCell::new(Some(source)),
            is_pass_through,
            value: UnsafeCell::new(None),
            version: Cell::new(0),
            last_update_id: Cell::new(0),
        }))
    }

    /// Returns `true` if this variable value is a cloned local.
    ///
    /// When this is `false` the value is read from another variable, when it is `true` it is read from local value.
    pub fn is_cloned<Vr: WithVarsRead>(&self, vars: &Vr) -> bool {
        vars.with_vars_read(|v| self.source(v).is_none())
    }

    fn source<'a>(&'a self, _vars: &'a VarsRead) -> Option<&'a V> {
        // SAFETY: this is safe because we are holding a reference to vars and
        // variables require the mutable reference to vars for modifying.
        unsafe { &*self.0.source.get() }.as_ref()
    }

    /// Returns `true` if **the source variable is written* when modifying this variable.
    ///
    /// You can use [`pass_through`] to create a pass-through variable.
    ///
    /// [`pass_through`]: Self::pass_through
    #[inline]
    pub fn is_pass_through(&self) -> bool {
        self.0.is_pass_through
    }

    /// Reference the current value.
    ///
    /// The value can be from the source variable or a local clone if [`is_cloned`].
    ///
    /// [`is_cloned`]: Self::is_cloned
    #[inline]
    pub fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a T {
        let vars = vars.as_ref();

        if let Some(source) = self.source(vars) {
            source.get(vars)
        } else {
            // SAFETY: this is safe because we are tying the `Vars` lifetime to the value
            // and we require `&mut Vars` to modify the value.
            unsafe { &*self.0.value.get() }.as_ref().unwrap()
        }
    }

    /// Reference the current value if it [`is_new`].
    ///
    /// [`is_new`]: Self::is_new
    pub fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a T> {
        let vars = vars.as_ref();

        if let Some(source) = self.source(vars) {
            source.get_new(vars)
        } else if self.0.last_update_id.get() == vars.update_id() {
            Some(self.get(vars))
        } else {
            None
        }
    }

    /// If the current value changed in the last update.
    ///
    /// Returns `true` is the source variable is new or if [`is_cloned`] returns if the
    /// value was set in the previous update.
    ///
    /// [`is_cloned`]: Self::is_cloned
    #[inline]
    pub fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
        vars.with_vars(|vars| {
            if let Some(source) = self.source(vars) {
                source.is_new(vars)
            } else {
                self.0.last_update_id.get() == vars.update_id()
            }
        })
    }

    /// Gets the current value version.
    ///
    /// Returns the source variable version of if [`is_cloned`] returns the cloned version.
    /// The source version is copied and incremented by one on the first *write*. Subsequent
    /// *writes* increment the version by one.
    ///
    /// [`is_cloned`]: Self::is_cloned
    #[inline]
    pub fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> u32 {
        vars.with_vars_read(|vars| {
            if let Some(source) = self.source(vars) {
                source.version(vars)
            } else {
                self.0.version.get()
            }
        })
    }

    /// Returns `false` unless [`is_pass_through`] and the source variable is read-only.
    ///
    /// [`is_pass_through`]: Self::is_pass_through.
    #[inline]
    pub fn is_read_only<Vw: WithVars>(&self, vars: &Vw) -> bool {
        self.is_pass_through() && self.is_read_only(vars)
    }

    /// Schedule a value modification for this variable.
    ///
    /// If [`is_pass_through`] pass the `modify` to the source variable, otherwise
    /// clones the source variable and modifies that value on the first call and then
    /// modified that cloned value in subsequent calls.
    ///
    /// Can return an error only if [`is_pass_through`], otherwise always succeeds.
    ///
    /// [`is_pass_through`]: Self::is_pass_through
    #[inline]
    pub fn modify<Vw, M>(&self, vars: &Vw, modify: M) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        M: FnOnce(&mut VarModify<T>) + 'static,
    {
        vars.with_vars(|vars| {
            if let Some(source) = self.source(vars) {
                if self.is_pass_through() {
                    return source.modify(vars, modify);
                }

                // SAFETY: this is safe because the `value` is not touched when `source` is some.
                let value = unsafe { &mut *self.0.value.get() };
                if value.is_none() {
                    *value = Some(source.get_clone(vars));
                    self.0.version.set(source.version(vars));
                }
            }

            let self_ = self.clone();
            vars.push_change(Box::new(move |update_id| {
                // SAFETY: this is safe because Vars requires a mutable reference to apply changes.
                // the `modifying` flag is only used for `deep_clone`.
                unsafe {
                    *self_.0.source.get() = None;
                }
                let mut guard = VarModify::new(unsafe { &mut *self_.0.value.get() }.as_mut().unwrap());
                modify(&mut guard);
                if guard.touched() {
                    self_.0.last_update_id.set(update_id);
                    self_.0.version.set(self_.0.version.get().wrapping_add(1));
                }
                guard.touched()
            }));

            Ok(())
        })
    }

    /// Causes the variable to notify update without changing the value.
    ///
    /// This counts as a *write* so unless [`is_pass_through`] is `true` the value will
    /// be cloned and [`is_cloned`] set to `true` on touch.
    ///
    /// Can return an error only if [`is_pass_through`], otherwise always succeeds.
    ///
    /// [`is_pass_through`]: Self::is_pass_through
    /// [`is_cloned`]: Self::is_cloned
    #[inline]
    pub fn touch<Vw: WithVars>(&self, vars: &Vw) -> Result<(), VarIsReadOnly> {
        self.modify(vars, |v| v.touch())
    }

    /// Schedule a new value for this variable.
    ///
    /// If [`is_pass_through`] pass the `new_value` to the source variable, otherwise
    /// the `new_value` will become the variable value on the next update. Unlike [`modify`]
    /// and [`touch`] this method never clones the source variable.
    ///
    /// Can return an error only if [`is_pass_through`], otherwise always succeeds.
    ///
    /// [`is_pass_through`]: Self::is_pass_through
    /// [`modify`]: Self::modify
    /// [`touch`]: Self::touch
    #[inline]
    pub fn set<Vw, N>(&self, vars: &Vw, new_value: N) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<T>,
    {
        let new_value = new_value.into();
        vars.with_vars(|vars| {
            if let Some(source) = self.source(vars) {
                if self.is_pass_through() {
                    return source.set(vars, new_value);
                }

                // SAFETY: this is safe because the `value` is not touched when `source` is some.
                unsafe {
                    *self.0.value.get() = Some(new_value);
                }
                self.0.version.set(source.version(vars));

                let self_ = self.clone();
                vars.push_change(Box::new(move |update_id| {
                    // SAFETY: this is safe because Vars requires a mutable reference to apply changes.
                    // the `modifying` flag is only used for `deep_clone`.
                    unsafe {
                        *self_.0.source.get() = None;
                    }
                    self_.0.last_update_id.set(update_id);
                    self_.0.version.set(self_.0.version.get().wrapping_add(1));

                    true
                }));
            } else {
                let self_ = self.clone();
                vars.push_change(Box::new(move |update_id| {
                    // SAFETY: this is safe because Vars requires a mutable reference to apply changes.
                    // the `modifying` flag is only used for `deep_clone`.
                    unsafe {
                        *self_.0.value.get() = Some(new_value);
                    }
                    self_.0.last_update_id.set(update_id);
                    self_.0.version.set(self_.0.version.get().wrapping_add(1));

                    true
                }));
            }

            Ok(())
        })
    }
}
impl<T: VarValue, V: Var<T>> crate::private::Sealed for RcCowVar<T, V> {}
impl<T: VarValue, V: Var<T>> Var<T> for RcCowVar<T, V> {
    type AsReadOnly = ReadOnlyVar<T, Self>;

    fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a T {
        self.get(vars)
    }

    fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a T> {
        self.get_new(vars)
    }

    fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
        self.is_new(vars)
    }

    fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> u32 {
        self.version(vars)
    }

    fn is_read_only<Vw: WithVars>(&self, vars: &Vw) -> bool {
        self.is_read_only(vars)
    }

    fn strong_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }

    /// Returns `false` unless [`is_pass_through`] and the source variable is always read-only.
    ///
    /// [`is_pass_through`]: Self::is_pass_through
    fn always_read_only(&self) -> bool {
        self.is_pass_through() && self.0.source_always_read_only
    }

    /// Returns `true` unless [`is_pass_through`] and the source variable cannot update.
    ///
    /// [`is_pass_through`]: Self::is_pass_through
    fn can_update(&self) -> bool {
        !self.is_pass_through() || self.0.source_can_update
    }

    fn into_value<Vr: WithVarsRead>(self, vars: &Vr) -> T {
        match Rc::try_unwrap(self.0) {
            Ok(v) => {
                if let Some(source) = v.source.into_inner() {
                    source.into_value(vars)
                } else {
                    v.value.into_inner().unwrap()
                }
            }
            Err(v) => RcCowVar(v).get_clone(vars),
        }
    }

    fn modify<Vw, M>(&self, vars: &Vw, modify: M) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        M: FnOnce(&mut VarModify<T>) + 'static,
    {
        self.modify(vars, modify)
    }

    fn set<Vw, N>(&self, vars: &Vw, new_value: N) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<T>,
    {
        self.set(vars, new_value)
    }

    fn touch<Vw: WithVars>(&self, vars: &Vw) -> Result<(), VarIsReadOnly> {
        self.touch(vars)
    }

    fn into_read_only(self) -> Self::AsReadOnly {
        ReadOnlyVar::new(self)
    }

    fn update_mask(&self) -> UpdateMask {
        self.0.update_mask.clone()
    }
}
impl<T: VarValue, V: Var<T>> IntoVar<T> for RcCowVar<T, V> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}
