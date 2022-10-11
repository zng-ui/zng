use crate::prelude::new_widget::*;

/// Horizontal stack layout.
///
/// # Z-Index
///
/// By default the widgets are layout without overlap, but you can use properties like [`transform`] to cause
/// a widget overlap, in this case the widget will be rendered above its previous sibling and below its next sibling,
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

    #[impl_ui_node(struct HStackNode {
        children: impl WidgetList,

        #[var] spacing: impl Var<Length>,
        #[var] align: impl Var<Align>,
    })]
    impl UiNode for HStackNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            let mut changed = false;
            self.children.update_all(ctx, updates, &mut changed);

            if changed || self.spacing.is_new(ctx) || self.align.is_new(ctx) {
                ctx.updates.layout_and_render();
            }
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            let spacing = self.spacing.get().layout(ctx.for_x(), |_| Px(0));
            let align = self.align.get();

            let constrains = ctx.constrains();
            if let Some(known) = constrains.fill_or_exact() {
                return known;
            }

            let mut size = PxSize::zero();

            ctx.with_constrains(
                |c| align.child_constrains(c).with_unbounded_x(),
                |ctx| {
                    self.children.measure_all(
                        ctx,
                        |_, _| {},
                        |_, a| {
                            size.height = size.height.max(a.size.height);
                            if a.size.width > Px(0) {
                                size.width += a.size.width + spacing;
                            }
                        },
                    );
                },
            );

            if size.width > Px(0) {
                size.width -= spacing;
            }

            constrains.fill_size_or(size)
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let spacing = self.spacing.get().layout(ctx.for_x(), |_| Px(0));
            let align = self.align.get();
            let align_baseline = align.is_baseline();

            let constrains = ctx.constrains();
            let mut panel_height = None;

            if let Some(known) = constrains.y.fill_or_exact() {
                panel_height = Some(known);
            } else if align.is_fill_y() {
                // need width before layout because children fill and widest child define width.

                let mut max_h = Px(0);
                ctx.as_measure().with_constrains(
                    |c| align.child_constrains(c).with_unbounded_x(),
                    |ctx| {
                        self.children.measure_all(
                            ctx,
                            |_, _| {},
                            |_, a| {
                                max_h = max_h.max(a.size.height);
                            },
                        );
                    },
                );
                panel_height = Some(constrains.y.fill_or(max_h));
            }

            if let Some(panel_height) = panel_height {
                let mut x = Px(0);
                let align_y = if align_baseline {
                    1.fct()
                } else if align.is_fill_y() {
                    0.fct()
                } else {
                    align.y
                };
                ctx.with_constrains(
                    |c| {
                        align
                            .child_constrains(c.with_fill_y(true).with_max_y(panel_height))
                            .with_unbounded_x()
                    },
                    |ctx| {
                        self.children.layout_all(
                            ctx,
                            wl,
                            |_, _, _| {},
                            |_, wl, a| {
                                let y = (panel_height - a.size.height) * align_y;
                                wl.translate(PxVector::new(x, y));
                                wl.translate_baseline(align_baseline);

                                if a.size.width > Px(0) {
                                    x += a.size.width + spacing;
                                }
                            },
                        );
                    },
                );

                if x > Px(0) {
                    x -= spacing;
                }

                let panel_width = constrains.x.fill_or(x);

                let align_x = if align.is_fill_x() { 0.fct() } else { align.x };
                let extra_x = (panel_width - x) * align_x;

                if extra_x != Px(0) {
                    self.children.outer_all(wl, true, |wlt, _| {
                        wlt.translate(PxVector::new(extra_x, Px(0)));
                    });
                }

                PxSize::new(panel_width, panel_height)
            } else {
                let mut max_height = Px(0);
                let mut x = Px(0);
                ctx.with_constrains(
                    |c| c.with_unbounded_x(),
                    |ctx| {
                        self.children.layout_all(
                            ctx,
                            wl,
                            |_, _, _| {},
                            |_, wl, a| {
                                wl.translate(PxVector::new(x, Px(0)));
                                wl.translate_baseline(align_baseline);

                                max_height = max_height.max(a.size.height);

                                if a.size.width > Px(0) {
                                    x += a.size.width + spacing;
                                }
                            },
                        )
                    },
                );

                let panel_height = constrains.y.clamp(max_height);

                if x > Px(0) {
                    x -= spacing;
                }
                let panel_width = constrains.x.fill_or(x);

                let align_x = if align.is_fill_x() { 0.fct() } else { align.x };
                let extra_x = (panel_width - x) * align_x;

                let align_y = if align_baseline {
                    1.fct()
                } else if align.is_fill_y() {
                    0.fct()
                } else {
                    align.y
                };

                self.children.outer_all(wl, true, |wlt, a| {
                    let y = (panel_height - a.size.height) * align_y;
                    wlt.translate(PxVector::new(extra_x, y));
                });

                PxSize::new(panel_width, panel_height)
            }
        }
    }
}

