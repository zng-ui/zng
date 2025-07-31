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
    any::Any,
    marker::PhantomData,
    ops,
    sync::{Arc, Weak},
};

use parking_lot::Mutex;
use smallbox::{SmallBox, smallbox};
use zng_clone_move::clmv;
#[doc(hidden)]
pub use zng_var_proc_macros::merge_var as __merge_var;

use crate::{
    AnyVar, AnyVarValue, BoxAnyVarValue, ContextVar, Response, ResponseVar, Var, VarImpl, VarInstanceTag, VarValue, WeakVarImpl,
    any_contextual_var, any_var, contextual_var,
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
    Var::new_any(var_merge_impl(inputs, smallbox!(merge)))
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

fn var_merge_impl(inputs: Box<[AnyVar]>, merge: MergeFn) -> AnyVar {
    if inputs.iter().any(|i| i.capabilities().is_contextual()) {
        let merge = Arc::new(Mutex::new(merge));
        return any_contextual_var(move || {
            let mut inputs = inputs.clone();
            for v in inputs.iter_mut() {
                if v.capabilities().is_contextual() {
                    *v = v.current_context();
                }
            }
            var_merge_tail(inputs, smallbox!(clmv!(merge, |inputs: &[AnyVar]| { merge.lock()(inputs) })))
        });
    }
    var_merge_tail(inputs, merge)
}
fn var_merge_tail(inputs: Box<[AnyVar]>, mut merge: MergeFn) -> AnyVar {
    let output = any_var(merge(&inputs));
    let data = Arc::new(MergeVarData {
        inputs,
        merge: Mutex::new((merge, 0)),
        output,
    });

    for input in &data.inputs {
        let weak = Arc::downgrade(&data);
        input
            .hook(move |_| {
                if let Some(data) = weak.upgrade() {
                    // Multiple inputs can update on the same cycle,
                    // to avoid running merge multiple times schedule an output modify
                    // so it runs after the current burst on the same cycle, and use
                    // this counter to skip subsequent modify requests on the same cycle
                    let modify_id = data.merge.lock().1;
                    data.output.modify(clmv!(weak, |output| {
                        if let Some(data) = weak.upgrade() {
                            let mut m = data.merge.lock();
                            if m.1 != modify_id {
                                // already applied
                                return;
                            }

                            let new_value = m.0(&data.inputs);
                            output.set(new_value);
                        }
                    }));
                    true
                } else {
                    false
                }
            })
            .perm();
    }

    AnyVar(smallbox!(MergeVar(data)))
}

type MergeFn = SmallBox<dyn FnMut(&[AnyVar]) -> BoxAnyVarValue + Send + 'static, smallbox::space::S4>;

struct MergeVarData {
    inputs: Box<[AnyVar]>,
    merge: Mutex<(MergeFn, usize)>,
    output: AnyVar,
}

struct MergeVar(Arc<MergeVarData>);
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
    fn clone_boxed(&self) -> SmallBox<dyn VarImpl, smallbox::space::S2> {
        smallbox!(Self(self.0.clone()))
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

    fn var_eq(&self, other: &dyn Any) -> bool {
        match other.downcast_ref::<Self>() {
            Some(other) => Arc::ptr_eq(&self.0, &other.0),
            None => false,
        }
    }

    fn var_instance_tag(&self) -> VarInstanceTag {
        VarInstanceTag(Arc::as_ptr(&self.0) as _)
    }

    fn downgrade(&self) -> SmallBox<dyn super::WeakVarImpl, smallbox::space::S2> {
        smallbox!(WeakMergeVar(Arc::downgrade(&self.0)))
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

    fn hook_animation_stop(&self, handler: crate::animation::AnimationStopFn) -> Result<(), crate::animation::AnimationStopFn> {
        self.0.output.0.hook_animation_stop(handler)
    }

    fn current_context(&self) -> SmallBox<dyn VarImpl, smallbox::space::S2> {
        self.clone_boxed()
    }
}

struct WeakMergeVar(Weak<MergeVarData>);
impl fmt::Debug for WeakMergeVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("WeakMergeVar").field(&self.0.as_ptr()).finish()
    }
}
impl WeakVarImpl for WeakMergeVar {
    fn clone_boxed(&self) -> SmallBox<dyn WeakVarImpl, smallbox::space::S2> {
        smallbox!(WeakMergeVar(self.0.clone()))
    }

    fn strong_count(&self) -> usize {
        self.0.strong_count()
    }

    fn upgrade(&self) -> Option<SmallBox<dyn VarImpl, smallbox::space::S2>> {
        let s = self.0.upgrade()?;
        Some(smallbox!(MergeVar(s)))
    }
}

/// Build a [`merge_var!`] from any number of input vars of the same type `I`.
#[derive(Clone)]
pub struct MergeVarBuilder<I: VarValue> {
    inputs: Vec<AnyVar>,
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
    pub fn push(&mut self, input: impl MergeInput<I>) {
        self.inputs.push(input.into_merge_input().into())
    }

    /// Build the merge var.
    pub fn build<O: VarValue>(self, merge: impl FnMut(VarMergeInputs<I>) -> O + Send + 'static) -> Var<O> {
        if self.inputs.iter().any(|i| i.capabilities().is_contextual()) {
            let merge = Arc::new(Mutex::new(merge));
            return contextual_var(move || {
                let builder = MergeVarBuilder {
                    inputs: self.inputs.iter().map(|v| v.current_context()).collect(),
                    _type: PhantomData,
                };
                builder.build_tail(clmv!(merge, |inputs| merge.lock()(inputs)))
            });
        }
        self.build_tail(merge)
    }
    fn build_tail<O: VarValue>(self, mut merge: impl FnMut(VarMergeInputs<I>) -> O + Send + 'static) -> Var<O> {
        let any = var_merge_impl(
            self.inputs.into_boxed_slice(),
            smallbox!(move |vars: &[AnyVar]| {
                let values: Box<[BoxAnyVarValue]> = vars.iter().map(|v| v.get()).collect();
                let out = merge(VarMergeInputs {
                    inputs: &values[..],
                    _type: PhantomData,
                });
                BoxAnyVarValue::new(out)
            }),
        );
        Var::new_any(any)
    }
}
impl<I: VarValue> Default for MergeVarBuilder<I> {
    fn default() -> Self {
        Self::new()
    }
}

/// Input arguments for the merge closure of [`VarMergeBuilder`] merge vars.
pub struct VarMergeInputs<'a, I: VarValue> {
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
