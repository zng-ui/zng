#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Wrap panel, properties and nodes.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::sync::Arc;

use crate_util::RecycleVec;
use zng_app::widget::node::PanelListRange;
use zng_ext_font::{BidiLevel, unicode_bidi_levels, unicode_bidi_sort};
use zng_layout::{
    context::{InlineConstraints, InlineConstraintsMeasure, InlineSegment, InlineSegmentPos, TextSegmentKind},
    unit::{GridSpacing, PxGridSpacing},
};
use zng_wgt::{
    node::{with_index_len_node, with_index_node, with_rev_index_node},
    prelude::*,
};
use zng_wgt_text::*;

mod crate_util;

/// Wrapping inline layout.
#[widget($crate::Wrap {
    ($children:expr) => {
        children = $children;
    };
})]
pub struct Wrap(WidgetBase);
impl Wrap {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|wgt| {
            let child = node(
                wgt.capture_ui_node_list_or_empty(property_id!(Self::children)),
                wgt.capture_var_or_else(property_id!(Self::spacing), || {
                    LINE_SPACING_VAR.map(|s| GridSpacing {
                        column: Length::zero(),
                        row: s.clone(),
                    })
                }),
                wgt.capture_var_or_else(property_id!(Self::children_align), || TEXT_ALIGN_VAR),
            );
            wgt.set_child(child);
        });
    }

    widget_impl! {
        /// Alignment of children in this widget and of nested wrap panels and texts.
        ///
        /// Note that this is only used for children alignment in this widget if [`children_align`] is not set on it.
        ///
        /// [`children_align`]: fn@children_align
        pub txt_align(align: impl IntoVar<Align>);

        /// Space in between rows of this widget and of nested wrap panels and texts.
        ///
        /// Note that this is only used for row spacing in this widget if [`spacing`] is not set on it.
        ///
        /// [`spacing`]: fn@spacing
        pub line_spacing(spacing: impl IntoVar<Length>);
    }
}

/// Inlined wrap items.
#[property(CHILD, capture, default(ui_vec![]), widget_impl(Wrap))]
pub fn children(children: impl UiNodeList) {}

/// Space in between items and rows.
///
/// Note that column space is limited for bidirectional inline items as it only inserts spacing between
/// items once and bidirectional text can interleave items, consider using [`word_spacing`] for inline text.
///
/// [`LINE_SPACING_VAR`]: zng_wgt_text::LINE_SPACING_VAR
/// [`line_spacing`]: fn@zng_wgt_text::txt_align
/// [`word_spacing`]: fn@zng_wgt_text::word_spacing
#[property(LAYOUT, capture, widget_impl(Wrap))]
pub fn spacing(spacing: impl IntoVar<GridSpacing>) {}

/// Children align.
#[property(LAYOUT, capture, widget_impl(Wrap))]
pub fn children_align(align: impl IntoVar<Align>) {}

/// Wrap node.
///
/// Can be used directly to inline widgets without declaring a wrap widget info. This node is the child
/// of the `Wrap!` widget.
pub fn node(children: impl UiNodeList, spacing: impl IntoVar<GridSpacing>, children_align: impl IntoVar<Align>) -> impl UiNode {
    let children = PanelList::new(children).track_info_range(*PANEL_LIST_ID);
    let spacing = spacing.into_var();
    let children_align = children_align.into_var();
    let mut layout = InlineLayout::default();

    match_node_list(children, move |children, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&spacing).sub_var_layout(&children_align);
        }
        UiNodeOp::Update { updates } => {
            let mut any = false;
            children.update_all(updates, &mut any);

            if any {
                WIDGET.layout();
            }
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let spacing = spacing.layout();
            children.delegated();
            *desired_size = layout.measure(wm, children.children(), children_align.get(), spacing);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let spacing = spacing.layout();
            children.delegated();
            // rust-analyzer does not find `layout` here if called with dot.
            *final_size = InlineLayout::layout(&mut layout, wl, children.children(), children_align.get(), spacing);
        }
        _ => {}
    })
}

/// Create a node that estimates the size of wrap panel children.
///
/// The estimation assumes that all items have a size of `child_size`.
pub fn lazy_size(children_len: impl IntoVar<usize>, spacing: impl IntoVar<GridSpacing>, child_size: impl IntoVar<Size>) -> impl UiNode {
    // we don't use `properties::size(NilUiNode, child_size)` because that size disables inlining.
    let size = child_size.into_var();
    let sample = match_node_leaf(move |op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&size);
        }
        UiNodeOp::Measure { desired_size, .. } => {
            *desired_size = size.layout();
        }
        UiNodeOp::Layout { final_size, .. } => {
            *final_size = size.layout();
        }
        _ => {}
    });

    lazy_sample(children_len, spacing, sample)
}

