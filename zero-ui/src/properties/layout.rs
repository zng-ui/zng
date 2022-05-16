//! Properties that affect the widget layout only.

use zero_ui::prelude::new_property::*;

/// Margin space around the widget.
///
/// This property adds side offsets to the widget inner visual, it will be combined with the other
/// layout properties of the widget to define the inner visual position and widget size.
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::*;
///
/// button! {
///     margin = 10;
///     content = text("Click Me!")
/// }
/// # ;
/// ```
///
/// In the example the button has `10` layout pixels of space in all directions around it. You can
/// also control each side in specific:
///
/// ```
/// # use zero_ui::prelude::*;
/// container! {
///     content = button! {
///         margin = (10, 5.pct());
///         content = text("Click Me!")
///     };
///     margin = (1, 2, 3, 4);
/// }
/// # ;
/// ```
///
/// In the example the button has `10` pixels of space above and bellow and `5%` of the container width to the left and right.
/// The container itself has margin of `1` to the top, `2` to the right, `3` to the bottom and `4` to the left.
#[property(layout, default(0))]
pub fn margin(child: impl UiNode, margin: impl IntoVar<SideOffsets>) -> impl UiNode {
    struct MarginNode<T, M> {
        child: T,
        margin: M,
        size_increment: PxSize,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, M: Var<SideOffsets>> UiNode for MarginNode<T, M> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &self.margin);
            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.margin.is_new(ctx) {
                ctx.updates.layout();
            }
            self.child.update(ctx);
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let margin = self.margin.get(ctx.vars).layout(ctx.metrics, |_| PxSideOffsets::zero());
            self.size_increment = PxSize::new(margin.horizontal(), margin.vertical());

            wl.translate(PxVector::new(margin.left, margin.top)); // TODO !!: review this, does it need to be "pre-translate".

            ctx.with_sub_size(self.size_increment, |ctx| self.child.layout(ctx, wl))
        }
    }
    MarginNode {
        child,
        margin: margin.into_var(),
        size_increment: PxSize::zero(),
    }
}

/// Margin space around the *content* of an widget.
///
/// This property is [`margin`](fn@margin) with priority `child_layout`.
#[property(child_layout, default(0))]
pub fn padding(child: impl UiNode, padding: impl IntoVar<SideOffsets>) -> impl UiNode {
    margin(child, padding)
}

/// Aligns the widget within the available space.
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::*;
///
/// container! {
///     content = button! {
///         align = Align::TOP;
///         content = text("Click Me!")
///     };
/// }
/// # ;
/// ```
///
/// In the example the button is positioned at the top-center of the container. See [`Align`] for
/// more details.
#[property(layout, default(Align::FILL))]
pub fn align(child: impl UiNode, alignment: impl IntoVar<Align>) -> impl UiNode {
    struct AlignNode<T, A> {
        child: T,
        alignment: A,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, A: Var<Align>> UiNode for AlignNode<T, A> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &self.alignment);
            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.alignment.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let align = self.alignment.get(ctx.vars);

            let size = ctx.constrains().fill_size();

            let child_size = ctx.with_constrains(
                |mut c| {
                    if align.is_fill_width() {
                        c = c.with_width_fill(size.width);
                    } else {
                        c = c.with_fill_x(false);
                    }
                    if align.is_fill_height() {
                        c = c.with_height_fill(size.height);
                    } else {
                        c = c.with_fill_y(false);
                    }
                    c
                },
                |ctx| self.child.layout(ctx, wl),
            );

            let child_rect = align.solve(child_size, Px(0), size);

            wl.translate(child_rect.origin.to_vector());

            if align.is_baseline() {
                wl.translate_baseline(1.0);
            }

            size
        }
    }

    AlignNode {
        child,
        alignment: alignment.into_var(),
    }
}

/// Aligns the widget *content* within the available space.
///
/// This property is [`align`](fn@align) with priority `child_layout`.
#[property(child_layout, default(Align::FILL))]
pub fn child_align(child: impl UiNode, alignment: impl IntoVar<Align>) -> impl UiNode {
    align(child, alignment)
}

