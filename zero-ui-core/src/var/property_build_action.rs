use std::time::Duration;

use crate::{units::*, widget_builder::AnyPropertyBuildAction};

#[doc(hidden)]
pub fn easing_property_build_action(
    duration: Duration,
    easing: impl Fn(EasingTime) -> EasingStep + Send + Sync + 'static,
) -> Vec<Box<dyn AnyPropertyBuildAction>> {
    todo!()
}
