use std::{
    marker::PhantomData,
    sync::{Arc, Weak},
};

use super::{util::VarData, *};

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

use parking_lot::Mutex;
#[doc(hidden)]
pub use zero_ui_proc_macros::merge_var as __merge_var;

// used by the __merge_var! proc-macro.
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

struct MergeData<T> {
    inputs: Box<[Box<dyn AnyVarValue>]>,
    input_handles: Box<[VarHandle]>,
    merge: Box<dyn FnMut(&[Box<dyn AnyVarValue>]) -> T + Send + Sync>,
    last_apply_request: VarApplyUpdateId,
}

struct Data<T: VarValue> {
    m: Mutex<MergeData<T>>,
    value: VarData<T>,
}

/// See [`merge_var!`].
pub struct RcMergeVar<T: VarValue>(Arc<Data<T>>);

#[doc(hidden)]
pub type ContextualizedRcMergeVar<T> = types::ContextualizedVar<T, RcMergeVar<T>>;

/// Weak reference to [`RcMergeVar<T>`].
pub struct WeakMergeVar<T: VarValue>(Weak<Data<T>>);

impl<T: VarValue> RcMergeVar<T> {
    #[doc(hidden)]
    pub fn new(
        inputs: Box<[Box<dyn AnyVar>]>,
        merge: impl FnMut(&[Box<dyn AnyVarValue>]) -> T + Send + 'static,
    ) -> ContextualizedRcMergeVar<T> {
        Self::new_impl(inputs, Arc::new(Mutex::new(merge)))
    }

    fn new_impl(
        inputs: Box<[Box<dyn AnyVar>]>,
        merge: Arc<Mutex<dyn FnMut(&[Box<dyn AnyVarValue>]) -> T + Send + 'static>>,
    ) -> types::ContextualizedVar<T, RcMergeVar<T>> {
        types::ContextualizedVar::new(Arc::new(move || {
            let merge = merge.clone();
            RcMergeVar::new_contextualized(&inputs, Box::new(move |values| merge.lock()(values)))
        }))
    }

    fn new_contextualized(input_vars: &[Box<dyn AnyVar>], mut merge: Box<dyn FnMut(&[Box<dyn AnyVarValue>]) -> T + Send + Sync>) -> Self {
        let inputs: Box<[_]> = input_vars.iter().map(|v| v.get_any()).collect();
        let rc_merge = Arc::new(Data {
            value: VarData::new(merge(&inputs)),
            m: Mutex::new(MergeData {
                inputs,
                input_handles: Box::new([]),
                merge,
                last_apply_request: VarApplyUpdateId::initial(),
            }),
        });
        let wk_merge = Arc::downgrade(&rc_merge);

        let input_handles: Box<[_]> = input_vars
            .iter()
            .enumerate()
            .filter_map(|(i, var)| {
                if var.capabilities().contains(VarCapabilities::NEW) {
                    let wk_merge = wk_merge.clone();
                    let handle = var.hook(Box::new(move |vars, _, value| {
                        if let Some(rc_merge) = wk_merge.upgrade() {
                            let mut data = rc_merge.m.lock();
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

        rc_merge.m.lock().input_handles = input_handles;

        Self(rc_merge)
    }

    fn update_merge(rc_merge: Arc<Data<T>>) -> VarUpdateFn {
        Box::new(move |vars, updates| {
            let mut m = rc_merge.m.lock();
            let m = &mut *m;
            let new_value = (m.merge)(&m.inputs);
            rc_merge.value.apply_modify(vars, updates, |v| *v = Cow::Owned(new_value));
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
        self.0.value.last_update()
    }

    fn capabilities(&self) -> VarCapabilities {
        if self.0.m.lock().inputs.is_empty() {
            VarCapabilities::empty()
        } else {
            VarCapabilities::NEW
        }
    }

    fn hook(&self, pos_modify_action: Box<dyn Fn(&Vars, &mut Updates, &dyn AnyVarValue) -> bool + Send + Sync>) -> VarHandle {
        self.0.value.push_hook(pos_modify_action)
    }

    fn strong_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }

    fn weak_count(&self) -> usize {
        Arc::weak_count(&self.0)
    }

    fn actual_var_any(&self) -> BoxedAnyVar {
        Box::new(self.clone())
    }

    fn downgrade_any(&self) -> BoxedAnyWeakVar {
        Box::new(WeakMergeVar(Arc::downgrade(&self.0)))
    }

    fn is_animating(&self) -> bool {
        self.0.value.is_animating()
    }

    fn modify_importance(&self) -> usize {
        self.0.value.modify_importance()
    }

    fn var_ptr(&self) -> VarPtr {
        VarPtr::new_arc(&self.0)
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
        self.0.value.with(read)
    }

    fn modify<V, F>(&self, _: &V, _: F) -> Result<(), VarIsReadOnlyError>
    where
        V: WithVars,
        F: FnOnce(&mut Cow<T>) + 'static,
    {
        Err(VarIsReadOnlyError {
            capabilities: self.capabilities(),
        })
    }

    fn actual_var(self) -> Self {
        self
    }

    fn downgrade(&self) -> WeakMergeVar<T> {
        WeakMergeVar(Arc::downgrade(&self.0))
    }

    fn into_value(self) -> T {
        match Arc::try_unwrap(self.0) {
            Ok(data) => data.value.into_value(),
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
