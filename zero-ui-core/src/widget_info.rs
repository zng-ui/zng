//! Widget info tree.

use std::cell::Cell;
use std::fmt;
use std::mem;
use std::ops;
use std::rc::Rc;

use ego_tree::Tree;

use crate::context::Updates;
use crate::crate_util::IdMap;
use crate::event::EventUpdateArgs;
use crate::handler::WidgetHandler;
use crate::state::OwnedStateMap;
use crate::state::StateMap;
use crate::units::*;
use crate::var::Var;
use crate::var::VarValue;
use crate::var::VarsRead;
use crate::var::WithVarsRead;
use crate::widget_base::Visibility;
use crate::window::WindowId;
use crate::WidgetId;

unique_id_64! {
    /// Identifies a [`WidgetInfoTree`] snapshot, can be use for more speedy [`WidgetPath`] resolution.
    struct WidgetInfoTreeId;
}

/// Helper for computing widget bounds during [`UiNode::arrange`].
///
/// Widget bounds are kept up-to-date in [`WidgetInfo`], properties that offset their child nodes must
/// use [`with_offset`] to note this offset, custom widget implementers use [`with_widget`] and [`with_inner`] to
/// mark the widget bounds.
///
/// [`UiNode::arrange`]: crate::UiNode::arrange
/// [`with_widget`]: WidgetOffset::with_widget
/// [`with_offset`]: WidgetOffset::with_offset
/// [`with_inner`]: WidgetOffset::with_inner
pub struct WidgetOffset {
    offset: PxPoint,
    inner_bounds: PxRect,
}
impl WidgetOffset {
    /// New root.
    pub(crate) fn new() -> Self {
        WidgetOffset {
            offset: PxPoint::zero(),
            inner_bounds: PxRect::zero(),
        }
    }

    /// Calls `f` within the scope of a new widget outer bounds, both widget bounds are updated,
    /// the [`implicit_base::new`] function calls this method, custom implementation of `new` must
    /// either wrap its result on the implicit implementation or call this method directly.
    ///
    /// [`implicit_base::new`]: crate::widget_base::implicit_base::new
    pub fn with_widget(&mut self, outer_bounds: &BoundsRect, inner_bounds: &BoundsRect, final_size: PxSize, f: impl FnOnce(&mut Self)) {
        let wgt_bounds = PxRect::new(self.offset, final_size);
        outer_bounds.set(wgt_bounds);

        f(self);

        inner_bounds.set(self.inner_bounds);

        // we are the inner-bounds of parent too if it did not set one.
        self.inner_bounds = wgt_bounds;
    }

    /// Returns the current offset.
    pub fn offset(&self) -> PxPoint {
        self.offset
    }

    /// Calls `f` with an added offset, every property that offsets its child node must
    /// call [`UiNode::arrange`] on the child inside `f`.
    ///
    /// [`UiNode::arrange`]: crate::UiNode::arrange
    pub fn with_offset(&mut self, offset: PxVector, f: impl FnOnce(&mut Self)) {
        self.offset += offset;
        f(self);
        self.offset -= offset;
    }

    /// Calls `f` with an offset overwrite, for layouts where the widget is relative to the window root.
    pub fn with_root_offset(&mut self, offset: PxPoint, f: impl FnOnce(&mut Self)) {
        let offset = mem::replace(&mut self.offset, offset);
        f(self);
        self.offset = offset;
    }

    /// Calls `f` within a [`RenderTransform`] that modifies all inner widgets.
    pub fn with_transform(&mut self, transform: &RenderTransform, f: impl FnOnce(&mut Self)) {
        // TODO incorporate all transforms to calculate child bounds.
        let offset = transform.transform_vector2d(euclid::vec2(0.0, 0.0));
        self.with_offset(PxVector::new(Px(offset.x as i32), Px(offset.y as i32)), f)
    }

    /// Marks the widget *size* bounds, the [`implicit_base::new_inner`] function calls this method, custom implementations
    /// of `new_inner` must either wrap its result on the implicit implementation or call this method directly.
    ///
    /// [`implicit_base::new_inner`]: crate::widget_base::implicit_base::new_inner
    pub fn with_inner(&mut self, final_size: PxSize, f: impl FnOnce(&mut Self)) {
        f(self);
        self.inner_bounds = PxRect::new(self.offset, final_size);
    }
}

