use zero_ui::prelude_wgt::{ui_node, NilUiNode, UiNode};

struct Node1 {
    inner: NilUiNode,
}
#[ui_node(delegate2 = &mut self.inner)]
impl UiNode for Node1 {}

fn main() {}
