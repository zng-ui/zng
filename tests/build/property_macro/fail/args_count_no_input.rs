use zero_ui::core::{property, UiNode};

#[property(context)]
pub fn no_inputs(child: impl UiNode) -> impl UiNode {
    child
}

fn main() {}
