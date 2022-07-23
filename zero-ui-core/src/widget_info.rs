//! Widget info tree.

use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
    fmt,
    marker::PhantomData,
    mem, ops,
    rc::Rc,
    time::{Duration, Instant},
};

use crate::{
    border::ContextBorders,
    context::{InfoContext, LayoutContext, LayoutMetricsSnapshot, OwnedStateMap, StateMap, Updates},
    crate_util::IdMap,
    event::EventUpdateArgs,
    handler::WidgetHandler,
    impl_from_and_into_var,
    render::{FrameId, FrameValue},
    ui_list::ZIndex,
    units::*,
    var::{Var, VarValue, VarsRead, WithVarsRead},
    window::WindowId,
    UiNode, Widget, WidgetId,
};

mod tree;
use tree::Tree;

mod path;
pub use path::*;

mod builder;
pub use builder::*;

pub mod iter;
pub use iter::TreeFilter;

mod hit;
pub(crate) use hit::HitTestClips;

pub use self::hit::RelativeHitZ;
use self::iter::TreeIterator;

/// Bundle of widget info data from the current widget.
#[derive(Clone, Default)]
pub struct WidgetContextInfo {
    /// Bounds layout info.
    pub bounds: WidgetBoundsInfo,
    /// Border and corners info.
    pub border: WidgetBorderInfo,
}
impl WidgetContextInfo {
    /// New default.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Stats over the lifetime of a widget info tree.
///
/// The stats for a tree are available in [`WidgetInfoTree::stats`].
#[derive(Debug, Clone)]
pub struct WidgetInfoTreeStats {
    /// Duration of the [`UiNode::info`] call for the window content.
    pub build_time: Duration,

    /// Count of widgets that where reused from a previous tree.
    pub reused_widgets: u32,

    /// Last window frame that touched this tree.
    ///
    /// Before the first render this is [`FrameId::INVALID`].
    pub last_frame: FrameId,

    /// Last window frame that moved or resized the inner bounds of widgets.
    pub bounds_updated_frame: FrameId,

    /// Count of moved or resized widgets in the last `bounds_updated_frame`.
    pub bounds_updated: u32,

    /// Last window frame that changed visibility of widgets.
    pub visibility_updated_frame: FrameId,
}
impl WidgetInfoTreeStats {
    fn new(build_start: Instant, reused_widgets: u32) -> Self {
        Self {
            build_time: build_start.elapsed(),
            reused_widgets,
            last_frame: FrameId::INVALID,
            bounds_updated_frame: FrameId::INVALID,
            bounds_updated: 0,
            visibility_updated_frame: FrameId::INVALID,
        }
    }

    fn update(&mut self, frame: FrameId, update: WidgetInfoTreeStatsUpdate) {
        self.last_frame = frame;

        if update.bounds_updated > 0 {
            self.bounds_updated = update.bounds_updated;
            self.bounds_updated_frame = frame;
        } else if self.bounds_updated_frame == FrameId::INVALID {
            self.bounds_updated_frame = frame;
        }

        // can double count if changed to collapsed from visible, so we don't show this stat.
        if update.visibility_updated > 0 || self.visibility_updated_frame == FrameId::INVALID {
            self.visibility_updated_frame = frame;
        }
    }
}
#[derive(Default)]
struct WidgetInfoTreeStatsUpdate {
    bounds_updated: u32,
    visibility_updated: u32,
}
impl WidgetInfoTreeStatsUpdate {
    fn take(&mut self) -> Self {
        mem::take(self)
    }
}

/// A tree of [`WidgetInfo`].
///
/// The tree is behind an `Rc` pointer so cloning and storing this type is very cheap.
///
/// Instantiated using [`WidgetInfoBuilder`].
#[derive(Clone)]
pub struct WidgetInfoTree(Rc<WidgetInfoTreeInner>);
struct WidgetInfoTreeInner {
    window_id: WindowId,
    tree: Tree<WidgetInfoData>,
    lookup: IdMap<WidgetId, tree::NodeId>,
    interactivity_filters: InteractivityFilters,
    out_of_bounds: RefCell<Rc<Vec<tree::NodeId>>>,
    spatial_bounds: PxBox,
    build_meta: Rc<OwnedStateMap>,
    stats: RefCell<WidgetInfoTreeStats>,
    stats_update: RefCell<WidgetInfoTreeStatsUpdate>,
    out_of_bounds_update: RefCell<Vec<(tree::NodeId, bool)>>,
    scale_factor: Cell<Factor>,
}
impl PartialEq for WidgetInfoTree {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for WidgetInfoTree {}
impl WidgetInfoTree {
    /// Blank window that contains only the root widget taking no space.
    pub fn blank(window_id: WindowId, root_id: WidgetId) -> Self {
        WidgetInfoBuilder::new(window_id, root_id, WidgetBoundsInfo::new(), WidgetBorderInfo::new(), 1.fct(), None)
            .finalize()
            .0
    }

    /// Statistics abound the info tree.
    pub fn stats(&self) -> WidgetInfoTreeStats {
        self.0.stats.borrow().clone()
    }

    /// Scale factor of the last rendered frame.
    pub fn scale_factor(&self) -> Factor {
        self.0.scale_factor.get()
    }

    /// Custom metadata associated with the tree during info build.
    ///
    /// Any widget (that was not reused) can have inserted metadata.
    pub fn build_meta(&self) -> &StateMap {
        &self.0.build_meta.0
    }

    /// Reference to the root widget in the tree.
    pub fn root(&self) -> WidgetInfo {
        WidgetInfo::new(self, self.0.tree.root().id())
    }

    /// All widgets including `root`.
    pub fn all_widgets(&self) -> iter::TreeIter {
        self.root().self_and_descendants()
    }

    /// Id of the window that owns all widgets represented in the tree.
    pub fn window_id(&self) -> WindowId {
        self.0.window_id
    }

    /// Reference to the widget in the tree, if it is present.
    pub fn get(&self, widget_id: impl Into<WidgetId>) -> Option<WidgetInfo> {
        self.0.lookup.get(&widget_id.into()).map(|i| WidgetInfo::new(self, *i))
    }

    /// If the tree contains the widget.
    pub fn contains(&self, widget_id: impl Into<WidgetId>) -> bool {
        self.0.lookup.contains_key(&widget_id.into())
    }

    /// Reference to the widget or first parent that is present.
    pub fn get_or_parent(&self, path: &WidgetPath) -> Option<WidgetInfo> {
        self.get(path.widget_id())
            .or_else(|| path.ancestors().iter().rev().find_map(|&id| self.get(id)))
    }

    /// If the widgets in this tree have been rendered at least once, after the first render the widget bounds info are always up-to-date
    /// and spatial queries can be made on the widgets.
    pub fn is_rendered(&self) -> bool {
        self.0.stats.borrow().last_frame != FrameId::INVALID
    }

    /// Iterator over all widgets with inner-bounds not fully contained by their parent inner bounds.
    pub fn out_of_bounds(&self) -> impl std::iter::ExactSizeIterator<Item = WidgetInfo> {
        let out = self.0.out_of_bounds.borrow().clone();
        (0..out.len()).map(move |i| WidgetInfo::new(self, out[i]))
    }

    /// Gets the bounds box that envelops all widgets, including the out-of-bounds widgets.
    pub fn spatial_bounds(&self) -> PxRect {
        self.0.spatial_bounds.to_rect()
    }

    fn bounds_changed(&self) {
        self.0.stats_update.borrow_mut().bounds_updated += 1;
    }

    fn in_bounds_changed(&self, widget_id: WidgetId, in_bounds: bool) {
        let id = *self.0.lookup.get(&widget_id).unwrap();
        self.0.out_of_bounds_update.borrow_mut().push((id, in_bounds));
    }

    fn visibility_changed(&self) {
        self.0.stats_update.borrow_mut().visibility_updated += 1;
    }

    pub(crate) fn after_render(&self, frame_id: FrameId, scale_factor: Factor) {
        let mut stats = self.0.stats.borrow_mut();
        stats.update(frame_id, self.0.stats_update.borrow_mut().take());

        let mut out_of_bounds_update = self.0.out_of_bounds_update.borrow_mut();

        if !out_of_bounds_update.is_empty() {
            // update out-of-bounds list, reuses the same vec most of the time,
            // unless a spatial iter was generated and not dropped before render.

            let mut out_of_bounds_mut = self.0.out_of_bounds.borrow_mut();
            let mut out_of_bounds = Rc::try_unwrap(mem::take(&mut *out_of_bounds_mut)).unwrap_or_else(|rc| (*rc).clone());

            for (id, insert) in out_of_bounds_update.drain(..) {
                if insert {
                    out_of_bounds.push(id);
                } else if let Some(i) = out_of_bounds.iter().position(|i| *i == id) {
                    out_of_bounds.swap_remove(i);
                }
            }
            *out_of_bounds_mut = Rc::new(out_of_bounds);
        }

        self.0.scale_factor.set(scale_factor);
    }

    pub(crate) fn after_render_update(&self, frame_id: FrameId) {
        self.after_render(frame_id, self.0.scale_factor.get());
    }
}
impl fmt::Debug for WidgetInfoTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let nl = if f.alternate() { "\n   " } else { " " };

