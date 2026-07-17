//! Variable that merges multiple others.

///<span data-del-macro-root></span> Initializes a new [`Var<T>`](crate::Var) with value made
/// by merging multiple other variables.
///
/// # Arguments
///
/// All arguments are separated by comma like a function call.
///
/// * `var0..N`: A list of *vars*, minimal 2.
/// * `merge`: A new closure that produces a new value from references to all variable values. `FnMut(&var0_T, ..) -> merge_T`
///
/// Note that the *vars* can be of any of `Var<T>`, `ContextVar<T>` or `ResponseVar<T>`, that is, already constructed
/// var types, not all types that convert into var.
///
/// # Contextualized
///
/// The merge var is contextualized when needed, meaning if any input is [contextual] at the moment the var is created it
/// is also contextual. The full output type of this macro is a `Var<O>`, the `O` type is defined by the output of the merge closure.
///
/// [contextual]: crate::VarCapability::CONTEXT
///
/// # Examples
///
/// ```
/// # use zng_var::*;
/// # use zng_txt::*;
/// # macro_rules! Text { ($($tt:tt)*) => { () } }
/// let var0: Var<Txt> = var_from("Hello");
/// let var1: Var<Txt> = var_from("World");
///
/// let greeting_text = Text!(merge_var!(var0, var1, |a, b| formatx!("{a} {b}!")));
/// ```
#[macro_export]
macro_rules! merge_var {
    ($($tt:tt)+) => {
        $crate::__merge_var! {
            $crate,
            $($tt)+
        }
    };
}

use std::{any::TypeId, fmt::Write as _, marker::PhantomData, sync::Arc};

use parking_lot::Mutex;
use zng_clone_move::clmv;
use zng_txt::Txt;
#[doc(hidden)]
pub use zng_var_proc_macros::merge_var as __merge_var;

use crate::{
    AnyVar, AnyVarModify, AnyVarValue, BoxAnyVarValue, ContextVar, Response, ResponseVar, Var, VarImpl, VarInstanceTag, VarModify,
    VarValue, WeakAnyVar, any_contextual_var, any_var,
};

#[doc(hidden)]
pub fn merge_var_input<I: VarValue>(input: impl MergeInput<I>) -> AnyVar {
    input.into_merge_input().into()
}

#[doc(hidden)]
pub fn merge_var_with(var: &AnyVar, visitor: &mut dyn FnMut(&dyn AnyVarValue)) {
    var.0.with(visitor);
}

#[doc(hidden)]
pub fn merge_var_output<O: VarValue>(output: O) -> BoxAnyVarValue {
    BoxAnyVarValue::new(output)
}

#[doc(hidden)]
pub fn merge_var<O: VarValue>(inputs: Box<[AnyVar]>, merge: impl FnMut(&[AnyVar]) -> BoxAnyVarValue + Send + 'static) -> Var<O> {
    Var::new_any(merge_var_impl(inputs, Arc::new(Mutex::new(merge)), TypeId::of::<O>()))
}

#[doc(hidden)]
#[diagnostic::on_unimplemented(note = "merge_var! and expr_var! inputs can be: Var<T>, ContextVar<T> or ResponseVar<T>")]
pub trait MergeInput<T: VarValue> {
    fn into_merge_input(self) -> Var<T>;
}
impl<T: VarValue> MergeInput<T> for Var<T> {
    fn into_merge_input(self) -> Var<T> {
        self
    }
}
impl<T: VarValue> MergeInput<T> for ContextVar<T> {
    fn into_merge_input(self) -> Var<T> {
        self.into()
    }
}
impl<T: VarValue> MergeInput<Response<T>> for ResponseVar<T> {
    fn into_merge_input(self) -> Var<Response<T>> {
        self.into()
    }
}

