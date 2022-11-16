use std::sync::{Arc, Weak};

use super::*;

///<span data-del-macro-root></span> Initializes a new conditional var.
///
/// A condition var updates when the first `true` condition changes or the mapped var for the current condition changes.
///
/// # Syntax
///
/// The macro expects a list of `condition-var => condition-value-var`, the list is separated by comma.
/// The last condition must be the `_` token that maps to the value for when none of the conditions are `true`.
///
/// The `condition-var` must be an expression that evaluates to an `impl Var<bool>` type. The `condition-value-var` must
/// by any type that implements `IntoVar`. All condition values must be of the same [`VarValue`] type.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::var::*;
/// # use zero_ui_core::text::*;
/// # fn text(text: impl IntoVar<Text>) { }
/// let condition = var(true);
/// let when_false = var("condition: false".to_text());
///
/// let t = text(when_var! {
///     condition.clone() => "condition: true".to_text(),
///     _ => when_false.clone(),
/// });
/// ```
///
/// In the example if `condition` or `when_false` are modified the text updates.
///
/// # `cfg`
///
/// Every condition can be annotated with attributes, including `#[cfg(..)]`.
///
/// ```
/// # use zero_ui_core::var::*;
/// # use zero_ui_core::text::*;
/// # fn text(text: impl IntoVar<Text>) { }
/// # let condition0 = var(true);
/// # let condition1 = var(true);
/// let t = text(when_var! {
///     #[cfg(some_flag)]
///     condition0 => "is condition 0".to_text(),
///     #[cfg(not(some_flag))]
///     condition1 => "is condition 1".to_text(),
///     _ => "is default".to_text(),
/// });
/// ```
///
/// In the example above only one of the conditions will be compiled, the generated variable is the same
/// type as if you had written a single condition.
///
/// # Contextualized
///
/// The when var is contextualized, meaning is a [`ContextVar<T>`] is used for one of the inputs it will be resolved to the
/// context where the merge is first used, not where it is created. The full output type of this macro is `ContextualizedVar<T, RcWhenVar<T>>`.
#[macro_export]
macro_rules! when_var {
    ($($tt:tt)*) => {
        $crate::var::types::__when_var! {
            $crate::var
            $($tt)*
        }
    }
}

#[doc(inline)]
pub use crate::when_var;

use parking_lot::Mutex;
#[doc(hidden)]
pub use zero_ui_proc_macros::when_var as __when_var;

#[doc(hidden)]
pub type ContextualizedRcWhenVar<T> = types::ContextualizedVar<T, RcWhenVar<T>>;

/// Manually build a [`RcWhenVar<T>`].
#[derive(Clone)]
pub struct WhenVarBuilder<T: VarValue> {
    default: BoxedVar<T>,
    conditions: Vec<(BoxedVar<bool>, BoxedVar<T>)>,
}
impl<T: VarValue> WhenVarBuilder<T> {
    /// Start building with the default value.
    pub fn new(default: impl IntoVar<T>) -> Self {
        Self {
            default: default.into_var().boxed(),
            conditions: vec![],
        }
    }

    /// Push a condition and value.
    pub fn push(&mut self, condition: impl IntoVar<bool>, value: impl IntoVar<T>) {
        self.conditions.push((condition.into_var().boxed(), value.into_var().boxed()));
    }

