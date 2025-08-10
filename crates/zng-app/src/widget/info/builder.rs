use parking_lot::Mutex;
use zng_layout::{
    context::{InlineConstraints, InlineConstraintsLayout, InlineConstraintsMeasure, InlineSegment, InlineSegmentPos, LAYOUT, LayoutMask},
    unit::{Factor, Px, PxBox, PxPoint, PxRect, PxSize, PxVector},
};
use zng_state_map::{OwnedStateMap, StateId, StateMapMut, StateValue};
use zng_unique_id::{IdMap, IdSet};

use crate::{
    DInstant, INSTANT,
    render::TransformStyle,
    update::{InfoUpdates, LayoutUpdates, UpdateFlags},
    widget::{WIDGET, WidgetId, WidgetUpdateMode, border::BORDER, node::UiNode},
    window::{WINDOW, WindowId},
};

use super::{hit::ParallelSegmentOffsets, *};

/// Tag for the [`WidgetInfo::meta`] state-map.
pub enum WidgetInfoMeta {}

/// Widget info tree builder.
///
/// See [`WidgetInfoTree`] for more details.
pub struct WidgetInfoBuilder {
    info_widgets: Arc<InfoUpdates>,
    window_id: WindowId,
    pub(super) access_enabled: access::AccessEnabled,
    started_access: bool,

    node: tree::NodeId,
    widget_id: WidgetId,
    meta: Arc<Mutex<OwnedStateMap<WidgetInfoMeta>>>,

    tree: Tree<WidgetInfoData>,
    interactivity_filters: InteractivityFilters,

    scale_factor: Factor,

    build_meta: Arc<Mutex<OwnedStateMap<WidgetInfoMeta>>>,

    build_start: DInstant,
    pushed_widgets: u32,
}
impl WidgetInfoBuilder {
    /// Starts building a info tree with the root information.
    pub fn new(
        info_widgets: Arc<InfoUpdates>,
        window_id: WindowId,
        access_enabled: access::AccessEnabled,
        root_id: WidgetId,
        root_bounds_info: WidgetBoundsInfo,
        root_border_info: WidgetBorderInfo,
        scale_factor: Factor,
    ) -> Self {
        let tree = Tree::new(WidgetInfoData {
            id: root_id,
            is_reused: false,
            bounds_info: root_bounds_info,
            border_info: root_border_info,
            meta: Arc::new(OwnedStateMap::new()),
            interactivity_filters: vec![],
            local_interactivity: Interactivity::ENABLED,
            cache: Mutex::new(WidgetInfoCache { interactivity: None }),
        });
        let mut lookup = IdMap::default();
        let root_node = tree.root().id();
        lookup.insert(root_id, root_node);

        let mut builder = WidgetInfoBuilder {
            info_widgets,
            window_id,
            access_enabled,
            started_access: access_enabled.is_enabled() && WINDOW.info().access_enabled().is_disabled(),
            node: root_node,
            tree,
            interactivity_filters: vec![],
            meta: Arc::default(),
            widget_id: root_id,
            scale_factor,
            build_meta: Arc::default(),
            build_start: INSTANT.now(),
            pushed_widgets: 1, // root is always new.
        };

        if let Some(mut b) = builder.access() {
            b.set_role(super::access::AccessRole::Application);
        }

        builder
    }

