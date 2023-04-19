use zero_ui::core::{
    ui_node,
    widget_instance::{NilUiNode, UiNode},
};

struct Node1 {
    inner: NilUiNode,
}
#[ui_node(delegate2 = &mut self.inner)]
impl UiNode for Node1 {}

fn main() {}
