use crate::core::{
    context::{LayoutContext, WidgetContext},
    render::FrameBuilder,
    units::{LayoutSize, LayoutTransform, Transform},
    var::{IntoVar, LocalVar},
};
use crate::core::{impl_ui_node, property, UiNode};

struct TransformNode<C: UiNode, T: LocalVar<Transform>> {
    child: C,
    transform: T,
    layout_transform: LayoutTransform,
}

#[impl_ui_node(child)]
impl<C: UiNode, T: LocalVar<Transform>> UiNode for TransformNode<C, T> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);
        self.transform.init_local(ctx.vars);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);
        if self.transform.update_local(ctx.vars).is_some() {
            ctx.updates.push_layout();
        }
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.layout_transform = self.transform.get_local().to_layout(final_size, ctx);
        self.child.arrange(final_size, ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_transform(self.layout_transform, |frame| self.child.render(frame))
    }
}

#[property(outer)]
pub fn transform(child: impl UiNode, transform: impl IntoVar<Transform>) -> impl UiNode {
    TransformNode {
        child,
        transform: transform.into_local(),
        layout_transform: LayoutTransform::identity(),
    }
}
