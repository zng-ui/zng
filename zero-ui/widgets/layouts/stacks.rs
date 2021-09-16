use crate::prelude::new_widget::*;

/// Horizontal stack layout.
///
/// # Example
///
/// ```
/// # use zero_ui::prelude::*;
/// let text = v_stack! {
///     spacing = 5.0;
///     items = widgets![
///         text("1. Hello"),
///         text("2. World"),
///     ];
/// };
/// ```
///
/// ## `h_stack()`
///
/// If you only want to set the `items` property you can use the [`h_stack`](function@h_stack) shortcut function.
#[widget($crate::widgets::layouts::h_stack)]
pub mod h_stack {

    use super::*;

    properties! {
        child {
            /// Widget items.
            #[allowed_in_when = false]
            items(impl WidgetList) = widgets![];

            /// Space in-between items.
            spacing(impl IntoVar<Length>) = 0.0;

            /// Margin around all items together.
            margin as padding;

            /// Items alignment.
            ///
            /// Horizontal alignment applies to all items together, vertical alignment applies to each
            /// item individually. The default is [`LEFT_FILL`].
            ///
            /// [`LEFT_FILL`]: Alignment::LEFT_FILL
            items_align(impl IntoVar<Alignment>) = Alignment::LEFT_FILL;
        }
    }

    #[inline]
    fn new_child(items: impl WidgetList, spacing: impl IntoVar<Length>, items_align: impl IntoVar<Alignment>) -> impl UiNode {
        HStackNode {
            rectangles: vec![euclid::Rect::zero(); items.len()].into_boxed_slice(),
            items_width: Px(0),
            visible_count: 0,
            children: items,
            spacing: spacing.into_var(),
            align: items_align.into_var(),
        }
    }

    struct HStackNode<C, S, A> {
        children: C,
        rectangles: Box<[PxRect]>,
        items_width: Px,
        visible_count: u32,

        spacing: S,
        align: A,
    }
    #[impl_ui_node(children)]
    impl<C: WidgetList, S: Var<Length>, A: Var<Alignment>> UiNode for HStackNode<C, S, A> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            self.children.update_all(ctx);

            if self.spacing.is_new(ctx) || self.align.is_new(ctx) {
                ctx.updates.layout();
            }
        }

        fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
            let mut ds = PxSize::zero();
            self.visible_count = 0;

            let rectangles = &mut self.rectangles;
            let visible_count = &mut self.visible_count;
            self.children.measure_all(
                ctx,
                |_, _| available_size,
                |i, s, _| {
                    rectangles[i].size = s;
                    ds.height = ds.height.max(s.height);
                    if s.width > Px(0) {
                        ds.width += s.width;
                        *visible_count += 1;
                    }
                },
            );

            let spacing = self.spacing.get(ctx.vars).to_layout(ctx, available_size.width);

            ds.width += Px(self.visible_count.saturating_sub(1) as i32) * spacing;
            self.items_width = ds.width;

            if self.align.get(ctx).fill_width() {
                ds.max(available_size.to_px())
            } else {
                ds
            }
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, final_size: PxSize) {
            let spacing = self.spacing.get(ctx.vars).to_layout(ctx, AvailablePx::Finite(final_size.width));
            let align = self.align.copy(ctx);
            let fill_width = align.fill_width();

            // if `fill_width` and there is space to fill we give the extra width divided equally
            // for each visible item. The fill alignment is usually only set for the height so this is a corner case.
            let extra_width = if fill_width && self.items_width < final_size.width {
                let vis_count = Px(self.visible_count.saturating_sub(1) as i32);
                (final_size.width - vis_count * spacing) / vis_count
            } else {
                Px(0)
            };

            // offset for each item to apply the vertical alignment.
            let mut x_offset = if fill_width {
                Px(0)
            } else {
                let diff = final_size.width - self.items_width;
                diff * align.x.0
            };

            let rectangles = &mut self.rectangles;
            let fill_height = align.fill_height();

            self.children.arrange_all(ctx, |i, _| {
                let r = &mut rectangles[i];

                r.size.width += extra_width;
                r.origin.x = x_offset;

                x_offset += r.size.width + spacing;

                if fill_height {
                    r.size.height = final_size.height;
                    r.origin.y = Px(0);
                } else {
                    r.size.height = r.size.height.min(final_size.height);
                    r.origin.y = (final_size.height - r.size.height) * align.y.0;
                };

                r.size
            });
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.children.render_all(|i| self.rectangles[i].origin, ctx, frame);
        }
    }
}

