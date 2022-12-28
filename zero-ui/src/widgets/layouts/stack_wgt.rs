use crate::prelude::new_widget::*;

mod direction;
use direction::*;

#[widget($crate::widgets::layouts::stack)]
pub mod stack {
    pub use super::direction::StackDirection;
    use super::*;

    inherit!(widget_base::base);

    properties! {
        /// Widget items.
        pub widget_base::children;

        /// Stack direction.
        pub direction(impl IntoVar<StackDirection>);

        /// Space in-between items.
        ///
        /// The spacing is added along non-zero axis for each item offset after the first, so the spacing may
        /// not always be in-between items if a non-standard [`direction`] is used.
        ///
        /// [`direction`]: fn@direction
        pub spacing(impl IntoVar<Length>);

        /// Spacing around the items stack, inside the border.
        pub crate::properties::padding;

        /// Items alignment.
        ///
        /// The items are aligned along axis that don't change, as defined by the [`direction`].
        ///
        /// The default is [`FILL`].
        ///
        /// [`FILL`]: Align::FILL
        /// [`direction`]: fn@direction
        pub children_align(impl IntoVar<Align>) = Align::FILL;
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(|wgt| {
            let children = wgt.capture_ui_node_list_or_empty(property_id!(self::children));
            let spacing = wgt.capture_var_or_default(property_id!(self::spacing));
            let direction = wgt.capture_var_or_default(property_id!(self::direction));
            let children_align = wgt.capture_var_or_else(property_id!(self::children_align), || Align::FILL);

            let node = StackNode {
                children: ZSortingList::new(children),
                direction,
                spacing,
                children_align,
            };
            let child = widget_base::nodes::children_layout(node);

            wgt.set_child(child);
        });
    }
}

#[ui_node(struct StackNode {
    children: impl UiNodeList,

    #[var] direction: impl Var<StackDirection>,
    #[var] spacing: impl Var<Length>,
    #[var] children_align: impl Var<Align>,
})]
impl StackNode {
    #[UiNode]
    fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
        let mut changed = false;
        self.children.update_all(ctx, updates, &mut changed);

        if changed || self.direction.is_new(ctx) || self.spacing.is_new(ctx) || self.children_align.is_new(ctx) {
            ctx.updates.layout_render();
        }
    }

    #[UiNode]
    fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
        let constrains = ctx.constrains();
        if let Some(known) = constrains.fill_or_exact() {
            return known;
        }

        let direction = self.direction.get();
        let children_align = self.children_align.get();
        let child_align = direction.filter_align(children_align);

        let spacing = self.layout_spacing(ctx);
        let max_size = self.child_max_size(ctx, child_align);

        // layout children, size, raw position + spacing only.
        let mut item_bounds = euclid::Box2D::zero();
        ctx.with_constrains(
            move |_| {
                constrains
                    .with_fill(child_align.is_fill_x(), child_align.is_fill_y())
                    .with_max_size(max_size)
                    .with_new_min(Px(0), Px(0))
            },
            |ctx| {
                let mut item_rect = PxRect::zero();
                let mut child_spacing = PxVector::zero();
                self.children.for_each(|_, c| {
                    let size = c.measure(ctx, wm);
                    if size.is_empty() {
                        return true; // continue, skip collapsed
                    }

                    let offset = direction.layout(ctx, item_rect, size) + child_spacing;

                    item_rect.origin = offset.to_point();
                    item_rect.size = size;

                    let item_box = item_rect.to_box2d();
                    item_bounds.min = item_bounds.min.min(item_box.min);
                    item_bounds.max = item_bounds.max.max(item_box.max);
                    child_spacing = spacing;

                    true
                });
            },
        );

        constrains.fill_size_or(item_bounds.size())
    }

    #[UiNode]
    fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
        let constrains = ctx.constrains();
        let direction = self.direction.get();
        let children_align = self.children_align.get();
        let child_align = direction.filter_align(children_align);

        let spacing = self.layout_spacing(ctx);
        let max_size = self.child_max_size(&mut ctx.as_measure(), child_align);

        // layout children, size, raw position + spacing only.
        let mut item_bounds = euclid::Box2D::zero();
        ctx.with_constrains(
            move |_| {
                constrains
                    .with_fill(child_align.is_fill_x(), child_align.is_fill_y())
                    .with_max_size(max_size)
                    .with_new_min(Px(0), Px(0))
            },
            |ctx| {
                let mut item_rect = PxRect::zero();
                let mut child_spacing = PxVector::zero();
                self.children.for_each_mut(|_, c| {
                    let size = c.layout(ctx, wl);
                    if size.is_empty() {
                        return true; // continue, skip collapsed
                    }

                    let offset = direction.layout(ctx, item_rect, size) + child_spacing;

                    wl.with_outer(c, false, |wl, _| wl.translate(offset));

                    item_rect.origin = offset.to_point();
                    item_rect.size = size;

                    let item_box = item_rect.to_box2d();
                    item_bounds.min = item_bounds.min.min(item_box.min);
                    item_bounds.max = item_bounds.max.max(item_box.max);
                    child_spacing = spacing;

                    true
                });
            },
        );

        // final position, align child inside item_bounds and item_bounds in the panel area.
        let child_align = child_align.xy(ctx.direction());
        let items_size = item_bounds.size();
        let panel_size = constrains.fill_size_or(items_size);
        let children_offset = -item_bounds.min.to_vector() + (panel_size - items_size).to_vector() * children_align.xy(ctx.direction());

        // !!: underline align?
        self.children.for_each_mut(|_, c| {
            let size = c.with_context(|ctx| ctx.widget_info.bounds.outer_size()).unwrap_or_default();
            let child_offset = (items_size - size).to_vector() * child_align;
            wl.with_outer(c, true, |wl, _| wl.translate(children_offset + child_offset));

            true
        });

        panel_size
    }

    /// Spacing to add on each axis.
    fn layout_spacing(&self, ctx: &LayoutMetrics) -> PxVector {
        let direction_vector = self.direction.get().vector(ctx.direction());

        let spacing = self.spacing.get();
        let mut spacing = match (direction_vector.x == 0, direction_vector.y == 0) {
            (false, false) => PxVector::new(spacing.layout(ctx.for_x(), |_| Px(0)), spacing.layout(ctx.for_y(), |_| Px(0))),
            (true, false) => PxVector::new(Px(0), spacing.layout(ctx.for_y(), |_| Px(0))),
            (false, true) => PxVector::new(spacing.layout(ctx.for_x(), |_| Px(0)), Px(0)),
            (true, true) => PxVector::zero(),
        };
        if direction_vector.x < 0 {
            spacing.x = -spacing.x;
        }
        if direction_vector.y < 0 {
            spacing.y = -spacing.y;
        }

        spacing
    }

    /// Max size to layout each child with.
    fn child_max_size(&self, ctx: &mut MeasureContext, child_align: Align) -> PxSize {
        let constrains = ctx.constrains();

        // need measure when children fill, but the panel does not.
        let mut need_measure = false;
        let mut max_size = PxSize::zero();
        let mut measure_constrains = constrains;
        match (constrains.x.fill_or_exact(), constrains.y.fill_or_exact()) {
            (None, None) => {
                need_measure = child_align.is_fill_x() || child_align.is_fill_y();
                if !need_measure {
                    max_size = constrains.max_size().unwrap_or_else(|| PxSize::new(Px::MAX, Px::MAX));
                }
            }
            (None, Some(h)) => {
                max_size.height = h;
                need_measure = child_align.is_fill_x();

                if need_measure {
                    measure_constrains = constrains.with_fill_x(false);
                } else {
                    max_size.width = Px::MAX;
                }
            }
            (Some(w), None) => {
                max_size.width = w;
                need_measure = child_align.is_fill_y();

                if need_measure {
                    measure_constrains = constrains.with_fill_y(false);
                } else {
                    max_size.height = Px::MAX;
                }
            }
            (Some(w), Some(h)) => max_size = PxSize::new(w, h),
        }

        // find largest child, the others will fill to its size.
        if need_measure {
            ctx.with_constrains(
                move |_| measure_constrains.with_new_min(Px(0), Px(0)),
                |ctx| {
                    self.children.for_each(|_, c| {
                        let size = c.measure(ctx, &mut WidgetMeasure::new());
                        max_size = max_size.max(size);
                        true
                    });
                },
            );

            max_size = constrains.clamp_size(max_size);
        }

        max_size
    }
}