        write!(
            f,
            "WidgetInfoTree(Rc<{{{nl}window_id: {},{nl}widget_count: {},{nl}...}}>)",
            self.0.window_id,
            self.0.lookup.len(),
            nl = nl
        )
    }
}

#[derive(Default, Debug)]
struct WidgetBoundsData {
    prev_offsets_pass: Cell<LayoutPassId>,
    prev_outer_offset: Cell<PxVector>,
    prev_inner_offset: Cell<PxVector>,
    prev_child_offset: Cell<PxVector>,
    working_pass: Cell<LayoutPassId>,

    outer_offset: Cell<PxVector>,
    inner_offset: Cell<PxVector>,
    child_offset: Cell<PxVector>,
    offsets_pass: Cell<LayoutPassId>,

    childs_changed: Cell<bool>,

    measure_outer_size: Cell<PxSize>,
    outer_size: Cell<PxSize>,
    inner_size: Cell<PxSize>,
    baseline: Cell<Px>,
    inner_offset_baseline: Cell<bool>,

    measure_metrics: Cell<Option<LayoutMetricsSnapshot>>,
    measure_metrics_used: Cell<LayoutMask>,
    metrics: Cell<Option<LayoutMetricsSnapshot>>,
    metrics_used: Cell<LayoutMask>,

    outer_transform: Cell<PxTransform>,
    inner_transform: Cell<PxTransform>,
    rendered: Cell<Option<(ZIndex, ZIndex)>>,

    outer_bounds: Cell<PxRect>,
    inner_bounds: Cell<PxRect>,

    hit_clips: RefCell<HitTestClips>,
    hit_index: Cell<u32>,

    is_in_bounds: Cell<bool>,
}

/// Shared reference to layout size and offsets of a widget and rendered transforms and bounds.
///
/// Can be retrieved in the [`WidgetContextInfo`] and [`WidgetInfo`].
#[derive(Default, Clone, Debug)]
pub struct WidgetBoundsInfo(Rc<WidgetBoundsData>);
impl WidgetBoundsInfo {
    /// New default.
    pub fn new() -> Self {
        Self::default()
    }

    /// Constructor for tests.
    #[cfg(test)]
    #[cfg_attr(doc_nightly, doc(cfg(test)))]
    pub fn new_test(
        inner: PxRect,
        outer: Option<PxRect>,
        outer_transform: Option<PxTransform>,
        inner_transform: Option<PxTransform>,
        rendered: Option<ZIndex>,
    ) -> Self {
        let r = Self::default();
        r.set_inner_offset(inner.origin.to_vector());
        r.set_inner_size(inner.size);

        if let Some(outer) = outer {
            r.set_outer_offset(outer.origin.to_vector());
            r.set_outer_size(outer.size);
        }

        if let Some(transform) = outer_transform {
            r.init_outer_transform(transform);
        }
        if let Some(transform) = inner_transform {
            r.init_inner_transform(transform);
        }

        r.init_rendered(rendered);

        r
    }

    /// New info with bound sizes known.
    pub fn new_size(outer: PxSize, inner: PxSize) -> Self {
        let me = Self::new();
        me.set_outer_size(outer);
        me.set_inner_size(inner);
        me
    }

    /// Gets the widget's outer bounds offset inside the parent widget.
    pub fn outer_offset(&self) -> PxVector {
        self.0.outer_offset.get()
    }

    pub(crate) fn measure_outer_size(&self) -> PxSize {
        self.0.measure_outer_size.get()
    }

    /// Gets the widget's outer bounds size.
    pub fn outer_size(&self) -> PxSize {
        self.0.outer_size.get()
    }

    /// Gets the widget's inner bounds offset inside the outer bounds.
    ///
    /// If [`inner_offset_baseline`] is `true` the [`baseline`] is added from this value.
    ///
    /// [`inner_offset_baseline`]: Self::baseline
    /// [`baseline`]: Self::baseline
    pub fn inner_offset(&self) -> PxVector {
        let mut r = self.0.inner_offset.get();
        if self.inner_offset_baseline() {
            r.y += self.baseline();
        }
        r
    }

    /// If the [`baseline`] is added from the [`inner_offset`].
    ///
    /// [`baseline`]: Self::baseline
    /// [`inner_offset`]: Self::inner_offset
    pub fn inner_offset_baseline(&self) -> bool {
        self.0.inner_offset_baseline.get()
    }

    /// Gets the widget's child offset inside the inner bounds.
    ///
    /// If the widget's child is another widget this is zero and the offset is added to that child's outer offset instead.
    pub fn child_offset(&self) -> PxVector {
        self.0.child_offset.get()
    }

    /// Gets the widget's inner bounds size.
    pub fn inner_size(&self) -> PxSize {
        self.0.inner_size.get()
    }

    /// The baseline offset up from the inner bounds bottom line.
    ///
    /// Note that if [`inner_offset_baseline`] is `true` the [`inner_offset`] is already added by the baseline.
    ///
    /// [`inner_offset_baseline`]: Self::inner_offset_baseline
    /// [`inner_offset`]: Self::inner_offset
    pub fn baseline(&self) -> Px {
        self.0.baseline.get()
    }

    /// Gets the global transform of the widget's outer bounds during the last render or render update.
    pub fn outer_transform(&self) -> PxTransform {
        self.0.outer_transform.get()
    }

    /// Gets the global transform of the widget's inner bounds during the last render or render update.
    pub fn inner_transform(&self) -> PxTransform {
        self.0.inner_transform.get()
    }

    /// Get the z-index of the widget in the latest window frame if it was rendered.
    ///
    /// Note that widgets can render in the back and front of each descendant, these indexes are the *back-most* index, the moment
    /// the [`FrameBuilder::push_widget`] was called for the widget and the *front-most* index, the moment the `push_widget` finishes.
    ///
    /// [`FrameBuilder::push_widget`]: crate::render::FrameBuilder::push_widget
    pub fn rendered(&self) -> Option<(ZIndex, ZIndex)> {
        self.0.rendered.get()
    }

    /// Gets if the [`inner_bounds`] are fully inside the parent inner bounds.
    ///
    /// [`inner_bounds`]: Self::inner_bounds
    pub fn is_in_bounds(&self) -> bool {
        self.0.is_in_bounds.get()
    }

    pub(super) fn set_rendered(&self, rendered: Option<(ZIndex, ZIndex)>, info: &WidgetInfoTree) {
        if self.0.rendered.get().is_some() != rendered.is_some() {
            info.visibility_changed();
        }
        self.0.rendered.set(rendered);
    }
    #[cfg(test)]
    fn init_rendered(&self, rendered: Option<ZIndex>) {
        self.0.rendered.set(rendered.map(|i| (i, i)));
    }

    pub(super) fn set_outer_transform(&self, transform: PxTransform, info: &WidgetInfoTree) {
        let bounds = transform
            .outer_transformed(PxBox::from_size(self.outer_size()))
            .unwrap_or_default()
            .to_rect();

        if self.0.outer_bounds.get().size.is_empty() != bounds.size.is_empty() {
            info.visibility_changed();
        }

        self.0.outer_bounds.set(bounds);
        self.0.outer_transform.set(transform);
    }
    #[cfg(test)]
    fn init_outer_transform(&self, transform: PxTransform) {
        let bounds = transform
            .outer_transformed(PxBox::from_size(self.outer_size()))
            .unwrap_or_default()
            .to_rect();

        self.0.outer_bounds.set(bounds);
        self.0.outer_transform.set(transform);
    }

    pub(super) fn set_inner_transform(
        &self,
        transform: PxTransform,
        info: &WidgetInfoTree,
        widget_id: WidgetId,
        parent_inner: Option<PxRect>,
    ) {
        let bounds = transform
            .outer_transformed(PxBox::from_size(self.inner_size()))
            .unwrap_or_default()
            .to_rect();

        if self.0.inner_bounds.get() != bounds {
            self.0.inner_bounds.set(bounds);
            info.bounds_changed();
        }
        let in_bounds = parent_inner.map(|r| r.contains_rect(&bounds)).unwrap_or(true);
        if self.0.is_in_bounds.get() != in_bounds {
            self.0.is_in_bounds.set(in_bounds);
            info.in_bounds_changed(widget_id, in_bounds);
        }

        self.0.inner_transform.set(transform);
    }

    #[cfg(test)]
    fn init_inner_transform(&self, transform: PxTransform) {
        let bounds = transform
            .outer_transformed(PxBox::from_size(self.inner_size()))
            .unwrap_or_default()
            .to_rect();

        self.0.inner_bounds.set(bounds);
        self.0.inner_transform.set(transform);
    }

    /// Outer bounding box, updated after every render.
    pub fn outer_bounds(&self) -> PxRect {
        self.0.outer_bounds.get()
    }

    /// Calculate the bounding box that envelops the actual size and position of the inner bounds last rendered.
    pub fn inner_bounds(&self) -> PxRect {
        self.0.inner_bounds.get()
    }

    /// Last layout pass that updated the offsets or any of the descendant offsets.
    ///
    /// The version is different every time any of the offsets on the widget or descendants changes after a layout update.
    /// Widget implementers can use this version when optimizing `render` and `render_update`, the [`implicit_base::nodes::widget`]
    /// widget does this.
    ///
    /// [`implicit_base::nodes::widget`]: crate::widget_base::implicit_base::nodes::widget
    pub fn offsets_pass(&self) -> LayoutPassId {
        if self.0.childs_changed.get() {
            self.0.working_pass.get()
        } else {
            self.0.offsets_pass.get()
        }
    }

