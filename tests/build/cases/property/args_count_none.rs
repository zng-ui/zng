use zero_ui::core::{property, widget_instance::UiNode};

#[property(CONTEXT)]
pub fn no_args() -> impl UiNode {
    zero_ui::core::widget_instance::NilUiNode
}

fn main() {}
