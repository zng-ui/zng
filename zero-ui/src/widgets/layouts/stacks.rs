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
///     children = ui_list![
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

    inherit!(widget_base::base);

    properties! {
        /// Widget items.
        pub widget_base::children;

        /// Space in-between items.
        pub spacing(impl IntoVar<Length>);

        /// Spacing around the items stack, inside the border.
        pub crate::properties::padding;

        /// Items alignment.
        ///
        /// Horizontal alignment applies to all items together, vertical alignment applies to each
        /// item individually. The default is [`FILL_LEFT`].
        ///
        /// [`FILL_LEFT`]: Align::FILL_LEFT
        pub children_align(impl IntoVar<Align>) = Align::FILL_LEFT;
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(|wgt| {
            let children = wgt.capture_ui_node_list_or_empty(property_id!(self::children));
            let spacing = wgt.capture_var_or_default(property_id!(self::spacing));
            let children_align = wgt.capture_var_or_else(property_id!(self::children_align), || Align::FILL_LEFT);

            let node = HStackNode {
                children: ZSortingList::new(children),
                spacing: spacing.into_var(),
                align: children_align.into_var(),
            };
            let child = widget_base::nodes::children_layout(node);

            wgt.set_child(child);
        });
    }

    #[ui_node(struct HStackNode {
        children: impl UiNodeList,

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
                    self.children.for_each(|_, n| {
                        let s = n.measure(ctx);
                        size.height = size.height.max(s.height);
                        if s.width > Px(0) {
                            size.width += s.width + spacing;
                        }
                        true
                    });
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
                        self.children.for_each(|_, n| {
                            let s = n.measure(ctx);
                            max_h = max_h.max(s.height);
                            true
                        });
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
                        self.children.for_each_mut(|_, n| {
                            let s = n.layout(ctx, wl);

                            let y = (panel_height - s.height) * align_y;
                            wl.translate(PxVector::new(x, y));
                            wl.translate_baseline(align_baseline);

                            if s.width > Px(0) {
                                x += s.width + spacing;
                            }
                            true
                        });
                    },
                );

                if x > Px(0) {
                    x -= spacing;
                }

                let panel_width = constrains.x.fill_or(x);

                let align_x = if align.is_fill_x() { 0.fct() } else { align.x };
                let extra_x = (panel_width - x) * align_x;

                if extra_x != Px(0) {
                    self.children.for_each_mut(|_, n| {
                        wl.with_outer(n, true, |wlt, _| {
                            wlt.translate(PxVector::new(extra_x, Px(0)));
                        });

                        true
                    });
                }

                PxSize::new(panel_width, panel_height)
            } else {
                let mut max_height = Px(0);
                let mut x = Px(0);
                ctx.with_constrains(
                    |c| c.with_unbounded_x(),
                    |ctx| {
                        self.children.for_each_mut(|_, n| {
                            let s = n.layout(ctx, wl);

                            wl.translate(PxVector::new(x, Px(0)));
                            wl.translate_baseline(align_baseline);

                            max_height = max_height.max(s.height);
                            if s.width > Px(0) {
                                x += s.width + spacing;
                            }

                            true
                        });
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

                self.children.for_each_mut(|_, n| {
                    wl.with_outer(n, true, |wlt, n| {
                        let node_size = n.with_context(|ctx| ctx.widget_info.bounds.outer_size()).unwrap();
                        let y = (panel_height - node_size.height) * align_y;
                        wlt.translate(PxVector::new(extra_x, y));
                    });
                    true
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
///     children = ui_list![
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

    inherit!(widget_base::base);

    properties! {
        /// Widget items.
        pub widget_base::children;

        /// Space in-between items.
        pub spacing(impl IntoVar<Length>);

        /// Spacing around the items stack, inside the border.
        pub crate::properties::padding;

        /// Items alignment.
        ///
        /// Vertical alignment applies to all items together, horizontal alignment applies to each
        /// item individually. The default is [`FILL_TOP`].
        ///
        /// [`FILL_TOP`]: Align::FILL_TOP
        pub children_align(impl IntoVar<Align>) = Align::FILL_TOP;
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(|wgt| {
            let children = wgt.capture_ui_node_list_or_empty(property_id!(self::children));
            let spacing = wgt.capture_var_or_default(property_id!(self::spacing));
            let children_align = wgt.capture_var_or_else(property_id!(self::children_align), || Align::FILL_LEFT);

            let node = VStackNode {
                children: ZSortingList::new(children),
                spacing: spacing.into_var(),
                align: children_align.into_var(),
            };
            let child = widget_base::nodes::children_layout(node);

            wgt.set_child(child);
        });
    }

    #[ui_node(struct VStackNode {
        children: impl UiNodeList,

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
                    self.children.for_each(|_, n| {
                        let s = n.measure(ctx);

                        size.width = size.width.max(s.width);
                        if s.height > Px(0) {
                            size.height += s.height + spacing;
                        }
                        true
                    });
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
                        self.children.for_each(|_, n| {
                            let s = n.measure(ctx);
                            max_w = max_w.max(s.width);
                            true
                        });
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
                        self.children.for_each_mut(|_, n| {
                            let s = n.layout(ctx, wl);

                            let x = (panel_width - s.width) * align_x;
                            wl.translate(PxVector::new(x, y));
                            wl.translate_baseline(align_baseline);

                            if s.height > Px(0) {
                                y += s.height + spacing;
                            }

                            true
                        });
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
                    self.children.for_each_mut(|_, n| {
                        wl.with_outer(n, true, |wlt, _| {
                            wlt.translate(PxVector::new(Px(0), extra_y));
                        });
                        true
                    });
                }

                PxSize::new(panel_width, panel_height)
            } else {
                let mut max_width = Px(0);
                let mut y = Px(0);
                ctx.with_constrains(
                    |c| c.with_unbounded_y(),
                    |ctx| {
                        self.children.for_each_mut(|_, n| {
                            let s = n.layout(ctx, wl);

                            wl.translate(PxVector::new(Px(0), y));
                            wl.translate_baseline(align_baseline);

                            max_width = max_width.max(s.width);

                            if s.height > Px(0) {
                                y += s.height + spacing;
                            }

                            true
                        });
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

                self.children.for_each_mut(|_, n| {
                    wl.with_outer(n, true, |wlt, n| {
                        let node_size = n.with_context(|ctx| ctx.widget_info.bounds.outer_size()).unwrap();
                        let x = (panel_width - node_size.width) * align_x;
                        wlt.translate(PxVector::new(x, extra_y));
                    });
                    true
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
/// let text = h_stack(ui_list![
///     text("Hello "),
///     text("World"),
/// ]);
/// ```
///
/// # `h_stack!`
///
/// This function is just a shortcut for [`h_stack!`](module@v_stack). Use the full widget
/// to better configure the horizontal stack widget.
pub fn h_stack(children: impl UiNodeList) -> impl UiNode {
    h_stack! {
        children;
    }
}

/// Basic vertical stack layout.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// let text = v_stack(ui_list![
///     text("1. Hello"),
///     text("2. World"),
/// ]);
/// ```
///
/// # `v_stack!`
///
/// This function is just a shortcut for [`v_stack!`](module@v_stack). Use the full widget
/// to better configure the vertical stack widget.
pub fn v_stack(children: impl UiNodeList) -> impl UiNode {
    v_stack! {
        children;
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
///     children = ui_list![
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

    inherit!(widget_base::base);

    properties! {
        /// Widget items.
        pub widget_base::children;

        /// Spacing around the items stack, inside the border.
        pub crate::properties::padding;

        /// Items alignment.
        ///
        /// Align applies to each item individually. The default is [`FILL`].
        ///
        /// [`FILL`]: Align::FILL
        pub children_align(impl IntoVar<Align>) = Align::FILL;
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(|wgt| {
            let children = wgt.capture_ui_node_list_or_empty(property_id!(self::children));
            let children_align = wgt.capture_var_or_else(property_id!(self::children_align), || Align::FILL);
            let node = ZStackNode {
                children: ZSortingList::new(children),
                align: children_align.into_var(),
            };
            let child = widget_base::nodes::children_layout(node);

            wgt.set_child(child);
        });
    }

    #[ui_node(struct ZStackNode {
        children: impl UiNodeList,
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
                    self.children.for_each(|_, n| {
                        let s = n.measure(ctx);
                        let child_size = align.measure(s, constrains);
                        size = size.max(child_size);
                        true
                    });
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
                        self.children.for_each(|_, n| {
                            let s = n.measure(ctx);
                            max_size = max_size.max(s);
                            true
                        });
                    },
                );

                size = Some(constrains.fill_size_or(max_size));
            }

            if let Some(size) = size {
                ctx.with_constrains(
                    |_| align.child_constrains(PxConstrains2d::new_fill_size(size)),
                    |ctx| {
                        self.children.for_each_mut(|_, n| {
                            let s = n.layout(ctx, wl);
                            align.layout(s, constrains, wl);
                            true
                        });
                    },
                );

                size
            } else {
                let mut size = PxSize::zero();
                ctx.with_constrains(
                    |c| align.child_constrains(c),
                    |ctx| {
                        self.children.for_each_mut(|_, n| {
                            let s = n.layout(ctx, wl);
                            let child_size = align.layout(s, constrains, wl);
                            size = size.max(child_size);
                            true
                        });
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

                self.children.for_each_mut(|_, n| {
                    wl.with_outer(n, false, |wlt, n| {
                        let s = n.with_context(|ctx| ctx.widget_info.bounds.outer_size()).unwrap();
                        let x = (size.width - s.width) * align_x;
                        let y = (size.height - s.height) * align_y;
                        wlt.translate(PxVector::new(x, y));
                    });
                    true
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
/// let text = z_stack(ui_list![
///     text("back"),
///     text("front"),
/// ]);
/// ```
///
/// # `z_stack!`
///
/// This function is just a shortcut for [`z_stack!`](module@z_stack). Use the full widget
/// to better configure the layering stack widget.
pub fn z_stack(children: impl UiNodeList) -> impl UiNode {
    z_stack! { children; }
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
    #[ui_node(struct StackNodesNode {
        children: impl UiNodeList,
    })]
    impl StackNodesNode {}

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
    #[ui_node(struct StackNodesFillNode {
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
                self.children.for_each(|_, n| {
                    let s = n.measure(ctx);
                    size = size.max(s);
                    true
                });
                size
            } else {
                let mut size = self.children.with_node(index, |n| n.measure(ctx));
                let constrains = (self.constrains)(ctx.peek(|m| m.constrains()), index, size);
                ctx.with_constrains(
                    |_| constrains,
                    |ctx| {
                        self.children.for_each(|i, n| {
                            if i != index {
                                size = size.max(n.measure(ctx));
                            }
                            true
                        });
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
                self.children.for_each_mut(|_, n| {
                    let s = n.layout(ctx, wl);
                    size = size.max(s);
                    true
                });
                size
            } else {
                let mut size = self.children.with_node_mut(index, |n| n.layout(ctx, wl));
                let constrains = (self.constrains)(ctx.peek(|m| m.constrains()), index, size);
                ctx.with_constrains(
                    |_| constrains,
                    |ctx| {
                        self.children.for_each_mut(|i, n| {
                            if i != index {
                                let s = n.layout(ctx, wl);
                                size = size.max(s);
                            }
                            true
                        });
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