    /// Snapshot of the [`LayoutMetrics`] on the last layout.
    ///
    /// The [`metrics_used`] value indicates what fields where actually used in the last layout.
    ///
    /// Is `None` if the widget is collapsed.
    ///
    /// [`LayoutMetrics`]: crate::context::LayoutMetrics
    /// [`metrics_used`]: Self::metrics_used
    pub fn metrics(&self) -> Option<LayoutMetricsSnapshot> {
        self.0.metrics.get()
    }

    /// All [`metrics`] fields used by the widget or descendants on the last layout.
    ///
    /// [`metrics`]: Self::metrics
    pub fn metrics_used(&self) -> LayoutMask {
        self.0.metrics_used.get()
    }

    /// Gets the relative hit-test Z for `window_point` against the hit-test shapes rendered for the widget.
    pub fn hit_test_z(&self, window_point: PxPoint) -> RelativeHitZ {
        let hit_clips = self.0.hit_clips.borrow();
        if hit_clips.is_hit_testable() {
            hit_clips.hit_test_z(&self.0.inner_transform.get(), window_point)
        } else {
            RelativeHitZ::NoHit
        }
    }

    /// Index of this widget in the parent hit-test items.
    fn hit_test_index(&self) -> usize {
        self.0.hit_index.get() as usize
    }

    /// Returns `true` if a hit-test clip that affects the `child` removes the `window_point` hit on the child.
    pub fn hit_test_clip_child(&self, child: WidgetInfo, window_point: PxPoint) -> bool {
        let hit_clips = self.0.hit_clips.borrow();
        if hit_clips.is_hit_testable() {
            hit_clips.clip_child(child.bounds_info().hit_test_index(), &self.0.inner_transform.get(), window_point)
        } else {
            false
        }
    }

    pub(crate) fn update_hit_test_transform(&self, value: FrameValue<PxTransform>) {
        self.0.hit_clips.borrow_mut().update_transform(value);
    }

    pub(crate) fn measure_metrics(&self) -> Option<LayoutMetricsSnapshot> {
        self.0.measure_metrics.get()
    }
    pub(crate) fn measure_metrics_used(&self) -> LayoutMask {
        self.0.measure_metrics_used.get()
    }

    fn begin_pass(&self, pass: LayoutPassId) {
        // Record current state as previous state on the first call of the `pass`, see `Self::end_pass`.

        if self.0.working_pass.get() != pass {
            self.0.working_pass.set(pass);
            self.0.childs_changed.set(false);

            self.0.prev_outer_offset.set(self.0.outer_offset.get());
            self.0.prev_inner_offset.set(self.0.inner_offset.get());
            self.0.prev_child_offset.set(self.0.child_offset.get());
            self.0.prev_offsets_pass.set(self.0.offsets_pass.get());
        }
    }

    fn end_pass(&self) -> i32 {
        // Check for changes against the previously recorded values, returns an offset to add to the parent's
        // changed child counter.
        //
        // How this works:
        //
        // Begin/end pass can be called multiple times in a "true" layout pass, due to intrinsic second passes or the
        // usage of `with_outer`, so an end pass can detect an intermediary value change, and return +1 to add to the parent,
        // then on the *intrinsic pass*, it detects that actually there was no change, and return -1 to fix the parent count.

        // if actually changed from previous global pass
        let changed = self.0.prev_outer_offset.get() != self.0.outer_offset.get()
            || self.0.prev_inner_offset.get() != self.0.inner_offset.get()
            || self.0.prev_child_offset.get() != self.0.child_offset.get();

        // if already processed one end_pass request and returned +1
        let believed_changed = self.0.offsets_pass.get() == self.0.working_pass.get();

        if changed {
            if believed_changed {
                0 // already updated, no need to add to the parent counter.
            } else {
                //
                self.0.offsets_pass.set(self.0.working_pass.get());
                1
            }
        } else if believed_changed {
            self.0.offsets_pass.set(self.0.prev_offsets_pass.get());
            -1 // second intrinsic pass returned value to previous, need to remove one from the parent counter.
        } else {
            0 // did not update the parent incorrectly.
        }
    }

    fn set_changed_child(&self) {
        self.0.childs_changed.set(true);
    }

    fn set_outer_offset(&self, offset: PxVector) {
        self.0.outer_offset.set(offset);
    }

    fn set_outer_size(&self, size: PxSize) {
        self.0.outer_size.set(size);
    }

    pub(crate) fn set_measure_outer_size(&self, size: PxSize) {
        self.0.measure_outer_size.set(size);
    }

    fn set_inner_offset(&self, offset: PxVector) {
        self.0.inner_offset.set(offset);
    }

    fn set_child_offset(&self, offset: PxVector) {
        self.0.child_offset.set(offset);
    }

    fn set_inner_size(&self, size: PxSize) {
        self.0.inner_size.set(size);
    }

    fn set_baseline(&self, baseline: Px) {
        self.0.baseline.set(baseline);
    }

    fn set_inner_offset_baseline(&self, enabled: bool) {
        self.0.inner_offset_baseline.set(enabled);
    }

    fn set_metrics(&self, metrics: Option<LayoutMetricsSnapshot>, used: LayoutMask) {
        self.0.metrics.set(metrics);
        self.0.metrics_used.set(used);
    }

    pub(crate) fn set_measure_metrics(&self, metrics: Option<LayoutMetricsSnapshot>, used: LayoutMask) {
        self.0.measure_metrics.set(metrics);
        self.0.measure_metrics_used.set(used);
    }

    pub(crate) fn set_hit_clips(&self, clips: HitTestClips) {
        *self.0.hit_clips.borrow_mut() = clips;
    }

    pub(crate) fn set_hit_index(&self, index: usize) {
        self.0.hit_index.set(index as u32);
    }
}

#[derive(Default, Debug)]
struct WidgetBorderData {
    offsets: Cell<PxSideOffsets>,
    corner_radius: Cell<PxCornerRadius>,
}

/// Shared reference to the combined *border* and corner radius of a [`WidgetInfo`].
#[derive(Default, Clone, Debug)]
pub struct WidgetBorderInfo(Rc<WidgetBorderData>);
impl WidgetBorderInfo {
    /// New default.
    pub fn new() -> Self {
        Self::default()
    }

    /// Constructor for tests.
    #[cfg(test)]
    #[cfg_attr(doc_nightly, doc(cfg(test)))]
    pub fn new_test(offsets: PxSideOffsets, corner_radius: PxCornerRadius) -> Self {
        let r = Self::default();
        r.set_offsets(offsets);
        r.set_corner_radius(corner_radius);
        r
    }

    /// Sum of the widths of all borders set on the widget.
    pub fn offsets(&self) -> PxSideOffsets {
        self.0.offsets.get()
    }

    /// Corner radius set on the widget, this is the *outer* curve of border corners.
    pub fn corner_radius(&self) -> PxCornerRadius {
        self.0.corner_radius.get()
    }

    /// Computes the [`corner_radius`] deflated by [`offsets`], this is the *inner* curve of border corners.
    ///
    /// [`corner_radius`]: Self::corner_radius
    /// [`offsets`]: Self::offsets
    pub fn inner_corner_radius(&self) -> PxCornerRadius {
        self.corner_radius().deflate(self.offsets())
    }

    /// Compute the inner offset plus [`offsets`] left, top.
    ///
    /// [`offsets`]: Self::offsets
    pub fn inner_offset(&self, bounds: &WidgetBoundsInfo) -> PxVector {
        let o = self.offsets();
        let o = PxVector::new(o.left, o.top);
        bounds.inner_offset() + o
    }

    /// Compute the inner size offset by [`offsets`].
    ///
    /// [`offsets`]: Self::offsets
    pub fn inner_size(&self, bounds: &WidgetBoundsInfo) -> PxSize {
        let o = self.offsets();
        bounds.inner_size() - PxSize::new(o.horizontal(), o.vertical())
    }

    /// Compute the inner transform offset by the [`offsets`].
    ///
    /// [`offsets`]: Self::offsets
    pub fn inner_transform(&self, bounds: &WidgetBoundsInfo) -> PxTransform {
        let o = self.offsets();
        let o = PxVector::new(o.left, o.top);
        bounds.inner_transform().pre_translate(o.cast())
    }

    pub(super) fn set_offsets(&self, widths: PxSideOffsets) {
        self.0.offsets.set(widths);
    }

    pub(super) fn set_corner_radius(&self, radius: PxCornerRadius) {
        self.0.corner_radius.set(radius)
    }
}

#[derive(Clone)]
struct WidgetInfoData {
    widget_id: WidgetId,
    bounds_info: WidgetBoundsInfo,
    border_info: WidgetBorderInfo,
    meta: Rc<OwnedStateMap>,
    interactivity_filters: InteractivityFilters,
    interactivity_cache: Cell<Option<Interactivity>>,
    local_interactivity: Cell<Interactivity>,
}
impl fmt::Debug for WidgetInfoData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WidgetInfoData")
            .field("widget_id", &self.widget_id)
            .finish_non_exhaustive()
    }
}

