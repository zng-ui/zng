use std::{any::Any, sync::Arc, time::Duration};

use crate::{
    units::*,
    widget_builder::{AnyPropertyBuildAction, PropertyBuildAction, PropertyInputTypes, WhenBuildAction},
};

use super::{animation::Transitionable, BoxedVar, Var, VarValue};

type EasingFn = Arc<dyn Fn(EasingTime) -> EasingStep + Send + Sync>;

#[doc(hidden)]
#[allow(non_camel_case_types)]
pub trait easing_property: Send + Sync + Clone + Copy {
    fn easing_property_unset(self);
    fn easing_property(self, duration: Duration, easing: EasingFn) -> Vec<Box<dyn AnyPropertyBuildAction>>;
    fn easing_when_data(self, duration: Duration, easing: EasingFn) -> WhenBuildAction;
}

#[doc(hidden)]
#[allow(non_camel_case_types)]
pub trait easing_property_input_Transitionable: Any + Send {
    fn easing(self, duration: Duration, easing: EasingFn) -> Self;
}
impl<T: VarValue + Transitionable> easing_property_input_Transitionable for BoxedVar<T> {
    fn easing(self, duration: Duration, easing: EasingFn) -> Self {
        Var::easing(&self, duration, move |t| easing(t)).boxed()
    }
}

macro_rules! impl_easing_property_inputs {
    ($T0:ident, $($T:ident,)*) => {
        impl_easing_property_inputs! {
            $($T,)*
        }

        impl<
            $T0: easing_property_input_Transitionable,
            $($T: easing_property_input_Transitionable),*
        > easing_property for PropertyInputTypes<($T0, $($T,)*)> {
            fn easing_property_unset(self) { }
            fn easing_property(self, duration: Duration, easing: EasingFn) -> Vec<Box<dyn AnyPropertyBuildAction>> {
                if duration == Duration::ZERO {
                    vec![]
                } else {
                    vec![
                        Box::new(PropertyBuildAction::<$T0>::new(clone_move!(easing, |v| easing_property_input_Transitionable::easing(v, duration, easing.clone())))),
                        $(Box::new(PropertyBuildAction::<$T>::new(clone_move!(easing, |v| easing_property_input_Transitionable::easing(v, duration, easing.clone())))),)*
                    ]
                }
            }
            fn easing_when_data(self, duration: Duration, easing: EasingFn) -> WhenBuildAction {
                if duration == Duration::ZERO {
                    WhenBuildAction::new_no_default((duration, easing))
                } else {
                    WhenBuildAction::new(
                        (duration, easing),
                        move |(duration, easing)| {
                            self.easing_property(*duration, easing.clone())
                        }
                    )
                }
            }
        }
    };
    () => { };
}
impl_easing_property_inputs! {
    I0, I1, I2, I3, I4, I5, I6, I7, I8, I9, I10, I11, I12, I13, I14, I15,
}
