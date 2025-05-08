//! Widget info tree.

use std::{borrow::Cow, fmt, mem, ops, sync::Arc, time::Duration};

pub mod access;

mod tree;
use parking_lot::{MappedMutexGuard, Mutex, MutexGuard, RwLock};
use tree::Tree;

mod path;
pub use path::*;

mod builder;
pub use builder::*;

pub mod iter;
pub use iter::TreeFilter;

mod hit;
pub(crate) use hit::{HitTestClips, ParallelSegmentOffsets};
use zng_clone_move::clmv;
use zng_layout::{
    context::{LayoutMask, LayoutMetricsSnapshot},
    unit::{
        DistanceKey, Factor, FactorUnits, Orientation2D, Px, PxBox, PxCornerRadius, PxPoint, PxRect, PxSideOffsets, PxSize, PxTransform,
        PxVector, euclid,
    },
};
use zng_state_map::{OwnedStateMap, StateMapRef};
use zng_txt::{Txt, formatx};
use zng_unique_id::{IdEntry, IdMap};
use zng_var::impl_from_and_into_var;
use zng_view_api::{ViewProcessGen, display_list::FrameValueUpdate, window::FrameId};

use crate::{DInstant, render::TransformStyle, window::WindowId};

pub use self::hit::RelativeHitZ;
use self::{access::AccessEnabled, hit::ParallelSegmentId, iter::TreeIterator};

use super::{WidgetId, node::ZIndex};

/// Stats over the lifetime of a widget info tree.
///
/// The stats for a tree are available in [`WidgetInfoTree::stats`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WidgetInfoTreeStats {
    /// Number of times info was rebuild for the window.
    pub generation: u32,

    /// Duration of the [`UiNode::info`] call for the window content.
    ///
    /// [`UiNode::info`]: crate::widget::node::UiNode::info
    pub build_time: Duration,

    /// Count of widgets that where reused from a previous tree.
    pub reused_widgets: u32,

    /// Last window frame that rendered this tree.
    ///
    /// Before the first render this is `FrameId::INVALID`.
    pub last_frame: FrameId,

    /// Last window frame that moved or resized the inner bounds of at least one widget.
    pub bounds_updated_frame: FrameId,

    /// Count of moved or resized widgets in the last `bounds_updated_frame`.
    pub bounds_updated: u32,

    /// Last window frame that changed visibility of at least one widget.
    pub vis_updated_frame: FrameId,
}
impl WidgetInfoTreeStats {
    fn new(build_start: DInstant, reused_widgets: u32, generation: u32) -> Self {
        Self {
            generation,
            build_time: build_start.elapsed(),
            reused_widgets,
            last_frame: FrameId::INVALID,
            bounds_updated_frame: FrameId::INVALID,
            bounds_updated: 0,
            vis_updated_frame: FrameId::INVALID,
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

        // we don't show `vis_updated` because if can be counted twice when visibility changes from collapsed.
        if update.vis_updated > 0 || self.vis_updated_frame == FrameId::INVALID {
            self.vis_updated_frame = frame;
        }
    }
}
#[derive(Default)]
struct WidgetInfoTreeStatsUpdate {
    bounds_updated: u32,
    vis_updated: u32,
}
impl WidgetInfoTreeStatsUpdate {
    fn take(&mut self) -> Self {
        mem::take(self)
    }
}

/// A tree of [`WidgetInfo`].
///
/// The tree is behind an `Arc` pointer so cloning and storing this type is very cheap.
///
/// Instantiated using [`WidgetInfoBuilder`].
#[derive(Clone)]
pub struct WidgetInfoTree(Arc<WidgetInfoTreeInner>);
struct WidgetInfoTreeInner {
    window_id: WindowId,
    access_enabled: AccessEnabled,
    tree: Tree<WidgetInfoData>,
    lookup: IdMap<WidgetId, tree::NodeId>,
    interactivity_filters: InteractivityFilters,
    build_meta: Arc<OwnedStateMap<WidgetInfoMeta>>,
    frame: RwLock<WidgetInfoTreeFrame>,
}
// info that updates every frame
struct WidgetInfoTreeFrame {
    stats: WidgetInfoTreeStats,
    stats_update: WidgetInfoTreeStatsUpdate,
    out_of_bounds_update: Vec<(tree::NodeId, bool)>,
    scale_factor: Factor,
    view_process_gen: ViewProcessGen,

    out_of_bounds: Arc<Vec<tree::NodeId>>,
    spatial_bounds: PxBox,

    widget_count_offsets: ParallelSegmentOffsets,

    transform_changed_subs: IdMap<WidgetId, PxTransform>,
    visibility_changed_subs: IdMap<WidgetId, Visibility>,
}
impl PartialEq for WidgetInfoTree {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for WidgetInfoTree {}
impl WidgetInfoTree {
    /// Blank window that contains only the root widget taking no space.
    pub fn wgt(window_id: WindowId, root_id: WidgetId) -> Self {
        WidgetInfoBuilder::new(
            Arc::default(),
            window_id,
            AccessEnabled::empty(),
            root_id,
            WidgetBoundsInfo::new(),
            WidgetBorderInfo::new(),
            1.fct(),
        )
        .finalize(None, false)
    }

    /// Statistics abound the info tree.
    pub fn stats(&self) -> WidgetInfoTreeStats {
        self.0.frame.read().stats.clone()
    }

    /// Scale factor of the last rendered frame.
    pub fn scale_factor(&self) -> Factor {
        self.0.frame.read().scale_factor
    }

    /// View-process generation.
    ///
    /// Is [`ViewProcessGen::INVALID`] before first render and in headless apps.
    ///
    /// [`ViewProcessGen::INVALID`]: zng_view_api::ViewProcessGen::INVALID
    pub fn view_process_gen(&self) -> ViewProcessGen {
        self.0.frame.read().view_process_gen
    }

    /// Custom metadata associated with the tree during info build.
    ///
    /// Any widget (that was not reused) can have inserted metadata.
    pub fn build_meta(&self) -> StateMapRef<WidgetInfoMeta> {
        self.0.build_meta.borrow()
    }

    /// Reference to the root widget in the tree.
    pub fn root(&self) -> WidgetInfo {
        WidgetInfo::new(self.clone(), self.0.tree.root().id())
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
        self.0.lookup.get(&widget_id.into()).map(|i| WidgetInfo::new(self.clone(), *i))
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
        self.0.frame.read().stats.last_frame != FrameId::INVALID
    }

    /// Iterator over all widgets with inner-bounds not fully contained by their parent inner bounds.
    pub fn out_of_bounds(&self) -> impl std::iter::ExactSizeIterator<Item = WidgetInfo> + 'static + use<> {
        let out = self.0.frame.read().out_of_bounds.clone();
        let me = self.clone();
        (0..out.len()).map(move |i| WidgetInfo::new(me.clone(), out[i]))
    }

    /// Gets the bounds box that envelops all widgets, including the out-of-bounds widgets.
    pub fn spatial_bounds(&self) -> PxRect {
        self.0.frame.read().spatial_bounds.to_rect()
    }

    /// Total number of widgets in the tree.
    ///
    /// Is never zero, every tree has at least the root widget.
    #[expect(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.0.lookup.len()
    }

    fn bounds_changed(&self) {
        self.0.frame.write().stats_update.bounds_updated += 1;
    }

    fn in_bounds_changed(&self, widget_id: WidgetId, in_bounds: bool) {
        let id = *self.0.lookup.get(&widget_id).unwrap();
        self.0.frame.write().out_of_bounds_update.push((id, in_bounds));
    }

    fn visibility_changed(&self) {
        self.0.frame.write().stats_update.vis_updated += 1;
    }

