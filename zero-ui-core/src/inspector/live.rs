//! UI live inspector.
//!
//! Interactive UI inspectors can use this module as data source.

use std::{fmt, sync::Arc};

use parking_lot::Mutex;

use crate::{
    text::Txt,
    var::*,
    widget_builder::WidgetType,
    widget_info::{WidgetInfo, WidgetInfoTree},
    widget_instance::WidgetId,
    IdMap,
};

use super::WidgetInfoInspectorExt;

/// Represents an actively inspected widget tree.
#[derive(Clone)]
pub struct InspectedTree {
    tree: ArcVar<WidgetInfoTree>,
    widgets: Arc<Mutex<IdMap<WidgetId, InspectedWidget>>>,
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
        self.tree.var_ptr() == other.tree.var_ptr()
    }
}
impl InspectedTree {
    /// Initial inspection.
    pub fn new(tree: WidgetInfoTree) -> Self {
        Self {
            widgets: Arc::new(Mutex::new(IdMap::new())),
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

        let mut widgets = self.widgets.lock();
        let mut removed = false;
        for (k, v) in widgets.iter() {
            if let Some(w) = tree.get(*k) {
                v.update(w);
            } else {
                v.removed.set(true);
                removed = true;
            }
        }
        // update can drop children inspectors so we can't update inside the retain closure.
        widgets.retain(|k, v| v.info.strong_count() > 1 && (!removed || tree.get(*k).is_some()));

        self.tree.set(tree);
    }

    /// Create a weak reference to this tree.
    pub fn downgrade(&self) -> WeakInspectedTree {
        WeakInspectedTree {
            tree: self.tree.downgrade(),
            widgets: Arc::downgrade(&self.widgets),
        }
    }

    /// Latest info.
    pub fn tree(&self) -> impl Var<WidgetInfoTree> {
        self.tree.read_only()
    }

    /// Gets a widget inspector if the widget is in the latest info.
    pub fn inspect(&self, widget_id: WidgetId) -> Option<InspectedWidget> {
        match self.widgets.lock().entry(widget_id) {
            hashbrown::hash_map::Entry::Occupied(e) => Some(e.get().clone()),
            hashbrown::hash_map::Entry::Vacant(e) => self.tree.with(|t| {
                t.get(widget_id)
                    .map(|w| e.insert(InspectedWidget::new(w, self.downgrade())).clone())
            }),
        }
    }

    /// Gets a widget inspector for the root widget.
    pub fn inspect_root(&self) -> InspectedWidget {
        self.inspect(self.tree.with(|t| t.root().id())).unwrap()
    }
}

/// Represents a weak reference to a [`InspectedTree`].
#[derive(Clone)]
pub struct WeakInspectedTree {
    tree: types::WeakArcVar<WidgetInfoTree>,
    widgets: std::sync::Weak<Mutex<IdMap<WidgetId, InspectedWidget>>>,
}
impl WeakInspectedTree {
    /// Try to get a strong reference to the inspected tree.
    pub fn upgrade(&self) -> Option<InspectedTree> {
        Some(InspectedTree {
            tree: self.tree.upgrade()?,
            widgets: self.widgets.upgrade()?,
        })
    }
}

struct InspectedWidgetCache {
    tree: WeakInspectedTree,
    children: Option<BoxedVar<Vec<InspectedWidget>>>,
    parent_property_name: Option<BoxedVar<Txt>>,
}

/// Represents an actively inspected widget.
///
/// See [`InspectedTree::inspect`].
#[derive(Clone)]
pub struct InspectedWidget {
    info: ArcVar<WidgetInfo>,
    removed: ArcVar<bool>,
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
        self.info.var_ptr() == other.info.var_ptr()
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
    pub fn removed(&self) -> impl Var<bool> {
        self.removed.read_only()
    }

    /// Latest info.
    pub fn info(&self) -> impl Var<WidgetInfo> {
        self.info.read_only()
    }

    /// Widget id.
    pub fn id(&self) -> WidgetId {
        self.info.with(|i| i.id())
    }

    /// Count of ancestor widgets.
    pub fn depth(&self) -> impl Var<usize> {
        self.info.map(|w| w.depth()).actual_var()
    }

    /// Count of descendant widgets.
    pub fn descendants_len(&self) -> impl Var<usize> {
        self.info.map(|w| w.descendants_len()).actual_var()
    }

    /// Widget type, if the widget was built with inspection info.
    pub fn wgt_type(&self) -> impl Var<Option<WidgetType>> {
        self.info.map(|w| Some(w.inspector_info()?.builder.widget_type())).actual_var()
    }

    /// Widget type name, or `"<widget>"` if widget was not built with inspection info.
    pub fn wgt_type_name(&self) -> impl Var<Txt> {
        self.info
            .map(|w| match w.inspector_info().map(|i| i.builder.widget_type()) {
                Some(t) => Txt::from_str(t.name()),
                None => Txt::from_static("<widget>"),
            })
            .actual_var()
    }

    /// Gets the parent's property that has this widget as an input.
    ///
    /// Is an empty string if the widget is not inserted by any property.
    pub fn parent_property_name(&self) -> impl Var<Txt> {
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
                    .actual_var()
                    .boxed()
            })
            .clone()
    }

    /// Inspect the widget children.
    pub fn children(&self) -> impl Var<Vec<InspectedWidget>> {
        let mut cache = self.cache.lock();
        let cache = &mut *cache;
        cache
            .children
            .get_or_insert_with(|| {
                let tree = cache.tree.clone();
                self.info
                    .map(move |w| {
                        if let Some(tree) = tree.upgrade() {
                            w.children().map(|w| tree.inspect(w.id()).unwrap()).collect()
                        } else {
                            vec![]
                        }
                    })
                    .actual_var()
                    .boxed()
            })
            .clone()
    }
}
