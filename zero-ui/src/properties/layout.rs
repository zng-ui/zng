//! Properties that affect the widget layout only.

use std::fmt;

use zero_ui::prelude::new_property::*;

/// Margin space around the widget.
///
/// This property adds side offsets to the widget inner visual, it will be combined with the other
/// layout properties of the widget to define the inner visual position and widget size.
///
/// This property disables inline layout for the widget.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// button! {
///     margin = 10;
///     child = text!("Click Me!")
/// }
/// # ;
/// ```
///
/// In the example the button has `10` layout pixels of space in all directions around it. You can
/// also control each side in specific:
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// container! {
///     child = button! {
///         margin = (10, 5.pct());
///         child = text!("Click Me!")
///     };
///     margin = (1, 2, 3, 4);
/// }
/// # ;
/// ```
///
/// In the example the button has `10` pixels of space above and bellow and `5%` of the container width to the left and right.
/// The container itself has margin of `1` to the top, `2` to the right, `3` to the bottom and `4` to the left.
///
#[property(LAYOUT, default(0))]
pub fn margin(child: impl UiNode, margin: impl IntoVar<SideOffsets>) -> impl UiNode {
    #[ui_node(struct MarginNode {
        child: impl UiNode,
        #[var] margin: impl Var<SideOffsets>,
    })]
    impl UiNode for MarginNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.margin.is_new(ctx) {
                ctx.updates.layout();
            }
            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            let margin = self.margin.get().layout(ctx.metrics, |_| PxSideOffsets::zero());
            let size_increment = PxSize::new(margin.horizontal(), margin.vertical());
            ctx.with_inline_constrains(
                wm,
                |_| None,
                |ctx, wm| ctx.with_sub_size(size_increment, |ctx| self.child.measure(ctx, wm)),
            )
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let margin = self.margin.get().layout(ctx.metrics, |_| PxSideOffsets::zero());
            let size_increment = PxSize::new(margin.horizontal(), margin.vertical());

            wl.translate(PxVector::new(margin.left, margin.top));
            ctx.with_sub_size(size_increment, |ctx| self.child.layout(ctx, wl))
        }
    }
    MarginNode {
        child,
        margin: margin.into_var(),
    }
}

/// Margin space around the *content* of a widget.
///
/// This property is [`margin`](fn@margin) with nest group `CHILD_LAYOUT`.
#[property(CHILD_LAYOUT, default(0))]
pub fn padding(child: impl UiNode, padding: impl IntoVar<SideOffsets>) -> impl UiNode {
    margin(child, padding)
}

/// Aligns the widget within the available space.
///
/// This property disables inline layout for the widget.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// #
/// container! {
///     child = button! {
///         align = Align::TOP;
///         child = text!("Click Me!")
///     };
/// }
/// # ;
/// ```
///
/// In the example the button is positioned at the top-center of the container. See [`Align`] for
/// more details.
#[property(LAYOUT, default(Align::FILL))]
pub fn align(child: impl UiNode, alignment: impl IntoVar<Align>) -> impl UiNode {
    #[ui_node(struct AlignNode {
        child: impl UiNode,
        #[var] alignment: impl Var<Align>,
    })]
    impl UiNode for AlignNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.alignment.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            let align = self.alignment.get();
            let child_size = ctx.with_inline_constrains(
                wm,
                |_| None,
                |ctx, wm| ctx.with_constrains(|c| align.child_constrains(c), |ctx| self.child.measure(ctx, wm)),
            );
            align.measure(child_size, ctx.constrains())
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let align = self.alignment.get();
            let child_size = ctx.with_constrains(|c| align.child_constrains(c), |ctx| self.child.layout(ctx, wl));
            align.layout(child_size, ctx.constrains(), ctx.direction(), wl)
        }
    }
    AlignNode {
        child,
        alignment: alignment.into_var(),
    }
}

