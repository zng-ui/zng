use crate::prelude::new_widget::*;

/// Horizontal stack layout.
///
/// # Z-Index
///
/// By default the widgets are layout without overlap, but you can use properties like [`transform`] to cause
/// an widget overlap, in this case the widget will be rendered above its previous sibling and below its next sibling,
/// you can change this by setting the [`z_index`] property in the item widget.
///
/// # Examples
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
/// # `h_stack()`
///
/// If you only want to set the `items` property you can use the [`h_stack`] shortcut function.
///
/// # `stack_nodes`
///
/// If you only want to create an overlaying effect composed of multiple nodes you can use the [`stack_nodes`] function.
///
/// [`transform`]: fn@transform
/// [`z_index`]: fn@z_index
/// [`h_stack`]: fn@h_stack
/// [`stack_nodes`]: fn@stack_nodes
#[widget($crate::widgets::layouts::h_stack)]
pub mod h_stack {
    use super::*;

    properties! {
        /// Widget items.
        #[allowed_in_when = false]
        items(impl WidgetList) = widgets![];

        /// Space in-between items.
        spacing(impl IntoVar<Length>) = 0.0;

        /// Margin around all items together.
        padding;

        /// Items alignment.
        ///
        /// Horizontal alignment applies to all items together, vertical alignment applies to each
        /// item individually. The default is [`FILL_LEFT`].
        ///
        /// [`FILL_LEFT`]: Align::FILL_LEFT
        items_align(impl IntoVar<Align>) = Align::FILL_LEFT;
    }

    #[inline]
    fn new_child(items: impl WidgetList, spacing: impl IntoVar<Length>, items_align: impl IntoVar<Align>) -> impl UiNode {
        HStackNode {
            children_info: vec![ChildInfo::default(); items.len()],
            items_width: Px(0),
            visible_count: 0,
            children: ZSortedWidgetList::new(items),
            spacing: spacing.into_var(),
            align: items_align.into_var(),
        }
    }

    struct HStackNode<C, S, A> {
        children: C,
        children_info: Vec<ChildInfo>,
        items_width: Px,
        visible_count: u32,

        spacing: S,
        align: A,
    }
    #[impl_ui_node(children)]
    impl<C: WidgetList, S: Var<Length>, A: Var<Align>> UiNode for HStackNode<C, S, A> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            self.children.subscriptions_all(ctx, subscriptions);
            subscriptions.vars(ctx).var(&self.spacing).var(&self.align);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            let mut changed = false;
            self.children.update_all(ctx, &mut changed);

            if changed {
                self.children_info.resize(self.children.len(), ChildInfo::default());
            }

            if changed || self.spacing.is_new(ctx) || self.align.is_new(ctx) {
                ctx.updates.layout_and_render();
            }
        }

        fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
            let mut ds = PxSize::zero();
            self.visible_count = 0;

            self.children.measure_all(
                ctx,
                |_, _| available_size,
                |_, args| {
                    self.children_info[args.index].desired_size = args.desired_size;
                    ds.height = ds.height.max(args.desired_size.height);
                    if args.desired_size.width > Px(0) {
                        ds.width += args.desired_size.width;
                        self.visible_count += 1;
                    }
                },
            );

            let spacing = self.spacing.get(ctx.vars).to_layout(ctx, available_size.width, Px(0));

            ds.width += Px(self.visible_count.saturating_sub(1) as i32) * spacing;
            self.items_width = ds.width;

            if self.align.get(ctx).is_fill_width() {
                ds.max(available_size.to_px())
            } else {
                ds
            }
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
            let spacing = self
                .spacing
                .get(ctx.vars)
                .to_layout(ctx, AvailablePx::Finite(final_size.width), Px(0));
            let align = self.align.copy(ctx);
            let fill_width = align.is_fill_width();

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

            let fill_height = align.is_fill_height();
            let baseline = align.is_baseline();

            self.children.arrange_all(ctx, widget_layout, |_, args| {
                let mut size = self.children_info[args.index].desired_size;

                let spacing = if size.width > Px(0) { spacing } else { Px(0) };

                size.width += extra_width;

                let x = x_offset;
                let y;

                x_offset += size.width + spacing;

                if fill_height {
                    size.height = final_size.height;
                    y = Px(0);
                } else if baseline {
                    y = final_size.height - size.height;
                    args.translate_baseline = Some(true);
                } else {
                    size.height = size.height.min(final_size.height);
                    y = (final_size.height - size.height) * align.y.0;
                };

                let offset = PxVector::new(x, y);
                if offset != PxVector::zero() {
                    args.pre_translate = Some(offset);
                }

                size
            });
        }
    }
}

