use zero_ui::core::{impl_ui_node, UiNode};

struct NotANodeList;

struct MyNode {
    inner: NotANodeList,
}

#[impl_ui_node(delegate_list: &self.inner, delegate_list_mut: &mut self.inner)]
impl UiNode for MyNode {}

fn main() {}
