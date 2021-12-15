use zero_ui::core::{impl_ui_node, UiNode, UiNodeList};
struct NodeNotMut<C> {
    inner: C,
}
#[impl_ui_node(delegate_list = &self.inner, delegate_list_mut = &self.inner)]
impl<C: UiNodeList> UiNode for NodeNotMut<C> {}

fn main() {}
