//! Widget info tree.

use std::{cell::Cell, fmt, mem, ops, rc::Rc};

use ego_tree::Tree;

use crate::{
    border::ContextBorders,
    context::{InfoContext, LayoutContext, OwnedStateMap, StateMap, Updates},
    crate_util::{IdMap, IdSet},
    event::EventUpdateArgs,
    handler::WidgetHandler,
    units::*,
    var::{Var, VarValue, VarsRead, WithVarsRead},
    widget_base::Visibility,
    window::WindowId,
    Widget, WidgetId,
};

unique_id_64! {
    /// Identifies a [`WidgetInfoTree`] snapshot, can be use for more speedy [`WidgetPath`] resolution.
    struct WidgetInfoTreeId;
}

/// Represents the in-progress layout pass for an widget tree.
pub struct WidgetLayout {
    t: WidgetLayoutTransform,
}
impl WidgetLayout {
    // # Requirements
    //
    // * Outer can be affected by parent widget only.
    // * Inner can be affected by widget only.
    // * Parent widget can pre-load child outer-transforms, applied when the outer-transform is visited.
    // * Parent widget can detect when they don't actually have a child, so they can simulate padding for child nodes.
    // * Parent panels can set the children outer transforms directly, in case they need to layout every child first to compute position.
    //
    // ## Nice to Have
    //
    // * Everything implemented in `implicit_base` for single child nodes, only panel implementers should have to learn
    //   the details of the layout pass.
    // * Update `WidgetLayoutInfo` as soon as possible, docs only promises up-to-date values after full layout, but
    //   we may find some use for partial values in the info references.
    //
    // ## Preview & Return
    //
    // * Like the event tracks, going down to leaf nodes we are in *preview*, returning up to root we are in *return*.
    // * Each node only affects its *inner* transform, so to affect transform in preview we need to *buffer* until the
    //   inner transform is visited, during return we can now know the *inner* info, so we can update it directly.
    // * Some transforms can only be applied on the *completed* transform, like the origin point, we *buffer* these until
    //   the *inner* transform goes fully out of scope at a parent scope.

    /// Defines the root widget outer-bounds scope.
    ///
    /// The default window implementation calls this.
    pub fn with_root_widget(ctx: &mut LayoutContext, layout: impl FnOnce(&mut LayoutContext, &mut Self) -> PxSize) -> PxSize {
        let mut wl = Self {
            t: WidgetLayoutTransform {
                transform_buf: RenderTransform::identity(),
                baseline: Px(0),
                known: None,
                known_target: KnownTarget::Outer,
                known_collapsed: false,
            },
        };
        wl.with_widget(ctx, layout)
    }

    /// Defines a widget outer-bounds scope, applies pending transforms to the outer transform,
    /// calls `layout`, then sets the transform target to the outer transform.
    ///
    /// The default widget constructor calls this, see [`implicit_base::nodes::widget`].
    ///
    /// [`implicit_base::nodes::widget`]: crate::widget_base::implicit_base::nodes::widget
    pub fn with_widget(&mut self, ctx: &mut LayoutContext, layout: impl FnOnce(&mut LayoutContext, &mut Self) -> PxSize) -> PxSize {
        self.known = None; // in case of WidgetList.
        self.baseline = Px(0);

        // drain preview transforms.
        ctx.widget_info.bounds.set_outer_transform(mem::take(&mut self.transform_buf));

        let size = layout(ctx, self);

        ctx.widget_info.bounds.set_outer_size(size);

        #[cfg(debug_assertions)]
        if !self.known_collapsed && (self.known.is_none() || !matches!(self.known_target, KnownTarget::Inner)) {
            tracing::error!("widget `{:?}` did not setup inner bounds during layout", ctx.path);
        }

        // setup returning transforms target.
        self.known = Some(ctx.widget_info.bounds.clone());
        self.known_target = KnownTarget::Outer;

        size
    }

    /// Defines a widget inner-bounds scope, applies pending transforms to the inner transform,
    /// calls `layout`, then sets the transform target to the inner transform.
    ///
    /// This method also updates the border info.
    ///
    /// The default widget borders constructor calls this, see [`implicit_base::nodes::inner`].
    ///
    /// [`implicit_base::nodes::inner`]: crate::widget_base::implicit_base::nodes::inner
    pub fn with_inner(&mut self, ctx: &mut LayoutContext, layout: impl FnOnce(&mut LayoutContext, &mut Self) -> PxSize) -> PxSize {
        #[cfg(debug_assertions)]
        if self.known.is_some() {
            tracing::error!("widget `{:?}` started inner bounds in the return path of another bounds", ctx.path)
        }
        self.known = None;

        // drain preview transforms.
        ctx.widget_info.bounds.set_inner_transform(mem::take(&mut self.transform_buf));
        ctx.widget_info.bounds.set_baseline(mem::take(&mut self.baseline));

        let size = ContextBorders::with_inner(ctx, |ctx| layout(ctx, self));

        ctx.widget_info.bounds.set_inner_size(size);

        // setup returning transforms target.
        self.known = Some(ctx.widget_info.bounds.clone());
        self.known_target = KnownTarget::Inner;

        size
    }