    pub(crate) fn after_render(
        &self,
        frame_id: FrameId,
        scale_factor: Factor,
        view_process_gen: Option<ViewProcessGen>,
        widget_count_offsets: Option<ParallelSegmentOffsets>,
    ) {
        let mut frame = self.0.frame.write();
        let stats_update = frame.stats_update.take();
        frame.stats.update(frame_id, stats_update);

        if !frame.out_of_bounds_update.is_empty() {
            // update out-of-bounds list, reuses the same vec most of the time,
            // unless a spatial iter was generated and not dropped before render.

            let mut out_of_bounds = Arc::try_unwrap(mem::take(&mut frame.out_of_bounds)).unwrap_or_else(|rc| (*rc).clone());

            for (id, remove) in frame.out_of_bounds_update.drain(..) {
                if remove {
                    if let Some(i) = out_of_bounds.iter().position(|i| *i == id) {
                        out_of_bounds.swap_remove(i);
                    }
                } else {
                    out_of_bounds.push(id);
                }
            }
            frame.out_of_bounds = Arc::new(out_of_bounds);
        }

        let mut spatial_bounds = self.root().outer_bounds().to_box2d();
        for out in frame.out_of_bounds.iter() {
            let b = WidgetInfo::new(self.clone(), *out).inner_bounds().to_box2d();
            spatial_bounds = spatial_bounds.union(&b);
        }
        frame.spatial_bounds = spatial_bounds;

        frame.scale_factor = scale_factor;
        if let Some(vp_gen) = view_process_gen {
            frame.view_process_gen = vp_gen;
        }
        if let Some(w) = widget_count_offsets {
            frame.widget_count_offsets = w;
        }

        let mut changes = IdMap::new();
        TRANSFORM_CHANGED_EVENT.visit_subscribers::<()>(|wid| {
            if let Some(wgt) = self.get(wid) {
                let transform = wgt.inner_transform();
                match frame.transform_changed_subs.entry(wid) {
                    IdEntry::Occupied(mut e) => {
                        let prev = e.insert(transform);
                        if prev != transform {
                            changes.insert(wid, prev);
                        }
                    }
                    IdEntry::Vacant(e) => {
                        e.insert(transform);
                    }
                }
            }
            ops::ControlFlow::Continue(())
        });
        if !changes.is_empty() {
            if (frame.transform_changed_subs.len() - changes.len()) > 500 {
                frame
                    .transform_changed_subs
                    .retain(|k, _| TRANSFORM_CHANGED_EVENT.is_subscriber(*k));
            }

            TRANSFORM_CHANGED_EVENT.notify(TransformChangedArgs::now(self.clone(), changes));
        }
        drop(frame); // wgt.visibility can read frame

        let mut changes = IdMap::new();
        VISIBILITY_CHANGED_EVENT.visit_subscribers::<()>(|wid| {
            if let Some(wgt) = self.get(wid) {
                let visibility = wgt.visibility();
                let mut frame = self.0.frame.write();
                match frame.visibility_changed_subs.entry(wid) {
                    IdEntry::Occupied(mut e) => {
                        let prev = e.insert(visibility);
                        if prev != visibility {
                            changes.insert(wid, prev);
                        }
                    }
                    IdEntry::Vacant(e) => {
                        e.insert(visibility);
                    }
                }
            }
            ops::ControlFlow::Continue(())
        });
        if !changes.is_empty() {
            if (self.0.frame.read().visibility_changed_subs.len() - changes.len()) > 500 {
                self.0
                    .frame
                    .write()
                    .visibility_changed_subs
                    .retain(|k, _| VISIBILITY_CHANGED_EVENT.is_subscriber(*k));
            }

            VISIBILITY_CHANGED_EVENT.notify(VisibilityChangedArgs::now(self.clone(), changes));
        }
    }

    pub(crate) fn after_render_update(&self, frame_id: FrameId) {
        let scale_factor = self.0.frame.read().scale_factor;
        self.after_render(frame_id, scale_factor, None, None);
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

#[derive(Debug, Default)]
struct WidgetBoundsData {
    inner_offset: PxVector,
    child_offset: PxVector,
    parent_child_offset: PxVector,

    inline: Option<WidgetInlineInfo>,
    measure_inline: Option<WidgetInlineMeasure>,

    measure_outer_size: PxSize,
    outer_size: PxSize,
    inner_size: PxSize,
    baseline: Px,
    inner_offset_baseline: bool,

    transform_style: TransformStyle,
    perspective: f32,
    perspective_origin: Option<PxPoint>,

    measure_metrics: Option<LayoutMetricsSnapshot>,
    measure_metrics_used: LayoutMask,
    metrics: Option<LayoutMetricsSnapshot>,
    metrics_used: LayoutMask,

    outer_transform: PxTransform,
    inner_transform: PxTransform,
    rendered: Option<WidgetRenderInfo>,

    outer_bounds: PxRect,
    inner_bounds: PxRect,

    hit_clips: HitTestClips,
    hit_index: hit::HitChildIndex,

    is_in_bounds: Option<bool>,
    is_partially_culled: bool,
    cannot_auto_hide: bool,
    is_collapsed: bool,
}

/// Widget render data.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WidgetRenderInfo {
    // Visible/hidden.
    pub visible: bool,

    pub parent_perspective: Option<(f32, PxPoint)>,

    // raw z-index in widget_count units.
    pub seg_id: ParallelSegmentId,
    pub back: usize,
    pub front: usize,
}

/// Shared reference to layout size, offsets, rendered transforms and bounds of a widget.
///
/// Can be retrieved in the [`WIDGET`] and [`WidgetInfo`].
///
/// [`WIDGET`]: crate::widget::WIDGET
#[derive(Default, Clone, Debug)]
pub struct WidgetBoundsInfo(Arc<Mutex<WidgetBoundsData>>);
impl PartialEq for WidgetBoundsInfo {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for WidgetBoundsInfo {}
impl WidgetBoundsInfo {
    /// New default.
    pub fn new() -> Self {
        Self::default()
    }

    /// New info with bound sizes known.
    pub fn new_size(outer: PxSize, inner: PxSize) -> Self {
        let me = Self::new();
        me.set_outer_size(outer);
        me.set_inner_size(inner);
        me
    }

    /// Gets the widget's last measured outer bounds size.
    ///
    /// This size is expected to be the same if the widget is layout using the same exact parameters it was measured.
    pub fn measure_outer_size(&self) -> PxSize {
        self.0.lock().measure_outer_size
    }

    /// Gets the widget's last layout outer bounds size.
    pub fn outer_size(&self) -> PxSize {
        self.0.lock().outer_size
    }

    /// Gets the widget's inner bounds offset inside the outer bounds.
    ///
    /// If [`inner_offset_baseline`] is `true` the [`baseline`] is added from this value.
    ///
    /// [`inner_offset_baseline`]: Self::baseline
    /// [`baseline`]: Self::baseline
    pub fn inner_offset(&self) -> PxVector {
        let mut r = self.0.lock().inner_offset;
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
        self.0.lock().inner_offset_baseline
    }

    /// Gets the widget's child offset inside the inner bounds.
    ///
    /// If the widget's child is another widget this is zero and the offset is set on that child's [`parent_child_offset`] instead.
    ///
    /// [`parent_child_offset`]: Self::parent_child_offset
    pub fn child_offset(&self) -> PxVector {
        self.0.lock().child_offset
    }

    /// Gets the widget's inner bounds size.
    pub fn inner_size(&self) -> PxSize {
        self.0.lock().inner_size
    }

    /// The baseline offset up from the inner bounds bottom line.
    ///
    /// Note that if [`inner_offset_baseline`] is `true` the [`inner_offset`] is already added by the baseline. Parent
    /// panel widgets implementing baseline offset must use the [`final_baseline`] value to avoid offsetting more then once.
    ///
    /// [`inner_offset_baseline`]: Self::inner_offset_baseline
    /// [`inner_offset`]: Self::inner_offset
    /// [`final_baseline`]: Self::final_baseline
    pub fn baseline(&self) -> Px {
        self.0.lock().baseline
    }

    /// Gets the baseline offset of the widget after [`inner_offset`] is applied.
    ///
    /// Returns `Px(0)` if [`inner_offset_baseline`], otherwise returns [`baseline`].
    ///
    /// [`inner_offset`]: Self::inner_offset
    /// [`inner_offset_baseline`]: Self::inner_offset_baseline
    /// [`baseline`]: Self::baseline
    pub fn final_baseline(&self) -> Px {
        let s = self.0.lock();
        if s.inner_offset_baseline { Px(0) } else { s.baseline }
    }

    /// Gets the global transform of the widget's outer bounds during the last render or render update.
    pub fn outer_transform(&self) -> PxTransform {
        self.0.lock().outer_transform
    }

    /// Offset rendered in the widget inner set by the parent widget.
    ///
    /// Note that this offset is applied to the [`outer_transform`](Self::outer_transform) already.
    pub fn parent_child_offset(&self) -> PxVector {
        self.0.lock().parent_child_offset
    }

    /// Gets the global transform of the widget's inner bounds during the last render or render update.
    pub fn inner_transform(&self) -> PxTransform {
        self.0.lock().inner_transform
    }

    /// Gets the latest inline measure info.
    ///
    /// Note that this info may not be the same that was used to update the [`inline`] layout info.
    /// This value is only useful for panels implementing inline, just after the widget was measured.
    ///
    /// Returns `None` if the latest widget measure was not in an inlining context.
    ///
    /// [`inline`]: Self::inline
    pub fn measure_inline(&self) -> Option<WidgetInlineMeasure> {
        self.0.lock().measure_inline.clone()
    }

    /// Exclusive read the latest inline layout info.
    ///
    /// Returns `None` if the latest widget layout was not in an inlining context.
    pub fn inline(&self) -> Option<MappedMutexGuard<WidgetInlineInfo>> {
        let me = self.0.lock();
        if me.inline.is_some() {
            Some(MutexGuard::map(me, |m| m.inline.as_mut().unwrap()))
        } else {
            None
        }
    }

    /// Gets the widget's latest render info, if it was rendered visible or hidden. Returns `None` if the widget was collapsed.
    pub fn rendered(&self) -> Option<bool> {
        self.0.lock().rendered.map(|i| i.visible)
    }

    pub(crate) fn render_info(&self) -> Option<WidgetRenderInfo> {
        self.0.lock().rendered
    }

