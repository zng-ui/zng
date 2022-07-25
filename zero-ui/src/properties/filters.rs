//! Color filter properties, [`opacity`](fn@opacity), [`filter`](fn@filter) and more.

use crate::prelude::new_property::*;

/// Color filter, or combination of filters.
///
/// This property allows setting multiple filters at once, there is also a property for every
/// filter for easier value updating.
///
/// # Performance
///
/// The performance for setting specific filter properties versus this one is the same, except for [`opacity`]
/// which can be animated using only frame updates instead of generating a new frame every change.
///
/// [`opacity`]: fn@opacity
#[property(context, default(Filter::default()))]
pub fn filter(child: impl UiNode, filter: impl IntoVar<Filter>) -> impl UiNode {
    struct FilterNode<C, F> {
        child: C,
        filter: F,
        render_filter: Option<RenderFilter>,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, F: Var<Filter>> UiNode for FilterNode<C, F> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.render_filter = self.filter.get(ctx).try_render();
            self.child.init(ctx);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.filter);
            self.child.subscriptions(ctx, subs);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if let Some(f) = self.filter.get_new(ctx.vars) {
                if let Some(f) = f.try_render() {
                    self.render_filter = Some(f);
                    ctx.updates.render();
                } else {
                    self.render_filter = None;
                    ctx.updates.layout();
                }
            }
            self.child.update(ctx)
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            self.child.measure(ctx)
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            if self.render_filter.is_none() {
                self.render_filter = Some(self.filter.get(ctx.vars).layout(ctx.metrics));
                ctx.updates.render();
            }
            self.child.layout(ctx, wl)
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.push_inner_filter(self.render_filter.clone().unwrap(), |frame| self.child.render(ctx, frame));
        }
    }
    FilterNode {
        child,
        filter: filter.into_var(),
        render_filter: None,
    }
}

/// Inverts the colors of the widget.
///
/// Zero does not invert, one fully inverts.
///
/// This property is a shorthand way of setting [`filter`] to [`color::invert`] using variable mapping.
///
/// [`filter`]: fn@filter
#[property(context, default(false))]
pub fn invert_color(child: impl UiNode, amount: impl IntoVar<Factor>) -> impl UiNode {
    filter(child, amount.into_var().map(|&a| color::invert(a)))
}

/// Blur the widget.
///
/// This property is a shorthand way of setting [`filter`] to [`color::blur`] using variable mapping.
///
/// [`filter`]: fn@filter
#[property(context, default(0))]
pub fn blur(child: impl UiNode, radius: impl IntoVar<Length>) -> impl UiNode {
    filter(child, radius.into_var().map(|r| color::blur(r.clone())))
}

/// Sepia tone the widget.
///
/// zero is the original colors, one is the full desaturated brown look.
///
/// This property is a shorthand way of setting [`filter`] to [`color::sepia`] using variable mapping.
///
/// [`filter`]: fn@filter
#[property(context, default(false))]
pub fn sepia(child: impl UiNode, amount: impl IntoVar<Factor>) -> impl UiNode {
    filter(child, amount.into_var().map(|&a| color::sepia(a)))
}

/// Grayscale tone the widget.
///
/// Zero is the original colors, one if the full grayscale.
///
/// This property is a shorthand way of setting [`filter`] to [`color::grayscale`] using variable mapping.
///
/// [`filter`]: fn@filter
#[property(context, default(false))]
pub fn grayscale(child: impl UiNode, amount: impl IntoVar<Factor>) -> impl UiNode {
    filter(child, amount.into_var().map(|&a| color::grayscale(a)))
}

/// Drop-shadow effect for the widget.
///
/// The shadow is *pixel accurate*.
///
/// This property is a shorthand way of setting [`filter`] to [`color::drop_shadow`] using variable merging.
///
/// [`filter`]: fn@filter
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

/// Adjust the widget colors brightness.
///
/// Zero removes all brightness, one is the original brightness.
///
/// This property is a shorthand way of setting [`filter`] to [`color::brightness`] using variable mapping.
///
/// [`filter`]: fn@filter
#[property(context, default(1.0))]
pub fn brightness(child: impl UiNode, amount: impl IntoVar<Factor>) -> impl UiNode {
    filter(child, amount.into_var().map(|&a| color::brightness(a)))
}

/// Adjust the widget colors contrast.
///
/// Zero removes all contrast, one is the original contrast.
///
/// This property is a shorthand way of setting [`filter`] to [`color::brightness`] using variable mapping.
///
/// [`filter`]: fn@filter
#[property(context, default(1.0))]
pub fn contrast(child: impl UiNode, amount: impl IntoVar<Factor>) -> impl UiNode {
    filter(child, amount.into_var().map(|&a| color::contrast(a)))
}

/// Adjust the widget colors saturation.
///
/// Zero fully desaturates, one is the original saturation.
///
/// This property is a shorthand way of setting [`filter`] to [`color::saturate`] using variable mapping.
///
/// [`filter`]: fn@filter
#[property(context, default(1.0))]
pub fn saturate(child: impl UiNode, amount: impl IntoVar<Factor>) -> impl UiNode {
    filter(child, amount.into_var().map(|&a| color::saturate(a)))
}

/// Hue shift the widget colors.
///
/// Adds `angle` to the [`hue`] of the widget colors.
///
/// This property is a shorthand way of setting [`filter`] to [`color::hue_rotate`] using variable mapping.
///
/// [`filter`]: fn@filter
/// [`hue`]: Hsla::hue
#[property(context, default(0.deg()))]
pub fn hue_rotate(child: impl UiNode, angle: impl IntoVar<AngleDegree>) -> impl UiNode {
    filter(child, angle.into_var().map(|&a| color::hue_rotate(a)))
}

/// Opacity/transparency of the widget.
///
/// This property provides the same visual result as setting [`filter`] to [`color::opacity(opacity)`](color::opacity),
/// **but** updating the opacity is faster in this property.
///
/// [`filter`]: fn@filter
#[property(context, default(1.0))]
pub fn opacity(child: impl UiNode, alpha: impl IntoVar<Factor>) -> impl UiNode {
    struct OpacityNode<C, A> {
        child: C,
        alpha_value: A,
        frame_key: Option<FrameValueKey<f32>>,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, A: Var<Factor>> UiNode for OpacityNode<C, A> {
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.alpha_value);
            self.child.subscriptions(ctx, subs);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.alpha_value.is_new(ctx) {
                ctx.updates.render_update();
            }
            self.child.update(ctx);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            let opacity = self.alpha_value.get(ctx).0;
            let opacity = if let Some(frame_key) = self.frame_key {
                frame_key.bind(opacity, self.alpha_value.is_animating(ctx))
            } else {
                FrameValue::Value(opacity)
            };
            frame.push_inner_opacity(opacity, |frame| self.child.render(ctx, frame));
        }

        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            if let Some(frame_key) = self.frame_key {
                update.update_f32(frame_key.update(self.alpha_value.get(ctx).0, self.alpha_value.is_animating(ctx)));
            }
            self.child.render_update(ctx, update);
        }
    }

    let alpha_value = alpha.into_var();
    let frame_key = if alpha_value.can_update() {
        Some(FrameValueKey::new_unique())
    } else {
        None
    };
    OpacityNode {
        child,
        alpha_value,
        frame_key,
    }
}
