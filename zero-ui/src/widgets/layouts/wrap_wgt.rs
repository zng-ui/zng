use std::sync::Arc;

use crate::{crate_util::RecycleVec, prelude::new_widget::*};

use task::parking_lot::Mutex;

/// Wrapping inline layout.
#[widget($crate::widgets::layouts::wrap)]
pub mod wrap {
    use super::*;

    use crate::widgets::text::{LINE_SPACING_VAR, TEXT_ALIGN_VAR};

    inherit!(widget_base::base);

    properties! {
        /// Widget items.
        pub widget_base::children;

        /// Space in-between items and rows.
        ///
        /// This property only defines the spacing for rows of this panel, but it is set
        /// to [`LINE_SPACING_VAR`] for rows and zero for *column space* by default, so you can use
        /// the [`line_spacing`] property if you want to affect all nested wrap and text widgets.
        ///
        /// Note that *column space* is limited for bidirectional inline items as it only inserts spacing between
        /// items once and bidirectional text can interleave items, consider using [`word_spacing`] for inline text.
        ///
        /// [`LINE_SPACING_VAR`]: crate::widgets::text::LINE_SPACING_VAR
        /// [`line_spacing`]: fn@crate::widgets::text::txt_align
        /// [`word_spacing`]: fn@crate::widgets::text::word_spacing
        pub spacing(impl IntoVar<GridSpacing>);

        /// Children align.
        ///
        /// This property only defines the align for children inside this panel, but it is set
        /// to [`TEXT_ALIGN_VAR`] by default, so you can use the [`txt_align`] property if you want
        /// to affect all nested wrap and text widgets.
        ///
        /// [`TEXT_ALIGN_VAR`]: crate::widgets::text::TEXT_ALIGN_VAR
        /// [`txt_align`]: fn@crate::widgets::text::txt_align
        pub children_align(impl IntoVar<Align>);

        /// Alignment of children in this widget and of nested wrap panels and texts.
        ///
        /// Note that this only sets the [`children_align`] if that property is not set (default) or is set to [`TEXT_ALIGN_VAR`].
        ///
        /// [`children_align`]: fn@children_align
        pub crate::widgets::text::txt_align;

        /// Spacing in-between rows of this widget and of nested wrap panels and texts.
        ///
        /// Note that this only sets the [`row_spacing`] if that property is no set (default), or is set to [`LINE_SPACING_VAR`] mapped to
        /// the [`GridSpacing::row`] value.
        ///
        /// [`row_spacing`]: fn@crate::widgets::text::row_spacing
        /// [`LINE_SPACING_VAR`]: crate::widgets::text::LINE_SPACING_VAR
        pub crate::widgets::text::line_spacing;
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(|wgt| {
            let child = node(
                wgt.capture_ui_node_list_or_empty(property_id!(self::children)),
                wgt.capture_var_or_else(property_id!(self::spacing), || {
                    LINE_SPACING_VAR.map(|s| GridSpacing {
                        column: Length::zero(),
                        row: s.clone(),
                    })
                }),
                wgt.capture_var_or_else(property_id!(self::children_align), || TEXT_ALIGN_VAR),
            );
            wgt.set_child(child);
        });
    }
    /// Wrap node.
    ///
    /// Can be used directly to inline widgets without declaring a wrap widget info.  This node is the child
    /// of the `wrap!` widget.
    pub fn node(children: impl UiNodeList, spacing: impl IntoVar<GridSpacing>, children_align: impl IntoVar<Align>) -> impl UiNode {
        WrapNode {
            children: PanelList::new(children),
            spacing: spacing.into_var(),
            children_align: children_align.into_var(),
            layout: Default::default(),
        }
    }

    #[doc(inline)]
    pub use super::{lazy_sample, lazy_size};
}

/// Create a node that estimates the size for a wrap panel children where all items have the same `child_size`.
pub fn lazy_size(children_len: impl IntoVar<usize>, spacing: impl IntoVar<GridSpacing>, child_size: impl IntoVar<Size>) -> impl UiNode {
    #[ui_node(struct InlineSizeNode {
        #[var] size: impl Var<Size>
    })]
    impl UiNode for InlineSizeNode {
        fn update(&mut self, _: &WidgetUpdates) {
            if self.size.is_new() {
                UPDATES.layout();
            }
        }
        fn measure(&self, _: &mut WidgetMeasure) -> PxSize {
            self.size.layout()
        }
        fn layout(&mut self, _: &mut WidgetLayout) -> PxSize {
            self.size.layout()
        }
    }

    // we don't use `properties::size(NilUiNode, child_size)` because that size disables inlining.
    let sample = InlineSizeNode {
        size: child_size.into_var(),
    };

    lazy_sample(children_len, spacing, sample)
}