    /// Gets if the [`inner_bounds`] are fully inside the parent inner bounds.
    ///
    /// [`inner_bounds`]: Self::inner_bounds
    pub fn is_in_bounds(&self) -> bool {
        self.0.lock().is_in_bounds.unwrap_or(false)
    }

    /// Gets if the widget only renders if [`outer_bounds`] intersects with the [`FrameBuilder::auto_hide_rect`].
    ///
    /// This is `true` by default and can be disabled using [`allow_auto_hide`]. If set to `false`
    /// the widget is always rendered, but descendant widgets can still auto-hide.
    ///
    /// [`outer_bounds`]: Self::outer_bounds
    /// [`FrameBuilder::auto_hide_rect`]: crate::render::FrameBuilder::auto_hide_rect
    /// [`allow_auto_hide`]: WidgetLayout::allow_auto_hide
    pub fn can_auto_hide(&self) -> bool {
        !self.0.lock().cannot_auto_hide
    }

    fn set_can_auto_hide(&self, enabled: bool) {
        self.0.lock().cannot_auto_hide = !enabled;
    }

    pub(crate) fn is_actually_out_of_bounds(&self) -> bool {
        self.0.lock().is_in_bounds.map(|is| !is).unwrap_or(false)
    }

    pub(crate) fn set_rendered(&self, rendered: Option<WidgetRenderInfo>, info: &WidgetInfoTree) {
        let mut m = self.0.lock();
        if m.rendered.map(|i| i.visible) != rendered.map(|i| i.visible) {
            info.visibility_changed();
        }
        m.rendered = rendered;
    }

    pub(crate) fn set_outer_transform(&self, transform: PxTransform, info: &WidgetInfoTree) {
        let bounds = transform
            .outer_transformed(PxBox::from_size(self.outer_size()))
            .unwrap_or_default()
            .to_rect();

        let mut m = self.0.lock();

        if m.outer_bounds.size.is_empty() != bounds.size.is_empty() {
            info.visibility_changed();
        }

        m.outer_bounds = bounds;
        m.outer_transform = transform;
    }

    pub(crate) fn set_parent_child_offset(&self, offset: PxVector) {
        self.0.lock().parent_child_offset = offset;
    }

    pub(crate) fn set_inner_transform(
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

        let mut m = self.0.lock();

        if m.inner_bounds != bounds {
            m.inner_bounds = bounds;
            info.bounds_changed();
        }
        let in_bounds = parent_inner.map(|r| r.contains_rect(&bounds)).unwrap_or(true);
        if let Some(prev) = m.is_in_bounds {
            if prev != in_bounds {
                m.is_in_bounds = Some(in_bounds);
                info.in_bounds_changed(widget_id, in_bounds);
            }
        } else {
            m.is_in_bounds = Some(in_bounds);
            if !in_bounds {
                info.in_bounds_changed(widget_id, in_bounds);
            }
        }

        m.inner_transform = transform;
    }

    /// Outer bounding box, updated after every render.
    pub fn outer_bounds(&self) -> PxRect {
        self.0.lock().outer_bounds
    }

    /// Calculate the bounding box that envelops the actual size and position of the inner bounds last rendered.
    pub fn inner_bounds(&self) -> PxRect {
        self.0.lock().inner_bounds
    }

    /// Gets the inline rows for inline widgets or inner bounds for block widgets.
    ///
    /// The rectangles are in the root space.
    pub fn inner_rects(&self) -> Vec<PxRect> {
        let m = self.0.lock();
        if let Some(i) = &m.inline {
            let offset = m.inner_bounds.origin.to_vector();
            let mut rows = i.rows.clone();
            for r in &mut rows {
                r.origin += offset;
            }
            rows
        } else {
            vec![m.inner_bounds]
        }
    }

    /// Call `visitor` for each [`inner_rects`] without allocating or locking.
    ///
    // The visitor parameters are the rect, inline row index and rows length. If the widget
    /// is not inline both index and len are zero.
    ///
    /// [`inner_rects`]: Self::inner_rects
    pub fn visit_inner_rects<B>(&self, mut visitor: impl FnMut(PxRect, usize, usize) -> ops::ControlFlow<B>) -> Option<B> {
        let m = self.0.lock();
        let inner_bounds = m.inner_bounds;
        let inline_range = m.inline.as_ref().map(|i| 0..i.rows.len());
        drop(m);

        if let Some(inline_range) = inline_range {
            let offset = inner_bounds.origin.to_vector();
            let len = inline_range.len();

            for i in inline_range {
                let mut r = match self.0.lock().inline.as_ref().and_then(|inl| inl.rows.get(i).copied()) {
                    Some(r) => r,
                    None => break, // changed mid visit
                };
                r.origin += offset;
                match visitor(r, i, len) {
                    ops::ControlFlow::Continue(()) => continue,
                    ops::ControlFlow::Break(r) => return Some(r),
                }
            }
            None
        } else {
            match visitor(inner_bounds, 0, 0) {
                ops::ControlFlow::Continue(()) => None,
                ops::ControlFlow::Break(r) => Some(r),
            }
        }
    }

    /// If the widget and descendants was collapsed during layout.
    pub fn is_collapsed(&self) -> bool {
        self.0.lock().is_collapsed
    }

    /// Gets if the widget preserves 3D perspective.
    pub fn transform_style(&self) -> TransformStyle {
        self.0.lock().transform_style
    }

    /// Gets the widget perspective and perspective origin (in the inner bounds).
    pub fn perspective(&self) -> Option<(f32, PxPoint)> {
        let p = self.0.lock();
        if p.perspective.is_finite() {
            let s = p.inner_size;
            let o = p.perspective_origin.unwrap_or_else(|| PxPoint::new(s.width / 2.0, s.height / 2.0));
            Some((p.perspective, o))
        } else {
            None
        }
    }

    /// Snapshot of the [`LayoutMetrics`] on the last layout.
    ///
    /// The [`metrics_used`] value indicates what fields where actually used in the last layout.
    ///
    /// Is `None` if the widget is collapsed.
    ///
    /// [`LayoutMetrics`]: zng_layout::context::LayoutMetrics
    /// [`metrics_used`]: Self::metrics_used
    pub fn metrics(&self) -> Option<LayoutMetricsSnapshot> {
        self.0.lock().metrics.clone()
    }

    /// All [`metrics`] fields used by the widget or descendants on the last layout.
    ///
    /// [`metrics`]: Self::metrics
    pub fn metrics_used(&self) -> LayoutMask {
        self.0.lock().metrics_used
    }

    /// Gets the relative hit-test Z for `window_point` against the hit-test shapes rendered for the widget.
    pub fn hit_test_z(&self, window_point: PxPoint) -> RelativeHitZ {
        let m = self.0.lock();
        if m.hit_clips.is_hit_testable() {
            m.hit_clips.hit_test_z(&m.inner_transform, window_point)
        } else {
            RelativeHitZ::NoHit
        }
    }

    /// Index of this widget in the parent hit-test items.
    fn hit_test_index(&self) -> hit::HitChildIndex {
        self.0.lock().hit_index
    }

    /// Returns `true` if a hit-test clip that affects the `child` removes the `window_point` hit on the child.
    pub fn hit_test_clip_child(&self, child: &WidgetInfo, window_point: PxPoint) -> bool {
        let m = self.0.lock();
        if m.hit_clips.is_hit_testable() {
            m.hit_clips
                .clip_child(child.bounds_info().hit_test_index(), &m.inner_transform, window_point)
        } else {
            false
        }
    }

    pub(crate) fn update_hit_test_transform(&self, value: FrameValueUpdate<PxTransform>) {
        self.0.lock().hit_clips.update_transform(value);
    }

    pub(crate) fn measure_metrics(&self) -> Option<LayoutMetricsSnapshot> {
        self.0.lock().measure_metrics.clone()
    }
    pub(crate) fn measure_metrics_used(&self) -> LayoutMask {
        self.0.lock().measure_metrics_used
    }

    fn set_outer_size(&self, size: PxSize) {
        let mut s = self.0.lock();
        if !size.is_empty() {
            s.is_collapsed = false;
        }
        s.outer_size = size;
    }

    fn set_is_collapsed(&self, collapsed: bool) {
        self.0.lock().is_collapsed = collapsed;
    }

    fn take_inline(&self) -> Option<WidgetInlineInfo> {
        self.0.lock().inline.take()
    }

    fn set_inline(&self, inline: Option<WidgetInlineInfo>) {
        self.0.lock().inline = inline;
    }

    pub(super) fn set_measure_inline(&self, inline: Option<WidgetInlineMeasure>) {
        self.0.lock().measure_inline = inline;
    }

    pub(crate) fn set_measure_outer_size(&self, size: PxSize) {
        self.0.lock().measure_outer_size = size;
    }

    fn set_inner_offset(&self, offset: PxVector) {
        self.0.lock().inner_offset = offset;
    }

    fn set_child_offset(&self, offset: PxVector) {
        self.0.lock().child_offset = offset;
    }

    fn set_inner_size(&self, size: PxSize) {
        self.0.lock().inner_size = size;
    }