    /// Finish the build.
    pub fn build(mut self) -> RcWhenVar<T> {
        self.conditions.shrink_to_fit();
        for (c, v) in self.conditions.iter_mut() {
            fn panic_placeholder<T: VarValue>() -> BoxedVar<T> {
                types::ContextualizedVar::<T, BoxedVar<T>>::new(Arc::new(|| unreachable!())).boxed()
            }
            take_mut::take_or_recover(c, panic_placeholder::<bool>, Var::actual_var);
            take_mut::take_or_recover(v, panic_placeholder::<T>, Var::actual_var);
        }

        let rc_when = Arc::new(Data {
            default: self.default,
            conditions: self.conditions,
            w: Mutex::new(WhenData {
                input_handles: Box::new([]),
                hooks: vec![],
                last_update: VarUpdateId::never(),
                last_apply_request: VarApplyUpdateId::initial(),
                active: usize::MAX,
            }),
        });
        let wk_when = Arc::downgrade(&rc_when);

        {
            let mut data = rc_when.w.lock();
            let data = &mut *data;

            // capacity can be n*2+1, but we only bet on conditions being `NEW`.
            let mut input_handles = Vec::with_capacity(rc_when.conditions.len());
            if rc_when.default.capabilities().contains(VarCapabilities::NEW) {
                input_handles.push(rc_when.default.hook(RcWhenVar::handle_value(wk_when.clone(), usize::MAX)));
            }
            for (i, (c, v)) in rc_when.conditions.iter().enumerate() {
                if c.get() && data.active > i {
                    data.active = i;
                }

                if c.capabilities().contains(VarCapabilities::NEW) {
                    input_handles.push(c.hook(RcWhenVar::handle_condition(wk_when.clone(), i)));
                }
                if v.capabilities().contains(VarCapabilities::NEW) {
                    input_handles.push(v.hook(RcWhenVar::handle_value(wk_when.clone(), i)));
                }
            }

            data.input_handles = input_handles.into_boxed_slice();
        }

        RcWhenVar(rc_when)
    }

    /// Defer build to a [`types::ContextualizedVar`] first use.
    pub fn contextualized_build(self) -> types::ContextualizedVar<T, RcWhenVar<T>> {
        types::ContextualizedVar::new(Arc::new(move || self.clone().build()))
    }
}

/// Manually build a [`RcWhenVar<T>`] from type erased parts.
pub struct AnyWhenVarBuilder {
    default: BoxedAnyVar,
    conditions: Vec<(BoxedVar<bool>, BoxedAnyVar)>,
}
impl AnyWhenVarBuilder {
    /// Start building with only the default value.
    pub fn new<O: VarValue>(default: impl IntoVar<O>) -> Self {
        Self::new_any(default.into_var().boxed_any())
    }

    /// Start building with already boxed var.
    pub fn new_any(default: BoxedAnyVar) -> AnyWhenVarBuilder {
        Self {
            default,
            conditions: vec![],
        }
    }

    /// Create a builder from the parts of a formed [`when_var!`].
    pub fn from_var<O: VarValue>(var: &types::ContextualizedVar<O, RcWhenVar<O>>) -> Self {
        let var = var.borrow_init();
        Self {
            default: var.0.default.clone_any(),
            conditions: var.0.conditions.iter().map(|(c, v)| (c.clone(), v.clone_any())).collect(),
        }
    }

    /// Returns the number of conditions set.
    pub fn condition_count(&self) -> usize {
        self.conditions.len()
    }

    /// Set/replace the default value.
    pub fn set_default<O: VarValue>(&mut self, default: impl IntoVar<O>) {
        self.set_default_any(default.into_var().boxed_any());
    }

    /// Set/replace the default value with an already typed erased var.
    pub fn set_default_any(&mut self, default: BoxedAnyVar) {
        self.default = default;
    }

    /// Push a when condition.
    pub fn push<C, O, V>(&mut self, condition: C, value: V)
    where
        C: Var<bool>,
        O: VarValue,
        V: IntoVar<O>,
    {
        self.push_any(condition.boxed(), value.into_var().boxed_any())
    }

    /// Push a when condition already boxed and type erased.
    pub fn push_any(&mut self, condition: BoxedVar<bool>, value: BoxedAnyVar) {
        self.conditions.push((condition, value));
    }

    /// Replace the default value if `other` has default and extend the conditions with clones of `other`.
    pub fn replace_extend(&mut self, other: &Self) {
        self.default = other.default.clone_any();
        self.extend(other);
    }

    /// Extend the conditions with clones of `other`.
    pub fn extend(&mut self, other: &Self) {
        for (c, v) in other.conditions.iter() {
            self.conditions.push((c.clone(), v.clone_any()));
        }
    }

