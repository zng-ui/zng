use zng::prelude_wgt::{FillUiNode, IntoVar, UiNode, property};

#[property(CONTEXT)]
pub fn invalid_child(child: FillUiNode, input: impl IntoVar<bool>) -> UiNode {
    let _ = input;
    zng::prelude_wgt::IntoUiNode::into_node(child)
}

fn main() {}