/// Aligns the widget *content* within the available space.
///
/// This property is [`align`](fn@align) with nest group `CHILD_LAYOUT`.
#[property(CHILD_LAYOUT, default(Align::FILL))]
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
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
///
/// button! {
///     offset = (100, 20.pct());
///     child = text!("Click Me!")
/// }
/// # ;
/// ```
///
/// In the example the button is offset 100 layout pixels to the right and 20% of the fill height down.
///
/// # `x` and `y`
///
/// You can use the [`x`](fn@x) and [`y`](fn@y) properties to only set the position in one dimension.
#[property(LAYOUT, default((0, 0)))]
pub fn offset(child: impl UiNode, offset: impl IntoVar<Vector>) -> impl UiNode {
    #[ui_node(struct OffsetNode {
        child: impl UiNode,
        #[var] offset: impl Var<Vector>,
    })]
    impl UiNode for OffsetNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.offset.is_new(ctx) {
                ctx.updates.layout();
            }
            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            self.child.measure(ctx, wm)
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
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
///
/// button! {
///     x = 20.pct();
///     child = text!("Click Me!")
/// };
/// # ;
/// ```
///
/// In the example the button is moved 20 percent of the fill width to the right.
///
/// # `offset`
///
/// You can set both `x` and `y` at the same time using the [`offset`](fn@offset) property.
#[property(LAYOUT, default(0))]
pub fn x(child: impl UiNode, x: impl IntoVar<Length>) -> impl UiNode {
    #[ui_node(struct XNode {
        child: impl UiNode,
        #[var] x: impl Var<Length>,
    })]
    impl UiNode for XNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.x.is_new(ctx) {
                ctx.updates.layout();
            }
            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            self.child.measure(ctx, wm)
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
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
///
/// button! {
///     y = 20.pct();
///     child = text!("Click Me!")
/// }
/// # ;
/// ```
///
/// In the example the button is moved down 20 percent of the fill height.
///
/// # `offset`
///
/// You can set both `x` and `y` at the same time using the [`offset`](fn@offset) property.
#[property(LAYOUT, default(0))]
pub fn y(child: impl UiNode, y: impl IntoVar<Length>) -> impl UiNode {
    #[ui_node(struct YNode {
        child: impl UiNode,
        #[var] y: impl Var<Length>,
    })]
    impl UiNode for YNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.y.is_new(ctx) {
                ctx.updates.layout();
            }
            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            self.child.measure(ctx, wm)
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
/// This property disables inline layout for the widget.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # let label = formatx!("");
///
/// button! {
///     child = text!(label);
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
#[property(SIZE-2, default((0, 0)))]
pub fn min_size(child: impl UiNode, min_size: impl IntoVar<Size>) -> impl UiNode {
    #[ui_node(struct MinSizeNode {
        child: impl UiNode,
        #[var] min_size: impl Var<Size>,
    })]
    impl UiNode for MinSizeNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.min_size.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            let min = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.min_size.get().layout(ctx.metrics, |_| PxSize::zero()),
            );
            let size = ctx.with_inline_constrains(
                wm,
                |_| None,
                |ctx, wm| ctx.with_constrains(|c| c.with_min_size(min), |ctx| self.child.measure(ctx, wm)),
            );
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
/// This property disables inline layout for the widget.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # let label = formatx!("");
///
/// button! {
///     child = text!(label);
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
#[property(SIZE-2, default(0))]
pub fn min_width(child: impl UiNode, min_width: impl IntoVar<Length>) -> impl UiNode {
    #[ui_node(struct MinWidthNode {
        child: impl UiNode,
        #[var] min_width: impl Var<Length>,
    })]
    impl UiNode for MinWidthNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.min_width.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            let min = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.min_width.get().layout(ctx.metrics.for_x(), |_| Px(0)),
            );
            let mut size = ctx.with_inline_constrains(
                wm,
                |_| None,
                |ctx, wm| ctx.with_constrains(|c| c.with_min_x(min), |ctx| self.child.measure(ctx, wm)),
            );
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
/// This property disables inline layout for the widget.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # let btn_content = text!("");
/// #
/// button! {
///     child = btn_content;
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
#[property(SIZE-2, default(0))]
pub fn min_height(child: impl UiNode, min_height: impl IntoVar<Length>) -> impl UiNode {
    #[ui_node(struct MinHeightNode {
        child: impl UiNode,
        #[var] min_height: impl Var<Length>,
    })]
    impl UiNode for MinHeightNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.min_height.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            let min = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.min_height.get().layout(ctx.metrics.for_y(), |_| Px(0)),
            );
            let mut size = ctx.with_inline_constrains(
                wm,
                |_| None,
                |ctx, wm| ctx.with_constrains(|c| c.with_min_y(min), |ctx| self.child.measure(ctx, wm)),
            );
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
/// This property disables inline layout for the widget.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # let btn_content = text!("");
/// #
/// button! {
///     child = btn_content;
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
#[property(SIZE-1)]
pub fn max_size(child: impl UiNode, max_size: impl IntoVar<Size>) -> impl UiNode {
    #[ui_node(struct MaxSizeNode {
        child: impl UiNode,
        #[var] max_size: impl Var<Size>,
    })]
    impl UiNode for MaxSizeNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.max_size.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            let max = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.max_size.get().layout(ctx.metrics, |ctx| ctx.constrains().fill_size()),
            );
            let size = ctx.with_inline_constrains(
                wm,
                |_| None,
                |ctx, wm| ctx.with_constrains(|c| c.with_max_size(max), |ctx| self.child.measure(ctx, wm)),
            );
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
/// This property disables inline layout for the widget.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # let btn_content = text!("");
///
/// button! {
///     child = btn_content;
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
#[property(SIZE-1)]
pub fn max_width(child: impl UiNode, max_width: impl IntoVar<Length>) -> impl UiNode {
    #[ui_node(struct MaxWidthNode {
        child: impl UiNode,
        #[var] max_width: impl Var<Length>,
    })]
    impl UiNode for MaxWidthNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.max_width.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            let max = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.max_width.get().layout(ctx.metrics.for_x(), |ctx| ctx.constrains().fill()),
            );

            let mut size = ctx.with_inline_constrains(
                wm,
                |_| None,
                |ctx, wm| ctx.with_constrains(|c| c.with_max_x(max), |ctx| self.child.measure(ctx, wm)),
            );
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
/// This property disables inline layout for the widget.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// # let btn_content = text!("");
///
/// button! {
///     child = btn_content;
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
#[property(SIZE-1)]
pub fn max_height(child: impl UiNode, max_height: impl IntoVar<Length>) -> impl UiNode {
    #[ui_node(struct MaxHeightNode {
        child: impl UiNode,
        #[var] max_height: impl Var<Length>,
    })]
    impl UiNode for MaxHeightNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.max_height.is_new(ctx) {
                ctx.updates.layout();
            }

            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            let max = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.max_height.get().layout(ctx.metrics.for_y(), |ctx| ctx.constrains().fill()),
            );

            let mut size = ctx.with_inline_constrains(
                wm,
                |_| None,
                |ctx, wm| ctx.with_constrains(|c| c.with_max_y(max), |ctx| self.child.measure(ctx, wm)),
            );
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
/// When set the widget is layout with exact size constrains, clamped by the contextual min/max.
/// Relative values are computed from the constrains maximum bounded size.
///
/// This property disables inline layout for the widget. This property sets the [`SIZE_PROPERTY_INFO_ID`].
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// button! {
///     background_color = rgb(255, 0, 0);
///     size = (200, 300);
///     child = text!("200x300 red");
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
#[property(SIZE)]
pub fn size(child: impl UiNode, size: impl IntoVar<Size>) -> impl UiNode {
    #[ui_node(struct SizeNode {
        child: impl UiNode,
        #[var] size: impl Var<Size>,
    })]
    impl UiNode for SizeNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.auto_subs(ctx);
            self.child.init(ctx);
            self.size.with(|s| SizePropertyLength::set(ctx.widget_state.reborrow(), s));
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.child.update(ctx, updates);
            self.size.with_new(ctx.vars, |s| {
                SizePropertyLength::set(ctx.widget_state.reborrow(), s);
                ctx.updates.layout();
            });
        }

        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            let size = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.size.get().layout(ctx.metrics, |ctx| ctx.constrains().fill_size()),
            );
            let size = ctx.constrains().clamp_size(size);
            ctx.with_inline_constrains(
                wm,
                |_| None,
                |ctx, wm| ctx.with_constrains(|_| PxConstrains2d::new_exact_size(size), |ctx| self.child.measure(ctx, wm)),
            );
            size
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let size = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.size.get().layout(ctx.metrics, |ctx| ctx.constrains().fill_size()),
            );
            let size = ctx.constrains().clamp_size(size);
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
/// When set the widget is layout with exact size constrains, clamped by the contextual min/max.
/// Relative values are computed from the constrains maximum bounded width.
///
/// This property disables inline layout for the widget. This property sets the [`SIZE_PROPERTY_INFO_ID`] width.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// button! {
///     background_color = rgb(255, 0, 0);
///     width = 200;
///     child = text!("200x? red");
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
#[property(SIZE)]
pub fn width(child: impl UiNode, width: impl IntoVar<Length>) -> impl UiNode {
    #[ui_node(struct WidthNode {
        child: impl UiNode,
        #[var] width: impl Var<Length>,
    })]
    impl UiNode for WidthNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.auto_subs(ctx);
            self.child.init(ctx);
            self.width.with(|w| SizePropertyLength::set_width(ctx.widget_state.reborrow(), w));
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.child.update(ctx, updates);
            self.width.with_new(ctx.vars, |w| {
                SizePropertyLength::set_width(ctx.widget_state.reborrow(), w);
                ctx.updates.layout();
            });
        }

        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            let width = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.width.get().layout(ctx.metrics.for_x(), |ctx| ctx.constrains().fill()),
            );
            let width = ctx.constrains().x.clamp(width);
            let mut size = ctx.with_inline_constrains(
                wm,
                |_| None,
                |ctx, wm| ctx.with_constrains(|c| c.with_exact_x(width), |ctx| self.child.measure(ctx, wm)),
            );
            size.width = width;
            size
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let width = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.width.get().layout(ctx.metrics.for_x(), |ctx| ctx.constrains().fill()),
            );
            let width = ctx.constrains().x.clamp(width);
            let mut size = ctx.with_constrains(|c| c.with_exact_x(width), |ctx| self.child.layout(ctx, wl));
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
/// When set the widget is layout with exact size constrains, clamped by the contextual min/max.
/// Relative values are computed from the constrains maximum bounded height.
///
/// This property disables inline layout for the widget. This property sets the [`SIZE_PROPERTY_INFO_ID`] height.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// button! {
///     background_color = rgb(255, 0, 0);
///     height = 300;
///     child = text!("?x300 red");
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
#[property(SIZE)]
pub fn height(child: impl UiNode, height: impl IntoVar<Length>) -> impl UiNode {
    #[ui_node(struct HeightNode {
        child: impl UiNode,
        #[var] height: impl Var<Length>,
    })]
    impl UiNode for HeightNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.auto_subs(ctx);
            self.child.init(ctx);
            self.height.with(|h| SizePropertyLength::set_height(ctx.widget_state.reborrow(), h));
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.child.update(ctx, updates);
            self.height.with_new(ctx.vars, |h| {
                SizePropertyLength::set_height(ctx.widget_state.reborrow(), h);
                ctx.updates.layout();
            });
        }

        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            let height = ctx.with_constrains(
                |c| c.with_fill_vector(c.is_bounded()),
                |ctx| self.height.get().layout(ctx.metrics.for_y(), |ctx| ctx.constrains().fill()),
            );
            let height = ctx.constrains().y.clamp(height);
            let mut size = ctx.with_inline_constrains(
                wm,
                |_| None,
                |ctx, wm| ctx.with_constrains(|c| c.with_new_exact_y(height), |ctx| self.child.measure(ctx, wm)),
            );
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
#[property(BORDER, default(Length::Default))]
pub fn baseline(child: impl UiNode, baseline: impl IntoVar<Length>) -> impl UiNode {
    #[ui_node(struct BaselineNode {
        child: impl UiNode,
        #[var] baseline: impl Var<Length>,
    })]
    impl UiNode for BaselineNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.baseline.is_new(ctx) {
                ctx.updates.layout();
            }
            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            self.child.measure(ctx, wm)
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

