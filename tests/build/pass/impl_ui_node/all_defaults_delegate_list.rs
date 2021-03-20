use zero_ui::core::{impl_ui_node, NilUiNode, UiNode, UiNodeList};

struct Node<C> {
    inner: C,
}
#[impl_ui_node(delegate_list = &self.inner, delegate_list_mut = &mut self.inner)]
impl<C: UiNodeList> UiNode for Node<C> {}

fn type_assert<T: UiNode>(_: T) {}

fn main() {
    type_assert(Node { inner: [NilUiNode] });
}
