use crate::core::{
    context::state_key,
    property,
    var::{BoxLocalVar, IntoVar, Var},
    UiNode,
};
use crate::properties::set_widget_state;

state_key! {
    /// Widget state key for the stack optional `spacing` property.
    pub struct StackSpacing: BoxLocalVar<f32>;
}

/// Sets the [`StackSpacing`] widget state on the attached widget.
#[property(context)]
pub fn stack_spacing(child: impl UiNode, value: impl IntoVar<f32>) -> impl UiNode {
    set_widget_state(child, StackSpacing, Box::new(value.into_var().as_local()))
}