/// Retain the widget's previous width if the new layout width is smaller.
/// The widget is layout using its previous width as the minimum width constrain.
///
/// This property disables inline layout for the widget.
#[property(SIZE, default(false))]
pub fn sticky_width(child: impl UiNode, sticky: impl IntoVar<bool>) -> impl UiNode {
    #[ui_node(struct StickyWidthNode {
        child: impl UiNode,
        #[var] sticky: impl Var<bool>,
    })]
    impl UiNode for StickyWidthNode {
        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            let min = ctx.widget_info.bounds.inner_size().width;
            let mut size = ctx.with_inline_constrains(
                wm,
                |_| None,
                |ctx, wm| ctx.with_constrains(|c| c.with_min_x(min), |ctx| self.child.measure(ctx, wm)),
            );
            size.width = size.width.max(min);
            size
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let min = ctx.widget_info.bounds.inner_size().width;
            let mut size = ctx.with_constrains(|c| c.with_min_x(min), |ctx| self.child.layout(ctx, wl));
            size.width = size.width.max(min);
            size
        }
    }
    StickyWidthNode {
        child,
        sticky: sticky.into_var(),
    }
}

/// Retain the widget's previous height if the new layout height is smaller.
/// The widget is layout using its previous height as the minimum height constrain.
///
/// This property disables inline layout for the widget.
#[property(SIZE, default(false))]
pub fn sticky_height(child: impl UiNode, sticky: impl IntoVar<bool>) -> impl UiNode {
    #[ui_node(struct StickyHeightNode {
        child: impl UiNode,
        #[var] sticky: impl Var<bool>,
    })]
    impl UiNode for StickyHeightNode {
        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            let min = ctx.widget_info.bounds.inner_size().height;
            let mut size = ctx.with_inline_constrains(
                wm,
                |_| None,
                |ctx, wm| ctx.with_constrains(|c| c.with_min_x(min), |ctx| self.child.measure(ctx, wm)),
            );
            size.height = size.height.max(min);
            size
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let min = ctx.widget_info.bounds.inner_size().height;
            let mut size = ctx.with_constrains(|c| c.with_min_x(min), |ctx| self.child.layout(ctx, wl));
            size.height = size.height.max(min);
            size
        }
    }
    StickyHeightNode {
        child,
        sticky: sticky.into_var(),
    }
}

