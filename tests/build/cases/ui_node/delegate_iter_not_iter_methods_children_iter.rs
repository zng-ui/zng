use zero_ui::core::{ui_node, UiNode};

struct NoIterMethods;

struct MyNode {
    children: NoIterMethods,
}

#[ui_node(children_iter)]
impl UiNode for MyNode {}

fn main() {}
