use zero_ui::core::{property, UiNode};

#[property(context)]
pub fn invalid_destruct(child: impl UiNode, (a, b): (bool, u8)) -> impl UiNode {
    child
}

fn main() {}