/// Create a node that estimates the size for a wrap panel children where all items have the same size as `child_sample`.
pub fn lazy_sample(children_len: impl IntoVar<usize>, spacing: impl IntoVar<GridSpacing>, child_sample: impl UiNode) -> impl UiNode {
    #[ui_node(struct LazyWrapNode {
        child: impl UiNode,
        #[var] children_len: impl Var<usize>,
        #[var] spacing: impl Var<GridSpacing>,
    })]
    impl UiNode for LazyWrapNode {
        fn update(&mut self, updates: &WidgetUpdates) {
            if self.children_len.is_new() || self.spacing.is_new() {
                WIDGET.layout();
            }
            self.child.update(updates);
        }

        fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
            let child_size = self.child.measure(wm);
            InlineLayout::estimate_measure(wm, self.children_len.get(), child_size, self.spacing.layout())
        }

        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            let child_size = self.child.layout(wl);
            InlineLayout::estimate_layout(wl, self.children_len.get(), child_size, self.spacing.layout())
        }
    }
    LazyWrapNode {
        children_len: children_len.into_var(),
        spacing: spacing.into_var(),
        child: child_sample,
    }
}

#[ui_node(struct WrapNode {
    children: PanelList,
    #[var] spacing: impl Var<GridSpacing>,
    #[var] children_align: impl Var<Align>,
    layout: Mutex<InlineLayout>
})]
impl UiNode for WrapNode {
    fn update(&mut self, updates: &WidgetUpdates) {
        let mut any = false;
        self.children.update_all(updates, &mut any);

        if any || self.spacing.is_new() || self.children_align.is_new() {
            WIDGET.layout();
        }
    }

    fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
        let spacing = self.spacing.layout();
        self.layout.lock().measure(wm, &self.children, self.children_align.get(), spacing)
    }

    #[allow_(zero_ui::missing_delegate)] // false positive
    fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
        let spacing = self.spacing.layout();
        self.layout
            .get_mut()
            .layout(wl, &mut self.children, self.children_align.get(), spacing)
    }
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
pub struct InlineLayout {
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

