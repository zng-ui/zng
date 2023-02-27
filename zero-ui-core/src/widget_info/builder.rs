use std::hash::Hash;

use crate::{
    context::{StateMapMut, LAYOUT, WIDGET, WINDOW},
    text::TextSegmentKind,
};

use super::*;

/// Tag for the [`WidgetInfo::meta`] state-map.
pub enum WidgetInfoMeta {}

/// Widget info tree builder.
///
/// See [`WidgetInfoTree`] for more details.
pub struct WidgetInfoBuilder {
    window_id: WindowId,

    node: tree::NodeId,
    widget_id: WidgetId,
    meta: OwnedStateMap<WidgetInfoMeta>,

    tree: Tree<WidgetInfoData>,
    lookup: IdMap<WidgetId, tree::NodeId>,
    interactivity_filters: InteractivityFilters,

    scale_factor: Factor,

    build_meta: OwnedStateMap<WidgetInfoMeta>,

    build_start: Instant,
    pushed_widgets: u32,

    out_of_bounds: Vec<tree::NodeId>,
}
impl WidgetInfoBuilder {
    /// Starts building a info tree with the root information.
    pub fn new(
        window_id: WindowId,
        root_id: WidgetId,
        root_bounds_info: WidgetBoundsInfo,
        root_border_info: WidgetBorderInfo,
        scale_factor: Factor,
        used_data: Option<UsedWidgetInfoBuilder>,
    ) -> Self {
        let used_data = used_data.unwrap_or_else(UsedWidgetInfoBuilder::fallback);
        let tree = Tree::with_capacity(
            WidgetInfoData {
                widget_id: root_id,
                bounds_info: root_bounds_info,
                border_info: root_border_info,
                meta: Arc::new(OwnedStateMap::new()),
                interactivity_filters: vec![],
                local_interactivity: Interactivity::ENABLED,
                cache: Mutex::new(WidgetInfoCache { interactivity: None }),
            },
            used_data.tree_capacity,
        );
        let mut lookup = IdMap::default();
        lookup.reserve(used_data.tree_capacity);
        let root_node = tree.root().id();
        lookup.insert(root_id, root_node);

        WidgetInfoBuilder {
            window_id,
            node: root_node,
            tree,
            interactivity_filters: Vec::with_capacity(used_data.interactivity_filters_capacity),
            out_of_bounds: Vec::with_capacity(used_data.out_of_bounds_capacity),
            lookup,
            meta: OwnedStateMap::new(),
            widget_id: root_id,
            scale_factor,
            build_meta: OwnedStateMap::new(),
            build_start: Instant::now(),
            pushed_widgets: 1, // root is always new.
        }
    }

    fn node(&mut self, id: tree::NodeId) -> tree::NodeMut<WidgetInfoData> {
        self.tree.index_mut(id)
    }

    /// Current widget id.
    pub fn widget_id(&self) -> WidgetId {
        self.widget_id
    }

    /// Widget tree build metadata.
    ///
    /// This metadata can be modified only by pushed widgets, **not** by the reused widgets.
    pub fn build_meta(&mut self) -> StateMapMut<WidgetInfoMeta> {
        self.build_meta.borrow_mut()
    }

    /// Current widget metadata.
    pub fn meta(&mut self) -> StateMapMut<WidgetInfoMeta> {
        self.meta.borrow_mut()
    }

    /// Calls `f` in a new widget context.
    ///
    /// Only call this in widget node implementations.
    ///
    /// # Panics
    ///
    /// If the `id` was already pushed or reused in this builder.
    pub fn push_widget(&mut self, id: WidgetId, bounds_info: WidgetBoundsInfo, border_info: WidgetBorderInfo, f: impl FnOnce(&mut Self)) {
        let parent_node = self.node;
        let parent_widget_id = self.widget_id;
        let parent_meta = mem::take(&mut self.meta);

        let was_out_of_bounds = bounds_info.is_actually_out_of_bounds();

        self.widget_id = id;
        self.node = self
            .node(parent_node)
            .push_child(WidgetInfoData {
                widget_id: id,
                bounds_info,
                border_info,
                meta: Arc::new(OwnedStateMap::new()),
                interactivity_filters: vec![],
                local_interactivity: Interactivity::ENABLED,
                cache: Mutex::new(WidgetInfoCache { interactivity: None }),
            })
            .id();

        if was_out_of_bounds {
            self.out_of_bounds.push(self.node);
        }

        self.pushed_widgets += 1;

        if self.lookup.insert(id, self.node).is_some() {
            panic!("pushed widget `{id:?}` was already pushed or reused");
        }

        f(self);

        let meta = Arc::new(mem::replace(&mut self.meta, parent_meta));
        let mut node = self.node(self.node);
        node.value().meta = meta;
        node.close();

        self.node = parent_node;
        self.widget_id = parent_widget_id;
    }

