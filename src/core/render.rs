use crate::core::context::LazyStateMap;
use crate::core::types::*;
use crate::core::UiNode;
use ego_tree::Tree;
use std::mem;
use webrender::api::*;

pub struct FrameBuilder {
    display_list: DisplayListBuilder,

    info: FrameInfoBuilder,
    info_id: WidgetInfoId,

    widget_id: WidgetId,
    meta: LazyStateMap,
    cursor: CursorIcon,
    hit_testable: bool,

    clip_id: ClipId,
    spatial_id: SpatialId,

    offset: LayoutPoint,
}

impl FrameBuilder {
    #[inline]
    pub fn new(frame_id: FrameId, window_id: WindowId, pipeline_id: PipelineId, root_id: WidgetId, root_size: LayoutSize) -> Self {
        let info = FrameInfoBuilder::new(window_id, frame_id, root_id, root_size);
        FrameBuilder {
            display_list: DisplayListBuilder::new(pipeline_id, root_size),
            info_id: info.root_id(),
            info,
            widget_id: root_id,
            meta: LazyStateMap::default(),
            cursor: CursorIcon::default(),
            hit_testable: true,
            clip_id: ClipId::root(pipeline_id),
            spatial_id: SpatialId::root_reference_frame(pipeline_id),
            offset: LayoutPoint::zero(),
        }
    }

    /// Direct access to the display list builder.
    #[inline]
    pub fn display_list(&mut self) -> &mut DisplayListBuilder {
        &mut self.display_list
    }

    /// Current widget.
    #[inline]
    pub fn widget_id(&self) -> WidgetId {
        self.widget_id
    }

    /// Current widget metadata.
    #[inline]
    pub fn meta(&mut self) -> &mut LazyStateMap {
        &mut self.meta
    }

    /// Current cursor.
    #[inline]
    pub fn cursor(&self) -> CursorIcon {
        self.cursor
    }

    /// Current clipping node.
    #[inline]
    pub fn clip_id(&self) -> ClipId {
        self.clip_id
    }

    /// Current spatial node.
    #[inline]
    pub fn spatial_id(&self) -> SpatialId {
        self.spatial_id
    }

    /// Current widget [`ItemTag`]. The first number is the raw [`widget_id`](FrameBuilder::widget_id),
    /// the second number is the raw [`cursor`](FrameBuilder::cursor).
    ///
    /// For more details on how the ItemTag is used see [`FrameHitInfo::new`](FrameHitInfo::new).
    #[inline]
    pub fn item_tag(&self) -> ItemTag {
        (self.widget_id.get(), self.cursor as u16)
    }

    /// If the context is hit-testable.
    #[inline]
    pub fn hit_testable(&self) -> bool {
        self.hit_testable
    }

    /// Common item properties given a `clip_rect` and the current context.
    ///
    /// This is a common case helper,
    #[inline]
    pub fn common_item_properties(&self, clip_rect: LayoutRect) -> CommonItemProperties {
        CommonItemProperties {
            clip_rect,
            hit_info: if self.hit_testable { Some(self.item_tag()) } else { None },
            clip_id: self.clip_id,
            spatial_id: self.spatial_id,
            flags: PrimitiveFlags::empty(),
        }
    }

    /// Calls [`render`](UiNode::render) for `node` inside a new widget context.
    pub fn push_widget(&mut self, id: WidgetId, area: LayoutSize, child: &impl UiNode) {
        // The hit-test bounding-box used to take the coordinates of the widget hit
        // if the widget id is hit in another ItemTag that is not WIDGET_HIT_AREA.
        //
        // This is done so we have consistent hit coordinates with precise hit area.
        self.display_list.push_hit_test(&CommonItemProperties {
            hit_info: Some((id.get(), WIDGET_HIT_AREA)),
            clip_rect: LayoutRect::from_size(area),
            clip_id: self.clip_id,
            spatial_id: self.spatial_id,
            flags: PrimitiveFlags::empty(),
        });

        let parent_id = mem::replace(&mut self.widget_id, id);

        let parent_meta = mem::take(&mut self.meta);

        let mut bounds = LayoutRect::from_size(area);
        bounds.origin = self.offset;

        let node = self.info.push(self.info_id, id, bounds);
        let parent_node = mem::replace(&mut self.info_id, node);

        child.render(self);

        self.info.set_meta(node, mem::replace(&mut self.meta, parent_meta));

        self.widget_id = parent_id;
        self.info_id = parent_node;
    }

