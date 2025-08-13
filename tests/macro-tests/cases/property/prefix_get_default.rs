use zng::prelude_wgt::{IntoUiNode, IntoVar, UiNode, property};

#[property(CONTEXT)]
pub fn get_state(child: impl IntoUiNode, state: impl IntoVar<u32>) -> UiNode {
    let _ = (child, state);
    UiNode::nil()
}

#[property(CONTEXT)]
pub fn get_state_invalid(child: impl IntoUiNode, state: impl IntoVar<NotDefault>) -> UiNode {
    let _ = (child, state);
    UiNode::nil()
}

#[derive(Debug, Clone)]
pub struct NotDefault {}

fn main() {}