/// [`WidgetInfoTree`] builder.
pub struct WidgetInfoBuilder {
    window_id: WindowId,

    node: ego_tree::NodeId,
    widget_id: WidgetId,
    meta: OwnedStateMap,

    tree: Tree<WidgetInfoInner>,
}
impl WidgetInfoBuilder {
    /// Starts building a info tree with the root information, the `root_bounds` must be a shared reference
    /// to the size of the window content that is always kept up-to-date, the origin must be always zero.
    #[inline]
    pub fn new(
        window_id: WindowId,
        root_id: WidgetId,
        root_bounds: BoundsRect,
        rendered: WidgetRendered,
        used_data: Option<UsedWidgetInfoBuilder>,
    ) -> Self {
        debug_assert_eq!(PxPoint::zero(), root_bounds.get().origin);

        let tree = Tree::with_capacity(
            WidgetInfoInner {
                widget_id: root_id,
                inner_bounds: root_bounds.clone(),
                outer_bounds: root_bounds,
                rendered,
                meta: OwnedStateMap::new(),
            },
            used_data.map(|d| d.capacity).unwrap_or(100),
        );

        let root_node = tree.root().id();
        WidgetInfoBuilder {
            window_id,
            node: root_node,
            tree,
            meta: OwnedStateMap::new(),
            widget_id: root_id,
        }
    }

    #[inline]
    fn node(&mut self, id: ego_tree::NodeId) -> ego_tree::NodeMut<WidgetInfoInner> {
        self.tree.get_mut(id).unwrap()
    }

    /// Current widget id.
    #[inline]
    pub fn widget_id(&self) -> WidgetId {
        self.widget_id
    }

    /// Current widget metadata.
    #[inline]
    pub fn meta(&mut self) -> &mut StateMap {
        &mut self.meta.0
    }

    /// Calls `f` in a new widget context.
    ///
    /// Both `outer_bounds` and `bounds` must be a shared reference to rectangles that are updated every layout.
    #[inline]
    pub fn push_widget(
        &mut self,
        id: WidgetId,
        outer_bounds: BoundsRect,
        inner_bounds: BoundsRect,
        rendered: WidgetRendered,
        f: impl FnOnce(&mut Self),
    ) {
        let parent_node = self.node;
        let parent_widget_id = self.widget_id;
        let parent_meta = mem::take(&mut self.meta);

        self.widget_id = id;
        self.node = self
            .node(parent_node)
            .append(WidgetInfoInner {
                widget_id: id,
                inner_bounds,
                outer_bounds,
                rendered,
                meta: OwnedStateMap::new(),
            })
            .id();

        f(self);

        self.node(self.node).value().meta = mem::replace(&mut self.meta, parent_meta);
        self.node = parent_node;
        self.widget_id = parent_widget_id;
    }

    /// Build the info tree.
    #[inline]
    pub fn finalize(mut self) -> (WidgetInfoTree, UsedWidgetInfoBuilder) {
        self.tree.root_mut().value().meta = self.meta;
        let root_id = self.tree.root().id();

        // we build a WidgetId => NodeId lookup
        //
        // in debug mode we validate that the same WidgetId is not repeated
        //
        let valid_nodes = self
            .tree
            .nodes()
            .filter(|n| n.parent().is_some() || n.id() == root_id)
            .map(|n| (n.value().widget_id, n.id()));

        #[cfg(debug_assertions)]
        let (repeats, lookup) = {
            let mut repeats = IdMap::default();
            let mut lookup = IdMap::default();

            for (widget_id, node_id) in valid_nodes {
                if let Some(prev) = lookup.insert(widget_id, node_id) {
                    repeats.entry(widget_id).or_insert_with(|| vec![prev]).push(node_id);
                }
            }

            (repeats, lookup)
        };
        #[cfg(not(debug_assertions))]
        let lookup = valid_nodes.collect();

        let r = WidgetInfoTree(Rc::new(WidgetInfoTreeInner {
            id: WidgetInfoTreeId::new_unique(),
            window_id: self.window_id,
            lookup,
            tree: self.tree,
        }));

        #[cfg(debug_assertions)]
        for (widget_id, repeats) in repeats {
            tracing::error!(target: "render", "widget id `{widget_id:?}` appears more then once in {:?}{}", self.window_id, {
                let mut places = String::new();
                use std::fmt::Write;
                for node in &repeats {
                    let info = WidgetInfo::new(&r, *node);
                    write!(places, "\n    {}", info.path()).unwrap();
                }
                places
            });
        }

        let cap = UsedWidgetInfoBuilder {
            capacity: r.0.lookup.capacity(),
        };

        (r, cap)
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
    id: WidgetInfoTreeId,
    window_id: WindowId,
    tree: Tree<WidgetInfoInner>,
    lookup: IdMap<WidgetId, ego_tree::NodeId>,
}
impl WidgetInfoTree {
    /// Blank window that contains only the root widget taking no space.
    #[inline]
    pub fn blank(window_id: WindowId, root_id: WidgetId) -> Self {
        WidgetInfoBuilder::new(window_id, root_id, BoundsRect::new(), WidgetRendered::new(), None)
            .finalize()
            .0
    }

