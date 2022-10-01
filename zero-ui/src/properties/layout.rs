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
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.margin);
            self.child.subscriptions(ctx, subs);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.margin.is_new(ctx) {
                ctx.updates.layout();
            }
            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            let margin = self.margin.get().layout(ctx.metrics, |_| PxSideOffsets::zero());
            let size_increment = PxSize::new(margin.horizontal(), margin.vertical());
            ctx.with_sub_size(size_increment, |ctx| self.child.measure(ctx))
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let margin = self.margin.get().layout(ctx.metrics, |_| PxSideOffsets::zero());
            self.size_increment = PxSize::new(margin.horizontal(), margin.vertical());

            wl.translate(PxVector::new(margin.left, margin.top));

            ctx.with_sub_size(self.size_increment, |ctx| self.child.layout(ctx, wl))
        }
    }
    MarginNode {
        child,
        margin: margin.into_var(),
        size_increment: PxSize::zero(),
    }
}

/// Margin space around the *content* of a widget.
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
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.alignment);
            self.child.subscriptions(ctx, subs);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.alignment.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            let align = self.alignment.get();
            let child_size = ctx.with_constrains(|c| align.child_constrains(c), |ctx| self.child.measure(ctx));
            align.measure(child_size, ctx.constrains())
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let align = self.alignment.get();
            let child_size = ctx.with_constrains(|c| align.child_constrains(c), |ctx| self.child.layout(ctx, wl));
            align.layout(child_size, ctx.constrains(), wl)
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

/// Widget layout offset.
///
/// Relative values are computed of the parent fill size or the widget's size, whichever is greater.
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::*;
///
/// button! {
///     offset = (100, 20.pct());
///     content = text("Click Me!")
/// }
/// # ;
/// ```
///
/// In the example the button is offset 100 layout pixels to the right and 20% of the fill height down.
///
/// # `x` and `y`
///
/// You can use the [`x`](fn@x) and [`y`](fn@y) properties to only set the position in one dimension.
#[property(layout, default((0, 0)))]
pub fn offset(child: impl UiNode, offset: impl IntoVar<Vector>) -> impl UiNode {
    struct OffsetNode<T: UiNode, O: Var<Vector>> {
        child: T,
        offset: O,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, O: Var<Vector>> UiNode for OffsetNode<T, O> {
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.offset);
            self.child.subscriptions(ctx, subs);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.offset.is_new(ctx) {
                ctx.updates.layout();
            }
            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            self.child.measure(ctx)
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let size = self.child.layout(ctx, wl);
            let offset = ctx.with_constrains(
                |c| {
                    let size = c.fill_size().max(size);
                    PxConstrains2d::new_exact_size(size)
                },
                |ctx| self.offset.get().layout(ctx.metrics, |_| PxVector::zero()),
            );
            wl.translate(offset);
            size
        }
    }
    OffsetNode {
        child,
        offset: offset.into_var(),
    }
}

/// Offset on the ***x*** axis.
///
/// Relative values are computed of the parent fill width or the widget's width, whichever is greater.
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::*;
///
/// button! {
///     x = 20.pct();
///     content = text("Click Me!")
/// };
/// # ;
/// ```
///
/// In the example the button is moved 20 percent of the fill width to the right.
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
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.x);
            self.child.subscriptions(ctx, subs);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.x.is_new(ctx) {
                ctx.updates.layout();
            }
            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            self.child.measure(ctx)
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let size = self.child.layout(ctx, wl);
            let x = ctx.with_constrains(
                |c| {
                    let size = c.fill_size().max(size);
                    PxConstrains2d::new_exact_size(size)
                },
                |ctx| self.x.get().layout(ctx.metrics.for_x(), |_| Px(0)),
            );
            wl.translate(PxVector::new(x, Px(0)));
            size
        }
    }
    XNode { child, x: x.into_var() }
}

/// Offset on the ***y*** axis.
///
/// Relative values are computed of the parent fill height or the widget's height, whichever is greater.
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::*;
///
/// button! {
///     y = 20.pct();
///     content = text("Click Me!")
/// }
/// # ;
/// ```
///
/// In the example the button is moved down 20 percent of the fill height.
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
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.y);
            self.child.subscriptions(ctx, subs);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.y.is_new(ctx) {
                ctx.updates.layout();
            }
            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            self.child.measure(ctx)
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let size = self.child.layout(ctx, wl);
            let y = ctx.with_constrains(
                |c| {
                    let size = c.fill_size().max(size);
                    PxConstrains2d::new_exact_size(size)
                },
                |ctx| self.y.get().layout(ctx.metrics.for_y(), |_| Px(0)),
            );
            wl.translate(PxVector::new(Px(0), y));
            size
        }
    }
    YNode { child, y: y.into_var() }
}