    fn node(&mut self, id: tree::NodeId) -> tree::NodeMut<'_, WidgetInfoData> {
        self.tree.index_mut(id)
    }

    /// Current widget id.
    pub fn widget_id(&self) -> WidgetId {
        self.widget_id
    }

    /// Widget info tree build metadata.
    ///
    /// This metadata can be modified only by pushed widgets, **not** by the reused widgets.
    pub fn with_build_meta<R>(&mut self, visitor: impl FnOnce(StateMapMut<WidgetInfoMeta>) -> R) -> R {
        visitor(self.build_meta.lock().borrow_mut())
    }
    /// Set the info tree build metadata `id` to `value`.
    pub fn set_build_meta<T: StateValue>(&mut self, id: impl Into<StateId<T>>, value: impl Into<T>) {
        let id = id.into();
        let value = value.into();
        self.with_build_meta(|mut s| s.set(id, value));
    }
    /// Sets the info tree build metadata `id` without value.
    pub fn flag_build_meta(&mut self, id: impl Into<StateId<()>>) {
        let id = id.into();
        self.with_build_meta(|mut s| s.flag(id));
    }

    /// Current widget info metadata.
    pub fn with_meta<R>(&mut self, visitor: impl FnOnce(StateMapMut<WidgetInfoMeta>) -> R) -> R {
        visitor(self.meta.lock().borrow_mut())
    }
    /// Set the widget info metadata `id` to `value`.
    ///
    /// Returns the previous set value.
    pub fn set_meta<T: StateValue>(&mut self, id: impl Into<StateId<T>>, value: impl Into<T>) {
        let id = id.into();
        let value = value.into();
        self.with_meta(|mut s| s.set(id, value));
    }
    /// Sets the widget info metadata `id` without value.
    pub fn flag_meta(&mut self, id: impl Into<StateId<()>>) {
        let id = id.into();
        self.with_meta(|mut s| s.flag(id));
    }

    /// Calls `f` to build the context widget info.
    ///
    /// Note that `f` is only called if the widget info cannot be reused.
    pub fn push_widget(&mut self, f: impl FnOnce(&mut Self)) {
        let id = WIDGET.id();
        if !WIDGET.take_update(UpdateFlags::INFO) && !self.info_widgets.delivery_list().enter_widget(id) && !self.started_access {
            // reuse
            let tree = WINDOW.info();
            if let Some(wgt) = tree.get(id) {
                self.tree.index_mut(self.node).push_reuse(wgt.node(), &mut |old_data| {
                    let mut r = old_data.clone();
                    r.is_reused = true;
                    r.cache.get_mut().interactivity = None;
                    for filter in &r.interactivity_filters {
                        self.interactivity_filters.push(filter.clone());
                    }
                    r
                });
                return;
            }
        }

        let parent_node = self.node;
        let parent_widget_id = self.widget_id;
        let parent_meta = mem::take(&mut self.meta);

        let bounds_info = WIDGET.bounds();
        let border_info = WIDGET.border();

        self.widget_id = id;
        self.node = self
            .node(parent_node)
            .push_child(WidgetInfoData {
                id,
                is_reused: false,
                bounds_info,
                border_info,
                meta: Arc::new(OwnedStateMap::new()),
                interactivity_filters: vec![],
                local_interactivity: Interactivity::ENABLED,
                cache: Mutex::new(WidgetInfoCache { interactivity: None }),
            })
            .id();

        self.pushed_widgets += 1;

        f(self);

        let meta = mem::replace(&mut self.meta, parent_meta);
        let mut node = self.node(self.node);
        node.value().meta = Arc::new(Arc::try_unwrap(meta).unwrap().into_inner());
        node.close();

        self.node = parent_node;
        self.widget_id = parent_widget_id;
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
    /// widget tree, and are re-registered for the tree if the current widget is reused.
    ///
    /// Note that the filter can make the assumption that parent widgets affect all descendants and if the filter is intended to
    /// affect only the current widget and descendants you can use [`push_interactivity`] instead.
    ///
    /// [`interactivity`]: WidgetInfo::interactivity
    /// [`push_interactivity`]: Self::push_interactivity
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

    /// Create a new info builder that can be built in parallel and merged back onto this one using [`parallel_fold`].
    ///
    /// [`parallel_fold`]: Self::parallel_fold
    /// [`push_widget`]: Self::push_widget
    pub fn parallel_split(&self) -> ParallelBuilder<Self> {
        let node = self.tree.index(self.node).value();
        let tree = Tree::new(WidgetInfoData {
            id: node.id,
            is_reused: node.is_reused,
            bounds_info: node.bounds_info.clone(),
            border_info: node.border_info.clone(),
            meta: node.meta.clone(),
            interactivity_filters: vec![],
            local_interactivity: node.local_interactivity,
            cache: Mutex::new(WidgetInfoCache { interactivity: None }),
        });
        ParallelBuilder(Some(Self {
            info_widgets: self.info_widgets.clone(),
            window_id: self.window_id,
            access_enabled: self.access_enabled,
            started_access: self.started_access,
            widget_id: self.widget_id,
            meta: self.meta.clone(),
            node: tree.root().id(),
            tree,
            interactivity_filters: vec![],
            scale_factor: self.scale_factor,
            build_meta: self.build_meta.clone(),
            build_start: self.build_start,
            pushed_widgets: 0,
        }))
    }

    /// Collect info from `split` into `self`.
    pub fn parallel_fold(&mut self, mut split: ParallelBuilder<Self>) {
        let mut split = split.take();

        self.interactivity_filters.append(&mut split.interactivity_filters);
        self.pushed_widgets += split.pushed_widgets;
        {
            debug_assert!(Arc::ptr_eq(&self.meta, &split.meta));

            let mut split_node = split.tree.root_mut();
            let mut node = self.node(self.node);
            let split_node = split_node.value();
            let node = node.value();

            node.interactivity_filters.append(&mut split_node.interactivity_filters);
            node.local_interactivity |= split_node.local_interactivity;
        }

        self.tree.index_mut(self.node).parallel_fold(split.tree, &mut |d| WidgetInfoData {
            id: d.id,
            is_reused: d.is_reused,
            bounds_info: d.bounds_info.clone(),
            border_info: d.border_info.clone(),
            meta: d.meta.clone(),
            interactivity_filters: mem::take(&mut d.interactivity_filters),
            local_interactivity: d.local_interactivity,
            cache: Mutex::new(d.cache.get_mut().clone()),
        });
    }

    /// Build the info tree.
    ///
    /// Also notifies [`WIDGET_INFO_CHANGED_EVENT`] and [`INTERACTIVITY_CHANGED_EVENT`] if `notify` is true.
    pub fn finalize(mut self, previous_tree: Option<WidgetInfoTree>, notify: bool) -> WidgetInfoTree {
        let mut node = self.tree.root_mut();
        let meta = Arc::new(Arc::try_unwrap(self.meta).unwrap().into_inner());
        node.value().meta = meta;
        node.close();

        let generation;
        let widget_count_offsets;
        let spatial_bounds;
        let transform_changed_subs;
        let visibility_changed_subs;

        if let Some(t) = &previous_tree {
            let t = t.0.frame.read();
            generation = t.stats.generation.wrapping_add(1);
            widget_count_offsets = t.widget_count_offsets.clone();
            spatial_bounds = t.spatial_bounds;
            transform_changed_subs = t.transform_changed_subs.clone();
            visibility_changed_subs = t.visibility_changed_subs.clone();
        } else {
            generation = 0;
            widget_count_offsets = ParallelSegmentOffsets::default();
            spatial_bounds = PxBox::zero();
            transform_changed_subs = IdMap::new();
            visibility_changed_subs = IdMap::new();
        }

        let mut lookup = IdMap::new();
        lookup.reserve(self.tree.len());
        let mut out_of_bounds = vec![];

        for (id, data) in self.tree.iter() {
            if lookup.insert(data.id, id).is_some() {
                tracing::error!("widget `{}` repeated in info tree", data.id);
            }
            if data.bounds_info.is_actually_out_of_bounds() {
                out_of_bounds.push(id);
            }
        }
        out_of_bounds.shrink_to_fit();

        let tree = WidgetInfoTree(Arc::new(WidgetInfoTreeInner {
            window_id: self.window_id,
            access_enabled: self.access_enabled,
            lookup,
            interactivity_filters: self.interactivity_filters,
            build_meta: Arc::new(mem::take(&mut self.build_meta.lock())),

            frame: RwLock::new(WidgetInfoTreeFrame {
                stats: WidgetInfoTreeStats::new(self.build_start, self.tree.len() as u32 - self.pushed_widgets, generation),
                stats_update: Default::default(),
                out_of_bounds: Arc::new(out_of_bounds),
                out_of_bounds_update: Default::default(),
                scale_factor: self.scale_factor,
                spatial_bounds,
                widget_count_offsets,
                transform_changed_subs,
                visibility_changed_subs,
                view_process_gen: ViewProcessGen::INVALID,
            }),

            tree: self.tree,
        }));

        if notify {
            let prev_tree = previous_tree.unwrap_or_else(|| WidgetInfoTree::wgt(tree.window_id(), tree.root().id()));
            let args = WidgetInfoChangedArgs::now(tree.window_id(), prev_tree.clone(), tree.clone());
            WIDGET_INFO_CHANGED_EVENT.notify(args);

            let mut targets = IdSet::default();
            INTERACTIVITY_CHANGED_EVENT.visit_subscribers::<()>(|wid| {
                if let Some(wgt) = tree.get(wid) {
                    let prev = prev_tree.get(wid).map(|w| w.interactivity());
                    let new_int = wgt.interactivity();
                    if prev != Some(new_int) {
                        targets.insert(wid);
                    }
                }
                ops::ControlFlow::Continue(())
            });
            if !targets.is_empty() {
                let args = InteractivityChangedArgs::now(prev_tree, tree.clone(), targets);
                INTERACTIVITY_CHANGED_EVENT.notify(args);
            }
        }

        tree
    }
}

