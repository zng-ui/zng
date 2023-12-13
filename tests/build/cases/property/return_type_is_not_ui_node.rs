use zero_ui::wgt_prelude::{property, IntoVar, UiNode};

pub struct NotUiNode;

#[property(CONTEXT)]
pub fn invalid_output(child: impl UiNode, _input: impl IntoVar<bool>) -> NotUiNode {
    NotUiNode
}

fn main() {}
