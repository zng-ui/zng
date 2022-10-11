use zero_ui::core::{
    context::{WidgetContext, WidgetUpdates},
    ui_node, NilUiNode, UiNode,
};

struct Node1<C> {
    child: C,
}
#[ui_node(child)]
impl<C: UiNode> UiNode for Node1<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        // calls self.child.init like the default
        // `child` impl would have done.
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
        let _ = (ctx, updates);
        // does not call self.child.update(ctx, updates);
    }
}

fn assert_type<N: zero_ui::core::UiNode>(_: N) {}

fn main() {
    // missing_delegate is a 'lint' and will become a full lint when
    // custom diagnostics is stable, meaning the UiNode impl should still
    // be generated.
    assert_type(Node1 { child: NilUiNode })
}
