use zero_ui::core::{property, widget_instance::UiNode};

#[property(context)]
pub fn no_args() -> impl UiNode {
    zero_ui::core::NilUiNode
}

fn main() {}