    /// Reference to the root widget in the tree.
    #[inline]
    pub fn root(&self) -> WidgetInfo {
        WidgetInfo::new(self, self.0.tree.root().id())
    }

    /// All widgets including `root`.
    #[inline]
    pub fn all_widgets(&self) -> impl Iterator<Item = WidgetInfo> {
        self.0.tree.root().descendants().map(move |n| WidgetInfo::new(self, n.id()))
    }

    /// Id of the window that owns all widgets represented in the tree.
    #[inline]
    pub fn window_id(&self) -> WindowId {
        self.0.window_id
    }

    /// Reference to the widget in the tree, if it is present.
    #[inline]
    pub fn find(&self, widget_id: WidgetId) -> Option<WidgetInfo> {
        self.0
            .lookup
            .get(&widget_id)
            .and_then(|i| self.0.tree.get(*i).map(|n| WidgetInfo::new(self, n.id())))
    }

    /// If the tree contains the widget.
    #[inline]
    pub fn contains(&self, widget_id: WidgetId) -> bool {
        self.0.lookup.contains_key(&widget_id)
    }

    /// Reference to the widget in the tree, if it is present.
    ///
    /// Faster then [`find`](Self::find) if the widget path was generated by `self`.
    #[inline]
    pub fn get(&self, path: &WidgetPath) -> Option<WidgetInfo> {
        if let Some((tree_id, id)) = path.node_id {
            if tree_id == self.0.id {
                return self.0.tree.get(id).map(|n| WidgetInfo::new(self, n.id()));
            }
        }

        self.find(path.widget_id())
    }

    /// Reference to the widget or first parent that is present.
    #[inline]
    pub fn get_or_parent(&self, path: &WidgetPath) -> Option<WidgetInfo> {
        self.get(path)
            .or_else(|| path.ancestors().iter().rev().find_map(|&id| self.find(id)))
    }
}
impl fmt::Debug for WidgetInfoTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let nl = if f.alternate() { "\n   " } else { " " };

        write!(
            f,
            "WidgetInfoTree(Rc<{{{nl}id: {},{nl}window_id: {},{nl}widget_count: {},{nl}...}}>)",
            self.0.id.sequential(),
            self.0.window_id,
            self.0.lookup.len(),
            nl = nl
        )
    }
}

/// Full address of a widget in a specific [`WidgetInfoTree`].
#[derive(Clone)]
pub struct WidgetPath {
    node_id: Option<(WidgetInfoTreeId, ego_tree::NodeId)>,
    window_id: WindowId,
    path: Box<[WidgetId]>,
}
impl PartialEq for WidgetPath {
    /// Paths are equal if they share the same [window](Self::window_id) and [widget paths](Self::widgets_path).
    fn eq(&self, other: &Self) -> bool {
        self.window_id == other.window_id && self.path == other.path
    }
}
impl Eq for WidgetPath {}
impl fmt::Debug for WidgetPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("WidgetPath")
                .field("window_id", &self.window_id)
                .field("path", &self.path)
                .finish_non_exhaustive()
        } else {
            write!(f, "{self}")
        }
    }
}
impl fmt::Display for WidgetPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}//", self.window_id)?;
        for w in self.ancestors() {
            write!(f, "{w}/")?;
        }
        write!(f, "{}", self.widget_id())
    }
}
impl WidgetPath {
    /// New custom widget path.
    ///
    /// The path is not guaranteed to have ever existed.
    pub fn new<P: Into<Box<[WidgetId]>>>(window_id: WindowId, path: P) -> WidgetPath {
        WidgetPath {
            node_id: None,
            window_id,
            path: path.into(),
        }
    }