/// Vertical stack layout.
///
/// # Example
///
/// ```
/// # use zero_ui::prelude::*;
/// let text = h_stack! {
///     spacing = 5.0;
///     items = widgets![
///         text("Hello"),
///         text("World"),
///     ];
/// };
/// ```
/// ## `v_stack()`
///
/// If you only want to set the `items` property you can use the [`v_stack`](function@v_stack) shortcut function.
#[widget($crate::widgets::layouts::v_stack)]
pub mod v_stack {
    use super::*;

    properties! {
        child {
            /// Space in-between items.
            spacing(impl IntoVar<Length>) = 0.0;
            /// Widget items.
            #[allowed_in_when = false]
            items(impl WidgetList) = widgets![];
            /// Items margin.
            margin as padding;

            /// Items alignment.
            ///
            /// Vertical alignment applies to all items together, horizontal alignment applies to each
            /// item individually. The default is [`FILL_TOP`].
            ///
            /// [`FILL_TOP`]: Alignment::FILL_TOP
            items_align(impl IntoVar<Alignment>) = Alignment::FILL_TOP;
        }
    }

    #[inline]
    fn new_child(items: impl WidgetList, spacing: impl IntoVar<Length>, items_align: impl IntoVar<Alignment>) -> impl UiNode {
        VStackNode {
            rectangles: vec![euclid::Rect::zero(); items.len()].into_boxed_slice(),
            items_height: Px(0),
            visible_count: 0,
            children: items,
            spacing: spacing.into_var(),
            align: items_align.into_var(),
        }
    }

    struct VStackNode<C, S, A> {
        children: C,
        rectangles: Box<[PxRect]>,
        items_height: Px,
        visible_count: usize,

        spacing: S,
        align: A,
    }
    #[impl_ui_node(children)]
    impl<C: WidgetList, S: Var<Length>, A: Var<Alignment>> UiNode for VStackNode<C, S, A> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            self.children.update_all(ctx);

            if self.spacing.is_new(ctx) || self.align.is_new(ctx) {
                ctx.updates.layout();
            }
        }

        fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
            let mut ds = PxSize::zero();
            self.visible_count = 0;

            let rectangles = &mut self.rectangles;
            let visible_count = &mut self.visible_count;
            self.children.measure_all(
                ctx,
                |_, _| available_size,
                |i, s, _| {
                    rectangles[i].size = s;
                    ds.width = ds.width.max(s.width);
                    if s.height > Px(0) {
                        ds.height += s.height;
                        *visible_count += 1;
                    }
                },
            );

            let spacing = self.spacing.get(ctx.vars).to_layout(ctx, available_size.height);

            ds.height += Px(self.visible_count.saturating_sub(1) as i32) * spacing;
            self.items_height = ds.height;

            if self.align.get(ctx).fill_height() {
                ds.max(available_size.to_px())
            } else {
                ds
            }
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, final_size: PxSize) {
            let spacing = self.spacing.get(ctx.vars).to_layout(ctx, AvailablePx::Finite(final_size.height));
            let align = self.align.copy(ctx);
            let fill_height = align.fill_height();

            // if `fill_height` and there is space to fill we give the extra height divided equally
            // for each visible item. The fill alignment is usually only set for the width so this is a corner case.
            let extra_height = if fill_height && self.items_height < final_size.height {
                let vis_count = Px(self.visible_count.saturating_sub(1) as i32);
                (final_size.height - vis_count * spacing) / vis_count
            } else {
                Px(0)
            };

            // offset for each item to apply the vertical alignment.
            let mut y_offset = if fill_height {
                Px(0)
            } else {
                let diff = final_size.height - self.items_height;
                diff * align.y.0
            };

            let rectangles = &mut self.rectangles;
            let fill_width = align.fill_width();

            self.children.arrange_all(ctx, |i, _| {
                let r = &mut rectangles[i];

                r.size.height += extra_height;
                r.origin.y = y_offset;

                y_offset += r.size.height + spacing;

                if fill_width {
                    r.size.width = final_size.width;
                    r.origin.x = Px(0);
                } else {
                    r.size.width = r.size.width.min(final_size.width);
                    r.origin.x = (final_size.width - r.size.width) * align.x.0;
                };

                r.size
            });
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.children.render_all(|i| self.rectangles[i].origin, ctx, frame);
        }
    }
}

