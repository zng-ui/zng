use zero_ui::core::{ui_node, UiNode};

struct Node {}

#[ui_node(children_iter)]
impl UiNode for Node {}

fn main() {}
