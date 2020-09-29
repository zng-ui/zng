use crate::core::{
    context::LayoutContext,
    context::WidgetContext,
    render::FrameBuilder,
    units::*,
    var::{IntoVar, LocalVar},
    UiNode,
};
use crate::core::{impl_ui_node, property};

struct AlignNode<T: UiNode, A: LocalVar<Alignment>> {
    child: T,
    alignment: A,

    final_size: LayoutSize,
    child_rect: LayoutRect,
}

#[impl_ui_node(child)]
impl<T: UiNode, A: LocalVar<Alignment>> UiNode for AlignNode<T, A> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.alignment.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.alignment.update_local(ctx.vars).is_some() {
            ctx.updates.push_layout();
        }

        self.child.update(ctx);
    }

    fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        self.child_rect.size = self.child.measure(available_size, ctx);
        self.child_rect.size
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.final_size = final_size;
        self.child_rect.size = final_size.min(self.child_rect.size);
        self.child.arrange(self.child_rect.size, ctx);

        let alignment = self.alignment.get_local();

        self.child_rect.origin = LayoutPoint::new(
            (final_size.width - self.child_rect.size.width) * alignment.x.0,
            (final_size.height - self.child_rect.size.height) * alignment.y.0,
        )
        .snap_to(ctx.pixel_grid());
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_reference_frame(self.child_rect.origin, |frame| self.child.render(frame));
    }
}

/// Aligns the widget within the available space.
///
/// The property argument is an [`Alignment`] value.
#[property(outer)]
pub fn align(child: impl UiNode, alignment: impl IntoVar<Alignment>) -> impl UiNode {
    AlignNode {
        child,
        alignment: alignment.into_local(),
        final_size: LayoutSize::zero(),
        child_rect: LayoutRect::zero(),
    }
}