/// Create a node that estimates the size of wrap panel children.
///
/// The estimation assumes that all items have the size of `child_sample`.
pub fn lazy_sample(children_len: impl IntoVar<usize>, spacing: impl IntoVar<GridSpacing>, child_sample: impl UiNode) -> impl UiNode {
    let children_len = children_len.into_var();
    let spacing = spacing.into_var();

    match_node(child_sample, move |sample, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&children_len).sub_var_layout(&spacing);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let child_size = sample.measure(wm);
            *desired_size = InlineLayout::estimate_measure(wm, children_len.get(), child_size, spacing.layout());
        }
        UiNodeOp::Layout { wl, final_size } => {
            let child_size = sample.layout(wl);
            *final_size = InlineLayout::estimate_layout(wl, children_len.get(), child_size, spacing.layout());
        }
        _ => {}
    })
}

/// Info about segments of a widget in a row.
#[derive(Debug, Clone)]
enum ItemSegsInfo {
    Block(Px),
    Built {
        measure: Arc<Vec<InlineSegment>>,
        layout: Arc<Vec<InlineSegmentPos>>,
        x: f32,
        width: f32,
    },
}
impl ItemSegsInfo {
    pub fn new_collapsed() -> Self {
        Self::Block(Px(0))
    }

    pub fn new_block(width: Px) -> Self {
        Self::Block(width)
    }

    pub fn new_inlined(measure: Arc<Vec<InlineSegment>>) -> Self {
        Self::Built {
            measure,
            layout: Arc::new(vec![]),
            x: 0.0,
            width: 0.0,
        }
    }

    pub fn measure(&self) -> &[InlineSegment] {
        match self {
            ItemSegsInfo::Built { measure, .. } => measure,
            _ => &[],
        }
    }

    pub fn layout_mut(&mut self) -> &mut Vec<InlineSegmentPos> {
        self.build();
        match self {
            ItemSegsInfo::Built { measure, layout, .. } => {
                // Borrow checker limitation does not allow `if let Some(l) = Arc::get_mut(..) { l } else { <insert-return> }`

                if Arc::get_mut(layout).is_none() {
                    *layout = Arc::new(vec![]);
                }

                let r = Arc::get_mut(layout).unwrap();
                r.resize(measure.len(), InlineSegmentPos { x: 0.0 });

                r
            }
            _ => unreachable!(),
        }
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&InlineSegment, &mut InlineSegmentPos)> {
        self.build();
        match self {
            ItemSegsInfo::Built { measure, layout, .. } => {
                if Arc::get_mut(layout).is_none() {
                    *layout = Arc::new(vec![]);
                }

                let r = Arc::get_mut(layout).unwrap();
                r.resize(measure.len(), InlineSegmentPos { x: 0.0 });

                measure.iter().zip(r)
            }
            _ => unreachable!(),
        }
    }

    /// Only valid if has bidi items and is up-to-date.
    pub fn x_width_segs(&self) -> (Px, Px, Arc<Vec<InlineSegmentPos>>) {
        match self {
            ItemSegsInfo::Built { layout, x, width, .. } => (Px(x.floor() as i32), Px(width.ceil() as i32), layout.clone()),
            _ => unreachable!(),
        }
    }

    #[cfg(debug_assertions)]
    pub fn measure_width(&self) -> f32 {
        match self {
            ItemSegsInfo::Block(w) => w.0 as f32,
            ItemSegsInfo::Built { measure, .. } => measure.iter().map(|s| s.width).sum(),
        }
    }

    fn build(&mut self) {
        match self {
            ItemSegsInfo::Block(width) => {
                let width = width.0 as f32;
                *self = ItemSegsInfo::Built {
                    measure: Arc::new(vec![InlineSegment {
                        width,
                        kind: TextSegmentKind::OtherNeutral,
                    }]),
                    layout: Arc::new(Vec::with_capacity(1)),
                    x: 0.0,
                    width,
                }
            }
            ItemSegsInfo::Built { .. } => {}
        }
    }

    fn set_x_width(&mut self, new_x: f32, new_width: f32) {
        match self {
            ItemSegsInfo::Built { x, width, .. } => {
                *x = new_x;
                *width = new_width;
            }
            _ => unreachable!(),
        }
    }
}

/// Info about a row managed by wrap.
#[derive(Default, Debug, Clone)]
struct RowInfo {
    size: PxSize,
    first_child: usize,
    item_segs: Vec<ItemSegsInfo>,
}
impl crate::crate_util::Recycle for RowInfo {
    fn recycle(&mut self) {
        self.size = Default::default();
        self.first_child = Default::default();
        self.item_segs.clear();
    }
}

#[derive(Default)]
struct InlineLayout {
    first_wrapped: bool,
    rows: RecycleVec<RowInfo>,
    desired_size: PxSize,

