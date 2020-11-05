use crate::prelude::new_property::*;

struct MarginNode<T: UiNode, M: VarLocal<SideOffsets>> {
    child: T,
    margin: M,
    size_increment: LayoutSize,
    child_rect: LayoutRect,
}
#[impl_ui_node(child)]
impl<T: UiNode, M: VarLocal<SideOffsets>> UiNode for MarginNode<T, M> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.margin.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.margin.update_local(ctx.vars).is_some() {
            ctx.updates.push_layout();
        }
        self.child.update(ctx);
    }

    fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        let margin = self.margin.get_local().to_layout(available_size, ctx);
        self.size_increment = LayoutSize::new(margin.left + margin.right, margin.top + margin.bottom);
        self.child_rect.origin = LayoutPoint::new(margin.left, margin.top);
        self.child.measure(available_size - self.size_increment, ctx) + self.size_increment
    }

    fn arrange(&mut self, mut final_size: LayoutSize, ctx: &mut LayoutContext) {
        final_size -= self.size_increment;
        self.child_rect.size = final_size;
        self.child.arrange(final_size, ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_reference_frame(self.child_rect.origin, |frame| self.child.render(frame));
    }
}

/// Margin space around the widget.
#[property(outer)]
pub fn margin(child: impl UiNode, margin: impl IntoVar<SideOffsets>) -> impl UiNode {
    MarginNode {
        child,
        margin: margin.into_local(),
        size_increment: LayoutSize::zero(),
        child_rect: LayoutRect::zero(),
    }
}
