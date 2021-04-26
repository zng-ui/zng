use zero_ui::core::{impl_ui_node, NilUiNode, UiNode};

struct Node1 {
    inner: NilUiNode,
}
#[impl_ui_node(delegate = &self.inner)]
impl UiNode for Node1 {}

struct Node2 {
    inner: NilUiNode,
}
#[impl_ui_node(delegate_mut = &mut self.inner)]
impl UiNode for Node2 {}

fn main() {}