    /// Reuse the widget info branch from the previous tree.
    ///
    /// All info state is preserved in the new info tree, all [interactivity filters] registered by the widget also affect
    /// the new info tree.
    ///
    /// Only call this in widget node implementations that monitor the updates requested by their content.
    ///
    /// # Panics
    ///
    /// If the `ctx.path.widget_id()` was already pushed or reused in this builder.
    ///
    /// [interactivity filters]: Self::push_interactivity_filter
    pub fn push_widget_reuse(&mut self) {
        let id = WIDGET.id();

        debug_assert_ne!(
            self.widget_id, id,
            "can only call `push_widget` or `push_widget_reuse` for each widget"
        );

        let tree = WINDOW.widget_tree();
        let wgt = tree
            .get(id)
            .unwrap_or_else(|| panic!("cannot reuse `{:?}`, not found in previous tree", id));

        self.tree.index_mut(self.node).push_reuse(
            wgt.node(),
            &mut |old_data| {
                let r = old_data.clone();
                r.cache.lock().interactivity = None;
                for filter in &r.interactivity_filters {
                    self.interactivity_filters.push(filter.clone());
                }
                r
            },
            &mut |new_node| {
                let wgt_id = new_node.value().widget_id;
                if self.lookup.insert(wgt_id, new_node.id()).is_some() {
                    panic!("reused widget `{wgt_id:?}` was already pushed or reused");
                }
                if new_node.value().bounds_info.is_actually_out_of_bounds() {
                    self.out_of_bounds.push(new_node.id());
                }
            },
        );
    }

    /// Add the `interactivity` bits to the current widget's interactivity, it will affect the widget and all descendants.
    ///
    /// Also see [`push_interactivity_filter`] to affect the interactivity of widgets outside the current one.
    ///
    /// [`push_interactivity_filter`]: Self::push_interactivity_filter
    pub fn push_interactivity(&mut self, interactivity: Interactivity) {
        let mut node = self.node(self.node);
        let v = node.value();
        v.local_interactivity |= interactivity;
    }

    /// Register a closure that returns the [`Interactivity`] allowed for each widget.
    ///
    /// Widgets [`interactivity`] is computed from all interactivity filters and parents. Interactivity filters are global to the
    /// widget tree, and are re-registered for the tree if the current widget is [reused].
    ///
    /// Note that the filter can make the assumption that parent widgets affect all descendants and if the filter is intended to
    /// affect only the current widget and descendants you can use [`push_interactivity`] instead.
    ///
    /// [`interactivity`]: WidgetInfo::interactivity
    /// [`push_interactivity`]: Self::push_interactivity
    /// [reused]: Self::push_widget_reuse
    pub fn push_interactivity_filter(&mut self, filter: impl Fn(&InteractivityFilterArgs) -> Interactivity + Send + Sync + 'static) {
        let filter = Arc::new(filter);
        self.interactivity_filters.push(filter.clone());
        self.node(self.node).value().interactivity_filters.push(filter);
    }

    /// Calls the `info` closure and returns the range of children inserted by it.
    pub fn with_children_range(&mut self, info: impl FnOnce(&mut Self)) -> ops::Range<usize> {
        let before_count = self.tree.index(self.node).children_count();
        info(self);
        before_count..self.tree.index(self.node).children_count()
    }

    /// Build the info tree.
    pub fn finalize(mut self) -> (WidgetInfoTree, UsedWidgetInfoBuilder) {
        let mut node = self.tree.root_mut();
        let meta = Arc::new(self.meta);
        node.value().meta = meta;
        node.close();

        let r = WidgetInfoTree(Arc::new(WidgetInfoTreeInner {
            window_id: self.window_id,
            lookup: self.lookup,
            interactivity_filters: self.interactivity_filters,
            build_meta: Arc::new(self.build_meta),

            frame: Mutex::new(WidgetInfoTreeFrame {
                stats: WidgetInfoTreeStats::new(self.build_start, self.tree.len() as u32 - self.pushed_widgets),
                stats_update: Default::default(),
                out_of_bounds: Arc::new(self.out_of_bounds),
                out_of_bounds_update: Default::default(),
                scale_factor: self.scale_factor,
                spatial_bounds: PxBox::zero(),
            }),

            tree: self.tree,
        }));

        let cap = UsedWidgetInfoBuilder {
            tree_capacity: r.0.tree.len(),
            interactivity_filters_capacity: r.0.interactivity_filters.len(),
            out_of_bounds_capacity: r.0.frame.lock().out_of_bounds.len(),
        };

        (r, cap)
    }
}

