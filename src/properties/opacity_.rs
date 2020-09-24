use crate::core::{
    context::WidgetContext,
    render::{FrameBinding, FrameBindingKey, FrameBuilder, FrameUpdate},
    var::{IntoVar, LocalVar, ObjVar},
};
use crate::core::{impl_ui_node, property, units::FactorNormal, UiNode};

struct OpacityNode<C: UiNode, O: LocalVar<FactorNormal>> {
    child: C,
    opacity: O,
    frame_key: Option<FrameBindingKey<f32>>,
}

#[impl_ui_node(child)]
impl<C: UiNode, O: LocalVar<FactorNormal>> UiNode for OpacityNode<C, O> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.opacity.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.opacity.update_local(ctx.vars).is_some() {
            ctx.updates.push_render_update();
        }
        self.child.update(ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        let opacity = self.opacity.get_local().0;
        let opacity = if let Some(frame_key) = self.frame_key {
            frame_key.bind(opacity)
        } else {
            FrameBinding::Value(opacity)
        };
        frame
            .widget_filters()
            .expect("opacity property is `context`, expected `widget_filters` access")
            .push_opacity(opacity);
        self.child.render(frame);
    }

    fn render_update(&self, update: &mut FrameUpdate) {
        if let Some(frame_key) = self.frame_key {
            update.update_f32(frame_key.update(self.opacity.get_local().0));
        }
        self.child.render_update(update);
    }
}

/// Opacity/transparency of the widget.
#[property(context)]
pub fn opacity(child: impl UiNode, opacity: impl IntoVar<FactorNormal>) -> impl UiNode {
    let opacity = opacity.into_local();
    let frame_key = if opacity.can_update() {
        Some(FrameBindingKey::new_unique())
    } else {
        None
    };

    OpacityNode { child, opacity, frame_key }
}