/// Reference to a widget info in a [`WidgetInfoTree`].
#[derive(Clone, Copy)]
pub struct WidgetInfo<'a> {
    tree: &'a WidgetInfoTree,
    node_id: tree::NodeId,
}
impl<'a> PartialEq for WidgetInfo<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.node_id == other.node_id && self.tree == other.tree
    }
}
impl<'a> Eq for WidgetInfo<'a> {}
impl<'a> std::hash::Hash for WidgetInfo<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.node_id, state)
    }
}
impl<'a> std::fmt::Debug for WidgetInfo<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WidgetInfo")
            .field("[path]", &self.path().to_string())
            .field("[meta]", self.meta())
            .finish_non_exhaustive()
    }
}

impl<'a> WidgetInfo<'a> {
    fn new(tree: &'a WidgetInfoTree, node_id: tree::NodeId) -> Self {
        Self { tree, node_id }
    }

    fn node(&self) -> tree::NodeRef<'a, WidgetInfoData> {
        self.tree.0.tree.index(self.node_id)
    }

    fn info(&self) -> &'a WidgetInfoData {
        self.node().value()
    }

    /// Widget id.
    pub fn widget_id(self) -> WidgetId {
        self.info().widget_id
    }

    /// Full path to this widget.
    pub fn path(self) -> WidgetPath {
        let mut path: Vec<_> = self.ancestors().map(|a| a.widget_id()).collect();
        path.reverse();
        path.push(self.widget_id());

        WidgetPath::new(self.tree.0.window_id, path)
    }

    /// Full path to this widget with [`interactivity`] values.
    ///
    /// [`interactivity`]: Self::interactivity
    pub fn interaction_path(self) -> InteractionPath {
        let mut path = vec![];

        let mut blocked = None;
        let mut disabled = None;

        for w in self.self_and_ancestors() {
            let intera = w.interactivity();
            if intera.contains(Interactivity::BLOCKED) {
                blocked = Some(path.len());
            }
            if intera.contains(Interactivity::DISABLED) {
                disabled = Some(path.len());
            }

            path.push(w.widget_id());
        }
        path.reverse();

        let len = path.len();

        let path = WidgetPath::new(self.tree.0.window_id, path);
        InteractionPath::new_internal(
            path,
            blocked.map(|i| len - i - 1).unwrap_or(len),
            disabled.map(|i| len - i - 1).unwrap_or(len),
        )
    }

    /// Gets the [`path`] if it is different from `old_path`.
    ///
    /// Only allocates a new path if needed.
    ///
    /// # Panics
    ///
    /// If `old_path` does not point to the same widget id as `self`.
    ///
    /// [`path`]: Self::path
    pub fn new_path(self, old_path: &WidgetPath) -> Option<WidgetPath> {
        assert_eq!(old_path.widget_id(), self.widget_id());
        if self
            .ancestors()
            .zip(old_path.ancestors().iter().rev())
            .any(|(ancestor, id)| ancestor.widget_id() != *id)
        {
            Some(self.path())
        } else {
            None
        }
    }

    /// Gets the [`interaction_path`] if it is different from `old_path`.
    ///
    /// Only allocates a new path if needed.
    ///
    /// Panics
    ///
    /// If `old_path` does not point to the same widget id as `self`.
    ///
    /// [`interaction_path`]: Self::interaction_path
    pub fn new_interaction_path(self, old_path: &InteractionPath) -> Option<InteractionPath> {
        assert_eq!(old_path.widget_id(), self.widget_id());

        if self.interactivity() != old_path.interactivity()
            || self
                .ancestors()
                .zip(old_path.zip())
                .any(|(anc, (id, int))| anc.widget_id() != id || anc.interactivity() != int)
        {
            Some(self.interaction_path())
        } else {
            None
        }
    }

    /// Get the z-index of the widget in the latest window frame if it was rendered.
    ///
    /// Note that widgets can render in the back and front of each descendant, these indexes are the *back-most* index, the moment
    /// the widget starts rendering and the *front-most* index at the moment the widget and all contents finishes rendering.
    ///
    /// This value is updated every [`render`] without causing a tree rebuild.
    ///
    /// [`render`]: crate::UiNode::render
    pub fn rendered(self) -> Option<(ZIndex, ZIndex)> {
        self.info().bounds_info.rendered()
    }

    /// Compute the visibility of the widget or the widget's descendants.
    ///
    /// If is [`rendered`] is [`Visible`], if not and the [`bounds_info`] outer size is zero then is [`Collapsed`] else
    /// is [`Hidden`].
    ///
    /// [`rendered`]: Self::rendered
    /// [`Visible`]: Visibility::Visible
    /// [`bounds_info`]: Self::bounds_info
    /// [`Collapsed`]: Visibility::Collapsed
    /// [`Hidden`]: Visibility::Hidden
    pub fn visibility(self) -> Visibility {
        if self.rendered().is_some() {
            Visibility::Visible
        } else if self.info().bounds_info.outer_size() == PxSize::zero() {
            Visibility::Collapsed
        } else {
            Visibility::Hidden
        }
    }

    /// Get or compute the interactivity of the widget.
    ///
    /// The interactivity of a widget is the combined result of all interactivity filters applied to it and its ancestors.
    /// If a parent is blocked this is blocked, same for disabled.
    pub fn interactivity(self) -> Interactivity {
        if let Some(cache) = self.info().interactivity_cache.get() {
            cache
        } else {
            let mut interactivity = self.info().local_interactivity.get();

            if interactivity != Interactivity::BLOCKED_DISABLED {
                interactivity |= self.parent().map(|n| n.interactivity()).unwrap_or(Interactivity::ENABLED);
                if interactivity != Interactivity::BLOCKED_DISABLED {
                    for filter in &self.tree.0.interactivity_filters {
                        interactivity |= filter(&InteractivityFilterArgs { info: self });
                        if interactivity == Interactivity::BLOCKED_DISABLED {
                            break;
                        }
                    }
                }
            }

            self.info().interactivity_cache.set(Some(interactivity));
            interactivity
        }
    }

    /// All the transforms introduced by this widget, starting from the outer info.
    ///
    /// This information is up-to-date, it is updated every layout and render without causing a tree rebuild.
    pub fn bounds_info(self) -> WidgetBoundsInfo {
        self.info().bounds_info.clone()
    }

    /// Clone a reference to the widget border and corner radius information.
    ///
    /// This information is up-to-date, it is updated every layout without causing a tree rebuild.
    pub fn border_info(self) -> WidgetBorderInfo {
        self.info().border_info.clone()
    }

    /// Size of the widget outer area, not transformed.
    ///
    /// Returns an up-to-date size, the size is updated every layout without causing a tree rebuild.
    pub fn outer_size(self) -> PxSize {
        self.info().bounds_info.outer_size()
    }

    /// Size of the widget inner area, not transformed.
    ///
    /// Returns an up-to-date size, the size is updated every layout without causing a tree rebuild.
    pub fn inner_size(self) -> PxSize {
        self.info().bounds_info.inner_size()
    }

    /// Size of the widget child area, not transformed.
    ///
    /// Returns an up-to-date size, the size is updated every layout without causing a tree rebuild.
    pub fn inner_border_size(self) -> PxSize {
        let info = self.info();
        info.border_info.inner_size(&info.bounds_info)
    }

    /// Gets the baseline offset up from the inner bounds bottom line.
    pub fn baseline(self) -> Px {
        self.info().bounds_info.baseline()
    }

    /// Widget outer transform in window space.
    ///
    /// Returns an up-to-date transform, the transform is updated every render or render update without causing a tree rebuild.
    pub fn outer_transform(self) -> PxTransform {
        self.info().bounds_info.outer_transform()
    }

    /// Widget inner transform in the window space.
    ///
    /// Returns an up-to-date transform, the transform is updated every render or render update without causing a tree rebuild.
    pub fn inner_transform(self) -> PxTransform {
        self.info().bounds_info.inner_transform()
    }

    /// Widget outer rectangle in the window space.
    ///
    /// Returns an up-to-date rect, the bounds are updated every render or render update without causing a tree rebuild.
    pub fn outer_bounds(self) -> PxRect {
        let info = self.info();
        info.bounds_info.outer_bounds()
    }

    /// Widget inner rectangle in the window space.
    ///
    /// Returns an up-to-date rect, the bounds are updated every render or render update without causing a tree rebuild.
    pub fn inner_bounds(self) -> PxRect {
        let info = self.info();
        info.bounds_info.inner_bounds()
    }

    /// Widget inner bounds center in the window space.
    pub fn center(self) -> PxPoint {
        self.inner_bounds().center()
    }

    /// Custom metadata associated with the widget during info build.
    pub fn meta(self) -> &'a StateMap {
        &self.info().meta.0
    }

    /// Reference the [`WidgetInfoTree`] that owns `self`.
    pub fn tree(self) -> &'a WidgetInfoTree {
        self.tree
    }

    /// Reference to the root widget.
    pub fn root(self) -> Self {
        self.tree.root()
    }

    /// Reference to the widget that contains this widget.
    ///
    /// Is `None` only for [`root`](WidgetInfoTree::root).
    pub fn parent(self) -> Option<Self> {
        self.node().parent().map(move |n| WidgetInfo::new(self.tree, n.id()))
    }

    /// Reference to the previous widget within the same parent.
    pub fn prev_sibling(self) -> Option<Self> {
        self.node().prev_sibling().map(move |n| WidgetInfo::new(self.tree, n.id()))
    }

    /// Reference to the next widget within the same parent.
    pub fn next_sibling(self) -> Option<Self> {
        self.node().next_sibling().map(move |n| WidgetInfo::new(self.tree, n.id()))
    }

    /// Reference to the first widget within this widget.
    pub fn first_child(self) -> Option<Self> {
        self.node().first_child().map(move |n| WidgetInfo::new(self.tree, n.id()))
    }

    /// Reference to the last widget within this widget.
    pub fn last_child(self) -> Option<Self> {
        self.node().last_child().map(move |n| WidgetInfo::new(self.tree, n.id()))
    }

    /// If the parent widget has multiple children.
    pub fn has_siblings(self) -> bool {
        self.node().has_siblings()
    }

    /// If the widget has at least one child.
    pub fn has_children(self) -> bool {
        self.node().has_children()
    }

    /// All parent children except this widget.
    pub fn siblings(self) -> impl Iterator<Item = WidgetInfo<'a>> {
        self.prev_siblings().chain(self.next_siblings())
    }

    /// Iterator over the direct descendants of the widget.
    pub fn children(self) -> iter::Children<'a> {
        let mut r = self.self_and_children();
        r.next();
        r.next_back();
        r
    }

    /// Count of [`children`].
    ///
    /// [`children`]: Self::children
    pub fn children_count(self) -> usize {
        self.node().children_count()
    }

    /// Iterator over the widget and the direct descendants of the widget.
    pub fn self_and_children(self) -> iter::Children<'a> {
        iter::Children::new(self)
    }

    /// Iterator over all widgets contained by this widget.
    pub fn descendants(self) -> iter::TreeIter<'a> {
        let mut d = self.self_and_descendants();
        d.next();
        d
    }

    /// Total number of [`descendants`].
    ///
    /// [`descendants`]: Self::descendants
    pub fn descendants_len(self) -> usize {
        self.node().descendants_range().len()
    }

    /// Iterator over the widget and all widgets contained by it.
    pub fn self_and_descendants(self) -> iter::TreeIter<'a> {
        iter::TreeIter::self_and_descendants(self)
    }

    /// Iterator over parent -> grandparent -> .. -> root.
    pub fn ancestors(self) -> iter::Ancestors<'a> {
        let mut r = self.self_and_ancestors();
        r.next();
        r
    }

    /// Create an object that can check if widgets are descendant of `self` in O(1) time.
    pub fn descendants_range(self) -> WidgetDescendantsRange<'a> {
        WidgetDescendantsRange {
            _tree: PhantomData,
            range: self.node().descendants_range(),
        }
    }

    /// If `self` is an ancestor of `maybe_descendant`.
    pub fn is_ancestor(self, maybe_descendant: WidgetInfo<'a>) -> bool {
        self.descendants_range().contains(maybe_descendant)
    }

    /// If `self` is inside `maybe_ancestor`.
    pub fn is_descendant(self, maybe_ancestor: WidgetInfo<'a>) -> bool {
        maybe_ancestor.descendants_range().contains(self)
    }

    /// Iterator over self -> parent -> grandparent -> .. -> root.
    pub fn self_and_ancestors(self) -> iter::Ancestors<'a> {
        iter::Ancestors::new(self)
    }

    /// Iterator over all previous widgets within the same parent.
    pub fn prev_siblings(self) -> iter::PrevSiblings<'a> {
        let mut r = self.self_and_prev_siblings();
        r.next();
        r
    }

    /// Iterator over self and all previous widgets within the same parent.
    pub fn self_and_prev_siblings(self) -> iter::PrevSiblings<'a> {
        iter::PrevSiblings::new(self)
    }

    /// Iterator over all next widgets within the same parent.
    pub fn next_siblings(self) -> iter::NextSiblings<'a> {
        let mut r = self.self_and_next_siblings();
        r.next();
        r
    }

    /// Iterator over self and all next widgets within the same parent.
    pub fn self_and_next_siblings(self) -> iter::NextSiblings<'a> {
        iter::NextSiblings::new(self)
    }

    /// Iterator over all previous widgets within the same `ancestor`, including descendants of siblings.
    ///
    /// If `ancestor` is not actually an ancestor iterates to the root.
    pub fn prev_siblings_in(self, ancestor: WidgetInfo<'a>) -> iter::RevTreeIter<'a> {
        iter::TreeIter::prev_siblings_in(self, ancestor)
    }

    /// Iterator over self, descendants and all previous widgets within the same `ancestor`.
    ///
    /// If `ancestor` is not actually an ancestor iterates to the root.
    pub fn self_and_prev_siblings_in(self, ancestor: WidgetInfo<'a>) -> iter::RevTreeIter<'a> {
        iter::TreeIter::self_and_prev_siblings_in(self, ancestor)
    }

    /// Iterator over all next widgets within the same `ancestor`, including descendants of siblings.
    ///
    /// If `ancestor` is not actually an ancestor iterates to the root.
    pub fn next_siblings_in(self, ancestor: WidgetInfo<'a>) -> iter::TreeIter<'a> {
        iter::TreeIter::next_siblings_in(self, ancestor)
    }

    /// Iterator over self, descendants and all next widgets within the same `ancestor`.
    ///
    /// If `ancestor` is not actually an ancestor iterates to the root.
    pub fn self_and_next_siblings_in(self, ancestor: WidgetInfo<'a>) -> iter::TreeIter<'a> {
        iter::TreeIter::self_and_next_siblings_in(self, ancestor)
    }

    /// The [`center`] orientation in relation to a `origin`.
    ///
    /// Returns `None` if the `origin` is the center.
    ///
    /// [`center`]: Self::center
    pub fn orientation_from(self, origin: PxPoint) -> Option<Orientation2D> {
        let o = self.center();
        for &d in &[
            Orientation2D::Above,
            Orientation2D::Right,
            Orientation2D::Below,
            Orientation2D::Left,
        ] {
            if d.is(origin, o) {
                return Some(d);
            }
        }
        None
    }

    /// All the parent's children except this widget, sorted by the [`distance_key`] to this widget's center.
    ///
    /// [`distance_key`]: Self::distance_key
    pub fn closest_siblings(self) -> Vec<WidgetInfo<'a>> {
        let mut vec: Vec<_> = self.siblings().collect();
        let origin = self.center();
        vec.sort_by_key(|w| w.distance_key(origin));
        vec
    }

    /// Value that indicates the distance between this widget center and `origin`.
    pub fn distance_key(self, origin: PxPoint) -> DistanceKey {
        DistanceKey::from_points(origin, self.center())
    }

    /// Count of ancestors.
    pub fn depth(self) -> usize {
        self.ancestors().count()
    }

    /// First ancestor of `self` and `other`.
    ///
    /// Returns `None` if `other` is not from the same tree.
    pub fn shared_ancestor(self, other: Self) -> Option<WidgetInfo<'a>> {
        if self.tree == other.tree {
            let a = self.path();
            let b = other.path();
            let shared = a.shared_ancestor(&b).unwrap();
            self.tree.get(shared.widget_id())
        } else {
            None
        }
    }

    /// Gets Z-index a hit-test of `point` against the hit-test shapes rendered for this widget and hit-test clips of parent widgets.
    ///
    /// A hit happens if the point is inside [`inner_bounds`] and at least one hit-test shape rendered for the widget contains the point.
    ///
    /// [`inner_bounds`]: WidgetInfo::inner_bounds
    fn hit_test_z(self, point: PxPoint) -> Option<ZIndex> {
        let bounds = &self.info().bounds_info;
        if bounds.inner_bounds().contains(point) {
            let z = match bounds.hit_test_z(point) {
                RelativeHitZ::NoHit => None,
                RelativeHitZ::Back => bounds.rendered().map(|(b, _)| b),
                RelativeHitZ::Over(w) => self.tree.get(w).and_then(|w| w.info().bounds_info.rendered()).map(|(_, f)| f),
                RelativeHitZ::Front => bounds.rendered().map(|(_, f)| f),
            };

            if z.is_some() {
                let mut parent = self.parent();
                let mut child = self;

                while let Some(p) = parent {
                    if p.info().bounds_info.hit_test_clip_child(child, point) {
                        return None;
                    }

                    parent = p.parent();
                    child = p;
                }
            }

            z
        } else {
            None
        }
    }

    /// Returns `true` if this widget's inner bounds are fully contained by the parent inner bounds.
    pub fn is_in_bounds(self) -> bool {
        self.info().bounds_info.is_in_bounds()
    }

    /// Iterator over all descendants with inner bounds not fully contained by their parent inner bounds.
    pub fn out_of_bounds(self) -> impl Iterator<Item = WidgetInfo<'a>> {
        let range = self.descendants_range();
        self.tree.out_of_bounds().filter(move |w| range.contains(*w))
    }

    /// Iterator over self and descendants, first all in-bounds descendants, then all out-of-bounds.
    ///
    /// If the `filter` returns `false` the widget and all it's in-bounds descendants are skipped, otherwise they are yielded. After
    /// all in-bounds descendants reachable from `self` and filtered the iterator changes to each out-of-bounds descendants and their
    /// in-bounds descendants.
    pub fn spatial_iter(self, filter: impl Fn(WidgetInfo<'a>) -> bool + Clone + 'a) -> impl Iterator<Item = WidgetInfo<'a>> {
        self.self_and_descendants()
            .tree_filter(clone_move!(filter, |w| {
                if w.is_in_bounds() && filter(w) {
                    TreeFilter::Include
                } else {
                    TreeFilter::SkipAll
                }
            }))
            .chain(self.out_of_bounds().flat_map(clone_move!(filter, |w| {
                let self_id = w.widget_id();
                w.self_and_descendants().tree_filter(clone_move!(filter, |w| {
                    if (w.is_in_bounds() || w.widget_id() == self_id) && filter(w) {
                        TreeFilter::Include
                    } else {
                        TreeFilter::SkipAll
                    }
                }))
            })))
    }

    /// Iterator over self and all descendants with inner bounds that contain the `point`.
    pub fn inner_contains(self, point: PxPoint) -> impl Iterator<Item = WidgetInfo<'a>> {
        self.spatial_iter(move |w| w.inner_bounds().contains(point))
    }

    /// Spatial iterator over self and descendants with inner bounds that intersects the `rect`.
    pub fn inner_intersects(self, rect: PxRect) -> impl Iterator<Item = WidgetInfo<'a>> {
        let rect = rect.to_box2d();
        self.spatial_iter(move |w| w.inner_bounds().to_box2d().intersects(&rect))
    }

    /// Spatial iterator over self and descendants with inner bounds that fully envelops the `rect`.
    pub fn inner_contains_rect(self, rect: PxRect) -> impl Iterator<Item = WidgetInfo<'a>> {
        let rect = rect.to_box2d();
        self.spatial_iter(move |w| w.inner_bounds().to_box2d().contains_box(&rect))
    }

    /// Spatial iterator over self and descendants with inner bounds that are fully inside the `rect`.
    pub fn inner_contained(self, rect: PxRect) -> impl Iterator<Item = WidgetInfo<'a>> {
        let rect = rect.to_box2d();
        self.spatial_iter(move |w| rect.contains_box(&w.inner_bounds().to_box2d()))
    }

    /// Spatial iterator over self and descendants with center point inside the `area`.
    pub fn center_contained(&self, area: PxRect) -> impl Iterator<Item = WidgetInfo<'a>> {
        let area = area.to_box2d();
        self.spatial_iter(move |w| w.inner_bounds().to_box2d().intersects(&area))
            .filter(move |w| area.contains(w.center()))
    }

    /// Spatial iterator over self and descendants with center point within the `max_radius` of the `origin`.
    pub fn center_in_distance(&self, origin: PxPoint, max_radius: Px) -> impl Iterator<Item = WidgetInfo<'a>> + '_ {
        let area = PxRect::new(origin, PxSize::splat(max_radius))
            .inflate(max_radius, max_radius)
            .to_box2d();

        let distance_key = DistanceKey::from_distance(max_radius);

        self.spatial_iter(move |w| w.inner_bounds().to_box2d().intersects(&area))
            .filter(move |w| w.distance_key(origin) <= distance_key)
    }

    /// Gets all widgets of self and descendants hit by a `point`, sorted by z-index of the hit, front to back.
    pub fn hit_test(self, point: PxPoint) -> HitTestInfo {
        let _span = tracing::trace_span!("hit_test").entered();

        let mut hits: Vec<_> = self
            .inner_contains(point)
            .filter_map(|w| {
                w.hit_test_z(point).map(|z| HitInfo {
                    widget_id: w.widget_id(),
                    z_index: z,
                })
            })
            .collect();

        hits.sort_by(|a, b| b.z_index.cmp(&a.z_index));

        HitTestInfo {
            window_id: self.tree.0.window_id,
            frame_id: self.tree.0.stats.borrow().last_frame,
            point,
            hits,
        }
    }

    /// Find the descendant with center point nearest of `origin` within the `max_radius`.
    ///
    /// This method is faster than using sorting the result of [`center_in_distance`], but is slower if any point in distance is acceptable.
    ///
    /// [`center_in_distance`]: Self::center_in_distance
    pub fn nearest(self, origin: PxPoint, max_radius: Px) -> Option<WidgetInfo<'a>> {
        self.nearest_filtered(origin, max_radius, |_| true)
    }

    /// Find the widget, self or descendant, with center point nearest of `origin` within the `max_radius` and approved by the `filter` closure.
    pub fn nearest_filtered(self, origin: PxPoint, max_radius: Px, filter: impl FnMut(WidgetInfo<'a>) -> bool) -> Option<WidgetInfo<'a>> {
        self.nearest_bounded_filtered(origin, max_radius, self.tree.spatial_bounds(), filter)
    }

    /// Find the widget, self or descendant, with center point nearest of `origin` within the `max_radius` and inside `bounds`;
    /// and approved by the `filter` closure.
    pub fn nearest_bounded_filtered(
        self,
        origin: PxPoint,
        max_radius: Px,
        bounds: PxRect,
        mut filter: impl FnMut(WidgetInfo<'a>) -> bool,
    ) -> Option<WidgetInfo<'a>> {
        // search quadrants of `128` -> `256` -> .. until one quadrant finds at least a widget centered in it,
        // the nearest widget centered in the smallest quadrant is selected.
        let max_quad = self.tree.spatial_bounds().intersection(&bounds)?;

        let mut source_quad = PxRect::new(origin - PxVector::splat(Px(64)), PxSize::splat(Px(128)));
        let mut search_quad = source_quad.intersection(&max_quad)?;

        let max_diameter = max_radius * Px(2);

        let mut dist = if max_radius != Px::MAX {
            DistanceKey::from_distance(max_radius + Px(1))
        } else {
            DistanceKey::NONE_MAX
        };

        let mut nearest = None;
        loop {
            for w in self.center_contained(search_quad) {
                let w_dist = w.distance_key(origin);
                if w_dist < dist && filter(w) {
                    dist = w_dist;
                    nearest = Some(w);
                }
            }

            let source_width = source_quad.width();
            if nearest.is_some() || source_width >= max_diameter {
                break;
            } else {
                source_quad = source_quad.inflate(source_width, source_width);
                let new_search = match source_quad.intersection(&max_quad) {
                    Some(b) if b != search_quad => b,
                    _ => break, // filled bounds
                };
                search_quad = new_search;
            }
        }

        if nearest.is_some() {
            // ensure that we are not skipping a closer widget because the nearest was in a corner of the search quad.
            let distance = PxVector::splat(Px(2) * dist.distance().unwrap_or(Px(0)));

            let quad = euclid::Box2D::new(origin - distance, origin + distance).intersection_unchecked(&max_quad.to_box2d());

            for w in self.center_contained(quad.to_rect()) {
                let w_dist = w.distance_key(origin);
                if w_dist < dist && filter(w) {
                    dist = w_dist;
                    nearest = Some(w);
                }
            }
        }

        nearest
    }

    /// Spatial iterator over all widgets, self and descendants, with center in the direction defined by `orientation` and
    /// within `max_radius` of  the `origin`, widgets are only visited once and the distance is clipped by the [`bounds`], use [`Px::MAX`]
    /// on the distance to visit all widgets in the direction.
    ///
    /// [`bounds`]: Self::bounds
    pub fn oriented(self, origin: PxPoint, max_distance: Px, orientation: Orientation2D) -> impl Iterator<Item = WidgetInfo<'a>> {
        let distance_bounded = max_distance != Px::MAX;
        let distance_key = if distance_bounded {
            DistanceKey::from_distance(max_distance)
        } else {
            DistanceKey::NONE_MAX
        };
        orientation
            .search_bounds(origin, max_distance, self.tree.spatial_bounds().to_box2d())
            .flat_map(move |sq| self.inner_intersects(sq.to_rect()).map(move |w| (sq, w)))
            .filter_map(move |(sq, w)| {
                let center = w.center();
                if sq.contains(center)
                    && orientation.is(origin, center)
                    && (!distance_bounded || DistanceKey::from_points(origin, center) <= distance_key)
                {
                    Some(w)
                } else {
                    None
                }
            })
    }

    /// Find the widget with center point nearest of `origin` within the `max_distance` and with `orientation` to origin.
    ///
    /// This method is faster than using sorting the result of [`oriented`], but is slower if any point in distance and orientation is acceptable.
    ///
    /// [`oriented`]: Self::oriented
    pub fn nearest_oriented(self, origin: PxPoint, max_distance: Px, orientation: Orientation2D) -> Option<WidgetInfo<'a>> {
        self.nearest_oriented_filtered(origin, max_distance, orientation, |_| true)
    }

    /// Find the widget with center point nearest of `origin` within the `max_distance` and with `orientation` to origin, and approved by the `filter` closure.
    ///
    /// This method is faster than using sorting the result of [`oriented`], but is slower if any point in distance and orientation is acceptable.
    ///
    /// [`oriented`]: Self::oriented
    pub fn nearest_oriented_filtered(
        self,
        origin: PxPoint,
        max_distance: Px,
        orientation: Orientation2D,
        mut filter: impl FnMut(WidgetInfo<'a>) -> bool,
    ) -> Option<WidgetInfo<'a>> {
        let mut dist = DistanceKey::from_distance(max_distance + Px(1));
        let mut nearest = None;
        let mut last_quad = euclid::Box2D::zero();

        for search_quad in orientation.search_bounds(origin, max_distance, self.tree.spatial_bounds().to_box2d()) {
            for w in self.center_contained(search_quad.to_rect()) {
                if orientation.is(origin, w.center()) {
                    let w_dist = w.distance_key(origin);
                    if w_dist < dist && filter(w) {
                        dist = w_dist;
                        nearest = Some(w);
                    }
                }
            }

            if nearest.is_some() {
                last_quad = search_quad;
                break;
            }
        }

        if nearest.is_some() {
            // ensure that we are not skipping a closer widget because the nearest was in a corner of the search quad.

            match orientation {
                Orientation2D::Above => {
                    let extra = last_quad.height() / Px(2);
                    last_quad.max.y = last_quad.min.y;
                    last_quad.min.y -= extra;
                }
                Orientation2D::Right => {
                    let extra = last_quad.width() / Px(2);
                    last_quad.min.x = last_quad.max.x;
                    last_quad.max.x += extra;
                }
                Orientation2D::Below => {
                    let extra = last_quad.height() / Px(2);
                    last_quad.min.y = last_quad.max.y;
                    last_quad.max.y += extra;
                }
                Orientation2D::Left => {
                    let extra = last_quad.width() / Px(2);
                    last_quad.max.x = last_quad.min.x;
                    last_quad.min.x -= extra;
                }
            }

            for w in self.center_contained(last_quad.to_rect()) {
                let w_dist = w.distance_key(origin);
                if w_dist < dist && filter(w) {
                    dist = w_dist;
                    nearest = Some(w);
                }
            }
        }

        nearest
    }
}