/// Vertical stack layout.
///
/// # Z-Index
///
/// By default the widgets are layout without overlap, but you can use properties like [`transform`] to cause
/// a widget overlap, in this case the widget will be rendered above its previous sibling and below its next sibling,
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

    #[impl_ui_node(struct VStackNode {
        children: impl WidgetList,

        #[var] spacing: impl Var<Length>,
        #[var] align: impl Var<Align>,
    })]
    impl UiNode for VStackNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            let mut changed = false;
            self.children.update_all(ctx, updates, &mut changed);

            if changed || self.spacing.is_new(ctx) || self.align.is_new(ctx) {
                ctx.updates.layout_and_render();
            }
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            let spacing = self.spacing.get().layout(ctx.for_y(), |_| Px(0));
            let align = self.align.get();

            let constrains = ctx.constrains();
            if let Some(known) = constrains.fill_or_exact() {
                return known;
            }

            let mut size = PxSize::zero();

            ctx.with_constrains(
                |c| align.child_constrains(c).with_unbounded_y(),
                |ctx| {
                    self.children.measure_all(
                        ctx,
                        |_, _| {},
                        |_, a| {
                            size.width = size.width.max(a.size.width);
                            if a.size.height > Px(0) {
                                size.height += a.size.height + spacing;
                            }
                        },
                    );
                },
            );

            if size.height > Px(0) {
                size.height -= spacing;
            }

            constrains.fill_size_or(size)
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let spacing = self.spacing.get().layout(ctx.for_y(), |_| Px(0));
            let align = self.align.get();
            let align_baseline = align.is_baseline();

            let constrains = ctx.constrains();
            let mut panel_width = None;

            if let Some(known) = constrains.x.fill_or_exact() {
                panel_width = Some(known);
            } else if align.is_fill_x() {
                // need width before layout because children fill and widest child define width.

                let mut max_w = Px(0);
                ctx.as_measure().with_constrains(
                    |c| align.child_constrains(c).with_unbounded_y(),
                    |ctx| {
                        self.children.measure_all(
                            ctx,
                            |_, _| {},
                            |_, a| {
                                max_w = max_w.max(a.size.width);
                            },
                        );
                    },
                );
                panel_width = Some(constrains.x.fill_or(max_w));
            }

            if let Some(panel_width) = panel_width {
                let mut y = Px(0);
                let align_x = if align.is_fill_x() { 0.fct() } else { align.x };
                ctx.with_constrains(
                    |c| {
                        align
                            .child_constrains(c.with_fill_x(true).with_max_x(panel_width))
                            .with_unbounded_y()
                    },
                    |ctx| {
                        self.children.layout_all(
                            ctx,
                            wl,
                            |_, _, _| {},
                            |_, wl, a| {
                                let x = (panel_width - a.size.width) * align_x;
                                wl.translate(PxVector::new(x, y));
                                wl.translate_baseline(align_baseline);

                                if a.size.height > Px(0) {
                                    y += a.size.height + spacing;
                                }
                            },
                        );
                    },
                );

                if y > Px(0) {
                    y -= spacing;
                }

                let panel_height = constrains.y.fill_or(y);

                let align_y = if align_baseline {
                    1.fct()
                } else if align.is_fill_y() {
                    0.fct()
                } else {
                    align.y
                };
                let extra_y = (panel_height - y) * align_y;

                if extra_y != Px(0) {
                    self.children.outer_all(wl, true, |wlt, _| {
                        wlt.translate(PxVector::new(Px(0), extra_y));
                    });
                }

                PxSize::new(panel_width, panel_height)
            } else {
                let mut max_width = Px(0);
                let mut y = Px(0);
                ctx.with_constrains(
                    |c| c.with_unbounded_y(),
                    |ctx| {
                        self.children.layout_all(
                            ctx,
                            wl,
                            |_, _, _| {},
                            |_, wl, a| {
                                wl.translate(PxVector::new(Px(0), y));
                                wl.translate_baseline(align_baseline);

                                max_width = max_width.max(a.size.width);

                                if a.size.height > Px(0) {
                                    y += a.size.height + spacing;
                                }
                            },
                        )
                    },
                );

                let panel_width = constrains.x.clamp(max_width);

                if y > Px(0) {
                    y -= spacing;
                }
                let panel_height = constrains.y.fill_or(y);

                let align_y = if align_baseline {
                    1.fct()
                } else if align.is_fill_y() {
                    0.fct()
                } else {
                    align.y
                };
                let extra_y = (panel_height - y) * align_y;

                let align_x = if align.is_fill_x() { 0.fct() } else { align.x };

                self.children.outer_all(wl, true, |wlt, a| {
                    let x = (panel_width - a.size.width) * align_x;
                    wlt.translate(PxVector::new(x, extra_y));
                });

                PxSize::new(panel_width, panel_height)
            }
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
        let node = ZStackNode {
            children: ZSortedWidgetList::new(items),
            align: items_align.into_var(),
        };
        implicit_base::nodes::children_layout(node)
    }

    #[impl_ui_node(struct ZStackNode {
        children: impl WidgetList,
        #[var] align: impl Var<Align>,
    })]
    impl UiNode for ZStackNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            let mut changed = false;
            self.children.update_all(ctx, updates, &mut changed);

            if changed || self.align.is_new(ctx) {
                ctx.updates.layout_and_render();
            }
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            let constrains = ctx.constrains();
            if let Some(known) = constrains.fill_or_exact() {
                return known;
            }

            let align = self.align.get();
            let mut size = PxSize::zero();
            ctx.with_constrains(
                |c| align.child_constrains(c),
                |ctx| {
                    self.children.measure_all(
                        ctx,
                        |_, _| {},
                        |_, args| {
                            let child_size = align.measure(args.size, constrains);
                            size = size.max(child_size);
                        },
                    );
                },
            );

            constrains.fill_size_or(size)
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let align = self.align.get();

            let constrains = ctx.constrains();
            let mut size = None;

            if let Some(known) = constrains.fill_or_exact() {
                size = Some(known)
            } else if align.is_fill_x() || align.is_fill_y() {
                // need size before layout because children fill and largest child defines size.

                let mut max_size = PxSize::zero();
                ctx.as_measure().with_constrains(
                    |c| align.child_constrains(c),
                    |ctx| {
                        self.children.measure_all(
                            ctx,
                            |_, _| {},
                            |_, a| {
                                max_size = max_size.max(a.size);
                            },
                        );
                    },
                );

                size = Some(constrains.fill_size_or(max_size));
            }

            if let Some(size) = size {
                ctx.with_constrains(
                    |_| align.child_constrains(PxConstrains2d::new_fill_size(size)),
                    |ctx| {
                        self.children.layout_all(
                            ctx,
                            wl,
                            |_, _, _| {},
                            |_, wl, args| {
                                align.layout(args.size, constrains, wl);
                            },
                        );
                    },
                );

                size
            } else {
                let mut size = PxSize::zero();
                ctx.with_constrains(
                    |c| align.child_constrains(c),
                    |ctx| {
                        self.children.layout_all(
                            ctx,
                            wl,
                            |_, _, _| {},
                            |_, wl, args| {
                                let child_size = align.layout(args.size, constrains, wl);
                                size = size.max(child_size);
                            },
                        );
                    },
                );

                let size = constrains.fill_size_or(size);

                let align_x = if align.is_fill_x() { 0.fct() } else { align.x };
                let align_y = if align.is_baseline() {
                    1.fct()
                } else if align.is_fill_y() {
                    0.fct()
                } else {
                    align.y
                };

                self.children.outer_all(wl, false, |wlt, a| {
                    let x = (size.width - a.size.width) * align_x;
                    let y = (size.height - a.size.height) * align_y;
                    wlt.translate(PxVector::new(x, y));
                });

                size
            }
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

