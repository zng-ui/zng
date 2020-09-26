//! Color filter properties, [`opacity`], [`filter`] and more.

use crate::core::{
    color::{self, Filter},
    context::WidgetContext,
    render::{FrameBinding, FrameBindingKey, FrameBuilder, FrameUpdate},
    var::{IntoVar, LocalVar, ObjVar, Var},
};
use crate::core::{impl_ui_node, property, units::FactorNormal, UiNode};

struct FilterNode<C: UiNode, F: LocalVar<Filter>> {
    child: C,
    filter: F,
}
#[impl_ui_node(child)]
impl<C: UiNode, F: LocalVar<Filter>> UiNode for FilterNode<C, F> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.filter.init_local(ctx.vars);
        self.child.init(ctx)
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.filter.update_local(ctx.vars).is_some() {
            ctx.updates.push_render()
        }
        self.child.update(ctx)
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame
            .widget_filters()
            .expect("filter property is `context`, expected `widget_filters` access")
            .push_filter(self.filter.get_local().clone());
        self.child.render(frame)
    }
}

/// Color filter, or combination of filters.
///
/// This property allows setting multiple filters at once, there is also a property for every
/// filter for easier value updating.
///
/// # Performance
///
/// The performance for setting specific filter properties versus this one is the same, except for [`opacity`](module@opacity)
/// with is optimized for animation.
#[property(context)]
pub fn filter(child: impl UiNode, filter: impl IntoVar<Filter>) -> impl UiNode {
    FilterNode {
        child,
        filter: filter.into_local(),
    }
}

/// Inverts the colors of the widget.
///
/// This property is a shorthand way of setting [`filter`] to [`color::invert(amount)`](color::invert) using variable merging.
#[property(context)]
pub fn invert_color(child: impl UiNode, amount: impl IntoVar<FactorNormal>) -> impl UiNode {
    filter::set(child, amount.into_var().map(|&a| color::invert(a)))
}

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
///
/// This property provides the same visual result as setting [`filter`] to [`color::opacity(opacity)`](color::opacity),
/// **but** updating the opacity is faster in this property.
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