    /// Id of the window that contains the widgets.
    #[inline]
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    /// Widgets that contain [`widget_id`](WidgetPath::widget_id), root first.
    #[inline]
    pub fn ancestors(&self) -> &[WidgetId] {
        &self.path[..self.path.len() - 1]
    }

    /// The widget.
    #[inline]
    pub fn widget_id(&self) -> WidgetId {
        self.path[self.path.len() - 1]
    }

    /// [`ancestors`](WidgetPath::ancestors) and [`widget_id`](WidgetPath::widget_id), root first.
    #[inline]
    pub fn widgets_path(&self) -> &[WidgetId] {
        &self.path[..]
    }

    /// If the `widget_id` is part of the path.
    #[inline]
    pub fn contains(&self, widget_id: WidgetId) -> bool {
        self.path.iter().any(move |&w| w == widget_id)
    }

    /// Make a path to an ancestor id that is contained in the current path.
    #[inline]
    pub fn ancestor_path(&self, ancestor_id: WidgetId) -> Option<WidgetPath> {
        self.path.iter().position(|&id| id == ancestor_id).map(|i| WidgetPath {
            node_id: None,
            window_id: self.window_id,
            path: self.path[..i].iter().copied().collect(),
        })
    }

    /// Get the inner most widget parent shared by both `self` and `other`.
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
                    path: path.into(),
                });
            }
        }
        None
    }

    /// Gets a path to the root widget of this path.
    #[inline]
    pub fn root_path(&self) -> WidgetPath {
        WidgetPath {
            node_id: None,
            window_id: self.window_id,
            path: Box::new([self.path[0]]),
        }
    }
}

/// Shared reference to the bounds of a [`WidgetInfo`].
///
/// Widget bounds are updated every layout without causing a tree rebuild.
#[derive(Default, Clone, Debug)]
pub struct BoundsRect(Rc<Cell<PxRect>>);
impl BoundsRect {
    /// New default.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// New with `size` and origin zero.
    #[inline]
    pub fn from_size(size: PxSize) -> Self {
        BoundsRect(Rc::new(Cell::new(PxRect::from_size(size))))
    }

    /// Get a copy of the current bounds.
    #[inline]
    pub fn get(&self) -> PxRect {
        self.0.get()
    }

    /// Replace the current bounds.
    #[inline]
    pub fn set(&self, bounds: PxRect) {
        self.0.set(bounds);
    }

    /// Replace the current origin.
    #[inline]
    pub fn set_origin(&self, origin: PxPoint) {
        let mut rect = self.0.get();
        rect.origin = origin;
        self.0.set(rect);
    }

    /// Replace the current size.
    #[inline]
    pub fn set_size(&self, size: PxSize) {
        let mut rect = self.0.get();
        rect.size = size;
        self.0.set(rect);
    }
}

#[derive(Default, Debug)]
struct BoundsData {
    transform: Cell<RenderTransform>,
    size: Cell<PxSize>,
}

/// Shared reference to the transform and size of a [`WidgetInfo`] outer or inner bounds.
#[derive(Default, Clone, Debug)]
pub struct BoundsInfo(Rc<BoundsData>);
impl BoundsInfo {
    /// New default.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a copy of the current transform.
    ///
    /// The origin is the window root top-left.
    #[inline]
    pub fn transform(&self) -> RenderTransform {
        self.0.transform.get()
    }

    /// Set the current transform.
    #[inline]
    pub fn set_transform(&self, transform: RenderTransform) {
        self.0.transform.set(transform)
    }

    /// Get a copy of the current raw size.
    ///
    /// Note that this is not transformed.
    #[inline]
    pub fn size(&self) -> PxSize {
        self.0.size.get()
    }

    /// Set the current raw size.
    #[inline]
    pub fn set_size(&self, size: PxSize) {
        self.0.size.set(size)
    }

    /// Calculate the bounding box.
    pub fn bounds(&self) -> PxRect {
        let transform = self.transform();
        let size = self.size();

        let rect = PxRect::from_size(size).to_wr();
        let bounds = transform.outer_transformed_box2d(&rect).unwrap();

        bounds.to_px()
    }
}

