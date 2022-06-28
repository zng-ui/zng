use super::*;

/// Widget info tree builder.
///
/// See [`WidgetInfoTree`] for more details.
pub struct WidgetInfoBuilder {
    window_id: WindowId,

    node: ego_tree::NodeId,
    widget_id: WidgetId,
    meta: OwnedStateMap,

    tree: Tree<WidgetInfoData>,
    interactivity_filters: InteractivityFilters,
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
        let used_data = used_data.unwrap_or_else(UsedWidgetInfoBuilder::fallback);
        let tree = Tree::with_capacity(
            WidgetInfoData {
                widget_id: root_id,
                bounds_info: root_bounds_info,
                border_info: root_border_info,
                render_info: root_render_info,
                meta: Rc::new(OwnedStateMap::new()),
                interactivity_filters: vec![],
                interactivity_cache: Cell::new(None),
                local_interactivity: Cell::new(Interactivity::ENABLED),
            },
            used_data.tree_capacity,
        );

        let root_node = tree.root().id();
        WidgetInfoBuilder {
            window_id,
            node: root_node,
            tree,
            interactivity_filters: Vec::with_capacity(used_data.interactivity_filters_capacity),
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
                interactivity_filters: vec![],
                interactivity_cache: Cell::new(None),
                local_interactivity: Cell::new(Interactivity::ENABLED),
            })
            .id();

        f(self);

        self.node(self.node).value().meta = Rc::new(mem::replace(&mut self.meta, parent_meta));
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
    /// [interactivity filters]: Self::push_interactivity_filter
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

        Self::clone_append(
            wgt.node(),
            &mut self.tree.get_mut(self.node).unwrap(),
            &mut self.interactivity_filters,
        );
    }
    fn clone_append(
        from: ego_tree::NodeRef<WidgetInfoData>,
        to: &mut ego_tree::NodeMut<WidgetInfoData>,
        interactivity_filters: &mut InteractivityFilters,
    ) {
        let node = from.value().clone();
        node.interactivity_cache.set(None);
        let mut to = to.append(node);
        for filter in &from.value().interactivity_filters {
            interactivity_filters.push(filter.clone());
        }
        for from in from.children() {
            Self::clone_append(from, &mut to, interactivity_filters);
        }
    }

    /// Add the `interactivity` bits to the current widget's interactivity, it will affect the widget and all descendants.
    ///
    /// Also see [`push_interactivity_filter`] to affect the interactivity of widgets outside the current one.
    ///
    /// [`push_interactivity_filter`]: Self::push_interactivity_filter
    pub fn push_interactivity(&mut self, interactivity: Interactivity) {
        let mut node = self.node(self.node);
        let v = node.value();
        *v.local_interactivity.get_mut() |= interactivity;
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
    pub fn push_interactivity_filter(&mut self, filter: impl Fn(&InteractivityFilterArgs) -> Interactivity + 'static) {
        let filter = Rc::new(filter);
        self.interactivity_filters.push(filter.clone());
        self.node(self.node).value().interactivity_filters.push(filter);
    }

    /// Calls the `info` closure and returns the range of children visited by it.
    pub fn with_children_range(&mut self, info: impl FnOnce(&mut Self)) -> ops::Range<usize> {
        let before_count = self.tree.get(self.node).unwrap().children().count();
        info(self);
        before_count..self.tree.get(self.node).unwrap().children().count()
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
            interactivity_filters: self.interactivity_filters,
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
            interactivity_filters_capacity: r.0.interactivity_filters.len(),
        };

        (r, cap)
    }
}

/// Represents the in-progress layout pass for a widget tree.
pub struct WidgetLayout {
    t: WidgetLayoutTranslation,
    known_collapsed: bool,
    known_child_offset_changed: i32,
    child_offset_changed: i32,
}
impl WidgetLayout {
    // # Requirements
    //
    // * Outer can be affected by parent widget only.
    // * Inner can be affected by widget only.
    // * Parent widget can pre-load child outer-offsets, applied when the outer-bounds is visited.
    // * Parent widget can detect when they don't actually have a child, so they can simulate padding for child nodes.
    // * Parent panels can set the children outer-offsets directly, in case they need to layout every child first to compute position.
    //
    // ## Nice to Have
    //
    // * Everything implemented in `implicit_base` for single child nodes, only panel implementers should have to learn
    //   the details of the layout pass.
    //
    // ## Preview & Return
    //
    // * Like the event tracks, going down to leaf nodes we are in *preview*, returning up to root we are in *return*.
    // * Each node only affects its *inner* offset, so to affect the translate in preview we need to *buffer* until the
    //   inner bounds is visited, during return we can now know the *inner* info, so we can update it directly.

