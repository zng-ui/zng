use zero_ui::core::{ui_node, widget_instance::UiNode};

struct Node<C> {
    inner: C,
}
#[ui_node(delegate = &mut self.iner)]
impl<C: UiNode> UiNode for Node<C> {}

fn main() {}
