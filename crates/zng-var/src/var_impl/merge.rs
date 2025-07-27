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
/// let greeting_text = Text!(var_merge!(var0, var1, |a, b| formatx!("{a} {b}!")));
/// ```
#[macro_export]
macro_rules! var_merge {
    ($($tt:tt)+) => {
        $crate::__var_merge! {
            $crate,
            $($tt)+
        }
    };
}

use std::{
    any::Any,
    sync::{Arc, Weak},
};

use parking_lot::Mutex;
use smallbox::{SmallBox, smallbox};
use zng_clone_move::clmv;
#[doc(hidden)]
pub use zng_var_proc_macros::var_merge as __var_merge;

use crate::{
    BoxedVarValueAny, ContextVar, Response, ResponseVar, Var, VarAny, VarImpl, VarInstanceTag, VarValue, VarValueAny, WeakVarImpl,
    box_value_any, var_any,
};

use super::VarCapability;

#[doc(hidden)]
pub fn var_merge_input<I: VarValue>(input: impl MergeInput<I>) -> VarAny {
    input.into_merge_input().into()
}

#[doc(hidden)]
pub fn var_merge_with(var: &VarAny, visitor: &mut dyn FnMut(&dyn VarValueAny)) {
    var.0.with(visitor);
}

#[doc(hidden)]
pub fn var_merge_output<O: VarValue>(output: O) -> BoxedVarValueAny {
    box_value_any(output)
}

#[doc(hidden)]
pub fn var_merge<O: VarValue>(inputs: Box<[VarAny]>, merge: impl FnMut(&[VarAny]) -> BoxedVarValueAny + Send + 'static) -> Var<O> {
    Var::new_any(var_merge_impl(inputs, smallbox!(merge)))
}

#[doc(hidden)]
#[diagnostic::on_unimplemented(note = "var_merge! and var_expr! inputs can be: Var<T>, ContextVar<T> or ResponseVar<T>")]
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

fn var_merge_impl(inputs: Box<[VarAny]>, merge: MergeFn) -> VarAny {
    if inputs.iter().any(|i| i.capabilities().is_contextual()) {
        todo!("!!: TODO")
    }
    var_merge_tail(inputs, merge)
}
fn var_merge_tail(inputs: Box<[VarAny]>, mut merge: MergeFn) -> VarAny {
    let output = var_any(merge(&inputs));
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
                    let _ = data.output.modify(clmv!(weak, |output| {
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

    VarAny(smallbox!(MergeVar(data)))
}

type MergeFn = SmallBox<dyn FnMut(&[VarAny]) -> BoxedVarValueAny + Send + 'static, smallbox::space::S4>;

struct MergeVarData {
    inputs: Box<[VarAny]>,
    merge: Mutex<(MergeFn, usize)>,
    output: VarAny,
}

struct MergeVar(Arc<MergeVarData>);
impl VarImpl for MergeVar {
    fn clone_boxed(&self) -> SmallBox<dyn VarImpl, smallbox::space::S2> {
        smallbox!(Self(self.0.clone()))
    }

    fn value_type(&self) -> std::any::TypeId {
        self.0.output.0.value_type()
    }

    #[cfg(feature = "value_type_name")]
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
        self.0.output.0.capabilities().as_read_only()
    }

    fn with(&self, visitor: &mut dyn FnMut(&dyn VarValueAny)) {
        self.0.output.0.with(visitor);
    }

    fn get(&self) -> BoxedVarValueAny {
        self.0.output.0.get()
    }

    fn set(&self, _: BoxedVarValueAny) -> bool {
        false
    }

    fn update(&self) -> bool {
        false
    }

    fn modify(&self, _: SmallBox<dyn FnMut(&mut super::VarModifyAny) + Send + 'static, smallbox::space::S4>) -> bool {
        false
    }

    fn hook(&self, on_new: SmallBox<dyn FnMut(&crate::VarAnyHookArgs) -> bool + Send + 'static, smallbox::space::S4>) -> super::VarHandle {
        self.0.output.0.hook(on_new)
    }

    fn last_update(&self) -> crate::VarUpdateId {
        self.0.output.0.last_update()
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
