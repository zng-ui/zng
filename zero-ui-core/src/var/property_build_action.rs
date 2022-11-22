use std::{any::Any, sync::Arc, time::Duration};

use crate::{
    units::*,
    widget_builder::{AnyPropertyBuildAction, PropertyBuildAction, PropertyInputTypes},
};

use super::{animation::Transitionable, BoxedVar, Var, VarValue};

#[doc(hidden)]
pub fn easing_property_build_action(
    duration: Duration,
    easing: impl Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
) -> Vec<Box<dyn AnyPropertyBuildAction>> {
    todo!()
}

type EasingFn = Arc<dyn Fn(EasingTime) -> EasingStep + Send + Sync>;
pub trait EasingPropertyCompatible {
    fn build(self, duration: Duration, easing: EasingFn) -> Vec<Box<dyn AnyPropertyBuildAction>>;
}
pub trait EasingInputCompatible: Any + Send {
    fn easing(self, duration: Duration, easing: EasingFn) -> Self;
}
impl<T: VarValue + Transitionable> EasingInputCompatible for BoxedVar<T> {
    fn easing(self, duration: Duration, easing: EasingFn) -> Self {
        Var::easing(&self, duration, move |t| easing(t)).boxed()
    }
}

// default fallback
impl<Tuple> EasingPropertyCompatible for PropertyInputTypes<Tuple> {
    fn build(self, _: Duration, _: EasingFn) -> Vec<Box<dyn AnyPropertyBuildAction>> {
        vec![]
    }
}
macro_rules! impl_easing_property_inputs {
    ($T0:ident, $($T:ident,)*) => {
        impl_easing_property_inputs! {
            $($T,)*
        }

        impl<'a, $T0: EasingInputCompatible, $($T: EasingInputCompatible),*> EasingPropertyCompatible for &'a PropertyInputTypes<($T0, $($T,)*)> {
            fn build(self, duration: Duration, easing: EasingFn) -> Vec<Box<dyn AnyPropertyBuildAction>> {
                vec![
                    Box::new(PropertyBuildAction::<$T0>::new(clone_move!(easing, |v| EasingInputCompatible::easing(v, duration, easing.clone())))),
                    $(Box::new(PropertyBuildAction::<$T>::new(clone_move!(easing, |v| EasingInputCompatible::easing(v, duration, easing.clone())))),)*
                ]
            }
        }
    };
    () => { };
}
impl_easing_property_inputs! {
    I0, I1, I2, I3, I4, I5, I6, I7, I8, I9, I10, I11, I12, I13, I14, I15,
}