/// Retain the widget's previous size if the new layout size is smaller.
/// The widget is layout using its previous size as the minimum size constrain.
///
/// This property disables inline layout for the widget.
#[property(SIZE, default(false))]
pub fn sticky_size(child: impl UiNode, sticky: impl IntoVar<bool>) -> impl UiNode {
    #[ui_node(struct StickyHeightNode {
        child: impl UiNode,
        #[var] sticky: impl Var<bool>,
    })]
    impl UiNode for StickyHeightNode {
        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            let min = ctx.widget_info.bounds.inner_size();
            let mut size = ctx.with_inline_constrains(
                wm,
                |_| None,
                |ctx, wm| ctx.with_constrains(|c| c.with_min_size(min), |ctx| self.child.measure(ctx, wm)),
            );
            size = size.max(min);
            size
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let min = ctx.widget_info.bounds.inner_size();
            let mut size = ctx.with_constrains(|c| c.with_min_size(min), |ctx| self.child.layout(ctx, wl));
            size = size.max(min);
            size
        }
    }
    StickyHeightNode {
        child,
        sticky: sticky.into_var(),
    }
}

/// Placement of a node inserted by the [`child_insert`] property.
///
/// [`child_insert`]: fn@child_insert
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChildInsertPlace {
    /// Insert node above the child.
    Above,
    /// Insert node to the right of child.
    Right,
    /// Insert node below the child.
    Below,
    /// Insert node to the left of child.
    Left,

    /// Insert node to the left of child in [`LayoutDirection::LTR`] contexts and to the right of child
    /// in [`LayoutDirection::RTL`] contexts.
    Start,
    /// Insert node to the right of child in [`LayoutDirection::LTR`] contexts and to the left of child
    /// in [`LayoutDirection::RTL`] contexts.
    End,
}
impl fmt::Debug for ChildInsertPlace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "ChildInsertPlace::")?;
        }
        match self {
            Self::Above => write!(f, "Above"),
            Self::Right => write!(f, "Right"),
            Self::Below => write!(f, "Below"),
            Self::Left => write!(f, "Left"),
            Self::Start => write!(f, "Start"),
            Self::End => write!(f, "End"),
        }
    }
}
impl ChildInsertPlace {
    /// Convert [`ChildInsertPlace::Start`] and [`ChildInsertPlace::End`] to the fixed place they represent in the `direction` context.
    pub fn resolve_direction(self, direction: LayoutDirection) -> Self {
        match self {
            Self::Start => match direction {
                LayoutDirection::LTR => Self::Left,
                LayoutDirection::RTL => Self::Right,
            },
            Self::End => match direction {
                LayoutDirection::LTR => Self::Right,
                LayoutDirection::RTL => Self::Left,
            },
            p => p,
        }
    }

