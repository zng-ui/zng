use std::{
    cell::{Cell, RefCell, UnsafeCell},
    rc::{Rc, Weak},
};

use once_cell::unsync::OnceCell;

use crate::widget_info::UpdateSlot;

use super::{animation::WeakAnimationHandle, *};

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
bitflags! {
    struct Flags: u8 {
        const SOURCE_ALWAYS_READ_ONLY = 0b_0000_0001;
        const SOURCE_CAN_UPDATE =       0b_0000_0010;
        const SOURCE_IS_CONTEXTUAL =    0b_0000_0100;
        const IS_PASS_THROUGH =         0b_0000_1000;
    }
}
struct CowData<T, V> {
    source: UnsafeCell<Option<V>>,
    flags: Cell<Flags>,
    update_mask: OnceCell<UpdateMask>,

    value: UnsafeCell<Option<T>>,
    version: VarVersionCell,
    animation: RefCell<(Option<WeakAnimationHandle>, u32)>,
    last_update_id: Cell<u32>,
}
impl<T: VarValue, V: Var<T>> Clone for RcCowVar<T, V> {
    fn clone(&self) -> Self {
        RcCowVar(Rc::clone(&self.0))
    }
}
impl<T: VarValue, V: Var<T>> RcCowVar<T, V> {
    /// Returns a new var that reads from `source`.
    pub fn new(source: V) -> Self {
        Self::new_(source, false)
    }

    /// Returns a new [`RcCowVar`] that **is not clone-on-write**.
    ///
    /// Modifying the returned variable modifies the `source`. You can use this to
    /// avoid boxing variables in methods that can return either the source variable
    /// or an override variable.
    pub fn pass_through(source: V) -> Self {
        Self::new_(source, true)
    }