    // has segments in the opposite direction, requires bidi sorting and positioning.
    has_bidi_inline: bool,
    bidi_layout_fresh: bool,
    // reused heap alloc
    bidi_sorted: Vec<usize>,
    bidi_levels: Vec<BidiLevel>,
    bidi_default_segs: Arc<Vec<InlineSegmentPos>>,
}
impl InlineLayout {
    pub fn estimate_measure(wm: &mut WidgetMeasure, children_len: usize, child_size: PxSize, spacing: PxGridSpacing) -> PxSize {
        if children_len == 0 {
            return PxSize::zero();
        }

        let metrics = LAYOUT.metrics();
        let constraints = metrics.constraints();

        if let (None, Some(known)) = (metrics.inline_constraints(), constraints.fill_or_exact()) {
            return known;
        }

        let max_x = constraints.x.max().unwrap_or(Px::MAX).max(child_size.width);

        if let Some(inline) = wm.inline() {
            let inline_constraints = metrics.inline_constraints().unwrap().measure();

            inline.first_wrapped = inline_constraints.first_max < child_size.width;

            let mut first_max_x = max_x;
            if !inline.first_wrapped {
                first_max_x = inline_constraints.first_max;
            }

            inline.first.height = child_size.height.max(inline_constraints.mid_clear_min);

            let column_len = (first_max_x - child_size.width) / (child_size.width + spacing.column) + Px(1);
            inline.first.width = (column_len - Px(1)) * (child_size.width + spacing.column) + child_size.width;

            let children_len = Px(children_len as _) - column_len;
            inline.last_wrapped = children_len.0 > 0;

            let mut size = inline.first;

            if inline.last_wrapped {
                let column_len = (max_x - child_size.width) / (child_size.width + spacing.column) + Px(1);

                size.width = size
                    .width
                    .max((column_len - Px(1)) * (child_size.width + spacing.column) + child_size.width);

                let mid_len = children_len / column_len;
                if mid_len.0 > 0 {
                    size.height += (spacing.row + child_size.height) * mid_len;
                }

                let last_len = children_len % column_len;
                inline.last.height = child_size.height;
                if last_len.0 > 0 {
                    inline.last.width = (last_len - Px(1)) * (child_size.width + spacing.column) + child_size.width;

                    size.height += spacing.row + child_size.height;
                } else {
                    inline.last.width = max_x;
                }
            } else {
                inline.last = inline.first;
            }

            debug_assert_eq!(inline.first.is_empty(), inline.last.is_empty());

            size
        } else {
            let column_len = (max_x - child_size.width) / (child_size.width + spacing.column) + Px(1);
            let row_len = (Px(children_len as i32) / column_len).max(Px(1));

            // spacing in between means space available to divide for pairs (width + column) has 1 less item.
            let desired_size = PxSize::new(
                (column_len - Px(1)) * (child_size.width + spacing.column) + child_size.width,
                (row_len - Px(1)) * (child_size.height + spacing.row) + child_size.height,
            );
            constraints.clamp_size(desired_size)
        }
    }

    pub fn measure(&mut self, wm: &mut WidgetMeasure, children: &mut PanelList, child_align: Align, spacing: PxGridSpacing) -> PxSize {
        let metrics = LAYOUT.metrics();
        let constraints = metrics.constraints();

        if let (None, Some(known)) = (metrics.inline_constraints(), constraints.fill_or_exact()) {
            return known;
        }

        self.measure_rows(wm, &metrics, children, child_align, spacing);

        if let Some(inline) = wm.inline() {
            inline.first_wrapped = self.first_wrapped;
            inline.last_wrapped = self.rows.len() > 1;

            if let Some(first) = self.rows.first() {
                inline.first = first.size;
                inline.with_first_segs(|i| {
                    i.extend(first.item_segs.iter().flat_map(|i| i.measure().iter().copied()));
                });
            } else {
                inline.first = PxSize::zero();
                inline.with_first_segs(|i| i.clear());
            }
            if let Some(last) = self.rows.last() {
                inline.last = last.size;
                inline.with_last_segs(|i| {
                    i.extend(last.item_segs.iter().flat_map(|i| i.measure().iter().copied()));
                })
            } else {
                inline.last = PxSize::zero();
                inline.with_last_segs(|i| i.clear());
            }
        }

        constraints.clamp_size(self.desired_size)
    }

    pub fn estimate_layout(wl: &mut WidgetLayout, children_len: usize, child_size: PxSize, spacing: PxGridSpacing) -> PxSize {
        let is_inline = wl.inline().is_some();
        let mut wm = wl.to_measure(if is_inline { Some(Default::default()) } else { None });
        let size = if let Some(inline) = wl.inline() {
            let mut size = Self::estimate_measure(&mut wm, children_len, child_size, spacing);
            if let Some(m_inline) = wm.inline() {
                inline.invalidate_negative_space();
                inline.inner_size = size;

                let inline_constraints = LAYOUT.inline_constraints().unwrap().layout();

                let mut mid_height = size.height;

                if !m_inline.first_wrapped {
                    inline.rows.push(inline_constraints.first);
                    mid_height -= child_size.height + spacing.row;
                }
                if m_inline.last_wrapped {
                    mid_height -= spacing.row + child_size.height;
                    inline.rows.push(PxRect::new(
                        PxPoint::new(Px(0), spacing.row + child_size.height),
                        PxSize::new(size.width, mid_height),
                    ));
                    inline.rows.push(inline_constraints.last);
                }

                size.height = inline_constraints.last.origin.y + inline_constraints.last.size.height;
            }
            size
        } else {
            Self::estimate_measure(&mut wm, children_len, child_size, spacing)
        };

        let width = LAYOUT.constraints().x.fill_or(size.width);
        PxSize::new(width, size.height)
    }