crate::event::event! {
    /// A window widget tree was rebuild.
    pub static WIDGET_INFO_CHANGED_EVENT: WidgetInfoChangedArgs;

    /// Widget interactivity has changed after an info update.
    ///
    /// All subscribers of this event are checked after info rebuild, if the interactivity changes from the previous tree
    /// the event notifies.
    ///
    /// The event only notifies if the widget is present in the new info tree.
    pub static INTERACTIVITY_CHANGED_EVENT: InteractivityChangedArgs;

    /// Widget visibility has changed after render.
    ///
    /// All subscribers of this event are checked after render, if the previous visibility was recorded and
    /// the new visibility is different an event is sent to the widget.
    pub static VISIBILITY_CHANGED_EVENT: VisibilityChangedArgs;

    /// A widget global inner transform has changed after render.
    ///
    /// All subscribers of this event are checked after render, if the previous inner transform was recorded and
    /// the new inner transform is different an event is sent to the widget.
    pub static TRANSFORM_CHANGED_EVENT: TransformChangedArgs;
}

crate::event::event_args! {
    /// [`WIDGET_INFO_CHANGED_EVENT`] args.
    pub struct WidgetInfoChangedArgs {
        /// Window ID.
        pub window_id: WindowId,

        /// Previous widget tree.
        ///
        /// This is an empty tree before the first tree build.
        pub prev_tree: WidgetInfoTree,

        /// New widget tree.
        pub tree: WidgetInfoTree,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// [`TRANSFORM_CHANGED_EVENT`] args.
    pub struct TransformChangedArgs {
        /// Widget tree where some widgets have new inner transforms.
        pub tree: WidgetInfoTree,

        /// All event subscribers that changed inner-transform mapped to the previous inner-transform.
        pub changed: IdMap<WidgetId, PxTransform>,

        ..

        /// Target the `changed` widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            for id in self.changed.keys() {
                if let Some(wgt) = self.tree.get(*id) {
                    list.insert_wgt(&wgt);
                }
            }
        }
    }

    /// [`VISIBILITY_CHANGED_EVENT`] args.
    pub struct VisibilityChangedArgs {
        /// Widget tree where some widgets have new visibility.
        pub tree: WidgetInfoTree,

        /// All event subscribers that changed visibility mapped to the previous visibility.
        pub changed: IdMap<WidgetId, Visibility>,

        ..

        /// Target the `changed` widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            for id in self.changed.keys() {
                if let Some(wgt) = self.tree.get(*id) {
                    list.insert_wgt(&wgt);
                }
            }
        }
    }

    /// [`INTERACTIVITY_CHANGED_EVENT`] args.
    pub struct InteractivityChangedArgs {
        /// Previous tree with old interactivity values.
        pub prev_tree: WidgetInfoTree,

        /// New tree with new interactivity values.
        pub tree: WidgetInfoTree,

        /// All event subscribers that changed interactivity in this info update.
        pub changed: IdSet<WidgetId>,

        ..

        /// Target the `changed` widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            for id in self.changed.iter() {
                if let Some(wgt) = self.tree.get(*id) {
                    list.insert_wgt(&wgt);
                }
            }
        }
    }
}
impl TransformChangedArgs {
    /// Gets the previous and new inner transform of the widget.
    pub fn change(&self, id: WidgetId) -> Option<(PxTransform, PxTransform)> {
        let prev = *self.changed.get(&id)?;
        let new = self.tree.get(id)?.inner_transform();
        Some((prev, new))
    }

    /// Gets the movement between previous and new transformed top-left corner.
    pub fn offset(&self, id: WidgetId) -> Option<PxVector> {
        let (prev, new) = self.change(id)?;

        let prev = prev.transform_point(PxPoint::zero()).unwrap_or_default();
        let new = new.transform_point(PxPoint::zero()).unwrap_or_default();
        Some(prev - new)
    }
}
impl InteractivityChangedArgs {
    /// Previous interactivity of this widget.
    ///
    /// Returns `None` if the widget was not in the previous info tree.
    pub fn prev_interactivity(&self, widget_id: WidgetId) -> Option<Interactivity> {
        self.prev_tree.get(widget_id).map(|w| w.interactivity())
    }

    /// New interactivity of the widget.
    ///
    /// # Panics
    ///
    /// Panics if `widget_id` is not in [`tree`]. This method must be called only for [`changed`].
    ///
    /// [`tree`]: Self::tree
    /// [`changed`]: Self::changed
    pub fn new_interactivity(&self, widget_id: WidgetId) -> Interactivity {
        if let Some(w) = self.tree.get(widget_id) {
            w.interactivity()
        } else if self.changed.contains(&widget_id) {
            panic!("widget {widget_id} was in targets and not in new tree, invalid args");
        } else {
            panic!("widget {widget_id} is not in targets");
        }
    }

    /// Widget was disabled or did not exist, now is enabled.
    pub fn is_enable(&self, widget_id: WidgetId) -> bool {
        self.prev_interactivity(widget_id).unwrap_or(Interactivity::DISABLED).is_disabled()
            && self.new_interactivity(widget_id).is_enabled()
    }

