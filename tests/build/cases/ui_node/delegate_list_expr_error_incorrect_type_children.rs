use zero_ui::core::{ui_node, UiNode};

struct NotANodeList;

struct MyNode {
    children: NotANodeList,
}

#[ui_node(children)]
impl UiNode for MyNode {}

fn main() {}
