use zng::prelude_wgt::{property, IntoVar, UiNode};

#[property(CONTEXT)]
pub fn get_state(child: impl IntoUiNode, state: impl IntoVar<u32>) -> UiNode {
    let _ = (child, state);
    zng::prelude_wgt::NilUiNode
}

#[property(CONTEXT)]
pub fn get_state_invalid(child: impl IntoUiNode, state: impl IntoVar<NotDefault>) -> UiNode {
    let _ = (child, state);
    zng::prelude_wgt::NilUiNode
}

#[derive(Debug, Clone)]
pub struct NotDefault {}

fn main() {}