    /// Defines a widget child scope, drops the current layout target, calls `layout`, then if no transform targets where set
    /// by `layout` returns the size and if the [`child_in_inner_transform`] must be rendered.
    ///
    /// The default widget child layout constructor implements this, see [`implicit_base::nodes::child_layout`].
    ///
    /// [`implicit_base::nodes::child_layout`]: crate::widget_base::implicit_base::nodes::child_layout
    /// [`child_in_inner_transform`]: WidgetBoundsInfo::child_in_inner_transform
    pub fn with_child(&mut self, ctx: &mut LayoutContext, layout: impl FnOnce(&mut LayoutContext, &mut Self) -> PxSize) -> (PxSize, bool) {
        self.known = None; // in case of WidgetList?

        let size = layout(ctx, self);

        let collapse = mem::take(&mut self.known_collapsed);
        if self.known.is_none() && !collapse {
            ctx.widget_info.bounds.set_child_transform(mem::take(&mut self.transform_buf));
            ctx.widget_info.bounds.set_baseline(mem::take(&mut self.baseline));

            self.known = Some(ctx.widget_info.bounds.clone());
            self.known_target = KnownTarget::Child;

            (size, true)
        } else {
            (size, false)
        }
    }

    /// Defines a widget children scope, drops the current layout target, calls `layout`, then intercepts all transforms
    /// targeting the *child outer*, register it as the widget's child transform for the caller to render.
    ///
    /// The [`implicit_base::nodes::children_layout`] implements children bounds. Callers must render the [`child_in_inner_transform`].
    ///
    /// [`implicit_base::nodes::children_layout`]: crate::widget_base::implicit_base::nodes::children_layout
    /// [`child_in_inner_transform`]: WidgetBoundsInfo::child_in_inner_transform
    pub fn with_children(&mut self, ctx: &mut LayoutContext, layout: impl FnOnce(&mut LayoutContext, &mut Self) -> PxSize) -> PxSize {
        #[cfg(debug_assertions)]
        if self.known.is_some() {
            tracing::error!(
                "widget `{:?}` started children bounds in the return path of another bounds",
                ctx.path
            )
        }
        self.known = None;

        // drain preview transforms.
        ctx.widget_info.bounds.set_child_transform(mem::take(&mut self.transform_buf));

        let r = layout(ctx, self);

        // setup returning transforms target.
        self.known = Some(ctx.widget_info.bounds.clone());
        self.known_target = KnownTarget::Child;

        r
    }

    /// Overwrite the widget's outer transform, the `transform` closure is called with the
    /// [`WidgetLayoutTransform`] set to apply directly to the `widget` outer info, after it returns `self` has
    /// the same state it had before.
    ///
    /// This is a limited version of the [`with_child`] method, useful for cases where multiple children need
    /// to be layout first before each child's position can be computed, in these scenarios this method avoids a second
    /// layout pass by using the [`Widget`] trait to access and replace the outer transform.
    ///
    /// If `keep_previous` is `true` the new transform is *added* to the previous.
    ///
    /// [`with_child`]: Self::with_child
    pub fn with_outer<W: Widget, R>(
        &mut self,
        widget: &mut W,
        keep_previous: bool,
        transform: impl FnOnce(&mut WidgetLayoutTransform, &mut W) -> R,
    ) -> R {
        if !keep_previous {
            widget.bounds_info().set_outer_transform(RenderTransform::identity());
        }

        let mut wl = WidgetLayout {
            t: WidgetLayoutTransform {
                transform_buf: RenderTransform::identity(),
                baseline: Px(0),
                known: Some(widget.bounds_info().clone()),
                known_target: KnownTarget::Outer,
                known_collapsed: false,
            },
        };

        transform(&mut wl, widget)
    }

    /// Collapse the layout of `self` and descendants, the size is set to zero and the transform to identity.
    ///
    /// Nodes that set the visibility to the equivalent of [`Collapsed`] must skip layout and return [`PxSize::zero`] as
    /// the the size, ignoring the min-size constrains, and call this method to update all the descendant
    /// bounds information to be a zero-sized point at the current transform.
    ///
    /// [`Collapsed`]: Visibility::Collapsed
    pub fn collapse(&mut self, ctx: &mut LayoutContext) {
        self.known = None;
        self.known_collapsed = true;

        let widget_id = ctx.path.widget_id();
        if let Some(w) = ctx.info_tree.find(widget_id) {
            for w in w.self_and_descendants() {
                let info = w.info();
                info.bounds_info.set_outer_size(PxSize::zero());
                info.bounds_info.set_inner_size(PxSize::zero());
                info.bounds_info.set_baseline(Px(0));
                info.bounds_info.set_outer_transform(RenderTransform::identity());
                info.bounds_info.set_inner_transform(RenderTransform::identity());
                info.bounds_info.set_child_transform(RenderTransform::identity());
            }
        } else {
            tracing::error!("collapse did not find `{}` in the info tree", widget_id)
        }
    }
}
impl ops::Deref for WidgetLayout {
    type Target = WidgetLayoutTransform;

    fn deref(&self) -> &Self::Target {
        &self.t
    }
}
impl ops::DerefMut for WidgetLayout {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.t
    }
}

enum KnownTarget {
    Outer,
    Inner,
    Child,
}

/// Mutable access to the transform of a widget bounds in [`WidgetLayout`].
///
/// Note that [`WidgetLayout`] dereferences to this type.
pub struct WidgetLayoutTransform {
    transform_buf: RenderTransform,
    baseline: Px,

