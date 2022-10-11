use zero_ui::core::{ui_node, UiNode};

struct NotANodeList;

struct MyNode {
    inner: NotANodeList,
}

#[ui_node(delegate_list = &self.inner, delegate_list_mut = &mut self.inner)]
impl UiNode for MyNode {}

fn main() {}