    /// Widget was enabled or did not exist, now is disabled.
    pub fn is_disable(&self, widget_id: WidgetId) -> bool {
        self.prev_interactivity(widget_id).unwrap_or(Interactivity::ENABLED).is_enabled() && self.new_interactivity(widget_id).is_disabled()
    }

    /// Widget was blocked or did not exist, now is unblocked.
    pub fn is_unblock(&self, widget_id: WidgetId) -> bool {
        self.prev_interactivity(widget_id).unwrap_or(Interactivity::BLOCKED).is_blocked() && !self.new_interactivity(widget_id).is_blocked()
    }

    /// Widget was unblocked or did not exist, now is blocked.
    pub fn is_block(&self, widget_id: WidgetId) -> bool {
        !self.prev_interactivity(widget_id).unwrap_or(Interactivity::BLOCKED).is_blocked() && self.new_interactivity(widget_id).is_blocked()
    }

    /// Widget was visually disabled or did not exist, now is visually enabled.
    pub fn is_vis_enable(&self, widget_id: WidgetId) -> bool {
        self.prev_interactivity(widget_id)
            .unwrap_or(Interactivity::DISABLED)
            .is_vis_disabled()
            && self.new_interactivity(widget_id).is_vis_enabled()
    }

    /// Widget was visually enabled or did not exist, now is visually disabled.
    pub fn is_vis_disable(&self, widget_id: WidgetId) -> bool {
        self.prev_interactivity(widget_id)
            .unwrap_or(Interactivity::ENABLED)
            .is_vis_enabled()
            && self.new_interactivity(widget_id).is_vis_disabled()
    }

    /// Returns the previous and new interactivity if the widget was enabled, disabled or is new.
    pub fn enabled_change(&self, widget_id: WidgetId) -> Option<(Option<Interactivity>, Interactivity)> {
        self.change_check(widget_id, Interactivity::is_enabled)
    }

    /// Returns the previous and new interactivity if the widget was visually enabled, visually disabled or is new.
    pub fn vis_enabled_change(&self, widget_id: WidgetId) -> Option<(Option<Interactivity>, Interactivity)> {
        self.change_check(widget_id, Interactivity::is_vis_enabled)
    }

    /// Returns the previous and new interactivity if the widget was blocked, unblocked or is new.
    pub fn blocked_change(&self, widget_id: WidgetId) -> Option<(Option<Interactivity>, Interactivity)> {
        self.change_check(widget_id, Interactivity::is_blocked)
    }

    fn change_check(&self, widget_id: WidgetId, mtd: impl Fn(Interactivity) -> bool) -> Option<(Option<Interactivity>, Interactivity)> {
        let new = self.new_interactivity(widget_id);
        let prev = self.prev_interactivity(widget_id);
        if let Some(prev) = prev {
            if mtd(prev) != mtd(new) { Some((Some(prev), new)) } else { None }
        } else {
            Some((prev, new))
        }
    }

    /// Widget is new, no previous interactivity state is known, events that filter by interactivity change
    /// update by default if the widget is new.
    pub fn is_new(&self, widget_id: WidgetId) -> bool {
        !self.prev_tree.contains(widget_id) && self.tree.contains(widget_id)
    }
}

impl VisibilityChangedArgs {
    /// Gets the previous and new visibility for the widget, if it has changed.
    pub fn change(&self, widget_id: WidgetId) -> Option<(Visibility, Visibility)> {
        let prev = *self.changed.get(&widget_id)?;
        let new = self.tree.get(widget_id)?.visibility();
        Some((prev, new))
    }

    /// Gets the previous visibility of the widget, if it has changed.
    pub fn prev_vis(&self, widget_id: WidgetId) -> Option<Visibility> {
        self.changed.get(&widget_id).copied()
    }

    /// Gets the new visibility of the widget, if it has changed.
    pub fn new_vis(&self, widget_id: WidgetId) -> Option<Visibility> {
        self.change(widget_id).map(|(_, n)| n)
    }

    /// Widget was visible or hidden, now is collapsed.
    pub fn is_collapse(&self, widget_id: WidgetId) -> bool {
        matches!(
            self.change(widget_id),
            Some((Visibility::Visible | Visibility::Hidden, Visibility::Collapsed))
        )
    }

    /// Widget was visible or collapsed, now is hidden.
    pub fn is_hide(&self, widget_id: WidgetId) -> bool {
        matches!(
            self.change(widget_id),
            Some((Visibility::Visible | Visibility::Collapsed, Visibility::Hidden))
        )
    }

    /// Widget was not hidden or collapsed, now is visible.
    pub fn is_show(&self, widget_id: WidgetId) -> bool {
        matches!(
            self.change(widget_id),
            Some((Visibility::Hidden | Visibility::Collapsed, Visibility::Visible))
        )
    }
}

/// Info about the input inline connecting rows of the widget.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct WidgetInlineMeasure {
    /// Preferred first size.
    ///
    /// In left-to-right direction the origin is `top_left`, in right-to-left direction the origin is `top_right - first.width`.
    pub first: PxSize,

    /// Indicates that `first` starts in the next row, not in the *current* row defined by the inline constraints.
    pub first_wrapped: bool,

    /// Inline segments in the first row.
    ///
    /// The sum of segment widths must be less or equal to the `first.width`.
    pub first_segs: Arc<Vec<InlineSegment>>,

    /// Preferred last size.
    ///
    /// In left-to-right direction the origin is `bottom_left - last.height`, in right-to-left direction
    /// the origin is `bottom_right - last`.
    ///
    /// Must be equal to `first` if did not wrap.
    ///
    /// Must not be empty if first is not empty, that is, must not wrap if the last item can fit in the previous row.
    pub last: PxSize,

    /// Indicates that `last` starts in a next row, not in the same row as the first.
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

    /// If all value are not different from initial.
    ///
    /// This indicates the widget has not handled the inline config yet.
    pub fn is_default(&self) -> bool {
        self.first.is_empty()
            && !self.first_wrapped
            && self.first_segs.is_empty()
            && self.last.is_empty()
            && !self.last_wrapped
            && self.last_segs.is_empty()
    }
}

