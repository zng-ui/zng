//! Stack widgets, properties and nodes.

use crate::prelude::new_widget::*;

mod types;
pub use types::*;

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
/// let text = Stack! {
///     direction = StackDirection::top_to_bottom();
///     padding = 10;
///     spacing = 5;
///     align = Align::CENTER;
///     children_align = Align::FILL;
///     children = ui_vec![
///         Text!("one"),
///         Text!("two"),
///         Text!("three"),
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
/// [`direction`]: fn@direction
/// [`z_index`]: fn@crate::prelude::z_index
#[widget($crate::widgets::layouts::Stack)]
pub struct Stack(WidgetBase);
impl Stack {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|wgt| {
            let child = node(
                wgt.capture_ui_node_list_or_empty(property_id!(Self::children)),
                wgt.capture_var_or_default(property_id!(Self::direction)),
                wgt.capture_var_or_default(property_id!(Self::spacing)),
                wgt.capture_var_or_else(property_id!(Self::children_align), || Align::FILL),
            );
            wgt.set_child(child);
        });
    }

    widget_impl! {
        /// Widget items.
        pub widget_base::children(children: impl UiNodeList);
    }
}

/// Stack direction.
#[property(LAYOUT, capture, widget_impl(Stack))]
pub fn direction(direction: impl IntoVar<StackDirection>) {}

/// Space in-between items.
///
/// The spacing is added along non-zero axis for each item offset after the first, so the spacing may
/// not always be in-between items if a non-standard [`direction`] is used.
///
/// [`direction`]: fn@direction
#[property(LAYOUT, capture, widget_impl(Stack))]
pub fn spacing(spacing: impl IntoVar<Length>) {}

/// Items alignment.
///
/// The items are aligned along axis that don't change, as defined by the [`direction`].
///
/// The default is [`FILL`].
///
/// [`FILL`]: Align::FILL
/// [`direction`]: fn@direction
#[property(LAYOUT, capture, default(Align::FILL), widget_impl(Stack))]
pub fn children_align(align: impl IntoVar<Align>) {}

/// Stack node.
///
/// Can be used directly to stack widgets without declaring a stack widget info. This node is the child
/// of the `Stack!` widget.
pub fn node(
    children: impl UiNodeList,
    direction: impl IntoVar<StackDirection>,
    spacing: impl IntoVar<Length>,
    children_align: impl IntoVar<Align>,
) -> impl UiNode {
    let children = PanelList::new(children).track_info_range(&PANEL_LIST_ID);
    let direction = direction.into_var();
    let spacing = spacing.into_var();
    let children_align = children_align.into_var();

    match_node_list(children, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_layout(&direction)
                .sub_var_layout(&spacing)
                .sub_var_layout(&children_align);
        }
        UiNodeOp::Update { updates } => {
            let mut changed = false;
            c.update_all(updates, &mut changed);

            if changed {
                WIDGET.layout();
            }
        }
        UiNodeOp::Measure { wm, desired_size } => {
            c.delegated();
            *desired_size = measure(wm, c.children(), direction.get(), spacing.get(), children_align.get());
        }
        UiNodeOp::Layout { wl, final_size } => {
            c.delegated();
            *final_size = layout(wl, c.children(), direction.get(), spacing.get(), children_align.get());
        }
        _ => {}
    })
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
    let children_len = children_len.into_var();
    let direction = direction.into_var();
    let spacing = spacing.into_var();

    match_node(child_sample, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_layout(&children_len)
                .sub_var_layout(&direction)
                .sub_var_layout(&spacing);
        }
        op @ UiNodeOp::Measure { .. } | op @ UiNodeOp::Layout { .. } => {
            let mut measure = |wm| {
                let constraints = LAYOUT.constraints();
                if let Some(known) = constraints.fill_or_exact() {
                    child.delegated();
                    return known;
                }

                let len = Px(children_len.get() as i32);
                if len.0 == 0 {
                    child.delegated();
                    return PxSize::zero();
                }

                let child_size = child.measure(wm);

                let direction = direction.get();
                let dv = direction.vector(LayoutDirection::LTR);
                let ds = if dv.x == 0 && dv.y != 0 {
                    // vertical stack
                    let spacing = spacing.layout_y();
                    PxSize::new(child_size.width, (len - Px(1)) * (child_size.height + spacing) + child_size.height)
                } else if dv.x != 0 && dv.y == 0 {
                    // horizontal stack
                    let spacing = spacing.layout_x();
                    PxSize::new((len - Px(1)) * (child_size.width + spacing) + child_size.width, child_size.height)
                } else {
                    // unusual stack
                    let spacing = spacing_from_direction(dv, spacing.get());

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

                constraints.fill_size_or(ds)
            };

            match op {
                UiNodeOp::Measure { wm, desired_size } => {
                    *desired_size = measure(wm);
                }
                UiNodeOp::Layout { wl, final_size } => {
                    *final_size = measure(&mut wl.to_measure(None));
                }
                _ => unreachable!(),
            }
        }
        _ => {}
    })
}

