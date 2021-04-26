use zero_ui::core::{property, UiNode};

#[property(context, allowed_in_when = false)]
pub fn invalid_destruct(child: impl UiNode, (a, b): (bool, u8)) -> impl UiNode {
    child
}

fn main() {}