    pub fn layout(&mut self, wl: &mut WidgetLayout, children: &mut PanelList, child_align: Align, spacing: PxGridSpacing) -> PxSize {
        let metrics = LAYOUT.metrics();
        let inline_constraints = metrics.inline_constraints();
        let direction = metrics.direction();

        if inline_constraints.is_none() {
            // if not already measured by parent inline
            self.measure_rows(&mut wl.to_measure(None), &metrics, children, child_align, spacing);
        }
        if self.has_bidi_inline && !self.bidi_layout_fresh {
            self.layout_bidi(inline_constraints.clone(), direction, spacing.column);
        }

        let constraints = metrics.constraints();
        let child_align_x = child_align.x(direction);
        let child_align_y = child_align.y();

        let panel_width = constraints.x.fill_or(self.desired_size.width);

        let (first, mid, last) = if let Some(s) = inline_constraints.map(|c| c.layout()) {
            (s.first, s.mid_clear, s.last)
        } else {
            // define our own first and last
            let mut first = PxRect::from_size(self.rows[0].size);
            let mut last = PxRect::from_size(self.rows.last().unwrap().size);

            #[cfg(debug_assertions)]
            if self.has_bidi_inline {
                let segs_max = self.rows[0]
                    .item_segs
                    .iter()
                    .map(|s| {
                        let (x, width, _) = s.x_width_segs();
                        x + width
                    })
                    .max()
                    .unwrap_or_default();

                if (first.width() - segs_max).abs() > Px(10) {
                    tracing::error!("align error, used width: {:?}, but segs max is: {:?}", first.width(), segs_max);
                }
            }

            first.origin.x = (panel_width - first.size.width) * child_align_x;
            last.origin.x = (panel_width - last.size.width) * child_align_x;
            last.origin.y = self.desired_size.height - last.size.height;

            if let Some(y) = constraints.y.fill_or_exact() {
                let align_y = (y - self.desired_size.height) * child_align_y;
                first.origin.y += align_y;
                last.origin.y += align_y;
            }

            (first, Px(0), last)
        };
        let panel_height = constraints.y.fill_or(last.origin.y - first.origin.y + last.size.height);

        let child_constraints = PxConstraints2d::new_unbounded().with_fill_x(true).with_max_x(panel_width);

        if let Some(inline) = wl.inline() {
            inline.rows.clear();
        }

        let fill_width = if child_align.is_fill_x() {
            Some(panel_width.0 as f32)
        } else {
            None
        };

        LAYOUT.with_constraints(child_constraints, || {
            let mut row = first;
            let mut row_segs = &self.rows[0].item_segs;
            let mut row_advance = Px(0);
            let mut next_row_i = 1;
            let mut row_segs_i_start = 0;

            let mut fill_scale = None;
            if let Some(mut f) = fill_width {
                if wl.is_inline() {
                    f = first.width().0 as f32;
                }
                fill_scale = Some(f / self.rows[0].size.width.0 as f32);
            }

            children.for_each(|i, child, o| {
                if next_row_i < self.rows.len() && self.rows[next_row_i].first_child == i {
                    // new row
                    if let Some(inline) = wl.inline() {
                        inline.rows.push(row);
                    }
                    if next_row_i == self.rows.len() - 1 {
                        row = last;
                    } else {
                        row.origin.y += row.size.height + spacing.row;
                        if next_row_i == 1 {
                            // clear first row
                            row.origin.y += mid;
                        }

                        row.size = self.rows[next_row_i].size;
                        row.origin.x = (panel_width - row.size.width) * child_align_x;
                    }
                    row_segs = &self.rows[next_row_i].item_segs;
                    row_segs_i_start = self.rows[next_row_i].first_child;
                    next_row_i += 1;
                    row_advance = Px(0);

                    fill_scale = None;
                    if let Some(mut f) = fill_width {
                        if wl.is_inline() || next_row_i < self.rows.len() {
                            if wl.is_inline() && next_row_i == self.rows.len() {
                                f = last.width().0 as f32;
                            }
                            // fill row, if it is not the last in a block layout
                            fill_scale = Some(f / self.rows[next_row_i - 1].size.width.0 as f32);
                        }
                    }
                }

                let (bidi_x, bidi_width, bidi_segs) = if self.has_bidi_inline {
                    row_segs[i - row_segs_i_start].x_width_segs()
                } else {
                    (Px(0), Px(0), self.bidi_default_segs.clone())
                };

                let child_inline = child
                    .with_context(WidgetUpdateMode::Ignore, || WIDGET.bounds().measure_inline())
                    .flatten();
                if let Some(child_inline) = child_inline {
                    let child_desired_size = child
                        .with_context(WidgetUpdateMode::Ignore, || WIDGET.bounds().measure_outer_size())
                        .unwrap_or_default();
                    if child_desired_size.is_empty() {
                        // collapsed, continue.
                        wl.collapse_child(i);
                        return;
                    }

                    let mut child_first = PxRect::from_size(child_inline.first);
                    let mut child_mid = Px(0);
                    let mut child_last = PxRect::from_size(child_inline.last);

                    if child_inline.last_wrapped {
                        // child wraps
                        debug_assert_eq!(self.rows[next_row_i].first_child, i + 1);

                        child_first.origin.x = row.origin.x + row_advance;
                        if let LayoutDirection::RTL = direction {
                            child_first.origin.x -= row_advance;
                        }
                        child_first.origin.y += (row.size.height - child_first.size.height) * child_align_y;
                        child_mid = (row.size.height - child_first.size.height).max(Px(0));
                        child_last.origin.y = child_desired_size.height - child_last.size.height;

                        if self.has_bidi_inline {
                            child_first.origin.x = row.origin.x + bidi_x;
                            child_first.size.width = bidi_width;
                        }

                        let next_row = if next_row_i == self.rows.len() - 1 {
                            last
                        } else {
                            let mut r = row;
                            r.origin.y += child_last.origin.y;
                            r.size = self.rows[next_row_i].size;
                            r.origin.x = (panel_width - r.size.width) * child_align_x;
                            r
                        };
                        child_last.origin.x = next_row.origin.x;
                        if let LayoutDirection::RTL = direction {
                            child_last.origin.x += next_row.size.width - child_last.size.width;
                        }
                        child_last.origin.y += (next_row.size.height - child_last.size.height) * child_align_y;

                        let (last_bidi_x, last_bidi_width, last_bidi_segs) = if self.has_bidi_inline {
                            self.rows[next_row_i].item_segs[0].x_width_segs()
                        } else {
                            (Px(0), Px(0), self.bidi_default_segs.clone())
                        };

                        if self.has_bidi_inline {
                            child_last.origin.x = next_row.origin.x + last_bidi_x;
                            child_last.size.width = last_bidi_width;
                        }

                        if let Some(s) = fill_scale {
                            child_first.size.width *= s;

                            // child wraps, so last is different row
                            if wl.is_inline() || next_row_i < self.rows.len() - 1 {
                                let mut f = fill_width.unwrap();
                                if wl.is_inline() && next_row_i == self.rows.len() - 1 {
                                    f = last.width().0 as f32;
                                }
                                fill_scale = Some(f / next_row.size.width.0 as f32);
                                let s = fill_scale.unwrap();
                                child_last.size.width *= s;
                            } else {
                                // only fill last row if Wrap! is nested/inlined
                                fill_scale = None;
                            }
                        }

                        let (_, define_ref_frame) =
                            wl.with_child(|wl| wl.layout_inline(child_first, child_mid, child_last, bidi_segs, last_bidi_segs, child));
                        o.child_offset = PxVector::new(Px(0), row.origin.y);
                        o.define_reference_frame = define_ref_frame;

                        // new row
                        if let Some(inline) = wl.inline() {
                            inline.rows.push(row);
                            child.with_context(WidgetUpdateMode::Ignore, || {
                                if let Some(inner) = WIDGET.bounds().inline() {
                                    if inner.rows.len() >= 3 {
                                        inline.rows.extend(inner.rows[1..inner.rows.len() - 1].iter().map(|r| {
                                            let mut r = *r;
                                            r.origin.y += row.origin.y;
                                            r
                                        }));
                                    }
                                } else {
                                    tracing::error!("child inlined in measure, but not in layout")
                                }
                            });
                        }
                        row = next_row;
                        row_advance = child_last.size.width + spacing.column;
                        row_segs = &self.rows[next_row_i].item_segs;
                        row_segs_i_start = self.rows[next_row_i].first_child - 1; // next row first item is also this widget
                        debug_assert_eq!(row_segs_i_start, i);
                        next_row_i += 1;
                    } else {
                        // child inlined, but fits in the row

                        let mut offset = PxVector::new(row_advance, Px(0));
                        if let LayoutDirection::RTL = direction {
                            offset.x = row.size.width - child_last.size.width - offset.x;
                        }
                        offset.y = (row.size.height - child_inline.first.height) * child_align_y;

                        let mut max_size = child_inline.first;

                        if self.has_bidi_inline {
                            max_size.width = bidi_width;
                            child_first.size.width = bidi_width;
                            child_last.size.width = bidi_width;
                        }

                        if let Some(s) = fill_scale {
                            child_first.size.width *= s;
                            child_last.size.width *= s;
                            max_size.width *= s;
                        }

                        let (_, define_ref_frame) = wl.with_child(|wl| {
                            LAYOUT.with_constraints(child_constraints.with_fill(false, false).with_max_size(max_size), || {
                                wl.layout_inline(child_first, child_mid, child_last, bidi_segs.clone(), bidi_segs, child)
                            })
                        });
                        o.child_offset = row.origin.to_vector() + offset;
                        if self.has_bidi_inline {
                            o.child_offset.x = row.origin.x + bidi_x;
                        }
                        o.define_reference_frame = define_ref_frame;

                        row_advance += child_last.size.width + spacing.column;
                    }
                } else {
                    // inline block
                    let max_width = if self.has_bidi_inline {
                        bidi_width
                    } else {
                        row.size.width - row_advance
                    };
                    let (size, define_ref_frame) = LAYOUT.with_constraints(
                        child_constraints.with_fill(false, false).with_max(max_width, row.size.height),
                        || wl.with_child(|wl| wl.layout_block(child)),
                    );
                    if size.is_empty() {
                        // collapsed, continue.
                        o.child_offset = PxVector::zero();
                        o.define_reference_frame = false;
                        return;
                    }

                    let mut offset = PxVector::new(row_advance, Px(0));
                    if let LayoutDirection::RTL = direction {
                        offset.x = row.size.width - size.width - offset.x;
                    }
                    offset.y = (row.size.height - size.height) * child_align_y;
                    o.child_offset = row.origin.to_vector() + offset;
                    if self.has_bidi_inline {
                        o.child_offset.x = row.origin.x + bidi_x;
                    }
                    o.define_reference_frame = define_ref_frame;
                    row_advance += size.width + spacing.column;
                }
            });

            if let Some(inline) = wl.inline() {
                // last row
                inline.rows.push(row);
            }
        });

        children.commit_data().request_render();

        constraints.clamp_size(PxSize::new(panel_width, panel_height))
    }

