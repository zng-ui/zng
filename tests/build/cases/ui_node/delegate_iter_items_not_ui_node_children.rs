use zero_ui::core::{ui_node, UiNode};

struct NotUiNode;

struct MyNode {
    children: Vec<NotUiNode>,
}

#[ui_node(children_iter)]
impl UiNode for MyNode {}

fn main() {}