    /// Push a hit-test `rect` using [`common_item_properties`](FrameBuilder::common_item_properties)
    /// if [`hit_testable`](FrameBuilder::hit_testable) is `true`.
    #[inline]
    pub fn push_hit_test(&mut self, rect: LayoutRect) {
        if self.hit_testable {
            self.display_list.push_hit_test(&self.common_item_properties(rect));
        }
    }

    /// Calls `f` while [`hit_testable`](FrameBuilder::hit_testable) is set to `hit_testable`.
    #[inline]
    pub fn push_hit_testable(&mut self, hit_testable: bool, f: impl FnOnce(&mut FrameBuilder)) {
        let parent_hit_testable = mem::replace(&mut self.hit_testable, hit_testable);
        f(self);
        self.hit_testable = parent_hit_testable;
    }

    /// Calls `f` with a new [`clip_id`](FrameBuilder::clip_id) that clips to `bounds`.
    #[inline]
    pub fn push_simple_clip(&mut self, bounds: LayoutSize, f: impl FnOnce(&mut FrameBuilder)) {
        let parent_clip_id = self.clip_id;

        self.clip_id = self.display_list.define_clip(
            &SpaceAndClipInfo {
                spatial_id: self.spatial_id,
                clip_id: self.clip_id,
            },
            LayoutRect::from_size(bounds),
            None,
            None,
        );

        f(self);

        self.clip_id = parent_clip_id;
    }

    /// Calls `f` inside a new reference frame at `origin`.
    #[inline]
    pub fn push_reference_frame(&mut self, origin: LayoutPoint, f: impl FnOnce(&mut FrameBuilder)) {
        let parent_spatial_id = self.spatial_id;
        self.spatial_id = self.display_list.push_reference_frame(
            origin,
            parent_spatial_id,
            TransformStyle::Flat,
            PropertyBinding::default(),
            ReferenceFrameKind::Transform,
        );

        let offset = origin.to_vector();
        self.offset += offset;

        f(self);

        self.display_list.pop_reference_frame();
        self.spatial_id = parent_spatial_id;
        self.offset -= offset;
    }

    /// Push a border using [`common_item_properties`](FrameBuilder::common_item_properties).
    #[inline]
    pub fn push_border(&mut self, rect: LayoutRect, widths: LayoutSideOffsets, details: BorderDetails) {
        self.display_list
            .push_border(&self.common_item_properties(rect.clone()), rect, widths, details);
    }

    /// Push a text run using [`common_item_properties`](FrameBuilder::common_item_properties).
    #[inline]
    pub fn push_text(
        &mut self,
        rect: LayoutRect,
        glyphs: &[GlyphInstance],
        font_instance_key: FontInstanceKey,
        color: ColorF,
        glyph_options: Option<GlyphOptions>,
    ) {
        self.display_list.push_text(
            &self.common_item_properties(rect.clone()),
            rect.clone(),
            glyphs,
            font_instance_key,
            color,
            glyph_options,
        );
    }

    /// Calls `f` while [`item_tag`](FrameBuilder::item_tag) indicates the `cursor`.
    ///
    /// Note that for the cursor to be used `node` or its children must push a hit-testable item.
    #[inline]
    pub fn push_cursor(&mut self, cursor: CursorIcon, f: impl FnOnce(&mut FrameBuilder)) {
        let parent_cursor = std::mem::replace(&mut self.cursor, cursor);
        f(self);
        self.cursor = parent_cursor;
    }

    /// Push a color rectangle using [`common_item_properties`](FrameBuilder::common_item_properties).
    #[inline]
    pub fn push_color(&mut self, rect: LayoutRect, color: ColorF) {
        self.display_list.push_rect(&self.common_item_properties(rect), color);
    }

    /// Push a linear gradient rectangle using [`common_item_properties`](FrameBuilder::common_item_properties).
    #[inline]
    pub fn push_linear_gradient(&mut self, rect: LayoutRect, start: LayoutPoint, end: LayoutPoint, stops: &[GradientStop]) {
        self.display_list.push_stops(stops);

        let gradient = Gradient {
            start_point: start,
            end_point: end,
            extend_mode: ExtendMode::Clamp,
        };
        let tile_size = rect.size;
        let tile_spacing = LayoutSize::zero();

        self.display_list
            .push_gradient(&self.common_item_properties(rect.clone()), rect, gradient, tile_size, tile_spacing);
    }