    fn measure_rows(
        &mut self,
        wm: &mut WidgetMeasure,
        metrics: &LayoutMetrics,
        children: &mut PanelList,
        child_align: Align,
        spacing: PxGridSpacing,
    ) {
        self.rows.begin_reuse();
        self.bidi_layout_fresh = false;

        self.first_wrapped = false;
        self.desired_size = PxSize::zero();
        self.has_bidi_inline = false;

        let direction = metrics.direction();
        let constraints = metrics.constraints();
        let inline_constraints = metrics.inline_constraints();
        let child_inline_constrain = constraints.x.max_or(Px::MAX);
        let child_constraints = PxConstraints2d::new_unbounded()
            .with_fill_x(child_align.is_fill_x())
            .with_max_x(child_inline_constrain);
        let mut row = self.rows.new_item();
        LAYOUT.with_constraints(child_constraints, || {
            children.for_each(|i, child, _| {
                let mut inline_constrain = child_inline_constrain;
                let mut wrap_clear_min = Px(0);
                if self.rows.is_empty() && !self.first_wrapped {
                    if let Some(InlineConstraints::Measure(InlineConstraintsMeasure {
                        first_max, mid_clear_min, ..
                    })) = inline_constraints
                    {
                        inline_constrain = first_max;
                        wrap_clear_min = mid_clear_min;
                    }
                }
                if inline_constrain < Px::MAX {
                    inline_constrain -= row.size.width;
                }

                let (inline, size) = wm.measure_inline(inline_constrain, row.size.height - spacing.row, child);

                let can_collapse = size.is_empty()
                    && match &inline {
                        Some(i) => i.first_segs.is_empty(),
                        None => true,
                    };
                if can_collapse {
                    row.item_segs.push(ItemSegsInfo::new_collapsed());
                    // collapsed, continue.
                    return;
                }

                if let Some(inline) = inline {
                    if !self.has_bidi_inline {
                        self.has_bidi_inline =
                            inline
                                .first_segs
                                .iter()
                                .chain(inline.last_segs.iter())
                                .any(|s| match s.kind.strong_direction() {
                                    Some(d) => d != direction,
                                    None => false,
                                });
                    }

                    // item mid-rows can be wider
                    self.desired_size.width = self.desired_size.width.max(size.width);

                    if inline.first_wrapped {
                        // wrap by us, detected by child
                        if row.size.is_empty() {
                            debug_assert!(self.rows.is_empty());
                            self.first_wrapped = true;
                        } else {
                            row.size.width -= spacing.column;
                            row.size.width = row.size.width.max(Px(0));
                            self.desired_size.width = self.desired_size.width.max(row.size.width);
                            self.desired_size.height += row.size.height + spacing.row;

                            self.rows.push_renew(&mut row);
                        }

                        row.size = inline.first;
                        row.first_child = i;
                    } else {
                        row.size.width += inline.first.width;
                        row.size.height = row.size.height.max(inline.first.height);
                    }
                    row.item_segs.push(ItemSegsInfo::new_inlined(inline.first_segs.clone()));

                    if inline.last_wrapped {
                        // wrap by child
                        self.desired_size.width = self.desired_size.width.max(row.size.width);
                        self.desired_size.height += size.height - inline.first.height;

                        self.rows.push_renew(&mut row);
                        row.size = inline.last;
                        row.size.width += spacing.column;
                        row.first_child = i + 1;
                        row.item_segs.push(ItemSegsInfo::new_inlined(inline.last_segs));
                    } else {
                        // child inlined, but fit in row
                        row.size.width += spacing.column;
                    }
                } else if size.width <= inline_constrain {
                    row.size.width += size.width + spacing.column;
                    row.size.height = row.size.height.max(size.height);
                    row.item_segs.push(ItemSegsInfo::new_block(size.width));
                } else {
                    // wrap by us
                    if row.size.is_empty() {
                        debug_assert!(self.rows.is_empty());
                        self.first_wrapped = true;
                    } else {
                        row.size.width -= spacing.column;
                        row.size.width = row.size.width.max(Px(0));
                        self.desired_size.width = self.desired_size.width.max(row.size.width);
                        self.desired_size.height += row.size.height.max(wrap_clear_min) + spacing.row;
                        self.rows.push_renew(&mut row);
                    }

                    row.size = size;
                    row.size.width += spacing.column;
                    row.first_child = i;
                    row.item_segs.push(ItemSegsInfo::new_block(size.width));
                }
            });
        });

        // last row
        row.size.width -= spacing.column;
        row.size.width = row.size.width.max(Px(0));
        self.desired_size.width = self.desired_size.width.max(row.size.width);
        self.desired_size.height += row.size.height; // no spacing because it's single line or already added in [^wrap by us]
        self.rows.push(row);

        self.rows.commit_reuse();

        #[cfg(debug_assertions)]
        for (i, row) in self.rows.iter().enumerate() {
            let width = row.size.width;
            let sum_width = row.item_segs.iter().map(|s| Px(s.measure_width() as i32)).sum::<Px>()
                + spacing.column * Px(row.item_segs.len().saturating_sub(1) as _);

            if (sum_width - width) > Px(1) {
                if metrics.inline_constraints().is_some() && (i == 0 || i == self.rows.len() - 1) {
                    tracing::error!(
                        "Wrap![{}] panel row {i} inline width is {width}, but sum of segs is {sum_width}",
                        WIDGET.id()
                    );
                    continue;
                }

                tracing::error!(
                    "Wrap![{}] panel row {i} computed width {width}, but sum of segs is {sum_width}",
                    WIDGET.id()
                );
            }
        }
    }

