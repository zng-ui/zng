use zng::prelude_wgt::{property, IntoVar, NilUiNode, UiNode};

struct NotUiNode;

#[property(CONTEXT)]
pub fn invalid_child(child: NotUiNode, input: impl IntoVar<bool>) -> UiNode {
    let _ = (child, input);
    NilUiNode
}

fn main() {}
