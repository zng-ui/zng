use zero_ui::core::{ui_node, UiNode};

struct Node;
#[ui_node(invalid_arg)]
impl UiNode for Node {}

fn main() {}