/// Represents a segment in an inlined widget first or last row.
///
/// This info is used by inlining parent to sort the joiner row in a way that preserves bidirectional text flow.
///
/// See [`WidgetInlineMeasure::first_segs`] for more details.
#[derive(Clone, Copy, Debug)]
pub struct InlineSegment {
    /// Width of the segment, in pixels.
    pub width: f32,
    /// Info for bidirectional reorder.
    pub kind: TextSegmentKind,
}
impl PartialEq for InlineSegment {
    fn eq(&self, other: &Self) -> bool {
        about_eq(self.width, other.width, 0.001) && self.kind == other.kind
    }
}
impl Eq for InlineSegment {}
impl Hash for InlineSegment {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        about_eq_hash(self.width, 0.001, state);
        self.kind.hash(state);
    }
}

/// Represents an [`InlineSegment`] positioned by the inlining parent.
///
/// See [`InlineConstrainsLayout::first_segs`] for more details.
///
/// [`InlineConstrainsLayout::first_segs`]: crate::context::InlineConstrainsLayout::first_segs
#[derive(Clone, Copy, Debug)]
pub struct InlineSegmentPos {
    /// Seg offset to the right from the row origin, in pixels.
    pub x: f32,
}
impl PartialEq for InlineSegmentPos {
    fn eq(&self, other: &Self) -> bool {
        about_eq(self.x, other.x, 0.001)
    }
}
impl Eq for InlineSegmentPos {}
impl Hash for InlineSegmentPos {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        about_eq_hash(self.x, 0.001, state);
    }
}

/// Info about the input inline connecting rows of the widget.
#[derive(Clone, Debug, Default)]
pub struct WidgetInlineMeasure {
    /// Maximum fill width possible on the first row.
    pub first_max_fill: Px,
    /// Maximum fill width possible on the last row.
    pub last_max_fill: Px,

    /// Preferred first size.
    ///
    /// In left-to-right direction the origin is `top_left`, in right-to-left direction the origin is `top_right - first.width`.
    pub first: PxSize,

    /// Indicates that `first` starts in the next row, not in the *current* row defined by the inline constrains.
    pub first_wrapped: bool,

    /// Inline segments in the first row.
    ///
    /// The sum of segment widths must be less or equal to the `first.width`.
    pub first_segs: Arc<Vec<InlineSegment>>,

    /// Preferred last size.
    ///
    /// In left-to-right direction the origin is `bottom_left - last.height`, in right-to-left direction
    /// the origin is `bottom_right - last`.
    pub last: PxSize,

    /// Indicates that `last` starts in the next row, not in the same row as the first.
    pub last_wrapped: bool,

    /// Inline segments in the last row.
    ///
    /// The sum of segment widths must be less or equal to the `last.width`.
    pub last_segs: Arc<Vec<InlineSegment>>,
}
impl WidgetInlineMeasure {
    /// Visit a mutable reference to the new [`first_segs`] value, `f` is called with
    /// an empty vec that can be reused or new.
    ///
    /// [`first_segs`]: Self::first_segs
    pub fn with_first_segs(&mut self, f: impl FnOnce(&mut Vec<InlineSegment>)) {
        Self::with_segs(&mut self.first_segs, f)
    }

    /// Visit a mutable reference to the new [`last_segs`] value, `f` is called with
    /// an empty vec that can be reused or new.
    ///
    /// [`last_segs`]: Self::last_segs
    pub fn with_last_segs(&mut self, f: impl FnOnce(&mut Vec<InlineSegment>)) {
        Self::with_segs(&mut self.last_segs, f)
    }

    fn with_segs(items: &mut Arc<Vec<InlineSegment>>, f: impl FnOnce(&mut Vec<InlineSegment>)) {
        match Arc::get_mut(items) {
            Some(items) => {
                items.clear();
                f(items);
            }
            None => {
                let mut new = vec![];
                f(&mut new);
                *items = Arc::new(new);
            }
        }
    }
}

/// Info about a segment in the first or last row of an inlined widget.
///
/// See [`WidgetInlineInfo::first_segs`] for more details.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InlineSegmentInfo {
    /// Segment offset from the row rectangle origin.
    pub x: Px,
    /// Segment width.
    ///
    /// The segment height is the row rectangle height.
    pub width: Px,
}

