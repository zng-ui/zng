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
            .snap_to(ctx.pixel_grid());
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
