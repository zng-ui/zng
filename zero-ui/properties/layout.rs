//! Properties that affect the widget layout only.

use zero_ui::prelude::new_property::*;

/// Margin space around the widget.
///
/// # Example
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
#[property(outer)]
pub fn margin(child: impl UiNode, margin: impl IntoVar<SideOffsets>) -> impl UiNode {
    struct MarginNode<T: UiNode, M: VarLocal<SideOffsets>> {
        child: T,
        margin: M,
        size_increment: LayoutSize,
        child_rect: LayoutRect,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, M: VarLocal<SideOffsets>> UiNode for MarginNode<T, M> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.margin.init_local(ctx.vars);
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.margin.update_local(ctx.vars).is_some() {
                ctx.updates.layout();
            }
            self.child.update(ctx);
        }

        fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
            let margin = self.margin.get_local().to_layout(available_size, ctx);
            self.size_increment = LayoutSize::new(margin.left + margin.right, margin.top + margin.bottom);
            self.child_rect.origin = LayoutPoint::new(margin.left, margin.top);
            self.child.measure(available_size - self.size_increment, ctx) + self.size_increment
        }

        fn arrange(&mut self, mut final_size: LayoutSize, ctx: &mut LayoutContext) {
            final_size -= self.size_increment;
            self.child_rect.size = final_size;
            self.child.arrange(final_size, ctx);
        }

        fn render(&self, frame: &mut FrameBuilder) {
            frame.push_reference_frame(self.child_rect.origin, |frame| self.child.render(frame));
        }
    }
    MarginNode {
        child,
        margin: margin.into_local(),
        size_increment: LayoutSize::zero(),
        child_rect: LayoutRect::zero(),
    }
}

/// Aligns the widget within the available space.
///
/// # Example
///
/// ```
/// use zero_ui::prelude::*;
///
/// container! {
///     content = button! {
///         align = Alignment::TOP;
///         content = text("Click Me!")
///     };
/// }
/// # ;
/// ```
///
/// In the example the button is positioned at the top-center of the container. See [`Alignment`] for
/// more details.
#[property(outer)]
pub fn align(child: impl UiNode, alignment: impl IntoVar<Alignment>) -> impl UiNode {
    struct AlignNode<T, A> {
        child: T,
        alignment: A,
        child_rect: LayoutRect,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, A: VarLocal<Alignment>> UiNode for AlignNode<T, A> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.alignment.init_local(ctx.vars);
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.alignment.update_local(ctx.vars).is_some() {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
            self.child_rect.size = self.child.measure(available_size, ctx);
            self.child_rect.size
        }

        fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
            self.child_rect.size = final_size.min(self.child_rect.size);
            self.child.arrange(self.child_rect.size, ctx);

            let alignment = self.alignment.get_local();

            self.child_rect.origin = LayoutPoint::new(
                (final_size.width - self.child_rect.size.width) * alignment.x.0,
                (final_size.height - self.child_rect.size.height) * alignment.y.0,
            )
            .snap_to(*ctx.pixel_grid);
        }

        fn render(&self, frame: &mut FrameBuilder) {
            frame.push_reference_frame(self.child_rect.origin, |frame| self.child.render(frame));
        }
    }

    AlignNode {
        child,
        alignment: alignment.into_local(),
        child_rect: LayoutRect::zero(),
    }
}

/// Widget left-top offset.
///
/// # Example
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
#[property(outer)]
pub fn position(child: impl UiNode, position: impl IntoVar<Point>) -> impl UiNode {
    struct PositionNode<T: UiNode, P: VarLocal<Point>> {
        child: T,
        position: P,
        final_position: LayoutPoint,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, P: VarLocal<Point>> UiNode for PositionNode<T, P> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.position.init_local(ctx.vars);
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.position.update_local(ctx.vars).is_some() {
                ctx.updates.layout();
            }
            self.child.update(ctx);
        }

        fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
            self.child.arrange(final_size, ctx);
            self.final_position = self.position.get_local().to_layout(final_size, ctx);
        }

        fn render(&self, frame: &mut FrameBuilder) {
            frame.push_reference_frame(self.final_position, |frame| self.child.render(frame));
        }
    }
    PositionNode {
        child,
        position: position.into_local(),
        final_position: LayoutPoint::zero(),
    }
}

