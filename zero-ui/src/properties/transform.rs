//! Transform properties, [`scale`](module@scale), [`rotate`](module@rotate), [`transform`](module@transform) and more.

use crate::prelude::new_property::*;

/// Custom transform.
///
/// See [`Transform`] for how to initialize a custom transform.
///
/// This property does not affect layout, the widget is transformed only during rendering.
#[property(context, default(Transform::identity()))]
pub fn transform(child: impl UiNode, transform: impl IntoVar<Transform>) -> impl UiNode {
    struct TransformNode<C, T> {
        child: C,
        transform: T,
        render_transform: Option<RenderTransform>,
    }
    #[impl_ui_node(child)]
    impl<C, T> UiNode for TransformNode<C, T>
    where
        C: UiNode,
        T: Var<Transform>,
    {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &self.transform);
            self.child.subscriptions(ctx, subscriptions);
        }

        fn init(&mut self, ctx: &mut WidgetContext) {
            self.render_transform = self.transform.get(ctx).try_render();
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);
            if let Some(t) = self.transform.get_new(ctx.vars) {
                if let Some(t) = t.try_render() {
                    self.render_transform = Some(t);
                    ctx.updates.render_update();
                } else {
                    self.render_transform = None;
                    ctx.updates.layout();
                }
            }
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
            if self.render_transform.is_none() {
                let t = self.transform.get(ctx).to_render(ctx, AvailableSize::finite(final_size));
                self.render_transform = Some(t);
                ctx.updates.render_update();
            }
            widget_layout.with_inner_transform(self.render_transform.as_ref().unwrap(), |wo| {
                self.child.arrange(ctx, wo, final_size)
            });
        }
    }
    TransformNode {
        child,
        transform: transform.into_var(),
        render_transform: None,
    }
}

/// Rotate transform.
///
/// This property is a shorthand way of setting [`transform`] to [`rotate(angle)`](units::rotate) using variable mapping.
///
/// This property does not affect layout, the widget is rotated only during rendering.
///
/// [`transform`]: fn@transform
#[property(context, default(0.rad()))]
pub fn rotate(child: impl UiNode, angle: impl IntoVar<AngleRadian>) -> impl UiNode {
    transform(child, angle.into_var().map(|&a| units::rotate(a)))
}

/// Scale transform.
///
/// This property is a shorthand way of setting [`transform`] to [`scale(s)`](units::scale) using variable mapping.
///
/// This property does not affect layout, the widget is scaled only during rendering.
///
/// [`transform`]: fn@transform
#[property(context, default(1.0))]
pub fn scale(child: impl UiNode, s: impl IntoVar<Factor>) -> impl UiNode {
    transform(child, s.into_var().map(|&x| units::scale(x)))
}

/// Scale X and Y transform.
///
/// This property is a shorthand way of setting [`transform`] to [`scale_xy(x, y)`](units::scale) using variable merging.
///
/// This property does not affect layout, the widget is scaled only during rendering.
///
/// [`transform`]: fn@transform
#[property(context, default(1.0, 1.0))]
pub fn scale_xy(child: impl UiNode, x: impl IntoVar<Factor>, y: impl IntoVar<Factor>) -> impl UiNode {
    transform(child, merge_var!(x.into_var(), y.into_var(), |&x, &y| units::scale_xy(x, y)))
}

/// Scale X transform.
///
/// This property is a shorthand way of setting [`transform`] to [`scale_x(x)`](units::scale_x) using variable mapping.
///
/// This property does not affect layout, the widget is scaled only during rendering.
///
/// [`transform`]: fn@transform
#[property(context, default(1.0))]
pub fn scale_x(child: impl UiNode, x: impl IntoVar<Factor>) -> impl UiNode {
    transform(child, x.into_var().map(|&x| units::scale_x(x)))
}

/// Scale Y transform.
///
/// This property is a shorthand way of setting [`transform`] to [`scale_y(y)`](units::scale_y) using variable mapping.
///
/// This property does not affect layout, the widget is scaled only during rendering.
///
/// [`transform`]: fn@transform
#[property(context, default(1.0))]
pub fn scale_y(child: impl UiNode, y: impl IntoVar<Factor>) -> impl UiNode {
    transform(child, y.into_var().map(|&y| units::scale_y(y)))
}

