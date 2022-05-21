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
/// [`transform`]: fn@transform
/// [`z_index`]: fn@z_index
/// [`h_stack`]: fn@h_stack
#[widget($crate::widgets::layouts::h_stack)]
pub mod h_stack {
    use super::*;

    properties! {
        /// Widget items.
        #[allowed_in_when = false]
        items(impl WidgetList) = widgets![];

        /// Space in-between items.
        spacing(impl IntoVar<Length>) = 0.0;

        /// Spacing around the items stack, inside the border.
        padding;

        /// Items alignment.
        ///
        /// Horizontal alignment applies to all items together, vertical alignment applies to each
        /// item individually. The default is [`FILL_LEFT`].
        ///
        /// [`FILL_LEFT`]: Align::FILL_LEFT
        items_align(impl IntoVar<Align>) = Align::FILL_LEFT;
    }

    fn new_child(items: impl WidgetList, spacing: impl IntoVar<Length>, items_align: impl IntoVar<Align>) -> impl UiNode {
        let node = HStackNode {
            children: ZSortedWidgetList::new(items),
            spacing: spacing.into_var(),
            align: items_align.into_var(),
        };
        implicit_base::nodes::children_layout(node)
    }

    struct HStackNode<C, S, A> {
        children: C,

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

            if changed || self.spacing.is_new(ctx) || self.align.is_new(ctx) {
                ctx.updates.layout_and_render();
            }
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let spacing = self.spacing.get(ctx.vars).layout(ctx.for_x(), |_| Px(0));
            let align = self.align.copy(ctx);

            let mut size = PxSize::zero();

            ctx.with_constrains(
                |c| align.child_constrains(c).with_unbounded_x(),
                |ctx| {
                    self.children.layout_all(
                        ctx,
                        wl,
                        |_, _, _| {},
                        |_, wl, a| {
                            wl.translate(PxVector::new(size.width, Px(0)));

                            size.height = size.height.max(a.size.height);

                            if a.size.width > Px(0) {
                                // only add spacing for visible items.
                                size.width += a.size.width + spacing;
                            }
                        },
                    );
                },
            );

            let c = ctx.constrains();
            if align.is_fill_y() && !c.y.is_fill_max() && !c.y.is_exact() {
                // panel is not fill-y but items are, so we need to fill to the widest item.
                ctx.with_constrains(
                    move |c| c.with_max_y(c.y.clamp(size.height)).with_fill_x(true).with_unbounded_x(),
                    |ctx| {
                        size.width = Px(0);
                        for i in 0..self.children.len() {
                            let o_size = self.children.widget_bounds_info(i).outer_size();
                            if Some(o_size.height) != ctx.constrains().y.max() {
                                // only need second pass for items that don't fill
                                let (a_size, _) = wl.with_child(ctx, |ctx, wl| {
                                    wl.translate(PxVector::new(size.width, Px(0)));
                                    self.children.widget_layout(i, ctx, wl)
                                });

                                size.height = size.height.max(a_size.height);

                                if a_size.width > Px(0) {
                                    size.width += a_size.width + spacing;
                                }
                            } else {
                                // item already fills width, but may have moved due to sibling new fill size
                                self.children.widget_outer(i, wl, false, |wlt, _| {
                                    wlt.translate(PxVector::new(size.width, Px(0)));

                                    if o_size.width > Px(0) {
                                        size.width += o_size.width + spacing;
                                    }
                                });
                            }
                        }
                    },
                );
            }

            if size.width > Px(0) {
                // spacing is only in between items.
                size.width -= spacing;
            }
            let best_size = ctx.constrains().fill_size_or(size);
            let extra_width = best_size.width - size.width;
            let mut extra_x = Px(0);

            if align.is_fill_x() {
                if extra_width != Px(0) {
                    // TODO distribute/take width
                }
            } else if extra_width > Px(0) {
                extra_x = extra_width * align.x;
            }
            if !align.is_fill_y() && align.y > 0.fct() {
                self.children.outer_all(wl, true, |wlt, a| {
                    let y = (best_size.height - a.size.height) * align.y;
                    wlt.translate(PxVector::new(extra_x, y));
                });
            } else if extra_x > Px(0) {
                self.children.outer_all(wl, true, |wlt, _| {
                    wlt.translate(PxVector::new(extra_x, Px(0)));
                });
            }

            best_size.max(size)
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

        /// Spacing around the items stack, inside the border.
        padding;

        /// Items alignment.
        ///
        /// Vertical alignment applies to all items together, horizontal alignment applies to each
        /// item individually. The default is [`FILL_TOP`].
        ///
        /// [`FILL_TOP`]: Align::FILL_TOP
        items_align(impl IntoVar<Align>) = Align::FILL_TOP;
    }

    fn new_child(items: impl WidgetList, spacing: impl IntoVar<Length>, items_align: impl IntoVar<Align>) -> impl UiNode {
        let node = VStackNode {
            children: ZSortedWidgetList::new(items),
            spacing: spacing.into_var(),
            align: items_align.into_var(),
        };
        implicit_base::nodes::children_layout(node)
    }

    struct VStackNode<C, S, A> {
        children: C,

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

            if changed || self.spacing.is_new(ctx) || self.align.is_new(ctx) {
                ctx.updates.layout_and_render();
            }
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let spacing = self.spacing.get(ctx.vars).layout(ctx.for_y(), |_| Px(0));
            let align = self.align.copy(ctx);