/// Left offset.
///
/// # Example
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
#[property(outer)]
pub fn x(child: impl UiNode, x: impl IntoVar<Length>) -> impl UiNode {
    struct XNode<T: UiNode, X: VarLocal<Length>> {
        child: T,
        x: X,
        final_x: LayoutLength,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, X: VarLocal<Length>> UiNode for XNode<T, X> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.x.init_local(ctx.vars);
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.x.update_local(ctx.vars).is_some() {
                ctx.updates.layout();
            }
            self.child.update(ctx);
        }

        fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
            self.child.arrange(final_size, ctx);
            self.final_x = self.x.get_local().to_layout(LayoutLength::new(final_size.width), ctx);
        }

        fn render(&self, frame: &mut FrameBuilder) {
            frame.push_reference_frame(LayoutPoint::new(self.final_x.0, 0.0), |frame| self.child.render(frame));
        }
    }
    XNode {
        child,
        x: x.into_local(),
        final_x: LayoutLength::new(0.0),
    }
}

/// Top offset.
///
/// # Example
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
#[property(outer)]
pub fn y(child: impl UiNode, y: impl IntoVar<Length>) -> impl UiNode {
    struct YNode<T: UiNode, Y: VarLocal<Length>> {
        child: T,
        y: Y,
        final_y: LayoutLength,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, Y: VarLocal<Length>> UiNode for YNode<T, Y> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.y.init_local(ctx.vars);
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.y.update_local(ctx.vars).is_some() {
                ctx.updates.layout();
            }
            self.child.update(ctx);
        }

        fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
            self.child.arrange(final_size, ctx);
            self.final_y = self.y.get_local().to_layout(LayoutLength::new(final_size.height), ctx);
        }

        fn render(&self, frame: &mut FrameBuilder) {
            frame.push_reference_frame(LayoutPoint::new(0.0, self.final_y.0), |frame| self.child.render(frame));
        }
    }
    YNode {
        child,
        y: y.into_local(),
        final_y: LayoutLength::new(0.0),
    }
}

/// Minimum size of the widget.
///
/// The widget size can be larger then this but not smaller.
///
/// # Example
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
#[property(size)]
pub fn min_size(child: impl UiNode, min_size: impl IntoVar<Size>) -> impl UiNode {
    struct MinSizeNode<T: UiNode, S: VarLocal<Size>> {
        child: T,
        min_size: S,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, S: VarLocal<Size>> UiNode for MinSizeNode<T, S> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.min_size.init_local(ctx.vars);
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.min_size.update_local(ctx.vars).is_some() {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
            let min_size = self.min_size.get_local().to_layout(available_size, ctx);
            let desired_size = self
                .child
                .measure(replace_layout_any_size(min_size, available_size).max(available_size), ctx);
            desired_size.max(replace_layout_any_size(min_size, desired_size))
        }

        fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
            let min_size = replace_layout_any_size(self.min_size.get_local().to_layout(final_size, ctx), final_size);
            self.child.arrange(min_size.max(final_size), ctx);
        }
    }
    MinSizeNode {
        child,
        min_size: min_size.into_local(),
    }
}

