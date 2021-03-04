use zero_ui::core::{impl_ui_node, UiNode};

struct Node<C> {
    inner: C,
}
#[impl_ui_node(delegate: &self.inner, delegate_mut: &mut self.iner)]
impl<C: UiNode> UiNode for Node<C> {}

fn main() {}
