use std::{cell::Ref, marker::PhantomData, rc::Weak};

use super::*;

crate::context::context_value! {
    /// Signal for nested ContextualizedVar.
    static INIT_CONTEXT: bool = false;
}
fn init<S>(init: &dyn Fn() -> S) -> S {
    INIT_CONTEXT.with_context(&mut Some(true), init)
}

/// Represents a variable that delays initialization until the first usage.
///
/// Usage that initializes the variable are all [`AnyVar`] and [`Var<T>`] methods except `read_only`, `downgrade` and `boxed`.
/// Clones of this variable are always not initialized and re-init on first usage.
///
/// This variable is used in the [`Var::map`] and other mapping methods to support mapping from [`ContextVar<T>`].
///
/// ```
/// # macro_rules! fake{($($tt:tt)*) => {}}
/// # fake! {
/// let wgt = my_wgt! {
///     my_property = MY_CTX_VAR.map(|&b| !b);
/// };
/// # }
/// ```
///
/// In the example above the mapping var will bind with the `MY_CTX_VAR` context inside the property node, not
/// the context at the moment the widget is instantiated.
pub struct ContextualizedVar<T, S> {
    _type: PhantomData<T>,
    init: Rc<dyn Fn() -> S>,
    actual: RefCell<Option<S>>,
}
impl<T: VarValue, S: Var<T>> ContextualizedVar<T, S> {
    /// New with initialization function.
    ///
    /// The `init` closure will be called on the first usage of the var, once after the var is cloned and any time
    /// a parent contextualized var is initializing.
    pub fn new(init: Rc<dyn Fn() -> S>) -> Self {
        Self {
            _type: PhantomData,
            init,
            actual: RefCell::new(None),
        }
    }

    /// Borrow/initialize the actual var.
    pub fn borrow_init(&self) -> Ref<S> {
        let act = self.actual.borrow();
        if act.is_some() && !INIT_CONTEXT.get() {
            return Ref::map(act, |opt| opt.as_ref().unwrap());
        }

        drop(act);

        let act = init(&*self.init);
        *self.actual.borrow_mut() = Some(act);

        let act = self.actual.borrow();
        Ref::map(act, |opt| opt.as_ref().unwrap())
    }

    /// Unwraps the initialized actual var or initializes it now.
    pub fn into_init(self) -> S {
        match self.actual.into_inner() {
            Some(s) if !INIT_CONTEXT.get() => s,
            _ => init(&*self.init),
        }
    }

    /// Clone the variable initialization, but not the inited actual var, the clone can than
    /// be inited in a different context.
    #[allow(clippy::should_implement_trait)]
    pub fn clone(&self) -> Self {
        // highlight docs and removes "redundant" clone warnings.
        Clone::clone(self)
    }
}

/// Weak var that upgrades to an uninitialized [`ContextualizedVar<T, S>`].
pub struct WeakContextualizedVar<T, S> {
    _type: PhantomData<T>,
    init: Weak<dyn Fn() -> S>,
}
impl<T: VarValue, S: Var<T>> WeakContextualizedVar<T, S> {
    /// New with weak init function.
    pub fn new(init: Weak<dyn Fn() -> S>) -> Self {
        Self { _type: PhantomData, init }
    }
}

impl<T: VarValue, S: Var<T>> Clone for ContextualizedVar<T, S> {
    fn clone(&self) -> Self {
        Self {
            _type: PhantomData,
            init: self.init.clone(),
            actual: RefCell::new(None),
        }
    }
}
impl<T: VarValue, S: Var<T>> Clone for WeakContextualizedVar<T, S> {
    fn clone(&self) -> Self {
        Self {
            _type: PhantomData,
            init: self.init.clone(),
        }
    }
}

impl<T: VarValue, S: Var<T>> crate::private::Sealed for ContextualizedVar<T, S> {}
impl<T: VarValue, S: Var<T>> crate::private::Sealed for WeakContextualizedVar<T, S> {}

impl<T: VarValue, S: Var<T>> AnyVar for ContextualizedVar<T, S> {
    fn clone_any(&self) -> BoxedAnyVar {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn double_boxed_any(self: Box<Self>) -> Box<dyn Any> {
        let me: BoxedVar<T> = self;
        Box::new(me)
    }

    fn var_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn get_any(&self) -> Box<dyn AnyVarValue> {
        Box::new(self.get())
    }

    fn set_any(&self, vars: &Vars, value: Box<dyn AnyVarValue>) -> Result<(), VarIsReadOnlyError> {
        self.modify(vars, var_set_any(value))
    }

    fn last_update(&self) -> VarUpdateId {
        self.borrow_init().last_update()
    }

    fn capabilities(&self) -> VarCapabilities {
        self.borrow_init().capabilities()
    }

    fn hook(&self, pos_modify_action: Box<dyn Fn(&Vars, &mut Updates, &dyn AnyVarValue) -> bool>) -> VarHandle {
        self.borrow_init().hook(pos_modify_action)
    }

    fn strong_count(&self) -> usize {
        Rc::strong_count(&self.init)
    }

    fn weak_count(&self) -> usize {
        Rc::weak_count(&self.init)
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        self.borrow_init().actual_var_any()
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        Box::new(self.downgrade())
    }

    fn is_animating(&self) -> bool {
        self.borrow_init().is_animating()
    }

    fn var_ptr(&self) -> VarPtr {
        VarPtr::new_rc(&self.init)
    }
}
impl<T: VarValue, S: Var<T>> AnyWeakVar for WeakContextualizedVar<T, S> {
    fn clone_any(&self) -> BoxedAnyWeakVar {
        Box::new(self.clone())
    }

    fn strong_count(&self) -> usize {
        self.init.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.init.weak_count()
    }

    fn upgrade_any(&self) -> Option<BoxedAnyVar> {
        self.upgrade().map(|c| Box::new(c) as _)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<T: VarValue, S: Var<T>> IntoVar<T> for ContextualizedVar<T, S> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T: VarValue, S: Var<T>> Var<T> for ContextualizedVar<T, S> {
    type ReadOnly = types::ReadOnlyVar<T, Self>;

    type ActualVar = S::ActualVar;

    type Downgrade = WeakContextualizedVar<T, S>;

    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.borrow_init().with(read)
    }

    fn modify<V, F>(&self, vars: &V, modify: F) -> Result<(), VarIsReadOnlyError>
    where
        V: WithVars,
        F: FnOnce(&mut VarModifyValue<T>) + 'static,
    {
        self.borrow_init().modify(vars, modify)
    }

    fn actual_var(self) -> Self::ActualVar {
        self.borrow_init();
        self.actual.into_inner().unwrap().actual_var()
    }

    fn downgrade(&self) -> Self::Downgrade {
        WeakContextualizedVar::new(Rc::downgrade(&self.init))
    }

    fn into_value(self) -> T {
        match self.actual.into_inner() {
            Some(act) if !INIT_CONTEXT.get() => act.into_value(),
            _ => init(&*self.init).into_value(),
        }
    }

    fn read_only(&self) -> Self::ReadOnly {
        types::ReadOnlyVar::new(self.clone())
    }
}
impl<T: VarValue, S: Var<T>> WeakVar<T> for WeakContextualizedVar<T, S> {
    type Upgrade = ContextualizedVar<T, S>;

    fn upgrade(&self) -> Option<Self::Upgrade> {
        self.init.upgrade().map(ContextualizedVar::new)
    }
}
