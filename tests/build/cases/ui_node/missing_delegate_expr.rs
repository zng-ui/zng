use zero_ui::wgt_prelude::{ui_node, UiNode};

struct Node1<C> {
    inner: C,
}
#[ui_node(delegate)]
impl<C: UiNode> UiNode for Node1<C> {}

struct Node2<C> {
    inner: C,
}

struct Node3<C> {
    inner: C,
}
#[ui_node(delegate =)]
impl<C: UiNode> UiNode for Node3<C> {}

fn main() {}
