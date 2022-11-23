use std::{any::Any, sync::Arc, time::Duration};

use crate::{
    units::*,
    var::{animation::AnimationHandle, WeakVar},
    widget_builder::{AnyPropertyBuildAction, PropertyBuildAction, PropertyInputTypes, WhenBuildAction},
};

use super::{animation::Transitionable, types, BoxedVar, ReadOnlyArcVar, Var, VarValue};

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
                        Box::new(PropertyBuildAction::<$T0>::new(clone_move!(easing, |a| easing_property_input_Transitionable::easing(a.input, duration, easing.clone())))),
                        $(Box::new(PropertyBuildAction::<$T>::new(clone_move!(easing, |a| easing_property_input_Transitionable::easing(a.input, duration, easing.clone())))),)*
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

impl<T: VarValue> super::types::ArcWhenVar<T> {
    /// Create a variable similar to [`Var::easing`], but with different duration and easing functions for each condition.
    ///
    /// The `condition_easing` must contain one entry for each when condition, entries can be `None`, the easing used
    /// is the first entry that corresponds to a `true` condition, or falls-back to the `default_easing`.
    pub fn easing_when(
        &self,
        condition_easing: Vec<Option<(Duration, EasingFn)>>,
        default_easing: (Duration, EasingFn),
    ) -> types::ContextualizedVar<T, ReadOnlyArcVar<T>>
    where
        T: Transitionable,
    {
        let source = self.clone();
        types::ContextualizedVar::new(Arc::new(move || {
            debug_assert_eq!(source.conditions().len(), condition_easing.len());

            let source_wk = source.downgrade();
            let easing_var = super::var(source.get());

            let condition_easing = condition_easing.clone();
            let default_easing = default_easing.clone();
            let mut _anim_handle = AnimationHandle::dummy();
            crate::var::var_bind(&source, &easing_var, move |vars, _, value, easing_var| {
                let source = source_wk.upgrade().unwrap();
                for ((c, _), easing) in source.conditions().iter().zip(&condition_easing) {
                    if let Some((duration, func)) = easing {
                        if c.get() {
                            let func = func.clone();
                            _anim_handle = easing_var.ease(vars, value.clone(), *duration, move |t| func(t));
                            return;
                        }
                    }
                }

                let (duration, func) = &default_easing;
                let func = func.clone();
                _anim_handle = easing_var.ease(vars, value.clone(), *duration, move |t| func(t));
            })
            .perm();
            easing_var.read_only()
        }))
    }
}
