use crate::core::{
    context::{LayoutContext, WidgetContext},
    impl_ui_node, property,
    render::FrameBuilder,
};
use crate::core::{
    units::*,
    var::{IntoVar, LocalVar},
    UiNode,
};

struct PositionNode<T: UiNode, P: LocalVar<Point>> {
    child: T,
    position: P,
    final_position: LayoutPoint,
}
#[impl_ui_node(child)]
impl<T: UiNode, P: LocalVar<Point>> UiNode for PositionNode<T, P> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.position.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.position.update_local(ctx.vars).is_some() {
            ctx.updates.push_layout();
        }
        self.child.update(ctx);
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.child.arrange(final_size, ctx);
        self.final_position = self.position.get_local().to_layout(final_size, ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_reference_frame(self.final_position, |frame| self.child.render(frame));
    }
}

/// Widget left-top offset.
#[property(outer)]
pub fn position(child: impl UiNode, position: impl IntoVar<Point>) -> impl UiNode {
    PositionNode {
        child,
        position: position.into_local(),
        final_position: LayoutPoint::zero(),
    }
}