    fn set_baseline(&self, baseline: Px) {
        self.0.lock().baseline = baseline;
    }

    fn set_inner_offset_baseline(&self, enabled: bool) {
        self.0.lock().inner_offset_baseline = enabled;
    }

    fn set_transform_style(&self, style: TransformStyle) {
        self.0.lock().transform_style = style;
    }

    fn raw_perspective(&self) -> f32 {
        self.0.lock().perspective
    }

    fn raw_perspective_origin(&self) -> Option<PxPoint> {
        self.0.lock().perspective_origin
    }

    fn set_perspective(&self, d: f32) {
        self.0.lock().perspective = d;
    }

    fn set_perspective_origin(&self, o: Option<PxPoint>) {
        self.0.lock().perspective_origin = o;
    }

    fn set_metrics(&self, metrics: Option<LayoutMetricsSnapshot>, used: LayoutMask) {
        self.0.lock().metrics = metrics;
        self.0.lock().metrics_used = used;
    }

    pub(crate) fn set_measure_metrics(&self, metrics: Option<LayoutMetricsSnapshot>, used: LayoutMask) {
        self.0.lock().measure_metrics = metrics;
        self.0.lock().measure_metrics_used = used;
    }

    pub(crate) fn set_hit_clips(&self, clips: HitTestClips) {
        self.0.lock().hit_clips = clips;
    }

    pub(crate) fn set_hit_index(&self, index: hit::HitChildIndex) {
        self.0.lock().hit_index = index;
    }

    pub(crate) fn is_partially_culled(&self) -> bool {
        self.0.lock().is_partially_culled
    }

    pub(crate) fn set_is_partially_culled(&self, is: bool) {
        self.0.lock().is_partially_culled = is;
    }
}

#[derive(Default, Debug)]
struct WidgetBorderData {
    offsets: PxSideOffsets,
    corner_radius: PxCornerRadius,
}

/// Shared reference to the combined *border* and corner radius of a [`WidgetInfo`].
#[derive(Default, Clone, Debug)]
pub struct WidgetBorderInfo(Arc<Mutex<WidgetBorderData>>);
impl WidgetBorderInfo {
    /// New default.
    pub fn new() -> Self {
        Self::default()
    }

    /// Constructor for tests.
    #[cfg(test)]
    pub fn new_test(offsets: PxSideOffsets, corner_radius: PxCornerRadius) -> Self {
        let r = Self::default();
        r.set_offsets(offsets);
        r.set_corner_radius(corner_radius);
        r
    }

    /// Sum of the widths of all borders set on the widget.
    pub fn offsets(&self) -> PxSideOffsets {
        self.0.lock().offsets
    }

    /// Corner radius set on the widget, this is the *outer* curve of border corners.
    pub fn corner_radius(&self) -> PxCornerRadius {
        self.0.lock().corner_radius
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
        self.0.lock().offsets = widths;
    }

    pub(super) fn set_corner_radius(&self, radius: PxCornerRadius) {
        self.0.lock().corner_radius = radius;
    }
}

struct WidgetInfoData {
    id: WidgetId,
    bounds_info: WidgetBoundsInfo,
    border_info: WidgetBorderInfo,
    meta: Arc<OwnedStateMap<WidgetInfoMeta>>,
    interactivity_filters: InteractivityFilters,
    local_interactivity: Interactivity,
    is_reused: bool,
    cache: Mutex<WidgetInfoCache>,
}
impl Clone for WidgetInfoData {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            bounds_info: self.bounds_info.clone(),
            border_info: self.border_info.clone(),
            meta: self.meta.clone(),
            interactivity_filters: self.interactivity_filters.clone(),
            local_interactivity: self.local_interactivity,
            is_reused: self.is_reused,
            cache: Mutex::new(match self.cache.try_lock() {
                Some(c) => c.clone(),
                None => WidgetInfoCache { interactivity: None },
            }),
        }
    }
}
impl fmt::Debug for WidgetInfoData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WidgetInfoData").field("id", &self.id).finish_non_exhaustive()
    }
}
#[derive(Clone)]
struct WidgetInfoCache {
    interactivity: Option<Interactivity>,
}

/// Reference to a widget info in a [`WidgetInfoTree`].
#[derive(Clone)]
pub struct WidgetInfo {
    tree: WidgetInfoTree,
    node_id: tree::NodeId,
}
impl PartialEq for WidgetInfo {
    fn eq(&self, other: &Self) -> bool {
        self.node_id == other.node_id && self.tree == other.tree
    }
}
impl Eq for WidgetInfo {}
impl std::hash::Hash for WidgetInfo {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.node_id, state)
    }
}
impl std::fmt::Debug for WidgetInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WidgetInfo")
            .field("[path]", &self.path().to_string())
            .field("[meta]", &self.meta())
            .finish_non_exhaustive()
    }
}

impl WidgetInfo {
    fn new(tree: WidgetInfoTree, node_id: tree::NodeId) -> Self {
        Self { tree, node_id }
    }

    fn node(&self) -> tree::NodeRef<WidgetInfoData> {
        self.tree.0.tree.index(self.node_id)
    }

    fn info(&self) -> &WidgetInfoData {
        self.node().value()
    }

    /// Widget id.
    pub fn id(&self) -> WidgetId {
        self.info().id
    }

    /// Full path to this widget.
    pub fn path(&self) -> WidgetPath {
        let mut path: Vec<_> = self.ancestors().map(|a| a.id()).collect();
        path.reverse();
        path.push(self.id());
        path.shrink_to_fit();

        WidgetPath::new(self.tree.0.window_id, path.into())
    }

    /// Path details to help finding the widget during debug.
    ///
    /// If the inspector metadata is present the widget type is included.
    pub fn trace_path(&self) -> Txt {
        let mut ws: Vec<_> = self.self_and_ancestors().collect();
        ws.reverse();

        use std::fmt::*;

        let mut s = String::new();

        let _ = write!(&mut s, "{:?}/", self.tree.window_id());
        for w in ws {
            #[cfg(feature = "inspector")]
            {
                use crate::widget::inspector::*;
                if let Some(info) = w.inspector_info() {
                    let mod_path = info.builder.widget_type().path;
                    let mod_ident = if let Some((_, ident)) = mod_path.rsplit_once(':') {
                        ident
                    } else {
                        mod_path
                    };

                    let id = w.id();
                    let name = id.name();
                    if !name.is_empty() {
                        let _ = write!(&mut s, "/{mod_ident}!({name:?})");
                    } else {
                        let _ = write!(&mut s, "/{mod_ident}!({})", id.sequential());
                    }
                } else {
                    let _ = write!(&mut s, "/{}", w.id());
                }
            }

            #[cfg(not(feature = "inspector"))]
            {
                let _ = write!(&mut s, "/{}", w.id());
            }
        }

        s.into()
    }

    /// Detailed id text.
    ///
    /// If the inspector metadata is present the widget type is included.
    pub fn trace_id(&self) -> Txt {
        #[cfg(feature = "inspector")]
        {
            use crate::widget::inspector::*;
            if let Some(info) = self.inspector_info() {
                let mod_path = info.builder.widget_type().path;
                let mod_ident = if let Some((_, ident)) = mod_path.rsplit_once(':') {
                    ident
                } else {
                    mod_path
                };

                let id = self.id();
                let name = id.name();
                if !name.is_empty() {
                    return formatx!("{mod_ident}!({name:?})");
                } else {
                    return formatx!("{mod_ident}!({})", id.sequential());
                }
            }
        }
        formatx!("{}", self.id())
    }