/// Widget left-top offset.
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::*;
///
/// container! {
///     content = button! {
///         position = (100, 20.pct());
///         content = text("Click Me!")
///     };
/// }
/// # ;
/// ```
///
/// In the example the button is manually positioned `100` layout pixels from the left of the container and
/// at `20` percent of the container height from the top of the container.
///
/// # `x` and `y`
///
/// You can use the [`x`](fn@x) and [`y`](fn@y) properties to only set the position in one dimension.
#[property(layout, default((0, 0)))]
pub fn position(child: impl UiNode, position: impl IntoVar<Point>) -> impl UiNode {
    struct PositionNode<T: UiNode, P: Var<Point>> {
        child: T,
        position: P,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, P: Var<Point>> UiNode for PositionNode<T, P> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &self.position);
            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.position.is_new(ctx) {
                ctx.updates.layout();
            }
            self.child.update(ctx);
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let pos = self.position.get(ctx.vars).layout(ctx.metrics, |_| PxPoint::zero());
            wl.translate(pos.to_vector());
            self.child.layout(ctx, wl)
        }
    }
    PositionNode {
        child,
        position: position.into_var(),
    }
}

/// Left offset.
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::*;
///
/// container! {
///     content = button! {
///         x = 20.pct();
///         content = text("Click Me!")
///     };
/// }
/// # ;
/// ```
///
/// In the example the button is manually positioned at `20` percent of the container width from the left of the container.
///
/// # `position`
///
/// You can set both `x` and `y` at the same time using the [`position`](fn@position) property.
#[property(layout, default(0))]
pub fn x(child: impl UiNode, x: impl IntoVar<Length>) -> impl UiNode {
    struct XNode<T: UiNode, X: Var<Length>> {
        child: T,
        x: X,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, X: Var<Length>> UiNode for XNode<T, X> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &self.x);
            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.x.is_new(ctx) {
                ctx.updates.layout();
            }
            self.child.update(ctx);
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let x = self.x.get(ctx.vars).layout(ctx.metrics.for_x(), |_| Px(0));
            wl.translate(PxVector::new(x, Px(0)));
            self.child.layout(ctx, wl)
        }
    }
    XNode { child, x: x.into_var() }
}

/// Top offset.
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::*;
///
/// container! {
///     content = button! {
///         y = 20.pct();
///         content = text("Click Me!")
///     };
/// }
/// # ;
/// ```
///
/// In the example the button is manually positioned at `20` percent of the container height from the top of the container.
///
/// # `position`
///
/// You can set both `x` and `y` at the same time using the [`position`](fn@position) property.
#[property(layout, default(0))]
pub fn y(child: impl UiNode, y: impl IntoVar<Length>) -> impl UiNode {
    struct YNode<T: UiNode, Y: Var<Length>> {
        child: T,
        y: Y,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, Y: Var<Length>> UiNode for YNode<T, Y> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &self.y);
            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.y.is_new(ctx) {
                ctx.updates.layout();
            }
            self.child.update(ctx);
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let y = self.y.get(ctx.vars).layout(ctx.metrics.for_y(), |_| Px(0));
            wl.translate(PxVector::new(Px(0), y));
            self.child.layout(ctx, wl)
        }
    }
    YNode { child, y: y.into_var() }
}

/// Minimum size of the widget.
///
/// The widget size can be larger then this but not smaller.
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::*;
/// # let label = formatx!("");
///
/// button! {
///     content = text(label);
///     min_size = (100, 50);
/// }
/// # ;
/// ```
///
/// In the example the button will change size depending on the `label` value but it will
/// always have a minimum width of `100` and a minimum height of `50`.
///
/// # `min_width` and `min_height`
///
/// You can use the [`min_width`](fn@min_width) and [`min_height`](fn@min_height) properties to only
/// set the minimum size of one dimension.
#[property(size, default((0, 0)))]
pub fn min_size(child: impl UiNode, min_size: impl IntoVar<Size>) -> impl UiNode {
    struct MinSizeNode<T: UiNode, S: Var<Size>> {
        child: T,
        min_size: S,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, S: Var<Size>> UiNode for MinSizeNode<T, S> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &self.min_size);
            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.min_size.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let min = self.min_size.get(ctx.vars).layout(ctx.metrics, |_| PxSize::zero());
            let size = ctx.with_constrains(|c| c.with_min(min), |ctx| self.child.layout(ctx, wl));
            size.max(min)
        }
    }
    MinSizeNode {
        child,
        min_size: min_size.into_var(),
    }
}

