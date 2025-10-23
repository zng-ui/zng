#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! Stack widgets, properties and nodes.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]

zng_wgt::enable_widget_macros!();

use zng_wgt::prelude::*;

mod types;
pub use types::*;

/// Stack layout.
///
/// Without [`direction`] this is a Z layering stack, with direction the traditional vertical and horizontal *stack panels*
/// can be created, other custom layouts are also supported, diagonal stacks, partially layered stacks and more. See
/// [`StackDirection`] for more details.
///
/// # Z-Index
///
/// By default the widgets are rendered in their logical order, the last widget renders in front of the others,
/// you can change this by setting the [`z_index`] property in the item widget.
///
/// # Shorthand
///
/// The `Stack!` macro provides shorthand syntax:
///
/// * `Stack!($children:expr)` creates a Z stack.
/// * `Stack!($direction:ident, $children:expr)` create stack on the given direction. The first parameter is
///   the name of one of the [`LayoutDirection`] associated functions.
/// * `Stack!($direction:ident, $spacing:expr, $children:expr)` create stack with the given direction, spacing between items and the items.
/// * `Stack!($direction:expr, $children:expr)` create stack on the given direction. The first parameter is an expression of
/// type [`LayoutDirection`]. Note that to avoid conflict with the alternative (`$direction:ident`) you can use braces `{my_direction}`.
/// * `Stack!($direction:expr, $spacing:expr, $children:expr)` create stack with the given direction expression, spacing between items
/// and the items.
///
/// # `stack_nodes`
///
/// If you only want to create an overlaying effect composed of multiple nodes you can use the [`stack_nodes`] function.
///
/// [`stack_nodes`]: fn@stack_nodes
///
/// [`direction`]: fn@direction
/// [`z_index`]: fn@zng_wgt::z_index
/// [`LayoutDirection`]: zng_wgt::prelude::LayoutDirection
#[widget($crate::Stack {
    ($children:expr) => {
        children = $children;
    };
    ($direction:ident, $children:expr) => {
        direction = $crate::StackDirection::$direction();
        children = $children;
    };
    ($direction:ident, $spacing:expr, $children:expr) => {
        direction = $crate::StackDirection::$direction();
        spacing = $spacing;
        children = $children;
    };
    ($direction:expr, $children:expr) => {
        direction = $direction;
        children = $children;
    };
    ($direction:expr, $spacing:expr, $children:expr) => {
        direction = $direction;
        spacing = $spacing;
        children = $children;
    };
})]
pub struct Stack(WidgetBase);
impl Stack {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|wgt| {
            let child = node(
                wgt.capture_ui_node_or_nil(property_id!(Self::children)),
                wgt.capture_var_or_default(property_id!(Self::direction)),
                wgt.capture_var_or_default(property_id!(Self::spacing)),
                wgt.capture_var_or_else(property_id!(Self::children_align), || Align::FILL),
            );
            wgt.set_child(child);
        });
    }
}

/// Stack items.
#[property(CHILD, default(ui_vec![]), widget_impl(Stack))]
pub fn children(wgt: &mut WidgetBuilding, children: impl IntoUiNode) {
    let _ = children;
    wgt.expect_property_capture();
}

/// Stack direction.
#[property(LAYOUT, widget_impl(Stack))]
pub fn direction(wgt: &mut WidgetBuilding, direction: impl IntoVar<StackDirection>) {
    let _ = direction;
    wgt.expect_property_capture();
}

/// Space in-between items.
///
/// The spacing is added along non-zero axis for each item offset after the first, the spacing is
/// scaled by the [direction factor].
///
/// [`direction`]: fn@direction
/// [direction factor]: StackDirection::direction_factor
#[property(LAYOUT, widget_impl(Stack))]
pub fn spacing(wgt: &mut WidgetBuilding, spacing: impl IntoVar<Length>) {
    let _ = spacing;
    wgt.expect_property_capture();
}