/// Info about the inlined rows of the widget.
#[derive(Debug, Default)]
pub struct WidgetInlineInfo {
    /// Last layout rows of the widget.
    ///
    /// The rectangles are in the widget's inner space, from top to bottom.
    pub rows: Vec<PxRect>,

    /// Segments of the first row.
    ///
    /// If this is empty the entire row width is a continuous segment, otherwise the row is segmented and
    /// the widget can be interleaved with sibling widgets due to Unicode bidirectional text sorting algorithm.
    ///
    /// Note that the segment count may be less then [`WidgetInlineMeasure::first_segs`] as contiguous segments
    /// may be merged.
    ///
    /// The segments are from left to right.
    pub first_segs: Vec<InlineSegmentInfo>,

    /// Segments of the last row.
    pub last_segs: Vec<InlineSegmentInfo>,

    /// Widget inner size when the rows where last updated.
    pub inner_size: PxSize,

    negative_space: Mutex<(Arc<Vec<PxRect>>, bool)>,
}
impl WidgetInlineInfo {
    /// Replace the [`first_segs`] with `segs`.
    ///
    /// The segments are sorted when needed, but prefer inputs that are mostly sorted.
    ///
    /// The segments are merged when there is no gap or there is a small one pixel overlap to the previous segment.
    ///
    /// [`first_segs`]: Self::first_segs
    pub fn set_first_segs(&mut self, segs: impl Iterator<Item = InlineSegmentInfo>) {
        Self::set_segs(&mut self.first_segs, segs);
        self.invalidate_negative_space();
    }

    /// Replace the [`last_segs`] with `segs`.
    ///
    /// The segments are sorted when needed, but prefer inputs that are mostly sorted.
    ///
    /// The segments are merged when there is no gap or there is a small one pixel overlap to the previous segment.
    ///
    /// [`last_segs`]: Self::last_segs
    pub fn set_last_segs(&mut self, segs: impl Iterator<Item = InlineSegmentInfo>) {
        Self::set_segs(&mut self.last_segs, segs);
        self.invalidate_negative_space();
    }

    fn set_segs(vec: &mut Vec<InlineSegmentInfo>, segs: impl Iterator<Item = InlineSegmentInfo>) {
        vec.clear();

        let mut needs_sort = false;

        for seg in segs {
            if seg.width <= Px(0) {
                continue;
            }

            if let Some(last) = vec.last_mut() {
                let la = last.x;
                let lb = last.x + last.width;

                let a = seg.x;
                let b = seg.x + seg.width;

                if la.max(a) <= lb.min(b) {
                    // merge overlap
                    last.x = a.min(la);
                    last.width = b.max(lb) - last.x;
                    continue;
                }

                needs_sort |= a < la;
            }
            vec.push(seg);
        }

        if needs_sort {
            vec.sort_unstable_by_key(|s| s.x);
        }
    }

    /// Gets the union of all row rectangles.
    pub fn union(&self) -> PxRect {
        self.rows.iter().fold(PxRect::zero(), |union, row| union.union(row))
    }

    /// Gets or computes the negative space of the [`rows`] in the [`inner_size`] space, that is, all the areas that are
    /// not covered by any row and not covered by the first and last row segments.
    ///
    /// This is computed on demand and cached.
    ///
    /// [`rows`]: Self::rows
    /// [`inner_size`]: Self::inner_size
    pub fn negative_space(&self) -> Arc<Vec<PxRect>> {
        let mut space = self.negative_space.lock();
        if space.1 {
            return space.0.clone();
        }

        let mut vec = Arc::try_unwrap(mem::take(&mut space.0)).unwrap_or_default();
        vec.clear();

        self.negative_enveloped(&mut vec, PxRect::from_size(self.inner_size));

        let r = Arc::new(vec);
        *space = (r.clone(), true);
        r
    }

    /// Invalidates the [`negative_space`] cache.
    ///
    /// [`negative_space`]: Self::negative_space
    pub fn invalidate_negative_space(&mut self) {
        self.negative_space.get_mut().1 = false;
    }