/// Minimum width of the widget.
///
/// The widget width can be larger then this but not smaller.
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::*;
/// # let label = formatx!("");
///
/// button! {
///     content = text(label);
///     min_width = 100;
/// }
/// # ;
/// ```
///
/// In the example the button will change size depending on the `label` value but it will
/// always have a minimum width of `100`.
///
/// # `min_size`
///
/// You can set both `min_width` and `min_height` at the same time using the [`min_size`](fn@min_size) property.
#[property(size, default(0))]
pub fn min_width(child: impl UiNode, min_width: impl IntoVar<Length>) -> impl UiNode {
    struct MinWidthNode<T: UiNode, W: Var<Length>> {
        child: T,
        min_width: W,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, W: Var<Length>> UiNode for MinWidthNode<T, W> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &self.min_width);
            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.min_width.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let min = self.min_width.get(ctx.vars).layout(ctx.metrics.for_x(), |_| Px(0));
            let mut size = ctx.with_constrains(|c| c.with_min_width(min), |ctx| self.child.layout(ctx, wl));
            size.width = size.width.max(min);
            size
        }
    }
    MinWidthNode {
        child,
        min_width: min_width.into_var(),
    }
}

/// Minimum height of the widget.
///
/// The widget height can be larger then this but not smaller.
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::*;
/// # let btn_content = text("");
///
/// button! {
///     content = btn_content;
///     min_height = 50;
/// }
/// # ;
/// ```
///
/// In the example the button will change size depending on the `btn_content` value but it will
/// always have a minimum height of `50`.
///
/// # `min_size`
///
/// You can set both `min_width` and `min_height` at the same time using the [`min_size`](fn@min_size) property.
#[property(size, default(0))]
pub fn min_height(child: impl UiNode, min_height: impl IntoVar<Length>) -> impl UiNode {
    struct MinHeightNode<T: UiNode, H: Var<Length>> {
        child: T,
        min_height: H,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, H: Var<Length>> UiNode for MinHeightNode<T, H> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &self.min_height);
            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.min_height.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let min = self.min_height.get(ctx.vars).layout(ctx.metrics.for_y(), |_| Px(0));
            let mut size = ctx.with_constrains(|c| c.with_min_height(min), |ctx| self.child.layout(ctx, wl));
            size.height = size.height.max(min);
            size
        }
    }
    MinHeightNode {
        child,
        min_height: min_height.into_var(),
    }
}

/// Maximum size of the widget.
///
/// The widget size can be smaller then this but not larger.
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::*;
/// # let btn_content = text("");
///
/// button! {
///     content = btn_content;
///     max_size = (200, 100);
/// }
/// # ;
/// ```
///
/// In the example the button will change size depending on the `btn_content` value but it will
/// always have a maximum width of `200` and a maximum height of `100`.
///
/// # `max_width` and `max_height`
///
/// You can use the [`max_width`](fn@max_width) and [`max_height`](fn@max_height) properties to only
/// set the maximum size of one dimension.
#[property(size)]
pub fn max_size(child: impl UiNode, max_size: impl IntoVar<Size>) -> impl UiNode {
    struct MaxSizeNode<T: UiNode, S: Var<Size>> {
        child: T,
        max_size: S,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, S: Var<Size>> UiNode for MaxSizeNode<T, S> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &self.max_size);
            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.max_size.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let max = self.max_size.get(ctx.vars).layout(ctx.metrics, |ctx| ctx.constrains().fill_size());
            let size = ctx.with_constrains(|c| c.with_max(max), |ctx| self.child.layout(ctx, wl));
            size.min(max)
        }
    }
    MaxSizeNode {
        child,
        max_size: max_size.into_var(),
    }
}

/// Maximum width of the widget.
///
/// The widget width can be smaller then this but not larger.
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::*;
/// # let btn_content = text("");
///
/// button! {
///     content = btn_content;
///     max_width = 200;
/// }
/// # ;
/// ```
///
/// In the example the button will change size depending on the `btn_content` value but it will
/// always have a maximum width of `200`.
///
/// # `max_size`
///
/// You can set both `max_width` and `max_height` at the same time using the [`max_size`](fn@max_size) property.
#[property(size)]
pub fn max_width(child: impl UiNode, max_width: impl IntoVar<Length>) -> impl UiNode {
    struct MaxWidthNode<T: UiNode, W: Var<Length>> {
        child: T,
        max_width: W,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, W: Var<Length>> UiNode for MaxWidthNode<T, W> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &self.max_width);
            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.max_width.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let max = self
                .max_width
                .get(ctx.vars)
                .layout(ctx.metrics.for_x(), |ctx| ctx.constrains().fill_length());

            let mut size = ctx.with_constrains(|c| c.with_max_width(max), |ctx| self.child.layout(ctx, wl));
            size.width = size.width.min(max);
            size
        }
    }
    MaxWidthNode {
        child,
        max_width: max_width.into_var(),
    }
}

