use zero_ui::core::{property, widget_instance::UiNode};

#[property(context)]
pub fn invalid_destruct(child: impl UiNode, (a, b): (bool, u8)) -> impl UiNode {
    child
}

fn main() {}
