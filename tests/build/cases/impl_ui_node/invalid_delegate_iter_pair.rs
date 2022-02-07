use zero_ui::core::{impl_ui_node, UiNode, ui_list::WidgetVec};

struct Node1 {
    inner: WidgetVec,
}
#[impl_ui_node(delegate_iter = self.inner.iter())]
impl UiNode for Node1 {}

struct Node2 {
    inner: WidgetVec,
}
#[impl_ui_node(delegate_iter_mut = self.inner.iter_mut())]
impl UiNode for Node2 {}

fn main() {}
