use zero_ui::wgt_prelude::{property, IntoVar, UiNode};

#[property(CONTEXT)]
pub fn has_state(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    let _ = (child, state);
    zero_ui::wgt_prelude::NilUiNode
}

#[property(CONTEXT)]
pub fn has_state_invalid(child: impl UiNode, state: impl IntoVar<u32>) -> impl UiNode {
    let _ = (child, state);
    zero_ui::wgt_prelude::NilUiNode
}

fn main() {}
