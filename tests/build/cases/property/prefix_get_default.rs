use zng::prelude_wgt::{property, IntoVar, UiNode};

#[property(CONTEXT)]
pub fn get_state(child: impl UiNode, state: impl IntoVar<u32>) -> impl UiNode {
    let _ = (child, state);
    zng::prelude_wgt::NilUiNode
}

#[property(CONTEXT)]
pub fn get_state_invalid(child: impl UiNode, state: impl IntoVar<NotDefault>) -> impl UiNode {
    let _ = (child, state);
    zng::prelude_wgt::NilUiNode
}

#[derive(Debug, Clone)]
pub struct NotDefault {}

fn main() {}
