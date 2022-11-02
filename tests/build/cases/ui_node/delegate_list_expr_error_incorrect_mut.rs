use zero_ui::core::{
    ui_node,
    widget_instance::{UiNode, UiNodeList},
};
struct NodeNotMut<C> {
    inner: C,
}
#[ui_node(delegate_list = &self.inner, delegate_list_mut = &self.inner)]
impl<C: UiNodeList> UiNode for NodeNotMut<C> {}

fn main() {}