    fn new_(source: V, is_pass_through: bool) -> Self {
        let mut flags = Flags::empty();
        if source.always_read_only() {
            flags.insert(Flags::SOURCE_ALWAYS_READ_ONLY);
        }
        if source.can_update() {
            flags.insert(Flags::SOURCE_CAN_UPDATE);
        }
        if source.is_contextual() {
            flags.insert(Flags::SOURCE_IS_CONTEXTUAL);
        }
        if is_pass_through {
            flags.insert(Flags::IS_PASS_THROUGH);
        }

        RcCowVar(Rc::new(CowData {
            update_mask: OnceCell::default(),
            flags: Cell::new(flags),
            source: UnsafeCell::new(Some(source)),
            value: UnsafeCell::new(None),
            version: VarVersionCell::new(0),
            animation: RefCell::new((None, 0)),
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
    pub fn is_pass_through(&self) -> bool {
        self.0.flags.get().contains(Flags::IS_PASS_THROUGH)
    }

    /// Returns a weak reference to the variable.
    pub fn downgrade(&self) -> WeakRcCowVar<T, V> {
        WeakRcCowVar(Rc::downgrade(&self.0))
    }
}
impl<T: VarValue, V: Var<T>> crate::private::Sealed for RcCowVar<T, V> {}
impl<T: VarValue, V: Var<T>> Var<T> for RcCowVar<T, V> {
    type AsReadOnly = types::ReadOnlyVar<T, Self>;

    fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a T {
        let vars = vars.as_ref();

        if let Some(source) = self.source(vars) {
            source.get(vars)
        } else {
            // SAFETY: this is safe because we are tying the `Vars` lifetime to the value
            // and we require `&mut Vars` to modify the value.
            unsafe { &*self.0.value.get() }.as_ref().unwrap()
        }
    }

    fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a T> {
        let vars = vars.as_ref();

        if let Some(source) = self.source(vars) {
            source.get_new(vars)
        } else if self.0.last_update_id.get() == vars.update_id() {
            Some(self.get(vars))
        } else {
            None
        }
    }

    fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
        vars.with_vars(|vars| {
            if let Some(source) = self.source(vars) {
                source.is_new(vars)
            } else {
                self.0.last_update_id.get() == vars.update_id()
            }
        })
    }

    fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> VarVersion {
        vars.with_vars_read(|vars| {
            if let Some(source) = self.source(vars) {
                source.version(vars)
            } else {
                self.0.version.get()
            }
        })
    }

    fn is_read_only<Vw: WithVars>(&self, vars: &Vw) -> bool {
        self.is_pass_through() && self.is_read_only(vars)
    }

    fn is_animating<Vr: WithVarsRead>(&self, vars: &Vr) -> bool {
        vars.with_vars_read(|vars| {
            if let Some(s) = self.source(vars) {
                s.is_animating(vars)
            } else {
                self.0.animation.borrow().0.as_ref().and_then(|w| w.upgrade()).is_some()
            }
        })
    }

    /// Returns `false` unless [`is_pass_through`] and the source variable is always read-only.
    ///
    /// [`is_pass_through`]: Self::is_pass_through
    fn always_read_only(&self) -> bool {
        self.is_pass_through() && self.0.flags.get().contains(Flags::SOURCE_ALWAYS_READ_ONLY)
    }

    /// Returns `true` if is still reading from the source variable and it is contextual, otherwise returns `false`.
    fn is_contextual(&self) -> bool {
        self.0.flags.get().contains(Flags::SOURCE_IS_CONTEXTUAL)
    }

    fn actual_var<Vw: WithVars>(&self, vars: &Vw) -> BoxedVar<T> {
        vars.with_vars(|vars| {
            if let Some(source) = self.source(vars) {
                if self.is_pass_through() {
                    return source.actual_var(vars);
                }

                if self.is_contextual() {
                    // stop being contextual.
                    let _ = self.touch(vars);
                }
            }

            self.clone().boxed()
        })
    }

    /// Returns `true` unless [`is_pass_through`] and the source variable cannot update.
    ///
    /// [`is_pass_through`]: Self::is_pass_through
    fn can_update(&self) -> bool {
        !self.is_pass_through() || self.0.flags.get().contains(Flags::SOURCE_CAN_UPDATE)
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
        M: FnOnce(VarModify<T>) + 'static,
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
            let (animation, started_in) = vars.current_animation();
            vars.push_change(self_, move |self_, update_id| {
                let mut prev_animation = self_.0.animation.borrow_mut();

                if prev_animation.1 > started_in {
                    // change caused by overwritten animation.
                    return UpdateMask::none();
                }

                // SAFETY: this is safe because Vars requires a mutable reference to apply changes.
                unsafe {
                    *self_.0.source.get() = None;
                }

                let mut flags = self_.0.flags.get();
                // rust-analyzer gets confused if we try `flags.set`
                Flags::set(&mut flags, Flags::SOURCE_IS_CONTEXTUAL, false);
                self_.0.flags.set(flags);

                let mut touched = false;
                modify(VarModify::new(unsafe { &mut *self_.0.value.get() }.as_mut().unwrap(), &mut touched));
                if touched {
                    self_.0.last_update_id.set(update_id);
                    self_.0.version.set(self_.0.version.get().wrapping_add(1));

                    *prev_animation = (animation, started_in);

                    *self_.0.update_mask.get_or_init(|| UpdateSlot::next().mask())
                } else {
                    UpdateMask::none()
                }
            });

            Ok(())
        })
    }

    fn into_read_only(self) -> Self::AsReadOnly {
        types::ReadOnlyVar::new(self)
    }

    fn update_mask<Vr: WithVarsRead>(&self, vars: &Vr) -> UpdateMask {
        *vars.with_vars_read(|vars| {
            self.0.update_mask.get_or_init(|| {
                if let Some(source) = self.source(vars) {
                    source.update_mask(vars)
                } else {
                    UpdateSlot::next().mask()
                }
            })
        })
    }

    type Weak = WeakRcCowVar<T, V>;

    fn is_rc(&self) -> bool {
        true
    }

    fn downgrade(&self) -> Option<Self::Weak> {
        Some(self.downgrade())
    }

    fn strong_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }

    fn weak_count(&self) -> usize {
        Rc::weak_count(&self.0)
    }

    fn as_ptr(&self) -> *const () {
        Rc::as_ptr(&self.0) as _
    }
}
impl<T: VarValue, V: Var<T>> IntoVar<T> for RcCowVar<T, V> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}
impl<T: VarValue, V: Var<T>> any::AnyVar for RcCowVar<T, V> {
    fn into_any(self) -> Box<dyn any::AnyVar> {
        Box::new(self)
    }

    any_var_impls!(Var);
}

/// A weak reference to a [`RcCowVar`].
pub struct WeakRcCowVar<T: VarValue, V: Var<T>>(Weak<CowData<T, V>>);
impl<T: VarValue, V: Var<T>> crate::private::Sealed for WeakRcCowVar<T, V> {}
impl<T: VarValue, V: Var<T>> Clone for WeakRcCowVar<T, V> {
    fn clone(&self) -> Self {
        WeakRcCowVar(self.0.clone())
    }
}
impl<T: VarValue, V: Var<T>> any::AnyWeakVar for WeakRcCowVar<T, V> {
    fn into_any(self) -> Box<dyn any::AnyWeakVar> {
        Box::new(self)
    }

    any_var_impls!(WeakVar);
}
impl<T: VarValue, V: Var<T>> WeakVar<T> for WeakRcCowVar<T, V> {
    type Strong = RcCowVar<T, V>;

    fn upgrade(&self) -> Option<Self::Strong> {
        self.0.upgrade().map(RcCowVar)
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
