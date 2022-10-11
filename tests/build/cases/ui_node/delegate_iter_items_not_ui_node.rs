use zero_ui::core::{ui_node, UiNode};

struct NotUiNode;

struct MyNode {
    inner: Vec<NotUiNode>,
}

#[ui_node(delegate_iter = self.inner.iter(), delegate_iter_mut = self.inner.iter_mut())]
impl UiNode for MyNode {}

fn main() {}
