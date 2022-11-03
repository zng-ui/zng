use zero_ui::core::{property, widget_instance::UiNode};

#[property(context)]
pub fn invalid<'a>(child: impl UiNode, input: &'a str) -> impl UiNode {
    child
}

fn main() {}
