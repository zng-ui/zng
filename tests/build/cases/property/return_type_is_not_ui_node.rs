use zero_ui::core::{property, var::IntoVar, widget_instance::UiNode};

pub struct NotUiNode;

#[property(CONTEXT)]
pub fn invalid_output(child: impl UiNode, _input: impl IntoVar<bool>) -> NotUiNode {
    NotUiNode
}

fn main() {}
