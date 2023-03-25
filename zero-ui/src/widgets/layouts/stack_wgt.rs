use crate::prelude::new_widget::*;

mod direction;
use crate::core::task::parking_lot::Mutex;
use direction::*;

/// Stack layout.
///
/// Without [`direction`] this is a Z layering stack, with direction the traditional vertical and horizontal *stack panels*
/// can be recreated, other custom layouts are also supported, diagonal stacks, partially layered stacks and more. See
/// [`StackDirection`] for more details.
///
/// # Z-Index
///
/// By default the widgets are rendered in their logical order, the last widget renders in front of the others,
/// you can change this by setting the [`z_index`] property in the item widget.
///
/// # Examples
///
/// The example creates a stack that positions each child under the previous one, in a vertical column. A space of 10
/// is reserved around the children and a space of 5 in between each child. The stack is centralized in the parent
/// widget, but the children fill the width of the widest child.
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// let text = stack! {
///     direction = StackDirection::top_to_bottom();
///     padding = 10;
///     spacing = 5;
///     align = Align::CENTER;
///     children_align = Align::FILL;
///     children = ui_vec![
///         text!("one"),
///         text!("two"),
///         text!("three"),
///     ];
/// };
/// ```
///
/// # `stack_nodes`
///
/// If you only want to create an overlaying effect composed of multiple nodes you can use the [`stack_nodes`] function.
///
/// [`stack_nodes`]: fn@stack_nodes
///
/// [`direction`]: fn@stack::direction
/// [`StackDirection`]: stack::StackDirection
/// [`z_index`]: fn@crate::prelude::z_index
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
            let child = node(
                wgt.capture_ui_node_list_or_empty(property_id!(self::children)),
                wgt.capture_var_or_default(property_id!(self::direction)),
                wgt.capture_var_or_default(property_id!(self::spacing)),
                wgt.capture_var_or_else(property_id!(self::children_align), || Align::FILL),
            );
            wgt.set_child(child);
        });
    }

    /// Stack node.
    ///
    /// Can be used directly to stack widgets without declaring a stack widget info. This node is the child
    /// of the `stack!` widget.
    pub fn node(
        children: impl UiNodeList,
        direction: impl IntoVar<StackDirection>,
        spacing: impl IntoVar<Length>,
        children_align: impl IntoVar<Align>,
    ) -> impl UiNode {
        StackNode {
            children: PanelList::new(children),
            direction: direction.into_var(),
            spacing: spacing.into_var(),
            children_align: children_align.into_var(),
        }
    }

    /// Create a node that estimates the size for a stack panel children where all items have the same `child_size`.
    pub fn lazy_size(
        children_len: impl IntoVar<usize>,
        direction: impl IntoVar<StackDirection>,
        spacing: impl IntoVar<Length>,
        child_size: impl IntoVar<Size>,
    ) -> impl UiNode {
        lazy_sample(children_len, direction, spacing, crate::properties::size(NilUiNode, child_size))
    }

    /// Create a node that estimates the size for a stack panel children where all items have the same size as `child_sample`.
    pub fn lazy_sample(
        children_len: impl IntoVar<usize>,
        direction: impl IntoVar<StackDirection>,
        spacing: impl IntoVar<Length>,
        child_sample: impl UiNode,
    ) -> impl UiNode {
        LazyStackNode {
            child: child_sample,
            children_len: children_len.into_var(),
            direction: direction.into_var(),
            spacing: spacing.into_var(),
        }
    }
}

#[ui_node(struct LazyStackNode {
    child: impl UiNode, // uses to estimate size.
    #[var] children_len: impl Var<usize>,
    #[var] direction: impl Var<StackDirection>,
    #[var] spacing: impl Var<Length>,
})]
impl UiNode for LazyStackNode {
    fn update(&mut self, updates: &WidgetUpdates) {
        if self.children_len.is_new() || self.direction.is_new() || self.spacing.is_new() {
            WIDGET.layout();
        }
        self.child.update(updates);
    }

    fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
        let constrains = LAYOUT.constrains();
        if let Some(known) = constrains.fill_or_exact() {
            return known;
        }

        let len = Px(self.children_len.get() as i32);
        if len.0 == 0 {
            return PxSize::zero();
        }

        let child_size = self.child.measure(wm);