    /// Defines the root widget outer-bounds scope.
    ///
    /// The default window implementation calls this.
    pub fn with_root_widget(
        ctx: &mut LayoutContext,
        pass_id: LayoutPassId,
        layout: impl FnOnce(&mut LayoutContext, &mut Self) -> PxSize,
    ) -> PxSize {
        let mut wl = Self {
            t: WidgetLayoutTranslation {
                pass_id,
                offset_buf: PxVector::zero(),
                baseline: Px(0),
                offset_baseline: false,
                known: None,
                known_target: KnownTarget::Outer,
            },
            known_collapsed: false,
            known_child_offset_changed: 0,
            child_offset_changed: 0,
        };
        let size = wl.with_widget(ctx, false, layout);
        wl.finish_known();
        if wl.child_offset_changed > 0 {
            ctx.updates.render_update();
        }
        size
    }

    fn finish_known(&mut self) {
        if let Some(bounds) = self.known.take() {
            if let KnownTarget::Outer = self.known_target {
                self.child_offset_changed += bounds.end_pass();
                let childs_changed = mem::take(&mut self.known_child_offset_changed) > 0;
                if childs_changed {
                    self.child_offset_changed += 1;
                    bounds.set_changed_child();
                }
            }
        }
    }

    /// Defines a widget outer-bounds scope, applies pending translations to the outer offset,
    /// calls `layout`, then sets the translation target to the outer bounds.
    ///
    /// If `reuse` is `true` and none of the used metrics have changed skips calling `layout` and returns the current outer-size, the
    /// outer transform is still updated.
    ///
    /// The default widget constructor calls this, see [`implicit_base::nodes::widget`].
    ///
    /// [`implicit_base::nodes::widget`]: crate::widget_base::implicit_base::nodes::widget
    pub fn with_widget(
        &mut self,
        ctx: &mut LayoutContext,
        reuse: bool,
        layout: impl FnOnce(&mut LayoutContext, &mut Self) -> PxSize,
    ) -> PxSize {
        self.finish_known(); // in case of WidgetList.
        self.baseline = Px(0);
        self.offset_baseline = false;
        let parent_child_offset_changed = mem::take(&mut self.child_offset_changed);

        ctx.widget_info.bounds.begin_pass(self.pass_id); // record prev state

        // drain preview translations.
        ctx.widget_info.bounds.set_outer_offset(mem::take(&mut self.offset_buf));

        let snap = ctx.metrics.snapshot();
        let mut uses = ctx.widget_info.bounds.metrics_used();
        let size;

        if reuse && ctx.widget_info.bounds.metrics().map(|m| m.masked_eq(&snap, uses)).unwrap_or(false) {
            size = ctx.widget_info.bounds.outer_size();
        } else {
            let parent_uses = ctx.metrics.enter_widget_ctx();
            size = layout(ctx, self);
            uses = ctx.metrics.exit_widget_ctx(parent_uses);

            ctx.widget_info.bounds.set_outer_size(size);
        };
        ctx.widget_info.bounds.set_metrics(Some(snap), uses);

        // setup returning translations target.
        self.finish_known();
        self.known = Some(ctx.widget_info.bounds.clone());
        self.known_target = KnownTarget::Outer;
        self.known_child_offset_changed = self.child_offset_changed;

        self.child_offset_changed += parent_child_offset_changed; // when parent inner closes this the flag is for the parent not this

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
        self.finish_known();

        // drain preview translations.
        ctx.widget_info.bounds.set_inner_offset(mem::take(&mut self.offset_buf));
        ctx.widget_info.bounds.set_baseline(mem::take(&mut self.baseline));
        ctx.widget_info
            .bounds
            .set_inner_offset_baseline(mem::take(&mut self.offset_baseline));

        let size = ContextBorders::with_inner(ctx, |ctx| layout(ctx, self));

        ctx.widget_info.bounds.set_inner_size(size);

        // setup returning translations target.
        self.finish_known();
        self.known = Some(ctx.widget_info.bounds.clone());
        self.known_target = KnownTarget::Inner;

        size
    }