/// Data from a previous [`WidgetInfoBuilder`], can be reused in the next rebuild for a performance boost.
pub struct UsedWidgetInfoBuilder {
    tree_capacity: usize,
    interactivity_filters_capacity: usize,
}
impl UsedWidgetInfoBuilder {
    fn fallback() -> Self {
        UsedWidgetInfoBuilder {
            tree_capacity: 100,
            interactivity_filters_capacity: 30,
        }
    }
}

macro_rules! update_slot {
    ($(
        $(#[$meta:meta])*
        $vis:vis struct $Slot:ident -> $Mask:ident;
    )+) => {$(
        $(#[$meta])*
        ///
        /// This `struct` is a single byte that represents an index in the full bitmap.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        $vis struct $Slot(u8);

        impl $Slot {
            /// Gets a slot.
            pub fn next() -> Self {
                thread_local! {
                    static SLOT: Cell<u8> = Cell::new(0);
                }

                let slot = SLOT.with(|s| {
                    let slot = s.get().wrapping_add(1);
                    s.set(slot);
                    slot
                });

                Self(slot)
            }

            /// Gets a mask representing just this slot.
            pub fn mask(self) -> $Mask {
                $Mask::from_slot(self)
            }
        }
    )+}
}
macro_rules! update_mask {
    ($(
        $(#[$meta:meta])*
        $vis:vis struct $Mask:ident <- $Slot:ident;
    )+) => {$(
        $(#[$meta])*
        ///
        /// This `struct` is a 256-bit bitmap of flagged slots.
        #[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
        $vis struct $Mask([u128; 2]);

        impl $Mask {
            /// Gets a mask representing just the `slot`.
            pub fn from_slot(slot: $Slot) -> Self {
                let mut r = Self::none();
                r.insert(slot);
                r
            }

            /// Returns a mask that represents no update.
            pub const fn none() -> Self {
                $Mask([0; 2])
            }

            /// Returns a mask that represents all updates.
            pub const fn all() -> Self {
                $Mask([u128::MAX; 2])
            }

            /// Returns `true` if this mask does not represent any update.
            pub fn is_none(&self) -> bool {
                self.0[0] == 0 && self.0[1] == 0
            }

            /// Flags the `slot` in this mask.
            pub fn insert(&mut self, slot: $Slot) {
                let slot = slot.0;
                if slot < 128 {
                    self.0[0] |= 1 << slot;
                } else {
                    self.0[1] |= 1 << (slot - 128);
                }
            }

            /// Returns `true` if the `slot` is set in this mask.
            pub fn contains(&self, slot: $Slot) -> bool {
                let slot = slot.0;
                if slot < 128 {
                    (self.0[0] & (1 << slot)) != 0
                } else {
                    (self.0[1] & (1 << (slot - 128))) != 0
                }
            }

            /// Flags all slots set in `other` in `self` as well.
            pub fn extend(&mut self, other: &Self) {
                self.0[0] |= other.0[0];
                self.0[1] |= other.0[1];
            }

            /// Returns `true` if any slot is set in both `self` and `other`.
            pub fn intersects(&self, other: &Self) -> bool {
                (self.0[0] & other.0[0]) != 0 || (self.0[1] & other.0[1]) != 0
            }
        }
        impl ops::BitOrAssign<Self> for $Mask {
            fn bitor_assign(&mut self, rhs: Self) {
                self.extend(&rhs)
            }
        }
        impl ops::BitOrAssign<$Slot> for $Mask {
            fn bitor_assign(&mut self, rhs: $Slot) {
                self.insert(rhs)
            }
        }
        impl ops::BitOr<Self> for $Mask {
            type Output = Self;

            fn bitor(mut self, rhs: Self) -> Self {
                self.extend(&rhs);
                self
            }
        }
        impl ops::BitOr<$Slot> for $Mask {
            type Output = Self;

            fn bitor(mut self, rhs: $Slot) -> Self {
                self.insert(rhs);
                self
            }
        }
        impl ops::BitOr<Self> for $Slot {
            type Output = $Mask;

            fn bitor(self, rhs: Self) -> $Mask {
                let mut m = self.mask();
                m.insert(rhs);
                m
            }
        }
        impl fmt::Debug for $Mask {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                use std::fmt::Write;

                let rows = [
                    self.0[0] as u64,
                    (self.0[0] >> 64) as u64,
                    self.0[1] as u64,
                    (self.0[1] >> 64) as u64
                ];

                writeln!(f, "{} {{", stringify!($Mask))?;

                let mut bmp = String::with_capacity(256 + 4);

                for row in rows {
                    write!(bmp, "    ")?;
                    for i in 0..64 {
                        let b = 1u64 << i;
                        if (b & row) == 0 {
                            write!(bmp, "░")?;
                        } else {
                            write!(bmp, "█")?;
                        }
                    }
                    writeln!(bmp)?;
                }

                write!(f, "{bmp}}}")
            }
        }

    )+}
}

update_slot! {
    /// Represents a single update source in a [`UpdateMask`].
    ///
    /// Anything that generates an [`UiNode::update`] has one of these slots reserved.
    ///
    /// [`UiNode::update`]: crate::UiNode::update
    pub struct UpdateSlot -> UpdateMask;

    /// Represents a single event in a [`EventMask`].
    ///
    /// Every event is assigned on of these slots.
    pub struct EventSlot -> EventMask;
}
update_mask! {
    /// Represents the combined update sources that affect an UI tree or widget.
    pub struct UpdateMask <- UpdateSlot;

    /// Represents the combined events that are listened by an UI tree or widget.
    pub struct EventMask <- EventSlot;
}

/// Represents all event and update subscriptions of a widget.
///
/// Properties must register their interest in events and variables here otherwise a call to [`UiNode::event`] or
/// [`UiNode::update`] can end-up skipped due to optimizations.
///
/// [`UiNode::event`]: crate::UiNode::event
/// [`UiNode::update`]: crate::UiNode::update
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct WidgetSubscriptions {
    event: EventMask,
    update: UpdateMask,
}
impl WidgetSubscriptions {
    /// New default, no subscriptions.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an [`Event`] or command subscription.
    ///
    /// [`Event`]: crate::event::Event
    pub fn event(&mut self, event: impl crate::event::Event) -> &mut Self {
        self.event.insert(event.slot());
        self
    }

    /// Register multiple event or command subscriptions.
    pub fn events(&mut self, mask: &EventMask) -> &mut Self {
        self.event.extend(mask);
        self
    }

    /// Register async handler waker update source.
    pub fn handler<A>(&mut self, handler: &impl WidgetHandler<A>) -> &mut Self
    where
        A: Clone + 'static,
    {
        handler.subscribe(self);
        self
    }

    /// Register a custom update source subscription.
    pub fn update(&mut self, slot: UpdateSlot) -> &mut Self {
        self.update.insert(slot);
        self
    }

    /// Register multiple update source subscriptions.
    pub fn updates(&mut self, mask: &UpdateMask) -> &mut Self {
        self.update.extend(mask);
        self
    }

    /// Register all subscriptions from `other` in `self`.
    pub fn extend(&mut self, other: &WidgetSubscriptions) -> &mut Self {
        self.events(&other.event).updates(&other.update)
    }

    /// Register a variable subscription.
    pub fn var<Vr, T>(&mut self, vars: &Vr, var: &impl Var<T>) -> &mut Self
    where
        Vr: WithVarsRead,
        T: VarValue,
    {
        self.update.extend(&var.update_mask(vars));
        self
    }

    /// Start a [`WidgetVarSubscriptions`] to register multiple variables without needing to reference the [`VarsRead`] for every variable.
    pub fn vars<'s, 'v>(&'s mut self, vars: &'v impl AsRef<VarsRead>) -> WidgetVarSubscriptions<'v, 's> {
        WidgetVarSubscriptions {
            vars: vars.as_ref(),
            subs: self,
        }
    }

    /// Returns `true` if the widget subscribes to events in the slot.
    pub fn event_contains(&self, event: &impl EventUpdateArgs) -> bool {
        self.event.contains(event.slot())
    }

    /// Returns `true` if the widget is interested in variables or other update sources that are flagged in `updates`.
    pub fn update_intersects(&self, updates: &Updates) -> bool {
        self.update.intersects(updates.current())
    }

    /// Returns the current set event subscriptions.
    pub fn event_mask(&self) -> EventMask {
        self.event
    }

    /// Returns the current set update subscriptions.
    pub fn update_mask(&self) -> UpdateMask {
        self.update
    }

    /// Returns if both event and update subscriptions are none.
    pub fn is_none(&self) -> bool {
        self.event.is_none() && self.update.is_none()
    }
}
impl ops::BitOr for WidgetSubscriptions {
    type Output = Self;

    fn bitor(mut self, rhs: Self) -> Self::Output {
        self |= rhs;
        self
    }
}
impl ops::BitOrAssign for WidgetSubscriptions {
    fn bitor_assign(&mut self, rhs: Self) {
        self.event |= rhs.event;
        self.update |= rhs.update;
    }
}

/// Helper for registering multiple [`WidgetSubscriptions::var`] without needing to reference the [`VarsRead`] instance for every variable.
pub struct WidgetVarSubscriptions<'v, 's> {
    vars: &'v VarsRead,
    /// The main [`WidgetSubscriptions`].
    pub subs: &'s mut WidgetSubscriptions,
}
impl<'v, 's> WidgetVarSubscriptions<'v, 's> {
    /// Register a variable subscriptions.
    pub fn var<T: VarValue>(self, var: &impl Var<T>) -> Self {
        Self {
            subs: self.subs.var(self.vars, var),
            vars: self.vars,
        }
    }
}

/// Argument for a interactivity filter function.
///
/// See [WidgetInfoBuilder::push_interactivity_filter].
#[derive(Debug)]
pub struct InteractivityFilterArgs<'a> {
    /// Widget being filtered.
    pub info: WidgetInfo<'a>,
}
impl<'a> InteractivityFilterArgs<'a> {
    /// New from `info`.
    pub fn new(info: WidgetInfo<'a>) -> Self {
        Self { info }
    }
}

type InteractivityFilters = Vec<Rc<dyn Fn(&InteractivityFilterArgs) -> Interactivity>>;

bitflags! {
    /// Represents the level of interaction allowed for a widget.
    pub struct Interactivity: u8 {
        /// Normal interactions allowed.
        ///
        /// This is the default value.
        const ENABLED = 0b00;

        /// Only "disabled" interactions allowed and disabled visuals.
        ///
        /// An example of disabled interaction is a tooltip that explains why a disabled button cannot be clicked.
        const DISABLED = 0b01;

        /// No interaction allowed, the widget must behave like a background visual.
        ///
        /// Note that widgets with blocked interaction are still hit-testable, so they can still be "clicked"
        /// as a visual part of an interactive parent widget.
        const BLOCKED = 0b10;

        /// `BLOCKED` with `DISABLED` visuals.
        const BLOCKED_DISABLED = Self::DISABLED.bits | Self::BLOCKED.bits;
    }
}
impl Interactivity {
    /// Normal interactions allowed.
    pub fn is_enabled(self) -> bool {
        self == Self::ENABLED
    }

    /// Enabled visuals, may still be blocked.
    pub fn is_visually_enabled(self) -> bool {
        !self.contains(Self::DISABLED)
    }

    /// Only "disabled" interactions allowed and disabled visuals.
    pub fn is_disabled(self) -> bool {
        self == Self::DISABLED
    }

    /// Disabled visuals, maybe also blocked.
    pub fn is_visually_disabled(self) -> bool {
        self.contains(Self::DISABLED)
    }

    /// No interaction allowed, may still be visually enabled.
    pub fn is_blocked(self) -> bool {
        self.contains(Self::BLOCKED)
    }
}
impl Default for Interactivity {
    /// `ENABLED`.
    fn default() -> Self {
        Interactivity::ENABLED
    }
}
impl_from_and_into_var! {
    /// * `true` -> `ENABLED`
    /// * `false` -> `DISABLED`
    fn from(enabled: bool) -> Interactivity {
        if enabled {
            Interactivity::ENABLED
        } else {
            Interactivity::DISABLED
        }
    }
}

/// Widget visibility.
///
/// The visibility status of a widget is computed from its outer-bounds in the last layout and if it rendered anything,
/// the visibility of a parent widget affects all descendant widgets, you can inspect the visibility using the
/// [`WidgetInfo::visibility`] method.
///
/// You can use  the [`visibility`] property to explicitly set the visibility of a widget, this property causes the widget to
/// layout and render according to specified visibility.
///
/// [`WidgetInfo::visibility`]: crate::widget_info::WidgetInfo::visibility
/// [`visibility`]: fn@crate::widget_base::visibility
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Visibility {
    /// The widget is visible, this is default.
    Visible,
    /// The widget is not visible, but still affects layout.
    ///
    /// Hidden widgets measure and reserve space in their parent but are not rendered.
    Hidden,
    /// The widget is not visible and does not affect layout.
    ///
    /// Collapsed widgets always measure to zero and are not rendered.
    Collapsed,
}
impl fmt::Debug for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "Visibility::")?;
        }
        match self {
            Visibility::Visible => write!(f, "Visible"),
            Visibility::Hidden => write!(f, "Hidden"),
            Visibility::Collapsed => write!(f, "Collapsed"),
        }
    }
}
impl Default for Visibility {
    /// [` Visibility::Visible`]
    fn default() -> Self {
        Visibility::Visible
    }
}
impl ops::BitOr for Visibility {
    type Output = Self;

