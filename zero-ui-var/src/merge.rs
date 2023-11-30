use std::{
    marker::PhantomData,
    ops,
    sync::{Arc, Weak},
};

use super::{util::VarData, *};

///<span data-del-macro-root></span> Initializes a new [`Var`](crate::Var) with value made
/// by merging multiple other variables.
///
/// # Arguments
///
/// All arguments are separated by comma like a function call.
///
/// * `var0..N`: A list of [vars](crate::Var), minimal 2.
/// * `merge`: A new closure that produces a new value from references to all variable values. `FnMut(&var0_T, ..) -> merge_T`
///
/// # Contextualized
///
/// The merge var is contextualized, meaning is a [`ContextVar<T>`] is used for one of the inputs it will be resolved to the
/// context where the merge is first used, not where it is created. The full output type of this macro is `ContextualizedVar<T, ArcMergeVar<T>>`.
///
/// # Examples
///
/// ```
/// # use zero_ui_var::*;
/// # use zero_ui_txt::*;
/// # macro_rules! Text { ($($tt:tt)*) => { () } }
/// let var0: ArcVar<Txt> = var_from("Hello");
/// let var1: ArcVar<Txt> = var_from("World");
///
/// let greeting_text = Text!(merge_var!(var0, var1, |a, b| formatx!("{a} {b}!")));
/// ```
#[macro_export]
macro_rules! merge_var {
    ($($tt:tt)+) => {
        $crate::types::__merge_var! {
            $crate,
            $($tt)+
        }
    };
}

use parking_lot::Mutex;
#[doc(hidden)]
pub use zero_ui_var_proc_macros::merge_var as __merge_var;

// used by the __merge_var! proc-macro.
#[doc(hidden)]
pub struct ArcMergeVarInput<T: VarValue, V: Var<T>>(PhantomData<(V, T)>);
impl<T: VarValue, V: Var<T>> ArcMergeVarInput<T, V> {
    pub fn new(_: &V) -> Self {
        ArcMergeVarInput(PhantomData)
    }

    #[allow(clippy::borrowed_box)]
    pub fn get<'v>(&self, value: &'v Box<dyn AnyVarValue>) -> &'v T {
        (**value).as_any().downcast_ref::<T>().unwrap()
    }
}

struct MergeData<T> {
    inputs: Box<[Box<dyn AnyVarValue>]>,
    input_handles: Box<[VarHandle]>,
    merge: Box<dyn FnMut(&[Box<dyn AnyVarValue>]) -> T + Send + Sync>,
}

struct Data<T: VarValue> {
    m: Mutex<MergeData<T>>,
    value: VarData<T>,
}

/// See [`merge_var!`].
pub struct ArcMergeVar<T: VarValue>(Arc<Data<T>>);

#[doc(hidden)]
pub type ContextualizedArcMergeVar<T> = types::ContextualizedVar<T, ArcMergeVar<T>>;

/// Weak reference to [`ArcMergeVar<T>`].
pub struct WeakMergeVar<T: VarValue>(Weak<Data<T>>);

impl<T: VarValue> ArcMergeVar<T> {
    #[doc(hidden)]
    pub fn new(
        inputs: Box<[Box<dyn AnyVar>]>,
        merge: impl FnMut(&[Box<dyn AnyVarValue>]) -> T + Send + 'static,
    ) -> ContextualizedArcMergeVar<T> {
        Self::new_impl(inputs, Arc::new(Mutex::new(merge)))
    }

    fn new_impl(
        inputs: Box<[Box<dyn AnyVar>]>,
        merge: Arc<Mutex<dyn FnMut(&[Box<dyn AnyVarValue>]) -> T + Send + 'static>>,
    ) -> types::ContextualizedVar<T, ArcMergeVar<T>> {
        types::ContextualizedVar::new(Arc::new(move || {
            let merge = merge.clone();
            ArcMergeVar::new_contextualized(&inputs, Box::new(move |values| merge.lock()(values)))
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
            }),
        });
        let wk_merge = Arc::downgrade(&rc_merge);