    known: Option<WidgetBoundsInfo>,
    known_target: KnownTarget,
    known_collapsed: bool,
}
impl WidgetLayoutTransform {
    /// Transforms the closest *inner* bounds.
    ///
    /// In the *preview* track, before the layout is delegated to a child node, the transform is buffered, as soon as the
    /// first child widget bounds is introduced the buffered transform is drained into it. In the *return* track, after the
    /// child layout, the child bounds reference is updated directly.
    pub fn transform(&mut self, transform: &RenderTransform) {
        self.transform_with(|t| *t = t.then(transform));
    }

    /// Insert the `transform` *before* the current transform.
    pub fn pre_transform(&mut self, transform: &RenderTransform) {
        self.transform_with(|t| *t = transform.then(t))
    }

    /// Adds the `offset` to the closest *inner* bounds transform.
    pub fn translate(&mut self, offset: PxVector) {
        self.transform_with(|t| *t = t.then_translate_px(offset));
    }

    /// Runs `op` on the transform of the closest *inner* bounds.
    pub fn transform_with(&mut self, op: impl FnOnce(&mut RenderTransform)) {
        if let Some(info) = &self.known {
            match self.known_target {
                KnownTarget::Outer => {
                    let mut transform = info.outer_transform();
                    op(&mut transform);
                    info.set_outer_transform(transform);
                }
                KnownTarget::Inner => {
                    let mut transform = info.inner_in_outer_transform();
                    op(&mut transform);
                    info.set_inner_transform(transform);
                }
                KnownTarget::Child => {
                    let mut transform = info.child_in_inner_transform();
                    op(&mut transform);
                    info.set_child_transform(transform);
                }
            }
        } else {
            op(&mut self.transform_buf);
        }
    }

    /// Set the baseline offset of the closest *inner* bounds. The offset is up from the bottom of the bounds.
    pub fn set_baseline(&mut self, baseline: Px) {
        if let Some(info) = &self.known {
            info.set_baseline(baseline);
        } else {
            self.baseline = baseline;
        }
    }

    /// Pre-translate the inner transform of the last visited widget by its baseline multiplied by `amount` on the *y* axis.
    ///
    /// Does nothing in the preview route.
    pub fn translate_baseline(&mut self, amount: f32) {
        if let Some(info) = &self.known {
            let baseline = info.baseline() * amount;
            if baseline != Px(0) {
                let inner_t = info.inner_in_outer_transform().pre_translate_px(PxVector::new(Px(0), baseline));
                info.set_inner_transform(inner_t);
            }
        }
    }
}

/// [`WidgetInfoTree`] builder.
pub struct WidgetInfoBuilder {
    window_id: WindowId,

    node: ego_tree::NodeId,
    widget_id: WidgetId,
    meta: OwnedStateMap,

    tree: Tree<WidgetInfoData>,
    interaction_filter: Vec<Box<dyn Fn(&InteractiveFilterArgs) -> bool>>,
}
impl WidgetInfoBuilder {
    /// Starts building a info tree with the root information.
    pub fn new(
        window_id: WindowId,
        root_id: WidgetId,
        root_bounds_info: WidgetBoundsInfo,
        root_border_info: WidgetBorderInfo,
        root_render_info: WidgetRenderInfo,
        used_data: Option<UsedWidgetInfoBuilder>,
    ) -> Self {
        debug_assert_eq!(RenderTransform::identity(), root_bounds_info.outer_transform());

        let used_data = used_data.unwrap_or_else(UsedWidgetInfoBuilder::fallback);
        let tree = Tree::with_capacity(
            WidgetInfoData {
                widget_id: root_id,
                bounds_info: root_bounds_info,
                border_info: root_border_info,
                render_info: root_render_info,
                meta: Rc::new(OwnedStateMap::new()),
            },
            used_data.tree_capacity,
        );

        let root_node = tree.root().id();
        WidgetInfoBuilder {
            window_id,
            node: root_node,
            tree,
            interaction_filter: Vec::with_capacity(used_data.interactive_capacity),
            meta: OwnedStateMap::new(),
            widget_id: root_id,
        }
    }

    fn node(&mut self, id: ego_tree::NodeId) -> ego_tree::NodeMut<WidgetInfoData> {
        self.tree.get_mut(id).unwrap()
    }

    /// Current widget id.
    pub fn widget_id(&self) -> WidgetId {
        self.widget_id
    }

    /// Current widget metadata.
    pub fn meta(&mut self) -> &mut StateMap {
        &mut self.meta.0
    }

    /// Calls `f` in a new widget context.
    ///
    /// Only call this in widget node implementations.
    pub fn push_widget(
        &mut self,
        id: WidgetId,
        bounds_info: WidgetBoundsInfo,
        border_info: WidgetBorderInfo,
        render_info: WidgetRenderInfo,
        f: impl FnOnce(&mut Self),
    ) {
        let parent_node = self.node;
        let parent_widget_id = self.widget_id;
        let parent_meta = mem::take(&mut self.meta);

        self.widget_id = id;
        self.node = self
            .node(parent_node)
            .append(WidgetInfoData {
                widget_id: id,
                bounds_info,
                border_info,
                render_info,
                meta: Rc::new(OwnedStateMap::new()),
            })
            .id();

        f(self);

        self.node(self.node).value().meta = Rc::new(mem::replace(&mut self.meta, parent_meta));
        self.node = parent_node;
        self.widget_id = parent_widget_id;
    }