            let mut size = PxSize::zero();

            ctx.with_constrains(
                |c| align.child_constrains(c).with_unbounded_y(),
                |ctx| {
                    self.children.layout_all(
                        ctx,
                        wl,
                        |_, _, _| {},
                        |_, wl, a| {
                            wl.translate(PxVector::new(Px(0), size.height));

                            size.width = size.width.max(a.size.width);

                            if a.size.height > Px(0) {
                                // only add spacing for visible items.
                                size.height += a.size.height + spacing;
                            }
                        },
                    );
                },
            );

            let c = ctx.constrains();
            if align.is_fill_x() && !c.x.is_fill_max() && !c.x.is_exact() {
                // panel is not fill-x but items are, so we need to fill to the widest item.
                ctx.with_constrains(
                    move |c| c.with_max_x(c.x.clamp(size.width)).with_fill_x(true).with_unbounded_y(),
                    |ctx| {
                        size.height = Px(0);
                        for i in 0..self.children.len() {
                            let o_size = self.children.widget_bounds_info(i).outer_size();
                            if Some(o_size.width) != ctx.constrains().x.max() {
                                // only need second pass for items that don't fill
                                let (a_size, _) = wl.with_child(ctx, |ctx, wl| {
                                    wl.translate(PxVector::new(Px(0), size.height));
                                    self.children.widget_layout(i, ctx, wl)
                                });

                                size.width = size.width.max(a_size.width);

                                if a_size.height > Px(0) {
                                    size.height += a_size.height + spacing;
                                }
                            } else {
                                // item already fills width, but may have moved due to sibling new fill size
                                self.children.widget_outer(i, wl, false, |wlt, _| {
                                    wlt.translate(PxVector::new(Px(0), size.height));

                                    if o_size.height > Px(0) {
                                        size.height += o_size.height + spacing;
                                    }
                                });
                            }
                        }
                    },
                );
            }

            if size.height > Px(0) {
                // spacing is only in between items.
                size.height -= spacing;
            }
            let best_size = ctx.constrains().fill_size_or(size);
            let extra_height = best_size.height - size.height;
            let mut extra_y = Px(0);

            if align.is_fill_y() {
                if extra_height != Px(0) {
                    // TODO distribute/take height
                }
            } else if extra_height > Px(0) {
                extra_y = extra_height * align.x;
            }
            if extra_height > Px(0) {
                if align.is_fill_y() {
                    // TODO distribute height
                } else {
                    extra_y = extra_height * align.y;
                }
            }
            if !align.is_fill_x() && align.x > 0.fct() {
                self.children.outer_all(wl, true, |wlt, a| {
                    let x = (best_size.width - a.size.width) * align.x;
                    wlt.translate(PxVector::new(x, extra_y));
                });
            } else if extra_y > Px(0) {
                self.children.outer_all(wl, true, |wlt, _| {
                    wlt.translate(PxVector::new(Px(0), extra_y));
                });
            }

            best_size.max(size)
        }
    }
}

/// Basic horizontal stack layout.
///
/// # Examples
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
/// # Examples
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
///
/// # `stack_nodes`
///
/// If you only want to create an overlaying effect composed of multiple nodes you can use the [`stack_nodes`] function.
///
/// [`stack_nodes`]: fn@stack_nodes
#[widget($crate::widgets::layouts::z_stack)]
pub mod z_stack {
    use super::*;

    properties! {
        /// Widget items.
        #[allowed_in_when = false]
        items(impl WidgetList) = widgets![];

        /// Spacing around the items stack, inside the border.
        padding;

        /// Items alignment.
        ///
        /// Align applies to each item individually. The default is [`FILL`].
        ///
        /// [`FILL`]: Align::FILL
        items_align(impl IntoVar<Align>) = Align::FILL;
    }

    fn new_child(items: impl WidgetList, items_align: impl IntoVar<Align>) -> impl UiNode {
        ZStackNode {
            children: ZSortedWidgetList::new(items),
            align: items_align.into_var(),
        }
    }
    struct ZStackNode<C, A> {
        children: C,
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

            if changed || self.align.is_new(ctx) {
                ctx.updates.layout_and_render();
            }
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let mut size = PxSize::zero();
            let align = self.align.copy(ctx);

            let parent_constrains = ctx.constrains();

            ctx.with_constrains(
                |c| align.child_constrains(c),
                |ctx| {
                    self.children.layout_all(
                        ctx,
                        wl,
                        |_, _, _| {},
                        |_, wl, args| {
                            let child_size = align.layout(args.size, parent_constrains, wl);
                            size = size.max(child_size);
                        },
                    );
                },
            );

            parent_constrains.clamp_size(size)
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

/// Creates a node that processes the `nodes` in the logical order they appear in the list, layouts for the largest node
/// and renders then on on top of the other from back to front.
///
/// This is the most simple *z-stack* implementation possible, it is a building block useful for quickly declaring
/// overlaying effects composed of multiple nodes, it does not do any alignment layout or z-sorting render,
/// for a complete z-stack panel widget see [`z_stack`].
///
/// [`z_stack`]: mod@z_stack
pub fn stack_nodes(nodes: impl UiNodeList) -> impl UiNode {
    struct NodesStackNode<C> {
        children: C,
    }
    #[impl_ui_node(children)]
    impl<C: UiNodeList> NodesStackNode<C> {}

    NodesStackNode { children: nodes }.cfg_boxed()
}
