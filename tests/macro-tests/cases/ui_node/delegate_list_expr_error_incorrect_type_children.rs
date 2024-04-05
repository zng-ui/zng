use zng::prelude_wgt::{ui_node, UiNode};

struct NotANodeList;

struct MyNode {
    children: NotANodeList,
}

#[ui_node(children)]
impl UiNode for MyNode {}

fn main() {}
