use zero_ui::core::{ui_node, widget_instance::UiNode};

struct Node1<C> {
    inner: C,
}
#[ui_node(delegate)]
impl<C: UiNode> UiNode for Node1<C> {}

struct Node2<C> {
    inner: C,
}
#[ui_node(delegate = &self.inner, delegate_mut)]
impl<C: UiNode> UiNode for Node2<C> {}

struct Node3<C> {
    inner: C,
}
#[ui_node(delegate =)]
impl<C: UiNode> UiNode for Node3<C> {}

fn main() {}
