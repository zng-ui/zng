use std::{collections::HashMap, fmt, ops, sync::Arc};

use parking_lot::Mutex;
use zng_app::widget::{
    builder::WidgetType,
    info::WidgetInfoTree,
    inspector::{InspectorInfo, WidgetInfoInspectorExt},
};
use zng_view_api::window::FrameId;
use zng_wgt::prelude::*;

#[derive(Default)]
struct InspectedTreeData {
    widgets: IdMap<WidgetId, InspectedWidget>,
    latest_frame: Option<Var<FrameId>>,
}

/// Represents an actively inspected widget tree.
#[derive(Clone)]
pub struct InspectedTree {
    tree: Var<WidgetInfoTree>,
    data: Arc<Mutex<InspectedTreeData>>,
}
impl fmt::Debug for InspectedTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InspectedTree")
            .field("tree", &self.tree.get())
            .finish_non_exhaustive()
    }
}
impl PartialEq for InspectedTree {
    fn eq(&self, other: &Self) -> bool {
        self.tree.var_eq(&other.tree)
    }
}
impl InspectedTree {
    /// Initial inspection.
    pub fn new(tree: WidgetInfoTree) -> Self {
        Self {
            data: Arc::new(Mutex::new(InspectedTreeData::default())),
            tree: var(tree),
        }
    }

    /// Update inspection.
    ///
    /// # Panics
    ///
    /// Panics if info is not for the same window ID.
    pub fn update(&self, tree: WidgetInfoTree) {
        assert_eq!(self.tree.with(|t| t.window_id()), tree.window_id());

        // update and retain
        self.tree.set(tree.clone());

        let mut data = self.data.lock();
        let mut removed = false;
        for (k, v) in data.widgets.iter() {
            if let Some(w) = tree.get(*k) {
                v.update(w);
            } else {
                v.removed.set(true);
                removed = true;
            }
        }
        // update can drop children inspectors so we can't update inside the retain closure.
        data.widgets
            .retain(|k, v| v.info.strong_count() > 1 && (!removed || tree.get(*k).is_some()));

        if let Some(f) = &data.latest_frame {
            if f.strong_count() == 1 {
                data.latest_frame = None;
            } else {
                f.set(tree.stats().last_frame);
            }
        }
    }

    /// Update all render watcher variables.
    pub fn update_render(&self) {
        let mut data = self.data.lock();
        if let Some(f) = &data.latest_frame {
            if f.strong_count() == 1 {
                data.latest_frame = None;
            } else {
                f.set(self.tree.with(|t| t.stats().last_frame));
            }
        }
    }

    /// Create a weak reference to this tree.
    pub fn downgrade(&self) -> WeakInspectedTree {
        WeakInspectedTree {
            tree: self.tree.downgrade(),
            data: Arc::downgrade(&self.data),
        }
    }

    /// Gets a widget inspector if the widget is in the latest info.
    pub fn inspect(&self, widget_id: WidgetId) -> Option<InspectedWidget> {
        match self.data.lock().widgets.entry(widget_id) {
            IdEntry::Occupied(e) => Some(e.get().clone()),
            IdEntry::Vacant(e) => self.tree.with(|t| {
                t.get(widget_id)
                    .map(|w| e.insert(InspectedWidget::new(w, self.downgrade())).clone())
            }),
        }
    }

    /// Gets a widget inspector for the root widget.
    pub fn inspect_root(&self) -> InspectedWidget {
        self.inspect(self.tree.with(|t| t.root().id())).unwrap()
    }

    /// Latest frame updated using [`update_render`].
    ///
    /// [`update_render`]: Self::update_render
    pub fn last_frame(&self) -> Var<FrameId> {
        let mut data = self.data.lock();
        data.latest_frame
            .get_or_insert_with(|| var(self.tree.with(|t| t.stats().last_frame)))
            .clone()
    }
}

/// Represents a weak reference to a [`InspectedTree`].
#[derive(Clone)]
pub struct WeakInspectedTree {
    tree: WeakVar<WidgetInfoTree>,
    data: std::sync::Weak<Mutex<InspectedTreeData>>,
}
impl WeakInspectedTree {
    /// Try to get a strong reference to the inspected tree.
    pub fn upgrade(&self) -> Option<InspectedTree> {
        Some(InspectedTree {
            tree: self.tree.upgrade()?,
            data: self.data.upgrade()?,
        })
    }
}

