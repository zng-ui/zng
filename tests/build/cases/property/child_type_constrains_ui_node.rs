use zero_ui::core::{property, NilUiNode, UiNode};

#[property(context, allowed_in_when = false)]
pub fn invalid_child(child: NilUiNode, input: bool) -> impl UiNode {
    child
}

fn main() {}
