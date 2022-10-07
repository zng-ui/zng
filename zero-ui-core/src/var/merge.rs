use std::{marker::PhantomData, rc::Weak};

use super::{animation::AnimateModifyInfo, *};

///<span data-del-macro-root></span> Initializes a new [`Var`](crate::var::Var) with value made
/// by merging multiple other variables.
///
/// # Arguments
///
/// All arguments are separated by comma like a function call.
///
/// * `var0..N`: A list of [vars](crate::var::Var), minimal 2.
/// * `merge`: A new closure that produces a new value from references to all variable values. `FnMut(&var0_T, ..) -> merge_T`
///
/// # Contextualized
///
/// The merge var is contextualized, meaning is a [`ContextVar<T>`] is used for one of the inputs it will be resolved to the
/// context where the merge is first used, not where it is created. The full output type of this macro is `ContextualizedVar<T, RcMergeVar<T>>`.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::var::*;
/// # use zero_ui_core::text::*;
/// # fn text(text: impl IntoVar<Text>) {  }
/// let var0: RcVar<Text> = var_from("Hello");
/// let var1: RcVar<Text> = var_from("World");
///
/// let greeting_text = text(merge_var!(var0, var1, |a, b| formatx!("{a} {b}!")));
/// ```
#[macro_export]
macro_rules! merge_var {
    ($($tt:tt)+) => {
        $crate::var::types::__merge_var! {
            $crate::var,
            $($tt)+
        }
    };
}
#[doc(inline)]
pub use crate::merge_var;

#[doc(hidden)]
pub use zero_ui_proc_macros::merge_var as __merge_var;

#[doc(hidden)]
pub struct RcMergeVarInput<T: VarValue, V: Var<T>>(PhantomData<(V, T)>);
impl<T: VarValue, V: Var<T>> RcMergeVarInput<T, V> {
    pub fn new(_: &V) -> Self {
        RcMergeVarInput(PhantomData)
    }

    #[allow(clippy::borrowed_box)]
    pub fn get<'t, 'v>(&'t self, value: &'v Box<dyn AnyVarValue>) -> &'v T {
        (**value).as_any().downcast_ref::<T>().unwrap()
    }
}

struct Data<T> {
    inputs: Box<[Box<dyn AnyVarValue>]>,
    input_handles: Box<[VarHandle]>,
    merge: Box<dyn FnMut(&[Box<dyn AnyVarValue>]) -> T>,
    value: T,
    last_update: VarUpdateId,
    last_apply_request: VarApplyUpdateId,
    hooks: Vec<VarHook>,
    animation: AnimateModifyInfo,
}

/// See [`merge_var!`].
pub struct RcMergeVar<T>(Rc<RefCell<Data<T>>>);

#[doc(hidden)]
pub type ContextualizedRcMergeVar<T> = types::ContextualizedVar<T, RcMergeVar<T>>;

/// Weak reference to [`RcMergeVar<T>`].
pub struct WeakMergeVar<T>(Weak<RefCell<Data<T>>>);

impl<T: VarValue> RcMergeVar<T> {
    #[doc(hidden)]
    pub fn new(inputs: Box<[Box<dyn AnyVar>]>, merge: impl FnMut(&[Box<dyn AnyVarValue>]) -> T + 'static) -> ContextualizedRcMergeVar<T> {
        Self::new_impl(inputs, Rc::new(RefCell::new(merge)))
    }

    fn new_impl(
        inputs: Box<[Box<dyn AnyVar>]>,
        merge: Rc<RefCell<dyn FnMut(&[Box<dyn AnyVarValue>]) -> T + 'static>>,
    ) -> types::ContextualizedVar<T, RcMergeVar<T>> {
        types::ContextualizedVar::new(Rc::new(move || {
            let merge = merge.clone();
            RcMergeVar::new_contextualized(&inputs, Box::new(move |values| merge.borrow_mut()(values)))
        }))
    }