fn merge_var_impl(inputs: Box<[AnyVar]>, merge: MergeFn, value_type: TypeId) -> AnyVar {
    if inputs.iter().any(|i| i.capabilities().is_contextual()) {
        return any_contextual_var(
            move || {
                let mut inputs = inputs.clone();
                for v in inputs.iter_mut() {
                    if v.capabilities().is_contextual() {
                        *v = v.current_context();
                    }
                }
                merge_var_tail(inputs, merge.clone())
            },
            value_type,
        );
    }
    merge_var_tail(inputs, merge)
}
fn merge_var_tail(inputs: Box<[AnyVar]>, merge: MergeFn) -> AnyVar {
    let output = any_var(merge.lock()(&inputs));
    struct InputData {
        inputs: Box<[AnyVar]>,
        merge: MergeFn,
        output_wk: WeakAnyVar,
    }
    let input_data = Arc::new(InputData {
        inputs,
        merge,
        output_wk: output.downgrade(),
    });
    for input in &input_data.inputs {
        let input_data_wk = Arc::downgrade(&input_data);
        input
            .hook(move |a| {
                // modify on each input update, if multiple inputs update on the same cycle
                // modify multiple times anyway, because services may be responding to the
                // *partial* merge state as it happens.
                if let Some(input_data) = input_data_wk.upgrade()
                    && let Some(output) = input_data.output_wk.upgrade()
                {
                    let update = a.update();
                    output.modify(clmv!(input_data_wk, |output| if let Some(input_data) = input_data_wk.upgrade() {
                        let new_value = input_data.merge.lock()(&input_data.inputs);
                        output.set(new_value);
                        if update {
                            output.update();
                        }
                    }));
                    true
                } else {
                    false
                }
            })
            .perm();
    }

    output.hold(input_data).perm();

    output.read_only()
}

fn merge_var_bidi_impl(inputs: Box<[AnyVar]>, merge: MergeFn, map_back: MapBackFn, value_type: TypeId) -> AnyVar {
    if inputs.iter().any(|i| i.capabilities().is_contextual()) {
        return any_contextual_var(
            move || {
                let mut inputs = inputs.clone();
                for v in inputs.iter_mut() {
                    if v.capabilities().is_contextual() {
                        *v = v.current_context();
                    }
                }
                merge_var_bidi_tail(inputs, merge.clone(), map_back.clone())
            },
            value_type,
        );
    }
    merge_var_bidi_tail(inputs, merge, map_back)
}
fn merge_var_bidi_tail(inputs: Box<[AnyVar]>, merge: MergeFn, map_back: MapBackFn) -> AnyVar {
    let output = any_var(merge.lock()(&inputs));
    struct InputData {
        inputs: Box<[AnyVar]>,
        merge: MergeFn,
        map_back: MapBackFn,
        output_wk: WeakAnyVar,
    }
    let input_data = Arc::new(InputData {
        inputs,
        merge,
        map_back,
        output_wk: output.downgrade(),
    });
    for input in &input_data.inputs {
        let input_data_wk = Arc::downgrade(&input_data);
        input
            .hook(move |a| {
                // modify on each input update, if multiple inputs update on the same cycle
                // modify multiple times anyway, because services may be responding to the
                // *partial* merge state as it happens.
                if let Some(input_data) = input_data_wk.upgrade()
                    && let Some(output) = input_data.output_wk.upgrade()
                {
                    let update = a.update();
                    output.modify(clmv!(input_data_wk, |output| if let Some(input_data) = input_data_wk.upgrade() {
                        let new_value = input_data.merge.lock()(&input_data.inputs);
                        output.set(new_value);
                        if update {
                            output.update();
                        }
                    }));
                    true
                } else {
                    false
                }
            })
            .perm();
    }

    output
        .hook(move |a| {
            let mut map_back = input_data.map_back.lock();
            for (i, input) in input_data.inputs.iter().enumerate() {
                if input.capabilities().can_modify() {
                    input.set(map_back(a.value(), i));
                }
            }
            true
        })
        .perm();

    output
}