    /// Finalizes the build.
    ///
    /// # Returns
    ///
    /// `(PipelineId, LayoutSize, BuiltDisplayList)` : The display list finalize data.
    /// `FrameInfo`: The built frame info.
    pub fn finalize(self) -> ((PipelineId, LayoutSize, BuiltDisplayList), FrameInfo) {
        (self.display_list.finalize(), self.info.build())
    }
}

/// Complement of [ItemTag] that indicates the hit area of a widget.
pub const WIDGET_HIT_AREA: u16 = u16::max_value();

fn unpack_cursor(raw: u16) -> CursorIcon {
    debug_assert!(raw <= CursorIcon::RowResize as u16);

    if raw <= CursorIcon::RowResize as u16 {
        unsafe { std::mem::transmute(raw as u8) }
    } else {
        CursorIcon::Default
    }
}

/// A hit-test hit.
#[derive(Clone, Debug)]
pub struct HitInfo {
    pub widget_id: WidgetId,
    pub point: LayoutPoint,
    pub cursor: CursorIcon,
}

/// A hit-test result.
#[derive(Clone, Debug)]
pub struct FrameHitInfo {
    window_id: WindowId,
    frame_id: FrameId,
    point: LayoutPoint,
    hits: Vec<HitInfo>,
}

impl FrameHitInfo {
    /// Initializes from a Webrender hit-test result.
    ///
    /// Only item tags produced by [FrameBuilder] are expected.
    ///
    /// The tag format is:
    ///
    /// * `u64`: Raw [WidgetId].
    /// * `u16`: Raw [CursorIcon] or `WIDGET_HIT_AREA`.
    ///
    /// Only widgets that are where hit by a cursor tag and `WIDGET_HIT_AREA` tag are included in
    /// the final result.
    ///
    /// The tag marked with `WIDGET_HIT_AREA` is used to determine the [HitInfo::point].
    #[inline]
    pub fn new(window_id: WindowId, frame_id: FrameId, point: LayoutPoint, hits: HitTestResult) -> Self {
        let mut candidates = Vec::default();
        let mut actual_hits = fnv::FnvHashMap::default();

        for hit in hits.items {
            if hit.tag.1 == WIDGET_HIT_AREA {
                candidates.push((hit.tag.0, hit.point_relative_to_item));
            } else {
                actual_hits.insert(hit.tag.0, hit.tag.1);
            }
        }

        let mut hits = Vec::default();

        for candidate in candidates {
            let raw_id = candidate.0;
            if let Some(raw_cursor) = actual_hits.remove(&raw_id) {
                hits.push(HitInfo {
                    // SAFETY: This is safe because we packed
                    widget_id: unsafe { WidgetId::from_raw(raw_id) },
                    point: candidate.1,
                    cursor: unpack_cursor(raw_cursor),
                })
            }
        }

        hits.shrink_to_fit();

        FrameHitInfo {
            window_id,
            frame_id,
            point,
            hits,
        }
    }

    /// The window that was hit-tested.
    #[inline]
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    /// The window frame that was hit-tested.
    #[inline]
    pub fn frame_id(&self) -> FrameId {
        self.frame_id
    }

    /// The point in the window that was hit-tested.
    #[inline]
    pub fn point(&self) -> LayoutPoint {
        self.point
    }

    /// Top-most cursor or `CursorIcon::Default` if there was no hit.
    #[inline]
    pub fn cursor(&self) -> CursorIcon {
        self.hits.first().map(|h| h.cursor).unwrap_or(CursorIcon::Default)
    }

    /// All hits, from top-most.
    #[inline]
    pub fn hits(&self) -> &[HitInfo] {
        &self.hits
    }

    /// The top hit.
    #[inline]
    pub fn target(&self) -> Option<&HitInfo> {
        self.hits.first()
    }

    /// Finds the widget in the hit-test result if it was hit.
    #[inline]
    pub fn find(&self, widget_id: WidgetId) -> Option<&HitInfo> {
        self.hits.iter().find(|h| h.widget_id == widget_id)
    }