    /// `Collapsed` | `Hidden` | `Visible` short circuit from left to right.
    fn bitor(self, rhs: Self) -> Self::Output {
        use Visibility::*;
        match (self, rhs) {
            (Collapsed, _) | (_, Collapsed) => Collapsed,
            (Hidden, _) | (_, Hidden) => Hidden,
            _ => Visible,
        }
    }
}
impl ops::BitOrAssign for Visibility {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}
impl_from_and_into_var! {
    /// * `true` -> `Visible`
    /// * `false` -> `Collapsed`
    fn from(visible: bool) -> Visibility {
        if visible { Visibility::Visible } else { Visibility::Collapsed }
    }
}

/// Represents the descendants of a widget, allows checking if widgets are descendant with O(1) time.
#[derive(Clone, PartialEq, Eq)]
pub struct WidgetDescendantsRange<'a> {
    _tree: PhantomData<&'a WidgetInfoTree>,
    range: std::ops::Range<usize>,
}
impl<'a> WidgetDescendantsRange<'a> {
    /// If the widget is a descendant.
    pub fn contains(&self, wgt: WidgetInfo<'a>) -> bool {
        self.range.contains(&wgt.node_id.get())
    }
}
impl<'a> Default for WidgetDescendantsRange<'a> {
    /// Empty range.
    fn default() -> Self {
        Self {
            _tree: PhantomData,
            range: 0..0,
        }
    }
}

