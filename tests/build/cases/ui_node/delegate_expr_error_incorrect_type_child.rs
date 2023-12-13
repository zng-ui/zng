use zero_ui::wgt_prelude::{ui_node, UiNode};

struct NotANode;

struct MyNode {
    child: NotANode,
}

#[ui_node(child)]
impl UiNode for MyNode {}

fn main() {}
