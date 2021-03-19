use zero_ui::core::{property, NilUiNode, UiNode};

struct NotUiNode;

#[property(context, allowed_in_when = false)]
pub fn invalid_child(child: NotUiNode, input: bool) -> impl UiNode {
    NilUiNode
}

fn main() {}