/// Items alignment.
///
/// The items are aligned along axis that don't change, as defined by the [`direction`].
///
/// The default is [`FILL`].
///
/// [`FILL`]: Align::FILL
/// [`direction`]: fn@direction
#[property(LAYOUT, default(Align::FILL), widget_impl(Stack))]
pub fn children_align(wgt: &mut WidgetBuilding, align: impl IntoVar<Align>) {
    let _ = align;
    wgt.expect_property_capture();
}

/// Stack node.
///
/// Can be used directly to stack widgets without declaring a stack widget info. This node is the child
/// of the `Stack!` widget.
pub fn node(
    children: impl IntoUiNode,
    direction: impl IntoVar<StackDirection>,
    spacing: impl IntoVar<Length>,
    children_align: impl IntoVar<Align>,
) -> UiNode {
    let children = PanelList::new(children).track_info_range(*PANEL_LIST_ID);
    let direction = direction.into_var();
    let spacing = spacing.into_var();
    let children_align = children_align.into_var();

    match_node(children, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_layout(&direction)
                .sub_var_layout(&spacing)
                .sub_var_layout(&children_align);
        }
        UiNodeOp::Update { updates } => {
            let mut changed = false;
            c.update_list(updates, &mut changed);

            if changed {
                WIDGET.layout();
            }
        }
        UiNodeOp::Measure { wm, desired_size } => {
            c.delegated();
            *desired_size = measure(wm, c.node_impl::<PanelList>(), direction.get(), spacing.get(), children_align.get());
        }
        UiNodeOp::Layout { wl, final_size } => {
            c.delegated();
            *final_size = layout(wl, c.node_impl::<PanelList>(), direction.get(), spacing.get(), children_align.get());
        }
        _ => {}
    })
}

/// Create a node that estimates the size of stack panel children.
///
/// The estimation assumes that all items have a size of `child_size`.
pub fn lazy_size(
    children_len: impl IntoVar<usize>,
    direction: impl IntoVar<StackDirection>,
    spacing: impl IntoVar<Length>,
    child_size: impl IntoVar<Size>,
) -> UiNode {
    lazy_sample(
        children_len,
        direction,
        spacing,
        zng_wgt_size_offset::size(UiNode::nil(), child_size),
    )
}

