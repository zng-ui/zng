use crate::core::{
    context::WidgetContext,
    render::FrameBuilder,
    var::{IntoVar, LocalVar},
};
use crate::core::{impl_ui_node, property, units::FactorNormal, UiNode};

struct InvertNode<C: UiNode, A: LocalVar<FactorNormal>> {
    child: C,
    amount: A,
}

#[impl_ui_node(child)]
impl<C: UiNode, A: LocalVar<FactorNormal>> UiNode for InvertNode<C, A> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.amount.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.amount.update_local(ctx.vars).is_some() {
            ctx.updates.push_render();
        }
        self.child.update(ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame
            .widget_filters()
            .expect("invert property is `context`, expected `widget_filters` access")
            .push_invert(self.amount.get_local().0);

        self.child.render(frame);
    }
}

/// Inverts the colors of the widget.
#[property(context)]
pub fn invert(child: impl UiNode, amount: impl IntoVar<FactorNormal>) -> impl UiNode {
    InvertNode {
        child,
        amount: amount.into_local(),
    }
}