    fn new_contextualized(input_vars: &[Box<dyn AnyVar>], mut merge: Box<dyn FnMut(&[Box<dyn AnyVarValue>]) -> T>) -> Self {
        let inputs: Box<[_]> = input_vars.iter().map(|v| v.get_any()).collect();
        let rc_merge = Rc::new(RefCell::new(Data {
            value: merge(&inputs),
            inputs,
            input_handles: Box::new([]),
            merge,
            last_update: VarUpdateId::never(),
            last_apply_request: VarApplyUpdateId::initial(),
            hooks: vec![],
            animation: AnimateModifyInfo::never(),
        }));
        let wk_merge = Rc::downgrade(&rc_merge);

        let input_handles: Box<[_]> = input_vars
            .iter()
            .enumerate()
            .filter_map(|(i, var)| {
                if var.capabilities().contains(VarCapabilities::CHANGE) {
                    let wk_merge = wk_merge.clone();
                    let handle = var.hook(Box::new(move |vars, _, value| {
                        if let Some(rc_merge) = wk_merge.upgrade() {
                            let mut data = rc_merge.borrow_mut();
                            let data_mut = &mut *data;
                            if value.as_any().type_id() == data_mut.inputs[i].as_any().type_id() {
                                data_mut.inputs[i] = value.clone_boxed();

                                if data_mut.last_apply_request != vars.apply_update_id() {
                                    data_mut.last_apply_request = vars.apply_update_id();
                                    drop(data);
                                    vars.schedule_update(RcMergeVar::update_merge(rc_merge));
                                }
                            }
                            true
                        } else {
                            false
                        }
                    }));

                    debug_assert!(!handle.is_dummy());

                    Some(handle)
                } else {
                    None
                }
            })
            .collect();

        rc_merge.borrow_mut().input_handles = input_handles;

        Self(rc_merge)
    }

    fn update_merge(rc_merge: Rc<RefCell<Data<T>>>) -> VarUpdateFn {
        Box::new(move |vars, updates| {
            let mut data = rc_merge.borrow_mut();
            let data = &mut *data;
            data.value = (data.merge)(&data.inputs);
            data.last_update = vars.update_id();
            data.animation = vars.current_animation();

            data.hooks.retain(|h| h.call(vars, updates, &data.value));
            updates.update_ext();
        })
    }
}

impl<T: VarValue> Clone for RcMergeVar<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: VarValue> Clone for WeakMergeVar<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: VarValue> crate::private::Sealed for RcMergeVar<T> {}
impl<T: VarValue> crate::private::Sealed for WeakMergeVar<T> {}

impl<T: VarValue> AnyVar for RcMergeVar<T> {
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

    fn set_any(&self, _: &Vars, _: Box<dyn AnyVarValue>) -> Result<(), VarIsReadOnlyError> {
        Err(VarIsReadOnlyError {
            capabilities: self.capabilities(),
        })
    }

    fn last_update(&self) -> VarUpdateId {
        self.0.borrow().last_update
    }

    fn capabilities(&self) -> VarCapabilities {
        if self.0.borrow().inputs.is_empty() {
            VarCapabilities::empty()
        } else {
            VarCapabilities::CHANGE
        }
    }

    fn hook(&self, pos_modify_action: Box<dyn Fn(&Vars, &mut Updates, &dyn AnyVarValue) -> bool>) -> VarHandle {
        let (handle, weak) = VarHandle::new(pos_modify_action);
        self.0.borrow_mut().hooks.push(weak);
        handle
    }

    fn strong_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }

    fn weak_count(&self) -> usize {
        Rc::weak_count(&self.0)
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        Box::new(self.clone())
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        Box::new(WeakMergeVar(Rc::downgrade(&self.0)))
    }

    fn is_animating(&self) -> bool {
        self.0.borrow().animation.is_animating()
    }

    fn var_ptr(&self) -> VarPtr {
        VarPtr::new_rc(&self.0)
    }
}

impl<T: VarValue> AnyWeakVar for WeakMergeVar<T> {
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
        self.0.upgrade().map(|rc| Box::new(RcMergeVar(rc)) as _)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<T: VarValue> IntoVar<T> for RcMergeVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T: VarValue> Var<T> for RcMergeVar<T> {
    type ReadOnly = types::ReadOnlyVar<T, Self>;

    type ActualVar = Self;

    type Downgrade = WeakMergeVar<T>;

    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        read(&self.0.borrow().value)
    }

    fn modify<V, F>(&self, _: &V, _: F) -> Result<(), VarIsReadOnlyError>
    where
        V: WithVars,
        F: FnOnce(&mut VarModifyValue<T>) + 'static,
    {
        Err(VarIsReadOnlyError {
            capabilities: self.capabilities(),
        })
    }

    fn actual_var(&self) -> Self {
        self.clone()
    }

    fn downgrade(&self) -> WeakMergeVar<T> {
        WeakMergeVar(Rc::downgrade(&self.0))
    }

    fn into_value(self) -> T {
        match Rc::try_unwrap(self.0) {
            Ok(data) => data.into_inner().value,
            Err(rc) => Self(rc).get(),
        }
    }

    fn read_only(&self) -> Self::ReadOnly {
        types::ReadOnlyVar::new(self.clone())
    }
}

impl<T: VarValue> WeakVar<T> for WeakMergeVar<T> {
    type Upgrade = RcMergeVar<T>;

    fn upgrade(&self) -> Option<RcMergeVar<T>> {
        self.0.upgrade().map(|rc| RcMergeVar(rc))
    }
}