/// Shared reference to the rendered status of a [`WidgetInfo`].
///
/// This status is updated every [`render`] without causing a tree rebuild.
///
/// [`render`]: crate::UiNode::render
#[derive(Default, Clone, Debug)]
pub struct WidgetRendered(Rc<Cell<bool>>);
impl WidgetRendered {
    /// New default.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get if the widget or child widgets rendered.
    #[inline]
    pub fn get(&self) -> bool {
        self.0.get()
    }

    /// Set if the widget or child widgets rendered.
    #[inline]
    pub fn set(&self, rendered: bool) {
        self.0.set(rendered);
    }
}

struct WidgetInfoInner {
    widget_id: WidgetId,
    outer_bounds: BoundsRect,
    inner_bounds: BoundsRect,
    rendered: WidgetRendered,
    meta: OwnedStateMap,
}

/// Reference to a widget info in a [`WidgetInfoTree`].
#[derive(Clone, Copy)]
pub struct WidgetInfo<'a> {
    tree: &'a WidgetInfoTree,
    node_id: ego_tree::NodeId,
}
impl<'a> PartialEq for WidgetInfo<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.node_id == other.node_id
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
            .finish()
    }
}

impl<'a> WidgetInfo<'a> {
    #[inline]
    fn new(tree: &'a WidgetInfoTree, node_id: ego_tree::NodeId) -> Self {
        Self { tree, node_id }
    }

