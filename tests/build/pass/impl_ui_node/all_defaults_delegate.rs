use zero_ui::core::{impl_ui_node, NilUiNode, UiNode};

struct Node<C> {
    inner: C,
}
#[impl_ui_node(delegate = &self.inner, delegate_mut = &mut self.inner)]
impl<C: UiNode> UiNode for Node<C> {}

fn type_assert<T: UiNode>(_: T) {}

fn main() {
    type_assert(Node { inner: NilUiNode });
}
