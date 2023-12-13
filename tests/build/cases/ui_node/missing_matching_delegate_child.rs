use zero_ui::wgt_prelude::{ui_node, NilUiNode, UiNode, WidgetUpdates};

struct Node1<C> {
    child: C,
}
#[ui_node(child)]
impl<C: UiNode> UiNode for Node1<C> {
    fn init(&mut self) {
        // calls self.child.init like the default
        // `child` impl would have done.
        self.child.init();
    }

    fn update(&mut self, updates: &WidgetUpdates) {
        let _ = updates;
        // does not call self.child.update(updates);
    }
}

fn assert_type<N: UiNode>(_: N) {}

fn main() {
    // missing_delegate is a 'lint' and will become a full lint when
    // custom diagnostics is stable, meaning the UiNode impl should still
    // be generated.
    assert_type(Node1 { child: NilUiNode })
}