    #[inline]
    fn node(&self) -> ego_tree::NodeRef<'a, WidgetInfoInner> {
        unsafe { self.tree.0.tree.get_unchecked(self.node_id) }
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
            window_id: self.tree.0.window_id,
            node_id: Some((self.tree.0.id, self.node_id)),
            path: path.into(),
        }
    }

    /// Gets the [`path`](Self::path) if it is different from `old_path`.
    ///
    /// Only allocates a new path if needed.
    ///
    /// # Panics
    ///
    /// If `old_path` does not point to the same widget id as `self`.
    #[inline]
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

    /// Returns `true` if the widget or the widget's descendants rendered in the last frame.
    ///
    /// This value is updated every [`render`] without causing a tree rebuild.
    ///
    /// [`render`]: crate::UiNode::render
    #[inline]
    pub fn rendered(self) -> bool {
        // widgets that don't render tend to not call `UiNode::render` on children,
        // so we need to check all parents because our flag can be out-of-date.
        self.info().rendered.get() && self.ancestors().all(|p| p.info().rendered.get())
    }

    /// Compute the visibility of the widget or the widget's descendants.
    ///
    /// If is [`rendered`] is [`Visible`], if not and the [`outer_bounds`] size is zero then is [`Collapsed`] else
    /// is [`Hidden`].
    ///
    /// [`rendered`]: Self::rendered
    /// [`Visible`]: Visibility::Visible
    /// [`outer_bounds`]: Self::outer_bounds
    /// [`Collapsed`]: Visibility::Collapsed
    /// [`Hidden`]: Visibility::Hidden
    #[inline]
    pub fn visibility(self) -> Visibility {
        if self.rendered() {
            Visibility::Visible
        } else if self.outer_bounds().size == PxSize::zero() {
            Visibility::Collapsed
        } else {
            Visibility::Hidden
        }
    }

    /// Widget rectangle in the window space, including *outer* properties like margin.
    ///
    /// Returns an up-to-date rect, the bounds are updated every layout without causing a tree rebuild.
    #[inline]
    pub fn outer_bounds(self) -> PxRect {
        self.info().outer_bounds.get()
    }

    /// Widget rectangle in the window space, but only the visible *inner* properties.
    ///
    /// Returns an up-to-date rect, the bounds are updated every layout without causing a tree rebuild.
    #[inline]
    pub fn inner_bounds(self) -> PxRect {
        self.info().inner_bounds.get()
    }

    /// Calculate the offsets from `outer_bounds` to `bounds`.
    pub fn outer_offsets(self) -> PxSideOffsets {
        let info = self.info();
        let outer_bounds = info.outer_bounds.get();
        let bounds = info.inner_bounds.get();

        Self::calc_offsets(outer_bounds, bounds)
    }

    /// Calculate the offsets from root's `outer_bounds` to this widget's `bounds`.
    pub fn root_offsets(self) -> PxSideOffsets {
        let outer = self.root().outer_bounds();
        let inner = self.inner_bounds();

        Self::calc_offsets(outer, inner)
    }

    fn calc_offsets(outer: PxRect, inner: PxRect) -> PxSideOffsets {
        let top = outer.origin.y - inner.origin.y;
        let left = outer.origin.x - inner.origin.x;
        let right = outer.size.width - inner.size.width;
        let bottom = outer.size.height - inner.size.height;

        PxSideOffsets::new(top, right, bottom, left)
    }

    /// Widget inner bounds center.
    #[inline]
    pub fn center(self) -> PxPoint {
        self.inner_bounds().center()
    }

    /// Metadata associated with the widget during render.
    #[inline]
    pub fn meta(self) -> &'a StateMap {
        &self.info().meta.0
    }

    /// Reference the [`WidgetInfoTree`] that owns `self`.
    #[inline]
    pub fn tree(self) -> &'a WidgetInfoTree {
        self.tree
    }

    /// Reference to the root widget.
    #[inline]
    pub fn root(self) -> Self {
        self.ancestors().last().unwrap_or(self)
    }

    /// Reference to the widget that contains this widget.
    ///
    /// Is `None` only for [`root`](WidgetInfoTree::root).
    #[inline]
    pub fn parent(self) -> Option<Self> {
        self.node().parent().map(move |n| WidgetInfo::new(self.tree, n.id()))
    }

    /// Reference to the previous widget within the same parent.
    #[inline]
    pub fn prev_sibling(self) -> Option<Self> {
        self.node().prev_sibling().map(move |n| WidgetInfo::new(self.tree, n.id()))
    }

    /// Reference to the next widget within the same parent.
    #[inline]
    pub fn next_sibling(self) -> Option<Self> {
        self.node().next_sibling().map(move |n| WidgetInfo::new(self.tree, n.id()))
    }

    /// Reference to the first widget within this widget.
    #[inline]
    pub fn first_child(self) -> Option<Self> {
        self.node().first_child().map(move |n| WidgetInfo::new(self.tree, n.id()))
    }

    /// Reference to the last widget within this widget.
    #[inline]
    pub fn last_child(self) -> Option<Self> {
        self.node().last_child().map(move |n| WidgetInfo::new(self.tree, n.id()))
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
        self.node().children().map(move |n| WidgetInfo::new(self.tree, n.id()))
    }

    /// Iterator over all widgets contained by this widget.
    #[inline]
    pub fn descendants(self) -> impl Iterator<Item = WidgetInfo<'a>> {
        //skip(1) due to ego_tree's descendants() including the node in the descendants
        self.node().descendants().skip(1).map(move |n| WidgetInfo::new(self.tree, n.id()))
    }

    /// Iterator over all widgets contained by this widget filtered by the `filter` closure.
    #[inline]
    pub fn filter_descendants<F: FnMut(WidgetInfo<'a>) -> DescendantFilter>(self, filter: F) -> FilterDescendants<'a, F> {
        let mut traverse = self.node().traverse();
        traverse.next(); // skip self.
        FilterDescendants {
            traverse,
            filter,
            tree: self.tree,
        }
    }

    /// Iterator over parent -> grandparent -> .. -> root.
    #[inline]
    pub fn ancestors(self) -> impl Iterator<Item = WidgetInfo<'a>> {
        self.node().ancestors().map(move |n| WidgetInfo::new(self.tree, n.id()))
    }

    /// Iterator over all previous widgets within the same parent.
    #[inline]
    pub fn prev_siblings(self) -> impl Iterator<Item = WidgetInfo<'a>> {
        self.node().prev_siblings().map(move |n| WidgetInfo::new(self.tree, n.id()))
    }

    /// Iterator over all next widgets within the same parent.
    #[inline]
    pub fn next_siblings(self) -> impl Iterator<Item = WidgetInfo<'a>> {
        self.node().next_siblings().map(move |n| WidgetInfo::new(self.tree, n.id()))
    }

    /// This widgets [`center`](Self::center) orientation in relation to a `origin`.
    #[inline]
    pub fn orientation_from(self, origin: PxPoint) -> WidgetOrientation {
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
    pub fn distance_key(self, origin: PxPoint) -> usize {
        let o = self.center();
        let a = (o.x - origin.x).0.pow(2);
        let b = (o.y - origin.y).0.pow(2);
        (a + b) as usize
    }

    fn closest_first(self, iter: impl Iterator<Item = WidgetInfo<'a>>) -> Vec<WidgetInfo<'a>> {
        let mut vec: Vec<_> = iter.collect();
        let origin = self.center();
        vec.sort_by_cached_key(|n| n.distance_key(origin));
        vec
    }
}

/// Widget tree filter result.
///
/// This `enum` is used by the [`filter_descendants`](WidgetInfo::filter_descendants) method on [`WidgetInfo`]. See its documentation for more.
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum DescendantFilter {
    /// Include the descendant and continue filtering its descendants.
    Include,
    /// Skip the descendant but continue filtering its descendants.
    Skip,
    /// Skip the descendant and its descendants.
    SkipAll,
    /// Include the descendant but skips its descendants.
    SkipDescendants,
}

/// An iterator that filters a widget tree.
///
/// This `struct` is created by the [`filter_descendants`](WidgetInfo::filter_descendants) method on [`WidgetInfo`]. See its documentation for more.
pub struct FilterDescendants<'a, F: FnMut(WidgetInfo<'a>) -> DescendantFilter> {
    traverse: ego_tree::iter::Traverse<'a, WidgetInfoInner>,
    filter: F,
    tree: &'a WidgetInfoTree,
}
impl<'a, F: FnMut(WidgetInfo<'a>) -> DescendantFilter> Iterator for FilterDescendants<'a, F> {
    type Item = WidgetInfo<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        use ego_tree::iter::Edge;
        #[allow(clippy::while_let_on_iterator)] // false positive https://github.com/rust-lang/rust-clippy/issues/7510
        while let Some(edge) = self.traverse.next() {
            if let Edge::Open(node) = edge {
                let widget = WidgetInfo::new(self.tree, node.id());
                match (self.filter)(widget) {
                    DescendantFilter::Include => return Some(widget),
                    DescendantFilter::Skip => continue,
                    DescendantFilter::SkipAll => {
                        for edge in &mut self.traverse {
                            if let Edge::Close(node2) = edge {
                                if node2 == node {
                                    break; // skip to close node.
                                }
                            }
                        }
                        continue;
                    }
                    DescendantFilter::SkipDescendants => {
                        for edge in &mut self.traverse {
                            if let Edge::Close(node2) = edge {
                                if node2 == node {
                                    break; // skip to close node.
                                }
                            }
                        }
                        return Some(widget);
                    }
                }
            }
        }
        None
    }
}