    /// If the widget is in was hit.
    #[inline]
    pub fn contains(&self, widget_id: WidgetId) -> bool {
        self.hits.iter().any(|h| h.widget_id == widget_id)
    }

    /// Gets a clone of `self` that only contains the hits that also happen in `other`.
    #[inline]
    pub fn intersection(&self, other: &FrameHitInfo) -> FrameHitInfo {
        let mut hits: Vec<_> = self.hits.iter().filter(|h| other.contains(h.widget_id)).cloned().collect();
        hits.shrink_to_fit();

        FrameHitInfo {
            window_id: self.window_id,
            frame_id: self.frame_id,
            point: self.point,
            hits,
        }
    }
}

/// [FrameInfo] builder.
pub struct FrameInfoBuilder {
    window_id: WindowId,
    frame_id: FrameId,
    tree: Tree<WidgetInfoInner>,
}

impl FrameInfoBuilder {
    /// Starts building a frame info with the frame root information.
    #[inline]
    pub fn new(window_id: WindowId, frame_id: FrameId, root_id: WidgetId, size: LayoutSize) -> Self {
        let tree = Tree::new(WidgetInfoInner {
            widget_id: root_id,
            bounds: LayoutRect::from_size(size),
            meta: LazyStateMap::default(),
        });

        FrameInfoBuilder { window_id, frame_id, tree }
    }

    /// Gets the root widget info id.
    #[inline]
    pub fn root_id(&self) -> WidgetInfoId {
        WidgetInfoId(self.tree.root().id())
    }

    #[inline]
    fn node(&mut self, id: WidgetInfoId) -> ego_tree::NodeMut<WidgetInfoInner> {
        self.tree
            .get_mut(id.0)
            .ok_or_else(|| format!("`{:?}` not found in this builder", id))
            .unwrap()
    }

    /// Takes the widget metadata already set for `id`.
    #[inline]
    pub fn take_meta(&mut self, id: WidgetInfoId) -> LazyStateMap {
        mem::take(&mut self.node(id).value().meta)
    }

    /// Sets the widget metadata for `id`.
    #[inline]
    pub fn set_meta(&mut self, id: WidgetInfoId, meta: LazyStateMap) {
        self.node(id).value().meta = meta;
    }

    /// Appends a widget child.
    #[inline]
    pub fn push(&mut self, parent: WidgetInfoId, widget_id: WidgetId, bounds: LayoutRect) -> WidgetInfoId {
        WidgetInfoId(
            self.node(parent)
                .append(WidgetInfoInner {
                    widget_id,
                    bounds,
                    meta: LazyStateMap::default(),
                })
                .id(),
        )
    }

    /// Builds the final frame info.
    #[inline]
    pub fn build(self) -> FrameInfo {
        FrameInfo {
            window_id: self.window_id,
            frame_id: self.frame_id,
            lookup: self.tree.nodes().map(|n| (n.value().widget_id, n.id())).collect(),
            tree: self.tree,
        }
    }
}

/// Id of a building widget info.
#[derive(Debug, Clone, Copy)]
pub struct WidgetInfoId(ego_tree::NodeId);

/// Information about a rendered frame.
///
/// Instantiated using [FrameInfoBuilder].
pub struct FrameInfo {
    window_id: WindowId,
    frame_id: FrameId,
    tree: Tree<WidgetInfoInner>,
    lookup: fnv::FnvHashMap<WidgetId, ego_tree::NodeId>,
}

impl FrameInfo {
    /// Blank window frame that contains only the root widget taking no space.
    #[inline]
    pub fn blank(window_id: WindowId, root_id: WidgetId) -> Self {
        FrameInfoBuilder::new(window_id, Epoch(0), root_id, LayoutSize::zero()).build()
    }

    /// Reference to the root widget in the frame.
    #[inline]
    pub fn root(&self) -> WidgetInfo {
        WidgetInfo::new(self, self.tree.root().id())
    }

    #[inline]
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    #[inline]
    pub fn frame_id(&self) -> FrameId {
        self.frame_id
    }

    /// Reference to the widget in the frame, if it is present.
    #[inline]
    pub fn find(&self, widget_id: WidgetId) -> Option<WidgetInfo> {
        self.lookup
            .get(&widget_id)
            .and_then(|i| self.tree.get(*i).map(|n| WidgetInfo::new(self, n.id())))
    }