    fn negative_enveloped(&self, space: &mut Vec<PxRect>, bounds: PxRect) {
        let bounds_max_x = bounds.max_x();
        let mut last_max_y = bounds.origin.y;

        for r in &self.rows {
            let spacing_y = r.origin.y - last_max_y;
            if spacing_y > Px(0) {
                space.push(PxRect::new(
                    PxPoint::new(bounds.origin.x, last_max_y),
                    PxSize::new(bounds.size.width, spacing_y),
                ));
            }
            last_max_y = r.max_y();

            let left = r.origin.x - bounds.origin.x;
            if left > Px(0) {
                space.push(PxRect::new(
                    PxPoint::new(bounds.origin.x, r.origin.y),
                    PxSize::new(left, r.size.height),
                ));
            }
            let max_x = r.max_x();
            let right = bounds_max_x - max_x;
            if right > Px(0) {
                space.push(PxRect::new(PxPoint::new(max_x, r.origin.y), PxSize::new(right, r.size.height)));
            }
        }
        let spacing_y = bounds.max_y() - last_max_y;
        if spacing_y > Px(0) {
            space.push(PxRect::new(
                PxPoint::new(bounds.origin.x, last_max_y),
                PxSize::new(bounds.size.width, spacing_y),
            ));
        }

        if let Some(r) = self.rows.first() {
            if !self.first_segs.is_empty() {
                let mut x = r.origin.x;
                for seg in self.first_segs.iter() {
                    let blank = seg.x - x;
                    if blank > Px(0) {
                        space.push(PxRect::new(PxPoint::new(x, r.origin.y), PxSize::new(blank, r.size.height)));
                    }
                    x = seg.x + seg.width;
                }
                let blank = r.max_x() - x;
                if blank > Px(0) {
                    space.push(PxRect::new(PxPoint::new(x, r.origin.y), PxSize::new(blank, r.size.height)));
                }
            }
        }
        if let Some(r) = self.rows.last() {
            if !self.last_segs.is_empty() {
                let mut x = r.origin.x;
                for seg in self.last_segs.iter() {
                    let blank = seg.x - x;
                    if blank > Px(0) {
                        space.push(PxRect::new(PxPoint::new(x, r.origin.y), PxSize::new(blank, r.size.height)));
                    }
                    x = seg.x + seg.width;
                }
                let blank = r.max_x() - x;
                if blank > Px(0) {
                    space.push(PxRect::new(PxPoint::new(x, r.origin.y), PxSize::new(blank, r.size.height)));
                }
            }
        }
    }
}

/// Represents the in-progress measure pass for a widget tree.
#[derive(Default)]
pub struct WidgetMeasure {
    inline: Option<WidgetInlineMeasure>,
}
impl WidgetMeasure {
    /// New default.
    pub fn new() -> Self {
        Self::default()
    }

    /// If the parent widget is doing inline flow layout.
    pub fn is_inline(&self) -> bool {
        self.inline.is_some()
    }

    /// Mutable reference to the current widget's inline info.
    ///
    /// The widget must configure this to be inlined in parent layout. This is only `Some(_)` if inline is enabled.
    ///
    /// See [`WidgetInlineMeasure`] for more details.
    pub fn inline(&mut self) -> Option<&mut WidgetInlineMeasure> {
        self.inline.as_mut()
    }

    /// Sets [`is_inline`] to `false`.
    ///
    /// Must be called before child delegation, otherwise children that inline may render expecting to fit in
    /// the inline flow.
    ///
    /// [`is_inline`]: Self::is_inline
    pub(crate) fn disable_inline(&mut self) {
        self.inline = None;
    }

    /// Measure an widget.
    ///
    /// The `reuse` flag indicates if the cached measure or layout size can be returned instead of calling `measure`. It should
    /// only be `false` if the widget has a pending layout request.
    pub fn with_widget(&mut self, reuse: bool, measure: impl FnOnce(&mut Self) -> PxSize) -> PxSize {
        let metrics = LAYOUT.metrics();
        let bounds = WIDGET.bounds();

        let snap = metrics.snapshot();
        if reuse {
            let measure_uses = bounds.measure_metrics_used();
            if bounds.measure_metrics().map(|m| m.masked_eq(&snap, measure_uses)).unwrap_or(false) {
                let mut reused = false;
                if let Some(inline) = self.inline() {
                    if let Some(prev) = bounds.measure_inline() {
                        *inline = prev;
                        reused = true;
                    }
                } else {
                    reused = bounds.measure_inline().is_none();
                }

                if reused {
                    return bounds.measure_outer_size();
                }
            }
        }

        let parent_uses = metrics.enter_widget_ctx();
        if LAYOUT.inline_constrains().is_some() && self.inline.is_none() {
            self.inline = Some(Default::default());
        } else {
            self.inline = None;
        }

        let size = measure(self);

        let measure_uses = metrics.exit_widget_ctx(parent_uses);
        bounds.set_measure_metrics(Some(snap), measure_uses);
        bounds.set_measure_outer_size(size);
        bounds.set_measure_inline(self.inline.take());

        size
    }
}