fn merge_var_bidi_modify_impl(inputs: Box<[AnyVar]>, merge: MergeFn, modify_back: ModifyBackFn, value_type: TypeId) -> AnyVar {
    if inputs.iter().any(|i| i.capabilities().is_contextual()) {
        return any_contextual_var(
            move || {
                let mut inputs = inputs.clone();
                for v in inputs.iter_mut() {
                    if v.capabilities().is_contextual() {
                        *v = v.current_context();
                    }
                }
                merge_var_bidi_modify_tail(inputs, merge.clone(), modify_back.clone())
            },
            value_type,
        );
    }
    merge_var_bidi_modify_tail(inputs, merge, modify_back)
}
fn merge_var_bidi_modify_tail(inputs: Box<[AnyVar]>, merge: MergeFn, modify_back: ModifyBackFn) -> AnyVar {
    let output = any_var(merge.lock()(&inputs));
    struct InputData {
        inputs: Box<[AnyVar]>,
        merge: MergeFn,
        modify_back: ModifyBackFn,
        output_wk: WeakAnyVar,
    }
    let input_data = Arc::new(InputData {
        inputs,
        merge,
        modify_back,
        output_wk: output.downgrade(),
    });
    #[derive(Debug, PartialEq, Clone, Copy)]
    struct InputToOutputTag(VarInstanceTag);
    #[derive(Debug, PartialEq, Clone, Copy)]
    struct OutputToInputsTag(VarInstanceTag);
    let input_to_output_tag = InputToOutputTag(output.var_instance_tag());
    let output_to_inputs_tag = OutputToInputsTag(output.var_instance_tag());
    for input in &input_data.inputs {
        let input_data_wk = Arc::downgrade(&input_data);
        input
            .hook(move |a| {
                // modify on each input update, if multiple inputs update on the same cycle
                // modify multiple times anyway, because services may be responding to the
                // *partial* merge state as it happens.
                if let Some(input_data) = input_data_wk.upgrade()
                    && let Some(output) = input_data.output_wk.upgrade()
                {
                    if a.contains_tag(&output_to_inputs_tag) {
                        return true;
                    }

                    let update = a.update();
                    output.modify(clmv!(input_data_wk, |output| if let Some(input_data) = input_data_wk.upgrade() {
                        let new_value = input_data.merge.lock()(&input_data.inputs);
                        let changed = output.set(new_value);
                        if update {
                            output.update();
                        }
                        if changed || update {
                            output.push_tag(input_to_output_tag);
                        }
                    }));
                    true
                } else {
                    false
                }
            })
            .perm();
    }

    output
        .hook(move |a| {
            if a.contains_tag(&input_to_output_tag) {
                return true;
            }
            for (i, input) in input_data.inputs.iter().enumerate() {
                if input.capabilities().can_modify() {
                    let output_wk = input_data.output_wk.clone();
                    let modify_back = input_data.modify_back.clone();
                    let update = a.update();
                    input.modify(move |m| {
                        if let Some(output) = output_wk.upgrade() {
                            let has_updated = m.check_update(|m| {
                                output.with(|o| {
                                    modify_back.lock()(o, i, m);
                                });
                                if update {
                                    m.update();
                                }
                            });
                            if has_updated {
                                m.push_tag(output_to_inputs_tag);
                            }
                        }
                    });
                }
            }
            true
        })
        .perm();

    output
}

type MergeFn = Arc<Mutex<dyn FnMut(&[AnyVar]) -> BoxAnyVarValue + Send + 'static>>;
type MapBackFn = Arc<Mutex<dyn FnMut(&dyn AnyVarValue, usize) -> BoxAnyVarValue + Send + 'static>>;
type ModifyBackFn = Arc<Mutex<dyn FnMut(&dyn AnyVarValue, usize, &mut AnyVarModify) + Send + 'static>>;

/// Build a [`merge_var!`] from any number of input vars of any type.
#[derive(Clone)]
pub struct AnyMergeVarBuilder {
    inputs: Vec<AnyVar>,
}
impl Default for AnyMergeVarBuilder {
    fn default() -> Self {
        Self::new()
    }
}
impl AnyMergeVarBuilder {
    /// new empty.
    pub fn new() -> Self {
        Self { inputs: vec![] }
    }

