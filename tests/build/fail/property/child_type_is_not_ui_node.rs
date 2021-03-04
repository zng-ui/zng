use zero_ui::core::{property, NilUiNode, UiNode};

struct NotUiNode;

#[property(context)]
pub fn invalid_child(child: NotUiNode, input: bool) -> impl UiNode {
    NilUiNode
}

fn main() {}
