use zero_ui::core::{impl_ui_node, UiNode};

struct NotANode;

struct MyNode {
    inner: NotANode,
}

#[impl_ui_node(delegate: &self.inner, delegate_mut: &mut self.inner)]
impl UiNode for MyNode {}

fn main() {}