/// Info about a segment in the first or last row of an inlined widget.
///
/// See [`WidgetInlineInfo::first_segs`] for more details.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct InlineSegmentInfo {
    /// Segment offset from the row rectangle origin.
    pub x: Px,
    /// Segment width.
    ///
    /// Note that the segment height is the row rectangle height.
    pub width: Px,
}

impl InlineSegmentInfo {
    /// New from `x` and `width`.
    pub fn new(x: Px, width: Px) -> Self {
        Self { x, width }
    }
}

/// Info about the inlined rows of the widget.
#[derive(Debug, Default)]
#[non_exhaustive]
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
            if seg.width <= 0 {
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

    ///Return info to default state, but retain memory for reuse.
    pub fn clear(&mut self) {
        self.first_segs.clear();
        self.last_segs.clear();
        self.rows.clear();
        self.inner_size = PxSize::zero();
        self.invalidate_negative_space();
    }

    /// If all value are not different from initial.
    ///
    /// This indicates the widget has not handled the inline config yet.
    pub fn is_default(&self) -> bool {
        self.rows.is_empty() && self.first_segs.is_empty() && self.last_segs.is_empty() && self.inner_size.is_empty()
    }
}
impl Clone for WidgetInlineInfo {
    fn clone(&self) -> Self {
        Self {
            rows: self.rows.clone(),
            first_segs: self.first_segs.clone(),
            last_segs: self.last_segs.clone(),
            inner_size: self.inner_size,
            negative_space: Mutex::new((Arc::new(vec![]), false)),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.clear();
        self.rows.extend_from_slice(&source.rows);
        self.first_segs.extend_from_slice(&source.first_segs);
        self.last_segs.extend_from_slice(&source.last_segs);
        self.inner_size = source.inner_size;
    }
}
impl PartialEq for WidgetInlineInfo {
    fn eq(&self, other: &Self) -> bool {
        self.rows == other.rows
            && self.first_segs == other.first_segs
            && self.last_segs == other.last_segs
            && self.inner_size == other.inner_size
    }
}

/// Represents the in-progress measure pass for a widget tree.
///
/// Use [`WidgetLayout::to_measure`] to instantiate.
pub struct WidgetMeasure {
    layout_widgets: Arc<LayoutUpdates>,
    inline: Option<WidgetInlineMeasure>,
    inline_locked: bool,
}
impl WidgetMeasure {
    pub(crate) fn new(layout_widgets: Arc<LayoutUpdates>) -> Self {
        Self {
            layout_widgets,
            inline: None,
            inline_locked: false,
        }
    }

    /// New with no widget layouts invalidated.
    ///
    /// Prefer [`WidgetLayout::to_measure`] instead of this.
    pub fn new_reuse(inline: Option<WidgetInlineMeasure>) -> Self {
        let mut r = Self::new(Arc::default());
        r.inline = inline;
        r
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
    /// Note that this disables inline for the calling widget's next layout too, every property that affects layout and does
    /// not support inline layout must propagate measure using this method to correctly configure the widget.
    ///
    /// Prefer [`measure_block`] as if also clears the layout constraints.
    ///
    /// [`is_inline`]: Self::is_inline
    /// [`measure_block`]: Self::measure_block
    pub fn disable_inline(&mut self) {
        if !self.inline_locked {
            self.inline = None;
        }
    }

    /// Disable inline and measure child with no inline constraints.
    pub fn measure_block(&mut self, child: &mut UiNode) -> PxSize {
        self.disable_inline();
        LAYOUT.with_no_inline(|| child.measure(self))
    }

    /// Measure the child node with inline enabled for the `child` node context.
    ///
    /// The `first_max` and `mid_clear_min` parameters match the [`InlineConstraintsMeasure`] members, and will be set in
    /// the `child` context.
    ///
    /// Note that this does not enabled inline in the calling widget if inlining was disabled by the parent nodes, it creates
    /// a new inlining context.
    ///
    /// Returns the inline requirements of the child and its desired bounds size, returns `None` requirements if the child
    /// disables inline or is not a full widget.
    ///
    /// [`InlineConstraintsMeasure`]: zng_layout::context::InlineConstraintsMeasure
    pub fn measure_inline(&mut self, first_max: Px, mid_clear_min: Px, child: &mut UiNode) -> (Option<WidgetInlineMeasure>, PxSize) {
        let constraints = InlineConstraints::Measure(InlineConstraintsMeasure::new(first_max, mid_clear_min));
        let metrics = LAYOUT.metrics().with_inline_constraints(Some(constraints));
        let size = LAYOUT.with_context(metrics, || child.measure(self));
        let inline = child
            .as_widget()
            .and_then(|mut w| w.with_context(WidgetUpdateMode::Ignore, || WIDGET.bounds().measure_inline()));
        (inline, size)
    }

    /// Measure a widget.
    pub fn with_widget(&mut self, measure: impl FnOnce(&mut Self) -> PxSize) -> PxSize {
        let metrics = LAYOUT.metrics();
        let bounds = WIDGET.bounds();

        let snap = metrics.snapshot();
        if !WIDGET.layout_is_pending(&self.layout_widgets) {
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
                    // LAYOUT.register_metrics_use(measure_uses); // measure does not propagate uses.
                    return bounds.measure_outer_size();
                }
            }
        }

        let parent_inline = self.inline.take();
        if LAYOUT.inline_constraints().is_some() {
            self.inline = Some(Default::default());
        }

        let (measure_uses, size) = LAYOUT.capture_metrics_use(|| measure(self));

        bounds.set_measure_metrics(Some(snap), measure_uses);
        bounds.set_measure_outer_size(size);

        if let Some(inline) = self.inline.take() {
            if inline.is_default() && !size.is_empty() {
                // widget did not handle inline
                bounds.set_measure_inline(None);
            } else {
                #[cfg(debug_assertions)]
                if !inline.last_wrapped && inline.first != inline.last {
                    tracing::error!(
                        "widget {:?} invalid inline measure, last {:?} != first {:?} but last did not wrap",
                        WIDGET.id(),
                        inline.last,
                        inline.first
                    );
                }

                bounds.set_measure_inline(Some(inline));
            }
        } else {
            bounds.set_measure_inline(None);
        }
        self.inline = parent_inline;

