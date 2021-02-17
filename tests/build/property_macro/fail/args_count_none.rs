use zero_ui::core::{property, UiNode};

#[property(context)]
pub fn no_args() -> impl UiNode {
    zero_ui::core::NilUiNode
}

fn main() { }