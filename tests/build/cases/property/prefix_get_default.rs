use zero_ui::wgt_prelude::{property, IntoVar, UiNode};

#[property(CONTEXT)]
pub fn get_state(child: impl UiNode, state: impl IntoVar<u32>) -> impl UiNode {
    let _ = (child, state);
    zero_ui::wgt_prelude::NilUiNode
}

#[property(CONTEXT)]
pub fn get_state_invalid(child: impl UiNode, state: impl IntoVar<NotDefault>) -> impl UiNode {
    let _ = (child, state);
    zero_ui::wgt_prelude::NilUiNode
}

#[derive(Debug, Clone)]
pub struct NotDefault {}

fn main() {}
