use zero_ui::core::{context::WidgetContext, impl_ui_node, UiNode};

struct Node1<C> {
    child: C,
}
#[impl_ui_node(child)]
#[allow_(zero_ui::missing_delegate)]
impl<C: UiNode> UiNode for Node1<C> {
    fn update(&mut self, ctx: &mut WidgetContext) {
        let _ = ctx;
        // does not call self.child.update(ctx);
    }
}

struct Node2<C> {
    child: C,
}
#[impl_ui_node(child)]
impl<C: UiNode> UiNode for Node2<C> {    
    #[allow_(zero_ui::missing_delegate)]
    fn update(&mut self, ctx: &mut WidgetContext) {
        let _ = ctx;
        // does not call self.child.update(ctx);
    }
}

fn main() {}