    /// Inserted node is to the left or right of child.
    pub fn is_x_axis(self) -> bool {
        !matches!(self, Self::Above | Self::Below)
    }

    /// Inserted node is above or bellow the child node.
    pub fn is_y_axis(self) -> bool {
        matches!(self, Self::Above | Self::Below)
    }
}

/// Insert the `insert` node in the `place` relative to the widget's child.
///
/// This property disables inline layout for the widget.
#[property(CHILD, default(ChildInsertPlace::Start, NilUiNode, 0))]
pub fn child_insert(
    child: impl UiNode,
    place: impl IntoVar<ChildInsertPlace>,
    insert: impl UiNode,
    spacing: impl IntoVar<Length>,
) -> impl UiNode {
    #[ui_node(struct ChildInsertNode {
        children: impl UiNodeList,
        #[var] place: impl Var<ChildInsertPlace>,
        #[var] spacing: impl Var<Length>,
        offset_key: FrameValueKey<PxTransform>,
        offset_child: u8,
        offset: PxVector,
    })]
    impl UiNode for ChildInsertNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.children.update_all(ctx, updates, &mut ());
            if self.place.is_new(ctx) || self.spacing.is_new(ctx) {
                ctx.updates.layout();
            }
        }

        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            ctx.with_inline_constrains(
                wm,
                |_| None,
                |ctx, wm| {
                    if self.place.get().is_x_axis() {
                        let mut spacing = self.spacing.get().layout(ctx.metrics.for_x(), |_| Px(0));
                        let insert_size = self.children.with_node(1, |n| {
                            ctx.with_constrains(|c| c.with_new_min(Px(0), Px(0)).with_fill_x(false), |ctx| n.measure(ctx, wm))
                        });
                        if insert_size.width == Px(0) {
                            spacing = Px(0);
                        }
                        let child_size = self.children.with_node(0, |n| {
                            ctx.with_constrains(|c| c.with_less_x(insert_size.width + spacing), |ctx| n.measure(ctx, wm))
                        });

                        PxSize::new(
                            insert_size.width + spacing + child_size.width,
                            insert_size.height.max(child_size.height),
                        )
                    } else {
                        let mut spacing = self.spacing.get().layout(ctx.metrics.for_y(), |_| Px(0));
                        let insert_size = self.children.with_node(1, |n| {
                            ctx.with_constrains(|c| c.with_new_min(Px(0), Px(0)).with_fill_y(false), |ctx| n.measure(ctx, wm))
                        });
                        if insert_size.height == Px(0) {
                            spacing = Px(0);
                        }
                        let child_size = self.children.with_node(0, |n| {
                            ctx.with_constrains(|c| c.with_less_y(insert_size.height + spacing), |ctx| n.measure(ctx, wm))
                        });
                        if child_size.height == Px(0) {
                            spacing = Px(0);
                        }
                        PxSize::new(
                            insert_size.width.max(child_size.width),
                            insert_size.height + spacing + child_size.height,
                        )
                    }
                },
            )
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let place = self.place.get().resolve_direction(ctx.direction());

            match place {
                ChildInsertPlace::Left | ChildInsertPlace::Right => {
                    let spacing = self.spacing.get().layout(ctx.metrics.for_x(), |_| Px(0));

                    let mut constrains_y = ctx.constrains().y;
                    if constrains_y.fill_or_exact().is_none() {
                        // measure to find fill height
                        let mut ctx = ctx.as_measure();
                        let ctx = &mut ctx;
                        let mut wm = WidgetMeasure::new();
                        let wm = &mut wm;
                        let mut spacing = spacing;

                        let insert_size = self.children.with_node(1, |n| {
                            ctx.with_constrains(|c| c.with_new_min(Px(0), Px(0)).with_fill_x(false), |ctx| n.measure(ctx, wm))
                        });
                        if insert_size.width == Px(0) {
                            spacing = Px(0);
                        }
                        let child_size = self.children.with_node(0, |n| {
                            ctx.with_constrains(|c| c.with_less_x(insert_size.width + spacing), |ctx| n.measure(ctx, wm))
                        });

                        constrains_y = constrains_y.with_fill(true).with_max(child_size.height.max(insert_size.height));
                    }

                    let mut spacing = spacing;
                    let insert_size = self.children.with_node_mut(1, |n| {
                        ctx.with_constrains(
                            |mut c| {
                                c.y = constrains_y;
                                c.with_new_min(Px(0), Px(0)).with_fill_x(false)
                            },
                            |ctx| n.layout(ctx, wl),
                        )
                    });
                    if insert_size.width == Px(0) {
                        spacing = Px(0);
                    }
                    let child_size = self.children.with_node_mut(0, |n| {
                        ctx.with_constrains(
                            |mut c| {
                                c.y = constrains_y;
                                c.with_less_x(insert_size.width + spacing)
                            },
                            |ctx| n.layout(ctx, wl),
                        )
                    });
                    if child_size.width == Px(0) {
                        spacing = Px(0);
                    }

                    // position
                    let (child, offset) = match place {
                        ChildInsertPlace::Left => (0, insert_size.width + spacing),
                        ChildInsertPlace::Right => (1, child_size.width + spacing),
                        _ => unreachable!(),
                    };
                    let offset = PxVector::new(offset, Px(0));
                    if self.offset != offset || self.offset_child != child {
                        self.offset_child = child;
                        self.offset = offset;
                        ctx.updates.render_update();
                    }

                    PxSize::new(
                        insert_size.width + spacing + child_size.width,
                        insert_size.height.max(child_size.height),
                    )
                }
                ChildInsertPlace::Above | ChildInsertPlace::Below => {
                    let spacing = self.spacing.get().layout(ctx.metrics.for_y(), |_| Px(0));

                    let mut constrains_x = ctx.constrains().x;
                    if constrains_x.fill_or_exact().is_none() {
                        // measure fill width

                        let mut ctx = ctx.as_measure();
                        let mut wm = WidgetMeasure::new();
                        let ctx = &mut ctx;
                        let wm = &mut wm;
                        let mut spacing = spacing;

                        let insert_size = self.children.with_node(1, |n| {
                            ctx.with_constrains(|c| c.with_new_min(Px(0), Px(0)).with_fill_y(false), |ctx| n.measure(ctx, wm))
                        });
                        if insert_size.height == Px(0) {
                            spacing = Px(0);
                        }
                        let child_size = self.children.with_node(0, |n| {
                            ctx.with_constrains(|c| c.with_less_y(insert_size.height + spacing), |ctx| n.measure(ctx, wm))
                        });

                        constrains_x = constrains_x.with_fill(true).with_max(child_size.width.max(insert_size.width));
                    }

                    let mut spacing = spacing;
                    let insert_size = self.children.with_node_mut(1, |n| {
                        ctx.with_constrains(
                            |mut c| {
                                c.x = constrains_x;
                                c.with_new_min(Px(0), Px(0)).with_fill_y(false)
                            },
                            |ctx| n.layout(ctx, wl),
                        )
                    });
                    if insert_size.height == Px(0) {
                        spacing = Px(0);
                    }
                    let child_size = self.children.with_node_mut(0, |n| {
                        ctx.with_constrains(
                            |mut c| {
                                c.x = constrains_x;
                                c.with_less_y(insert_size.height + spacing)
                            },
                            |ctx| n.layout(ctx, wl),
                        )
                    });

                    // position
                    let (child, offset) = match place {
                        ChildInsertPlace::Above => (0, insert_size.height + spacing),
                        ChildInsertPlace::Below => (1, child_size.height + spacing),
                        _ => unreachable!(),
                    };
                    let offset = PxVector::new(Px(0), offset);
                    if self.offset != offset || self.offset_child != child {
                        self.offset_child = child;
                        self.offset = offset;
                        ctx.updates.render_update();
                    }

                    PxSize::new(
                        insert_size.width.max(child_size.width),
                        insert_size.height + spacing + child_size.height,
                    )
                }
                _ => {
                    unreachable!()
                }
            }
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.children.for_each(|i, child| {
                if i as u8 == self.offset_child {
                    frame.push_reference_frame(
                        self.offset_key.into(),
                        self.offset_key.bind(self.offset.into(), false),
                        true,
                        true,
                        |frame| {
                            child.render(ctx, frame);
                        },
                    );
                } else {
                    child.render(ctx, frame);
                }
                true
            })
        }

        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            self.children.for_each(|i, child| {
                if i as u8 == self.offset_child {
                    update.with_transform(self.offset_key.update(self.offset.into(), false), true, |update| {
                        child.render_update(ctx, update);
                    });
                } else {
                    child.render_update(ctx, update);
                }
                true
            })
        }
    }
    ChildInsertNode {
        children: ui_vec![child, insert],
        place: place.into_var(),
        spacing: spacing.into_var(),
        offset_key: FrameValueKey::new_unique(),
        offset_child: 0,
        offset: PxVector::zero(),
    }
}

