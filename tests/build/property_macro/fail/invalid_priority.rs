use zero_ui::core::{property, UiNode};

#[property(invalid)]
pub fn invalid_priority(child: impl UiNode, input: bool) -> impl UiNode {
    let _ = input;
    child
}

fn main() {}