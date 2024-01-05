use zero_ui::prelude_wgt::{ui_node, UiNode};

struct NotANode;

struct MyNode {
    child: NotANode,
}

#[ui_node(child)]
impl UiNode for MyNode {}

fn main() {}
