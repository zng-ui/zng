use std::{
    cell::{Cell, RefCell, UnsafeCell},
    rc::{Rc, Weak},
};

use crate::crate_util::RunOnDrop;
use crate::widget_info::UpdateSlot;

use super::{easing::WeakAnimationHandle, *};

/// A [`Var`] that is a [`Rc`] pointer to its value.
pub struct RcVar<T: VarValue>(Rc<Data<T>>);
struct Data<T> {
    value: UnsafeCell<T>,
    modifying: Cell<bool>,
    animation: RefCell<(WeakAnimationHandle, u32)>,
    last_update_id: Cell<u32>,
    version: Cell<u32>,
    update_slot: UpdateSlot,
}
impl<T: Clone> Clone for Data<T> {
    fn clone(&self) -> Self {
        if self.modifying.get() {
            panic!("cannot `deep_clone`, value is mutable borrowed")
        }
        // SAFETY: we panic if `value` is exclusive borrowed.
        let value = unsafe { (&*self.value.get()).clone() };
        Data {
            value: UnsafeCell::new(value),
            modifying: Cell::new(false),
            animation: RefCell::new((WeakAnimationHandle::new(), 0)),
            last_update_id: Cell::new(self.last_update_id.get()),
            version: Cell::new(self.version.get()),
            update_slot: self.update_slot,
        }
    }
}
impl<T: VarValue> RcVar<T> {
    /// New [`RcVar`].
    ///
    /// You can also use the [`var`] function to initialize.
    pub fn new(initial_value: T) -> Self {
        RcVar(Rc::new(Data {
            value: UnsafeCell::new(initial_value),
            modifying: Cell::new(false),
            animation: RefCell::new((WeakAnimationHandle::new(), 0)),
            last_update_id: Cell::new(0),
            version: Cell::new(0),
            update_slot: UpdateSlot::next(),
        }))
    }

    /// Schedule a value modification for this variable.
    #[inline]
    pub fn modify<Vw, M>(&self, vars: &Vw, modify: M)
    where
        Vw: WithVars,
        M: FnOnce(VarModify<T>) + 'static,
    {
        vars.with_vars(|vars| {
            let self_ = self.clone();
            let (animation, started_in) = vars.current_animation();
            vars.push_change::<T>(Box::new(move |update_id| {
                let mut prev_animation = self_.0.animation.borrow_mut();

                if prev_animation.1 > started_in {
                    // change caused by overwritten animation.
                    return UpdateMask::none();
                }

                self_.0.modifying.set(true);
                let _drop = RunOnDrop::new(|| self_.0.modifying.set(false));

                // SAFETY: this is safe because Vars requires a mutable reference to apply changes.
                // the `modifying` flag is only used for `deep_clone`.
                let mut touched = false;
                modify(VarModify::new(unsafe { &mut *self_.0.value.get() }, &mut touched));
                if touched {
                    self_.0.last_update_id.set(update_id);
                    self_.0.version.set(self_.0.version.get().wrapping_add(1));

                    *prev_animation = (animation, started_in);

                    self_.0.update_slot.mask()
                } else {
                    UpdateMask::none()
                }
            }));
        })
    }

    /// Causes the variable to notify update without changing the value.
    #[inline]
    pub fn touch<Vw: WithVars>(&self, vars: &Vw) {
        self.modify(vars, |mut v| v.touch());
    }

    /// Schedule a new value for this variable.
    #[inline]
    pub fn set<Vw, N>(&self, vars: &Vw, new_value: N)
    where
        Vw: WithVars,
        N: Into<T>,
    {
        let new_value = new_value.into();
        self.modify(vars, move |mut v| *v = new_value)
    }

    /// Schedule a new value for this variable, the variable will only be set if
    /// the value is not equal to `new_value`.
    #[inline]
    pub fn set_ne<Vw, N>(&self, vars: &Vw, new_value: N) -> bool
    where
        Vw: WithVars,
        N: Into<T>,
        T: PartialEq,
    {
        vars.with_vars(|vars| {
            let new_value = new_value.into();
            if self.get(vars) != &new_value {
                self.set(vars, new_value);
                true
            } else {
                false
            }
        })
    }

    /// Returns a weak reference to the variable.
    #[inline]
    pub fn downgrade(&self) -> WeakRcVar<T> {
        WeakRcVar(Rc::downgrade(&self.0))
    }

