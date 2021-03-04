use zero_ui::core::{property, NilUiNode, UiNode};

#[property(context)]
pub fn invalid_child(child: NilUiNode, input: bool) -> impl UiNode {
    child
}

fn main() {}
