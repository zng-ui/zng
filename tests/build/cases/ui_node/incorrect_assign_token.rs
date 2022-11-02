use zero_ui::core::{
    ui_node,
    widget_instance::{NilUiNode, UiNode},
};

struct Node(NilUiNode);
#[ui_node(
    delegate: &self.0,
    delegate_mut: &mut self.0,
)]
impl UiNode for Node {}

fn main() {}
