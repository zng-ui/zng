use zero_ui::core::{property, widget_instance::UiNode, var::IntoVar};

#[property(CONTEXT)]
pub fn get_state(child: impl UiNode, state: impl IntoVar<u32>) -> impl UiNode {
    let _ = state;
    zero_ui::core::widget_instance::NilUiNode
}

#[property(CONTEXT)]
pub fn get_state_invalid(child: impl UiNode, state: impl IntoVar<NotDefault>) -> impl UiNode {
    let _ = state;
    zero_ui::core::widget_instance::NilUiNode
}

#[derive(Debug, Clone)]
pub struct NotDefault { }

fn main() {}