/// Basic horizontal stack layout.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// let text = h_stack(ui_list![
///     text("Hello "),
///     text("World"),
/// ]);
/// ```
///
/// # `stack!`
///
/// This function is just a shortcut for [`stack!`](mod@stack) with [`StackDirection::left_to_right`].
pub fn h_stack(children: impl UiNodeList) -> impl UiNode {
    stack! {
        direction = StackDirection::left_to_right();
        children;
    }
}

/// Basic vertical stack layout.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// let text = v_stack(ui_list![
///     text("Hello "),
///     text("World"),
/// ]);
/// ```
///
/// # `stack!`
///
/// This function is just a shortcut for [`stack!`](mod@stack) with [`StackDirection::top_to_bottom`].
pub fn v_stack(children: impl UiNodeList) -> impl UiNode {
    stack! {
        direction = StackDirection::top_to_bottom();
        children;
    }
}

/// Basic layering stack layout.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// let text = z_stack(ui_list![
///     text("Hello "),
///     text("World"),
/// ]);
/// ```
///
/// # `stack!`
///
/// This function is just a shortcut for [`stack!`](mod@stack) with [`StackDirection::none`].
pub fn z_stack(children: impl UiNodeList) -> impl UiNode {
    stack! {
        children;
    }
}

/// Creates a node that updates and layouts the `nodes` in the logical order they appear in the list
/// and renders then on on top of the other from back(0) to front(len-1). The layout size is the largest item width and height,
/// the parent constrains are used for the layout of each item.
///
/// This is the most simple *z-stack* implementation possible, it is a building block useful for quickly declaring
/// overlaying effects composed of multiple nodes, it does not do any alignment layout or z-sorting render,
/// for a complete stack panel widget see [`stack!`].
///
/// [`stack!`]: mod@stack
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
    constrains: impl Fn(PxConstrains2d, usize, PxSize) -> PxConstrains2d + Send + 'static,
) -> impl UiNode {
    #[ui_node(struct StackNodesFillNode {
        children: impl UiNodeList,
        #[var] index: impl Var<usize>,
        constrains: impl Fn(PxConstrains2d, usize, PxSize) -> PxConstrains2d + Send + 'static,
    })]
    impl UiNode for StackNodesFillNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.index.is_new(ctx) {
                ctx.updates.layout();
            }
            self.children.update_all(ctx, updates, &mut ());
        }

        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
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
                    let s = n.measure(ctx, wm);
                    size = size.max(s);
                    true
                });
                size
            } else {
                let mut size = self.children.with_node(index, |n| n.measure(ctx, wm));
                let constrains = (self.constrains)(ctx.peek(|m| m.constrains()), index, size);
                ctx.with_constrains(
                    |_| constrains,
                    |ctx| {
                        self.children.for_each(|i, n| {
                            if i != index {
                                size = size.max(n.measure(ctx, wm));
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