    /// Defines a widget child scope, drops the current layout target, calls `layout`, then returns the child size and
    /// `true` if there was no child widget inside `layout` and so the caller must render the [`child_offset`].
    ///
    /// If no inner [`Widget`] is found and the baseline is set during the call to `layout` the baseline is set to the current widget's inner bounds.
    ///
    /// The default widget child layout constructor implements this, see [`implicit_base::nodes::child_layout`].
    ///
    /// [`implicit_base::nodes::child_layout`]: crate::widget_base::implicit_base::nodes::child_layout
    /// [`child_offset`]: WidgetBoundsInfo::child_offset
    pub fn with_child(&mut self, ctx: &mut LayoutContext, layout: impl FnOnce(&mut LayoutContext, &mut Self) -> PxSize) -> (PxSize, bool) {
        self.finish_known(); // in case of WidgetList?

        let size = layout(ctx, self);

        let collapse = mem::take(&mut self.known_collapsed);
        if self.known.is_none() && !collapse {
            ctx.widget_info.bounds.set_child_offset(mem::take(&mut self.offset_buf));
            ctx.widget_info.bounds.set_baseline(mem::take(&mut self.baseline));
            ctx.widget_info
                .bounds
                .set_inner_offset_baseline(mem::take(&mut self.offset_baseline));

            // setup returning translations target.
            self.finish_known();
            self.known = Some(ctx.widget_info.bounds.clone());
            self.known_target = KnownTarget::Child;

            (size, true)
        } else {
            (size, false)
        }
    }

    /// Defines a widget children scope, drops the current layout target, calls `layout`, then intercepts all translations
    /// targeting the *child outer*, returns the panel node size.
    ///
    /// The caller must render the [`child_offset`].
    ///
    /// The [`implicit_base::nodes::children_layout`] implements children bounds
    ///
    /// [`implicit_base::nodes::children_layout`]: crate::widget_base::implicit_base::nodes::children_layout
    /// [`child_offset`]: WidgetBoundsInfo::child_offset
    pub fn with_children(&mut self, ctx: &mut LayoutContext, layout: impl FnOnce(&mut LayoutContext, &mut Self) -> PxSize) -> PxSize {
        #[cfg(debug_assertions)]
        if self.known.is_some() {
            tracing::error!(
                "widget `{:?}` started children bounds in the return path of another bounds",
                ctx.path
            )
        }
        self.finish_known();

        // drain preview translations.
        ctx.widget_info.bounds.set_child_offset(mem::take(&mut self.offset_buf));

        let r = layout(ctx, self);

        // setup returning translations target.
        self.finish_known();
        self.known = Some(ctx.widget_info.bounds.clone());
        self.known_target = KnownTarget::Child;

        r
    }

    /// Overwrite the widget's outer translate, the `translate` closure is called with the
    /// [`WidgetLayoutTranslation`] set to apply directly to the `widget` outer info, after it returns `self` has
    /// the same state it had before.
    ///
    /// This is a limited version of the [`with_child`] method, useful for cases where multiple children need
    /// to be layout first before each child's position can be computed, in these scenarios this method avoids a second
    /// layout pass by using the [`Widget`] trait to access and replace the outer transform.
    ///
    /// If `keep_previous` is `true` the new offset is *added* to the previous.
    ///
    /// [`with_child`]: Self::with_child
    pub fn with_outer<W: Widget, R>(
        &mut self,
        widget: &mut W,
        keep_previous: bool,
        translate: impl FnOnce(&mut WidgetLayoutTranslation, &mut W) -> R,
    ) -> R {
        self.with_outer_impl(widget.bounds_info().clone(), widget, keep_previous, translate)
    }

    /// Applies [`with_outer`] to the `node` if it is a full widget.
    ///
    /// Returns `Some(_)` if `translate` was called, or `None` if the `node` was not a full widget.
    ///
    /// [`with_outer`]: Self::with_outer
    pub fn try_with_outer<N: UiNode, R>(
        &mut self,
        node: &mut N,
        keep_previous: bool,
        translate: impl FnOnce(&mut WidgetLayoutTranslation, &mut N) -> R,
    ) -> Option<R> {
        node.try_bounds_info()
            .cloned()
            .map(|info| self.with_outer_impl(info, node, keep_previous, translate))
    }

    fn with_outer_impl<T, R>(
        &mut self,
        bounds: WidgetBoundsInfo,
        target: &mut T,
        keep_previous: bool,
        translate: impl FnOnce(&mut WidgetLayoutTranslation, &mut T) -> R,
    ) -> R {
        bounds.begin_pass(self.pass_id);

        if !keep_previous {
            bounds.set_outer_offset(PxVector::zero());
        }

        let mut wl = WidgetLayout {
            t: WidgetLayoutTranslation {
                pass_id: self.pass_id,
                offset_buf: PxVector::zero(),
                offset_baseline: false,
                baseline: Px(0),
                known: Some(bounds),
                known_target: KnownTarget::Outer,
            },
            known_collapsed: false,
            known_child_offset_changed: 0,
            child_offset_changed: 0,
        };

        let size = translate(&mut wl, target);

        self.child_offset_changed += wl.t.known.unwrap().end_pass();

        size
    }

