use zero_ui::core::{ui_node, UiNode};

struct NoIterMethods;

struct MyNode {
    inner: NoIterMethods,
}

#[ui_node(delegate_iter = self.inner.iter(), delegate_iter_mut = self.inner.iter_mut())]
impl UiNode for MyNode {}

fn main() {}