    /// Full path to this widget with [`interactivity`] values.
    ///
    /// [`interactivity`]: Self::interactivity
    pub fn interaction_path(&self) -> InteractionPath {
        let mut path = vec![];

        let mut blocked = None;
        let mut disabled = None;

        for w in self.self_and_ancestors() {
            let interactivity = w.interactivity();
            if interactivity.contains(Interactivity::BLOCKED) {
                blocked = Some(path.len());
            }
            if interactivity.contains(Interactivity::DISABLED) {
                disabled = Some(path.len());
            }

            path.push(w.id());
        }
        path.reverse();
        path.shrink_to_fit();

        let len = path.len();

        let path = WidgetPath::new(self.tree.0.window_id, path.into());
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
    pub fn new_path(&self, old_path: &WidgetPath) -> Option<WidgetPath> {
        assert_eq!(old_path.widget_id(), self.id());
        if self
            .ancestors()
            .zip(old_path.ancestors().iter().rev())
            .any(|(ancestor, id)| ancestor.id() != *id)
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
    /// # Panics
    ///
    /// If `old_path` does not point to the same widget id as `self`.
    ///
    /// [`interaction_path`]: Self::interaction_path
    pub fn new_interaction_path(&self, old_path: &InteractionPath) -> Option<InteractionPath> {
        assert_eq!(old_path.widget_id(), self.id());

        if self.interactivity() != old_path.interactivity()
            || self
                .ancestors()
                .zip(old_path.zip().rev().skip(1))
                .any(|(anc, (id, int))| anc.id() != id || anc.interactivity() != int)
        {
            Some(self.interaction_path())
        } else {
            None
        }
    }

    /// Get the z-index of the widget in the latest frame if it was rendered.
    ///
    /// Note that widgets can render in the back and front of each descendant, these indexes are the *back-most* index, the moment
    /// the widget starts rendering and the *front-most* index at the moment the widget and all contents finishes rendering.
    ///
    /// This value is updated every [`render`] without causing a tree rebuild.
    ///
    /// [`render`]: crate::widget::node::UiNode::render
    pub fn z_index(&self) -> Option<(ZIndex, ZIndex)> {
        self.info().bounds_info.render_info().map(|i| {
            let offset = self.tree.0.frame.read().widget_count_offsets.offset(i.seg_id);
            (ZIndex((i.back + offset) as u32), ZIndex((i.front + offset) as u32))
        })
    }

    /// Gets the visibility of the widget or the widget's descendants in the last rendered frame.
    ///
    /// A widget is [`Visible`] if it rendered at least one display item, [`Hidden`] if it rendered only space and
    /// hit-test items, [`Collapsed`] if it did not render. All widgets are [`Visible`] if no frame was ever rendered.
    ///
    /// [`Visible`]: Visibility::Visible
    /// [`Hidden`]: Visibility::Hidden
    /// [`Collapsed`]: Visibility::Collapsed
    pub fn visibility(&self) -> Visibility {
        match self.info().bounds_info.rendered() {
            Some(vis) => {
                if vis {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                }
            }
            None => {
                if self.tree.is_rendered() {
                    Visibility::Collapsed
                } else {
                    Visibility::Visible
                }
            }
        }
    }

    /// Get or compute the interactivity of the widget.
    ///
    /// The interactivity of a widget is the combined result of all interactivity filters applied to it and its ancestors.
    /// If a parent is blocked this is blocked, same for disabled.
    pub fn interactivity(&self) -> Interactivity {
        let cached = self.info().cache.lock().interactivity;
        if let Some(cache) = cached {
            cache
        } else {
            let mut cache = self.info().cache.lock();
            let mut interactivity = self.info().local_interactivity;

            if interactivity != Interactivity::BLOCKED_DISABLED {
                interactivity |= self.parent().map(|n| n.interactivity()).unwrap_or(Interactivity::ENABLED);
                if interactivity != Interactivity::BLOCKED_DISABLED {
                    let args = InteractivityFilterArgs { info: self.clone() };
                    for filter in &self.tree.0.interactivity_filters {
                        interactivity |= filter(&args);
                        if interactivity == Interactivity::BLOCKED_DISABLED {
                            break;
                        }
                    }
                }
            }

            cache.interactivity = Some(interactivity);
            interactivity
        }
    }

    /// All the transforms introduced by this widget, starting from the outer info.
    ///
    /// This information is up-to-date, it is updated every layout and render without causing a tree rebuild.
    pub fn bounds_info(&self) -> WidgetBoundsInfo {
        self.info().bounds_info.clone()
    }

    /// Clone a reference to the widget border and corner radius information.
    ///
    /// This information is up-to-date, it is updated every layout without causing a tree rebuild.
    pub fn border_info(&self) -> WidgetBorderInfo {
        self.info().border_info.clone()
    }

    /// Gets the 3D perspective for this widget.
    ///
    /// The `f32` is a distance from the Z-plane to the viewer, the point is the vanishing center in the parent widget inner bounds.
    pub fn perspective(&self) -> Option<(f32, PxPoint)> {
        self.parent()?.bounds_info().perspective()
    }

    /// Gets the transform style for this widget.
    ///
    /// Is `Flat` unless it or the parent widget sets `Preserve3D`.
    pub fn transform_style(&self) -> TransformStyle {
        if let TransformStyle::Flat = self.bounds_info().transform_style() {
            if let Some(p) = self.parent() {
                p.bounds_info().transform_style()
            } else {
                TransformStyle::Flat
            }
        } else {
            TransformStyle::Preserve3D
        }
    }

    /// Size of the widget outer area, not transformed.
    ///
    /// Returns an up-to-date size, the size is updated every layout without causing a tree rebuild.
    pub fn outer_size(&self) -> PxSize {
        self.info().bounds_info.outer_size()
    }

    /// Size of the widget inner area, not transformed.
    ///
    /// Returns an up-to-date size, the size is updated every layout without causing a tree rebuild.
    pub fn inner_size(&self) -> PxSize {
        self.info().bounds_info.inner_size()
    }

    /// Size of the widget child area, not transformed.
    ///
    /// Returns an up-to-date size, the size is updated every layout without causing a tree rebuild.
    pub fn inner_border_size(&self) -> PxSize {
        let info = self.info();
        info.border_info.inner_size(&info.bounds_info)
    }

    /// Gets the baseline offset up from the inner bounds bottom line.
    pub fn baseline(&self) -> Px {
        self.info().bounds_info.baseline()
    }

    /// Widget outer transform in window space.
    ///
    /// Returns an up-to-date transform, the transform is updated every render or render update without causing a tree rebuild.
    pub fn outer_transform(&self) -> PxTransform {
        self.info().bounds_info.outer_transform()
    }

    /// Widget inner transform in the window space.
    ///
    /// Returns an up-to-date transform, the transform is updated every render or render update without causing a tree rebuild.
    pub fn inner_transform(&self) -> PxTransform {
        self.info().bounds_info.inner_transform()
    }

    /// Widget outer rectangle in the window space.
    ///
    /// Returns an up-to-date rect, the bounds are updated every render or render update without causing a tree rebuild.
    pub fn outer_bounds(&self) -> PxRect {
        let info = self.info();
        info.bounds_info.outer_bounds()
    }

    /// Widget inner rectangle in the window space.
    ///
    /// Returns an up-to-date rect, the bounds are updated every render or render update without causing a tree rebuild.
    pub fn inner_bounds(&self) -> PxRect {
        let info = self.info();
        info.bounds_info.inner_bounds()
    }

    /// Compute the bounding box that envelops self and descendants inner bounds.
    pub fn spatial_bounds(&self) -> PxBox {
        self.out_of_bounds()
            .fold(self.inner_bounds().to_box2d(), |acc, w| acc.union(&w.inner_bounds().to_box2d()))
    }

    /// Widget inner bounds center in the window space.
    pub fn center(&self) -> PxPoint {
        self.inner_bounds().center()
    }

    /// Custom metadata associated with the widget during info build.
    pub fn meta(&self) -> StateMapRef<WidgetInfoMeta> {
        self.info().meta.borrow()
    }

    /// Reference the [`WidgetInfoTree`] that owns `self`.
    pub fn tree(&self) -> &WidgetInfoTree {
        &self.tree
    }

    /// If the widget info and all descendants did not change in the last rebuild.
    pub fn is_reused(&self) -> bool {
        self.info().is_reused
    }

    /// Reference to the root widget.
    pub fn root(&self) -> Self {
        self.tree.root()
    }

    /// Reference to the widget that contains this widget.
    ///
    /// Is `None` only for [`root`](WidgetInfoTree::root).
    pub fn parent(&self) -> Option<Self> {
        self.node().parent().map(move |n| WidgetInfo::new(self.tree.clone(), n.id()))
    }

    /// Reference to the previous widget within the same parent.
    pub fn prev_sibling(&self) -> Option<Self> {
        self.node().prev_sibling().map(move |n| WidgetInfo::new(self.tree.clone(), n.id()))
    }

    /// Reference to the next widget within the same parent.
    pub fn next_sibling(&self) -> Option<Self> {
        self.node().next_sibling().map(move |n| WidgetInfo::new(self.tree.clone(), n.id()))
    }

    /// Reference to the first widget within this widget.
    pub fn first_child(&self) -> Option<Self> {
        self.node().first_child().map(move |n| WidgetInfo::new(self.tree.clone(), n.id()))
    }

    /// Reference to the last widget within this widget.
    pub fn last_child(&self) -> Option<Self> {
        self.node().last_child().map(move |n| WidgetInfo::new(self.tree.clone(), n.id()))
    }

    /// If the parent widget has multiple children.
    pub fn has_siblings(&self) -> bool {
        self.node().has_siblings()
    }

    /// If the widget has at least one child.
    pub fn has_children(&self) -> bool {
        self.node().has_children()
    }

    /// All parent children except this widget.
    pub fn siblings(&self) -> impl Iterator<Item = WidgetInfo> + 'static + use<> {
        self.prev_siblings().chain(self.next_siblings())
    }

    /// Iterator over the direct descendants of the widget.
    pub fn children(&self) -> iter::Children {
        let mut r = self.self_and_children();
        r.next();
        r.next_back();
        r
    }

    /// Count of [`children`].
    ///
    /// [`children`]: Self::children
    pub fn children_count(&self) -> usize {
        self.node().children_count()
    }

    /// Iterator over the widget and the direct descendants of the widget.
    pub fn self_and_children(&self) -> iter::Children {
        iter::Children::new(self.clone())
    }

    /// Iterator over all widgets contained by this widget.
    pub fn descendants(&self) -> iter::TreeIter {
        let mut d = self.self_and_descendants();
        d.next();
        d
    }

    /// Total number of [`descendants`].
    ///
    /// [`descendants`]: Self::descendants
    pub fn descendants_len(&self) -> usize {
        self.node().descendants_range().len()
    }

    /// Iterator over the widget and all widgets contained by it.
    pub fn self_and_descendants(&self) -> iter::TreeIter {
        iter::TreeIter::self_and_descendants(self.clone())
    }

    /// Iterator over parent -> grandparent -> .. -> root.
    pub fn ancestors(&self) -> iter::Ancestors {
        let mut r = self.self_and_ancestors();
        r.next();
        r
    }

    /// Gets a value that can check if widgets are descendant of `self` in O(1) time.
    pub fn descendants_range(&self) -> WidgetDescendantsRange {
        WidgetDescendantsRange {
            tree: Some(self.tree.clone()),
            range: self.node().descendants_range(),
        }
    }

    /// Compare the position of `self` and `sibling` in the `ancestor` [`descendants`].
    ///
    /// [`descendants`]: Self::descendants
    pub fn cmp_sibling_in(&self, sibling: &WidgetInfo, ancestor: &WidgetInfo) -> Option<std::cmp::Ordering> {
        let range = ancestor.node().descendants_range();
        let index = self.node_id.get();
        let sibling_index = sibling.node_id.get();
        if range.contains(&index) && self.tree == sibling.tree && self.tree == ancestor.tree {
            return Some(index.cmp(&sibling_index));
        }
        None
    }

    /// If `self` is an ancestor of `maybe_descendant`.
    pub fn is_ancestor(&self, maybe_descendant: &WidgetInfo) -> bool {
        self.descendants_range().contains(maybe_descendant)
    }

    /// If `self` is inside `maybe_ancestor`.
    pub fn is_descendant(&self, maybe_ancestor: &WidgetInfo) -> bool {
        maybe_ancestor.descendants_range().contains(self)
    }

    /// Iterator over self -> parent -> grandparent -> .. -> root.
    pub fn self_and_ancestors(&self) -> iter::Ancestors {
        iter::Ancestors::new(self.clone())
    }

    /// Iterator over all previous widgets within the same parent.
    pub fn prev_siblings(&self) -> iter::PrevSiblings {
        let mut r = self.self_and_prev_siblings();
        r.next();
        r
    }

    /// Iterator over self and all previous widgets within the same parent.
    pub fn self_and_prev_siblings(&self) -> iter::PrevSiblings {
        iter::PrevSiblings::new(self.clone())
    }

    /// Iterator over all next widgets within the same parent.
    pub fn next_siblings(&self) -> iter::NextSiblings {
        let mut r = self.self_and_next_siblings();
        r.next();
        r
    }

    /// Iterator over self and all next widgets within the same parent.
    pub fn self_and_next_siblings(&self) -> iter::NextSiblings {
        iter::NextSiblings::new(self.clone())
    }

    /// Iterator over all previous widgets within the same `ancestor`, including descendants of siblings.
    ///
    /// If `ancestor` is not actually an ancestor iterates to the root.
    pub fn prev_siblings_in(&self, ancestor: &WidgetInfo) -> iter::RevTreeIter {
        iter::TreeIter::prev_siblings_in(self.clone(), ancestor.clone())
    }

    /// Iterator over self, descendants and all previous widgets within the same `ancestor`.
    ///
    /// If `ancestor` is not actually an ancestor iterates to the root.
    pub fn self_and_prev_siblings_in(&self, ancestor: &WidgetInfo) -> iter::RevTreeIter {
        iter::TreeIter::self_and_prev_siblings_in(self.clone(), ancestor.clone())
    }

    /// Iterator over all next widgets within the same `ancestor`, including descendants of siblings.
    ///
    /// If `ancestor` is not actually an ancestor iterates to the root.
    pub fn next_siblings_in(&self, ancestor: &WidgetInfo) -> iter::TreeIter {
        iter::TreeIter::next_siblings_in(self.clone(), ancestor.clone())
    }

    /// Iterator over self, descendants and all next widgets within the same `ancestor`.
    ///
    /// If `ancestor` is not actually an ancestor iterates to the root.
    pub fn self_and_next_siblings_in(&self, ancestor: &WidgetInfo) -> iter::TreeIter {
        iter::TreeIter::self_and_next_siblings_in(self.clone(), ancestor.clone())
    }

    /// The [`center`] orientation in relation to an `origin`.
    ///
    /// Returns `None` if the `origin` is the center.
    ///
    /// [`center`]: Self::center
    pub fn orientation_from(&self, origin: PxPoint) -> Option<Orientation2D> {
        let o = self.center();
        [
            Orientation2D::Above,
            Orientation2D::Right,
            Orientation2D::Below,
            Orientation2D::Left,
        ]
        .iter()
        .find(|&&d| d.point_is(origin, o))
        .copied()
    }

    /// Value that indicates the distance between this widget center and `origin`.
    pub fn distance_key(&self, origin: PxPoint) -> DistanceKey {
        DistanceKey::from_points(origin, self.center())
    }

    /// Value that indicates the distance between the nearest point inside this widgets rectangles and `origin`.
    ///
    /// The widgets rectangles is the inner bounds for block widgets or the row rectangles for inline widgets.
    pub fn rect_distance_key(&self, origin: PxPoint) -> DistanceKey {
        self.rect_distance_key_filtered(origin, |_, _, _| true)
    }

    /// Like [`rect_distance_key`], but only consider rectangles approved by `filter`.
    ///
    /// The filter parameters are the rectangle, the inline row index and the total inline rows length. If the widget
    /// is not inlined both the index and len are zero.
    ///
    /// [`rect_distance_key`]: Self::rect_distance_key
    pub fn rect_distance_key_filtered(&self, origin: PxPoint, mut filter: impl FnMut(PxRect, usize, usize) -> bool) -> DistanceKey {
        let mut d = DistanceKey::NONE_MAX;
        self.info().bounds_info.visit_inner_rects::<()>(|r, i, len| {
            if !filter(r, i, len) {
                return ops::ControlFlow::Continue(());
            }
            let dd = DistanceKey::from_rect_to_point(r, origin);
            d = d.min(dd);
            if d == DistanceKey::MIN {
                ops::ControlFlow::Break(())
            } else {
                ops::ControlFlow::Continue(())
            }
        });
        d
    }

    /// Count of ancestors.
    pub fn depth(&self) -> usize {
        self.ancestors().count()
    }

    /// First ancestor of `self` and `other`.
    ///
    /// Returns `None` if `other` is not from the same tree.
    pub fn shared_ancestor(&self, other: &Self) -> Option<WidgetInfo> {
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
    fn hit_test_z(&self, point: PxPoint) -> Option<ZIndex> {
        let bounds = &self.info().bounds_info;
        if bounds.inner_bounds().contains(point) {
            let z = match bounds.hit_test_z(point) {
                RelativeHitZ::NoHit => None,
                RelativeHitZ::Back => bounds.render_info().map(|i| (i.seg_id, i.back)),
                RelativeHitZ::Over(w) => self
                    .tree
                    .get(w)
                    .and_then(|w| w.info().bounds_info.render_info())
                    .map(|i| (i.seg_id, i.front)),
                RelativeHitZ::Front => bounds.render_info().map(|i| (i.seg_id, i.front)),
            };

            match z {
                Some((seg_id, z)) => {
                    let mut parent = self.parent();
                    let mut child = self.clone();

                    while let Some(p) = parent {
                        if p.info().bounds_info.hit_test_clip_child(&child, point) {
                            return None;
                        }

                        parent = p.parent();
                        child = p;
                    }

                    Some(ZIndex((z + self.tree.0.frame.read().widget_count_offsets.offset(seg_id)) as u32))
                }
                None => None,
            }
        } else {
            None
        }
    }

    /// Returns `true` if this widget's inner bounds are fully contained by the parent inner bounds.
    pub fn is_in_bounds(&self) -> bool {
        self.info().bounds_info.is_in_bounds()
    }

    /// Iterator over all descendants with inner bounds not fully contained by their parent inner bounds.
    pub fn out_of_bounds(&self) -> impl Iterator<Item = WidgetInfo> + 'static + use<> {
        let range = self.descendants_range();
        self.tree.out_of_bounds().filter(move |w| range.contains(w))
    }

    /// Iterator over self and descendants, first self, then all in-bounds descendants, then all out-of-bounds descendants.
    ///
    /// If the `filter` returns `false` the widget and all it's in-bounds descendants are skipped, otherwise they are yielded. After
    /// all in-bounds descendants reachable from `self` and filtered the iterator changes to each out-of-bounds descendants and their
    /// in-bounds descendants that are also filtered.
    pub fn spatial_iter<F>(&self, filter: F) -> impl Iterator<Item = WidgetInfo> + use<F>
    where
        F: Fn(&WidgetInfo) -> bool + Clone,
    {
        let self_id = self.id();
        self.self_and_descendants()
            .tree_filter(clmv!(filter, |w| {
                if (w.is_in_bounds() || w.id() == self_id) && filter(w) {
                    TreeFilter::Include
                } else {
                    TreeFilter::SkipAll
                }
            }))
            .chain(self.out_of_bounds().flat_map(clmv!(filter, |w| {
                let out_of_bound_root_id = w.id();
                w.self_and_descendants().tree_filter(clmv!(filter, |w| {
                    if (w.is_in_bounds() || w.id() == out_of_bound_root_id) && filter(w) {
                        TreeFilter::Include
                    } else {
                        TreeFilter::SkipAll
                    }
                }))
            })))
    }

    /// Iterator over self and all descendants with inner bounds that contain the `point`.
    pub fn inner_contains(&self, point: PxPoint) -> impl Iterator<Item = WidgetInfo> + 'static + use<> {
        self.spatial_iter(move |w| w.inner_bounds().contains(point))
    }