    fn layout_bidi(&mut self, constraints: Option<InlineConstraints>, direction: LayoutDirection, spacing_x: Px) {
        let spacing_x = spacing_x.0 as f32;
        let mut our_rows = 0..self.rows.len();

        if let Some(l) = constraints {
            let l = l.layout();
            our_rows = 0..0;

            if !self.rows.is_empty() {
                if l.first_segs.len() != self.rows[0].item_segs.iter().map(|s| s.measure().len()).sum::<usize>() {
                    // parent set first_segs empty (not sorted), or wrong
                    let mut x = 0.0;
                    for s in self.rows[0].item_segs.iter_mut() {
                        let mut spacing_x = spacing_x;
                        for (seg, pos) in s.iter_mut() {
                            pos.x = x;
                            x += seg.width + spacing_x;
                            spacing_x = 0.0;
                        }
                    }
                } else {
                    // parent set first_segs
                    for (pos, (_seg, seg_pos)) in l
                        .first_segs
                        .iter()
                        .zip(self.rows[0].item_segs.iter_mut().flat_map(|s| s.iter_mut()))
                    {
                        seg_pos.x = pos.x;
                    }
                }

                if self.rows.len() > 1 {
                    // last row not the same as first
                    let last_i = self.rows.len() - 1;
                    let last = &mut self.rows[last_i];
                    if l.last_segs.len() != last.item_segs.iter().map(|s| s.measure().len()).sum::<usize>() {
                        // parent set last_segs empty (not sorted), or wrong
                        let mut x = 0.0;
                        for s in last.item_segs.iter_mut() {
                            let mut spacing_x = spacing_x;
                            for (seg, pos) in s.iter_mut() {
                                pos.x = x;
                                x += seg.width + spacing_x;
                                spacing_x = 0.0;
                            }
                        }
                    } else {
                        // parent set last_segs
                        for (pos, (_seg, seg_pos)) in l.last_segs.iter().zip(last.item_segs.iter_mut().flat_map(|s| s.iter_mut())) {
                            seg_pos.x = pos.x;
                        }
                    }

                    if self.rows.len() > 2 {
                        our_rows = 1..self.rows.len() - 1;
                    }
                }
            }
        }

        for row in &mut self.rows[our_rows] {
            // rows we sort and set x

            unicode_bidi_levels(
                direction,
                row.item_segs.iter().flat_map(|i| i.measure().iter().map(|i| i.kind)),
                &mut self.bidi_levels,
            );

            unicode_bidi_sort(
                direction,
                row.item_segs
                    .iter()
                    .flat_map(|i| i.measure().iter().map(|i| i.kind))
                    .zip(self.bidi_levels.iter().copied()),
                0,
                &mut self.bidi_sorted,
            );

            let mut x = 0.0;

            let mut spacing_count = row.item_segs.len().saturating_sub(1);

            let mut last_item_i = usize::MAX;
            for &new_i in self.bidi_sorted.iter() {
                let mut segs_offset = 0;

                // `bidi_sorted` is flatten of `row.segs`
                for (i, s) in row.item_segs.iter_mut().enumerate() {
                    if segs_offset + s.measure().len() <= new_i {
                        segs_offset += s.measure().len();
                    } else {
                        let new_i = new_i - segs_offset;

                        if last_item_i != i {
                            last_item_i = i;
                            if x > 0.0 && spacing_count > 0 {
                                x += spacing_x;
                                spacing_count -= 1;
                            }
                        }

                        s.layout_mut()[new_i].x = x;
                        x += s.measure()[new_i].width;
                        break;
                    }
                }
            }
        }

        for row in self.rows.iter_mut() {
            // update seg.x and seg.width
            for seg in &mut row.item_segs {
                if seg.measure().is_empty() {
                    continue;
                }

                let mut seg_min = f32::MAX;
                let mut seg_max = f32::MIN;
                for (m, l) in seg.iter_mut() {
                    seg_min = seg_min.min(l.x);
                    seg_max = seg_max.max(l.x + m.width);
                }
                seg.set_x_width(seg_min, seg_max - seg_min);

                for (_, l) in seg.iter_mut() {
                    l.x -= seg_min;
                }
            }
        }
    }
}

