use zero_ui::core::{impl_ui_node, NilUiNode, UiNode};

struct Node<C> {
    child: C,
}
#[impl_ui_node(child)]
impl<C: UiNode> UiNode for Node<C> {}

fn type_assert<T: UiNode>(_: T) {}

fn main() {
    type_assert(Node { child: NilUiNode });
}
