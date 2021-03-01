use zero_ui::core::{impl_ui_node, UiNode};

struct NotANode;

struct MyNode {
    child: NotANode,
}

#[impl_ui_node(child)]
impl UiNode for MyNode {}

fn main() {}