    /// Spatial iterator over self and descendants with inner bounds that intersects the `rect`.
    pub fn inner_intersects(&self, rect: PxRect) -> impl Iterator<Item = WidgetInfo> + 'static + use<> {
        let rect = rect.to_box2d();
        self.spatial_iter(move |w| w.inner_bounds().to_box2d().intersects(&rect))
    }

    /// Spatial iterator over self and descendants with inner bounds that fully envelops the `rect`.
    pub fn inner_contains_rect(&self, rect: PxRect) -> impl Iterator<Item = WidgetInfo> + 'static + use<> {
        let rect = rect.to_box2d();
        self.spatial_iter(move |w| w.inner_bounds().to_box2d().contains_box(&rect))
    }

    /// Spatial iterator over self and descendants with inner bounds that are fully inside the `rect`.
    pub fn inner_contained(&self, rect: PxRect) -> impl Iterator<Item = WidgetInfo> + 'static + use<> {
        let rect = rect.to_box2d();
        self.spatial_iter(move |w| rect.contains_box(&w.inner_bounds().to_box2d()))
    }

    /// Spatial iterator over self and descendants with center point inside the `area`.
    pub fn center_contained(&self, area: PxRect) -> impl Iterator<Item = WidgetInfo> + 'static + use<> {
        let area = area.to_box2d();
        self.spatial_iter(move |w| w.inner_bounds().to_box2d().intersects(&area))
            .filter(move |w| area.contains(w.center()))
    }

    /// Spatial iterator over self and descendants with center point within the `max_radius` of the `origin`.
    pub fn center_in_distance(&self, origin: PxPoint, max_radius: Px) -> impl Iterator<Item = WidgetInfo> + 'static + use<> {
        let area = PxRect::new(origin, PxSize::splat(max_radius))
            .inflate(max_radius, max_radius)
            .to_box2d();

        let distance_key = DistanceKey::from_distance(max_radius);

        self.spatial_iter(move |w| w.inner_bounds().to_box2d().intersects(&area))
            .filter(move |w| w.distance_key(origin) <= distance_key)
    }

    /// Gets all widgets of self and descendants hit by a `point`, sorted by z-index of the hit, front to back.
    pub fn hit_test(&self, point: PxPoint) -> HitTestInfo {
        let _span = tracing::trace_span!("hit_test").entered();

        let mut hits: Vec<_> = self
            .inner_contains(point)
            .filter_map(|w| {
                w.hit_test_z(point).map(|z| HitInfo {
                    widget_id: w.id(),
                    z_index: z,
                })
            })
            .collect();

        hits.sort_by(|a, b| b.z_index.cmp(&a.z_index));

        HitTestInfo {
            window_id: self.tree.0.window_id,
            frame_id: self.tree.0.frame.read().stats.last_frame,
            point,
            hits,
        }
    }

    /// Find the descendant with center point nearest of `origin` within the `max_radius`.
    ///
    /// This method is faster than sorting the result of [`center_in_distance`], but is slower if any point in distance is acceptable.
    ///
    /// [`center_in_distance`]: Self::center_in_distance
    pub fn nearest(&self, origin: PxPoint, max_radius: Px) -> Option<WidgetInfo> {
        self.nearest_filtered(origin, max_radius, |_| true)
    }

    /// Find the widget, self or descendant, with center point nearest of `origin` within the `max_radius` and approved by the `filter` closure.
    pub fn nearest_filtered(&self, origin: PxPoint, max_radius: Px, filter: impl FnMut(&WidgetInfo) -> bool) -> Option<WidgetInfo> {
        self.nearest_bounded_filtered(origin, max_radius, self.tree.spatial_bounds(), filter)
    }

    /// Find the widget, self or descendant, with center point nearest of `origin` within the `max_radius` and inside `bounds`;
    /// and approved by the `filter` closure.
    pub fn nearest_bounded_filtered(
        &self,
        origin: PxPoint,
        max_radius: Px,
        bounds: PxRect,
        mut filter: impl FnMut(&WidgetInfo) -> bool,
    ) -> Option<WidgetInfo> {
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
                if w_dist < dist && filter(&w) {
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
                if w_dist < dist && filter(&w) {
                    dist = w_dist;
                    nearest = Some(w);
                }
            }
        }

        nearest
    }

    /// Find the descendant that has inner bounds or inline rows nearest to `origin` and are within `max_radius`.
    ///
    /// The distance is from any given point inside the bounds or inline rows rectangle to the origin. If origin is inside
    /// the rectangle the distance is zero. If multiple widgets have the same distance the nearest center point widget is used.
    pub fn nearest_rect(&self, origin: PxPoint, max_radius: Px) -> Option<WidgetInfo> {
        self.nearest_rect_filtered(origin, max_radius, |_, _, _, _| true)
    }

    /// Find the descendant that has inner bounds or inline rows nearest to `origin` and are within `max_radius` and are
    /// approved by the `filter` closure.
    ///
    /// The filter parameters are the widget, the rect, the rect row index and the widget inline rows length. If the widget is not inlined
    /// both index and len are zero.
    pub fn nearest_rect_filtered(
        &self,
        origin: PxPoint,
        max_radius: Px,
        filter: impl FnMut(&WidgetInfo, PxRect, usize, usize) -> bool,
    ) -> Option<WidgetInfo> {
        self.nearest_rect_bounded_filtered(origin, max_radius, self.tree.spatial_bounds(), filter)
    }

    /// Find the widget, self or descendant, with inner bounds or inline rows nearest of `origin` within the `max_radius` and inside `bounds`;
    /// and approved by the `filter` closure.
    ///
    /// The filter parameters are the widget, the rect, the rect row index and the widget inline rows length. If the widget is not inlined
    /// both index and len are zero.
    pub fn nearest_rect_bounded_filtered(
        &self,
        origin: PxPoint,
        max_radius: Px,
        bounds: PxRect,
        mut filter: impl FnMut(&WidgetInfo, PxRect, usize, usize) -> bool,
    ) -> Option<WidgetInfo> {
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
            for w in self.inner_intersects(search_quad) {
                let w_dist = w.rect_distance_key_filtered(origin, |rect, i, len| filter(&w, rect, i, len));
                if w_dist < dist {
                    dist = w_dist;
                    nearest = Some(w);
                } else if w_dist == DistanceKey::MIN {
                    let w_center_dist = w.distance_key(origin);
                    let center_dist = nearest.as_ref().unwrap().distance_key(origin);
                    if w_center_dist < center_dist {
                        nearest = Some(w);
                    }
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

        nearest
    }

    /// Spatial iterator over all widgets, self and descendants, with [`center`] in the direction defined by `orientation` and
    /// within `max_distance` of the `origin`, widgets are only visited once and the distance is clipped by the [`spatial_bounds`].
    ///
    /// Use `Px::MAX` on the distance to visit all widgets in the direction.
    ///
    /// The direction is defined by a 45º frustum cast from the `origin`, see [`Orientation2D::point_is`] for more details.
    ///
    /// [`spatial_bounds`]: WidgetInfoTree::spatial_bounds
    /// [`center`]: WidgetInfo::center
    /// [`Orientation2D::point_is`]: zng_layout::unit::Orientation2D::point_is
    pub fn oriented(
        &self,
        origin: PxPoint,
        max_distance: Px,
        orientation: Orientation2D,
    ) -> impl Iterator<Item = WidgetInfo> + 'static + use<> {
        let distance_bounded = max_distance != Px::MAX;
        let distance_key = if distance_bounded {
            DistanceKey::from_distance(max_distance)
        } else {
            DistanceKey::NONE_MAX
        };
        let me = self.clone();
        orientation
            .search_bounds(origin, max_distance, self.tree.spatial_bounds().to_box2d())
            .flat_map(move |sq| me.inner_intersects(sq.to_rect()).map(move |w| (sq, w)))
            .filter_map(move |(sq, w)| {
                let center = w.center();
                if sq.contains(center)
                    && orientation.point_is(origin, center)
                    && (!distance_bounded || DistanceKey::from_points(origin, center) <= distance_key)
                {
                    Some(w)
                } else {
                    None
                }
            })
    }

    /// Spatial iterator over all widgets, self and descendants, with [`inner_bounds`] in the direction defined by `orientation`
    /// in relation to `origin` and with [`center`] within `max_distance` of the `origin` center. Widgets are only visited once and
    /// the distance is clipped by the [`spatial_bounds`].
    ///
    /// Use `Px::MAX` on the distance to visit all widgets in the direction.
    ///
    /// The direction is a collision check between inner-bounds and origin, see [`Orientation2D::box_is`] for more details.
    ///
    /// [`spatial_bounds`]: WidgetInfoTree::spatial_bounds
    /// [`inner_bounds`]: WidgetInfo::inner_bounds
    /// [`center`]: WidgetInfo::center
    /// [`Orientation2D::box_is`]: zng_layout::unit::Orientation2D::box_is
    pub fn oriented_box(
        &self,
        origin: PxBox,
        max_distance: Px,
        orientation: Orientation2D,
    ) -> impl Iterator<Item = WidgetInfo> + 'static + use<> {
        let distance_bounded = max_distance != Px::MAX;
        let distance_key = if distance_bounded {
            DistanceKey::from_distance(max_distance)
        } else {
            DistanceKey::NONE_MAX
        };
        let me = self.clone();
        let origin_center = origin.center();
        orientation
            .search_bounds(origin_center, max_distance, self.tree.spatial_bounds().to_box2d())
            .flat_map(move |sq| me.inner_intersects(sq.to_rect()).map(move |w| (sq, w)))
            .filter_map(move |(sq, w)| {
                let bounds = w.inner_bounds().to_box2d();
                if sq.intersects(&bounds)
                    && orientation.box_is(origin, bounds)
                    && (!distance_bounded || DistanceKey::from_points(origin_center, bounds.center()) <= distance_key)
                {
                    Some(w)
                } else {
                    None
                }
            })
    }

    /// Find the widget with center point nearest of `origin` within the `max_distance` and with `orientation` to origin.
    ///
    /// This method is faster than searching the result of [`oriented`].
    ///
    /// [`oriented`]: Self::oriented
    pub fn nearest_oriented(&self, origin: PxPoint, max_distance: Px, orientation: Orientation2D) -> Option<WidgetInfo> {
        self.nearest_oriented_filtered(origin, max_distance, orientation, |_| true)
    }

    /// Find the widget with center point nearest of `origin` within the `max_distance` and with `orientation` to origin,
    /// and approved by the `filter` closure.
    ///
    /// This method is faster than searching the result of [`oriented`].
    ///
    /// [`oriented`]: Self::oriented
    pub fn nearest_oriented_filtered(
        &self,
        origin: PxPoint,
        max_distance: Px,
        orientation: Orientation2D,
        filter: impl FnMut(&WidgetInfo) -> bool,
    ) -> Option<WidgetInfo> {
        self.nearest_oriented_filtered_impl(origin, max_distance, orientation, filter, |w| {
            orientation.point_is(origin, w.center())
        })
    }

    /// Find the widget with center point nearest to `origin` center within the `max_distance` and with box `orientation` to origin.
    ///
    /// This method is faster than searching the result of [`oriented_box`].
    ///
    /// [`oriented_box`]: Self::oriented_box
    pub fn nearest_box_oriented(&self, origin: PxBox, max_distance: Px, orientation: Orientation2D) -> Option<WidgetInfo> {
        self.nearest_box_oriented_filtered(origin, max_distance, orientation, |_| true)
    }

    /// Find the widget with center point nearest to `origin` center within the `max_distance` and with box `orientation` to origin,
    /// and approved by the `filter` closure.
    ///
    /// This method is faster than searching the result of [`oriented_box`].
    ///
    /// [`oriented_box`]: Self::oriented_box
    pub fn nearest_box_oriented_filtered(
        &self,
        origin: PxBox,
        max_distance: Px,
        orientation: Orientation2D,
        filter: impl FnMut(&WidgetInfo) -> bool,
    ) -> Option<WidgetInfo> {
        self.nearest_oriented_filtered_impl(origin.center(), max_distance, orientation, filter, |w| {
            orientation.box_is(origin, w.inner_bounds().to_box2d())
        })
    }

    fn nearest_oriented_filtered_impl(
        &self,
        origin: PxPoint,
        max_distance: Px,
        orientation: Orientation2D,
        mut filter: impl FnMut(&WidgetInfo) -> bool,
        intersect: impl Fn(&WidgetInfo) -> bool,
    ) -> Option<WidgetInfo> {
        let mut dist = DistanceKey::from_distance(max_distance + Px(1));
        let mut nearest = None;
        let mut last_quad = euclid::Box2D::zero();

        for search_quad in orientation.search_bounds(origin, max_distance, self.tree.spatial_bounds().to_box2d()) {
            for w in self.center_contained(search_quad.to_rect()) {
                if intersect(&w) {
                    let w_dist = w.distance_key(origin);
                    if w_dist < dist && filter(&w) {
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
                if w_dist < dist && filter(&w) {
                    dist = w_dist;
                    nearest = Some(w);
                }
            }
        }

        nearest
    }
}

/// Argument for a interactivity filter function.
///
/// See [`WidgetInfoBuilder::push_interactivity_filter`] for more details.
#[derive(Debug)]
pub struct InteractivityFilterArgs {
    /// Widget being filtered.
    pub info: WidgetInfo,
}
impl InteractivityFilterArgs {
    /// New from `info`.
    pub fn new(info: WidgetInfo) -> Self {
        Self { info }
    }
}

type InteractivityFilters = Vec<Arc<dyn Fn(&InteractivityFilterArgs) -> Interactivity + Send + Sync>>;

bitflags::bitflags! {
    /// Represents the level of interaction allowed for a widget.
    #[derive(Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
    #[serde(transparent)]
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
        /// as a visual part of an interactive parent.
        const BLOCKED = 0b10;

        /// `BLOCKED` with `DISABLED` visuals.
        const BLOCKED_DISABLED = Self::DISABLED.bits() | Self::BLOCKED.bits();
    }
}
impl Interactivity {
    /// Normal interactions allowed.
    pub fn is_enabled(self) -> bool {
        self == Self::ENABLED
    }

