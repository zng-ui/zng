use zng::prelude_wgt::{IntoUiNode, IntoVar, UiNode, property};

#[property(CONTEXT)]
pub fn is_state(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    let _ = (child, state);
    UiNode::nil()
}

#[property(CONTEXT)]
pub fn is_state_invalid(child: impl IntoUiNode, state: impl IntoVar<u32>) -> UiNode {
    let _ = (child, state);
    UiNode::nil()
}

fn main() {}