fn measure(wm: &mut WidgetMeasure, children: &mut PanelList, direction: StackDirection, spacing: Length, children_align: Align) -> PxSize {
    let metrics = LAYOUT.metrics();
    let constraints = metrics.constraints();
    if let Some(known) = constraints.fill_or_exact() {
        return known;
    }

    let child_align = direction.filter_align(children_align);

    let spacing = layout_spacing(&metrics, &direction, spacing);
    let max_size = child_max_size(wm, children, child_align);

    // layout children, size, raw position + spacing only.
    let mut item_bounds = euclid::Box2D::zero();
    LAYOUT.with_constraints(
        constraints
            .with_fill(child_align.is_fill_x(), child_align.is_fill_y())
            .with_max_size(max_size)
            .with_new_min(Px(0), Px(0)),
        || {
            // parallel measure full widgets first
            children.measure_each(
                wm,
                |_, c, _, wm| {
                    if c.is_widget() {
                        c.measure(wm)
                    } else {
                        PxSize::zero()
                    }
                },
                |_, _| PxSize::zero(),
            );

            let mut item_rect = PxRect::zero();
            let mut child_spacing = PxVector::zero();
            children.for_each(|_, c, _| {
                // already parallel measured widgets, only measure other nodes.
                let size = match c.with_context(WidgetUpdateMode::Ignore, || WIDGET.bounds().measure_outer_size()) {
                    Some(wgt_size) => wgt_size,
                    None => c.measure(wm),
                };
                if size.is_empty() {
                    return; // continue, skip collapsed
                }

                let offset = direction.layout(item_rect, size) + child_spacing;

                item_rect.origin = offset.to_point();
                item_rect.size = size;

                let item_box = item_rect.to_box2d();
                item_bounds.min = item_bounds.min.min(item_box.min);
                item_bounds.max = item_bounds.max.max(item_box.max);
                child_spacing = spacing;
            });
        },
    );

    constraints.fill_size_or(item_bounds.size())
}
fn layout(wl: &mut WidgetLayout, children: &mut PanelList, direction: StackDirection, spacing: Length, children_align: Align) -> PxSize {
    let metrics = LAYOUT.metrics();
    let constraints = metrics.constraints();
    let child_align = direction.filter_align(children_align);

    let spacing = layout_spacing(&metrics, &direction, spacing);
    let max_size = child_max_size(&mut wl.to_measure(None), children, child_align);

    // layout children, size, raw position + spacing only.
    let mut item_bounds = euclid::Box2D::zero();
    LAYOUT.with_constraints(
        constraints
            .with_fill(child_align.is_fill_x(), child_align.is_fill_y())
            .with_max_size(max_size)
            .with_new_min(Px(0), Px(0)),
        || {
            // parallel layout widgets
            children.layout_each(
                wl,
                |_, c, o, wl| {
                    if c.is_widget() {
                        let (size, define_ref_frame) = wl.with_child(|wl| c.layout(wl));
                        debug_assert!(!define_ref_frame); // is widget, should define own frame.
                        o.define_reference_frame = define_ref_frame;
                        size
                    } else {
                        PxSize::zero()
                    }
                },
                |_, _| PxSize::zero(),
            );

            // layout other nodes and position everything.
            let mut item_rect = PxRect::zero();
            let mut child_spacing = PxVector::zero();
            children.for_each(|_, c, o| {
                let size = match c.with_context(WidgetUpdateMode::Ignore, || WIDGET.bounds().outer_size()) {
                    Some(wgt_size) => wgt_size,
                    None => {
                        let (size, define_ref_frame) = wl.with_child(|wl| c.layout(wl));
                        o.define_reference_frame = define_ref_frame;
                        size
                    }
                };

                if size.is_empty() {
                    o.child_offset = PxVector::zero();
                    o.define_reference_frame = false;
                    return; // continue, skip collapsed
                }

                let offset = direction.layout(item_rect, size) + child_spacing;
                o.child_offset = offset;

                item_rect.origin = offset.to_point();
                item_rect.size = size;

                let item_box = item_rect.to_box2d();
                item_bounds.min = item_bounds.min.min(item_box.min);
                item_bounds.max = item_bounds.max.max(item_box.max);
                child_spacing = spacing;
            });
        },
    );

    // final position, align child inside item_bounds and item_bounds in the panel area.
    let child_align = child_align.xy(LAYOUT.direction());
    let items_size = item_bounds.size();
    let panel_size = constraints.fill_size_or(items_size);
    let children_offset = -item_bounds.min.to_vector() + (panel_size - items_size).to_vector() * children_align.xy(LAYOUT.direction());
    let align_baseline = children_align.is_baseline();

    children.for_each(|_, c, o| {
        if let Some((size, baseline)) = c.with_context(WidgetUpdateMode::Ignore, || {
            let bounds = WIDGET.bounds();
            (bounds.outer_size(), bounds.final_baseline())
        }) {
            let child_offset = (items_size - size).to_vector() * child_align;
            o.child_offset += children_offset + child_offset;

            if align_baseline {
                o.child_offset.y += baseline;
            }
        } else {
            // non-widgets only align with item_bounds
            o.child_offset += children_offset;
        }
    });

    panel_size
}

