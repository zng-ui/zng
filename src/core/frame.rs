use crate::core::context::LazyStateMap;
use crate::core::types::*;
use crate::core::UiNode;
use ego_tree::Tree;
use webrender::api::*;

pub struct FrameBuilder {
    pub display_list: DisplayListBuilder,
    info: FrameInfoBuilder,
    widget_id: WidgetId,
    cursor: CursorIcon,
}

impl FrameBuilder {
    pub fn new(window_id: WindowId, frame_id: Epoch, root_id: WidgetId, root_size: LayoutSize, pipeline_id: PipelineId) -> Self {
        FrameBuilder {
            display_list: DisplayListBuilder::new(pipeline_id, root_size),
            info: FrameInfoBuilder::new(window_id, frame_id, root_id, root_size),
            widget_id: root_id,
            cursor: CursorIcon::default(),
        }
    }

    /// Current widget.
    pub fn widget_id(&self) -> WidgetId {
        self.widget_id
    }

    /// Current widget metadata.
    pub fn widget_meta(&mut self) -> &mut LazyStateMap {
        todo!()
    }

    fn item_tag(&self) -> ItemTag {
        (self.widget_id.get(), self.cursor as u16)
    }

    pub(crate) fn push_widget(&mut self, id: WidgetId, child: &impl UiNode) {
        let widget_hit = (id, u16::max_value());
        // self.push_hit_rect(widget_hit);

        let parent = std::mem::replace(&mut self.widget_id, id);
        child.render(self);
        self.widget_id = parent;
    }

    pub fn push_ui_node(&mut self, child: &impl UiNode, rect: &LayoutRect) {
        todo!()
    }

    pub fn push_border(&mut self, rect: &LayoutRect, widths: LayoutSideOffsets, details: BorderDetails) {
        todo!()
    }

    pub fn push_text(
        &mut self,
        rect: &LayoutRect,
        glyphs: &[GlyphInstance],
        font_instance_key: FontInstanceKey,
        color: ColorF,
        glyph_options: Option<GlyphOptions>,
    ) {
        todo!()
    }

    pub fn push_cursor(&mut self, cursor: CursorIcon, node: &impl UiNode) {
        let parent_cursor = std::mem::replace(&mut self.cursor, cursor);
        node.render(self);
        self.cursor = parent_cursor;
    }

    pub fn push_fill_color(&mut self, rect: &LayoutRect, color: ColorF) {
        todo!()
    }

    pub fn push_fill_gradient(&mut self, rect: &LayoutRect, start: LayoutPoint, end: LayoutPoint, stops: Vec<GradientStop>) {
        todo!()
    }

    pub fn finalize(self) -> ((PipelineId, LayoutSize, BuiltDisplayList), FrameInfo) {
        (self.display_list.finalize(), self.info.build())
    }
}

fn is_widget(raw: u16) -> bool {
    raw == u16::max_value()
}

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
#[derive(Clone, Debug, Default)]
pub struct FrameHitInfo {
    hits: Vec<HitInfo>,
}

impl FrameHitInfo {
    /// Initializes from a webrender hit-test result.
    #[inline]
    pub fn new(hits: HitTestResult) -> Self {
        // TODO solve: using the same WidgetId in multiple properties
        // will result in repeated entries here with potentially different
        // hit points, that don't match with the widget area.
        todo!()
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
}

/// [FrameInfo] builder.
pub struct FrameInfoBuilder {
    window_id: WindowId,
    frame_id: Epoch,
    tree: Tree<WidgetInfoInner>,
}

impl FrameInfoBuilder {
    /// Starts building a frame info with the frame root information.
    pub fn new(window_id: WindowId, frame_id: Epoch, root_id: WidgetId, size: LayoutSize) -> Self {
        FrameInfoBuilder {
            window_id,
            frame_id,
            tree: Tree::new(WidgetInfoInner {
                widget_id: root_id,
                bounds: LayoutRect::from_size(size),
                meta: LazyStateMap::default(),
            }),
        }
    }

    /// Builds the final frame info.
    pub fn build(self) -> FrameInfo {
        FrameInfo {
            window_id: self.window_id,
            frame_id: self.frame_id,
            lookup: self.tree.nodes().map(|n| (n.value().widget_id, n.id())).collect(),
            tree: self.tree,
        }
    }
}

/// Information about a rendered frame.
///
/// Instantiated using [FrameInfoBuilder].
pub struct FrameInfo {
    window_id: WindowId,
    frame_id: Epoch,
    tree: Tree<WidgetInfoInner>,
    lookup: fnv::FnvHashMap<WidgetId, ego_tree::NodeId>,
}

impl FrameInfo {
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
    pub fn frame_id(&self) -> Epoch {
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
            self.tree.get(path.node_id).map(|n| WidgetInfo::new(self, n.id()))
        } else {
            None
        }
    }
}

/// Full address of a widget in a specific [FrameInfo].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetPath {
    node_id: ego_tree::NodeId,
    window_id: WindowId,
    frame_id: Epoch,
    path: Box<[WidgetId]>,
}

impl WidgetPath {
    /// Window the [frame_id] belongs too.
    #[inline]
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    /// The frame of [window_id] this path was computed.
    #[inline]
    pub fn frame_id(&self) -> Epoch {
        self.frame_id
    }

    /// Widgets that contain [widget_id], root first.
    #[inline]
    pub fn ancestors(&self) -> &[WidgetId] {
        &self.path[..self.path.len() - 2]
    }

    /// The widget.
    #[inline]
    pub fn widget_id(&self) -> WidgetId {
        self.path[self.path.len() - 1]
    }

    /// [ancestors] and [widget_id], root first.
    #[inline]
    pub fn widgets_path(&self) -> &[WidgetId] {
        &self.path[..]
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
            node_id: self.node_id,
            path: path.into(),
        }
    }

    /// Widget retangle in the frame.
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
        _ => unreachable!(),
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
