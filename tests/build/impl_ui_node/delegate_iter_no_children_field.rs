use zero_ui::core::{impl_ui_node, UiNode};

struct Node {}

#[impl_ui_node(children_iter)]
impl UiNode for Node {}

fn main() {}