/// Spacing to add on each axis.
fn layout_spacing(ctx: &LayoutMetrics, direction: &StackDirection, spacing: Length) -> PxVector {
    let direction_vector = direction.vector(ctx.direction());
    spacing_from_direction(direction_vector, spacing)
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

/// Max size to layout each child with.
fn child_max_size(wm: &mut WidgetMeasure, children: &mut PanelList, child_align: Align) -> PxSize {
    let constraints = LAYOUT.constraints();

    // need measure when children fill, but the panel does not.
    let mut need_measure = false;
    let mut max_size = PxSize::zero();
    let mut measure_constraints = constraints;
    match (constraints.x.fill_or_exact(), constraints.y.fill_or_exact()) {
        (None, None) => {
            need_measure = child_align.is_fill_x() || child_align.is_fill_y();
            if !need_measure {
                max_size = constraints.max_size().unwrap_or_else(|| PxSize::new(Px::MAX, Px::MAX));
            }
        }
        (None, Some(h)) => {
            max_size.height = h;
            need_measure = child_align.is_fill_x();

            if need_measure {
                measure_constraints = constraints.with_fill_x(false);
            } else {
                max_size.width = Px::MAX;
            }
        }
        (Some(w), None) => {
            max_size.width = w;
            need_measure = child_align.is_fill_y();

            if need_measure {
                measure_constraints = constraints.with_fill_y(false);
            } else {
                max_size.height = Px::MAX;
            }
        }
        (Some(w), Some(h)) => max_size = PxSize::new(w, h),
    }

    // find largest child, the others will fill to its size.
    if need_measure {
        let max_items = LAYOUT.with_constraints(measure_constraints.with_new_min(Px(0), Px(0)), || {
            children.measure_each(wm, |_, c, _, wm| c.measure(wm), PxSize::max)
        });

        max_size = constraints.clamp_size(max_size.max(max_items));
    }

    max_size
}

/// Basic horizontal stack layout.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _scope = App::minimal();
/// let text = h_stack(ui_vec![
///     Text!("Hello "),
///     Text!("World"),
/// ]);
/// ```
///
/// # `Stack!`
///
/// This function is just a shortcut for [`Stack!`](struct@Stack) with [`StackDirection::left_to_right`].
pub fn h_stack(children: impl UiNodeList) -> impl UiNode {
    Stack! {
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
///     Text!("Hello "),
///     Text!("World"),
/// ]);
/// ```
///
/// # `Stack!`
///
/// This function is just a shortcut for [`Stack!`](struct@Stack) with [`StackDirection::top_to_bottom`].
pub fn v_stack(children: impl UiNodeList) -> impl UiNode {
    Stack! {
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
///     Text!("Hello "),
///     Text!("World"),
/// ]);
/// ```
///
/// # `Stack!`
///
/// This function is just a shortcut for [`Stack!`](struct@Stack) with [`StackDirection::none`].
pub fn z_stack(children: impl UiNodeList) -> impl UiNode {
    Stack! {
        children;
    }
}

/// Creates a node that updates and layouts the `nodes` in the logical order they appear in the list
/// and renders then on on top of the other from back(0) to front(len-1). The layout size is the largest item width and height,
/// the parent constraints are used for the layout of each item.
///
/// This is the most simple *z-stack* implementation possible, it is a building block useful for quickly declaring
/// overlaying effects composed of multiple nodes, it does not do any alignment layout or z-sorting render,
/// for a complete stack panel widget see [`Stack!`].
///
/// [`Stack!`]: struct@Stack
pub fn stack_nodes(nodes: impl UiNodeList) -> impl UiNode {
    match_node_list(nodes, |_, _| {})
}

/// Creates a node that updates the `nodes` in the logical order they appear, renders then on on top of the other from back(0) to front(len-1),
/// but layouts the `index` item first and uses its size to get `constraints` for the other items.
///
/// The layout size is the largest item width and height.
///
/// If the `index` is out of range the node logs an error and behaves like [`stack_nodes`].
pub fn stack_nodes_layout_by(
    nodes: impl UiNodeList,
    index: impl IntoVar<usize>,
    constraints: impl Fn(PxConstraints2d, usize, PxSize) -> PxConstraints2d + Send + 'static,
) -> impl UiNode {
    let index = index.into_var();
    #[cfg(dyn_closure)]
    let constraints: Box<dyn Fn(PxConstraints2d, usize, PxSize) -> PxConstraints2d + Send> = Box::new(constraints);

    match_node_list(nodes, move |children, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&index);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let index = index.get();
            let len = children.len();
            *desired_size = if index >= len {
                tracing::error!(
                    "index {} out of range for length {} in `{:?}#stack_nodes_layout_by`",
                    index,
                    len,
                    WIDGET.id()
                );

                children.measure_each(wm, |_, n, wm| n.measure(wm), PxSize::max)
            } else {
                let index_size = children.with_node(index, |n| n.measure(wm));
                let constraints = constraints(LAYOUT.metrics().constraints(), index, index_size);
                LAYOUT.with_constraints(constraints, || {
                    children.measure_each(
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
                })
            };
        }
        UiNodeOp::Layout { wl, final_size } => {
            let index = index.get();
            let len = children.len();
            *final_size = if index >= len {
                tracing::error!(
                    "index {} out of range for length {} in `{:?}#stack_nodes_layout_by`",
                    index,
                    len,
                    WIDGET.id()
                );

                children.layout_each(wl, |_, n, wl| n.layout(wl), PxSize::max)
            } else {
                let index_size = children.with_node(index, |n| n.layout(wl));
                let constraints = constraints(LAYOUT.metrics().constraints(), index, index_size);
                LAYOUT.with_constraints(constraints, || {
                    children.layout_each(
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
                })
            };
        }
        _ => {}
    })
}

static PANEL_LIST_ID: StaticStateId<zero_ui_core::widget_instance::PanelListRange> = StaticStateId::new_unique();

/// Get the child index for custom `when` expressions.
///
/// The child index is zero-based.
///
/// # Examples
///
/// This uses `get_index` to give every third button a different background.
///
/// ```
/// # use zero_ui::{prelude::*, properties::background_color, core::color::colors};
/// # let _scope = zero_ui::core::app::App::minimal();
/// # let _ =
/// Stack! {
///     direction = StackDirection::top_to_bottom();
///     spacing = 2;
///     children = (0..30).map(|i| Button! { child = Text!("Row {i}") }.boxed()).collect::<UiNodeVec>();
///     button::extend_style = style_fn!(|_| Style! {
///         when *#stack::get_index % 3 == 0 {
///             background_color = web_colors::DARK_GRAY;
///         }
///     });
/// }
/// # ;
/// ```
#[property(CONTEXT)]
pub fn get_index(child: impl UiNode, state: impl IntoVar<usize>) -> impl UiNode {
    let state = state.into_var();
    super::panel_nodes::with_index_node(child, &PANEL_LIST_ID, move |id| {
        let _ = state.set(id.unwrap_or(0));
    })
}

/// Get the child index and number of children.
#[property(CONTEXT)]
pub fn get_index_len(child: impl UiNode, state: impl IntoVar<(usize, usize)>) -> impl UiNode {
    let state = state.into_var();
    super::panel_nodes::with_index_len_node(child, &PANEL_LIST_ID, move |id_len| {
        let _ = state.set(id_len.unwrap_or((0, 0)));
    })
}

/// Get the child index, starting from the last child at `0`.
#[property(CONTEXT)]
pub fn get_rev_index(child: impl UiNode, state: impl IntoVar<usize>) -> impl UiNode {
    let state = state.into_var();
    super::panel_nodes::with_rev_index_node(child, &PANEL_LIST_ID, move |id| {
        let _ = state.set(id.unwrap_or(0));
    })
}

/// Get the child index as a factor of the total number of children.
#[property(CONTEXT, default(0.fct()))]
pub fn get_index_fct(child: impl UiNode, state: impl IntoVar<Factor>) -> impl UiNode {
    let state = state.into_var();
    super::panel_nodes::with_index_len_node(child, &PANEL_LIST_ID, move |id_len| {
        let (i, l) = id_len.unwrap_or((0, 0));
        if i == 0 || l == 0 {
            let _ = state.set(0.fct());
        } else {
            let _ = state.set((l as f32).fct() / (i as f32).fct());
        }
    })
}

/// If the child index is even.
///
/// Child index is zero-based, so the first is even, the next [`is_odd`].
///
/// [`is_odd`]: fn@is_odd
#[property(CONTEXT)]
pub fn is_even(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    let state = state.into_var();
    super::panel_nodes::with_index_node(child, &PANEL_LIST_ID, move |id| {
        let _ = state.set(id.map(|i| i % 2 == 0).unwrap_or(false));
    })
}

/// If the child index is odd.
///
/// Child index is zero-based, so the first [`is_even`], the next one is odd.
///
/// [`is_even`]: fn@is_even
#[property(CONTEXT)]
pub fn is_odd(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    let state = state.into_var();
    super::panel_nodes::with_index_node(child, &PANEL_LIST_ID, move |id| {
        let _ = state.set(id.map(|i| i % 2 != 0).unwrap_or(false));
    })
}

/// If the child is the first.
#[property(CONTEXT)]
pub fn is_first(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    let state = state.into_var();
    super::panel_nodes::with_index_node(child, &PANEL_LIST_ID, move |id| {
        let _ = state.set(id == Some(0));
    })
}

/// If the child is the last.
#[property(CONTEXT)]
pub fn is_last(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    let state = state.into_var();
    super::panel_nodes::with_rev_index_node(child, &PANEL_LIST_ID, move |id| {
        let _ = state.set(id == Some(0));
    })
}

/// Extension methods for [`WidgetInfo`] that may be a [`Stack!`] instance.
///
/// [`Stack!`]: struct@Stack
/// [`WidgetInfo`]: crate::core::widget_info::WidgetInfo
pub trait WidgetInfoStackExt {
    /// Gets the stack children, if this widget is a [`Stack!`] instance.
    ///
    /// [`Stack!`]: struct@Stack
    fn stack_children(&self) -> Option<crate::core::widget_info::iter::Children>;
}
impl WidgetInfoStackExt for crate::core::widget_info::WidgetInfo {
    fn stack_children(&self) -> Option<crate::core::widget_info::iter::Children> {
        crate::core::widget_instance::PanelListRange::get(self, &PANEL_LIST_ID)
    }
}
