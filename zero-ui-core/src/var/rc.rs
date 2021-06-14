use std::{
    cell::{Cell, UnsafeCell},
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
            guard.touched()
        }));
    }

    /// Causes the variable to notify update without changing the value.
    #[inline]
    pub fn touch(&self, vars: &Vars) {
        self.modify(vars, |v| v.touch());
    }

    /// Schedule a new value for this variable.
    #[inline]
    pub fn set<N>(&self, vars: &Vars, new_value: N)
    where
        N: Into<T>,
    {
        let new_value = new_value.into();
        self.modify(vars, move |v| **v = new_value)
    }

    /// Schedule a new value for this variable, the variable will only be set if
    /// the value is not equal to `new_value`.
    #[inline]
    pub fn set_ne<N>(&self, vars: &Vars, new_value: N) -> bool
    where
        N: Into<T>,
        T: PartialEq,
    {
        let new_value = new_value.into();
        if self.get(vars) != &new_value {
            self.set(vars, new_value);
            true
        } else {
            false
        }
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

    fn set<N>(&self, vars: &Vars, new_value: N) -> Result<(), VarIsReadOnly>
    where
        N: Into<T>,
    {
        self.set(vars, new_value);
        Ok(())
    }

    fn set_ne<N>(&self, vars: &Vars, new_value: N) -> Result<bool, VarIsReadOnly>
    where
        N: Into<T>,
        T: PartialEq,
    {
        Ok(self.set_ne(vars, new_value))
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