        size
    }

    /// Calls `measure` with inline force enabled on the widget.
    ///
    /// The widget will be inlining even if the parent widget is not inlining, if properties request [`disable_inline`]
    /// these requests are ignored.
    ///
    /// [`disable_inline`]: crate::widget::info::WidgetMeasure::disable_inline
    pub fn with_inline_visual(&mut self, measure: impl FnOnce(&mut Self) -> PxSize) -> PxSize {
        self.inline_locked = true;
        if self.inline.is_none() {
            self.inline = Some(Default::default());
        }
        let metrics = LAYOUT.metrics();
        let size = if metrics.inline_constraints().is_none() {
            let constraints = InlineConstraints::Measure(InlineConstraintsMeasure::new(metrics.constraints().x.max_or(Px::MAX), Px(0)));
            let metrics = metrics.with_inline_constraints(Some(constraints));
            LAYOUT.with_context(metrics, || measure(self))
        } else {
            measure(self)
        };
        self.inline_locked = false;

        let inline = self.inline.clone().unwrap();
        let bounds = WIDGET.bounds();
        if inline.is_default() && !size.is_empty() {
            // widget did not handle inline
            bounds.set_measure_inline(None);
        } else {
            bounds.set_measure_inline(Some(inline));
        }
        bounds.set_measure_outer_size(size);

        size
    }

    /// Start a parallel measure.
    ///
    /// Returns an instance that can be used to acquire multiple mutable [`WidgetMeasure`] during measure.
    /// The [`parallel_fold`] method must be called after the parallel processing is done.
    ///
    /// [`parallel_fold`]: Self::parallel_fold
    pub fn parallel_split(&self) -> ParallelBuilder<WidgetMeasure> {
        ParallelBuilder(Some(Self {
            layout_widgets: self.layout_widgets.clone(),
            inline: self.inline.clone(),
            inline_locked: self.inline_locked,
        }))
    }

    /// Collect the parallel changes back.
    pub fn parallel_fold(&mut self, mut split: ParallelBuilder<WidgetMeasure>) {
        let _ = split.take();
    }
}

/// Represents the in-progress layout pass for a widget tree.
pub struct WidgetLayout {
    layout_widgets: Arc<LayoutUpdates>,
    bounds: WidgetBoundsInfo,
    nest_group: LayoutNestGroup,
    inline: Option<WidgetInlineInfo>,
    needs_ref_count: Option<u32>,
}
impl WidgetLayout {
    /// Defines the root widget outer-bounds scope.
    ///
    /// The default window implementation calls this inside the root widget context.
    pub fn with_root_widget(layout_widgets: Arc<LayoutUpdates>, layout: impl FnOnce(&mut Self) -> PxSize) -> PxSize {
        Self {
            layout_widgets,
            bounds: WIDGET.bounds(),
            nest_group: LayoutNestGroup::Inner,
            inline: None,
            needs_ref_count: None,
        }
        .with_widget(layout)
    }

    /// Start a parallel layout.
    ///
    /// Returns an instance that can be used to acquire multiple mutable [`WidgetLayout`] during layout.
    /// The [`parallel_fold`] method must be called after the parallel processing is done.
    ///
    /// Must be called outside of the [child] scope.
    ///
    /// [child]: Self::with_child
    /// [`parallel_fold`]: Self::parallel_fold
    pub fn parallel_split(&self) -> ParallelBuilder<WidgetLayout> {
        if self.nest_group != LayoutNestGroup::Child && WIDGET.parent_id().is_some() {
            tracing::error!("called `parallel_split` outside child scope");
        }
        ParallelBuilder(Some(WidgetLayout {
            layout_widgets: self.layout_widgets.clone(),
            bounds: self.bounds.clone(),
            nest_group: LayoutNestGroup::Child,
            inline: None,
            needs_ref_count: None,
        }))
    }

    /// Collect the parallel changes back.
    pub fn parallel_fold(&mut self, mut split: ParallelBuilder<WidgetLayout>) {
        let folded = split.take();
        assert_eq!(self.bounds, folded.bounds);

        let count = self.needs_ref_count.unwrap_or(0) + folded.needs_ref_count.unwrap_or(0);
        self.needs_ref_count = Some(count);
    }

