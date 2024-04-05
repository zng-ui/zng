use zng::prelude_wgt::{ui_node, UiNode};
struct NodeNotMut<C> {
    inner: C,
}
#[ui_node(delegate = &self.inner)]
impl<C: UiNode> UiNode for NodeNotMut<C> {}

fn main() {}