        let direction = self.direction.get();
        let dv = direction.vector(LayoutDirection::LTR);
        let desired_size = if dv.x == 0 && dv.y != 0 {
            // vertical stack
            let spacing = self.spacing.layout_y();
            PxSize::new(child_size.width, (len - Px(1)) * (child_size.height + spacing) + child_size.height)
        } else if dv.x != 0 && dv.y == 0 {
            // horizontal stack
            let spacing = self.spacing.layout_x();
            PxSize::new((len - Px(1)) * (child_size.width + spacing) + child_size.width, child_size.height)
        } else {
            // unusual stack
            let spacing = spacing_from_direction(dv, self.spacing.get());

            let mut item_rect = PxRect::from_size(child_size);
            let mut item_bounds = euclid::Box2D::zero();
            let mut child_spacing = PxVector::zero();
            for _ in 0..len.0 {
                let offset = direction.layout(item_rect, child_size) + child_spacing;
                item_rect.origin = offset.to_point();
                let item_box = item_rect.to_box2d();
                item_bounds.min = item_bounds.min.min(item_box.min);
                item_bounds.max = item_bounds.max.max(item_box.max);
                child_spacing = spacing;
            }

            item_bounds.size()
        };

        constrains.fill_size_or(desired_size)
    }

    #[allow_(zero_ui::missing_delegate)]
    fn layout(&mut self, _: &mut WidgetLayout) -> PxSize {
        self.measure(&mut WidgetMeasure::new())
    }
}

#[ui_node(struct StackNode {
    children: PanelList,

    #[var] direction: impl Var<StackDirection>,
    #[var] spacing: impl Var<Length>,
    #[var] children_align: impl Var<Align>,
})]
impl StackNode {
    #[UiNode]
    fn update(&mut self, updates: &WidgetUpdates) {
        let mut changed = false;
        self.children.update_all(updates, &mut changed);

        if changed || self.direction.is_new() || self.spacing.is_new() || self.children_align.is_new() {
            WIDGET.layout().render();
        }
    }

    #[UiNode]
    fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
        let metrics = LAYOUT.metrics();
        let constrains = metrics.constrains();
        if let Some(known) = constrains.fill_or_exact() {
            return known;
        }

        let direction = self.direction.get();
        let children_align = self.children_align.get();
        let child_align = direction.filter_align(children_align);

        let spacing = self.layout_spacing(&metrics);
        let max_size = self.child_max_size(child_align);