/// Represents the in-progress layout pass for a widget tree.
pub struct WidgetLayout {
    pass_id: LayoutPassId,
    bounds: WidgetBoundsInfo,
    nest_group: LayoutNestGroup,
    inline: Option<WidgetInlineInfo>,
    child_count: Option<u32>,
}
impl WidgetLayout {
    /// Defines the root widget outer-bounds scope.
    ///
    /// The default window implementation calls this.
    pub fn with_root_widget(pass_id: LayoutPassId, layout: impl FnOnce(&mut Self) -> PxSize) -> PxSize {
        Self {
            pass_id,
            bounds: WIDGET.bounds(),
            nest_group: LayoutNestGroup::Inner,
            inline: None,
            child_count: None,
        }
        .with_widget(false, layout)
    }

    /// Defines a widget scope, translations inside `layout` target the widget's inner offset.
    ///
    /// If `reuse` is `true` and none of the used metrics have changed skips calling `layout` and returns the current outer-size, the
    /// outer transform is still updated.
    ///
    /// The default widget constructor calls this, see [`widget_base::nodes::widget`].
    ///
    /// [`widget_base::nodes::widget`]: crate::widget_base::nodes::widget
    pub fn with_widget(&mut self, reuse: bool, layout: impl FnOnce(&mut Self) -> PxSize) -> PxSize {
        let metrics = LAYOUT.metrics();
        let bounds = WIDGET.bounds();

        let snap = metrics.snapshot();
        if let Some(child_count) = &mut self.child_count {
            *child_count += 1;
        }
        if reuse && {
            let uses = bounds.metrics_used();
            bounds.metrics().map(|m| m.masked_eq(&snap, uses)).unwrap_or(false)
        } {
            // reuse
            return bounds.outer_size();
        }

        let parent_child_count = self.child_count.take();
        let parent_inline = self.inline.take();
        if LAYOUT.inline_constrains().is_some() && bounds.measure_inline().is_some() {
            // inline enabled by parent and widget
            self.inline = bounds.take_inline();
            if let Some(inline) = self.inline.as_mut() {
                inline.rows.clear();
                inline.invalidate_negative_space();
            } else {
                self.inline = Some(Default::default());
            }
        }
        let parent_bounds = mem::replace(&mut self.bounds, bounds);
        self.nest_group = LayoutNestGroup::Inner;
        let prev_inner_offset = self.bounds.inner_offset();
        let prev_child_offset = self.bounds.child_offset();
        let prev_baseline = self.bounds.baseline();
        let prev_inner_offset_baseline = self.bounds.inner_offset_baseline();
        let prev_can_auto_hide = self.bounds.can_auto_hide();
        self.bounds.set_inner_offset(PxVector::zero());
        self.bounds.set_child_offset(PxVector::zero());
        self.bounds.set_baseline(Px(0));
        self.bounds.set_inner_offset_baseline(false);
        self.bounds.set_can_auto_hide(true);

        let parent_uses = metrics.enter_widget_ctx();

        // layout
        let size = layout(self);

        let uses = metrics.exit_widget_ctx(parent_uses);

        self.bounds.set_outer_size(size);
        self.bounds.set_metrics(Some(snap), uses);
        if let Some(inline) = &mut self.inline {
            inline.inner_size = self.bounds.inner_size();
            inline.invalidate_negative_space();
        }
        self.bounds.set_inline(self.inline.take());

        if prev_can_auto_hide != self.bounds.can_auto_hide() {
            WIDGET.render();
        } else if prev_inner_offset != self.bounds.inner_offset()
            || prev_child_offset != self.bounds.child_offset()
            || prev_inner_offset_baseline != self.bounds.inner_offset_baseline()
            || (self.bounds.inner_offset_baseline() && prev_baseline != self.bounds.baseline())
        {
            WIDGET.render_update();
        }

        self.child_count = parent_child_count;
        self.inline = parent_inline;
        self.bounds = parent_bounds;
        self.nest_group = LayoutNestGroup::Child;

        size
    }

