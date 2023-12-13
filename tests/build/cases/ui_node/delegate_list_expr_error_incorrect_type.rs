use zero_ui::wgt_prelude::{ui_node, UiNode};

struct NotANodeList;

struct MyNode {
    inner: NotANodeList,
}

#[ui_node(delegate_list = &mut self.inner)]
impl UiNode for MyNode {}

fn main() {}
