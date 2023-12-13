use zero_ui::wgt_prelude::{ui_node, UiNode};

struct Node<C> {
    inner: C,
}
#[ui_node(delegate = &mut self.iner)]
impl<C: UiNode> UiNode for Node<C> {}

fn main() {}