/// Minimum size of the widget.
///
/// The widget size can be larger then this but not smaller.
/// Relative values are computed from the constrains maximum bounded size.
///
/// This property does not force the minimum constrained size, the `min_size` is only used
/// in a dimension if it is greater then the constrained minimum.
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
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.min_size);
            self.child.subscriptions(ctx, subs);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.min_size.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            let min = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.min_size.get().layout(ctx.metrics, |_| PxSize::zero()),
            );
            let size = ctx.with_constrains(|c| c.with_min_size(min), |ctx| self.child.measure(ctx));
            size.max(min)
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let min = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.min_size.get().layout(ctx.metrics, |_| PxSize::zero()),
            );
            let size = ctx.with_constrains(|c| c.with_min_size(min), |ctx| self.child.layout(ctx, wl));
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
/// Relative values are computed from the constrains maximum bounded width.
///
/// This property does not force the minimum constrained width, the `min_width` is only used
/// if it is greater then the constrained minimum.
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
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.min_width);
            self.child.subscriptions(ctx, subs);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.min_width.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            let min = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.min_width.get().layout(ctx.metrics.for_x(), |_| Px(0)),
            );
            let mut size = ctx.with_constrains(|c| c.with_min_x(min), |ctx| self.child.measure(ctx));
            size.width = size.width.max(min);
            size
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let min = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.min_width.get().layout(ctx.metrics.for_x(), |_| Px(0)),
            );
            let mut size = ctx.with_constrains(|c| c.with_min_x(min), |ctx| self.child.layout(ctx, wl));
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
/// Relative values are computed from the constrains maximum bounded height.
///
/// This property does not force the minimum constrained height, the `min_height` is only used
/// if it is greater then the constrained minimum.
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
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.min_height);
            self.child.subscriptions(ctx, subs);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.min_height.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            let min = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.min_height.get().layout(ctx.metrics.for_y(), |_| Px(0)),
            );
            let mut size = ctx.with_constrains(|c| c.with_min_y(min), |ctx| self.child.measure(ctx));
            size.height = size.height.max(min);
            size
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let min = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.min_height.get().layout(ctx.metrics.for_y(), |_| Px(0)),
            );
            let mut size = ctx.with_constrains(|c| c.with_min_y(min), |ctx| self.child.layout(ctx, wl));
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
/// The widget size can be smaller then this but not larger. Relative values are computed from the
/// constrains maximum bounded size.
///
/// This property does not force the maximum constrained size, the `max_size` is only used
/// in a dimension if it is less then the constrained maximum, or the maximum was not constrained.
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
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.max_size);
            self.child.subscriptions(ctx, subs);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.max_size.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            let max = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.max_size.get().layout(ctx.metrics, |ctx| ctx.constrains().fill_size()),
            );
            let size = ctx.with_constrains(|c| c.with_max_size(max), |ctx| self.child.measure(ctx));
            size.min(max)
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let max = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.max_size.get().layout(ctx.metrics, |ctx| ctx.constrains().fill_size()),
            );
            let size = ctx.with_constrains(|c| c.with_max_size(max), |ctx| self.child.layout(ctx, wl));
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
/// Relative values are computed from the constrains maximum bounded width.
///
/// This property does not force the maximum constrained width, the `max_width` is only used
/// if it is less then the constrained maximum, or the maximum was not constrained.
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
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.max_width);
            self.child.subscriptions(ctx, subs);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.max_width.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            let max = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.max_width.get().layout(ctx.metrics.for_x(), |ctx| ctx.constrains().fill()),
            );

            let mut size = ctx.with_constrains(|c| c.with_max_x(max), |ctx| self.child.measure(ctx));
            size.width = size.width.min(max);
            size
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let max = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.max_width.get().layout(ctx.metrics.for_x(), |ctx| ctx.constrains().fill()),
            );

            let mut size = ctx.with_constrains(|c| c.with_max_x(max), |ctx| self.child.layout(ctx, wl));
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
/// Relative values are computed from the constrains maximum bounded height.
///
/// This property does not force the maximum constrained height, the `max_height` is only used
/// if it is less then the constrained maximum, or the maximum was not constrained.
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
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.max_height);
            self.child.subscriptions(ctx, subs);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.max_height.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            let max = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.max_height.get().layout(ctx.metrics.for_y(), |ctx| ctx.constrains().fill()),
            );

            let mut size = ctx.with_constrains(|c| c.with_max_y(max), |ctx| self.child.measure(ctx));
            size.height = size.height.min(max);
            size
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let max = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.max_height.get().layout(ctx.metrics.for_y(), |ctx| ctx.constrains().fill()),
            );

            let mut size = ctx.with_constrains(|c| c.with_max_y(max), |ctx| self.child.layout(ctx, wl));
            size.height = size.height.min(max);
            size
        }
    }
    MaxHeightNode {
        child,
        max_height: max_height.into_var(),
    }
}

