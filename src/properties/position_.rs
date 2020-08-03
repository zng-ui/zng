use crate::core::{context::WidgetContext, impl_ui_node, property, render::FrameBuilder};
use crate::core::{
    types::*,
    var::{IntoVar, LocalVar},
    UiNode,
};

struct Position<T: UiNode, P: LocalVar<LayoutPoint>> {
    child: T,
    position: P,
    final_position: LayoutPoint,
}

#[impl_ui_node(child)]
impl<T: UiNode, P: LocalVar<LayoutPoint>> UiNode for Position<T, P> {
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

    fn arrange(&mut self, final_size: LayoutSize, pixels: PixelGrid) {
        self.child.arrange(final_size, pixels);
        self.final_position = self.position.get_local().snap_to(pixels)
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_reference_frame(self.final_position, |frame| self.child.render(frame));
    }
}

/// Widget left-top offset.
#[property(outer)]
pub fn position(child: impl UiNode, position: impl IntoVar<LayoutPoint>) -> impl UiNode {
    Position {
        child,
        position: position.into_local(),
        final_position: LayoutPoint::zero(),
    }
}
