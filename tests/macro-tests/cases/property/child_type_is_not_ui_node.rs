use zng::prelude_wgt::{IntoVar, UiNode, property};

struct NotUiNode;

#[property(CONTEXT)]
pub fn invalid_child(child: NotUiNode, input: impl IntoVar<bool>) -> UiNode {
    let _ = (child, input);
    UiNode::nil()
}

fn main() {}
