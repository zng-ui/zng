use zero_ui::core::{ui_list::WidgetVec, ui_node, UiNode};

struct Node1 {
    inner: WidgetVec,
}
#[ui_node(delegate_iter = self.inner.iter())]
impl UiNode for Node1 {}

struct Node2 {
    inner: WidgetVec,
}
#[ui_node(delegate_iter_mut = self.inner.iter_mut())]
impl UiNode for Node2 {}

fn main() {}