    /// Build the when var if all value variables are of type [`BoxedVar<T>`].
    pub fn build<T: VarValue>(&self) -> Option<RcWhenVar<T>> {
        let default = *self.default.clone().double_boxed_any().downcast::<BoxedVar<T>>().ok()?;

        let mut when = WhenVarBuilder::new(default);

        for (c, v) in &self.conditions {
            let value = *v.clone().double_boxed_any().downcast::<BoxedVar<T>>().ok()?;

            when.push(c.clone(), value);
        }

        Some(when.build())
    }

    /// Defer build to a [`types::ContextualizedVar`] first use.
    pub fn contextualized_build<T: VarValue>(self) -> Option<types::ContextualizedVar<T, RcWhenVar<T>>> {
        if self.default.var_type_id() == TypeId::of::<T>() {
            Some(types::ContextualizedVar::new(Arc::new(move || self.build().unwrap())))
        } else {
            None
        }
    }
}
impl fmt::Debug for AnyWhenVarBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnyWhenVarBuilder")
            .field("condition_count", &self.condition_count())
            .finish_non_exhaustive()
    }
}
impl Clone for AnyWhenVarBuilder {
    fn clone(&self) -> Self {
        Self {
            default: self.default.clone_any(),
            conditions: self.conditions.iter().map(|(c, v)| (c.clone(), v.clone_any())).collect(),
        }
    }
}

struct WhenData {
    input_handles: Box<[VarHandle]>,
    hooks: Vec<VarHook>,
    last_update: VarUpdateId,
    last_apply_request: VarApplyUpdateId,
    active: usize,
}

struct Data<T> {
    default: BoxedVar<T>,
    conditions: Vec<(BoxedVar<bool>, BoxedVar<T>)>,
    w: Mutex<WhenData>,
}

/// See [`when_var!`].
pub struct RcWhenVar<T>(Arc<Data<T>>);

/// Weak reference to a [`RcWhenVar<T>`].
pub struct WeakWhenVar<T>(Weak<Data<T>>);

impl<T: VarValue> RcWhenVar<T> {
    fn active(&self) -> &BoxedVar<T> {
        let active = self.0.w.lock().active;
        if active == usize::MAX {
            &self.0.default
        } else {
            &self.0.conditions[active].1
        }
    }

    fn handle_condition(wk_when: Weak<Data<T>>, i: usize) -> Box<dyn Fn(&Vars, &mut Updates, &dyn AnyVarValue) -> bool + Send + Sync> {
        Box::new(move |vars, _, value| {
            if let Some(rc_when) = wk_when.upgrade() {
                let mut data_mut = rc_when.w.lock();
                let mut update = false;

                match data_mut.active.cmp(&i) {
                    std::cmp::Ordering::Equal => {
                        if let Some(&false) = value.as_any().downcast_ref::<bool>() {
                            update = true;
                        }
                    }
                    std::cmp::Ordering::Greater => {
                        if let Some(&true) = value.as_any().downcast_ref::<bool>() {
                            update = true;
                        }
                    }
                    std::cmp::Ordering::Less => {}
                }

                if update && data_mut.last_apply_request != vars.apply_update_id() {
                    data_mut.last_apply_request = vars.apply_update_id();
                    drop(data_mut);
                    vars.schedule_update(RcWhenVar::apply_update(rc_when));
                }

                true
            } else {
                false
            }
        })
    }

    fn handle_value(wk_when: Weak<Data<T>>, i: usize) -> Box<dyn Fn(&Vars, &mut Updates, &dyn AnyVarValue) -> bool + Send + Sync> {
        Box::new(move |vars, _, _| {
            if let Some(rc_when) = wk_when.upgrade() {
                let mut data_mut = rc_when.w.lock();
                if data_mut.active == i && data_mut.last_apply_request != vars.apply_update_id() {
                    data_mut.last_apply_request = vars.apply_update_id();
                    drop(data_mut);
                    vars.schedule_update(RcWhenVar::apply_update(rc_when));
                }
                true
            } else {
                false
            }
        })
    }