/// Basic horizontal stack layout.
///
/// # Example
///
/// ```
/// # use zero_ui::prelude::*;
/// let text = h_stack(widgets![
///     text("Hello "),
///     text("World"),
/// ]);
/// ```
///
/// # `h_stack!`
///
/// This function is just a shortcut for [`h_stack!`](module@v_stack). Use the full widget
/// to better configure the horizontal stack widget.
pub fn h_stack(items: impl WidgetList) -> impl Widget {
    h_stack! {
        items;
    }
}

/// Basic vertical stack layout.
///
/// # Example
///
/// ```
/// # use zero_ui::prelude::*;
/// let text = v_stack(widgets![
///     text("1. Hello"),
///     text("2. World"),
/// ]);
/// ```
///
/// # `v_stack!`
///
/// This function is just a shortcut for [`v_stack!`](module@v_stack). Use the full widget
/// to better configure the vertical stack widget.
pub fn v_stack(items: impl WidgetList) -> impl Widget {
    v_stack! {
        items;
    }
}

/// Layering stack layout.
///
/// # Example
///
/// ```
/// # use zero_ui::prelude::*;
/// let text = z_stack! {
///     padding = 5.0;
///     items = nodes![
///         text("under"),
///         text("over"),
///     ];
/// };
/// ```
///
/// ## `z_stack()`
///
/// If you only want to set the `items` property you can use the [`z_stack`](function@z_stack) shortcut function.
#[widget($crate::widgets::layouts::z_stack)]
pub mod z_stack {
    use super::*;

    properties! {
        child {
            /// UiNode items.
            #[allowed_in_when = false]
            items(impl UiNodeList) = nodes![];
            /// Items margin.
            margin as padding;

            /// Items alignment.
            ///
            /// Alignment applies to each item individually. The default is [`FILL`].
            ///
            /// [`FILL`]: Alignment::FILL
            items_align(impl IntoVar<Alignment>) = Alignment::FILL;
        }
    }

    #[inline]
    fn new_child(items: impl UiNodeList, items_align: impl IntoVar<Alignment>) -> impl UiNode {
        ZStackNode {
            rectangles: vec![euclid::Rect::zero(); items.len()].into_boxed_slice(),
            children: items,
            align: items_align.into_var(),
        }
    }

    struct ZStackNode<C, A> {
        children: C,
        rectangles: Box<[PxRect]>,
        align: A,
    }
    #[impl_ui_node(children)]
    impl<I: UiNodeList, A: Var<Alignment>> UiNode for ZStackNode<I, A> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.align.is_new(ctx) {
                ctx.updates.layout()
            }
            self.children.update_all(ctx);
        }

        fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
            let rectangles = &mut self.rectangles;
            let mut ds = PxSize::zero();
            self.children.measure_all(
                ctx,
                |_, _| available_size,
                |i, s, _| {
                    ds = ds.max(s);
                    rectangles[i].size = s;
                },
            );
            ds
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, final_size: PxSize) {
            let align = self.align.copy(ctx);

            let rectangles = &mut self.rectangles;
            self.children.arrange_all(ctx, |i, _| {
                rectangles[i] = align.solve(rectangles[i].size, final_size);
                rectangles[i].size
            });
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.children.render_all(|i| self.rectangles[i].origin, ctx, frame);
        }
    }
}

/// Basic layering stack layout.
///
/// # Example
///
/// ```
/// # use zero_ui::prelude::*;
/// let text = z_stack(nodes![
///     text("under"),
///     text("over"),
/// ]);
/// ```
///
/// # `z_stack!`
///
/// This function is just a shortcut for [`z_stack!`](module@z_stack). Use the full widget
/// to better configure the layering stack widget.
pub fn z_stack(items: impl UiNodeList) -> impl Widget {
    z_stack! { items; }
}