    /// New with pre-allocated inputs.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inputs: Vec::with_capacity(capacity),
        }
    }

    /// Push an input.
    pub fn push(&mut self, input: AnyVar) {
        self.inputs.push(input);
    }

    fn read_only_inputs(self) -> Box<[AnyVar]> {
        let mut inputs = self.inputs;
        for input in &mut inputs {
            if !input.capabilities().is_always_read_only() {
                let v = input.read_only();
                *input = v;
            }
        }
        inputs.into_boxed_slice()
    }

    /// Build a read-only merge var.
    pub fn build_any(self, merge: impl FnMut(&[AnyVar]) -> BoxAnyVarValue + Send + 'static, output_type: TypeId) -> AnyVar {
        merge_var_impl(self.read_only_inputs(), Arc::new(Mutex::new(merge)), output_type)
    }

    /// Build a read-only strongly typed merge var.
    pub fn build<O: VarValue>(self, mut merge: impl FnMut(&[AnyVar]) -> O + Send + 'static) -> Var<O> {
        Var::new_any(self.build_any(move |inputs| BoxAnyVarValue::new(merge(inputs)), TypeId::of::<O>()))
    }

    /// Build a read-write merge var.
    pub fn build_bidi_any(
        self,
        merge: impl FnMut(&[AnyVar]) -> BoxAnyVarValue + Send + 'static,
        map_back: impl FnMut(&dyn AnyVarValue, usize) -> BoxAnyVarValue + Send + 'static,
        output_type: TypeId,
    ) -> AnyVar {
        merge_var_bidi_impl(
            self.inputs.into_boxed_slice(),
            Arc::new(Mutex::new(merge)),
            Arc::new(Mutex::new(map_back)),
            output_type,
        )
    }

    /// Build a read-write merge var that modifies each input back.
    pub fn build_bidi_any_modify(
        self,
        merge: impl FnMut(&[AnyVar]) -> BoxAnyVarValue + Send + 'static,
        modify_back: impl FnMut(&dyn AnyVarValue, usize, &mut AnyVarModify) + Send + 'static,
        output_type: TypeId,
    ) -> AnyVar {
        merge_var_bidi_modify_impl(
            self.inputs.into_boxed_slice(),
            Arc::new(Mutex::new(merge)),
            Arc::new(Mutex::new(modify_back)),
            output_type,
        )
    }

    /// Build a read-write strongly typed merge var.
    pub fn build_bidi<O: VarValue>(
        self,
        mut merge: impl FnMut(&[AnyVar]) -> O + Send + 'static,
        mut map_back: impl FnMut(&O, usize) -> BoxAnyVarValue + Send + 'static,
    ) -> Var<O> {
        Var::new_any(self.build_bidi_any(
            move |inputs| BoxAnyVarValue::new(merge(inputs)),
            move |output, input_idx| map_back(output.downcast_ref::<O>().unwrap(), input_idx),
            TypeId::of::<O>(),
        ))
    }

    /// Build a read-write strongly typed merge var that modifies each input back.
    pub fn build_bidi_modify<O: VarValue>(
        self,
        mut merge: impl FnMut(&[AnyVar]) -> O + Send + 'static,
        mut modify_back: impl FnMut(&O, usize, &mut AnyVarModify) + Send + 'static,
    ) -> Var<O> {
        Var::new_any(self.build_bidi_any_modify(
            move |inputs| BoxAnyVarValue::new(merge(inputs)),
            move |output, input_idx, m| modify_back(output.downcast_ref::<O>().unwrap(), input_idx, m),
            TypeId::of::<O>(),
        ))
    }

    /// Convert into a [`MergeVarBuilder<I>`].
    ///
    /// # Panics
    ///
    /// Panics if any input is not of type `I`.
    pub fn into_typed<I: VarValue>(self) -> MergeVarBuilder<I> {
        let i_id = TypeId::of::<I>();
        for input in &self.inputs {
            assert_eq!(i_id, input.value_type());
        }
        MergeVarBuilder {
            builder: self,
            _type: PhantomData,
        }
    }
}

/// Build a [`merge_var!`] from any number of input vars of the same type `I`.
#[derive(Clone)]
pub struct MergeVarBuilder<I: VarValue> {
    builder: AnyMergeVarBuilder,
    _type: PhantomData<fn() -> I>,
}
impl<I: VarValue> MergeVarBuilder<I> {
    /// New empty.
    pub fn new() -> Self {
        Self {
            builder: AnyMergeVarBuilder::new(),
            _type: PhantomData,
        }
    }

