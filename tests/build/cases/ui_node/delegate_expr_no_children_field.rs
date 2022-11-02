use zero_ui::core::{ui_node, widget_instance::UiNode};

struct Node {}

#[ui_node(children)]
impl UiNode for Node {}

fn main() {}