/// A hit-test hit.
#[derive(Clone, Debug)]
pub struct HitInfo {
    /// ID of widget hit.
    pub widget_id: WidgetId,

    /// Z-index of the hit.
    pub z_index: ZIndex,
}

/// A hit-test result.
#[derive(Clone, Debug)]
pub struct HitTestInfo {
    window_id: WindowId,
    frame_id: FrameId,
    point: PxPoint,
    hits: Vec<HitInfo>,
}
impl HitTestInfo {
    /// No hits info
    pub fn no_hits(window_id: WindowId) -> Self {
        HitTestInfo {
            window_id,
            frame_id: FrameId::INVALID,
            point: PxPoint::new(Px(-1), Px(-1)),
            hits: vec![],
        }
    }

    /// The window that was hit-tested.
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    /// The window frame that was hit-tested.
    pub fn frame_id(&self) -> FrameId {
        self.frame_id
    }

    /// The point in the window that was hit-tested.
    pub fn point(&self) -> PxPoint {
        self.point
    }

    /// All hits, from top-most.
    pub fn hits(&self) -> &[HitInfo] {
        &self.hits
    }

    /// The top hit.
    pub fn target(&self) -> Option<&HitInfo> {
        self.hits.first()
    }

    /// Finds the widget in the hit-test result if it was hit.
    pub fn find(&self, widget_id: WidgetId) -> Option<&HitInfo> {
        self.hits.iter().find(|h| h.widget_id == widget_id)
    }

    /// If the widget is in was hit.
    pub fn contains(&self, widget_id: WidgetId) -> bool {
        self.hits.iter().any(|h| h.widget_id == widget_id)
    }

    /// Gets a clone of `self` that only contains the hits that also happen in `other`.
    pub fn intersection(&self, other: &HitTestInfo) -> HitTestInfo {
        let mut hits: Vec<_> = self.hits.iter().filter(|h| other.contains(h.widget_id)).cloned().collect();
        hits.shrink_to_fit();

        HitTestInfo {
            window_id: self.window_id,
            frame_id: self.frame_id,
            point: self.point,
            hits,
        }
    }
}