    /// If the frame contains the widget.
    #[inline]
    pub fn contains(&self, widget_id: WidgetId) -> bool {
        self.lookup.contains_key(&widget_id)
    }

    /// Resolve the widget path in the frame, if it is the same frame.
    #[inline]
    pub fn get(&self, path: &WidgetPath) -> Option<WidgetInfo> {
        if path.window_id() == self.window_id() && path.frame_id() == self.frame_id() {
            if let Some(id) = path.node_id {
                self.tree.get(id).map(|n| WidgetInfo::new(self, n.id()))
            } else {
                self.find(path.widget_id())
            }
        } else {
            None
        }
    }
}

/// Full address of a widget in a specific [FrameInfo].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetPath {
    node_id: Option<ego_tree::NodeId>,
    window_id: WindowId,
    frame_id: FrameId,
    path: Box<[WidgetId]>,
}

impl WidgetPath {
    /// Window the [frame_id](WidgetPath::frame_id) belongs too.
    #[inline]
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    /// The frame of [window_id](WidgetPath::window_id) this path was computed.
    #[inline]
    pub fn frame_id(&self) -> FrameId {
        self.frame_id
    }

    /// Widgets that contain [widget_id](WidgetPath::widget_id), root first.
    #[inline]
    pub fn ancestors(&self) -> &[WidgetId] {
        &self.path[..self.path.len() - 2]
    }

    /// The widget.
    #[inline]
    pub fn widget_id(&self) -> WidgetId {
        self.path[self.path.len() - 1]
    }

    /// [ancestors](WidgetPath::ancestors) and [widget_id](WidgetPath::widget_id), root first.
    #[inline]
    pub fn widgets_path(&self) -> &[WidgetId] {
        &self.path[..]
    }

    /// If the `widget_id` is part of the path.
    #[inline]
    pub fn contains(&self, widget_id: WidgetId) -> bool {
        self.path.iter().any(move |&w| w == widget_id)
    }

    /// Get the inner most widget parent shared by both `self` and `other`.
    ///
    /// The [frame_id](WidgetPath::frame_id) of `self` is used in the result.
    #[inline]
    pub fn shared_ancestor(&self, other: &WidgetPath) -> Option<WidgetPath> {
        if self.window_id == other.window_id {
            let mut path = Vec::default();

            for (a, b) in self.path.iter().zip(other.path.iter()) {
                if a != b {
                    break;
                }
                path.push(*a);
            }

            if !path.is_empty() {
                return Some(WidgetPath {
                    node_id: None,
                    window_id: self.window_id,
                    frame_id: self.frame_id,
                    path: path.into(),
                });
            }
        }
        None
    }
}

struct WidgetInfoInner {
    widget_id: WidgetId,
    bounds: LayoutRect,
    meta: LazyStateMap,
}

/// Reference to a widget info in a [FrameInfo].
#[derive(Clone, Copy)]
pub struct WidgetInfo<'a> {
    frame: &'a FrameInfo,
    node_id: ego_tree::NodeId,
}

impl<'a> WidgetInfo<'a> {
    #[inline]
    fn new(frame: &'a FrameInfo, node_id: ego_tree::NodeId) -> Self {
        Self { frame, node_id }
    }