/// Skew transform.
///
/// This property is a shorthand way of setting [`transform`] to [`skew(x, y)`](units::skew) using variable merging.
///
/// This property does not affect layout, the widget is skewed only during rendering.
///
/// [`transform`]: fn@transform
#[property(context, default(0.rad(), 0.rad()))]
pub fn skew(child: impl UiNode, x: impl IntoVar<AngleRadian>, y: impl IntoVar<AngleRadian>) -> impl UiNode {
    transform(child, merge_var!(x.into_var(), y.into_var(), |&x, &y| units::skew(x, y)))
}

/// Skew X transform.
///
/// This property is a shorthand way of setting [`transform`] to [`skew_x(x)`](units::skew_x) using variable mapping.
///
/// This property does not affect layout, the widget is skewed only during rendering.
///
/// [`transform`]: fn@transform
#[property(context, default(0.rad()))]
pub fn skew_x(child: impl UiNode, x: impl IntoVar<AngleRadian>) -> impl UiNode {
    transform(child, x.into_var().map(|&x| units::skew_x(x)))
}

/// Skew Y transform.
///
/// This property is a shorthand way of setting [`transform`] to [`skew_y(y)`](units::skew_y) using variable mapping.
///
/// This property does not affect layout, the widget is skewed only during rendering.
///
/// [`transform`]: fn@transform
#[property(context)]
pub fn skew_y(child: impl UiNode, y: impl IntoVar<AngleRadian>) -> impl UiNode {
    transform(child, y.into_var().map(|&y| units::skew_y(y)))
}

/// Translate transform.
///
/// This property is a shorthand way of setting [`transform`] to [`translate(x, y)`](units::translate) using variable merging.
///
/// This property does not affect layout, the widget is moved only during rendering.
///
/// [`transform`]: fn@transform
#[property(context, default(0, 0))]
pub fn translate(child: impl UiNode, x: impl IntoVar<Length>, y: impl IntoVar<Length>) -> impl UiNode {
    transform(
        child,
        merge_var!(x.into_var(), y.into_var(), |x, y| units::translate(x.clone(), y.clone())),
    )
}

/// Translate X transform.
///
/// This property is a shorthand way of setting [`transform`] to [`translate_x(x)`](units::translate_x) using variable mapping.
///
/// This property does not affect layout, the widget is moved only during rendering.
///
/// [`transform`]: fn@transform
#[property(context, default(0))]
pub fn translate_x(child: impl UiNode, x: impl IntoVar<Length>) -> impl UiNode {
    transform(child, x.into_var().map(|x| units::translate_x(x.clone())))
}

/// Translate Y transform.
///
/// This property is a shorthand way of setting [`transform`] to [`translate_y(y)`](units::translate_y) using variable mapping.
///
/// This property does not affect layout, the widget is moved only during rendering.
///
/// [`transform`]: fn@transform
#[property(context, default(0))]
pub fn translate_y(child: impl UiNode, y: impl IntoVar<Length>) -> impl UiNode {
    transform(child, y.into_var().map(|y| units::translate_y(y.clone())))
}

/// Point relative to the widget inner bounds around which the widget transform is applied.
///
/// When unset the default origin is the center (50%, 50%).
#[property(context, default(Point::center()))]
pub fn transform_origin(child: impl UiNode, origin: impl IntoVar<Point>) -> impl UiNode {
    struct TransformOriginNode<C, O> {
        child: C,
        origin: O,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, O: Var<Point>> UiNode for TransformOriginNode<C, O> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &self.origin);
            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.origin.is_new(ctx) {
                ctx.updates.layout();
            }
            self.child.update(ctx);
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
            widget_layout.with_inner_transform_origin(self.origin.get(ctx.vars), |wl| self.child.arrange(ctx, wl, final_size));
        }
    }
    TransformOriginNode {
        child,
        origin: origin.into_var(),
    }
}