    /// Reuse the widget info branch from the previous tree.
    ///
    /// Only call this in widget node implementations that monitor the updates requested by their content.
    pub fn push_widget_reuse(&mut self, ctx: &mut InfoContext) {
        let widget_id = ctx.path.widget_id();

        debug_assert_ne!(
            self.widget_id, widget_id,
            "can only call `push_widget` or `push_widget_reuse` for each widget"
        );

        let wgt = ctx
            .info_tree
            .find(widget_id)
            .unwrap_or_else(|| panic!("cannot reuse `{:?}`, not found in previous tree", ctx.path));

        Self::clone_append(wgt.node(), &mut self.node(self.node));
    }
    fn clone_append(from: ego_tree::NodeRef<WidgetInfoData>, to: &mut ego_tree::NodeMut<WidgetInfoData>) {
        let mut to = to.append(from.value().clone());
        for from in from.children() {
            Self::clone_append(from, &mut to);
        }
    }

    /// Register a closure that returns `true` if the widget is interactive or `false` if it is not.
    ///
    /// Widgets [`allow_interaction`] if all registered closures allow it.
    ///
    /// [`allow_interaction`]: WidgetInfo::allow_interaction
    pub fn push_interaction_filter(&mut self, filter: impl Fn(&InteractiveFilterArgs) -> bool + 'static) {
        self.interaction_filter.push(Box::new(filter))
    }

    /// Build the info tree.
    pub fn finalize(mut self) -> (WidgetInfoTree, UsedWidgetInfoBuilder) {
        self.tree.root_mut().value().meta = Rc::new(self.meta);
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

        let mut lookup = IdMap::default();
        let mut repeats = IdSet::default();

        lookup.reserve(self.tree.nodes().len());
        for (w, n) in valid_nodes.clone() {
            if lookup.insert(w, n).is_some() {
                repeats.insert(w);
            }
        }

        let r = WidgetInfoTree(Rc::new(WidgetInfoTreeInner {
            id: WidgetInfoTreeId::new_unique(),
            window_id: self.window_id,
            lookup,
            tree: self.tree,
            interaction_filter: self.interaction_filter,
        }));

        if !repeats.is_empty() {
            // Panic if widget ID is seen in more than one place. If we don't panic here we will
            // probably panic in the view-process due to spatial IDs generated from widget IDs.

            let mut places = String::new();
            for repeated in repeats {
                use std::fmt::Write;

                let _ = writeln!(&mut places);
                for w in r.all_widgets() {
                    if w.widget_id() == repeated {
                        let _ = writeln!(&mut places, "    {}", w.path());
                    }
                }
            }

            panic!("repeated widget ID in `{:?}`:\n{places}\n", self.window_id);
        }

        let cap = UsedWidgetInfoBuilder {
            tree_capacity: r.0.lookup.capacity(),
            interactive_capacity: r.0.interaction_filter.len(),
        };

        (r, cap)
    }
}

/// Bundle of widget info data from the current widget.
#[derive(Clone, Default)]
pub struct WidgetContextInfo {
    /// Bounds layout info.
    pub bounds: WidgetBoundsInfo,
    /// Border and corners info.
    pub border: WidgetBorderInfo,
    /// Render visibility info.
    pub render: WidgetRenderInfo,
}
impl WidgetContextInfo {
    /// New default.
    pub fn new() -> Self {
        Self::default()
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
    tree: Tree<WidgetInfoData>,
    lookup: IdMap<WidgetId, ego_tree::NodeId>,
    interaction_filter: Vec<Box<dyn Fn(&InteractiveFilterArgs) -> bool>>,
}
impl WidgetInfoTree {
    /// Blank window that contains only the root widget taking no space.
    pub fn blank(window_id: WindowId, root_id: WidgetId) -> Self {
        WidgetInfoBuilder::new(
            window_id,
            root_id,
            WidgetBoundsInfo::new(),
            WidgetBorderInfo::new(),
            WidgetRenderInfo::new(),
            None,
        )
        .finalize()
        .0
    }

    /// Reference to the root widget in the tree.
    pub fn root(&self) -> WidgetInfo {
        WidgetInfo::new(self, self.0.tree.root().id())
    }

    /// All widgets including `root`.
    pub fn all_widgets(&self) -> impl Iterator<Item = WidgetInfo> {
        self.0.tree.root().descendants().map(move |n| WidgetInfo::new(self, n.id()))
    }

    /// Id of the window that owns all widgets represented in the tree.
    pub fn window_id(&self) -> WindowId {
        self.0.window_id
    }

    /// Reference to the widget in the tree, if it is present.
    pub fn find(&self, widget_id: WidgetId) -> Option<WidgetInfo> {
        self.0
            .lookup
            .get(&widget_id)
            .and_then(|i| self.0.tree.get(*i).map(|n| WidgetInfo::new(self, n.id())))
    }

    /// If the tree contains the widget.
    pub fn contains(&self, widget_id: WidgetId) -> bool {
        self.0.lookup.contains_key(&widget_id)
    }

