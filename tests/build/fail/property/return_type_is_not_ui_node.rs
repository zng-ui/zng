use zero_ui::core::{property, UiNode};

pub struct NotUiNode;

#[property(context)]
pub fn invalid_output(child: impl UiNode, input: bool) -> NotUiNode {
    NotUiNode
}

fn main() {}
