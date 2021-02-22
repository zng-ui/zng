use zero_ui::core::{impl_ui_node, ui_vec, UiNode, WidgetVec};

struct Node {
    inner: WidgetVec,
}
#[impl_ui_node(delegate_iter: self.inner.iter(), delegate_iter_mut: self.inner.iter_mut())]
impl UiNode for Node {}

fn type_assert<T: UiNode>(_: T) {}

fn main() {
    type_assert(Node { inner: ui_vec![] });
}