/// Maximum height of the widget.
///
/// The widget height can be smaller then this but not larger.
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::*;
/// # let btn_content = text("");
///
/// button! {
///     content = btn_content;
///     max_height = 100;
/// }
/// # ;
/// ```
///
/// In the example the button will change size depending on the `btn_content` value but it will
/// always have a maximum height of `100`.
///
/// # `max_size`
///
/// You can set both `max_width` and `max_height` at the same time using the [`max_size`](fn@max_size) property.
#[property(size)]
pub fn max_height(child: impl UiNode, max_height: impl IntoVar<Length>) -> impl UiNode {
    struct MaxHeightNode<T: UiNode, H: Var<Length>> {
        child: T,
        max_height: H,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, H: Var<Length>> UiNode for MaxHeightNode<T, H> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &self.max_height);
            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.max_height.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let max = self
                .max_height
                .get(ctx.vars)
                .layout(ctx.metrics.for_y(), |ctx| ctx.constrains().fill_length());

            let mut size = ctx.with_constrains(|c| c.with_max_height(max), |ctx| self.child.layout(ctx, wl));
            size.height = size.height.min(max);
            size
        }
    }
    MaxHeightNode {
        child,
        max_height: max_height.into_var(),
    }
}

/// Manually sets the size of the widget.
///
/// When set the widget is sized with the given value, independent of the parent available size.
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::*;
/// button! {
///     background_color = rgb(255, 0, 0);
///     size = (200, 300);
///     content = text("200x300 red");
/// }
/// # ;
/// ```
///
/// In the example the red button is set to a fixed size of `200` width and `300` height.
///
/// # `width` and `height`
///
/// You can use the [`width`](fn@width) and [`height`](fn@height) properties to only set the size of one dimension.
#[property(size)]
pub fn size(child: impl UiNode, size: impl IntoVar<Size>) -> impl UiNode {
    struct SizeNode<T: UiNode, S: Var<Size>> {
        child: T,
        size: S,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, S: Var<Size>> UiNode for SizeNode<T, S> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &self.size);
            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.size.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let size = self.size.get(ctx.vars).layout(ctx.metrics, |ctx| ctx.constrains().fill_size());
            ctx.with_constrains(|_| PxSizeConstrains::fixed(size), |ctx| self.child.layout(ctx, wl));
            size
        }
    }
    SizeNode {
        child,
        size: size.into_var(),
    }
}

/// Exact width of the widget.
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::*;
/// button! {
///     background_color = rgb(255, 0, 0);
///     width = 200;
///     content = text("200x? red");
/// }
/// # ;
/// ```
///
/// In the example the red button is set to a fixed width of `200`.
///
/// # `size`
///
/// You can set both `width` and `height` at the same time using the [`size`](fn@size) property.
#[property(size)]
pub fn width(child: impl UiNode, width: impl IntoVar<Length>) -> impl UiNode {
    struct WidthNode<T: UiNode, W: Var<Length>> {
        child: T,
        width: W,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, W: Var<Length>> UiNode for WidthNode<T, W> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &self.width);
            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.width.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let width = self
                .width
                .get(ctx.vars)
                .layout(ctx.metrics.for_x(), |ctx| ctx.constrains().fill_length());

            let mut size = ctx.with_constrains(|c| c.with_max_width(width).with_min_width(width), |ctx| self.child.layout(ctx, wl));
            size.width = width;
            size
        }
    }
    WidthNode {
        child,
        width: width.into_var(),
    }
}

/// Exact height of the widget.
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::*;
/// button! {
///     background_color = rgb(255, 0, 0);
///     height = 300;
///     content = text("?x300 red");
/// }
/// # ;
/// ```
///
/// In the example the red button is set to a fixed size of `300` height.
///
/// # `size`
///
/// You can set both `width` and `height` at the same time using the [`size`](fn@size) property.
#[property(size)]
pub fn height(child: impl UiNode, height: impl IntoVar<Length>) -> impl UiNode {
    struct HeightNode<T: UiNode, H: Var<Length>> {
        child: T,
        height: H,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, H: Var<Length>> UiNode for HeightNode<T, H> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &self.height);
            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.height.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let height = self
                .height
                .get(ctx.vars)
                .layout(ctx.metrics.for_y(), |ctx| ctx.constrains().fill_length());
            let mut size = ctx.with_constrains(
                |c| c.with_max_height(height).with_min_height(height),
                |ctx| self.child.layout(ctx, wl),
            );
            size.height = height;
            size
        }
    }
    HeightNode {
        child,
        height: height.into_var(),
    }
}