    /// Defines a widget inner scope, translations inside `layout` target the widget's child offset.
    ///
    /// This method also updates the border info.
    ///
    /// The default widget borders constructor calls this, see [`widget_base::nodes::widget_inner`].
    ///
    /// [`widget_base::nodes::widget_inner`]: crate::widget_base::nodes::widget_inner
    pub fn with_inner(&mut self, layout: impl FnOnce(&mut Self) -> PxSize) -> PxSize {
        self.nest_group = LayoutNestGroup::Child;
        let size = ContextBorders::with_inner(|| layout(self));
        WIDGET.bounds().set_inner_size(size);
        self.nest_group = LayoutNestGroup::Inner;
        size
    }

    /// Defines a widget child scope, translations inside `layout` still target the widget's child offset.
    ///
    /// Returns the child size and if a reference frame is required to offset the child.
    ///
    /// The default widget child layout constructor implements this, see [`widget_base::nodes::widget_child`].
    ///
    /// [`widget_base::nodes::widget_child`]: crate::widget_base::nodes::widget_child
    /// [`child_offset`]: WidgetBoundsInfo::child_offset
    pub fn with_child(&mut self, layout: impl FnOnce(&mut Self) -> PxSize) -> (PxSize, bool) {
        let parent_child_count = mem::replace(&mut self.child_count, Some(0));

        self.nest_group = LayoutNestGroup::Child;
        let child_size = layout(self);
        self.nest_group = LayoutNestGroup::Child;

        let need_ref_frame = self.child_count != Some(1);
        self.child_count = parent_child_count;
        (child_size, need_ref_frame)
    }

    /// Gets the current window layout pass.
    ///
    /// Widgets can be layout more then once per window layout pass, you can use this ID to identify such cases.
    pub fn pass_id(&self) -> LayoutPassId {
        self.pass_id
    }

    /// Adds the `offset` to the closest *inner* bounds offset.
    ///
    /// This affects the inner offset if called from a node inside the widget and before the `BORDER` group, or it affects
    /// the child offset if called inside the widget and inside the `BORDER` group.
    pub fn translate(&mut self, offset: PxVector) {
        match self.nest_group {
            LayoutNestGroup::Inner => {
                let mut o = self.bounds.inner_offset();
                o += offset;
                self.bounds.set_inner_offset(o);
            }
            LayoutNestGroup::Child => {
                let mut o = self.bounds.child_offset();
                o += offset;
                self.bounds.set_child_offset(o);
            }
        }
    }

    /// Set the baseline offset of the widget. The value is up from the bottom of the inner bounds.
    pub fn set_baseline(&mut self, baseline: Px) {
        self.bounds.set_baseline(baseline);
    }

    /// Set if the baseline is added to the inner offset  *y* axis.
    pub fn translate_baseline(&mut self, enabled: bool) {
        self.bounds.set_inner_offset_baseline(enabled);
    }

    /// Sets if the widget only renders if [`outer_bounds`] intersects with the [`FrameBuilder::auto_hide_rect`].
    ///
    /// This is `true` by default.
    ///
    /// [`outer_bounds`]: WidgetBoundsInfo::outer_bounds
    /// [`FrameBuilder::auto_hide_rect`]: crate::render::FrameBuilder::auto_hide_rect
    pub fn allow_auto_hide(&mut self, enabled: bool) {
        self.bounds.set_can_auto_hide(enabled);
    }

    /// Collapse the layout of `self` and descendants, the size and offsets are set to zero.
    ///
    /// Nodes that set the visibility to the equivalent of [`Collapsed`] must skip layout and return [`PxSize::zero`] as
    /// the the size, ignoring the min-size constrains, and call this method to update all the descendant
    /// bounds information to be a zero-sized point.
    ///
    /// Note that the widget will automatically not be rendered when collapsed.
    ///
    /// [`Collapsed`]: Visibility::Collapsed
    pub fn collapse(&mut self) {
        let tree = WINDOW.widget_tree();
        let id = WIDGET.id();
        if let Some(w) = tree.get(id) {
            for w in w.self_and_descendants() {
                let info = w.info();
                info.bounds_info.set_outer_size(PxSize::zero());
                info.bounds_info.set_inner_size(PxSize::zero());
                info.bounds_info.set_baseline(Px(0));
                info.bounds_info.set_inner_offset_baseline(false);
                info.bounds_info.set_can_auto_hide(true);
                info.bounds_info.set_inner_offset(PxVector::zero());
                info.bounds_info.set_child_offset(PxVector::zero());
                info.bounds_info.set_measure_metrics(None, LayoutMask::NONE);
                info.bounds_info.set_metrics(None, LayoutMask::NONE);
                info.bounds_info.set_is_collapsed(true);
                info.bounds_info.set_rendered(None, &tree);
            }
        } else {
            tracing::error!("collapse did not find `{}` in the info tree", id)
        }
    }

