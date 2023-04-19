use zero_ui::core::{ui_node, widget_instance::UiNode};

struct NotANodeList;

struct MyNode {
    inner: NotANodeList,
}

#[ui_node(delegate_list = &mut self.inner)]
impl UiNode for MyNode {}

fn main() {}