struct InspectedWidgetCache {
    tree: WeakInspectedTree,
    children: Option<Var<Vec<InspectedWidget>>>,
    parent_property_name: Option<Var<Txt>>,
}

/// Represents an actively inspected widget.
///
/// See [`InspectedTree::inspect`].
#[derive(Clone)]
pub struct InspectedWidget {
    info: Var<WidgetInfo>,
    removed: Var<bool>,
    cache: Arc<Mutex<InspectedWidgetCache>>,
}
impl fmt::Debug for InspectedWidget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InspectedWidget")
            .field("info", &self.info.get())
            .field("removed", &self.removed.get())
            .finish_non_exhaustive()
    }
}
impl PartialEq for InspectedWidget {
    fn eq(&self, other: &Self) -> bool {
        self.info.var_eq(&other.info)
    }
}
impl Eq for InspectedWidget {}
impl InspectedWidget {
    /// Initial inspection.
    fn new(info: WidgetInfo, tree: WeakInspectedTree) -> Self {
        Self {
            info: var(info),
            removed: var(false),
            cache: Arc::new(Mutex::new(InspectedWidgetCache {
                tree,
                children: None,
                parent_property_name: None,
            })),
        }
    }

    /// Update inspection.
    ///
    /// # Panics
    ///
    /// Panics if info is not for the same widget ID.
    fn update(&self, info: WidgetInfo) {
        assert_eq!(self.info.with(|i| i.id()), info.id());
        self.info.set(info);

        let mut cache = self.cache.lock();
        if let Some(c) = &cache.children {
            if c.strong_count() == 1 {
                cache.children = None;
            }
        }
        if let Some(c) = &cache.parent_property_name {
            if c.strong_count() == 1 {
                cache.parent_property_name = None;
            }
        }
    }

    /// If this widget inspector is permanently disconnected and will not update.
    ///
    /// This is set to `true` when an inspected widget is not found after an update, when `true`
    /// this inspector will not update even if the same widget ID is re-inserted in another update.
    pub fn removed(&self) -> Var<bool> {
        self.removed.read_only()
    }

    /// Latest info.
    pub fn info(&self) -> Var<WidgetInfo> {
        self.info.read_only()
    }

    /// Widget id.
    pub fn id(&self) -> WidgetId {
        self.info.with(|i| i.id())
    }

    /// Count of ancestor widgets.
    pub fn depth(&self) -> Var<usize> {
        self.info.map(|w| w.depth()).current_context()
    }

    /// Count of descendant widgets.
    pub fn descendants_len(&self) -> Var<usize> {
        self.info.map(|w| w.descendants_len()).current_context()
    }

    /// Widget type, if the widget was built with inspection info.
    pub fn wgt_type(&self) -> Var<Option<WidgetType>> {
        self.info.map(|w| Some(w.inspector_info()?.builder.widget_type())).current_context()
    }

    /// Widget macro name, or `"<widget>!"` if widget was not built with inspection info.
    pub fn wgt_macro_name(&self) -> Var<Txt> {
        self.info
            .map(|w| match w.inspector_info().map(|i| i.builder.widget_type()) {
                Some(t) => formatx!("{}!", t.name()),
                None => Txt::from_static("<widget>!"),
            })
            .current_context()
    }

    /// Gets the parent's property that has this widget as an input.
    ///
    /// Is an empty string if the widget is not inserted by any property.
    pub fn parent_property_name(&self) -> Var<Txt> {
        let mut cache = self.cache.lock();
        cache
            .parent_property_name
            .get_or_insert_with(|| {
                self.info
                    .map(|w| {
                        Txt::from_static(
                            w.parent_property()
                                .map(|(p, _)| w.parent().unwrap().inspect_property(p).unwrap().property().name)
                                .unwrap_or(""),
                        )
                    })
                    .current_context()
            })
            .clone()
    }

