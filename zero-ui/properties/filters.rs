//! Color filter properties, [`opacity`](fn@opacity), [`filter`](fn@filter) and more.

use crate::prelude::new_property::*;

/// Color filter, or combination of filters.
///
/// This property allows setting multiple filters at once, there is also a property for every
/// filter for easier value updating.
///
/// # Performance
///
/// The performance for setting specific filter properties versus this one is the same, except for [`opacity`](fn@opacity)
/// with is optimized for animation.
#[property(context, default(Filter::default()))]
pub fn filter(child: impl UiNode, filter: impl IntoVar<Filter>) -> impl UiNode {
    struct FilterNode<C, F> {
        child: C,
        filter: F,
        render_filter: RenderFilter,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, F: Var<Filter>> UiNode for FilterNode<C, F> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.filter.is_new(ctx) {
                ctx.updates.layout() //TODO don't use layout when not needed.
            }
            self.child.update(ctx)
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, final_size: LayoutSize) {
            self.render_filter = self.filter.get(ctx).to_render(ctx, final_size);
            self.child.arrange(ctx, final_size);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.with_widget_filter(self.render_filter.clone(), &self.child, ctx).unwrap();
        }
    }
    FilterNode {
        child,
        filter: filter.into_var(),
        render_filter: RenderFilter::default(),
    }
}

/// Inverts the colors of the widget.
///
/// This property is a shorthand way of setting [`filter`](fn@filter) to [`color::invert(amount)`](color::invert) using variable merging.
#[property(context, default(false))]
pub fn invert_color(child: impl UiNode, amount: impl IntoVar<FactorNormal>) -> impl UiNode {
    filter(child, amount.into_var().map(|&a| color::invert(a)))
}

#[property(context, default(0))]
pub fn blur(child: impl UiNode, radius: impl IntoVar<Length>) -> impl UiNode {
    filter(child, radius.into_var().map(|r| color::blur(r.clone())))
}

#[property(context, default(false))]
pub fn sepia(child: impl UiNode, amount: impl IntoVar<FactorNormal>) -> impl UiNode {
    filter(child, amount.into_var().map(|&a| color::sepia(a)))
}

#[property(context, default(false))]
pub fn grayscale(child: impl UiNode, amount: impl IntoVar<FactorNormal>) -> impl UiNode {
    filter(child, amount.into_var().map(|&a| color::grayscale(a)))
}

#[property(context, default((0, 0), 0, colors::BLACK.transparent()))]
pub fn drop_shadow(
    child: impl UiNode,
    offset: impl IntoVar<Point>,
    blur_radius: impl IntoVar<Length>,
    color: impl IntoVar<Rgba>,
) -> impl UiNode {
    filter(
        child,
        merge_var!(offset.into_var(), blur_radius.into_var(), color.into_var(), |o, r, &c| {
            color::drop_shadow(o.clone(), r.clone(), c)
        }),
    )
}

#[property(context, default(1.0))]
pub fn brightness(child: impl UiNode, amount: impl IntoVar<FactorNormal>) -> impl UiNode {
    filter(child, amount.into_var().map(|&a| color::brightness(a)))
}

#[property(context, default(1.0))]
pub fn contrast(child: impl UiNode, amount: impl IntoVar<FactorNormal>) -> impl UiNode {
    filter(child, amount.into_var().map(|&a| color::contrast(a)))
}

#[property(context, default(1.0))]
pub fn saturate(child: impl UiNode, amount: impl IntoVar<FactorNormal>) -> impl UiNode {
    filter(child, amount.into_var().map(|&a| color::saturate(a)))
}

#[property(context, default(0.deg()))]
pub fn hue_rotate(child: impl UiNode, angle: impl IntoVar<AngleDegree>) -> impl UiNode {
    filter(child, angle.into_var().map(|&a| color::hue_rotate(a)))
}

/// Opacity/transparency of the widget.
///
/// This property provides the same visual result as setting [`filter`] to [`color::opacity(opacity)`](color::opacity),
/// **but** updating the opacity is faster in this property.
#[property(context, default(1.0))]
pub fn opacity(child: impl UiNode, alpha: impl IntoVar<FactorNormal>) -> impl UiNode {
    struct OpacityNode<C, A> {
        child: C,
        alpha_value: A,
        frame_key: Option<FrameBindingKey<f32>>,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, A: Var<FactorNormal>> UiNode for OpacityNode<C, A> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.alpha_value.is_new(ctx) {
                ctx.updates.render_update();
            }
            self.child.update(ctx);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            let opacity = self.alpha_value.get(ctx).0;
            let opacity = if let Some(frame_key) = self.frame_key {
                frame_key.bind(opacity)
            } else {
                FrameBinding::Value(opacity)
            };
            frame.with_widget_opacity(opacity, &self.child, ctx).unwrap();
        }

        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            if let Some(frame_key) = self.frame_key {
                update.update_f32(frame_key.update(self.alpha_value.get(ctx).0));
            }
            self.child.render_update(ctx, update);
        }
    }

    let alpha_value = alpha.into_var();
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
