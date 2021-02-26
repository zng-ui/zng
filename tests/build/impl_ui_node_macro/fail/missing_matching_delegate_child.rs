use zero_ui::core::{context::WidgetContext, impl_ui_node, NilUiNode, UiNode};

struct Node1<C> {
    child: C,
}
#[impl_ui_node(child)]
impl<C: UiNode> UiNode for Node1<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        // calls self.child.init like the default
        // `child` impl would have done.
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        let _ = ctx;
        // does not call self.child.update(ctx);
    }

    #[allow_(zero_ui::missing_delegate)]
    fn update_hp(&mut self, ctx: &mut WidgetContext) {
        let _ = ctx;
        // does not call self.child.update_hp(ctx) but it is allowed.
    }
}

fn assert_type<N: zero_ui::core::UiNode>(_: N) {}

fn main() {
    // missing_delegate is a 'lint' and will become a full lint when
    // custom diagnostics is stable, meaning the UiNode impl should still
    // be generated.
    assert_type(Node1 { child: NilUiNode })
}