/// Vertical stack layout.
///
/// # Z-Index
///
/// By default the widgets are layout without overlap, but you can use properties like [`transform`] to cause
/// an widget overlap, in this case the widget will be rendered above its previous sibling and below its next sibling,
/// you can change this by setting the [`z_index`] property in the item widget.
///
/// # Examples
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
/// If you only want to set the `items` property you can use the [`v_stack`] shortcut function.
///
/// [`transform`]: fn@transform
/// [`z_index`]: fn@z_index
/// [`v_stack`]: fn@v_stack
#[widget($crate::widgets::layouts::v_stack)]
pub mod v_stack {
    use super::*;

    properties! {
        /// Space in-between items.
        spacing(impl IntoVar<Length>) = 0.0;

        /// Widget items.
        #[allowed_in_when = false]
        items(impl WidgetList) = widgets![];

        /// Items margin.
        padding;

        /// Items alignment.
        ///
        /// Vertical alignment applies to all items together, horizontal alignment applies to each
        /// item individually. The default is [`FILL_TOP`].
        ///
        /// [`FILL_TOP`]: Align::FILL_TOP
        items_align(impl IntoVar<Align>) = Align::FILL_TOP;
    }

    #[inline]
    fn new_child(items: impl WidgetList, spacing: impl IntoVar<Length>, items_align: impl IntoVar<Align>) -> impl UiNode {
        VStackNode {
            children_info: vec![ChildInfo::default(); items.len()],
            items_height: Px(0),
            visible_count: 0,
            children: ZSortedWidgetList::new(items),
            spacing: spacing.into_var(),
            align: items_align.into_var(),
        }
    }

    struct VStackNode<C, S, A> {
        children: C,
        children_info: Vec<ChildInfo>,
        items_height: Px,
        visible_count: usize,

        spacing: S,
        align: A,
    }
    #[impl_ui_node(children)]
    impl<C: WidgetList, S: Var<Length>, A: Var<Align>> UiNode for VStackNode<C, S, A> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.vars(ctx).var(&self.spacing).var(&self.align);
            self.children.subscriptions_all(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            let mut changed = false;
            self.children.update_all(ctx, &mut changed);

            if changed {
                self.children_info.resize(self.children.len(), ChildInfo::default());
            }

            if changed || self.spacing.is_new(ctx) || self.align.is_new(ctx) {
                ctx.updates.layout_and_render();
            }
        }

        fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
            let mut ds = PxSize::zero();
            self.visible_count = 0;

            self.children.measure_all(
                ctx,
                |_, _| available_size,
                |_, args| {
                    self.children_info[args.index].desired_size = args.desired_size;
                    ds.width = ds.width.max(args.desired_size.width);
                    if args.desired_size.height > Px(0) {
                        ds.height += args.desired_size.height;
                        self.visible_count += 1;
                    }
                },
            );

            let spacing = self.spacing.get(ctx.vars).to_layout(ctx, available_size.height, Px(0));

            ds.height += Px(self.visible_count.saturating_sub(1) as i32) * spacing;
            self.items_height = ds.height;

            if self.align.get(ctx).is_fill_height() {
                ds.max(available_size.to_px())
            } else {
                ds
            }
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
            let spacing = self
                .spacing
                .get(ctx.vars)
                .to_layout(ctx, AvailablePx::Finite(final_size.height), Px(0));
            let align = self.align.copy(ctx);
            let fill_height = align.is_fill_height();
            let baseline = align.is_baseline();

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

