use zero_ui::core::{
    ui_node,
    widget_instance::{NilUiNode, UiNode},
};

struct Node1 {
    inner: NilUiNode,
}
#[ui_node(delegate = &self.inner)]
impl UiNode for Node1 {}

struct Node2 {
    inner: NilUiNode,
}
#[ui_node(delegate_mut = &mut self.inner)]
impl UiNode for Node2 {}

fn main() {}
