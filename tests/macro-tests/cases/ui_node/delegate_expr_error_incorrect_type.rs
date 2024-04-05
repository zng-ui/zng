use zng::prelude_wgt::{ui_node, UiNode};

struct NotANode;

struct MyNode {
    inner: NotANode,
}

#[ui_node(delegate = &mut self.inner)]
impl UiNode for MyNode {}

fn main() {}
