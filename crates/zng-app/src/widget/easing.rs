use std::{any::Any, sync::Arc, time::Duration};

use super::builder::*;
use zng_layout::unit::*;
use zng_var::{
    Var, VarValue, VarWhenBuilder,
    animation::{
        Transitionable,
        easing::{EasingStep, EasingTime},
    },
};

pub use zng_app_proc_macros::easing;

type EasingFn = Arc<dyn Fn(EasingTime) -> EasingStep + Send + Sync>;

// Implemented for `PropertyInputTypes` up to 16 inputs
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
impl<T: VarValue + Transitionable> easing_property_input_Transitionable for Var<T> {
    // Called by PropertyBuildAction for each property input of properties that have #[easing(_)]
    fn easing(self, duration: Duration, easing: EasingFn, when_conditions_data: &[Option<Arc<dyn Any + Send + Sync>>]) -> Self {
        if let Some(when) = VarWhenBuilder::try_from_built(&self) {
            // property assigned normally and in when conditions, may need to coordinate multiple `#[easing(_)]` animations

            let conditions: Vec<_> = when_conditions_data
                .iter()
                .map(|d| d.as_ref().and_then(|d| d.downcast_ref::<(Duration, EasingFn)>().cloned()))
                .collect();
            if conditions.iter().any(|c| c.is_some()) {
                // at least one property assign has #[easing(duration, easing_fn)]
                // when.easing_when(conditions.clone(), (duration, easing.clone()))
                todo!()
            } else {
                // only normal property assign has #[easing(_)]
                Var::easing(&self, duration, move |t| easing(t))
            }
        } else {
            debug_assert!(when_conditions_data.is_empty());

            // property just assigned normally
            Var::easing(&self, duration, move |t| easing(t))
        }
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
            // #[easing(unset)] calls this simply to assert T: Transitionable, will generate a push_unset_property_build_action__ on the builder
            fn easing_property_unset(self) { }

            // #[easing(duration, easing?)] in normal assign calls this to build data for push_property_build_action__
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

            // #[easing(duration, easing?)] in when assign calls this to build data for push_when_build_action_data__
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