#[inline]
fn is_in_direction(direction: WidgetOrientation, origin: PxPoint, candidate: PxPoint) -> bool {
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

/// Orientation of a [`WidgetInfo`] relative to another point.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum WidgetOrientation {
    /// Widget is to the left of the reference point.
    Left,
    /// Widget is to the right of the reference point.
    Right,
    /// Widget is above the reference point.
    Above,
    /// Widget is below the reference point.
    Below,
}

/// Data from a previous [`WidgetInfoBuilder`], can be reused in the next rebuild for a performance boost.
pub struct UsedWidgetInfoBuilder {
    capacity: usize,
}

macro_rules! update_slot {
    ($(
        $(#[$meta:meta])*
        $vis:vis struct $Slot:ident -> $Mask:ident;
    )+) => {$(
        $(#[$meta])*
        ///
        /// This `struct` is a single byte that represents an index in the full bitmap.
        #[derive(Clone, Copy, Debug)]
        $vis struct $Slot(u8);

        impl $Slot {
            /// Gets a slot.
            #[inline]
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
            #[inline]
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
        #[derive(Clone, Copy, Default)]
        $vis struct $Mask([u128; 2]);

        impl $Mask {
            /// Gets a mask representing just the `slot`.
            pub fn from_slot(slot: $Slot) -> Self {
                let mut r = Self::none();
                r.insert(slot);
                r
            }

            /// Returns a mask that represents no update.
            #[inline]
            pub const fn none() -> Self {
                $Mask([0; 2])
            }

            /// Returns a mask that represents all updates.
            #[inline]
            pub const fn all() -> Self {
                $Mask([u128::MAX; 2])
            }

            /// Returns `true` if this mask does not represent any update.
            #[inline]
            pub fn is_none(&self) -> bool {
                self.0[0] == 0 && self.0[1] == 0
            }

            /// Flags the `slot` in this mask.
            #[inline]
            pub fn insert(&mut self, slot: $Slot) {
                let slot = slot.0;
                if slot < 128 {
                    self.0[0] |= 1 << slot;
                } else {
                    self.0[1] |= 1 << (slot - 128);
                }
            }

            /// Returns `true` if the `slot` is set in this mask.
            #[inline]
            pub fn contains(&self, slot: $Slot) -> bool {
                let slot = slot.0;
                if slot < 128 {
                    (self.0[0] & (1 << slot)) != 0
                } else {
                    (self.0[1] & (1 << (slot - 128))) != 0
                }
            }

            /// Flags all slots set in `other` in `self` as well.
            #[inline]
            pub fn extend(&mut self, other: &Self) {
                self.0[0] |= other.0[0];
                self.0[1] |= other.0[1];
            }

            /// Returns `true` if any slot is set in both `self` and `other`.
            #[inline]
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

/// Represents all event and update subscriptions of an widget.
///
/// Properties must register their interest in events and variables here otherwise a call to [`UiNode::event`] or
/// [`UiNode::update`] can end-up skipped due to optimizations.
///
/// [`UiNode::event`]: crate::UiNode::event
/// [`UiNode::update`]: crate::UiNode::update
#[derive(Debug, Default, Clone)]
pub struct WidgetSubscriptions {
    event: EventMask,
    update: UpdateMask,
}
impl WidgetSubscriptions {
    /// New default, no subscriptions.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an [`Event`] or command subscription.
    ///
    /// [`Event`]: crate::event::Event
    #[inline]
    pub fn event(&mut self, event: impl crate::event::Event) -> &mut Self {
        self.event.insert(event.slot());
        self
    }

    /// Register multiple event or command subscriptions.
    #[inline]
    pub fn events(&mut self, mask: &EventMask) -> &mut Self {
        self.event.extend(mask);
        self
    }

    /// Register async handler waker update source.
    #[inline]
    pub fn handler<A>(&mut self, handler: &impl WidgetHandler<A>) -> &mut Self
    where
        A: Clone + 'static,
    {
        handler.subscribe(self);
        self
    }

    /// Register a custom update source subscription.
    #[inline]
    pub fn update(&mut self, slot: UpdateSlot) -> &mut Self {
        self.update.insert(slot);
        self
    }

    /// Register multiple update source subscriptions.
    #[inline]
    pub fn updates(&mut self, mask: &UpdateMask) -> &mut Self {
        self.update.extend(mask);
        self
    }

    /// Register all subscriptions from `other` in `self`.
    #[inline]
    pub fn extend(&mut self, other: &WidgetSubscriptions) -> &mut Self {
        self.events(&other.event).updates(&other.update)
    }

    /// Register a variable subscription.
    #[inline]
    pub fn var<Vr, T>(&mut self, vars: &Vr, var: &impl Var<T>) -> &mut Self
    where
        Vr: WithVarsRead,
        T: VarValue,
    {
        self.update.extend(&var.update_mask(vars));
        self
    }

    /// Start a [`WidgetVarSubscriptions`] to register multiple variables without needing to reference the [`VarsRead`] for every variable.
    #[inline]
    pub fn vars<'s, 'v>(&'s mut self, vars: &'v impl AsRef<VarsRead>) -> WidgetVarSubscriptions<'v, 's> {
        WidgetVarSubscriptions {
            vars: vars.as_ref(),
            subscriptions: self,
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
    #[inline]
    pub fn event_mask(&self) -> EventMask {
        self.event
    }

    /// Returns the current set update subscriptions.
    #[inline]
    pub fn update_mask(&self) -> UpdateMask {
        self.update
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
    pub subscriptions: &'s mut WidgetSubscriptions,
}
impl<'v, 's> WidgetVarSubscriptions<'v, 's> {
    /// Register a variable subscriptions.
    #[inline]
    pub fn var<T: VarValue>(self, var: &impl Var<T>) -> Self {
        Self {
            subscriptions: self.subscriptions.var(self.vars, var),
            vars: self.vars,
        }
    }
}
