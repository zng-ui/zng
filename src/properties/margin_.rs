use crate::core::{
    context::WidgetContext,
    render::FrameBuilder,
    types::*,
    var::{IntoVar, LocalVar},
    UiNode,
};
use crate::core::{impl_ui_node, property};

struct Margin<T: UiNode, M: LocalVar<LayoutSideOffsets>> {
    child: T,
    margin: M,
    size_increment: LayoutSize,
    child_rect: LayoutRect,
}

#[impl_ui_node(child)]
impl<T: UiNode, M: LocalVar<LayoutSideOffsets>> UiNode for Margin<T, M> {
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

    fn measure(&mut self, available_size: LayoutSize, pixels: PixelGrid) -> LayoutSize {
        let margin = self.margin.get_local().snap_to(pixels);
        self.size_increment = LayoutSize::new(margin.left + margin.right, margin.top + margin.bottom);
        self.child_rect.origin = LayoutPoint::new(margin.left, margin.top);
        self.child.measure(available_size - self.size_increment, pixels) + self.size_increment
    }

    fn arrange(&mut self, mut final_size: LayoutSize, pixels: PixelGrid) {
        final_size -= self.size_increment;
        self.child_rect.size = final_size;
        self.child.arrange(final_size, pixels);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_reference_frame(self.child_rect.origin, |frame| self.child.render(frame));
    }
}

/// Margin space around the widget.
#[property(outer)]
pub fn margin(child: impl UiNode, margin: impl IntoVar<LayoutSideOffsets>) -> impl UiNode {
    Margin {
        child,
        margin: margin.into_local(),
        size_increment: LayoutSize::zero(),
        child_rect: LayoutRect::zero(),
    }
}
