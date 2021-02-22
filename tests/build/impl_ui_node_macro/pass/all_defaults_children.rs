use zero_ui::core::{impl_ui_node, NilUiNode, UiNode, UiNodeList};

struct Node<C> {
    children: C,
}
#[impl_ui_node(children)]
impl<C: UiNodeList> UiNode for Node<C> {}

fn type_assert<T: UiNode>(_: T) {}

fn main() {
    type_assert(Node { children: [NilUiNode] });
}