    /// Reference to the widget in the tree, if it is present.
    ///
    /// Faster then [`find`](Self::find) if the widget path was generated by `self`.
    pub fn get(&self, path: &WidgetPath) -> Option<WidgetInfo> {
        if let Some((tree_id, id)) = path.node_id {
            if tree_id == self.0.id {
                return self.0.tree.get(id).map(|n| WidgetInfo::new(self, n.id()));
            }
        }

        self.find(path.widget_id())
    }

    /// Reference to the widget or first parent that is present.
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
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    /// Widgets that contain [`widget_id`](WidgetPath::widget_id), root first.
    pub fn ancestors(&self) -> &[WidgetId] {
        &self.path[..self.path.len() - 1]
    }

    /// The widget.
    pub fn widget_id(&self) -> WidgetId {
        self.path[self.path.len() - 1]
    }

    /// [`ancestors`](WidgetPath::ancestors) and [`widget_id`](WidgetPath::widget_id), root first.
    pub fn widgets_path(&self) -> &[WidgetId] {
        &self.path[..]
    }

    /// If the `widget_id` is part of the path.
    pub fn contains(&self, widget_id: WidgetId) -> bool {
        self.path.iter().any(move |&w| w == widget_id)
    }

    /// Make a path to an ancestor id that is contained in the current path.
    pub fn ancestor_path(&self, ancestor_id: WidgetId) -> Option<WidgetPath> {
        self.path.iter().position(|&id| id == ancestor_id).map(|i| WidgetPath {
            node_id: None,
            window_id: self.window_id,
            path: self.path[..i].iter().copied().collect(),
        })
    }

    /// Get the inner most widget parent shared by both `self` and `other`.
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
    pub fn root_path(&self) -> WidgetPath {
        WidgetPath {
            node_id: None,
            window_id: self.window_id,
            path: Box::new([self.path[0]]),
        }
    }
}

#[derive(Default, Debug)]
struct WidgetBoundsData {
    outer_transform: Cell<RenderTransform>,
    inner_transform: Cell<RenderTransform>,
    child_transform: Cell<RenderTransform>,

    outer_size: Cell<PxSize>,
    inner_size: Cell<PxSize>,
    baseline: Cell<Px>,
}

/// Shared reference to the transforms and sizes of a [`WidgetInfo`].
#[derive(Default, Clone, Debug)]
pub struct WidgetBoundsInfo(Rc<WidgetBoundsData>);

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

    /// Gets the widget's outer bounds transform.
    ///
    /// The transform is relative to the parent widget's child transform.
    pub fn outer_transform(&self) -> RenderTransform {
        self.0.outer_transform.get()
    }

    /// Gets the widget's outer bounds size.
    pub fn outer_size(&self) -> PxSize {
        self.0.outer_size.get()
    }

    /// Gets the widget's inner bounds transform.
    ///
    /// The transform is relative to the parent widget's child transform, it includes the outer transform of this widget.
    pub fn inner_transform(&self) -> RenderTransform {
        self.inner_in_outer_transform().then(&self.outer_transform())
    }

    /// Gets the widget's *pure* inner transform.
    pub fn inner_in_outer_transform(&self) -> RenderTransform {
        self.0.inner_transform.get()
    }

    /// Gets the widget's inner bounds size.
    pub fn inner_size(&self) -> PxSize {
        self.0.inner_size.get()
    }

    /// Gets the baseline offset up from the inner bounds bottom line.
    pub fn baseline(&self) -> Px {
        self.0.baseline.get()
    }

    /// Gets the widget's child transform applied inside its inner bounds.
    ///
    /// Usually the child layout transforms is included on the outer transform of the child, but some parent widgets may render the
    /// transform themselves, if their *child* node is not a full widget or if they have more then one child.
    ///
    /// The transform is relative to the parent widget's child transform, it includes the inner and outer transforms of this widget.
    pub fn child_transform(&self) -> RenderTransform {
        self.child_in_inner_transform().then(&self.inner_transform())
    }

    /// Gets the widget's *pure* child transform.
    pub fn child_in_inner_transform(&self) -> RenderTransform {
        self.0.child_transform.get()
    }

    fn set_outer_transform(&self, transform: RenderTransform) {
        self.0.outer_transform.set(transform);
    }

    fn set_outer_size(&self, size: PxSize) {
        self.0.outer_size.set(size);
    }

    fn set_inner_transform(&self, transform: RenderTransform) {
        self.0.inner_transform.set(transform);
    }

    fn set_inner_size(&self, size: PxSize) {
        self.0.inner_size.set(size);
    }

    fn set_baseline(&self, baseline: Px) {
        self.0.baseline.set(baseline);
    }

    fn set_child_transform(&self, transform: RenderTransform) {
        self.0.child_transform.set(transform);
    }
}

/// Represents a computed transform of a child widget in a parent space.
#[derive(Debug, Clone, Copy)]
pub struct ParentChildTransform(pub RenderTransform);
impl ParentChildTransform {
    /// Transform a point from the parent space to the child space.
    pub fn point_in_child(&self, point_in_parent: PxPoint) -> PxPoint {
        self.0.inverse().unwrap().transform_px_point(point_in_parent).unwrap()
    }