        // layout children, size, raw position + spacing only.
        let mut item_bounds = euclid::Box2D::zero();
        LAYOUT.with_constrains(
            move |_| {
                constrains
                    .with_fill(child_align.is_fill_x(), child_align.is_fill_y())
                    .with_max_size(max_size)
                    .with_new_min(Px(0), Px(0))
            },
            || {
                let mut item_rect = PxRect::zero();
                let mut child_spacing = PxVector::zero();
                self.children.for_each(|_, c, _| {
                    let size = c.measure(wm);
                    if size.is_empty() {
                        return true; // continue, skip collapsed
                    }

                    let offset = direction.layout(item_rect, size) + child_spacing;

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
    fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
        let metrics = LAYOUT.metrics();
        let constrains = metrics.constrains();
        let direction = self.direction.get();
        let children_align = self.children_align.get();
        let child_align = direction.filter_align(children_align);

        let spacing = self.layout_spacing(&metrics);
        let max_size = self.child_max_size(child_align);

        // layout children, size, raw position + spacing only.
        let mut item_bounds = euclid::Box2D::zero();
        LAYOUT.with_constrains(
            move |_| {
                constrains
                    .with_fill(child_align.is_fill_x(), child_align.is_fill_y())
                    .with_max_size(max_size)
                    .with_new_min(Px(0), Px(0))
            },
            || {
                let mut item_rect = PxRect::zero();
                let mut child_spacing = PxVector::zero();
                self.children.for_each_mut(|_, c, o| {
                    let (size, define_ref_frame) = wl.with_child(|wl| c.layout(wl));

                    if size.is_empty() {
                        o.child_offset = PxVector::zero();
                        o.define_reference_frame = false;
                        return true; // continue, skip collapsed
                    }

                    let offset = direction.layout(item_rect, size) + child_spacing;
                    o.child_offset = offset;
                    o.define_reference_frame = define_ref_frame;

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
        let child_align = child_align.xy(LAYOUT.direction());
        let items_size = item_bounds.size();
        let panel_size = constrains.fill_size_or(items_size);
        let children_offset = -item_bounds.min.to_vector() + (panel_size - items_size).to_vector() * children_align.xy(LAYOUT.direction());
        let align_baseline = children_align.is_baseline();

        self.children.for_each_mut(|_, c, o| {
            let (size, baseline) = c
                .with_context(|| {
                    let bounds = WIDGET.bounds();
                    (bounds.outer_size(), bounds.final_baseline())
                })
                .unwrap_or_default();

            let child_offset = (items_size - size).to_vector() * child_align;
            o.child_offset += children_offset + child_offset;

            if align_baseline {
                o.child_offset.y += baseline;
            }

            true
        });

        panel_size
    }

    /// Spacing to add on each axis.
    fn layout_spacing(&self, ctx: &LayoutMetrics) -> PxVector {
        let direction_vector = self.direction.get().vector(ctx.direction());
        let spacing = self.spacing.get();
        spacing_from_direction(direction_vector, spacing)
    }

    /// Max size to layout each child with.
    fn child_max_size(&self, child_align: Align) -> PxSize {
        let constrains = LAYOUT.constrains();

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
            LAYOUT.with_constrains(
                move |_| measure_constrains.with_new_min(Px(0), Px(0)),
                || {
                    self.children.for_each(|_, c, _| {
                        let size = c.measure(&mut WidgetMeasure::new());
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

fn spacing_from_direction(direction_vector: euclid::Vector2D<i8, ()>, spacing: Length) -> PxVector {
    let mut spacing = match (direction_vector.x == 0, direction_vector.y == 0) {
        (false, false) => PxVector::new(spacing.layout_x(), spacing.layout_y()),
        (true, false) => PxVector::new(Px(0), spacing.layout_y()),
        (false, true) => PxVector::new(spacing.layout_x(), Px(0)),
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

/// Basic horizontal stack layout.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// let text = h_stack(ui_vec![
///     text!("Hello "),
///     text!("World"),
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
/// let text = v_stack(ui_vec![
///     text!("Hello "),
///     text!("World"),
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
/// let text = z_stack(ui_vec![
///     text!("Hello "),
///     text!("World"),
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

    StackNodesNode {
        // Mutex to enable parallel measure
        children: Mutex::new(nodes),
    }
    .cfg_boxed()
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
        fn update(&mut self, updates: &WidgetUpdates) {
            if self.index.is_new() {
                WIDGET.layout();
            }
            self.children.update_all(updates, &mut ());
        }

        fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
            let index = self.index.get();
            let len = self.children.len();
            if index >= len {
                tracing::error!(
                    "index {} out of range for length {} in `{:?}#stack_nodes_layout_by`",
                    index,
                    len,
                    WIDGET.id()
                );

                self.children.measure_each(wm, |_, n, wm| n.measure(wm), PxSize::max)
            } else {
                let index_size = self.children.with_node(index, |n| n.measure(wm));
                let constrains = (self.constrains)(LAYOUT.metrics().peek(|m| m.constrains()), index, index_size);
                LAYOUT.with_constrains(
                    |_| constrains,
                    || {
                        self.children.measure_each(
                            wm,
                            |i, n, wm| {
                                if i != index {
                                    n.measure(wm)
                                } else {
                                    index_size
                                }
                            },
                            PxSize::max,
                        )
                    },
                )
            }
        }
        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            let index = self.index.get();
            let len = self.children.len();
            if index >= len {
                tracing::error!(
                    "index {} out of range for length {} in `{:?}#stack_nodes_layout_by`",
                    index,
                    len,
                    WIDGET.id()
                );

                self.children.layout_each(wl, |_, n, wl| n.layout(wl), PxSize::max)
            } else {
                let index_size = self.children.with_node_mut(index, |n| n.layout(wl));
                let constrains = (self.constrains)(LAYOUT.metrics().peek(|m| m.constrains()), index, index_size);
                LAYOUT.with_constrains(
                    |_| constrains,
                    || {
                        self.children.layout_each(
                            wl,
                            |i, n, wl| {
                                if i != index {
                                    n.layout(wl)
                                } else {
                                    index_size
                                }
                            },
                            PxSize::max,
                        )
                    },
                )
            }
        }
    }
    StackNodesFillNode {
        children: Mutex::new(nodes),
        index: index.into_var(),
        constrains,
    }
    .cfg_boxed()
}