    fn apply_update(rc_merge: Arc<Data<T>>) -> VarUpdateFn {
        Box::new(move |vars, updates| {
            let mut data = rc_merge.w.lock();
            let data = &mut *data;

            data.active = usize::MAX;
            for (i, (c, _)) in rc_merge.conditions.iter().enumerate() {
                if c.get() {
                    data.active = i;
                    break;
                }
            }
            data.last_update = vars.update_id();

            let active = if data.active == usize::MAX {
                &rc_merge.default
            } else {
                &rc_merge.conditions[data.active].1
            };

            active.with(|value| {
                data.hooks.retain(|h| h.call(vars, updates, value));
            });
            updates.update_ext();
        })
    }
}

impl<T> Clone for RcWhenVar<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T> Clone for WeakWhenVar<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: VarValue> crate::private::Sealed for RcWhenVar<T> {}
impl<T: VarValue> crate::private::Sealed for WeakWhenVar<T> {}

impl<T: VarValue> AnyVar for RcWhenVar<T> {
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
        self.0.w.lock().last_update
    }

    fn capabilities(&self) -> VarCapabilities {
        if self.0.conditions.is_empty() {
            self.0.default.capabilities()
        } else {
            self.active().capabilities() | VarCapabilities::NEW | VarCapabilities::CAPS_CHANGE
        }
    }

    fn hook(&self, pos_modify_action: Box<dyn Fn(&Vars, &mut Updates, &dyn AnyVarValue) -> bool + Send + Sync>) -> VarHandle {
        let (handle, hook) = VarHandle::new(pos_modify_action);
        self.0.w.lock().hooks.push(hook);
        handle
    }

    fn strong_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }

    fn weak_count(&self) -> usize {
        Arc::weak_count(&self.0)
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        self.clone_any()
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        Box::new(WeakWhenVar(Arc::downgrade(&self.0)))
    }

    fn is_animating(&self) -> bool {
        self.active().is_animating()
    }

    fn var_ptr(&self) -> VarPtr {
        VarPtr::new_arc(&self.0)
    }
}

impl<T: VarValue> AnyWeakVar for WeakWhenVar<T> {
    fn clone_any(&self) -> BoxedAnyWeakVar {
        Box::new(self.clone())
    }

    fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    fn weak_count(&self) -> usize {
        self.0.weak_count()
    }

    fn upgrade_any(&self) -> Option<BoxedAnyVar> {
        self.0.upgrade().map(|rc| Box::new(RcWhenVar(rc)) as _)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<T: VarValue> IntoVar<T> for RcWhenVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T: VarValue> Var<T> for RcWhenVar<T> {
    type ReadOnly = types::ReadOnlyVar<T, Self>;

    type ActualVar = Self;

    type Downgrade = WeakWhenVar<T>;

    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.active().with(read)
    }

    fn modify<V, F>(&self, vars: &V, modify: F) -> Result<(), VarIsReadOnlyError>
    where
        V: WithVars,
        F: FnOnce(&mut Cow<T>) + 'static,
    {
        self.active().modify(vars, modify)
    }

    fn actual_var(self) -> Self {
        // inputs already actualized on ctor
        self
    }

    fn downgrade(&self) -> WeakWhenVar<T> {
        WeakWhenVar(Arc::downgrade(&self.0))
    }

    fn into_value(self) -> T {
        match Arc::try_unwrap(self.0) {
            Ok(mut v) => {
                let active = v.w.into_inner().active;
                if active == usize::MAX {
                    v.default.into_value()
                } else {
                    v.conditions.swap_remove(active).1.into_value()
                }
            }
            Err(rc) => Self(rc).get(),
        }
    }

    fn read_only(&self) -> Self::ReadOnly {
        types::ReadOnlyVar::new(self.clone())
    }
}

impl<T: VarValue> WeakVar<T> for WeakWhenVar<T> {
    type Upgrade = RcWhenVar<T>;

    fn upgrade(&self) -> Option<RcWhenVar<T>> {
        self.0.upgrade().map(|rc| RcWhenVar(rc))
    }
}