    /// Collapse layout of all descendants, the size and offsets are set to zero.
    ///
    /// Widgets that control the visibility of their children can use this method and then, in the same layout pass, layout
    /// the children that should be visible.
    ///
    /// Note that the widgets will automatically not be rendered when collapsed.
    ///
    /// [`Collapsed`]: Visibility::Collapsed
    pub fn collapse_descendants(&mut self) {
        let tree = WINDOW.widget_tree();
        let id = WIDGET.id();
        if let Some(w) = tree.get(id) {
            for w in w.descendants() {
                let info = w.info();
                info.bounds_info.set_outer_size(PxSize::zero());
                info.bounds_info.set_inner_size(PxSize::zero());
                info.bounds_info.set_baseline(Px(0));
                info.bounds_info.set_inner_offset_baseline(false);
                info.bounds_info.set_can_auto_hide(true);
                info.bounds_info.set_inner_offset(PxVector::zero());
                info.bounds_info.set_child_offset(PxVector::zero());
                info.bounds_info.set_measure_metrics(None, LayoutMask::NONE);
                info.bounds_info.set_metrics(None, LayoutMask::NONE);
                info.bounds_info.set_is_collapsed(true);
            }
        } else {
            tracing::error!("collapse_descendants did not find `{}` in the info tree", id)
        }
    }

    /// Collapse layout of the child and all its descendants, the size and offsets are set to zero.
    ///
    /// Widgets that control the visibility of their children can use this method and then, in the same layout pass, layout
    /// the children that should be visible.
    ///
    /// Note that the widgets will automatically not be rendered when collapsed.
    ///
    /// [`Collapsed`]: Visibility::Collapsed
    pub fn collapse_child(&mut self, index: usize) {
        let tree = WINDOW.widget_tree();
        let id = WIDGET.id();
        if let Some(w) = tree.get(id) {
            if let Some(w) = w.children().nth(index) {
                for w in w.self_and_descendants() {
                    let info = w.info();
                    info.bounds_info.set_outer_size(PxSize::zero());
                    info.bounds_info.set_inner_size(PxSize::zero());
                    info.bounds_info.set_baseline(Px(0));
                    info.bounds_info.set_inner_offset_baseline(false);
                    info.bounds_info.set_can_auto_hide(true);
                    info.bounds_info.set_inner_offset(PxVector::zero());
                    info.bounds_info.set_child_offset(PxVector::zero());
                    info.bounds_info.set_measure_metrics(None, LayoutMask::NONE);
                    info.bounds_info.set_metrics(None, LayoutMask::NONE);
                    info.bounds_info.set_is_collapsed(true);
                }
            } else {
                tracing::error!(
                    "collapse_child out-of-bounds for `{}` in the children of `{}` in the info tree",
                    index,
                    id
                )
            }
        } else {
            tracing::error!("collapse_child did not find `{}` in the info tree", id)
        }
    }

    /// If the parent widget is doing inline layout and this widget signaled that it can support this
    /// during measure.
    ///
    /// See [`WidgetMeasure::inline`] for more details.
    pub fn is_inline(&self) -> bool {
        self.inline.is_some()
    }

    /// Mutable reference to the current widget's inline info.
    ///
    /// This is `Some(_)` if the parent widget is doing inline layout and this widget signaled that it can be inlined
    /// in the previous measure pass. You can use [`MeasureContext::with_inline_constrains`] in the measure pass to disable
    /// inline in both passes, measure and layout.
    ///
    /// The rows and negative space are already reset when widget layout started, and the inner size will be updated when
    /// the widget layout ends, the inline layout node only needs to push rows.
    ///
    /// When this is `Some(_)` the [`LayoutMetrics::inline_constrains`] is also `Some(_)`.
    ///
    /// See [`WidgetInlineInfo`] for more details.
    ///
    /// [`LayoutMetrics::inline_constrains`]: crate::context::LayoutMetrics::inline_constrains
    pub fn inline(&mut self) -> Option<&mut WidgetInlineInfo> {
        self.inline.as_mut()
    }
}

#[derive(Debug, Clone, Copy)]
enum LayoutNestGroup {
    /// Inside widget, outside `BORDER`.
    Inner,
    /// Inside `BORDER`.
    Child,
}

/// Identifies the layout pass of a window.
///
/// This value is different for each window layout, but the same for children of panels that do more then one layout pass.
pub type LayoutPassId = u32;