    /// Transform a point from the child space to the parent space.
    pub fn point_in_parent(&self, point_in_child: PxPoint) -> PxPoint {
        self.0.transform_px_point(point_in_child).unwrap()
    }

    /// Transform a point from the parent space to the child space.
    pub fn vector_in_child(&self, vector_in_parent: PxVector) -> PxVector {
        self.0.inverse().unwrap().transform_px_vector(vector_in_parent)
    }

    /// Transform a point from the child space to the parent space.
    pub fn vector_in_parent(&self, vector_in_child: PxVector) -> PxVector {
        self.0.transform_px_vector(vector_in_child)
    }

    /// Returns a rectangle that encompasses the child `bounds` in the parent space.
    pub fn bounds_in_parent(&self, bounds: PxRect) -> PxRect {
        let rect = bounds.to_wr();
        let bounds = self.0.outer_transformed_box2d(&rect).unwrap();
        bounds.to_px()
    }
}
impl ops::Deref for ParentChildTransform {
    type Target = RenderTransform;

    fn deref(&self) -> &Self::Target {
        &self.0
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

    /// Compute the inner transform offset by the [`offsets`].
    ///
    /// [`offsets`]: Self::offsets
    pub fn inner_transform(&self, bounds: &WidgetBoundsInfo) -> RenderTransform {
        let o = self.offsets();
        let o = PxVector::new(o.left, o.top);
        bounds.inner_transform().pre_translate_px(o)
    }

    /// Compute the inner size offset by [`offsets`].
    ///
    /// [`offsets`]: Self::offsets
    pub fn inner_border_size(&self, bounds: &WidgetBoundsInfo) -> PxSize {
        let o = self.offsets();
        bounds.inner_size() - PxSize::new(o.horizontal(), o.vertical())
    }

    pub(super) fn set_offsets(&self, widths: PxSideOffsets) {
        self.0.offsets.set(widths);
    }

    pub(super) fn set_corner_radius(&self, radius: PxCornerRadius) {
        self.0.corner_radius.set(radius)
    }
}

#[derive(Default, Debug)]
struct WidgetRenderData {
    rendered: Cell<bool>,
}

/// Shared reference to the latest render information of a [`WidgetInfo`].
///
/// This status is updated every [`render`] without causing a tree rebuild.
///
/// [`render`]: crate::UiNode::render
#[derive(Default, Clone, Debug)]
pub struct WidgetRenderInfo(Rc<WidgetRenderData>);
impl WidgetRenderInfo {
    /// New default.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get if the widget or descendant widgets rendered in the latest window frame.
    pub fn rendered(&self) -> bool {
        self.0.rendered.get()
    }

    /// Set if the widget or child widgets rendered.
    pub(super) fn set_rendered(&self, rendered: bool) {
        self.0.rendered.set(rendered);
    }
}

#[derive(Clone)]
struct WidgetInfoData {
    widget_id: WidgetId,
    bounds_info: WidgetBoundsInfo,
    border_info: WidgetBorderInfo,
    render_info: WidgetRenderInfo,
    meta: Rc<OwnedStateMap>,
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
    fn new(tree: &'a WidgetInfoTree, node_id: ego_tree::NodeId) -> Self {
        Self { tree, node_id }
    }

    fn node(&self) -> ego_tree::NodeRef<'a, WidgetInfoData> {
        unsafe { self.tree.0.tree.get_unchecked(self.node_id) }
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
    pub fn rendered(self) -> bool {
        self.info().render_info.rendered()
    }

    /// Clone a reference to the widget latest render information.
    ///
    /// This information is up-to-date, it is updated every render without causing a tree rebuild.
    pub fn render_info(self) -> WidgetRenderInfo {
        self.info().render_info.clone()
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
        if self.rendered() {
            Visibility::Visible
        } else if self.info().bounds_info.outer_size() == PxSize::zero() {
            Visibility::Collapsed
        } else {
            Visibility::Hidden
        }
    }

    /// Returns `true` if interaction with this widget is allowed by all interactive filters.
    ///
    /// If `false` interaction behavior implementers must consider this widget *disabled*, disabled widgets do not receive keyboard
    /// or pointer events but can block hit-test if rendered above others.
    ///
    /// Note that not only [disabled] widgets can return `false` here, but only [disabled] widgets visually indicate that they are disabled.
    /// An example of a widget that is [enabled] but not interactive is one outside of a *modal overlay*.
    ///
    /// [disabled]: fn@crate::widget_base::enabled
    /// [enabled]: fn@crate::widget_base::enabled
    pub fn allow_interaction(self) -> bool {
        for filter in &self.tree.0.interaction_filter {
            if !filter(&InteractiveFilterArgs { info: self }) {
                return false;
            }
        }
        true
    }

    /// All the transforms introduced by this widget, starting from the outer info.
    ///
    /// This information is up-to-date, it is updated every layout without causing a tree rebuild.
    pub fn bounds_info(self) -> WidgetBoundsInfo {
        self.info().bounds_info.clone()
    }

    /// Clone a reference to the widget border and corner radius information.
    ///
    /// This information is up-to-date, it is updated every layout without causing a tree rebuild.
    pub fn border_info(self) -> WidgetBorderInfo {
        self.info().border_info.clone()
    }

