use zero_ui::wgt_prelude::{ui_node, UiNode, UiNodeList};
struct NodeNotMut<C> {
    inner: C,
}
#[ui_node(delegate_list = &self.inner)]
impl<C: UiNodeList> UiNode for NodeNotMut<C> {}

fn main() {}