/// Insert `insert` to the left of the widget's child.
///
/// This property disables inline layout for the widget. See [`child_insert`] for more details.
///
/// [`child_insert`]: fn@child_insert
#[property(CHILD, default(NilUiNode, 0))]
pub fn child_insert_left(child: impl UiNode, insert: impl UiNode, spacing: impl IntoVar<Length>) -> impl UiNode {
    child_insert(child, ChildInsertPlace::Left, insert, spacing)
}

/// Insert `insert` to the right of the widget's child.
///
/// This property disables inline layout for the widget. See [`child_insert`] for more details.
///
/// [`child_insert`]: fn@child_insert
#[property(CHILD, default(NilUiNode, 0))]
pub fn child_insert_right(child: impl UiNode, insert: impl UiNode, spacing: impl IntoVar<Length>) -> impl UiNode {
    child_insert(child, ChildInsertPlace::Right, insert, spacing)
}

/// Insert `insert` above the widget's child.
///
/// This property disables inline layout for the widget. See [`child_insert`] for more details.
///
/// [`child_insert`]: fn@child_insert
#[property(CHILD, default(NilUiNode, 0))]
pub fn child_insert_above(child: impl UiNode, insert: impl UiNode, spacing: impl IntoVar<Length>) -> impl UiNode {
    child_insert(child, ChildInsertPlace::Above, insert, spacing)
}