/// Exact size of the widget.
///
/// When set the widget is sized with the given value, independent of the layout constrains.
/// Relative values are computed from the constrains maximum bounded size.
///
/// To only set a preferred size that respects the layout constrains use the [`min_size`] and [`max_size`] instead.
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
/// You can use the [`width`] and [`height`] properties to only set the size of one dimension.
///
/// [`min_size`]: fn@min_size
/// [`max_size`]: fn@max_size
/// [`width`]: fn@width
/// [`height`]: fn@height
#[property(size)]
pub fn size(child: impl UiNode, size: impl IntoVar<Size>) -> impl UiNode {
    struct SizeNode<T: UiNode, S: Var<Size>> {
        child: T,
        size: S,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, S: Var<Size>> UiNode for SizeNode<T, S> {
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.size);
            self.child.subscriptions(ctx, subs);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.size.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.size.get().layout(ctx.metrics, |ctx| ctx.constrains().fill_size()),
            )
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let size = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.size.get().layout(ctx.metrics, |ctx| ctx.constrains().fill_size()),
            );
            ctx.with_constrains(|_| PxConstrains2d::new_exact_size(size), |ctx| self.child.layout(ctx, wl));
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
/// When set the widget width is sized with the given value, independent of the layout constrains.
/// Relative values are computed from the constrains maximum bounded width.
///
/// To only set a preferred width that respects the layout constrains use the [`min_width`] and [`max_width`] instead.
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
///
/// [`min_width`]: fn@min_width
/// [`max_width`]: fn@max_width
#[property(size)]
pub fn width(child: impl UiNode, width: impl IntoVar<Length>) -> impl UiNode {
    struct WidthNode<T: UiNode, W: Var<Length>> {
        child: T,
        width: W,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, W: Var<Length>> UiNode for WidthNode<T, W> {
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.width);
            self.child.subscriptions(ctx, subs);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.width.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            let width = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.width.get().layout(ctx.metrics.for_x(), |ctx| ctx.constrains().fill()),
            );

            let mut size = ctx.with_constrains(|c| c.with_new_exact_x(width), |ctx| self.child.measure(ctx));
            size.width = width;
            size
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let width = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.width.get().layout(ctx.metrics.for_x(), |ctx| ctx.constrains().fill()),
            );

            let mut size = ctx.with_constrains(|c| c.with_new_exact_x(width), |ctx| self.child.layout(ctx, wl));
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
/// When set the widget height is sized with the given value, independent of the layout constrains.
/// Relative values are computed from the constrains maximum bounded height.
///
/// To only set a preferred height that respects the layout constrains use the [`min_height`] and [`max_height`] instead.
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
///
/// [`min_height`]: fn@min_height
/// [`max_height`]: fn@max_height
#[property(size)]
pub fn height(child: impl UiNode, height: impl IntoVar<Length>) -> impl UiNode {
    struct HeightNode<T: UiNode, H: Var<Length>> {
        child: T,
        height: H,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, H: Var<Length>> UiNode for HeightNode<T, H> {
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.height);
            self.child.subscriptions(ctx, subs);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.height.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            let height = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.height.get().layout(ctx.metrics.for_y(), |ctx| ctx.constrains().fill()),
            );
            let mut size = ctx.with_constrains(|c| c.with_new_exact_y(height), |ctx| self.child.measure(ctx));
            size.height = height;
            size
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let height = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.height.get().layout(ctx.metrics.for_y(), |ctx| ctx.constrains().fill()),
            );
            let mut size = ctx.with_constrains(|c| c.with_new_exact_y(height), |ctx| self.child.layout(ctx, wl));
            size.height = height;
            size
        }
    }
    HeightNode {
        child,
        height: height.into_var(),
    }
}

/// Set or overwrite the baseline of the widget.
///
/// The `baseline` is a vertical offset from the bottom edge of the widget's inner bounds up, it defines the
/// line where the widget naturally *sits*, some widgets like [`text!`] have a non-zero default baseline, most others leave it at zero.
///
/// Relative values are computed from the widget's height.
///
/// [`text!`]: mod@crate::widgets::text
#[property(border, default(Length::Default))]
pub fn baseline(child: impl UiNode, baseline: impl IntoVar<Length>) -> impl UiNode {
    struct BaselineNode<C, B> {
        child: C,
        baseline: B,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, B: Var<Length>> UiNode for BaselineNode<C, B> {
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.baseline);
            self.child.subscriptions(ctx, subs);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.baseline.is_new(ctx) {
                ctx.updates.layout();
            }
            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            self.child.measure(ctx)
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let size = self.child.layout(ctx, wl);

            let inner_size = ctx.widget_info.bounds.inner_size();
            let default = ctx.widget_info.bounds.baseline();

            let baseline = ctx.with_constrains(
                |c| c.with_max_size(inner_size).with_fill(true, true),
                |ctx| self.baseline.get().layout(ctx.metrics.for_y(), |_| default),
            );
            wl.set_baseline(baseline);

            size
        }
    }
    BaselineNode {
        child: child.cfg_boxed(),
        baseline: baseline.into_var(),
    }
    .cfg_boxed()
}