/// Create a node that estimates the size of stack panel children.
///
/// The estimation assumes that all items have the size of `child_sample`.
pub fn lazy_sample(
    children_len: impl IntoVar<usize>,
    direction: impl IntoVar<StackDirection>,
    spacing: impl IntoVar<Length>,
    child_sample: impl IntoUiNode,
) -> UiNode {
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
                let constraints = LAYOUT.constraints().inner();
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
                let dv = direction.direction_factor(LayoutDirection::LTR);
                let ds = if dv.x == 0.fct() && dv.y != 0.fct() {
                    // vertical stack
                    let spacing = spacing.layout_y();
                    PxSize::new(child_size.width, (len - Px(1)) * (child_size.height + spacing) + child_size.height)
                } else if dv.x != 0.fct() && dv.y == 0.fct() {
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
    if let Some(known) = constraints.inner().fill_or_exact() {
        return known;
    }

    let child_align = children_align * direction.direction_scale();

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
            children.measure_list(
                wm,
                |_, c, _, wm| {
                    if c.as_widget().is_some() { c.measure(wm) } else { PxSize::zero() }
                },
                |_, _| PxSize::zero(),
            );

            let mut item_rect = PxRect::zero();
            let mut child_spacing = PxVector::zero();
            children.for_each_child(|_, c, _| {
                let size = match c.as_widget() {
                    // already parallel measured widgets, only measure other nodes.
                    Some(mut w) => w.with_context(WidgetUpdateMode::Ignore, || WIDGET.bounds().measure_outer_size()),
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

    constraints.inner().fill_size_or(item_bounds.size())
}
fn layout(wl: &mut WidgetLayout, children: &mut PanelList, direction: StackDirection, spacing: Length, children_align: Align) -> PxSize {
    let metrics = LAYOUT.metrics();
    let constraints = metrics.constraints();
    let child_align = children_align * direction.direction_scale();

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
            children.layout_list(
                wl,
                |_, c, o, wl| {
                    if c.as_widget().is_some() {
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
            children.for_each_child(|_, c, o| {
                let size = match c.as_widget() {
                    Some(mut w) => w.with_context(WidgetUpdateMode::Ignore, || WIDGET.bounds().outer_size()),
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
    let items_size = item_bounds.size();
    let panel_size = constraints.inner().fill_size_or(items_size);
    let children_offset = -item_bounds.min.to_vector() + (panel_size - items_size).to_vector() * children_align.xy(LAYOUT.direction());
    let align_baseline = children_align.is_baseline();
    let child_align = child_align.xy(LAYOUT.direction());

    children.for_each_child(|_, c, o| {
        match c.as_widget() {
            Some(mut w) => {
                let (size, baseline) = w.with_context(WidgetUpdateMode::Ignore, || {
                    let bounds = WIDGET.bounds();
                    (bounds.outer_size(), bounds.final_baseline())
                });
                let child_offset = (items_size - size).to_vector() * child_align;
                o.child_offset += children_offset + child_offset;

                if align_baseline {
                    o.child_offset.y += baseline;
                }
            }
            None => {
                // non-widgets only align with item_bounds
                o.child_offset += children_offset;
            }
        }
    });

    children.commit_data().request_render();

    panel_size
}

/// Spacing to add on each axis.
fn layout_spacing(ctx: &LayoutMetrics, direction: &StackDirection, spacing: Length) -> PxVector {
    let factor = direction.direction_factor(ctx.direction());
    spacing_from_direction(factor, spacing)
}
fn spacing_from_direction(factor: Factor2d, spacing: Length) -> PxVector {
    PxVector::new(spacing.layout_x(), spacing.layout_y()) * factor
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
            children.measure_list(wm, |_, c, _, wm| c.measure(wm), PxSize::max)
        });

        max_size = constraints.clamp_size(max_size.max(max_items));
    }

    max_size
}

/// Basic Z-stack node sized by one of the items.
///
/// Creates a node that updates the `nodes` in the logical order they appear, renders them one on top of the other from back(0)
/// to front(len-1), but layouts the `index` item first and uses its size to get `constraints` for the other items.
///
/// The layout size is the largest item width and height, usually the `index` size.
///
/// Note that if you don't need a custom `index` you can just use [`UiVec`] as a node directly, it implements basic Z-stack layout by default.
pub fn stack_nodes(
    nodes: impl IntoUiNode,
    index: impl IntoVar<usize>,
    constraints: impl Fn(PxConstraints2d, usize, PxSize) -> PxConstraints2d + Send + 'static,
) -> UiNode {
    stack_nodes_impl(nodes.into_node(), index, constraints)
}

fn stack_nodes_impl(
    nodes: UiNode,
    index: impl IntoVar<usize>,
    constraints: impl Fn(PxConstraints2d, usize, PxSize) -> PxConstraints2d + Send + 'static,
) -> UiNode {
    let nodes = nodes.into_list();
    let index = index.into_var();

    match_node(nodes, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&index);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let index = index.get();
            let len = c.node().children_len();
            *desired_size = if index >= len {
                tracing::error!("index {} out of range for length {} in `{:?}#stack_nodes`", index, len, WIDGET.id());

                c.measure(wm)
            } else {
                c.delegated();
                let index_size = c.node().with_child(index, |n| n.measure(wm));
                let constraints = constraints(LAYOUT.metrics().constraints(), index, index_size);
                LAYOUT.with_constraints(constraints, || {
                    c.measure_list(
                        wm,
                        |i, n, wm| {
                            if i != index { n.measure(wm) } else { index_size }
                        },
                        PxSize::max,
                    )
                })
            };
        }
        UiNodeOp::Layout { wl, final_size } => {
            let index = index.get();
            let len = c.node().children_len();
            *final_size = if index >= len {
                tracing::error!(
                    "index {} out of range for length {} in `{:?}#stack_nodes_layout_by`",
                    index,
                    len,
                    WIDGET.id()
                );

                c.layout(wl)
            } else {
                c.delegated();
                let index_size = c.node().with_child(index, |n| n.layout(wl));
                let constraints = constraints(LAYOUT.metrics().constraints(), index, index_size);
                LAYOUT.with_constraints(constraints, || {
                    c.layout_list(
                        wl,
                        |i, n, wl| {
                            if i != index { n.layout(wl) } else { index_size }
                        },
                        PxSize::max,
                    )
                })
            };
        }
        _ => {}
    })
}

static_id! {
    static ref PANEL_LIST_ID: StateId<zng_app::widget::node::PanelListRange>;
}

/// Get the child index in the parent stack.
///
/// The child index is zero-based.
#[property(CONTEXT)]
pub fn get_index(child: impl IntoUiNode, state: impl IntoVar<usize>) -> UiNode {
    let state = state.into_var();
    zng_wgt::node::with_index_node(child, *PANEL_LIST_ID, move |id| {
        state.set(id.unwrap_or(0));
    })
}

/// Get the child index and number of children.
#[property(CONTEXT)]
pub fn get_index_len(child: impl IntoUiNode, state: impl IntoVar<(usize, usize)>) -> UiNode {
    let state = state.into_var();
    zng_wgt::node::with_index_len_node(child, *PANEL_LIST_ID, move |id_len| {
        state.set(id_len.unwrap_or((0, 0)));
    })
}

/// Get the child index, starting from the last child at `0`.
#[property(CONTEXT)]
pub fn get_rev_index(child: impl IntoUiNode, state: impl IntoVar<usize>) -> UiNode {
    let state = state.into_var();
    zng_wgt::node::with_rev_index_node(child, *PANEL_LIST_ID, move |id| {
        state.set(id.unwrap_or(0));
    })
}

/// If the child index is even.
///
/// Child index is zero-based, so the first is even, the next [`is_odd`].
///
/// [`is_odd`]: fn@is_odd
#[property(CONTEXT)]
pub fn is_even(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    let state = state.into_var();
    zng_wgt::node::with_index_node(child, *PANEL_LIST_ID, move |id| {
        state.set(id.map(|i| i % 2 == 0).unwrap_or(false));
    })
}

/// If the child index is odd.
///
/// Child index is zero-based, so the first [`is_even`], the next one is odd.
///
/// [`is_even`]: fn@is_even
#[property(CONTEXT)]
pub fn is_odd(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    let state = state.into_var();
    zng_wgt::node::with_index_node(child, *PANEL_LIST_ID, move |id| {
        state.set(id.map(|i| i % 2 != 0).unwrap_or(false));
    })
}

/// If the child is the first.
#[property(CONTEXT)]
pub fn is_first(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    let state = state.into_var();
    zng_wgt::node::with_index_node(child, *PANEL_LIST_ID, move |id| {
        state.set(id == Some(0));
    })
}

/// If the child is the last.
#[property(CONTEXT)]
pub fn is_last(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    let state = state.into_var();
    zng_wgt::node::with_rev_index_node(child, *PANEL_LIST_ID, move |id| {
        state.set(id == Some(0));
    })
}

/// Extension methods for [`WidgetInfo`] that may represent a [`Stack!`] instance.
///
/// [`Stack!`]: struct@Stack
/// [`WidgetInfo`]: zng_app::widget::info::WidgetInfo
pub trait WidgetInfoStackExt {
    /// Gets the stack children, if this widget is a [`Stack!`] instance.
    ///
    /// [`Stack!`]: struct@Stack
    fn stack_children(&self) -> Option<zng_app::widget::info::iter::Children>;
}
impl WidgetInfoStackExt for zng_app::widget::info::WidgetInfo {
    fn stack_children(&self) -> Option<zng_app::widget::info::iter::Children> {
        zng_app::widget::node::PanelListRange::get(self, *PANEL_LIST_ID)
    }
}
