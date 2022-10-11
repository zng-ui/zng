use zero_ui::core::{ui_node, UiNode};

struct NotANode;

struct MyNode {
    inner: NotANode,
}

#[ui_node(delegate = &self.inner, delegate_mut = &mut self.inner)]
impl UiNode for MyNode {}

fn main() {}
