use zng::prelude_wgt::{property, IntoVar, UiNode};

#[property(CONTEXT)]
pub fn has_state(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    let _ = (child, state);
    zng::prelude_wgt::NilUiNode
}

#[property(CONTEXT)]
pub fn has_state_invalid(child: impl IntoUiNode, state: impl IntoVar<u32>) -> UiNode {
    let _ = (child, state);
    zng::prelude_wgt::NilUiNode
}

fn main() {}
