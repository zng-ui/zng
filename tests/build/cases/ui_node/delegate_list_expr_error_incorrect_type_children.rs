use zero_ui::wgt_prelude::{ui_node, UiNode};

struct NotANodeList;

struct MyNode {
    children: NotANodeList,
}

#[ui_node(children)]
impl UiNode for MyNode {}

fn main() {}
