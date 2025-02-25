use std::{any::Any, sync::Arc, time::Duration};

use super::builder::*;
use zng_layout::unit::*;
use zng_var::{
    BoxedVar, Var, VarValue,
    animation::{
        Transitionable,
        easing::{EasingStep, EasingTime},
    },
    types::{ArcWhenVar, ContextualizedVar},
};

pub use zng_app_proc_macros::easing;

type EasingFn = Arc<dyn Fn(EasingTime) -> EasingStep + Send + Sync>;

#[doc(hidden)]
#[expect(non_camel_case_types)]
pub trait easing_property: Send + Sync + Clone + Copy {
    fn easing_property_unset(self);
    fn easing_property(self, duration: Duration, easing: EasingFn) -> Vec<Box<dyn AnyPropertyBuildAction>>;
    fn easing_when_data(self, duration: Duration, easing: EasingFn) -> WhenBuildAction;
}

#[doc(hidden)]
#[expect(non_camel_case_types)]
#[diagnostic::on_unimplemented(note = "property type must be `Transitionable` to support `#[easing]`")]
pub trait easing_property_input_Transitionable: Any + Send {
    fn easing(self, duration: Duration, easing: EasingFn, when_conditions_data: &[Option<Arc<dyn Any + Send + Sync>>]) -> Self;
}
impl<T: VarValue + Transitionable> easing_property_input_Transitionable for BoxedVar<T> {
    fn easing(self, duration: Duration, easing: EasingFn, when_conditions_data: &[Option<Arc<dyn Any + Send + Sync>>]) -> Self {
        if let Some(when) = (*self).as_unboxed_any().downcast_ref::<ContextualizedVar<T>>() {
            let conditions: Vec<_> = when_conditions_data
                .iter()
                .map(|d| d.as_ref().and_then(|d| d.downcast_ref::<(Duration, EasingFn)>().cloned()))
                .collect();

            if conditions.iter().any(|c| c.is_some()) {
                let when = when.clone();
                return ContextualizedVar::new(move || {
                    when.borrow_init()
                        .as_any()
                        .downcast_ref::<ArcWhenVar<T>>()
                        .expect("expected `ArcWhenVar`")
                        .easing_when(conditions.clone(), (duration, easing.clone()))
                })
                .boxed();
            }
        } else if let Some(when) = (*self).as_unboxed_any().downcast_ref::<ArcWhenVar<T>>() {
            let conditions: Vec<_> = when_conditions_data
                .iter()
                .map(|d| d.as_ref().and_then(|d| d.downcast_ref::<(Duration, EasingFn)>().cloned()))
                .collect();

            if conditions.iter().any(|c| c.is_some()) {
                return when.easing_when(conditions.clone(), (duration, easing.clone())).boxed();
            }
        }
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
                        Box::new(PropertyBuildAction::<$T0>::new($crate::handler::clmv!(easing, |a| easing_property_input_Transitionable::easing(a.input, duration, easing.clone(), &a.when_conditions_data)))),
                        $(Box::new(PropertyBuildAction::<$T>::new($crate::handler::clmv!(easing, |a| easing_property_input_Transitionable::easing(a.input, duration, easing.clone(), &a.when_conditions_data)))),)*
                    ]
                }
            }
            fn easing_when_data(self, duration: Duration, easing: EasingFn) -> WhenBuildAction {
                if duration == Duration::ZERO {
                    WhenBuildAction::new_no_default((duration, easing))
                } else {
                    WhenBuildAction::new(
                        (duration, easing),
                        || {
                            let easing = Arc::new($crate::var::animation::easing::linear) as EasingFn;
                            vec![
                                Box::new(PropertyBuildAction::<$T0>::new($crate::handler::clmv!(easing, |a| easing_property_input_Transitionable::easing(a.input, 0.ms(), easing.clone(), &a.when_conditions_data)))),
                                $(Box::new(PropertyBuildAction::<$T>::new($crate::handler::clmv!(easing, |a| easing_property_input_Transitionable::easing(a.input, 0.ms(), easing.clone(), &a.when_conditions_data)))),)*
                            ]
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