        if let Some(inline) = wm.inline() {
            let inline_constraints = metrics.inline_constraints().unwrap().measure();

            inline.first_wrapped = inline_constraints.first_max < child_size.width;

            let mut children_len = Px(children_len as i32);

            if !inline.first_wrapped {
                // first row
                let max_x = inline_constraints.first_max;
                let column_len = (max_x - child_size.width) / (child_size.width + spacing.column) + Px(1);

                children_len -= column_len; // remove first row

                inline.first.width = (column_len - Px(1)) * (child_size.width + spacing.column) + child_size.width;
                inline.first.height = child_size.height.max(inline_constraints.mid_clear_min);
            }

            let max_x = constraints.x.max().unwrap_or(Px::MAX).max(child_size.width);
            let column_len = (max_x - child_size.width) / (child_size.width + spacing.column) + Px(1);
            let mut row_len = (children_len / column_len).max(Px(1));

            if column_len * row_len < children_len {
                row_len.0 += 1;
                debug_assert!(column_len * row_len >= children_len);
            }

            // spacing in between items means space available to divide for pairs (width+space) has 1 less item.
            let mut desired_size = PxSize::new(
                (column_len - Px(1)) * (child_size.width + spacing.column) + child_size.width,
                (row_len - Px(1)) * (child_size.height + spacing.row) + child_size.height,
            );
            if !inline.first_wrapped {
                // first row already taken from `children_len` and `row_len`.
                desired_size.height += inline.first.height;
                if children_len.0 > 0 {
                    desired_size.height += spacing.row;
                }
            }

            inline.last_wrapped = row_len.0 > 1;
            if inline.last_wrapped {
                let last_len = children_len % column_len;
                if last_len.0 > 0 {
                    inline.last.width = (last_len - Px(1)) * (child_size.width + spacing.column) + child_size.width;
                } else {
                    inline.last.width = desired_size.width;
                }
                inline.last.height = child_size.height;
            }

            if inline.first_wrapped {
                inline.first.width = desired_size.width;
                inline.first.height = child_size.height;
            }

            constraints.clamp_size(desired_size)
        } else {
            let max_x = constraints.x.max().unwrap_or(Px::MAX).max(child_size.width);
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

    pub fn measure(&mut self, wm: &mut WidgetMeasure, children: &PanelList, child_align: Align, spacing: PxGridSpacing) -> PxSize {
        let metrics = LAYOUT.metrics();
        let constraints = metrics.constraints();

        if let (None, Some(known)) = (metrics.inline_constraints(), constraints.fill_or_exact()) {
            return known;
        }

        self.measure_rows(&metrics, children, child_align, spacing);

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
        let size = if let Some(inline) = wl.inline() {
            let mut wm = WidgetMeasure::new_inline();
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
            Self::estimate_measure(&mut WidgetMeasure::new(), children_len, child_size, spacing)
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
            self.measure_rows(&metrics, children, child_align, spacing);
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
        let panel_height = last.origin.y + last.size.height;

        let child_constraints = PxConstraints2d::new_unbounded().with_fill_x(true).with_max_x(panel_width);

        if let Some(inline) = wl.inline() {
            inline.rows.clear();
        }

        LAYOUT.with_constraints(child_constraints, || {
            let mut row = first;
            let mut row_segs = &self.rows[0].item_segs;
            let mut row_advance = Px(0);
            let mut next_row_i = 1;
            let mut row_segs_i_start = 0;

            children.for_each_mut(|i, child, o| {
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
                }

                let (bidi_x, bidi_width, bidi_segs) = if self.has_bidi_inline {
                    row_segs[i - row_segs_i_start].x_width_segs()
                } else {
                    (Px(0), Px(0), self.bidi_default_segs.clone())
                };

                let child_inline = child.with_context(|| WIDGET.bounds().measure_inline()).flatten();
                if let Some(child_inline) = child_inline {
                    let child_desired_size = child.with_context(|| WIDGET.bounds().measure_outer_size()).unwrap_or_default();
                    if child_desired_size.is_empty() {
                        // collapsed, continue.
                        wl.collapse_child(i);
                        return true;
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

                        let (_, define_ref_frame) = wl.with_child(|wl| {
                            LAYOUT.layout_inline(wl, child_first, child_mid, child_last, bidi_segs, last_bidi_segs, child)
                        });
                        o.child_offset = PxVector::new(Px(0), row.origin.y);
                        o.define_reference_frame = define_ref_frame;

                        // new row
                        if let Some(inline) = wl.inline() {
                            inline.rows.push(row);
                            child.with_context(|| {
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

                        let (_, define_ref_frame) = wl.with_child(|wl| {
                            LAYOUT.with_constraints(child_constraints.with_fill(false, false).with_max_size(max_size), || {
                                LAYOUT.layout_inline(wl, child_first, child_mid, child_last, bidi_segs.clone(), bidi_segs, child)
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
                        || wl.with_child(|wl| LAYOUT.layout_block(wl, child)),
                    );
                    if size.is_empty() {
                        // collapsed, continue.
                        o.child_offset = PxVector::zero();
                        o.define_reference_frame = false;
                        return true;
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

                true
            });

            if let Some(inline) = wl.inline() {
                // last row
                inline.rows.push(row);
            }
        });

        constraints.clamp_size(PxSize::new(panel_width, panel_height))
    }

    fn measure_rows(&mut self, metrics: &LayoutMetrics, children: &PanelList, child_align: Align, spacing: PxGridSpacing) {
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

                let (inline, size) = LAYOUT.measure_inline(inline_constrain, row.size.height - spacing.row, child);

                if size.is_empty() {
                    row.item_segs.push(ItemSegsInfo::new_collapsed());
                    // collapsed, continue.
                    return true;
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

                true
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
            let sum_width = row.item_segs.iter().map(|s| Px(s.measure_width() as i32)).sum::<Px>();

            if (sum_width - width) > Px(1) {
                if metrics.inline_constraints().is_some() && (i == 0 || i == self.rows.len() - 1) {
                    tracing::error!("wrap! panel row {i} inline width is {width}, but sum of segs is {sum_width}");
                    continue;
                }

                tracing::error!("wrap! panel row {i} computed width {width}, but sum of segs is {sum_width}");
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

#[cfg(test)]
mod tests {
    use crate::core::{app::App, context::*};
    use crate::widgets::{container, wgt};

    use super::*;

    #[test]
    fn lazy_estimate() {
        let _app = App::minimal().run_headless(false);

        WINDOW.with_test_context(|| {
            let mut panel = wrap! {
                children = (0..100).map(|_| wgt! {
                    size = (120, 120);
                }).collect::<UiNodeVec>();
                spacing = 8;
            };
            let mut estimate = container! {
                child = wrap::lazy_size(100, 8, (120, 120));
            };

            WINDOW.test_init(&mut panel);
            WINDOW.test_init(&mut estimate);

            let m_constraints = PxConstraints2d::new_unbounded().with_max_x(Px(1184));
            let measure_constraints = InlineConstraintsMeasure {
                first_max: Px(1184),
                mid_clear_min: Px(-8),
            };
            let l_constraints = PxConstraints2d::new_unbounded().with_max_x(Px(1144));
            let layout_constraints = InlineConstraintsLayout {
                first: PxSize::new(Px(1144), Px(120)).into(),
                mid_clear: Px(0),
                last: PxRect::new(PxPoint::new(Px(0), Px(1408)), PxSize::new(Px(120), Px(120))),
                first_segs: Arc::new(vec![]),
                last_segs: Arc::new(vec![]),
            };
            let panel_size = WINDOW
                .test_layout_inline(
                    &mut panel,
                    (m_constraints, measure_constraints),
                    (l_constraints, layout_constraints.clone()),
                )
                .0;
            let estimate_size = WINDOW
                .test_layout_inline(
                    &mut estimate,
                    (m_constraints, measure_constraints),
                    (l_constraints, layout_constraints),
                )
                .0;

            let panel_bounds = panel.with_context(|| WIDGET.bounds()).unwrap();
            let estimate_bounds = estimate.with_context(|| WIDGET.bounds()).unwrap();

            assert_eq!(panel_size, estimate_size);
            assert!(panel_bounds.inline().is_some());
            assert!(estimate_bounds.inline().is_some());

            let panel_m_inline = panel_bounds.measure_inline().unwrap();
            let estimate_m_inline = estimate_bounds.measure_inline().unwrap();
            assert_eq!(panel_m_inline, estimate_m_inline);

            let panel_inline = panel_bounds.inline().as_deref().cloned().unwrap();
            let estimate_inline = estimate_bounds.inline().as_deref().cloned().unwrap();

            assert_eq!(panel_inline.inner_size, estimate_inline.inner_size);
            assert_eq!(panel_inline.rows[0], estimate_inline.rows[0]);
            assert_eq!(panel_inline.rows.last().unwrap(), estimate_inline.rows.last().unwrap());
        });
    }

    #[test]
    fn lazy_estimate_first_wrap() {
        let _app = App::minimal().run_headless(false);

        WINDOW.with_test_context(|| {
            let mut panel = wrap! {
                children = (0..100).map(|_| wgt! {
                    size = (120, 120);
                }).collect::<UiNodeVec>();
                spacing = 8;
            };
            let mut estimate = container! {
                child = wrap::lazy_size(100, 8, (120, 120));
            };

            WINDOW.test_init(&mut panel);
            WINDOW.test_init(&mut estimate);

            let m_constraints = PxConstraints2d::new_unbounded().with_max_x(Px(1184));
            let measure_constraints = InlineConstraintsMeasure {
                first_max: Px(32),
                mid_clear_min: Px(112),
            };
            let l_constraints = PxConstraints2d::new_unbounded().with_max_x(Px(1144));
            let layout_constraints = InlineConstraintsLayout {
                first: PxSize::new(Px(1144), Px(120)).into(),
                mid_clear: Px(0),
                last: PxRect::new(PxPoint::new(Px(0), Px(1416)), PxSize::new(Px(120), Px(120))),
                first_segs: Arc::new(vec![]),
                last_segs: Arc::new(vec![]),
            };
            let panel_size = WINDOW
                .test_layout_inline(
                    &mut panel,
                    (m_constraints, measure_constraints),
                    (l_constraints, layout_constraints.clone()),
                )
                .0;
            let estimate_size = WINDOW
                .test_layout_inline(
                    &mut estimate,
                    (m_constraints, measure_constraints),
                    (l_constraints, layout_constraints),
                )
                .0;

            let panel_bounds = panel.with_context(|| WIDGET.bounds()).unwrap();
            let estimate_bounds = estimate.with_context(|| WIDGET.bounds()).unwrap();

            assert_eq!(panel_size, estimate_size);
            assert!(panel_bounds.inline().is_some());
            assert!(estimate_bounds.inline().is_some());

            let panel_m_inline = panel_bounds.measure_inline().unwrap();
            let estimate_m_inline = estimate_bounds.measure_inline().unwrap();
            assert_eq!(panel_m_inline, estimate_m_inline);

            let panel_inline = panel_bounds.inline().as_deref().cloned().unwrap();
            let estimate_inline = estimate_bounds.inline().as_deref().cloned().unwrap();

            assert_eq!(panel_inline.inner_size, estimate_inline.inner_size);
            assert_eq!(panel_inline.rows[0], estimate_inline.rows[0]);
            assert_eq!(panel_inline.rows.last().unwrap(), estimate_inline.rows.last().unwrap());
        });
    }
}
