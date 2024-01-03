use std::{any::Any, sync::Arc, time::Duration};

use super::builder::*;
use zero_ui_layout::unit::*;
use zero_ui_var::{
    animation::{
        easing::{EasingStep, EasingTime},
        Transitionable,
    },
    types::{ArcWhenVar, ContextualizedVar},
    BoxedVar, Var, VarValue,
};

/// Expands a property assign to include an easing animation.
///
/// The attribute generates a [property build action] that applies [`Var::easing`] to the final variable inputs of the property.
///
/// # Arguments
///
/// The attribute takes one required argument and one optional that matches the [`Var::easing`]
/// parameters. The required first arg is the duration, the second arg is an easing function, if not present the [`easing::linear`] is used.
///
/// Some items are auto-imported in each argument scope, the [`TimeUnits`] are imported in the first argument, so you can use syntax
/// like `300.ms()` to declare the duration, all of the [`easing`] functions are imported in the second argument so you can use
/// the function names directly.
///
/// ## Unset
///
/// An alternative argument `unset` can be used instead to remove animations set by the inherited context or styles.
///
/// [`TimeUnits`]: zero_ui_unit::TimeUnits
/// [`easing`]: mod@zero_ui_var::animation::easing
/// [`easing::linear`]: zero_ui_var::animation::easing::linear
/// [property build action]: crate::widget::builder::WidgetBuilder::push_property_build_action
///
/// ## When
///
/// The attribute can also be set in `when` assigns, in this case the easing will be applied when the condition is active, so
/// only the transition to the `true` value is animated using the conditional easing.
///
/// Note that you can't `unset` easing in when conditions, but you can set it to `0.ms()`, if all easing set for a property are `0`
/// no easing variable is generated, but in contexts that actually have animation the when value will be set *immediately*,
/// by a zero sized animation.
///
/// # Examples
///
/// The example demonstrates setting and removing easing animations.
///
/// ```
/// # zero_ui_app::enable_widget_macros!();
/// # use zero_ui_app::{*, widget::{node::*, *}};
/// # use zero_ui_var::*;
/// # use zero_ui_color::*;
/// # use zero_ui_layout::unit::*;
/// # #[widget($crate::Foo)] pub struct Foo(base::WidgetBase);
/// # #[property(FILL, default(colors::BLACK))]
/// # pub fn background_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
/// #    child
/// # }
/// # #[property(LAYOUT, default(0))]
/// # pub fn margin(child: impl UiNode, color: impl IntoVar<SideOffsets>) -> impl UiNode {
/// #    child
/// # }
/// # fn main() {
/// Foo! {
///     #[easing(300.ms(), expo)] // set/override the easing.
///     background_color = colors::RED;
///
///     #[easing(unset)] // remove easing set by style or widget defaults.
///     margin = 0;
/// }
/// # ; }
/// ```
///
/// # Limitations
///
/// The attribute only works in properties that only have variable inputs of types that are `Transitionable`, if the attribute
/// is set in a property that does not match this a cryptic type error occurs, with a mention of `easing_property_input_Transitionable`.
///
#[doc(inline)]
pub use zero_ui_app_proc_macros::easing;

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
