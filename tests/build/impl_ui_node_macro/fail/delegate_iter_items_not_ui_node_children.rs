use zero_ui::core::{impl_ui_node, UiNode};

struct NotUiNode;

struct MyNode {
    children: Vec<NotUiNode>,
}

#[impl_ui_node(children_iter)]
impl UiNode for MyNode {}

fn main() {}