/// Creates a node that updates and layouts the `nodes` in the logical order they appear in the list
/// and renders then on on top of the other from back(0) to front(len-1). The layout size is the largest item width and height,
/// the parent constrains are used for the layout of each item.
///
/// This is the most simple *z-stack* implementation possible, it is a building block useful for quickly declaring
/// overlaying effects composed of multiple nodes, it does not do any alignment layout or z-sorting render,
/// for a complete z-stack panel widget see [`z_stack`].
///
/// [`z_stack`]: mod@z_stack
pub fn stack_nodes(nodes: impl UiNodeList) -> impl UiNode {
    struct StackNodesNode<C> {
        children: C,
    }
    #[impl_ui_node(children)]
    impl<C: UiNodeList> StackNodesNode<C> {}

    StackNodesNode { children: nodes }.cfg_boxed()
}

/// Creates a node that updates the `nodes` in the logical order they appear, renders then on on top of the other from back(0) to front(len-1),
/// but layouts the `index` item first and uses its size to get `constrains` for the other items.
///
/// The layout size is the largest item width and height.
///
/// If the `index` is out of range the node logs an error and behaves like [`stack_nodes`].
pub fn stack_nodes_layout_by(
    nodes: impl UiNodeList,
    index: impl IntoVar<usize>,
    constrains: impl Fn(PxConstrains2d, usize, PxSize) -> PxConstrains2d + 'static,
) -> impl UiNode {
    #[impl_ui_node(struct StackNodesFillNode {
        children: impl UiNodeList,
        #[var] index: impl Var<usize>,
        constrains: impl Fn(PxConstrains2d, usize, PxSize) -> PxConstrains2d + 'static,
    })]
    impl UiNode for StackNodesFillNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.index.is_new(ctx) {
                ctx.updates.layout();
            }
            self.children.update_all(ctx, updates, &mut ());
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            let index = self.index.get();
            let len = self.children.len();
            if index >= len {
                tracing::error!(
                    "index {} out of range for length {} in `{:?}#stack_nodes_layout_by`",
                    index,
                    len,
                    ctx.path
                );
                let mut size = PxSize::zero();
                self.children.measure_all(ctx, |_, _| {}, |_, args| size = size.max(args.size));
                size
            } else {
                let mut size = self.children.item_measure(index, ctx);
                let constrains = (self.constrains)(ctx.peek(|m| m.constrains()), index, size);
                ctx.with_constrains(
                    |_| constrains,
                    |ctx| {
                        for i in 0..len {
                            if i != index {
                                size = size.max(self.children.item_measure(i, ctx));
                            }
                        }
                    },
                );
                size
            }
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let index = self.index.get();
            let len = self.children.len();
            if index >= len {
                tracing::error!(
                    "index {} out of range for length {} in `{:?}#stack_nodes_layout_by`",
                    index,
                    len,
                    ctx.path
                );
                let mut size = PxSize::zero();
                self.children
                    .layout_all(ctx, wl, |_, _, _| {}, |_, _, args| size = size.max(args.size));
                size
            } else {
                let mut size = self.children.item_layout(index, ctx, wl);
                let constrains = (self.constrains)(ctx.peek(|m| m.constrains()), index, size);
                ctx.with_constrains(
                    |_| constrains,
                    |ctx| {
                        for i in 0..len {
                            if i != index {
                                size = size.max(self.children.item_layout(i, ctx, wl));
                            }
                        }
                    },
                );
                size
            }
        }
    }
    StackNodesFillNode {
        children: nodes,
        index: index.into_var(),
        constrains,
    }
    .cfg_boxed()
}