static_id! {
    static ref PANEL_LIST_ID: StateId<PanelListRange>;
}

/// Get the child index in the parent wrap.
///
/// The child index is zero-based.
#[property(CONTEXT)]
pub fn get_index(child: impl UiNode, state: impl IntoVar<usize>) -> impl UiNode {
    let state = state.into_var();
    with_index_node(child, *PANEL_LIST_ID, move |id| {
        let _ = state.set(id.unwrap_or(0));
    })
}

/// Get the child index and number of children.
#[property(CONTEXT)]
pub fn get_index_len(child: impl UiNode, state: impl IntoVar<(usize, usize)>) -> impl UiNode {
    let state = state.into_var();
    with_index_len_node(child, *PANEL_LIST_ID, move |id_len| {
        let _ = state.set(id_len.unwrap_or((0, 0)));
    })
}

/// Get the child index, starting from the last child at `0`.
#[property(CONTEXT)]
pub fn get_rev_index(child: impl UiNode, state: impl IntoVar<usize>) -> impl UiNode {
    let state = state.into_var();
    with_rev_index_node(child, *PANEL_LIST_ID, move |id| {
        let _ = state.set(id.unwrap_or(0));
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
    with_index_node(child, *PANEL_LIST_ID, move |id| {
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
    with_index_node(child, *PANEL_LIST_ID, move |id| {
        let _ = state.set(id.map(|i| i % 2 != 0).unwrap_or(false));
    })
}

/// If the child is the first.
#[property(CONTEXT)]
pub fn is_first(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    let state = state.into_var();
    with_index_node(child, *PANEL_LIST_ID, move |id| {
        let _ = state.set(id == Some(0));
    })
}

/// If the child is the last.
#[property(CONTEXT)]
pub fn is_last(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    let state = state.into_var();
    with_rev_index_node(child, *PANEL_LIST_ID, move |id| {
        let _ = state.set(id == Some(0));
    })
}

/// Extension methods for [`WidgetInfo`] that may represent a [`Wrap!`] instance.
///
/// [`Wrap!`]: struct@Wrap
/// [`WidgetInfo`]: zng_app::widget::info::WidgetInfo
pub trait WidgetInfoWrapExt {
    /// Gets the wrap children, if this widget is a [`Wrap!`] instance.
    ///
    /// [`Wrap!`]: struct@Wrap
    fn wrap_children(&self) -> Option<zng_app::widget::info::iter::Children>;
}
impl WidgetInfoWrapExt for WidgetInfo {
    fn wrap_children(&self) -> Option<zng_app::widget::info::iter::Children> {
        PanelListRange::get(self, *PANEL_LIST_ID)
    }
}
