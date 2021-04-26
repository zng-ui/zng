use zero_ui::core::{property, UiNode};

#[property(context)]
pub fn invalid<'a>(child: impl UiNode, input: &'a str) -> impl UiNode {
    child
}

fn main() {}