/// Minimum width of the widget.
///
/// The widget width can be larger then this but not smaller.
///
/// # Example
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
#[property(size)]
pub fn min_width(child: impl UiNode, min_width: impl IntoVar<Length>) -> impl UiNode {
    struct MinWidthNode<T: UiNode, W: VarLocal<Length>> {
        child: T,
        min_width: W,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, W: VarLocal<Length>> UiNode for MinWidthNode<T, W> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.min_width.init_local(ctx.vars);
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.min_width.update_local(ctx.vars).is_some() {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn measure(&mut self, mut available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
            let min_width = self
                .min_width
                .get_local()
                .to_layout(LayoutLength::new(available_size.width), ctx)
                .get();

            if !is_layout_any_size(min_width) {
                available_size.width = min_width.max(available_size.width);
                let mut desired_size = self.child.measure(available_size, ctx);
                desired_size.width = desired_size.width.max(min_width);
                desired_size
            } else {
                self.child.measure(available_size, ctx)
            }
        }

        fn arrange(&mut self, mut final_size: LayoutSize, ctx: &mut LayoutContext) {
            let min_width = self.min_width.get_local().to_layout(LayoutLength::new(final_size.width), ctx).get();
            if !is_layout_any_size(min_width) {
                final_size.width = min_width.max(final_size.width);
            }
            self.child.arrange(final_size, ctx);
        }
    }
    MinWidthNode {
        child,
        min_width: min_width.into_local(),
    }
}

/// Minimum height of the widget.
///
/// The widget height can be larger then this but not smaller.
///
/// # Example
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
#[property(size)]
pub fn min_height(child: impl UiNode, min_height: impl IntoVar<Length>) -> impl UiNode {
    struct MinHeightNode<T: UiNode, H: VarLocal<Length>> {
        child: T,
        min_height: H,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, H: VarLocal<Length>> UiNode for MinHeightNode<T, H> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.min_height.init_local(ctx.vars);
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.min_height.update_local(ctx.vars).is_some() {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn measure(&mut self, mut available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
            let min_height = self
                .min_height
                .get_local()
                .to_layout(LayoutLength::new(available_size.height), ctx)
                .get();
            if !is_layout_any_size(min_height) {
                available_size.height = min_height.max(available_size.height);
                let mut desired_size = self.child.measure(available_size, ctx);
                desired_size.height = desired_size.height.max(min_height);
                desired_size
            } else {
                self.child.measure(available_size, ctx)
            }
        }

        fn arrange(&mut self, mut final_size: LayoutSize, ctx: &mut LayoutContext) {
            let min_height = self
                .min_height
                .get_local()
                .to_layout(LayoutLength::new(final_size.height), ctx)
                .get();
            if !is_layout_any_size(min_height) {
                final_size.height = min_height.max(final_size.height);
            }
            self.child.arrange(final_size, ctx);
        }
    }
    MinHeightNode {
        child,
        min_height: min_height.into_local(),
    }
}

/// Maximum size of the widget.
///
/// The widget size can be smaller then this but not larger.
///
/// # Example
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
    struct MaxSizeNode<T: UiNode, S: VarLocal<Size>> {
        child: T,
        max_size: S,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, S: VarLocal<Size>> UiNode for MaxSizeNode<T, S> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.max_size.init_local(ctx.vars);
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.max_size.update_local(ctx.vars).is_some() {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
            let max_size = self.max_size.get_local().to_layout(available_size, ctx);
            self.child.measure(max_size.min(available_size), ctx).min(max_size)
        }

        fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
            self.child
                .arrange(self.max_size.get_local().to_layout(final_size, ctx).min(final_size), ctx);
        }
    }
    MaxSizeNode {
        child,
        max_size: max_size.into_local(),
    }
}

/// Maximum width of the widget.
///
/// The widget width can be smaller then this but not larger.
///
/// # Example
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
    struct MaxWidthNode<T: UiNode, W: VarLocal<Length>> {
        child: T,
        max_width: W,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, W: VarLocal<Length>> UiNode for MaxWidthNode<T, W> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.max_width.init_local(ctx.vars);
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.max_width.update_local(ctx.vars).is_some() {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn measure(&mut self, mut available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
            let max_width = self
                .max_width
                .get_local()
                .to_layout(LayoutLength::new(available_size.width), ctx)
                .get();

            // if max_width is LAYOUT_ANY_SIZE this still works because every other value
            // is smaller the positive infinity.
            available_size.width = available_size.width.min(max_width);

            let mut desired_size = self.child.measure(available_size, ctx);
            desired_size.width = desired_size.width.min(max_width);
            desired_size
        }

        fn arrange(&mut self, mut final_size: LayoutSize, ctx: &mut LayoutContext) {
            final_size.width = self
                .max_width
                .get_local()
                .to_layout(LayoutLength::new(final_size.width), ctx)
                .get()
                .min(final_size.width);
            self.child.arrange(final_size, ctx);
        }
    }
    MaxWidthNode {
        child,
        max_width: max_width.into_local(),
    }
}

