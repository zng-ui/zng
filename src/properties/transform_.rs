use crate::core::{
    context::WidgetContext,
    render::FrameBuilder,
    units::LayoutTransform,
    var::{IntoVar, LocalVar},
};
use crate::core::{impl_ui_node, property, UiNode};

struct TransformNode<C: UiNode, T: LocalVar<LayoutTransform>> {
    child: C,
    transform: T,
}

#[impl_ui_node(child)]
impl<C: UiNode, T: LocalVar<LayoutTransform>> UiNode for TransformNode<C, T> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);
        self.transform.init_local(ctx.vars);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);
        if self.transform.update_local(ctx.vars).is_some() {
            ctx.updates.push_render();
        }
    }
    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_transform(*self.transform.get_local(), |frame| self.child.render(frame))
    }
}

#[property(outer)]
pub fn transform(child: impl UiNode, transform: impl IntoVar<LayoutTransform>) -> impl UiNode {
    TransformNode {
        child,
        transform: transform.into_local(),
    }
}
