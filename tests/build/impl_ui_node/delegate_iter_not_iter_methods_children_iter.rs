use zero_ui::core::{impl_ui_node, UiNode};

struct NoIterMethods;

struct MyNode {
    children: NoIterMethods,
}

#[impl_ui_node(children_iter)]
impl UiNode for MyNode {}

fn main() {}
