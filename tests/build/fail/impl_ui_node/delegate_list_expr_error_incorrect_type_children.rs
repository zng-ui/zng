use zero_ui::core::{impl_ui_node, UiNode};

struct NotANodeList;

struct MyNode {
    children: NotANodeList,
}

#[impl_ui_node(children)]
impl UiNode for MyNode {}

fn main() {}
