use zero_ui::core::{ui_node, widget_instance::UiNode};
struct NodeNotMut<C> {
    inner: C,
}
#[ui_node(delegate = &self.inner, delegate_mut = &self.inner)]
impl<C: UiNode> UiNode for NodeNotMut<C> {}

fn main() {}