/// Insert `insert` below the widget's child.
///
/// This property disables inline layout for the widget. See [`child_insert`] for more details.
///
/// [`child_insert`]: fn@child_insert
#[property(CHILD, default(NilUiNode, 0))]
pub fn child_insert_below(child: impl UiNode, insert: impl UiNode, spacing: impl IntoVar<Length>) -> impl UiNode {
    child_insert(child, ChildInsertPlace::Below, insert, spacing)
}

/// Insert `insert` to the left of the widget's child in LTR contexts or to the right of the widget's child in RTL contexts.
///
/// This property disables inline layout for the widget. See [`child_insert`] for more details.
///
/// [`child_insert`]: fn@child_insert
#[property(CHILD, default(NilUiNode, 0))]
pub fn child_insert_start(child: impl UiNode, insert: impl UiNode, spacing: impl IntoVar<Length>) -> impl UiNode {
    child_insert(child, ChildInsertPlace::Start, insert, spacing)
}

/// Insert `insert` to the right of the widget's child in LTR contexts or to the right of the widget's child in RTL contexts.
///
/// This property disables inline layout for the widget. See [`child_insert`] for more details.
///
/// [`child_insert`]: fn@child_insert
#[property(CHILD, default(NilUiNode, 0))]
pub fn child_insert_end(child: impl UiNode, insert: impl UiNode, spacing: impl IntoVar<Length>) -> impl UiNode {
    child_insert(child, ChildInsertPlace::End, insert, spacing)
}