    /// Create a detached var with a clone of the current value.
    ///
    /// # Panics
    ///
    /// Panics is called inside a [`modify`] callback.
    ///
    /// [`modify`]: Self::modify
    pub fn deep_clone(&self) -> RcVar<T> {
        let mut rc = Rc::clone(&self.0);
        let _ = Rc::make_mut(&mut rc);
        RcVar(rc)
    }
}
impl<T: VarValue> Clone for RcVar<T> {
    fn clone(&self) -> Self {
        RcVar(Rc::clone(&self.0))
    }
}
impl<T: VarValue + Default> Default for RcVar<T> {
    fn default() -> Self {
        var(T::default())
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

/// New [`RcVar`] with [default] initial value.
///
/// [default]: Default
#[inline]
pub fn var_default<T: VarValue + Default>() -> RcVar<T> {
    RcVar::new(T::default())
}

/// A weak reference to a [`RcVar`].
pub struct WeakRcVar<T: VarValue>(Weak<Data<T>>);
impl<T: VarValue> crate::private::Sealed for WeakRcVar<T> {}
impl<T: VarValue> Clone for WeakRcVar<T> {
    fn clone(&self) -> Self {
        WeakRcVar(self.0.clone())
    }
}
impl<T: VarValue> WeakVar<T> for WeakRcVar<T> {
    type Strong = RcVar<T>;

    fn upgrade(&self) -> Option<Self::Strong> {
        self.0.upgrade().map(RcVar)
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

impl<T: VarValue> crate::private::Sealed for RcVar<T> {}
impl<T: VarValue> Var<T> for RcVar<T> {
    type AsReadOnly = types::ReadOnlyVar<T, Self>;
    type Weak = WeakRcVar<T>;

    #[inline]
    fn get<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> &'a T {
        let _vars = vars.as_ref();
        // SAFETY: this is safe because we are tying the `Vars` lifetime to the value
        // and we require `&mut Vars` to modify the value.
        unsafe { &*self.0.value.get() }
    }

    #[inline]
    fn get_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a T> {
        let vars = vars.as_ref();
        if self.0.last_update_id.get() == vars.update_id() {
            Some(self.get(vars))
        } else {
            None
        }
    }

    #[inline]
    fn into_value<Vr: WithVarsRead>(self, vars: &Vr) -> T {
        match Rc::try_unwrap(self.0) {
            Ok(v) => v.value.into_inner(),
            Err(v) => RcVar(v).get_clone(vars),
        }
    }

    #[inline]
    fn is_new<Vw: WithVars>(&self, vars: &Vw) -> bool {
        vars.with_vars(|vars| self.0.last_update_id.get() == vars.update_id())
    }

    #[inline]
    fn version<Vr: WithVarsRead>(&self, vars: &Vr) -> VarVersion {
        vars.with_vars_read(|_| VarVersion::normal(self.0.version.get()))
    }

    #[inline]
    fn is_read_only<Vw: WithVars>(&self, _: &Vw) -> bool {
        false
    }

    #[inline]
    fn is_animating<Vr: WithVarsRead>(&self, _: &Vr) -> bool {
        self.0.animation.borrow().0.upgrade().is_some()
    }

    #[inline]
    fn always_read_only(&self) -> bool {
        false
    }

    #[inline]
    fn can_update(&self) -> bool {
        true
    }

    #[inline]
    fn is_contextual(&self) -> bool {
        false
    }

    #[inline]
    fn actual_var<Vw: WithVars>(&self, _: &Vw) -> BoxedVar<T> {
        self.clone().boxed()
    }

    #[inline]
    fn downgrade(&self) -> Option<Self::Weak> {
        Some(self.downgrade())
    }

    #[inline]
    fn strong_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }

    #[inline]
    fn weak_count(&self) -> usize {
        Rc::weak_count(&self.0)
    }

    #[inline]
    fn as_ptr(&self) -> *const () {
        Rc::as_ptr(&self.0) as _
    }

    #[inline]
    fn is_rc(&self) -> bool {
        true
    }

    #[inline]
    fn modify<Vw, M>(&self, vars: &Vw, modify: M) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        M: FnOnce(VarModify<T>) + 'static,
    {
        self.modify(vars, modify);
        Ok(())
    }

    #[inline]
    fn set<Vw, N>(&self, vars: &Vw, new_value: N) -> Result<(), VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<T>,
    {
        self.set(vars, new_value);
        Ok(())
    }

    #[inline]
    fn set_ne<Vw, N>(&self, vars: &Vw, new_value: N) -> Result<bool, VarIsReadOnly>
    where
        Vw: WithVars,
        N: Into<T>,
        T: PartialEq,
    {
        Ok(self.set_ne(vars, new_value))
    }

    #[inline]
    fn into_read_only(self) -> Self::AsReadOnly {
        types::ReadOnlyVar::new(self)
    }

    #[inline]
    fn update_mask<Vr: WithVarsRead>(&self, _: &Vr) -> UpdateMask {
        self.0.update_slot.mask()
    }
}
impl<T: VarValue> IntoVar<T> for RcVar<T> {
    type Var = Self;

    #[inline]
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
pub type ResponseVar<T> = types::ReadOnlyVar<Response<T>, RcVar<Response<T>>>;

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
    pub fn rsp<'a, Vr: AsRef<VarsRead>>(&'a self, vars: &'a Vr) -> Option<&'a T> {
        match self.get(vars) {
            Response::Waiting => None,
            Response::Done(r) => Some(r),
        }
    }

    /// Returns a future that awaits until a response is received.
    ///
    /// Note that you must call [`rsp`] after this to get the response.
    ///
    /// [`rsp`]: Self::rsp
    pub async fn wait_rsp<Vr: WithVars>(&self, vars: &Vr) {
        while vars.with_vars(|v| self.rsp(v).is_none()) {
            self.wait_new(vars).await;
        }
    }

    /// References the response value if a response was set for this update.
    pub fn rsp_new<'a, Vw: AsRef<Vars>>(&'a self, vars: &'a Vw) -> Option<&'a T> {
        if let Some(new) = self.get_new(vars) {
            match new {
                Response::Waiting => None,
                Response::Done(r) => Some(r),
            }
        } else {
            None
        }
    }

    /// If the variable contains a response.
    #[inline]
    pub fn responded<Vr: WithVarsRead>(&self, vars: &Vr) -> bool {
        vars.with_vars_read(|vars| self.rsp(vars).is_some())
    }

    /// Copy the response value if a response was set.
    #[inline]
    pub fn rsp_copy<Vr: WithVarsRead>(&self, vars: &Vr) -> Option<T>
    where
        T: Copy,
    {
        vars.with_vars_read(|vars| self.rsp(vars).copied())
    }

    /// Returns a future that awaits until a response is received, returns a copy of the response.
    pub async fn wait_rsp_copy<Vw: WithVars>(&self, vars: &Vw) -> T
    where
        T: Copy,
    {
        loop {
            if let Some(v) = vars.with_vars(|v| self.rsp_copy(v)) {
                return v;
            }
            self.wait_new(vars).await;
        }
    }

    /// Clone the response value if a response was set.
    #[inline]
    pub fn rsp_clone<Vr: WithVarsRead>(&self, vars: &Vr) -> Option<T> {
        vars.with_vars_read(|vars| self.rsp(vars).cloned())
    }

    /// Returns a future that awaits until a response is received, returns a clone of the response.
    pub async fn wait_rsp_clone<Vw: WithVars>(&self, vars: &Vw) -> T {
        loop {
            if let Some(v) = vars.with_vars(|v| self.rsp_clone(v)) {
                return v;
            }
            self.wait_new(vars).await;
        }
    }

    /// Copy the response value if a response was set for this update.
    #[inline]
    pub fn rsp_new_copy<Vw: WithVars>(self, vars: &Vw) -> Option<T>
    where
        T: Copy,
    {
        vars.with_vars(|vars| self.rsp_new(vars).copied())
    }

    /// Clone the response value if a response was set for this update.
    #[inline]
    pub fn rsp_new_clone<Vw: WithVars>(self, vars: &Vw) -> Option<T> {
        vars.with_vars(|vars| self.rsp_new(vars).cloned())
    }

    /// If the variable has responded returns the response value or a clone of it if `self` is not the only reference to the response.
    /// If the variable has **not** responded returns `self` in the error.
    #[inline]
    pub fn try_into_rsp<Vr: WithVarsRead>(self, vars: &Vr) -> Result<T, Self> {
        vars.with_vars_read(|vars| {
            if self.responded(vars) {
                match self.into_value(vars) {
                    Response::Done(r) => Ok(r),
                    Response::Waiting => unreachable!(),
                }
            } else {
                Err(self)
            }
        })
    }

    /// Map the response value using `map`, if the variable is awaiting a response uses the `waiting_value` first.
    #[inline]
    pub fn map_rsp<O, I, M>(&self, waiting_value: I, map: M) -> impl Var<O>
    where
        O: VarValue,
        I: FnOnce() -> O + 'static,
        M: FnOnce(&T) -> O + 'static,
    {
        let mut map = Some(map);
        self.filter_map(
            move |_| waiting_value(),
            move |r| match r {
                Response::Waiting => None,
                Response::Done(r) => map.take().map(|m| m(r)),
            },
        )
    }
}

impl<T: VarValue> ResponderVar<T> {
    /// Sets the one time response.
    ///
    /// # Panics
    ///
    /// Panics if the variable is already in the done state.
    #[inline]
    pub fn respond<'a, Vw: WithVars>(&'a self, vars: &'a Vw, response: T) {
        vars.with_vars(|vars| {
            if let Response::Done(_) = self.get(vars) {
                panic!("already responded");
            }
            self.set(vars, Response::Done(response));
        })
    }

    /// Creates a [`ResponseVar`] linked to this responder.
    #[inline]
    pub fn response_var(&self) -> ResponseVar<T> {
        self.clone().into_read_only()
    }
}

/// A [`ReadOnlyVar`](types::ReadOnlyVar) wrapping an [`RcVar`].
pub type ReadOnlyRcVar<T> = types::ReadOnlyVar<T, RcVar<T>>;