    #[inline]
    fn node(&self) -> ego_tree::NodeRef<'a, WidgetInfoInner> {
        unsafe { self.frame.tree.get_unchecked(self.node_id) }
    }

    #[inline]
    fn info(&self) -> &'a WidgetInfoInner {
        self.node().value()
    }

    /// Widget id.
    #[inline]
    pub fn widget_id(self) -> WidgetId {
        self.info().widget_id
    }

    /// Full path to this widget.
    #[inline]
    pub fn path(self) -> WidgetPath {
        let mut path: Vec<_> = self.ancestors().map(|a| a.widget_id()).collect();
        path.reverse();
        path.push(self.widget_id());

        WidgetPath {
            frame_id: self.frame.frame_id,
            window_id: self.frame.window_id,
            node_id: Some(self.node_id),
            path: path.into(),
        }
    }

    /// Widget rectangle in the frame.
    #[inline]
    pub fn bounds(self) -> &'a LayoutRect {
        &self.info().bounds
    }

    /// Widget bounds center.
    #[inline]
    pub fn center(self) -> LayoutPoint {
        self.bounds().center()
    }

    /// Metadata associated with the widget during render.
    #[inline]
    pub fn meta(self) -> &'a LazyStateMap {
        &self.info().meta
    }

    /// Reference to the frame root widget.
    #[inline]
    pub fn root(self) -> Self {
        self.ancestors().last().unwrap_or(self)
    }

    /// Reference to the widget that contains this widget.
    ///
    /// Is `None` only for [root](FrameInfo::root).
    #[inline]
    pub fn parent(self) -> Option<Self> {
        self.node().parent().map(move |n| WidgetInfo::new(self.frame, n.id()))
    }

    /// Reference to the previous widget within the same parent.
    #[inline]
    pub fn prev_sibling(self) -> Option<Self> {
        self.node().prev_sibling().map(move |n| WidgetInfo::new(self.frame, n.id()))
    }

    /// Reference to the next widget within the same parent.
    #[inline]
    pub fn next_sibling(self) -> Option<Self> {
        self.node().next_sibling().map(move |n| WidgetInfo::new(self.frame, n.id()))
    }

    /// Reference to the first widget within this widget.
    #[inline]
    pub fn first_child(self) -> Option<Self> {
        self.node().first_child().map(move |n| WidgetInfo::new(self.frame, n.id()))
    }

    /// Reference to the last widget within this widget.
    #[inline]
    pub fn last_child(self) -> Option<Self> {
        self.node().last_child().map(move |n| WidgetInfo::new(self.frame, n.id()))
    }

    /// If the parent widget has multiple children.
    #[inline]
    pub fn has_siblings(self) -> bool {
        self.node().has_siblings()
    }

    /// If the widget has at least one child.
    #[inline]
    pub fn has_children(self) -> bool {
        self.node().has_children()
    }

    /// All parent children except this widget.
    #[inline]
    pub fn siblings(self) -> impl Iterator<Item = WidgetInfo<'a>> {
        self.prev_siblings().chain(self.next_siblings())
    }

    /// Iterator over the widgets directly contained by this widget.
    #[inline]
    pub fn children(self) -> impl DoubleEndedIterator<Item = WidgetInfo<'a>> {
        self.node().children().map(move |n| WidgetInfo::new(self.frame, n.id()))
    }

    /// Iterator over all widgets contained by this widget.
    #[inline]
    pub fn descendants(self) -> impl Iterator<Item = WidgetInfo<'a>> {
        self.node().descendants().map(move |n| WidgetInfo::new(self.frame, n.id()))
    }

    /// Iterator over parent -> grant-parent -> .. -> root.
    #[inline]
    pub fn ancestors(self) -> impl Iterator<Item = WidgetInfo<'a>> {
        self.node().ancestors().map(move |n| WidgetInfo::new(self.frame, n.id()))
    }

    /// Iterator over all previous widgets within the same parent.
    #[inline]
    pub fn prev_siblings(self) -> impl Iterator<Item = WidgetInfo<'a>> {
        self.node().prev_siblings().map(move |n| WidgetInfo::new(self.frame, n.id()))
    }

    /// Iterator over all next widgets within the same parent.
    #[inline]
    pub fn next_siblings(self) -> impl Iterator<Item = WidgetInfo<'a>> {
        self.node().next_siblings().map(move |n| WidgetInfo::new(self.frame, n.id()))
    }

    /// This widgets orientation in relation to a `origin`.
    #[inline]
    pub fn orientation_from(self, origin: LayoutPoint) -> WidgetOrientation {
        let o = self.center();
        for &d in &[
            WidgetOrientation::Left,
            WidgetOrientation::Right,
            WidgetOrientation::Above,
            WidgetOrientation::Below,
        ] {
            if is_in_direction(d, origin, o) {
                return d;
            }
        }
        unreachable!()
    }

    ///Iterator over all parent children except this widget with orientation in relation
    /// to this widget center.
    #[inline]
    pub fn oriented_siblings(self) -> impl Iterator<Item = (WidgetInfo<'a>, WidgetOrientation)> {
        let c = self.center();
        self.siblings().map(move |s| (s, s.orientation_from(c)))
    }

    /// All parent children except this widget, sorted by closest first.
    #[inline]
    pub fn closest_siblings(self) -> Vec<WidgetInfo<'a>> {
        self.closest_first(self.siblings())
    }

    /// All parent children except this widget, sorted by closest first and with orientation in
    /// relation to this widget center.
    #[inline]
    pub fn closest_oriented_siblings(self) -> Vec<(WidgetInfo<'a>, WidgetOrientation)> {
        let mut vec: Vec<_> = self.oriented_siblings().collect();
        let origin = self.center();
        vec.sort_by_cached_key(|n| n.0.distance_key(origin));
        vec
    }

    /// Unordered siblings to the left of this widget.
    #[inline]
    pub fn un_left_siblings(self) -> impl Iterator<Item = WidgetInfo<'a>> {
        self.oriented_siblings().filter_map(|(s, o)| match o {
            WidgetOrientation::Left => Some(s),
            _ => None,
        })
    }

    /// Unordered siblings to the right of this widget.
    #[inline]
    pub fn un_right_siblings(self) -> impl Iterator<Item = WidgetInfo<'a>> {
        self.oriented_siblings().filter_map(|(s, o)| match o {
            WidgetOrientation::Right => Some(s),
            _ => None,
        })
    }

    /// Unordered siblings to the above of this widget.
    #[inline]
    pub fn un_above_siblings(self) -> impl Iterator<Item = WidgetInfo<'a>> {
        self.oriented_siblings().filter_map(|(s, o)| match o {
            WidgetOrientation::Above => Some(s),
            _ => None,
        })
    }

    /// Unordered siblings to the below of this widget.
    #[inline]
    pub fn un_below_siblings(self) -> impl Iterator<Item = WidgetInfo<'a>> {
        self.oriented_siblings().filter_map(|(s, o)| match o {
            WidgetOrientation::Below => Some(s),
            _ => None,
        })
    }

    /// Siblings to the left of this widget sorted by closest first.
    #[inline]
    pub fn left_siblings(self) -> Vec<WidgetInfo<'a>> {
        self.closest_first(self.un_left_siblings())
    }

    /// Siblings to the right of this widget sorted by closest first.
    #[inline]
    pub fn right_siblings(self) -> Vec<WidgetInfo<'a>> {
        self.closest_first(self.un_right_siblings())
    }

    /// Siblings to the above of this widget sorted by closest first.
    #[inline]
    pub fn above_siblings(self) -> Vec<WidgetInfo<'a>> {
        self.closest_first(self.un_above_siblings())
    }

    /// Siblings to the below of this widget sorted by closest first.
    #[inline]
    pub fn below_siblings(self) -> Vec<WidgetInfo<'a>> {
        self.closest_first(self.un_below_siblings())
    }

    /// Value that indicates the distance between this widget center
    /// and `origin`.
    #[inline]
    pub fn distance_key(self, origin: LayoutPoint) -> usize {
        let o = self.center();
        let a = (o.x - origin.x).powf(2.);
        let b = (o.y - origin.y).powf(2.);
        (a + b) as usize
    }

    fn closest_first(self, iter: impl Iterator<Item = WidgetInfo<'a>>) -> Vec<WidgetInfo<'a>> {
        let mut vec: Vec<_> = iter.collect();
        let origin = self.center();
        vec.sort_by_cached_key(|n| n.distance_key(origin));
        vec
    }
}

#[inline]
fn is_in_direction(direction: WidgetOrientation, origin: LayoutPoint, candidate: LayoutPoint) -> bool {
    let (a, b, c, d) = match direction {
        WidgetOrientation::Left => (candidate.x, origin.x, candidate.y, origin.y),
        WidgetOrientation::Right => (origin.x, candidate.x, candidate.y, origin.y),
        WidgetOrientation::Above => (candidate.y, origin.y, candidate.x, origin.x),
        WidgetOrientation::Below => (origin.y, candidate.y, candidate.x, origin.x),
    };

    // checks if the candidate point is in between two imaginary perpendicular lines parting from the
    // origin point in the focus direction
    if a <= b {
        if c >= d {
            return c <= d + (b - a);
        } else {
            return c >= d - (b - a);
        }
    }

    false
}

/// Orientation of a [WidgetInfo] relative to another point.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum WidgetOrientation {
    Left,
    Right,
    Above,
    Below,
}