    /// Side of the widget outer area, not transformed.
    ///
    /// Returns an up-to-date size, the size is updated every layout without causing a tree rebuild.
    pub fn outer_size(self) -> PxSize {
        self.info().bounds_info.outer_size()
    }

    /// Side of the widget inner area, not transformed.
    ///
    /// Returns an up-to-date size, the size is updated every layout without causing a tree rebuild.
    pub fn inner_size(self) -> PxSize {
        self.info().bounds_info.inner_size()
    }

    /// Gets the baseline offset up from the inner bounds bottom line.
    pub fn baseline(self) -> Px {
        self.info().bounds_info.baseline()
    }

    /// Widget outer transform in the parent space.
    ///
    /// Returns an up-to-date transform, the transform is updated every layout without causing a tree rebuild.
    pub fn outer_transform(self) -> RenderTransform {
        self.info().bounds_info.outer_transform()
    }

    /// Widget inner transform in the parent space.
    ///
    /// Returns an up-to-date transform, the transform is updated every layout without causing a tree rebuild.
    pub fn inner_transform(self) -> RenderTransform {
        self.info().bounds_info.inner_transform()
    }

    /// Widget child transform in the parent space.
    ///
    /// Transforms of children are in this space.
    ///
    /// Returns an up-to-date transform, the transform is updated every layout without causing a tree rebuild.
    pub fn child_transform(self) -> RenderTransform {
        self.info().bounds_info.child_transform()
    }

    /// Gets the parent space, if `parent_id` is one of the widget's ancestors. The widget's transforms
    /// are relative to this space, so the inner transform in the parent is `inner.then(&parent)`.
    ///
    /// Returns identity if the widget is the parent id.
    pub fn parent_transform(self, parent_id: WidgetId) -> Option<ParentChildTransform> {
        let mut t = RenderTransform::identity();

        if parent_id == self.widget_id() {
            return Some(ParentChildTransform(t));
        }

        for p in self.ancestors() {
            if p.widget_id() == parent_id {
                return Some(ParentChildTransform(t));
            }
            t = p.child_transform().then(&t);
        }
        None
    }

    /// Widget outer transform in the `parent_id` space.
    ///
    /// Returns `None` if `parent_id` is not found, returns identity transform if `parent_id` is the current widget.
    pub fn outer_transform_in(self, parent_id: WidgetId) -> Option<ParentChildTransform> {
        self.parent_transform(parent_id)
            .map(|t| ParentChildTransform(self.outer_transform().then(&t)))
    }

    /// Widget inner transform in the `parent_id` space.
    ///
    /// Returns `None` if `parent_id` is not found, if `parent_id` is the current widget returns the identity transform or the
    /// inner transform depending on the `parent_outer` flag.
    pub fn inner_transform_in(self, parent_id: WidgetId) -> Option<ParentChildTransform> {
        self.parent_transform(parent_id)
            .map(|t| ParentChildTransform(self.inner_transform().then(&t)))
    }

    /// Widget outer rectangle in the window space.
    ///
    /// Returns an up-to-date rect, the bounds are updated every layout without causing a tree rebuild.
    pub fn outer_bounds(self) -> PxRect {
        let bounds = PxRect::from_size(self.outer_size());
        self.outer_transform_in(self.root().widget_id()).unwrap().bounds_in_parent(bounds)
    }

    /// Widget inner rectangle in the window space.
    ///
    /// Returns an up-to-date rect, the bounds are updated every layout without causing a tree rebuild.
    pub fn inner_bounds(self) -> PxRect {
        let bounds = PxRect::from_size(self.inner_size());
        self.inner_transform_in(self.root().widget_id()).unwrap().bounds_in_parent(bounds)
    }

    /// Widget inner bounds center in the window space.
    pub fn center(self) -> PxPoint {
        self.inner_bounds().center()
    }

    /// Metadata associated with the widget during render.
    pub fn meta(self) -> &'a StateMap {
        &self.info().meta.0
    }