            let fill_width = align.is_fill_width();

            self.children.arrange_all(ctx, widget_layout, |_, args| {
                let mut size = self.children_info[args.index].desired_size;

                let spacing = if size.height > Px(0) { spacing } else { Px(0) };
                size.height += extra_height;

                let x;
                let y = y_offset;

                if baseline {
                    args.translate_baseline = Some(true);
                }

                y_offset += size.height + spacing;

                if fill_width {
                    size.width = final_size.width;
                    x = Px(0);
                } else {
                    size.width = size.width.min(final_size.width);
                    x = (final_size.width - size.width) * align.x.0;
                };

                let offset = PxVector::new(x, y);
                if offset != PxVector::zero() {
                    args.pre_translate = Some(offset);
                }

                size
            });
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
/// # Z-Index
///
/// By default the widgets are rendered in their logical order, the last widget renders in front of the others,
/// you can change this by setting the [`z_index`] property in the item widget.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// let text = z_stack! {
///     padding = 5.0;
///     items = widgets![
///         text("one"),
///         text! { text = "three"; z_index = ZIndex::DEFAULT + 1; },
///         text("two"),
///     ];
/// };
/// ```
///
/// ## `z_stack()`
///
/// If you only want to set the `items` property you can use the [`z_stack`](function@z_stack) shortcut function.
///
/// [`z_index`]: fn@z_index
#[widget($crate::widgets::layouts::z_stack)]
pub mod z_stack {
    use super::*;

    properties! {
        /// Widget items.
        #[allowed_in_when = false]
        items(impl WidgetList) = widgets![];

        /// Items margin.
        padding;

        /// Items alignment.
        ///
        /// Align applies to each item individually. The default is [`FILL`].
        ///
        /// [`FILL`]: Align::FILL
        items_align(impl IntoVar<Align>) = Align::FILL;
    }

    #[inline]
    fn new_child(items: impl WidgetList, items_align: impl IntoVar<Align>) -> impl UiNode {
        ZStackNode {
            children_info: vec![ChildInfo::default(); items.len()],
            children: ZSortedWidgetList::new(items),
            align: items_align.into_var(),
        }
    }
    struct ZStackNode<C, A> {
        children: C,
        children_info: Vec<ChildInfo>,
        align: A,
    }
    #[impl_ui_node(children)]
    impl<I: UiNodeList, A: Var<Align>> UiNode for ZStackNode<I, A> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &self.align);
            self.children.subscriptions_all(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            let mut changed = false;
            self.children.update_all(ctx, &mut changed);

            if changed {
                self.children_info.resize(self.children.len(), ChildInfo::default());
            }

            if changed || self.align.is_new(ctx) {
                ctx.updates.layout_and_render();
            }
        }

        fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
            let mut ds = PxSize::zero();
            self.children.measure_all(
                ctx,
                |_, _| available_size,
                |_, args| {
                    ds = ds.max(args.desired_size);
                    self.children_info[args.index].desired_size = args.desired_size;
                },
            );
            ds
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
            let align = self.align.copy(ctx);
            let baseline = align.is_baseline();
            self.children.arrange_all(ctx, widget_layout, |_, args| {
                let size = self.children_info[args.index].desired_size.min(final_size);
                if baseline {
                    args.translate_baseline = Some(true);
                }
                let bounds = align.solve(size, size.height, final_size);
                if bounds.origin != PxPoint::zero() {
                    args.pre_translate = Some(bounds.origin.to_vector());
                }
                bounds.size
            });
        }
    }
}

/// Basic layering stack layout.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// let text = z_stack(widgets![
///     text("back"),
///     text("front"),
/// ]);
/// ```
///
/// # `z_stack!`
///
/// This function is just a shortcut for [`z_stack!`](module@z_stack). Use the full widget
/// to better configure the layering stack widget.
pub fn z_stack(items: impl WidgetList) -> impl Widget {
    z_stack! { items; }
}

#[derive(Default, Clone, Copy)]
struct ChildInfo {
    desired_size: PxSize,
}
