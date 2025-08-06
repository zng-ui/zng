use zng::prelude_wgt::{property, IntoVar, UiNode};

pub struct NotUiNode;

#[property(CONTEXT)]
pub fn invalid_output(_child: impl IntoUiNode, _input: impl IntoVar<bool>) -> NotUiNode {
    NotUiNode
}

fn main() {}