    /// Enabled visuals, may still be blocked.
    pub fn is_vis_enabled(self) -> bool {
        !self.contains(Self::DISABLED)
    }

    /// Only "disabled" interactions allowed and disabled visuals.
    pub fn is_disabled(self) -> bool {
        self == Self::DISABLED
    }

    /// Disabled visuals, maybe also blocked.
    pub fn is_vis_disabled(self) -> bool {
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
impl fmt::Debug for Interactivity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_enabled() {
            return write!(f, "ENABLED");
        }
        if *self == Self::BLOCKED_DISABLED {
            return write!(f, "BLOCKED_DISABLED");
        }
        if *self == Self::DISABLED {
            return write!(f, "DISABLED");
        }
        if *self == Self::BLOCKED {
            return write!(f, "BLOCKED");
        }
        write!(f, "Interactivity({:x})", self.bits())
    }
}

/// Widget visibility.
///
/// The visibility state of a widget is computed from its bounds in the last layout and if it rendered anything,
/// the visibility of a parent widget affects all descendant widgets, you can inspect the visibility using the
/// [`WidgetInfo::visibility`] method.
///
/// You can also explicitly hide or collapse a widget using the `visibility` property.
///
/// [`WidgetInfo::visibility`]: crate::widget::info::WidgetInfo::visibility
#[derive(Copy, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Visibility {
    /// The widget is visible.
    ///
    /// This is also the default state, before the first layout and render.
    Visible,
    /// The widget is not visible, but still affects layout.
    ///
    /// Hidden widgets reserve space in their parent but do not render.
    Hidden,
    /// The widget is not visible and does not affect layout.
    ///
    /// Collapsed widgets always measure to zero and do not render.
    Collapsed,
}
impl Visibility {
    /// Is visible.
    pub fn is_visible(self) -> bool {
        matches!(self, Self::Visible)
    }

    /// Is hidden.
    pub fn is_hidden(self) -> bool {
        matches!(self, Self::Hidden)
    }

    /// Is collapsed.
    pub fn is_collapsed(self) -> bool {
        matches!(self, Self::Collapsed)
    }
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
        if visible {
            Visibility::Visible
        } else {
            Visibility::Collapsed
        }
    }
}

/// Represents the descendants of a widget, allows checking if widgets are descendant with O(1) time.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct WidgetDescendantsRange {
    tree: Option<WidgetInfoTree>,
    range: std::ops::Range<usize>,
}
impl WidgetDescendantsRange {
    /// If the widget is a descendant.
    pub fn contains(&self, wgt: &WidgetInfo) -> bool {
        self.range.contains(&wgt.node_id.get()) && self.tree.as_ref() == Some(&wgt.tree)
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

    /// Search the widget in the hit-test result.
    pub fn find(&self, widget_id: WidgetId) -> Option<&HitInfo> {
        self.hits.iter().find(|h| h.widget_id == widget_id)
    }

    /// If the widget was hit.
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
