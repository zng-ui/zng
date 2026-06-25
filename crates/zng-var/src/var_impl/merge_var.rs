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
    sync::{Arc, Weak},
};

use parking_lot::Mutex;
use smallbox::SmallBox;
use zng_clone_move::clmv;
use zng_txt::Txt;
#[doc(hidden)]
pub use zng_var_proc_macros::merge_var as __merge_var;

use crate::{
    AnyVar, AnyVarValue, BoxAnyVarValue, ContextVar, DynAnyVar, DynWeakAnyVar, Response, ResponseVar, Var, VarHandle, VarImpl,
    VarInstanceTag, VarValue, WeakVarImpl, any_contextual_var, any_var,
};

use super::VarCapability;

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
    let data = Arc::new(MergeVarData { inputs, merge, output });

    for input in &data.inputs {
        let weak = Arc::downgrade(&data);
        input
            .hook(move |a| {
                if let Some(data) = weak.upgrade() {
                    // modify on each input update, if multiple inputs update on the same cycle
                    // modify multiple times anyway, because services may be responding to the
                    // *partial* merge state as it happens.
                    let update = a.update();
                    data.output.modify(clmv!(weak, |output| {
                        if let Some(data) = weak.upgrade() {
                            let mut m = data.merge.lock();
                            let new_value = m(&data.inputs);
                            output.set(new_value);
                            if update {
                                output.update();
                            }
                        }
                    }));
                    true
                } else {
                    false
                }
            })
            .perm();
    }

    AnyVar(DynAnyVar::Merge(MergeVar(data)))
}

type MergeFn = Arc<Mutex<dyn FnMut(&[AnyVar]) -> BoxAnyVarValue + Send + 'static>>;

struct MergeVarData {
    inputs: Box<[AnyVar]>,
    merge: MergeFn,
    output: AnyVar,
}

pub(crate) struct MergeVar(Arc<MergeVarData>);
impl fmt::Debug for MergeVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut b = f.debug_struct("MergeVar");
        b.field("var_instance_tag()", &self.var_instance_tag());
        b.field("inputs", &self.0.inputs);
        b.field("output", &self.0.output);
        b.finish()
    }
}
impl VarImpl for MergeVar {
    fn clone_dyn(&self) -> DynAnyVar {
        DynAnyVar::Merge(MergeVar(self.0.clone()))
    }

    fn value_type(&self) -> std::any::TypeId {
        self.0.output.0.value_type()
    }

    #[cfg(feature = "type_names")]
    fn value_type_name(&self) -> &'static str {
        self.0.output.0.value_type_name()
    }

    fn strong_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }

    fn var_eq(&self, other: &DynAnyVar) -> bool {
        match other {
            DynAnyVar::Merge(o) => Arc::ptr_eq(&self.0, &o.0),
            _ => false,
        }
    }

    fn var_instance_tag(&self) -> VarInstanceTag {
        VarInstanceTag(Arc::as_ptr(&self.0) as _)
    }

    fn downgrade(&self) -> DynWeakAnyVar {
        DynWeakAnyVar::Merge(WeakMergeVar(Arc::downgrade(&self.0)))
    }

    fn capabilities(&self) -> VarCapability {
        self.0.output.0.capabilities().as_always_read_only()
    }

    fn with(&self, visitor: &mut dyn FnMut(&dyn AnyVarValue)) {
        self.0.output.0.with(visitor);
    }

    fn get(&self) -> BoxAnyVarValue {
        self.0.output.0.get()
    }

    fn set(&self, _: BoxAnyVarValue) -> bool {
        false
    }

    fn update(&self) -> bool {
        false
    }

    fn modify(&self, _: SmallBox<dyn FnMut(&mut super::AnyVarModify) + Send + 'static, smallbox::space::S4>) -> bool {
        false
    }

    fn hook(&self, on_new: SmallBox<dyn FnMut(&crate::AnyVarHookArgs) -> bool + Send + 'static, smallbox::space::S4>) -> super::VarHandle {
        self.0.output.0.hook(on_new)
    }

    fn last_update(&self) -> crate::VarUpdateId {
        self.0.output.0.last_update()
    }

    fn modify_info(&self) -> crate::animation::ModifyInfo {
        self.0.output.0.modify_info()
    }

    fn modify_importance(&self) -> usize {
        self.0.output.0.modify_importance()
    }

    fn is_animating(&self) -> bool {
        self.0.output.0.is_animating()
    }

    fn hook_animation_stop(&self, handler: crate::animation::AnimationStopFn) -> VarHandle {
        self.0.output.0.hook_animation_stop(handler)
    }

    fn current_context(&self) -> DynAnyVar {
        self.clone_dyn()
    }
}

pub(crate) struct WeakMergeVar(Weak<MergeVarData>);
impl fmt::Debug for WeakMergeVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("WeakMergeVar").field(&self.0.as_ptr()).finish()
    }
}
impl WeakVarImpl for WeakMergeVar {
    fn clone_dyn(&self) -> DynWeakAnyVar {
        DynWeakAnyVar::Merge(WeakMergeVar(self.0.clone()))
    }

    fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    fn upgrade(&self) -> Option<DynAnyVar> {
        Some(DynAnyVar::Merge(MergeVar(self.0.upgrade()?)))
    }

    fn var_eq(&self, other: &DynWeakAnyVar) -> bool {
        match other {
            DynWeakAnyVar::Merge(o) => self.0.ptr_eq(&o.0),
            _ => false,
        }
    }
}

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
        merge_var(self.read_only_inputs(), move |inputs| BoxAnyVarValue::new(merge(inputs)))
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
