use zero_ui::core::{property, widget_instance::UiNode};

#[property(context)]
pub fn no_inputs(child: impl UiNode) -> impl UiNode {
    child
}

fn main() {}
