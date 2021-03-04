use zero_ui::core::{impl_ui_node, NilUiNode, UiNode};

struct Node1 {
    inner: NilUiNode,
}
#[impl_ui_node(delegate: &self.inner, delegate2: &mut self.inner)]
impl UiNode for Node1 {}

fn main() {}