    /// Reference the [`WidgetInfoTree`] that owns `self`.
    pub fn tree(self) -> &'a WidgetInfoTree {
        self.tree
    }

    /// Reference to the root widget.
    pub fn root(self) -> Self {
        self.ancestors().last().unwrap_or(self)
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

    /// Iterator over the widgets directly contained by this widget.
    pub fn children(self) -> impl DoubleEndedIterator<Item = WidgetInfo<'a>> {
        self.node().children().map(move |n| WidgetInfo::new(self.tree, n.id()))
    }

    /// Iterator over all widgets contained by this widget.
    pub fn descendants(self) -> impl Iterator<Item = WidgetInfo<'a>> {
        //skip(1) due to ego_tree's descendants() including the node in the descendants
        self.node().descendants().skip(1).map(move |n| WidgetInfo::new(self.tree, n.id()))
    }

    /// iterator over the widget and all widgets contained by it.
    pub fn self_and_descendants(self) -> impl Iterator<Item = WidgetInfo<'a>> {
        self.node().descendants().map(move |n| WidgetInfo::new(self.tree, n.id()))
    }

    /// Iterator over all widgets contained by this widget filtered by the `filter` closure.
    pub fn filter_descendants<F>(self, filter: F) -> FilterDescendants<'a, F>
    where
        F: FnMut(WidgetInfo<'a>) -> DescendantFilter,
    {
        let mut traverse = self.node().traverse();
        traverse.next(); // skip self.
        FilterDescendants {
            traverse,
            filter,
            tree: self.tree,
        }
    }

    /// Iterator over parent -> grandparent -> .. -> root.
    pub fn ancestors(self) -> impl Iterator<Item = WidgetInfo<'a>> {
        self.node().ancestors().map(move |n| WidgetInfo::new(self.tree, n.id()))
    }

    /// Iterator over self -> parent -> grandparent -> .. -> root.
    pub fn self_and_ancestors(self) -> impl Iterator<Item = WidgetInfo<'a>> {
        [self].into_iter().chain(self.ancestors())
    }

    /// Iterator over all previous widgets within the same parent.
    pub fn prev_siblings(self) -> impl Iterator<Item = WidgetInfo<'a>> {
        self.node().prev_siblings().map(move |n| WidgetInfo::new(self.tree, n.id()))
    }

    /// Iterator over all next widgets within the same parent.
    pub fn next_siblings(self) -> impl Iterator<Item = WidgetInfo<'a>> {
        self.node().next_siblings().map(move |n| WidgetInfo::new(self.tree, n.id()))
    }

    /// This widgets [`center`](Self::center) orientation in relation to a `origin`.
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
    pub fn oriented_siblings(self) -> impl Iterator<Item = (WidgetInfo<'a>, WidgetOrientation)> {
        let c = self.center();
        self.siblings().map(move |s| (s, s.orientation_from(c)))
    }

    /// All parent children except this widget, sorted by closest first.
    pub fn closest_siblings(self) -> Vec<WidgetInfo<'a>> {
        self.closest_first(self.siblings())
    }

    /// All parent children except this widget, sorted by closest first and with orientation in
    /// relation to this widget center.
    pub fn closest_oriented_siblings(self) -> Vec<(WidgetInfo<'a>, WidgetOrientation)> {
        let mut vec: Vec<_> = self.oriented_siblings().collect();
        let origin = self.center();
        vec.sort_by_cached_key(|n| n.0.distance_key(origin));
        vec
    }

    /// Unordered siblings to the left of this widget.
    pub fn un_left_siblings(self) -> impl Iterator<Item = WidgetInfo<'a>> {
        self.oriented_siblings().filter_map(|(s, o)| match o {
            WidgetOrientation::Left => Some(s),
            _ => None,
        })
    }

    /// Unordered siblings to the right of this widget.
    pub fn un_right_siblings(self) -> impl Iterator<Item = WidgetInfo<'a>> {
        self.oriented_siblings().filter_map(|(s, o)| match o {
            WidgetOrientation::Right => Some(s),
            _ => None,
        })
    }

    /// Unordered siblings to the above of this widget.
    pub fn un_above_siblings(self) -> impl Iterator<Item = WidgetInfo<'a>> {
        self.oriented_siblings().filter_map(|(s, o)| match o {
            WidgetOrientation::Above => Some(s),
            _ => None,
        })
    }

    /// Unordered siblings to the below of this widget.
    pub fn un_below_siblings(self) -> impl Iterator<Item = WidgetInfo<'a>> {
        self.oriented_siblings().filter_map(|(s, o)| match o {
            WidgetOrientation::Below => Some(s),
            _ => None,
        })
    }

    /// Siblings to the left of this widget sorted by closest first.
    pub fn left_siblings(self) -> Vec<WidgetInfo<'a>> {
        self.closest_first(self.un_left_siblings())
    }

    /// Siblings to the right of this widget sorted by closest first.
    pub fn right_siblings(self) -> Vec<WidgetInfo<'a>> {
        self.closest_first(self.un_right_siblings())
    }

    /// Siblings to the above of this widget sorted by closest first.
    pub fn above_siblings(self) -> Vec<WidgetInfo<'a>> {
        self.closest_first(self.un_above_siblings())
    }

    /// Siblings to the below of this widget sorted by closest first.
    pub fn below_siblings(self) -> Vec<WidgetInfo<'a>> {
        self.closest_first(self.un_below_siblings())
    }

    /// Value that indicates the distance between this widget center
    /// and `origin`.
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
    traverse: ego_tree::iter::Traverse<'a, WidgetInfoData>,
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
    tree_capacity: usize,
    interactive_capacity: usize,
}
impl UsedWidgetInfoBuilder {
    fn fallback() -> Self {
        UsedWidgetInfoBuilder {
            tree_capacity: 100,
            interactive_capacity: 30,
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

/// Represents all event and update subscriptions of an widget.
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
    pub subscriptions: &'s mut WidgetSubscriptions,
}
impl<'v, 's> WidgetVarSubscriptions<'v, 's> {
    /// Register a variable subscriptions.
    pub fn var<T: VarValue>(self, var: &impl Var<T>) -> Self {
        Self {
            subscriptions: self.subscriptions.var(self.vars, var),
            vars: self.vars,
        }
    }
}

/// Argument for a interactive filter function.
///
/// See [WidgetInfoBuilder::push_interaction_filter].
#[derive(Debug)]
pub struct InteractiveFilterArgs<'a> {
    /// Widget being filtered.
    pub info: WidgetInfo<'a>,
}
impl<'a> InteractiveFilterArgs<'a> {
    /// New from `info`.
    pub fn new(info: WidgetInfo<'a>) -> Self {
        Self { info }
    }
}
