use zero_ui::wgt_prelude::{ui_node, UiNode};

struct Node {}

#[ui_node(children)]
impl UiNode for Node {}

fn main() {}
