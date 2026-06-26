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

use core::fmt;
use std::{
    any::TypeId,
    fmt::Write as _,
    marker::PhantomData,
    ops,
    sync::Arc,
};

use parking_lot::Mutex;
use zng_clone_move::clmv;
use zng_txt::Txt;
#[doc(hidden)]
pub use zng_var_proc_macros::merge_var as __merge_var;

use crate::{
    AnyVar, AnyVarValue, BoxAnyVarValue, ContextVar, Response, ResponseVar, Var, VarImpl, VarValue, WeakAnyVar, any_contextual_var, any_var,
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
    merge_var_tail(inputs, merge)
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

type MergeFn = Arc<Mutex<dyn FnMut(&[AnyVar]) -> BoxAnyVarValue + Send + 'static>>;
type MapBackFn = Arc<Mutex<dyn FnMut(&dyn AnyVarValue, usize) -> BoxAnyVarValue + Send + 'static>>;

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
    pub fn build<O: VarValue>(self, mut merge: impl FnMut(VarMergeInputs<I>) -> O + Send + 'static) -> Var<O> {
        self.builder.build(move |inputs| {
            // TODO(breaking) optimize this so that we don't need to alloc vec,
            // maybe VarMergeInputs can offer an with(&self, index: usize, visitor...)
            let values: Vec<_> = inputs.iter().map(AnyVar::get).collect();
            merge(VarMergeInputs {
                inputs: &values,
                _type: PhantomData,
            })
        })
    }

    /// Build a read-write merge var.
    pub fn build_bidi<O: VarValue>(
        self,
        mut merge: impl FnMut(VarMergeInputs<I>) -> O + Send + 'static,
        mut map_back: impl FnMut(&O, usize) -> I + Send + 'static,
    ) -> Var<O> {
        self.builder.build_bidi(
            move |inputs| {
                // TODO(breaking) see build
                let values: Vec<_> = inputs.iter().map(AnyVar::get).collect();
                merge(VarMergeInputs {
                    inputs: &values,
                    _type: PhantomData,
                })
            },
            move |output, input_idx| BoxAnyVarValue::new(map_back(output, input_idx)),
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
            for t in t.iter() {
                write!(&mut s, "{sep}{}", t.as_ref()).unwrap();
                sep = &separator;
            }
            s.into()
        })
    }
}

/// Input arguments for the merge closure of [`MergeVarBuilder`] merge vars.
pub struct VarMergeInputs<'a, I: VarValue> {
    // TODO(breaking) rename to MergeVarInputs
    inputs: &'a [BoxAnyVarValue],
    _type: PhantomData<&'a I>,
}
impl<I: VarValue> VarMergeInputs<'_, I> {
    /// Number of inputs.
    #[expect(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.inputs.len()
    }

    /// Iterate over the values.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &I> + '_ {
        (0..self.len()).map(move |i| &self[i])
    }
}
impl<I: VarValue> ops::Index<usize> for VarMergeInputs<'_, I> {
    type Output = I;

    fn index(&self, index: usize) -> &Self::Output {
        self.inputs[index].downcast_ref().unwrap()
    }
}
impl<I: VarValue> fmt::Debug for VarMergeInputs<'_, I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt::Debug::fmt(self.inputs, f)
    }
}