    /// Collapse the layout of `self` and descendants, the size and offsets are set to zero.
    ///
    /// Nodes that set the visibility to the equivalent of [`Collapsed`] must skip layout and return [`PxSize::zero`] as
    /// the the size, ignoring the min-size constrains, and call this method to update all the descendant
    /// bounds information to be a zero-sized point.
    ///
    /// [`Collapsed`]: Visibility::Collapsed
    pub fn collapse(&mut self, ctx: &mut LayoutContext) {
        self.finish_known();
        self.known_collapsed = true;

        let widget_id = ctx.path.widget_id();
        if let Some(w) = ctx.info_tree.find(widget_id) {
            for w in w.self_and_descendants() {
                let info = w.info();
                info.bounds_info.set_outer_size(PxSize::zero());
                info.bounds_info.set_inner_size(PxSize::zero());
                info.bounds_info.set_baseline(Px(0));
                info.bounds_info.set_inner_offset_baseline(false);
                info.bounds_info.set_outer_offset(PxVector::zero());
                info.bounds_info.set_inner_offset(PxVector::zero());
                info.bounds_info.set_child_offset(PxVector::zero());
                info.bounds_info.set_measure_metrics(None, LayoutMask::NONE);
                info.bounds_info.set_metrics(None, LayoutMask::NONE);
            }
        } else {
            tracing::error!("collapse did not find `{}` in the info tree", widget_id)
        }
    }

    /// Collapse layout of all descendants, the size and offsets are set to zero.
    ///
    /// Widgets that control the visibility of their children can use this method and then, in the same layout pass, layout
    /// the children that should be visible.
    pub fn collapse_descendants(&mut self, ctx: &mut LayoutContext) {
        let widget_id = ctx.path.widget_id();
        if let Some(w) = ctx.info_tree.find(widget_id) {
            for w in w.descendants() {
                let info = w.info();
                info.bounds_info.set_outer_size(PxSize::zero());
                info.bounds_info.set_inner_size(PxSize::zero());
                info.bounds_info.set_baseline(Px(0));
                info.bounds_info.set_inner_offset_baseline(false);
                info.bounds_info.set_outer_offset(PxVector::zero());
                info.bounds_info.set_inner_offset(PxVector::zero());
                info.bounds_info.set_child_offset(PxVector::zero());
                info.bounds_info.set_measure_metrics(None, LayoutMask::NONE);
                info.bounds_info.set_metrics(None, LayoutMask::NONE);
            }
        } else {
            tracing::error!("collapse_descendants did not find `{}` in the info tree", widget_id)
        }
    }
}
impl ops::Deref for WidgetLayout {
    type Target = WidgetLayoutTranslation;

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

/// Identifies the layout pass of a window.
///
/// This value is different for each window layout, but the same for children of panels that do more then one layout pass.
pub type LayoutPassId = u32;

/// Mutable access to the offset of a widget bounds in [`WidgetLayout`].
///
/// Note that [`WidgetLayout`] dereferences to this type.
pub struct WidgetLayoutTranslation {
    pass_id: LayoutPassId,
    offset_buf: PxVector,
    baseline: Px,
    offset_baseline: bool,

    known: Option<WidgetBoundsInfo>,
    known_target: KnownTarget,
}
impl WidgetLayoutTranslation {
    /// Gets the current window layout pass.
    ///
    /// Widgets can be layout more then once per window layout pass, you can use this ID to identify such cases.
    pub fn pass_id(&self) -> LayoutPassId {
        self.pass_id
    }

    /// Adds the `offset` to the closest *inner* bounds offset.
    pub fn translate(&mut self, offset: PxVector) {
        if let Some(info) = &self.known {
            match self.known_target {
                KnownTarget::Outer => {
                    let mut o = info.outer_offset();
                    o += offset;
                    info.set_outer_offset(o);
                }
                KnownTarget::Inner => {
                    let mut o = info.inner_offset();
                    o += offset;
                    info.set_inner_offset(o);
                }
                KnownTarget::Child => {
                    let mut o = info.child_offset();
                    o += offset;
                    info.set_child_offset(o);
                }
            }
        } else {
            self.offset_buf += offset;
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

    /// If the inner offset of the last visited widget is added by its baseline on the *y* axis.
    pub fn translate_baseline(&mut self, enabled: bool) {
        if let Some(info) = &self.known {
            info.set_inner_offset_baseline(enabled);
        } else {
            self.offset_baseline = enabled;
        }
    }
}