/// Maximum height of the widget.
///
/// The widget height can be smaller then this but not larger.
///
/// # Example
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
    struct MaxHeightNode<T: UiNode, H: VarLocal<Length>> {
        child: T,
        max_height: H,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, H: VarLocal<Length>> UiNode for MaxHeightNode<T, H> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.max_height.init_local(ctx.vars);
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.max_height.update_local(ctx.vars).is_some() {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn measure(&mut self, mut available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
            let max_height = self
                .max_height
                .get_local()
                .to_layout(LayoutLength::new(available_size.height), ctx)
                .get();

            // if max_height is LAYOUT_ANY_SIZE this still works because every other value
            // is smaller the positive infinity.
            available_size.height = available_size.height.min(max_height);

            let mut desired_size = self.child.measure(available_size, ctx);
            desired_size.height = desired_size.height.min(max_height);
            desired_size
        }

        fn arrange(&mut self, mut final_size: LayoutSize, ctx: &mut LayoutContext) {
            final_size.height = self
                .max_height
                .get_local()
                .to_layout(LayoutLength::new(final_size.height), ctx)
                .get()
                .min(final_size.height);
            self.child.arrange(final_size, ctx);
        }
    }
    MaxHeightNode {
        child,
        max_height: max_height.into_local(),
    }
}

/// Manually sets the size of the widget.
///
/// When set the widget is sized with the given value, independent of the parent available size.
///
/// If the width or height is set to [positive infinity](is_layout_any_size) then the normal layout measuring happens.
///
/// # Example
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
    struct SizeNode<T: UiNode, S: VarLocal<Size>> {
        child: T,
        size: S,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, S: VarLocal<Size>> UiNode for SizeNode<T, S> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.size.init_local(ctx.vars);
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.size.update_local(ctx.vars).is_some() {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
            let size = self.size.get_local().to_layout(available_size, ctx);
            let desired_size = self.child.measure(replace_layout_any_size(size, available_size), ctx);
            replace_layout_any_size(size, desired_size)
        }

        fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
            let size = replace_layout_any_size(self.size.get_local().to_layout(final_size, ctx), final_size);
            self.child.arrange(size, ctx);
        }
    }
    SizeNode {
        child,
        size: size.into_local(),
    }
}

/// Exact width of the widget.
///
/// # Example
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
    struct WidthNode<T: UiNode, W: VarLocal<Length>> {
        child: T,
        width: W,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, W: VarLocal<Length>> UiNode for WidthNode<T, W> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.width.init_local(ctx.vars);
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.width.update_local(ctx.vars).is_some() {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn measure(&mut self, mut available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
            let width = self.width.get_local().to_layout(LayoutLength::new(available_size.width), ctx).get();
            if !is_layout_any_size(width) {
                available_size.width = width;
                let mut desired_size = self.child.measure(available_size, ctx);
                desired_size.width = width;
                desired_size
            } else {
                self.child.measure(available_size, ctx)
            }
        }

        fn arrange(&mut self, mut final_size: LayoutSize, ctx: &mut LayoutContext) {
            let width = self.width.get_local().to_layout(LayoutLength::new(final_size.width), ctx).get();
            if !is_layout_any_size(width) {
                final_size.width = width;
            }
            self.child.arrange(final_size, ctx)
        }
    }
    WidthNode {
        child,
        width: width.into_local(),
    }
}

/// Exact height of the widget.
///
/// # Example
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
    struct HeightNode<T: UiNode, H: VarLocal<Length>> {
        child: T,
        height: H,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, H: VarLocal<Length>> UiNode for HeightNode<T, H> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.height.init_local(ctx.vars);
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.height.update_local(ctx.vars).is_some() {
                ctx.updates.layout();
            }

            self.child.update(ctx);
        }

        fn measure(&mut self, mut available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
            let height = self
                .height
                .get_local()
                .to_layout(LayoutLength::new(available_size.height), ctx)
                .get();
            if !is_layout_any_size(height) {
                available_size.height = height;
                let mut desired_size = self.child.measure(available_size, ctx);
                desired_size.height = height;
                desired_size
            } else {
                self.child.measure(available_size, ctx)
            }
        }

        fn arrange(&mut self, mut final_size: LayoutSize, ctx: &mut LayoutContext) {
            let height = self.height.get_local().to_layout(LayoutLength::new(final_size.height), ctx).get();
            if !is_layout_any_size(height) {
                final_size.height = height;
            }
            self.child.arrange(final_size, ctx)
        }
    }
    HeightNode {
        child,
        height: height.into_local(),
    }
}

fn replace_layout_any_size(mut size: LayoutSize, replacement_size: LayoutSize) -> LayoutSize {
    if is_layout_any_size(size.width) {
        size.width = replacement_size.width;
    }
    if is_layout_any_size(size.height) {
        size.height = replacement_size.height;
    }

    size
}