        let input_handles: Box<[_]> = input_vars
            .iter()
            .enumerate()
            .filter_map(|(i, var)| {
                if var.capabilities().contains(VarCapabilities::NEW) {
                    let wk_merge = wk_merge.clone();
                    let handle = var.hook(Box::new(move |args| {
                        if let Some(rc_merge) = wk_merge.upgrade() {
                            let mut data = rc_merge.m.lock();
                            let data_mut = &mut *data;
                            if args.value_type() == data_mut.inputs[i].as_any().type_id() {
                                data_mut.inputs[i] = args.value().clone_boxed();

                                drop(data);
                                VARS.schedule_update(ArcMergeVar::update_merge(rc_merge), std::any::type_name::<T>());
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
        Box::new(move || {
            let mut m = rc_merge.m.lock();
            let m = &mut *m;
            let new_value = (m.merge)(&m.inputs);
            rc_merge.value.apply_modify(|v| v.set(new_value));
        })
    }
}

impl<T: VarValue> Clone for ArcMergeVar<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: VarValue> Clone for WeakMergeVar<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: VarValue> crate::private::Sealed for ArcMergeVar<T> {}
impl<T: VarValue> crate::private::Sealed for WeakMergeVar<T> {}

impl<T: VarValue> AnyVar for ArcMergeVar<T> {
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

    fn set_any(&self, _: Box<dyn AnyVarValue>) -> Result<(), VarIsReadOnlyError> {
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

    fn hook(&self, pos_modify_action: Box<dyn Fn(&VarHookArgs) -> bool + Send + Sync>) -> VarHandle {
        self.0.value.push_hook(pos_modify_action)
    }

    fn hook_animation_stop(&self, handler: Box<dyn FnOnce() + Send>) -> Result<(), Box<dyn FnOnce() + Send>> {
        self.0.value.push_animation_hook(handler)
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

    fn get_debug(&self) -> Txt {
        self.with(var_debug)
    }

    fn update(&self) -> Result<(), VarIsReadOnlyError> {
        Var::modify(self, var_update)
    }

    fn map_debug(&self) -> BoxedVar<Txt> {
        Var::map(self, var_debug).boxed()
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
        self.0.upgrade().map(|rc| Box::new(ArcMergeVar(rc)) as _)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<T: VarValue> IntoVar<T> for ArcMergeVar<T> {
    type Var = Self;

    fn into_var(self) -> Self::Var {
        self
    }
}

impl<T: VarValue> Var<T> for ArcMergeVar<T> {
    type ReadOnly = types::ReadOnlyVar<T, Self>;

    type ActualVar = Self;

    type Downgrade = WeakMergeVar<T>;

    type Map<O: VarValue> = contextualized::ContextualizedVar<O, ReadOnlyArcVar<O>>;
    type MapBidi<O: VarValue> = contextualized::ContextualizedVar<O, ArcVar<O>>;


    fn with<R, F>(&self, read: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.0.value.with(read)
    }

    fn modify<F>(&self, _: F) -> Result<(), VarIsReadOnlyError>
    where
        F: FnOnce(&mut VarModify<T>) + 'static,
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

    fn map<O, M>(&self, map: M) -> Self::Map<O>
    where
        O: VarValue,
        M: FnMut(&T) -> O + Send + 'static,
    {
        var_map(self, map)
    }

    fn map_bidi<O, M, B>(&self, map: M, map_back: B) -> Self::MapBidi<O>
        where
            O: VarValue,
            M: FnMut(&T) -> O + Send + 'static,
            B: FnMut(&O) -> T + Send + 'static {
        var_map_bidi(self, map, map_back)
    }
}

impl<T: VarValue> WeakVar<T> for WeakMergeVar<T> {
    type Upgrade = ArcMergeVar<T>;

    fn upgrade(&self) -> Option<ArcMergeVar<T>> {
        self.0.upgrade().map(|rc| ArcMergeVar(rc))
    }
}

/// Build a merge-var from any number of input vars of the same type `I`.
pub struct MergeVarBuilder<I: VarValue> {
    inputs: Vec<Box<dyn AnyVar>>,
    _type: PhantomData<fn() -> I>,
}
impl<I: VarValue> MergeVarBuilder<I> {
    /// New empty.
    pub fn new() -> Self {
        Self {
            inputs: vec![],
            _type: PhantomData,
        }
    }

    /// New with pre-allocated inputs.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inputs: Vec::with_capacity(capacity),
            _type: PhantomData,
        }
    }

    /// Push an input.
    pub fn push(&mut self, input: impl Var<I>) {
        self.inputs.push(input.boxed_any())
    }

    /// Build the merge var.
    pub fn build<O: VarValue>(
        self,
        mut merge: impl FnMut(MergeVarInputs<I>) -> O + Send + 'static,
    ) -> types::ContextualizedVar<O, ArcMergeVar<O>> {
        ArcMergeVar::new(self.inputs.into_boxed_slice(), move |inputs| {
            merge(MergeVarInputs {
                inputs,
                _type: PhantomData,
            })
        })
    }
}
impl<I: VarValue> Default for MergeVarBuilder<I> {
    fn default() -> Self {
        Self::new()
    }
}

/// Input arguments for the merge closure of [`MergeVarBuilder`] merge vars.
pub struct MergeVarInputs<'a, I: VarValue> {
    inputs: &'a [Box<dyn AnyVarValue>],
    _type: PhantomData<&'a I>,
}
impl<'a, I: VarValue> MergeVarInputs<'a, I> {
    /// Number of inputs.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.inputs.len()
    }

    /// Iterate over the values.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &I> + '_ {
        (0..self.len()).map(move |i| &self[i])
    }
}
impl<'a, I: VarValue> ops::Index<usize> for MergeVarInputs<'a, I> {
    type Output = I;

    fn index(&self, index: usize) -> &Self::Output {
        self.inputs[index].as_any().downcast_ref().unwrap()
    }
}
