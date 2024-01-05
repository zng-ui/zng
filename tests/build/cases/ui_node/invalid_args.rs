use zero_ui::prelude_wgt::{ui_node, UiNode};

struct Node;
#[ui_node(invalid_arg)]
impl UiNode for Node {}

fn main() {}
