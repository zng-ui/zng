//! Read-only static variable that owns the value locally (on the var SmallBox).

use crate::{Var, VarValue};

use super::*;

struct ConstVar<T: VarValue>(T);
pub(crate) struct WeakConstVar;

impl<T: VarValue> VarImpl for ConstVar<T> {
    fn value_type(&self) -> TypeId {
        TypeId::of::<T>()
    }

    #[cfg(feature = "value_type_name")]
    fn value_type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }

    fn clone_boxed(&self) -> SmallBox<dyn VarImpl, smallbox::space::S2> {
        smallbox!(Self(self.0.clone()))
    }

    fn current_context(&self) -> SmallBox<dyn VarImpl, smallbox::space::S2> {
        self.clone_boxed()
    }

    fn strong_count(&self) -> usize {
        1
    }

    fn var_eq(&self, _: &dyn Any) -> bool {
        false
    }

    fn var_instance_tag(&self) -> VarInstanceTag {
        VarInstanceTag::NOT_SHARED
    }

    fn downgrade(&self) -> SmallBox<dyn WeakVarImpl, smallbox::space::S2> {
        smallbox!(WeakConstVar)
    }

    fn get(&self) -> BoxAnyVarValue {
        BoxAnyVarValue::new(self.0.clone())
    }

    fn set(&self, _: BoxAnyVarValue) -> bool {
        false
    }

    fn with(&self, visitor: &mut dyn FnMut(&dyn AnyVarValue)) {
        visitor(&self.0);
    }

    fn update(&self) -> bool {
        false
    }

    fn capabilities(&self) -> VarCapability {
        VarCapability::empty()
    }

    fn modify(&self, _: SmallBox<dyn FnMut(&mut VarModifyAny) + Send + 'static, smallbox::space::S4>) -> bool {
        false
    }

    fn hook(&self, _: SmallBox<dyn FnMut(&AnyVarHookArgs) -> bool + Send + 'static, smallbox::space::S4>) -> VarHandle {
        VarHandle::dummy()
    }

    fn last_update(&self) -> VarUpdateId {
        VarUpdateId::never()
    }

    fn modify_importance(&self) -> usize {
        usize::MAX
    }

    fn is_animating(&self) -> bool {
        false
    }

    fn hook_animation_stop(&self, handler: AnimationStopFn) -> Result<(), AnimationStopFn> {
        Err(handler)
    }
}

impl WeakVarImpl for WeakConstVar {
    fn strong_count(&self) -> usize {
        0
    }

    fn upgrade(&self) -> Option<SmallBox<dyn VarImpl, smallbox::space::S2>> {
        None
    }

    fn clone_boxed(&self) -> SmallBox<dyn WeakVarImpl, smallbox::space::S2> {
        smallbox!(WeakConstVar)
    }
}

/// A value-to-var conversion that consumes the value.
///
/// Every [`Var<T>`] implements this to convert to itself, every [`VarValue`] implements this to
/// convert to a [`const_var`].
///
/// This trait is used by most properties, it allows then to accept literal values, variables and context variables
/// all with a single signature. Together with [`Var<T>`] this gives properties great flexibility of usage. Widget
/// `when` blocks also use [`IntoVar<T>`] to support changing the property value depending on the widget state.
///
/// Value types can also manually implement this to support a shorthand literal syntax for when they are used in properties,
/// this converts the *shorthand value* like a tuple into the actual value type and wraps it into a variable, usually [`const_var`].
/// Value types can implement the trait multiple times to support different shorthand syntaxes.
///
/// [`const_var`]: crate::const_var
#[diagnostic::on_unimplemented(
    note = "`IntoVar<T>` is implemented for all `T: VarValue`",
    note = "`IntoVar<T>` is implemented for `Var<T>`, `ContextVar<T>` and others"
)]
pub trait IntoVar<T: VarValue> {
    #[allow(missing_docs)]
    fn into_var(self) -> Var<T>;
}
impl<T: VarValue> IntoVar<T> for T {
    fn into_var(self) -> Var<T> {
        Var::new_impl(ConstVar::<T>(self))
    }
}
impl<T: VarValue> IntoVar<T> for Var<T> {
    fn into_var(self) -> Var<T> {
        self
    }
}

pub(crate) struct AnyConstVar(BoxAnyVarValue);

impl AnyConstVar {
    pub(crate) fn new(small_box: BoxAnyVarValue) -> Self {
        Self(small_box)
    }
}
impl VarImpl for AnyConstVar {
    fn clone_boxed(&self) -> SmallBox<dyn VarImpl, smallbox::space::S2> {
        smallbox!(AnyConstVar(self.0.clone_boxed()))
    }

    fn current_context(&self) -> SmallBox<dyn VarImpl, smallbox::space::S2> {
        self.clone_boxed()
    }

    fn value_type(&self) -> TypeId {
        self.0.type_id()
    }

    #[cfg(feature = "value_type_name")]
    fn value_type_name(&self) -> &'static str {
        let mut out = "";
        self.with(&mut |v| {
            out = v.type_name();
        });
        out
    }

    fn strong_count(&self) -> usize {
        1
    }

    fn var_eq(&self, _: &dyn Any) -> bool {
        false
    }

    fn var_instance_tag(&self) -> VarInstanceTag {
        VarInstanceTag::NOT_SHARED
    }

    fn downgrade(&self) -> SmallBox<dyn WeakVarImpl, smallbox::space::S2> {
        smallbox!(WeakConstVar)
    }

    fn capabilities(&self) -> VarCapability {
        VarCapability::empty()
    }

    fn with(&self, visitor: &mut dyn FnMut(&dyn AnyVarValue)) {
        visitor(&*self.0)
    }

    fn get(&self) -> BoxAnyVarValue {
        self.0.clone_boxed()
    }

    fn set(&self, _: BoxAnyVarValue) -> bool {
        false
    }

    fn update(&self) -> bool {
        false
    }

    fn modify(&self, _: SmallBox<dyn FnMut(&mut VarModifyAny) + Send + 'static, smallbox::space::S4>) -> bool {
        false
    }

    fn hook(&self, _: SmallBox<dyn FnMut(&AnyVarHookArgs) -> bool + Send + 'static, smallbox::space::S4>) -> VarHandle {
        VarHandle::dummy()
    }

    fn last_update(&self) -> VarUpdateId {
        VarUpdateId::never()
    }

    fn modify_importance(&self) -> usize {
        usize::MAX
    }

    fn is_animating(&self) -> bool {
        false
    }

    fn hook_animation_stop(&self, handler: AnimationStopFn) -> Result<(), AnimationStopFn> {
        Err(handler)
    }
}
