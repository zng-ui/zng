use zero_ui::core::{property, widget_instance::UiNode, var::IntoVar};

#[property(CONTEXT)]
pub fn is_state(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    let _ = state;
    zero_ui::core::widget_instance::NilUiNode
}

#[property(CONTEXT)]
pub fn is_state_invalid(child: impl UiNode, state: impl IntoVar<u32>) -> impl UiNode {
    let _ = state;
    zero_ui::core::widget_instance::NilUiNode
}

fn main() {}
