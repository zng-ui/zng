use zero_ui::core::{ui_node, UiNode, UiNodeList};

struct Node1<L> {
    inner: L,
}
#[ui_node(delegate_list = &self.inner)]
impl<L: UiNodeList> UiNode for Node1<L> {}

struct Node2<L> {
    inner: L,
}
#[ui_node(delegate_list_mut = &mut self.inner)]
impl<L> UiNode for Node2<L> {}

fn main() {}
