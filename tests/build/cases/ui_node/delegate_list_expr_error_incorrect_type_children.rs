use zero_ui::core::{ui_node, widget_instance::UiNode};

struct NotANodeList;

struct MyNode {
    children: NotANodeList,
}

#[ui_node(children)]
impl UiNode for MyNode {}

fn main() {}
