use std::{any::TypeId, marker::PhantomData, sync::Arc, time::Duration};

use crate::{
    units::*,
    widget_builder::{AnyPropertyBuildAction, PropertyBuildAction},
};

use super::{BoxedVar, Var, VarValue};

#[doc(hidden)]
pub fn easing_property_build_action(
    duration: Duration,
    easing: impl Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
) -> Vec<Box<dyn AnyPropertyBuildAction>> {
    todo!()
}

/// Represents the strong types of each input of a property.
///
/// # Examples
///
/// The example uses [`property_input_types!`] to collect the types and compares it to a manually generated types. Note
/// that the type is a tuple even if there is only one input.
///
/// ```
/// # use zero_ui_core::{*, widget_instance::*, widget_builder::*};
/// # use std::any::Any;
/// #[property(CONTEXT)]
/// pub fn foo(child: impl UiNode, bar: impl IntoVar<bool>) -> impl UiNode {
/// #    child
/// }
///
/// assert_eq!(
///     property_input_types!(foo).type_id(),
///     PropertyInputTypes::<(BoxedVar<bool>,)>::unit().type_id(),
/// );
/// ```
///
/// You can use the collected types in advanced code generation, such as attribute proc-macros targeting property assigns in widgets.
/// The next example demonstrates a trait that uses auto-deref to convert a trait bound to a `bool`:
///
/// ```
/// # use zero_ui_core::{*, widget_instance::*, widget_builder::*};
/// #[property(CONTEXT)]
/// pub fn foo(child: impl UiNode, bar: impl IntoVar<bool>) -> impl UiNode {
/// #    child
/// }
///
/// trait SingleBoolVar {
///     fn is_single_bool_var(self) -> bool;
/// }
///
/// // match
/// impl<'a, V: Var<bool>> SingleBoolVar for &'a PropertyInputTypes<(V,)> {
///     fn is_single_bool_var(self) -> bool {
///         true
///     }
/// }
///
/// // fallback impl
/// impl<T: Send + 'static> SingleBoolVar for PropertyInputTypes<T> {
///     fn is_single_bool_var(self) -> bool {
///         false
///     }
/// }
///
/// assert!((&property_input_types!(foo)).is_single_bool_var());
/// ```
/// 
/// Learn more about how this trick works and limitations 
/// [here](https://github.com/dtolnay/case-studies/blob/master/autoref-specialization/README.md).
pub struct PropertyInputTypes<Tuple>(PhantomData<Tuple>);
impl<Tuple> PropertyInputTypes<Tuple> {
    /// Unit value.
    pub fn unit() -> Self {
        Self(PhantomData)
    }
}
impl<Tuple> Clone for PropertyInputTypes<Tuple> {
    fn clone(&self) -> Self {
        Self::unit()
    }
}
impl<Tuple> Copy for PropertyInputTypes<Tuple> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let input_types = PropertyInputTypes::<(BoxedVar<bool>,)>::unit();
        assert!((&input_types).is_single_bool_var());
    }

    #[test]
    fn test_generic() {
        let input_types = PropertyInputTypes::<(BoxedVar<bool>,)>::unit();
        assert!(generic_test(input_types)); // this fails
    }

    fn generic_test<Tuple>(input_types: PropertyInputTypes<Tuple>) -> bool {
        (&input_types).is_single_bool_var()
    }
    trait SingleBoolVar {
        fn is_single_bool_var(self) -> bool;
    }
    impl<'a, V: Var<bool>> SingleBoolVar for &'a PropertyInputTypes<(V,)> {
        fn is_single_bool_var(self) -> bool {
            true
        }
    }
    impl<T> SingleBoolVar for PropertyInputTypes<T> {
        fn is_single_bool_var(self) -> bool {
            false
        }
    }
}
