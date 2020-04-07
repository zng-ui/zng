use crate::core::{
    context::WidgetContext,
    render::FrameBuilder,
    types::*,
    var::{IntoVar, Var},
    UiNode,
};
use crate::{impl_ui_node, property};

struct Margin<T: UiNode, M: Var<LayoutSideOffsets>> {
    child: T,
    margin: M,
    size_increment: LayoutSize,
    child_rect: LayoutRect,
}

#[impl_ui_node(child)]
impl<T: UiNode, M: Var<LayoutSideOffsets>> UiNode for Margin<T, M> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        let margin = self.margin.get(ctx.vars);
        self.child_rect.origin = LayoutPoint::new(margin.left, margin.top);
        self.size_increment = LayoutSize::new(margin.left + margin.right, margin.top + margin.bottom);

        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if let Some(margin) = self.margin.update(ctx.vars) {
            self.child_rect.origin = LayoutPoint::new(margin.left, margin.top);
            self.size_increment = LayoutSize::new(margin.left + margin.right, margin.top + margin.bottom);
            ctx.updates.push_layout();
        }

        self.child.update(ctx);
    }

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        self.child.measure(available_size - self.size_increment) + self.size_increment
    }

    fn arrange(&mut self, mut final_size: LayoutSize) {
        final_size = final_size - self.size_increment;
        self.child_rect.size = final_size;
        self.child.arrange(final_size);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_node(&self.child, &self.child_rect);
    }
}

#[property(outer)]
pub fn margin(child: impl UiNode, margin: impl IntoVar<LayoutSideOffsets>) -> impl UiNode {
    Margin {
        child,
        margin: margin.into_var(),
        size_increment: LayoutSize::zero(),
        child_rect: LayoutRect::zero(),
    }
}