    /// Defines a widget scope, translations inside `layout` target the widget's inner offset.
    ///
    /// If the widget layout is not invalidated and none of the used metrics have changed skips calling
    /// `layout` and returns the current outer-size, the outer transform is still updated.
    ///
    /// The default widget constructor calls this, see [`base::node::widget`].
    ///
    /// [`base::node::widget`]: crate::widget::base::node::widget
    pub fn with_widget(&mut self, layout: impl FnOnce(&mut Self) -> PxSize) -> PxSize {
        let metrics = LAYOUT.metrics();
        let bounds = WIDGET.bounds();

        let snap = metrics.snapshot();
        if let Some(c) = &mut self.needs_ref_count {
            *c += 1;
        }

        if !WIDGET.take_update(UpdateFlags::LAYOUT) && !self.layout_widgets.delivery_list().enter_widget(WIDGET.id()) {
            // layout not invalidated by request
            let uses = bounds.metrics_used();
            if bounds.metrics().map(|m| m.masked_eq(&snap, uses)).unwrap_or(false) {
                // layout not invalidated by used metrics
                LAYOUT.register_metrics_use(uses); // propagate to parent
                return bounds.outer_size();
            }
        }

        let parent_needs_ref_count = self.needs_ref_count.take();
        let parent_inline = self.inline.take();
        if LAYOUT.inline_constraints().is_some() && bounds.measure_inline().is_some() {
            // inline enabled by parent and widget
            self.inline = bounds.take_inline();
            if let Some(inline) = self.inline.as_mut() {
                inline.clear();
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
        let prev_transform_style = self.bounds.transform_style();
        let prev_perspective = self.bounds.raw_perspective();
        let prev_perspective_origin = self.bounds.raw_perspective_origin();
        self.bounds.set_inner_offset(PxVector::zero());
        self.bounds.set_child_offset(PxVector::zero());
        self.bounds.set_baseline(Px(0));
        self.bounds.set_inner_offset_baseline(false);
        self.bounds.set_can_auto_hide(true);
        self.bounds.set_transform_style(TransformStyle::Flat);
        self.bounds.set_perspective(f32::INFINITY);
        self.bounds.set_perspective_origin(None);

        // layout
        let (uses, size) = LAYOUT.capture_metrics_use(|| layout(self));

        LAYOUT.register_metrics_use(uses);
        self.bounds.set_outer_size(size);
        self.bounds.set_metrics(Some(snap), uses);
        if let Some(inline) = &mut self.inline {
            inline.inner_size = self.bounds.inner_size();
            inline.invalidate_negative_space();
        }
        self.bounds.set_inline(self.inline.take());

        if prev_can_auto_hide != self.bounds.can_auto_hide() || prev_transform_style != self.bounds.transform_style() {
            WIDGET.render();
        } else if prev_inner_offset != self.bounds.inner_offset()
            || prev_child_offset != self.bounds.child_offset()
            || prev_inner_offset_baseline != self.bounds.inner_offset_baseline()
            || prev_perspective != self.bounds.raw_perspective()
            || prev_perspective_origin != self.bounds.raw_perspective_origin()
            || (self.bounds.inner_offset_baseline() && prev_baseline != self.bounds.baseline())
        {
            WIDGET.render_update();
        }

        self.needs_ref_count = parent_needs_ref_count;
        self.inline = parent_inline;
        self.bounds = parent_bounds;
        self.nest_group = LayoutNestGroup::Child;

        size
    }

    /// Calls `layout` with inline force enabled on the widget.
    ///
    /// The widget will use the inline visual even if the parent did not inline it, but it will not
    /// inline if it has properties that disable inlining.
    pub fn with_inline_visual(&mut self, layout: impl FnOnce(&mut Self) -> PxSize) -> PxSize {
        if self.is_inline() {
            let size = layout(self);
            WIDGET.bounds().set_inline(self.inline.clone());
            size
        } else {
            let bounds = WIDGET.bounds();
            if let Some(measure) = bounds.measure_inline() {
                let constraints = InlineConstraintsLayout::new(
                    PxRect::from_size(measure.first),
                    Px(0),
                    {
                        let mut r = PxRect::from_size(measure.last);
                        r.origin.y = bounds.measure_outer_size().height - measure.last.height;
                        r
                    },
                    Arc::new(vec![]),
                    Arc::new(vec![]),
                );

                self.inline = Some(Default::default());

                let metrics = LAYOUT
                    .metrics()
                    .with_inline_constraints(Some(InlineConstraints::Layout(constraints)));
                let size = LAYOUT.with_context(metrics, || layout(self));

                bounds.set_inline(self.inline.clone());
                size
            } else {
                layout(self)
            }
        }
    }

    /// Defines the widget's inner scope, translations inside `layout` target the widget's child offset.
    ///
    /// This method also updates the border info.
    ///
    /// The default widget borders constructor calls this, see [`base::node::widget_inner`].
    ///
    /// [`base::node::widget_inner`]: crate::widget::base::node::widget_inner
    pub fn with_inner(&mut self, layout: impl FnOnce(&mut Self) -> PxSize) -> PxSize {
        self.nest_group = LayoutNestGroup::Child;
        let size = BORDER.with_inner(|| layout(self));
        WIDGET.bounds().set_inner_size(size);
        self.nest_group = LayoutNestGroup::Inner;
        size
    }

    /// Defines the widget's child scope, translations inside `layout` target the widget's child offset.
    ///
    /// Returns the child size and if a reference frame is required to offset the child.
    ///
    /// The default widget child layout constructor implements this, see [`base::node::widget_child`].
    ///
    /// [`base::node::widget_child`]: crate::widget::base::node::widget_child
    /// [`child_offset`]: WidgetBoundsInfo::child_offset
    /// [`with_branch_child`]: Self::with_branch_child
    pub fn with_child(&mut self, layout: impl FnOnce(&mut Self) -> PxSize) -> (PxSize, bool) {
        let parent_needs_ref_count = self.needs_ref_count.replace(0);

        self.nest_group = LayoutNestGroup::Child;
        let child_size = layout(self);
        self.nest_group = LayoutNestGroup::Child;

        let need_ref_frame = self.needs_ref_count != Some(1);
        self.needs_ref_count = parent_needs_ref_count;
        (child_size, need_ref_frame)
    }

    /// Ensure that the parent [`with_child`] will receive a reference frame request.
    ///
    /// Nodes that branch out children inside the widget's child scope must call this to ensure that the offsets
    /// are not given to the only widget child among other nodes.
    ///
    /// [`with_child`]: Self::with_child
    pub fn require_child_ref_frame(&mut self) {
        if let Some(c) = &mut self.needs_ref_count {
            *c += 2;
        }
    }

    /// Defines a custom scope that does not affect the widget's offsets, only any widget inside `layout`.
    ///
    /// Nodes that branch out children outside widget's child scope must use this method.
    ///
    /// Returns the output of `layout` and a translate vector if any translations inside `layout` where not handled
    /// by child widgets.
    pub fn with_branch_child(&mut self, layout: impl FnOnce(&mut Self) -> PxSize) -> (PxSize, PxVector) {
        let parent_needs_ref_count = self.needs_ref_count;
        let parent_translate = self.bounds.child_offset();
        let parent_inner_offset_baseline = self.bounds.inner_offset_baseline();
        self.bounds.set_child_offset(PxVector::zero());
        let parent_group = self.nest_group;

        self.nest_group = LayoutNestGroup::Child;
        let child_size = layout(self);

        let translate = self.bounds.child_offset();
        self.bounds.set_child_offset(parent_translate);
        self.bounds.set_inner_offset_baseline(parent_inner_offset_baseline);
        self.nest_group = parent_group;
        self.needs_ref_count = parent_needs_ref_count;

        (child_size, translate)
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

    /// Set if the baseline is added to the inner offset *y* axis.
    pub fn translate_baseline(&mut self, enabled: bool) {
        self.bounds.set_inner_offset_baseline(enabled);
    }

    /// Set if the widget preserved 3D perspective form the parent.
    pub fn set_transform_style(&mut self, style: TransformStyle) {
        self.bounds.set_transform_style(style);
    }

    /// Set the 3D perspective that defines the children 3D space.
    ///
    /// This is the distance from the Z-plane to the viewer.
    pub fn set_perspective(&mut self, d: f32) {
        self.bounds.set_perspective(d)
    }

    /// Sets the vanishing point of the children 3D space as a point in the inner bounds of this widget.
    pub fn set_perspective_origin(&mut self, origin: PxPoint) {
        self.bounds.set_perspective_origin(Some(origin))
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
    /// Nodes that set the visibility to the equivalent of [`Collapsed`] must skip layout and return `PxSize::zero` as
    /// the size, ignoring the min-size constraints, and call this method to update all the descendant
    /// bounds information to be a zero-sized point.
    ///
    /// Note that the widget will automatically not be rendered when collapsed.
    ///
    /// [`Collapsed`]: Visibility::Collapsed
    pub fn collapse(&mut self) {
        WIDGET.take_update(UpdateFlags::LAYOUT);
        let tree = WINDOW.info();
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
                info.bounds_info.set_measure_metrics(None, LayoutMask::empty());
                info.bounds_info.set_metrics(None, LayoutMask::empty());
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
        let tree = WINDOW.info();
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
                info.bounds_info.set_measure_metrics(None, LayoutMask::empty());
                info.bounds_info.set_metrics(None, LayoutMask::empty());
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
        let tree = WINDOW.info();
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
                    info.bounds_info.set_measure_metrics(None, LayoutMask::empty());
                    info.bounds_info.set_metrics(None, LayoutMask::empty());
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
    /// in the previous measure pass. You can use [`WidgetMeasure::disable_inline`] in the measure pass to disable
    /// inline in both passes, measure and layout.
    ///
    /// The rows and negative space are already reset when widget layout started, and the inner size will be updated when
    /// the widget layout ends, the inline layout node only needs to push rows.
    ///
    /// When this is `Some(_)` the [`LayoutMetrics::inline_constraints`] is also `Some(_)`.
    ///
    /// See [`WidgetInlineInfo`] for more details.
    ///
    /// [`LayoutMetrics::inline_constraints`]: zng_layout::context::LayoutMetrics::inline_constraints
    pub fn inline(&mut self) -> Option<&mut WidgetInlineInfo> {
        self.inline.as_mut()
    }

    /// Create an [`WidgetMeasure`] for an [`UiNode::measure`] call.
    ///
    /// [`UiNode::measure`]: crate::widget::node::UiNode::measure
    pub fn to_measure(&self, inline: Option<WidgetInlineMeasure>) -> WidgetMeasure {
        WidgetMeasure {
            layout_widgets: self.layout_widgets.clone(),
            inline,
            inline_locked: false,
        }
    }

    /// Layout the child node in a context without inline constraints.
    ///
    /// This must be called inside inlining widgets to layout block child nodes, otherwise the inline constraints from
    /// the calling widget propagate to the child.
    pub fn layout_block(&mut self, child: &mut UiNode) -> PxSize {
        LAYOUT.with_no_inline(|| child.layout(self))
    }

    /// Layout the child node with inline enabled in the `child` node context.
    ///
    /// The `mid_clear`, `last`, `first_segs` and `last_segs` parameters match the [`InlineConstraintsLayout`] members, and will be set in
    /// the `child` context.
    ///
    /// Returns the child final size.
    ///
    /// [`InlineConstraintsLayout`]: zng_layout::context::InlineConstraintsLayout
    pub fn layout_inline(
        &mut self,
        first: PxRect,
        mid_clear: Px,
        last: PxRect,
        first_segs: Arc<Vec<InlineSegmentPos>>,
        last_segs: Arc<Vec<InlineSegmentPos>>,
        child: &mut UiNode,
    ) -> PxSize {
        let constraints = InlineConstraints::Layout(InlineConstraintsLayout::new(first, mid_clear, last, first_segs, last_segs));
        let metrics = LAYOUT.metrics().with_inline_constraints(Some(constraints));
        LAYOUT.with_context(metrics, || child.layout(self))
    }

    /// Call `layout` with a different set of `layout_updates`.
    ///
    /// This is usually managed by the window implementer, nested windows can use this to override the updates.
    pub fn with_layout_updates(&mut self, layout_updates: Arc<LayoutUpdates>, layout: impl FnOnce(&mut WidgetLayout) -> PxSize) -> PxSize {
        let parent_layout_widgets = mem::replace(&mut self.layout_widgets, layout_updates);
        let r = layout(self);
        self.layout_widgets = parent_layout_widgets;
        r
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum LayoutNestGroup {
    /// Inside widget, outside `BORDER`.
    Inner,
    /// Inside `BORDER`.
    Child,
}

/// Represents a builder split from the main builder that can be used in parallel and then folded
/// back onto the main builder.
///
/// # Error
///
/// Logs an error on drop if it was not moved to the `B::parallel_fold` method.
#[must_use = "use in parallel task, then move it to `B::parallel_fold`"]
pub struct ParallelBuilder<B>(pub(crate) Option<B>);
impl<B> ParallelBuilder<B> {
    pub(crate) fn take(&mut self) -> B {
        self.0.take().expect("parallel builder finished")
    }
}
impl<B> ops::Deref for ParallelBuilder<B> {
    type Target = B;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref().expect("parallel builder finished")
    }
}
impl<B> ops::DerefMut for ParallelBuilder<B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut().expect("parallel builder finished")
    }
}
impl<B> Drop for ParallelBuilder<B> {
    fn drop(&mut self) {
        if self.0.is_some() && !std::thread::panicking() {
            tracing::error!("builder dropped without calling `{}::parallel_fold`", std::any::type_name::<B>())
        }
    }
}
