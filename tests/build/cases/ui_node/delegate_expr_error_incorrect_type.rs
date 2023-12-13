use zero_ui::wgt_prelude::{ui_node, UiNode};

struct NotANode;

struct MyNode {
    inner: NotANode,
}

#[ui_node(delegate = &mut self.inner)]
impl UiNode for MyNode {}

fn main() {}