/// Represents the size property value set on a widget.
///
/// Properties like [`size`], [`width`] and [`height`] set the [`SIZE_PROPERTY_INFO_ID`]
/// metadata in the widget state. Panels can use this info to implement [`Length::Leftover`] support.
///  
/// [`size`]: fn@size
/// [`width`]: fn@width
/// [`height`]: fn@height
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SizePropertyLength {
    /// Evaluates to [`PxConstrains2d::fill_size`] when measured, can serve as a request for *size-to-fit*.
    ///
    /// The `grid!` widget uses this to fit the column and row widgets to *their* cells, as they don't
    /// logically own the cells, this fit needs to be computed by the parent panel.
    Default,
    /// The [`Length::Leftover`] value. Evaluates to the [`LayoutMetrics::leftover`] value when measured, if
    /// a leftover value is not provided evaluates like a [`Length::Relative`].
    ///
    /// The *leftover* length needs to be computed by the parent panel, as it depends on the length of the sibling widgets,
    /// not just the panel constrains. Panels that support this, compute the value for each widget and measure/layout each using
    /// [`LayoutContext::with_leftover`] to inject the computed value.
    ///
    /// [`LayoutContext::with_leftover`]: crate::core::context::LayoutContext::with_leftover
    Leftover(Factor),
    /// Any of the other [`Length`] kinds. All contextual metrics needed to compute these values is already available
    /// in the [`LayoutMetrics`], panels that support [`Length::Leftover`] can layout this widget first to compute the
    /// leftover length.
    Exact,
}
impl Default for SizePropertyLength {
    fn default() -> Self {
        SizePropertyLength::Default
    }
}
impl SizePropertyLength {
    /// Set the width state.
    pub fn set_width(mut state: StateMapMut<state_map::Widget>, width: &Length) {
        let width = width.into();
        match state.entry(&SIZE_PROPERTY_INFO_ID) {
            state_map::StateMapEntry::Occupied(mut e) => e.get_mut().width = width,
            state_map::StateMapEntry::Vacant(e) => {
                e.insert(euclid::size2(width, SizePropertyLength::Default));
            }
        }
    }

    /// Set the height state.
    pub fn set_height(mut state: StateMapMut<state_map::Widget>, height: &Length) {
        let height = height.into();
        match state.entry(&SIZE_PROPERTY_INFO_ID) {
            state_map::StateMapEntry::Occupied(mut e) => e.get_mut().height = height,
            state_map::StateMapEntry::Vacant(e) => {
                e.insert(euclid::size2(SizePropertyLength::Default, height));
            }
        }
    }

    /// Set the size state.
    pub fn set(mut state: StateMapMut<state_map::Widget>, size: &Size) {
        let size = euclid::size2((&size.width).into(), (&size.height).into());
        state.set(&SIZE_PROPERTY_INFO_ID, size);
    }

    /// Get the size set in the state.
    pub fn get(state: StateMapRef<state_map::Widget>) -> euclid::Size2D<SizePropertyLength, ()> {
        state.get(&SIZE_PROPERTY_INFO_ID).cloned().unwrap_or_default()
    }

    /// Get the size set in the widget state.
    pub fn get_wgt(wgt: &impl UiNode) -> euclid::Size2D<SizePropertyLength, ()> {
        wgt.with_context(|ctx| Self::get(ctx.widget_state)).unwrap_or_default()
    }
}
impl From<&Length> for SizePropertyLength {
    fn from(value: &Length) -> Self {
        match value {
            Length::Default => SizePropertyLength::Default,
            Length::Leftover(f) => SizePropertyLength::Leftover(*f),
            _ => SizePropertyLength::Exact,
        }
    }
}

/// Id for widget state set by properties that directly enforce a widget size.
///
/// See [`SizePropertyLength`] for more details.
pub static SIZE_PROPERTY_INFO_ID: StaticStateId<euclid::Size2D<SizePropertyLength, ()>> = StaticStateId::new_unique();