    /// Inspect the widget children.
    pub fn children(&self) -> Var<Vec<InspectedWidget>> {
        let mut cache = self.cache.lock();
        let cache = &mut *cache;
        cache
            .children
            .get_or_insert_with(|| {
                let tree = cache.tree.clone();
                self.info
                    .map(move |w| {
                        if let Some(tree) = tree.upgrade() {
                            assert_eq!(&tree.tree.get(), w.tree());

                            w.children().map(|w| tree.inspect(w.id()).unwrap()).collect()
                        } else {
                            vec![]
                        }
                    })
                    .current_context()
            })
            .clone()
    }

    /// Inspect the builder, properties and intrinsic nodes that make up the widget.
    ///
    /// Is `None` when the widget is built without inspector info collection.
    pub fn inspector_info(&self) -> Var<Option<InspectedInfo>> {
        self.info.map(move |w| w.inspector_info().map(InspectedInfo)).current_context()
    }

    /// Create a variable that probes info after every frame is rendered.
    pub fn render_watcher<T: VarValue>(&self, mut probe: impl FnMut(&WidgetInfo) -> T + Send + 'static) -> Var<T> {
        var_merge!(
            self.info.clone(),
            self.cache.lock().tree.upgrade().unwrap().last_frame(),
            move |w, _| probe(w)
        )
    }
}

/// [`InspectorInfo`] that can be placed in a variable.
#[derive(Clone)]
pub struct InspectedInfo(pub Arc<InspectorInfo>);
impl fmt::Debug for InspectedInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}
impl PartialEq for InspectedInfo {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl ops::Deref for InspectedInfo {
    type Target = InspectorInfo;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Builder for [`INSPECTOR.register_watcher`].
///
/// [`INSPECTOR.register_watcher`]: INSPECTOR::register_watcher
#[non_exhaustive]
pub struct InspectorWatcherBuilder {
    watchers: HashMap<Txt, Var<Txt>>,
}
impl InspectorWatcherBuilder {
    /// Insert a watcher variable.
    pub fn insert(&mut self, name: impl Into<Txt>, value: impl IntoVar<Txt>) {
        self.insert_impl(name.into(), value.into_var());
    }
    fn insert_impl(&mut self, name: Txt, value: Var<Txt>) {
        self.watchers.insert(name, value);
    }
}

app_local! {
    #[allow(clippy::type_complexity)]
    static INSPECTOR_SV: Vec<Box<dyn FnMut(&InspectedWidget, &mut InspectorWatcherBuilder) + Send + Sync + 'static>> = vec![];
}

/// Service that configures the live inspector.
pub struct INSPECTOR;
impl INSPECTOR {
    /// Register a `watcher` that provides custom live state variables.
    ///
    /// In the default live inspector the `watcher` closure is called for the selected widget and the watcher values are presented
    /// in the `/* INFO */` section of the properties panel.
    ///
    /// Note that newly registered watchers only apply for subsequent inspections, it does not refresh current views.
    pub fn register_watcher(&self, watcher: impl FnMut(&InspectedWidget, &mut InspectorWatcherBuilder) + Send + Sync + 'static) {
        INSPECTOR_SV.write().push(Box::new(watcher));
    }

    /// Call all registered watchers on the `target`.
    ///
    /// Returns a vector of unique name and watcher variable, sorted  by name.
    pub fn build_watchers(&self, target: &InspectedWidget) -> Vec<(Txt, Var<Txt>)> {
        let mut builder = InspectorWatcherBuilder { watchers: HashMap::new() };
        self.default_watchers(target, &mut builder);
        for w in INSPECTOR_SV.write().iter_mut() {
            w(target, &mut builder);
        }
        let mut watchers: Vec<_> = builder.watchers.into_iter().collect();
        watchers.sort_by(|a, b| a.0.cmp(&b.0));
        watchers
    }

    fn default_watchers(&self, target: &InspectedWidget, builder: &mut InspectorWatcherBuilder) {
        builder.insert("interactivity", target.info().map(|i| formatx!("{:?}", i.interactivity())));
        builder.insert("visibility", target.render_watcher(|i| formatx!("{:?}", i.visibility())));
        builder.insert(
            "inner_bounds",
            target.render_watcher(|i| formatx!("{:?}", i.bounds_info().inner_bounds())),
        );
    }
}
