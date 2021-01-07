//! Color filter properties, [`opacity`](mod@opacity), [`filter`](mod@filter) and more.

use crate::prelude::new_property::*;

struct FilterNode<C: UiNode, F: VarLocal<Filter>> {
    child: C,
    filter: F,
    render_filter: RenderFilter,
}
#[impl_ui_node(child)]
impl<C: UiNode, F: VarLocal<Filter>> UiNode for FilterNode<C, F> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.filter.init_local(ctx.vars);
        self.child.init(ctx)
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.filter.update_local(ctx.vars).is_some() {
            ctx.updates.layout() //TODO don't use layout when not needed.
        }
        self.child.update(ctx)
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.render_filter = self.filter.get_local().to_render(final_size, ctx);
        self.child.arrange(final_size, ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.with_widget_filter(self.render_filter.clone(), &self.child).unwrap();
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
        render_filter: RenderFilter::default(),
    }
}

/// Inverts the colors of the widget.
///
/// This property is a shorthand way of setting [`filter`] to [`color::invert(amount)`](color::invert) using variable merging.
#[property(context)]
pub fn invert_color(child: impl UiNode, amount: impl IntoVar<FactorNormal>) -> impl UiNode {
    filter(child, amount.into_var().map(|&a| color::invert(a)))
}

#[property(context)]
pub fn blur(child: impl UiNode, radius: impl IntoVar<Length>) -> impl UiNode {
    filter(child, radius.into_var().map(|&r| color::blur(r)))
}

#[property(context)]
pub fn sepia(child: impl UiNode, amount: impl IntoVar<FactorNormal>) -> impl UiNode {
    filter(child, amount.into_var().map(|&a| color::sepia(a)))
}

#[property(context)]
pub fn grayscale(child: impl UiNode, amount: impl IntoVar<FactorNormal>) -> impl UiNode {
    filter(child, amount.into_var().map(|&a| color::grayscale(a)))
}

#[property(context)]
pub fn drop_shadow(
    child: impl UiNode,
    offset: impl IntoVar<Point>,
    blur_radius: impl IntoVar<Length>,
    color: impl IntoVar<Rgba>,
) -> impl UiNode {
    filter(
        child,
        merge_var!(offset.into_var(), blur_radius.into_var(), color.into_var(), |&o, &r, &c| {
            color::drop_shadow(o, r, c)
        }),
    )
}

#[property(context)]
pub fn brightness(child: impl UiNode, amount: impl IntoVar<FactorNormal>) -> impl UiNode {
    filter(child, amount.into_var().map(|&a| color::brightness(a)))
}

#[property(context)]
pub fn contrast(child: impl UiNode, amount: impl IntoVar<FactorNormal>) -> impl UiNode {
    filter(child, amount.into_var().map(|&a| color::contrast(a)))
}

#[property(context)]
pub fn saturate(child: impl UiNode, amount: impl IntoVar<FactorNormal>) -> impl UiNode {
    filter(child, amount.into_var().map(|&a| color::saturate(a)))
}

#[property(context)]
pub fn hue_rotate(child: impl UiNode, angle: impl IntoVar<AngleDegree>) -> impl UiNode {
    filter(child, angle.into_var().map(|&a| color::hue_rotate(a)))
}

struct OpacityNode<C: UiNode, A: VarLocal<FactorNormal>> {
    child: C,
    alpha_value: A,
    frame_key: Option<FrameBindingKey<f32>>,
}

#[impl_ui_node(child)]
impl<C: UiNode, A: VarLocal<FactorNormal>> UiNode for OpacityNode<C, A> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.alpha_value.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.alpha_value.update_local(ctx.vars).is_some() {
            ctx.updates.render_update();
        }
        self.child.update(ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        let opacity = self.alpha_value.get_local().0;
        let opacity = if let Some(frame_key) = self.frame_key {
            frame_key.bind(opacity)
        } else {
            FrameBinding::Value(opacity)
        };
        frame.with_widget_opacity(opacity, &self.child).unwrap();
    }

    fn render_update(&self, update: &mut FrameUpdate) {
        if let Some(frame_key) = self.frame_key {
            update.update_f32(frame_key.update(self.alpha_value.get_local().0));
        }
        self.child.render_update(update);
    }
}

/// Opacity/transparency of the widget.
///
/// This property provides the same visual result as setting [`filter`] to [`color::opacity(opacity)`](color::opacity),
/// **but** updating the opacity is faster in this property.
#[property(context)]
pub fn opacity(child: impl UiNode, alpha: impl IntoVar<FactorNormal>) -> impl UiNode {
    let alpha_value = alpha.into_local();
    let frame_key = if alpha_value.can_update() {
        Some(FrameBindingKey::new_unique())
    } else {
        None
    };

    OpacityNode {
        child,
        alpha_value,
        frame_key,
    }
}
