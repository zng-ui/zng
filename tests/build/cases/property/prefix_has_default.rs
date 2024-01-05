use zero_ui::prelude_wgt::{property, IntoVar, UiNode};

#[property(CONTEXT)]
pub fn has_state(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    let _ = (child, state);
    zero_ui::prelude_wgt::NilUiNode
}

#[property(CONTEXT)]
pub fn has_state_invalid(child: impl UiNode, state: impl IntoVar<u32>) -> impl UiNode {
    let _ = (child, state);
    zero_ui::prelude_wgt::NilUiNode
}

fn main() {}
