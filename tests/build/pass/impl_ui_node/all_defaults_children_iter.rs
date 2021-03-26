use zero_ui::core::{impl_ui_node, widget_vec, UiNode, WidgetVec};

struct Node {
    children: WidgetVec,
}
#[impl_ui_node(children_iter)]
impl UiNode for Node {}

fn type_assert<T: UiNode>(_: T) {}

fn main() {
    type_assert(Node { children: widget_vec![] });
}