    /// New with pre-allocated inputs.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            builder: AnyMergeVarBuilder::with_capacity(capacity),
            _type: PhantomData,
        }
    }

    /// Push an input.
    pub fn push(&mut self, input: impl MergeInput<I>) {
        self.builder.push(input.into_merge_input().into())
    }

    /// Build a red-only merge var.
    pub fn build<O: VarValue>(self, mut merge: impl FnMut(MergeVarInputs<I>) -> O + Send + 'static) -> Var<O> {
        self.builder.build(move |inputs| {
            merge(MergeVarInputs {
                inputs,
                _input_type: PhantomData,
            })
        })
    }

    /// Build a read-write merge var.
    pub fn build_bidi<O: VarValue>(
        self,
        mut merge: impl FnMut(MergeVarInputs<I>) -> O + Send + 'static,
        mut map_back: impl FnMut(&O, usize) -> I + Send + 'static,
    ) -> Var<O> {
        self.builder.build_bidi(
            move |inputs| {
                merge(MergeVarInputs {
                    inputs,
                    _input_type: PhantomData,
                })
            },
            move |output, input_idx| BoxAnyVarValue::new(map_back(output, input_idx)),
        )
    }

    /// Build a read-write merge var that modifies each input back.
    pub fn build_bidi_modify<O: VarValue>(
        self,
        mut merge: impl FnMut(MergeVarInputs<I>) -> O + Send + 'static,
        mut modify_back: impl FnMut(&O, usize, VarModify<I>) + Send + 'static,
    ) -> Var<O> {
        self.builder.build_bidi_modify(
            move |inputs| {
                merge(MergeVarInputs {
                    inputs,
                    _input_type: PhantomData,
                })
            },
            move |output, input_idx, m| modify_back(output, input_idx, m.downcast().unwrap()),
        )
    }
}
impl<I: VarValue> Default for MergeVarBuilder<I> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T: VarValue + AsRef<str>> MergeVarBuilder<T> {
    /// Build to a var that joins texts placing a `separator` between each.
    pub fn join_txt(self, separator: impl Into<Txt>) -> Var<Txt> {
        self.join_txt_impl(separator.into())
    }
    fn join_txt_impl(self, separator: Txt) -> Var<Txt> {
        self.build(move |t| {
            let mut s = String::new();
            let mut sep = "";
            t.with_each(|_, t| {
                write!(&mut s, "{sep}{}", t.as_ref()).unwrap();
                sep = &separator;
            });
            s.into()
        })
    }
}

/// Strongly typed input vars for [`MergeVarBuilder`]
pub struct MergeVarInputs<'a, I: VarValue> {
    inputs: &'a [AnyVar],
    _input_type: PhantomData<fn() -> &'a I>,
}
impl<'a, I: VarValue> MergeVarInputs<'a, I> {
    /// Number of inputs.
    pub fn len(&self) -> usize {
        self.inputs.len()
    }

    /// If has no inputs.
    pub fn is_empty(&self) -> bool {
        self.inputs.is_empty()
    }

    /// Visit the current value of the `index` input.
    pub fn with<R>(&self, index: usize, visit: impl FnOnce(&I) -> R) -> R {
        self.inputs[index].with(|v| visit(v.downcast_ref().unwrap()))
    }

    /// Clone the current value of the `index` input.
    pub fn get(&self, index: usize) -> I {
        self.with(index, |v| v.clone())
    }

    /// Clone the `index` input var.
    pub fn input(&self, index: usize) -> Var<I> {
        self.inputs[index].read_only().downcast().unwrap()
    }

    /// Iterate over clones of the current value of each input.
    pub fn iter(&self) -> std::iter::Map<std::slice::Iter<'a, AnyVar>, fn(&AnyVar) -> I> {
        Self {
            inputs: self.inputs,
            _input_type: self._input_type,
        }
        .into_iter()
    }

    /// Visit the current value of each input.
    pub fn with_each(&self, mut visit: impl FnMut(usize, &I)) {
        for (i, var) in self.inputs.iter().enumerate() {
            var.with(|v| visit(i, v.downcast_ref().unwrap()))
        }
    }
}
impl<'a, I: VarValue> std::iter::IntoIterator for MergeVarInputs<'a, I> {
    type Item = I;

    type IntoIter = std::iter::Map<std::slice::Iter<'a, AnyVar>, fn(&AnyVar) -> I>;

    fn into_iter(self) -> Self::IntoIter {
        self.inputs.iter().map(downcast)
    }
}
fn downcast<I: VarValue>(v: &AnyVar) -> I {
    v.with(|v| v.downcast_ref::<I>().unwrap().clone())
}
