use crate::core::{
    context::WidgetContext,
    render::FrameBuilder,
    types::*,
    var::{IntoVar, LocalVar},
    UiNode,
};
use crate::{impl_ui_node, property};

struct ClipToBounds<T: UiNode, S: LocalVar<bool>> {
    child: T,
    clip: S,
    bounds: LayoutSize,
}

#[impl_ui_node(child)]
impl<T: UiNode, S: LocalVar<bool>> UiNode for ClipToBounds<T, S> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.clip.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.clip.update_local(ctx.vars).is_some() {
            ctx.updates.push_render();
        }

        self.child.update(ctx);
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        self.bounds = final_size;
        self.child.arrange(final_size)
    }

    fn render(&self, frame: &mut FrameBuilder) {
        if *self.clip.get_local() {
            frame.push_clipped(&self.child, self.bounds)
        }
        self.child.render(frame);
    }
}

#[property(inner)]
pub fn clip_to_bounds(child: impl UiNode, clip: impl IntoVar<bool>) -> impl UiNode {
    ClipToBounds {
        child,
        clip: clip.into_local(),
        bounds: LayoutSize::zero(),
    }
}
