//! App updates API.

use std::{
    collections::{hash_map, HashMap},
    fmt, mem,
    sync::{atomic::AtomicBool, Arc},
    task::Waker,
};

use parking_lot::Mutex;
use zero_ui_app_context::app_local;
use zero_ui_handle::{Handle, HandleOwner, WeakHandle};
use zero_ui_unique_id::IdSet;
use zero_ui_var::VARS_APP;

use crate::{
    event::{AnyEvent, AnyEventArgs, AppDisconnected, EVENTS, EVENTS_SV},
    handler::{async_app_hn_once, AppHandler, AppHandlerArgs, AppWeakHandle},
    timer::TIMERS_SV,
    widget::{
        info::{InteractionPath, WidgetInfo, WidgetInfoTree, WidgetPath},
        instance::{BoxedUiNode, UiNode},
        WidgetId, WIDGET,
    },
    window::{WindowId, WINDOW},
    AppEventSender, AppExtension, LoopTimer,
};

/// Represents all the widgets and windows on route to an update target.
pub struct UpdateDeliveryList {
    subscribers: Box<dyn UpdateSubscribers>,

    windows: IdSet<WindowId>,
    widgets: IdSet<WidgetId>,
    search: IdSet<WidgetId>,
}
impl fmt::Debug for UpdateDeliveryList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpdateDeliveryList")
            .field("windows", &self.windows)
            .field("widgets", &self.widgets)
            .field("search", &self.search)
            .finish_non_exhaustive()
    }
}
impl Default for UpdateDeliveryList {
    fn default() -> Self {
        Self::new_any()
    }
}
impl UpdateDeliveryList {
    /// New list that only allows `subscribers`.
    pub fn new(subscribers: Box<dyn UpdateSubscribers>) -> Self {
        Self {
            subscribers,
            windows: IdSet::default(),
            widgets: IdSet::default(),
            search: IdSet::default(),
        }
    }

    /// New list that does not allow any entry.
    pub fn new_none() -> Self {
        struct UpdateDeliveryListNone;
        impl UpdateSubscribers for UpdateDeliveryListNone {
            fn contains(&self, _: WidgetId) -> bool {
                false
            }
            fn to_set(&self) -> IdSet<WidgetId> {
                IdSet::default()
            }
        }
        Self::new(Box::new(UpdateDeliveryListNone))
    }

    /// New list that allows all entries.
    ///
    /// This is the default value.
    pub fn new_any() -> Self {
        struct UpdateDeliveryListAny;
        impl UpdateSubscribers for UpdateDeliveryListAny {
            fn contains(&self, _: WidgetId) -> bool {
                true
            }
            fn to_set(&self) -> IdSet<WidgetId> {
                IdSet::default()
            }
        }
        Self::new(Box::new(UpdateDeliveryListAny))
    }

    pub(crate) fn insert_updates_root(&mut self, window_id: WindowId, root_id: WidgetId) {
        self.windows.insert(window_id);
        self.widgets.insert(root_id);
    }

    /// Insert the ancestors of `wgt` and `wgt` up-to the inner most that is included in the subscribers.
    pub fn insert_wgt(&mut self, wgt: &impl WidgetPathProvider) {
        let mut any = false;
        for w in wgt.widget_and_ancestors() {
            if any || self.subscribers.contains(w) {
                any = true;
                self.widgets.insert(w);
            }
        }
        if any {
            self.windows.insert(wgt.window_id());
        }
    }

    /// Insert the window by itself.
    pub fn insert_window(&mut self, id: WindowId) {
        self.windows.insert(id);
    }

    /// Register all subscribers for search and delivery.
    pub fn search_all(&mut self) {
        self.search = self.subscribers.to_set();
    }

    /// Register the widget of unknown location for search before delivery routing starts.
    pub fn search_widget(&mut self, widget_id: WidgetId) {
        if self.subscribers.contains(widget_id) {
            self.search.insert(widget_id);
        }
    }

    /// If the the list has pending widgets that must be found before delivery can start.
    pub fn has_pending_search(&mut self) -> bool {
        !self.search.is_empty()
    }

    /// Search all pending widgets in all `windows`, all search items are cleared, even if not found.
    pub fn fulfill_search<'a, 'b>(&'a mut self, windows: impl Iterator<Item = &'b WidgetInfoTree>) {
        for window in windows {
            self.search.retain(|w| {
                if let Some(w) = window.get(*w) {
                    for w in w.widget_and_ancestors() {
                        self.widgets.insert(w);
                    }
                    self.windows.insert(w.window_id());
                    false
                } else {
                    true
                }
            });
        }
        self.search.clear();
    }

    /// Copy windows, widgets and search from `other`, trusting that all values are allowed.
    fn extend_unchecked(&mut self, other: UpdateDeliveryList) {
        if self.windows.is_empty() {
            self.windows = other.windows;
        } else {
            self.windows.extend(other.windows);
        }

        if self.widgets.is_empty() {
            self.widgets = other.widgets;
        } else {
            self.widgets.extend(other.widgets);
        }

        if self.search.is_empty() {
            self.search = other.search;
        } else {
            self.search.extend(other.search);
        }
    }

    /// Returns `true` if the window is on the list.
    pub fn enter_window(&self, window_id: WindowId) -> bool {
        self.windows.contains(&window_id)
    }

    /// Returns `true` if the widget is on the list.
    pub fn enter_widget(&self, widget_id: WidgetId) -> bool {
        self.widgets.contains(&widget_id)
    }

    /// Windows in the delivery list.
    pub fn windows(&self) -> &IdSet<WindowId> {
        &self.windows
    }

    /// Found widgets in the delivery list, can be targets or ancestors of targets.
    pub fn widgets(&self) -> &IdSet<WidgetId> {
        &self.widgets
    }

    /// Not found target widgets.
    ///
    /// Each window searches for these widgets and adds then to the [`widgets`] list.
    ///
    /// [`widgets`]: Self::widgets
    pub fn search_widgets(&mut self) -> &IdSet<WidgetId> {
        &self.search
    }
}

/// Provides an iterator of widget IDs and a window ID.
pub trait WidgetPathProvider {
    /// Output of `widget_and_ancestors`.
    type WidgetIter<'s>: Iterator<Item = WidgetId>
    where
        Self: 's;

    /// The window parent.
    fn window_id(&self) -> WindowId;
    /// Iterate over the widget, parent, grandparent, .., root.
    fn widget_and_ancestors(&self) -> Self::WidgetIter<'_>;
}
impl WidgetPathProvider for WidgetInfo {
    type WidgetIter<'s> = std::iter::Map<crate::widget::info::iter::Ancestors, fn(WidgetInfo) -> WidgetId>;

    fn window_id(&self) -> WindowId {
        self.tree().window_id()
    }

    fn widget_and_ancestors(&self) -> Self::WidgetIter<'_> {
        fn wgt_to_id(wgt: WidgetInfo) -> WidgetId {
            wgt.id()
        }
        self.self_and_ancestors().map(wgt_to_id)
    }
}
impl WidgetPathProvider for WidgetPath {
    type WidgetIter<'s> = std::iter::Rev<std::iter::Copied<std::slice::Iter<'s, WidgetId>>>;

    fn window_id(&self) -> WindowId {
        self.window_id()
    }

    fn widget_and_ancestors(&self) -> Self::WidgetIter<'_> {
        self.widgets_path().iter().copied().rev()
    }
}
impl WidgetPathProvider for InteractionPath {
    type WidgetIter<'s> = std::iter::Rev<std::iter::Copied<std::slice::Iter<'s, WidgetId>>>;

    fn window_id(&self) -> WindowId {
        WidgetPath::window_id(self)
    }

    fn widget_and_ancestors(&self) -> Self::WidgetIter<'_> {
        self.widgets_path().iter().copied().rev()
    }
}

/// Represents a set of widgets that subscribe to an event source.
pub trait UpdateSubscribers: Send + Sync + 'static {
    /// Returns `true` if the widget is one of the subscribers.
    fn contains(&self, widget_id: WidgetId) -> bool;

    /// Gets all subscribers as a set.
    fn to_set(&self) -> IdSet<WidgetId>;
}

/// Represents a single event update.
pub struct EventUpdate {
    pub(crate) event: AnyEvent,
    pub(crate) args: Box<dyn AnyEventArgs>,
    pub(crate) delivery_list: UpdateDeliveryList,
    // never locked, only used to get `Sync`.
    pub(crate) pre_actions: Mutex<Vec<Box<dyn FnOnce(&EventUpdate) + Send>>>,
    pub(crate) pos_actions: Mutex<Vec<Box<dyn FnOnce(&EventUpdate) + Send>>>,
}
impl EventUpdate {
    /// The event.
    pub fn event(&self) -> AnyEvent {
        self.event
    }

    /// The update delivery list.
    pub fn delivery_list(&self) -> &UpdateDeliveryList {
        &self.delivery_list
    }

    /// The update delivery list.
    pub fn delivery_list_mut(&mut self) -> &mut UpdateDeliveryList {
        &mut self.delivery_list
    }

    /// The update args.
    pub fn args(&self) -> &dyn AnyEventArgs {
        &*self.args
    }

    /// Calls `handle` if the event targets the window.
    pub fn with_window<H, R>(&self, handle: H) -> Option<R>
    where
        H: FnOnce() -> R,
    {
        if self.delivery_list.enter_window(WINDOW.id()) {
            Some(handle())
        } else {
            None
        }
    }

    /// Calls `handle` if the event targets the widget and propagation is not stopped.
    pub fn with_widget<H, R>(&self, handle: H) -> Option<R>
    where
        H: FnOnce() -> R,
    {
        if self.delivery_list.enter_widget(WIDGET.id()) {
            if self.args.propagation().is_stopped() {
                None
            } else {
                Some(handle())
            }
        } else {
            None
        }
    }

    pub(crate) fn push_once_action(&mut self, action: Box<dyn FnOnce(&EventUpdate) + Send>, is_preview: bool) {
        if is_preview {
            self.pre_actions.get_mut().push(action);
        } else {
            self.pos_actions.get_mut().push(action);
        }
    }

    pub(crate) fn call_pre_actions(&mut self) {
        let _s = tracing::trace_span!("call_pre_actions");
        let actions = mem::take(self.pre_actions.get_mut());
        for action in actions {
            action(self)
        }
    }

    pub(crate) fn call_pos_actions(&mut self) {
        let _s = tracing::trace_span!("call_pos_actions");
        let actions = mem::take(self.pos_actions.get_mut());
        for action in actions {
            action(self)
        }
    }
}
impl fmt::Debug for EventUpdate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventUpdate")
            .field("event", &self.event)
            .field("args", &self.args)
            .field("delivery_list", &self.delivery_list)
            .finish_non_exhaustive()
    }
}

/// Widget info updates of the current cycle.
#[derive(Debug, Default)]
pub struct InfoUpdates {
    delivery_list: UpdateDeliveryList,
}
impl InfoUpdates {
    /// New with list.
    pub fn new(delivery_list: UpdateDeliveryList) -> Self {
        Self { delivery_list }
    }

    /// Request delivery list.
    pub fn delivery_list(&self) -> &UpdateDeliveryList {
        &self.delivery_list
    }

    /// Request delivery list.
    pub fn delivery_list_mut(&mut self) -> &mut UpdateDeliveryList {
        &mut self.delivery_list
    }

    /// Calls `handle` if info rebuild was requested for the window.
    pub fn with_window<H, R>(&self, window_id: WindowId, handle: H) -> Option<R>
    where
        H: FnOnce() -> R,
    {
        if self.delivery_list.enter_window(window_id) {
            Some(handle())
        } else {
            None
        }
    }

    /// Copy all delivery from `other` onto `self`.
    pub fn extend(&mut self, other: InfoUpdates) {
        self.delivery_list.extend_unchecked(other.delivery_list)
    }
}

/// Widget updates of the current cycle.
#[derive(Debug, Default)]
pub struct WidgetUpdates {
    pub(crate) delivery_list: UpdateDeliveryList,
}
impl WidgetUpdates {
    /// New with list.
    pub fn new(delivery_list: UpdateDeliveryList) -> Self {
        Self { delivery_list }
    }

    /// Updates delivery list.
    pub fn delivery_list(&self) -> &UpdateDeliveryList {
        &self.delivery_list
    }

    /// Updates delivery list.
    pub fn delivery_list_mut(&mut self) -> &mut UpdateDeliveryList {
        &mut self.delivery_list
    }

    /// Calls `handle` if update was requested for the [`WINDOW`].
    pub fn with_window<H, R>(&self, handle: H) -> Option<R>
    where
        H: FnOnce() -> R,
    {
        if self.delivery_list.enter_window(WINDOW.id()) {
            Some(handle())
        } else {
            None
        }
    }

    /// Calls `handle` if update was requested for the [`WIDGET`].
    pub fn with_widget<H, R>(&self, handle: H) -> Option<R>
    where
        H: FnOnce() -> R,
    {
        if WIDGET.take_update(UpdateFlags::UPDATE) || self.delivery_list.enter_widget(WIDGET.id()) {
            Some(handle())
        } else {
            None
        }
    }

    /// Copy all delivery from `other` onto `self`.
    pub fn extend(&mut self, other: WidgetUpdates) {
        self.delivery_list.extend_unchecked(other.delivery_list)
    }
}

/// Widget layout updates of the current cycle.
#[derive(Debug, Default)]
pub struct LayoutUpdates {
    pub(crate) delivery_list: UpdateDeliveryList,
}
impl LayoutUpdates {
    /// New with list.
    pub fn new(delivery_list: UpdateDeliveryList) -> Self {
        Self { delivery_list }
    }

    /// Request delivery list.
    pub fn delivery_list(&self) -> &UpdateDeliveryList {
        &self.delivery_list
    }

    /// Request delivery list.
    pub fn delivery_list_mut(&mut self) -> &mut UpdateDeliveryList {
        &mut self.delivery_list
    }

    /// Calls `handle` if layout rebuild was requested for the window.
    pub fn with_window<H, R>(&self, window_id: WindowId, handle: H) -> Option<R>
    where
        H: FnOnce() -> R,
    {
        if self.delivery_list.enter_window(window_id) {
            Some(handle())
        } else {
            None
        }
    }

    /// Copy all delivery from `other` onto `self`.
    pub fn extend(&mut self, other: LayoutUpdates) {
        self.delivery_list.extend_unchecked(other.delivery_list)
    }
}

/// Widget render updates of the current cycle.
#[derive(Debug, Default)]
pub struct RenderUpdates {
    delivery_list: UpdateDeliveryList,
}
impl RenderUpdates {
    /// New with list.
    pub fn new(delivery_list: UpdateDeliveryList) -> Self {
        Self { delivery_list }
    }

    /// Request delivery list.
    pub fn delivery_list(&self) -> &UpdateDeliveryList {
        &self.delivery_list
    }

    /// Request delivery list.
    pub fn delivery_list_mut(&mut self) -> &mut UpdateDeliveryList {
        &mut self.delivery_list
    }

    /// Calls `handle` if render frame rebuild or update was requested for the window.
    pub fn with_window<H, R>(&self, window_id: WindowId, handle: H) -> Option<R>
    where
        H: FnOnce() -> R,
    {
        if self.delivery_list.enter_window(window_id) {
            Some(handle())
        } else {
            None
        }
    }

    /// Copy all delivery from `other` onto `self`.
    pub fn extend(&mut self, other: RenderUpdates) {
        self.delivery_list.extend_unchecked(other.delivery_list)
    }
}

/// Extension methods for infinite loop diagnostics.
///
/// You can also use [`updates_trace_span`] to define a custom scope inside a node, and [`updates_trace_event`]
/// to log a custom entry.
///
/// Note that traces are only recorded if the "inspector" feature is active and a tracing subscriber is installed.
pub trait UpdatesTraceUiNodeExt {
    /// Defines a custom span.
    fn instrument<S: Into<String>>(self, tag: S) -> BoxedUiNode
    where
        Self: Sized;
}
impl<U: UiNode> UpdatesTraceUiNodeExt for U {
    fn instrument<S: Into<String>>(self, tag: S) -> BoxedUiNode {
        #[cfg(inspector)]
        {
            let tag = tag.into();
            self.trace(move |op| UpdatesTrace::custom_span(&tag, op.mtd_name()))
        }
        #[cfg(not(inspector))]
        {
            let _ = tag;
            self.trace(move |_| tracing::Span::none().entered())
        }
    }
}

/// Custom span in the app loop diagnostics.
///
/// See [`UpdatesTraceUiNodeExt`] for more details.
pub fn updates_trace_span(tag: &'static str) -> tracing::span::EnteredSpan {
    UpdatesTrace::custom_span(tag, "")
}

/// Custom log entry in the app loop diagnostics.
///
/// See [`UpdatesTraceUiNodeExt`] for more details.
pub fn updates_trace_event(tag: &str) {
    UpdatesTrace::log_custom(tag)
}

pub(crate) struct UpdatesTrace {
    context: Mutex<UpdateContext>,
    trace: Arc<Mutex<Vec<UpdateTrace>>>,

    widgets_stack: Mutex<Vec<(WidgetId, String)>>,
    node_parents_stack: Mutex<Vec<String>>,
    tags_stack: Mutex<Vec<String>>,
}
impl tracing::subscriber::Subscriber for UpdatesTrace {
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        metadata.target() == Self::UPDATES_TARGET
    }

    fn new_span(&self, span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        let r = match span.metadata().name() {
            "property" | "intrinsic" => {
                let name = visit_str(|v| span.record(v), "name");
                let mut ctx = self.context.lock();

                if let Some(p) = ctx.node_parent.replace(name) {
                    self.node_parents_stack.lock().push(p);
                }
                if let Some(p) = ctx.tag.replace(String::new()) {
                    self.tags_stack.lock().push(p);
                }

                tracing::span::Id::from_u64(1)
            }
            "widget" => {
                let id = visit_u64(|v| span.record(v), "raw_id").unwrap();
                if id == 0 {
                    panic!()
                }
                let id = WidgetId::from_raw(id);

                let name = visit_str(|v| span.record(v), "name");

                let mut ctx = self.context.lock();
                if let Some(p) = ctx.widget.replace((id, name)) {
                    self.widgets_stack.lock().push(p);
                }

                if let Some(p) = ctx.node_parent.replace(String::new()) {
                    self.node_parents_stack.lock().push(p);
                }

                if let Some(p) = ctx.tag.replace(String::new()) {
                    self.tags_stack.lock().push(p);
                }

                tracing::span::Id::from_u64(2)
            }
            "Window" => {
                let id = visit_u64(|v| span.record(v), "raw_id").unwrap() as u32;
                if id == 0 {
                    panic!()
                }
                let id = WindowId::from_raw(id);

                let mut ctx = self.context.lock();
                ctx.window_id = Some(id);

                if let Some(p) = ctx.tag.replace(String::new()) {
                    self.tags_stack.lock().push(p);
                }

                tracing::span::Id::from_u64(3)
            }
            "AppExtension" => {
                let name = visit_str(|v| span.record(v), "name");

                let mut ctx = self.context.lock();
                ctx.app_extension = Some(name);

                if let Some(p) = ctx.tag.replace(String::new()) {
                    self.tags_stack.lock().push(p);
                }

                tracing::span::Id::from_u64(4)
            }
            "tag" => {
                let tag = visit_str(|v| span.record(v), "tag");
                let mut ctx = self.context.lock();
                if let Some(p) = ctx.tag.replace(tag) {
                    self.tags_stack.lock().push(p);
                }
                tracing::span::Id::from_u64(5)
            }
            _ => tracing::span::Id::from_u64(u64::MAX),
        };
        // println!("{}", self.context.lock());
        r
    }

    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}

    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {
        let action = match visit_str(|v| event.record(v), "kind").as_str() {
            "update var" => UpdateAction::Var {
                type_name: visit_str(|v| event.record(v), "type_name"),
            },
            "notify event" => UpdateAction::Event {
                type_name: visit_str(|v| event.record(v), "type_name"),
            },
            "update request" => UpdateAction::Update,
            "layout request" => UpdateAction::Layout,
            "custom" => UpdateAction::Custom {
                tag: visit_str(|v| event.record(v), "tag"),
            },
            _ => return,
        };

        let ctx = self.context.lock().clone();
        // if ctx.app_extension.is_none() {
        //     return;
        // }

        let entry = UpdateTrace { ctx, action };
        self.trace.lock().push(entry);
    }

    fn enter(&self, _span: &tracing::span::Id) {}

    fn exit(&self, span: &tracing::span::Id) {
        let mut ctx = self.context.lock();
        if span == &tracing::span::Id::from_u64(1) {
            ctx.node_parent = self.node_parents_stack.lock().pop();
            ctx.tag = self.tags_stack.lock().pop();
        } else if span == &tracing::span::Id::from_u64(2) {
            ctx.widget = self.widgets_stack.lock().pop();
            ctx.node_parent = self.node_parents_stack.lock().pop();
            ctx.tag = self.tags_stack.lock().pop();
        } else if span == &tracing::span::Id::from_u64(3) {
            ctx.window_id = None;
            ctx.tag = self.tags_stack.lock().pop();
        } else if span == &tracing::span::Id::from_u64(4) {
            ctx.app_extension = None;
            ctx.tag = self.tags_stack.lock().pop();
        } else if span == &tracing::span::Id::from_u64(5) {
            ctx.tag = self.tags_stack.lock().pop();
        }
    }
}
static UPDATES_TRACE_ENABLED: AtomicBool = AtomicBool::new(false);
impl UpdatesTrace {
    const UPDATES_TARGET: &'static str = "zero-ui-updates";

    fn new() -> Self {
        UpdatesTrace {
            context: Mutex::new(UpdateContext::default()),
            trace: Arc::new(Mutex::new(Vec::with_capacity(100))),
            widgets_stack: Mutex::new(Vec::with_capacity(100)),
            node_parents_stack: Mutex::new(Vec::with_capacity(100)),
            tags_stack: Mutex::new(Vec::new()),
        }
    }

    /// If updates trace is currently collecting.
    #[inline(always)]
    pub fn is_tracing() -> bool {
        UPDATES_TRACE_ENABLED.load(atomic::Ordering::Relaxed)
    }

    /// Opens an app extension span.
    pub fn extension_span<E: AppExtension>(ext_mtd: &'static str) -> tracing::span::EnteredSpan {
        if Self::is_tracing() {
            tracing::trace_span!(target: UpdatesTrace::UPDATES_TARGET, "AppExtension", name = pretty_type_name::pretty_type_name::<E>(), %ext_mtd).entered()
        } else {
            tracing::span::Span::none().entered()
        }
    }

    /// Opens a window span.
    pub fn window_span(id: WindowId) -> tracing::span::EnteredSpan {
        if Self::is_tracing() {
            tracing::trace_span!(target: UpdatesTrace::UPDATES_TARGET, "Window", %id, raw_id = id.get() as u64).entered()
        } else {
            tracing::span::Span::none().entered()
        }
    }

    /// Opens a widget span.
    #[cfg(trace_widget)]
    pub fn widget_span(id: WidgetId, name: &'static str, node_mtd: &'static str) -> tracing::span::EnteredSpan {
        if Self::is_tracing() {
            tracing::trace_span!(target: UpdatesTrace::UPDATES_TARGET, "widget", %id, raw_id = id.get(), name, %node_mtd).entered()
        } else {
            tracing::span::Span::none().entered()
        }
    }

    /// Opens a property span.
    #[cfg(trace_wgt_item)]
    pub fn property_span(name: &'static str, node_mtd: &'static str) -> tracing::span::EnteredSpan {
        if Self::is_tracing() {
            tracing::trace_span!(target: UpdatesTrace::UPDATES_TARGET, "property", name, %node_mtd).entered()
        } else {
            tracing::span::Span::none().entered()
        }
    }

    /// Opens an intrinsic span.
    #[cfg(trace_wgt_item)]
    pub fn intrinsic_span(name: &'static str, node_mtd: &'static str) -> tracing::span::EnteredSpan {
        if Self::is_tracing() {
            tracing::trace_span!(target: UpdatesTrace::UPDATES_TARGET, "intrinsic", name, %node_mtd).entered()
        } else {
            tracing::span::Span::none().entered()
        }
    }

    /// Opens a custom named span.
    pub fn custom_span(name: &str, node_mtd: &'static str) -> tracing::span::EnteredSpan {
        if Self::is_tracing() {
            tracing::trace_span!(target: UpdatesTrace::UPDATES_TARGET, "tag", %name, %node_mtd).entered()
        } else {
            tracing::Span::none().entered()
        }
    }

    /// Log a direct update request.
    pub fn log_update() {
        if Self::is_tracing() {
            tracing::event!(target: UpdatesTrace::UPDATES_TARGET, tracing::Level::TRACE, {
                kind = "update request"
            });
        }
    }

    /// Log a direct layout request.
    pub fn log_layout() {
        if Self::is_tracing() {
            tracing::event!(target: UpdatesTrace::UPDATES_TARGET, tracing::Level::TRACE, {
                kind = "layout request"
            });
        }
    }

    /// Log a custom event.
    pub fn log_custom(tag: &str) {
        if Self::is_tracing() {
            tracing::event!(
                target: UpdatesTrace::UPDATES_TARGET,
                tracing::Level::TRACE,
                { kind = "custom", %tag }
            );
        }
    }

    /// Log a var update request.
    pub fn log_var(type_name: &str) {
        if Self::is_tracing() {
            tracing::event!(
                target: UpdatesTrace::UPDATES_TARGET,
                tracing::Level::TRACE,
                { kind = "update var", type_name = pretty_type_name::pretty_type_name_str(type_name) }
            );
        }
    }

    /// Log an event update request.
    pub fn log_event(event: AnyEvent) {
        if Self::is_tracing() {
            tracing::event!(
                target: UpdatesTrace::UPDATES_TARGET,
                tracing::Level::TRACE,
                { kind = "notify event", type_name = event.name() }
            );
        }
    }

    /// Run `action` collecting a trace of what caused updates.
    pub fn collect_trace<R>(trace: &mut Vec<UpdateTrace>, action: impl FnOnce() -> R) -> R {
        let trace_enabled = UPDATES_TRACE_ENABLED.swap(true, atomic::Ordering::Relaxed);

        let tracer = UpdatesTrace::new();
        let result = Arc::clone(&tracer.trace);
        let r = tracing::subscriber::with_default(tracer, action);
        trace.extend(Arc::try_unwrap(result).unwrap().into_inner());

        UPDATES_TRACE_ENABLED.store(trace_enabled, atomic::Ordering::Relaxed);

        r
    }

    /// Displays the top 20 most frequent update sources in the trace.
    pub fn format_trace(trace: Vec<UpdateTrace>) -> String {
        let mut frequencies = HashMap::with_capacity(50);
        for t in trace {
            match frequencies.entry(t) {
                hash_map::Entry::Vacant(e) => {
                    e.insert(1);
                }
                hash_map::Entry::Occupied(mut e) => {
                    *e.get_mut() += 1;
                }
            }
        }
        let mut frequencies: Vec<_> = frequencies.into_iter().collect();
        frequencies.sort_by_key(|(_, c)| -c);

        let mut trace = String::new();
        for (t, c) in frequencies.into_iter().take(20) {
            use std::fmt::Write;
            let _ = writeln!(&mut trace, "{t} ({c} times)");
        }
        trace
    }
}
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
struct UpdateContext {
    app_extension: Option<String>,
    window_id: Option<WindowId>,
    widget: Option<(WidgetId, String)>,
    node_parent: Option<String>,
    tag: Option<String>,
}
impl fmt::Display for UpdateContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(e) = &self.app_extension {
            write!(f, "{}", e.rsplit("::").next().unwrap())?;
        } else {
            write!(f, "<unknown>")?;
        }
        if let Some(w) = self.window_id {
            write!(f, "//{w}")?;
        }
        if let Some((id, name)) = &self.widget {
            write!(f, "/../{name}#{id}")?;
        }
        if let Some(p) = &self.node_parent {
            if !p.is_empty() {
                write!(f, "//{p}")?;
            }
        }
        if let Some(t) = &self.tag {
            if !t.is_empty() {
                write!(f, "//{t}")?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) struct UpdateTrace {
    ctx: UpdateContext,
    action: UpdateAction,
}
impl fmt::Display for UpdateTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.ctx, self.action)
    }
}
#[derive(Debug, PartialEq, Eq, Hash)]
enum UpdateAction {
    Update,
    Layout,
    Var { type_name: String },
    Event { type_name: String },
    Custom { tag: String },
}
impl fmt::Display for UpdateAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UpdateAction::Update => write!(f, "update"),
            UpdateAction::Layout => write!(f, "layout"),
            UpdateAction::Var { type_name } => write!(f, "update var of type {type_name}"),
            UpdateAction::Event { type_name } => write!(f, "update event {type_name}"),
            UpdateAction::Custom { tag } => write!(f, "{tag}"),
        }
    }
}

fn visit_str(record: impl FnOnce(&mut dyn tracing::field::Visit), name: &str) -> String {
    struct Visitor<'a> {
        name: &'a str,
        result: String,
    }
    impl<'a> tracing::field::Visit for Visitor<'a> {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            if field.name() == self.name {
                self.result = format!("{value:?}");
            }
        }
        fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
            if field.name() == self.name {
                self.result = value.to_owned();
            }
        }
    }

    let mut visitor = Visitor {
        name,
        result: String::new(),
    };
    record(&mut visitor);
    visitor.result
}
fn visit_u64(record: impl FnOnce(&mut dyn tracing::field::Visit), name: &str) -> Option<u64> {
    struct Visitor<'a> {
        name: &'a str,
        result: Option<u64>,
    }
    impl<'a> tracing::field::Visit for Visitor<'a> {
        fn record_debug(&mut self, _field: &tracing::field::Field, _value: &dyn std::fmt::Debug) {}
        fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
            if field.name() == self.name {
                self.result = Some(value)
            }
        }
    }

    let mut visitor = Visitor { name, result: None };
    record(&mut visitor);
    visitor.result
}

/// Update pump and schedule service.
pub struct UPDATES;
impl UPDATES {
    pub(crate) fn init(&self, event_sender: AppEventSender) {
        UPDATES_SV.write().event_sender = Some(event_sender);
    }

    #[must_use]
    #[cfg(any(test, doc, feature = "test_util"))]
    pub(crate) fn apply(&self) -> ContextUpdates {
        self.apply_updates() | self.apply_info() | self.apply_layout_render()
    }

    #[must_use]
    pub(crate) fn apply_updates(&self) -> ContextUpdates {
        let events = EVENTS.apply_updates();
        VARS_APP.apply_updates();

        let (update, update_widgets) = UPDATES.take_update();

        ContextUpdates {
            events,
            update,
            update_widgets,
            info: false,
            info_widgets: InfoUpdates::default(),
            layout: false,
            layout_widgets: LayoutUpdates::default(),
            render: false,
            render_widgets: RenderUpdates::default(),
            render_update_widgets: RenderUpdates::default(),
        }
    }
    #[must_use]
    pub(crate) fn apply_info(&self) -> ContextUpdates {
        let (info, info_widgets) = UPDATES.take_info();

        ContextUpdates {
            events: vec![],
            update: false,
            update_widgets: WidgetUpdates::default(),
            info,
            info_widgets,
            layout: false,
            layout_widgets: LayoutUpdates::default(),
            render: false,
            render_widgets: RenderUpdates::default(),
            render_update_widgets: RenderUpdates::default(),
        }
    }
    #[must_use]
    pub(crate) fn apply_layout_render(&self) -> ContextUpdates {
        let (layout, layout_widgets) = UPDATES.take_layout();
        let (render, render_widgets, render_update_widgets) = UPDATES.take_render();

        ContextUpdates {
            events: vec![],
            update: false,
            update_widgets: WidgetUpdates::default(),
            info: false,
            info_widgets: InfoUpdates::default(),
            layout,
            layout_widgets,
            render,
            render_widgets,
            render_update_widgets,
        }
    }

    pub(crate) fn on_app_awake(&self) {
        UPDATES_SV.write().app_awake(true);
    }

    pub(crate) fn on_app_sleep(&self) {
        UPDATES_SV.write().app_awake(false);
    }

    /// Returns next timer or animation tick time.
    pub(crate) fn next_deadline(&self, timer: &mut LoopTimer) {
        TIMERS_SV.write().next_deadline(timer);
        VARS_APP.next_deadline(timer);
    }

    /// Update timers and animations, returns next wake time.
    pub(crate) fn update_timers(&self, timer: &mut LoopTimer) {
        TIMERS_SV.write().apply_updates(timer);
        VARS_APP.update_animations(timer);
    }

    /// If a call to `apply_updates` will generate updates (ignoring timers).
    #[must_use]
    pub(crate) fn has_pending_updates(&self) -> bool {
        UPDATES_SV.read().update_ext.intersects(UpdateFlags::UPDATE | UpdateFlags::INFO)
            || VARS_APP.has_pending_updates()
            || EVENTS_SV.write().has_pending_updates()
            || TIMERS_SV.read().has_pending_updates()
    }

    #[must_use]
    pub(crate) fn has_pending_layout_or_render(&self) -> bool {
        UPDATES_SV
            .read()
            .update_ext
            .intersects(UpdateFlags::LAYOUT | UpdateFlags::RENDER | UpdateFlags::RENDER_UPDATE)
    }

    /// Create an [`AppEventSender`] that can be used to awake the app and send app events from threads outside of the app.
    pub fn sender(&self) -> AppEventSender {
        UPDATES_SV.read().event_sender.as_ref().unwrap().clone()
    }

    /// Create an std task waker that wakes the event loop and updates.
    pub fn waker(&self, target: impl Into<Option<WidgetId>>) -> Waker {
        UPDATES_SV.read().event_sender.as_ref().unwrap().waker(target)
    }

    pub(crate) fn update_flags_root(&self, flags: UpdateFlags, window_id: WindowId, root_id: WidgetId) {
        if flags.is_empty() {
            return;
        }

        let mut u = UPDATES_SV.write();
        if flags.contains(UpdateFlags::UPDATE) {
            u.update_widgets.insert_updates_root(window_id, root_id);
        }
        if flags.contains(UpdateFlags::INFO) {
            u.info_widgets.insert_updates_root(window_id, root_id);
        }
        if flags.contains(UpdateFlags::LAYOUT) {
            u.layout_widgets.insert_updates_root(window_id, root_id);
        }

        if flags.contains(UpdateFlags::RENDER) {
            u.render_widgets.insert_updates_root(window_id, root_id);
        } else if flags.contains(UpdateFlags::RENDER_UPDATE) {
            u.render_update_widgets.insert_updates_root(window_id, root_id);
        }

        u.update_ext |= flags;
    }

    pub(crate) fn update_flags(&self, flags: UpdateFlags, target: impl Into<Option<WidgetId>>) {
        if flags.is_empty() {
            return;
        }

        let mut u = UPDATES_SV.write();

        if let Some(id) = target.into() {
            if flags.contains(UpdateFlags::UPDATE) {
                u.update_widgets.search_widget(id);
            }
            if flags.contains(UpdateFlags::INFO) {
                u.info_widgets.search_widget(id);
            }
            if flags.contains(UpdateFlags::LAYOUT) {
                u.layout_widgets.search_widget(id);
            }

            if flags.contains(UpdateFlags::RENDER) {
                u.render_widgets.search_widget(id);
            } else if flags.contains(UpdateFlags::RENDER_UPDATE) {
                u.render_update_widgets.search_widget(id);
            }
        }

        u.update_ext |= flags;
    }

    /// Schedules an [`UpdateOp`] that optionally affects the `target` widget.
    pub fn update_op(&self, op: UpdateOp, target: impl Into<Option<WidgetId>>) -> &Self {
        let target = target.into();
        match op {
            UpdateOp::Update => self.update(target),
            UpdateOp::Info => self.update_info(target),
            UpdateOp::Layout => self.layout(target),
            UpdateOp::Render => self.render(target),
            UpdateOp::RenderUpdate => self.render_update(target),
        }
    }

    /// Schedules an [`UpdateOp`] for the window only.
    pub fn update_op_window(&self, op: UpdateOp, target: WindowId) -> &Self {
        match op {
            UpdateOp::Update => self.update_window(target),
            UpdateOp::Info => self.update_info_window(target),
            UpdateOp::Layout => self.layout_window(target),
            UpdateOp::Render => self.render_window(target),
            UpdateOp::RenderUpdate => self.render_update_window(target),
        }
    }

    /// Schedules an update that affects the `target`.
    ///
    /// After the current update cycle ends a new update will happen that includes the `target` widget.
    pub fn update(&self, target: impl Into<Option<WidgetId>>) -> &Self {
        UpdatesTrace::log_update();
        self.update_internal(target.into())
    }
    /// Implements `update` without `log_update`.
    pub(crate) fn update_internal(&self, target: Option<WidgetId>) -> &UPDATES {
        let mut u = UPDATES_SV.write();
        u.update_ext.insert(UpdateFlags::UPDATE);
        u.send_awake();
        if let Some(id) = target {
            u.update_widgets.search_widget(id);
        }
        self
    }

    /// Schedules an update for the window only.
    pub fn update_window(&self, target: WindowId) -> &Self {
        let mut u = UPDATES_SV.write();
        u.update_ext.insert(UpdateFlags::UPDATE);
        u.send_awake();
        u.update_widgets.insert_window(target);
        self
    }

    pub(crate) fn send_awake(&self) {
        UPDATES_SV.write().send_awake();
    }

    /// Schedules an info rebuild that affects the `target`.
    ///
    /// After the current update cycle ends a new update will happen that requests an info rebuild that includes the `target` widget.
    pub fn update_info(&self, target: impl Into<Option<WidgetId>>) -> &Self {
        let mut u = UPDATES_SV.write();
        u.update_ext.insert(UpdateFlags::INFO);
        u.send_awake();
        if let Some(id) = target.into() {
            u.info_widgets.search_widget(id);
        }
        self
    }

    /// Schedules an info rebuild for the window only.
    pub fn update_info_window(&self, target: WindowId) -> &Self {
        let mut u = UPDATES_SV.write();
        u.update_ext.insert(UpdateFlags::INFO);
        u.send_awake();
        u.info_widgets.insert_window(target);
        self
    }

    /// Schedules a layout update that affects the `target`.
    ///
    /// After the current update cycle ends and there are no more updates requested a layout pass is issued that includes the `target` widget.
    pub fn layout(&self, target: impl Into<Option<WidgetId>>) -> &Self {
        UpdatesTrace::log_layout();
        let mut u = UPDATES_SV.write();
        u.update_ext.insert(UpdateFlags::LAYOUT);
        u.send_awake();
        if let Some(id) = target.into() {
            u.layout_widgets.search_widget(id);
        }
        self
    }

    /// Schedules a layout update for the window only.
    pub fn layout_window(&self, target: WindowId) -> &Self {
        let mut u = UPDATES_SV.write();
        u.update_ext.insert(UpdateFlags::LAYOUT);
        u.send_awake();
        u.layout_widgets.insert_window(target);
        self
    }

    /// Schedules a full render that affects the `target`.
    ///
    /// After the current update cycle ends and there are no more updates or layouts requested a render pass is issued that
    /// includes the `target` widget.
    ///
    /// If no `target` is provided only the app extensions receive a render request.
    pub fn render(&self, target: impl Into<Option<WidgetId>>) -> &Self {
        let mut u = UPDATES_SV.write();
        u.update_ext.insert(UpdateFlags::RENDER);
        u.send_awake();
        if let Some(id) = target.into() {
            u.render_widgets.search_widget(id);
        }
        self
    }

    /// Schedules a new frame for the window only.
    pub fn render_window(&self, target: WindowId) -> &Self {
        let mut u = UPDATES_SV.write();
        u.update_ext.insert(UpdateFlags::RENDER);
        u.send_awake();
        u.render_widgets.insert_window(target);
        self
    }

    /// Schedules a render update that affects the `target`.
    ///
    /// After the current update cycle ends and there are no more updates or layouts requested a render pass is issued that
    /// includes the `target` widget marked for render update only. Note that if a full render was requested for another widget
    /// on the same window this request is upgraded to a full frame render.
    pub fn render_update(&self, target: impl Into<Option<WidgetId>>) -> &Self {
        let mut u = UPDATES_SV.write();
        u.update_ext.insert(UpdateFlags::RENDER_UPDATE);
        u.send_awake();
        if let Some(id) = target.into() {
            u.render_update_widgets.search_widget(id);
        }
        self
    }

    /// Schedules a render update for the window only.
    pub fn render_update_window(&self, target: WindowId) -> &Self {
        let mut u = UPDATES_SV.write();
        u.update_ext.insert(UpdateFlags::RENDER_UPDATE);
        u.send_awake();
        u.render_update_widgets.insert_window(target);
        self
    }

    /// Returns `true` is render or render update is requested for the window.
    pub fn is_pending_render(&self, window_id: WindowId) -> bool {
        let u = UPDATES_SV.read();
        u.render_widgets.enter_window(window_id) || u.render_update_widgets.enter_window(window_id)
    }

    /// Schedule the `future` to run in the app context, each future awake work runs as a *preview* update.
    ///
    /// Returns a handle that can be dropped to cancel execution.
    pub fn run<F: std::future::Future<Output = ()> + Send + 'static>(&self, future: F) -> OnUpdateHandle {
        self.run_hn_once(async_app_hn_once!(|_| future.await))
    }

    /// Schedule an *once* handler to run when these updates are applied.
    ///
    /// The callback is any of the *once* [`AppHandler`], including async handlers. If the handler is async and does not finish in
    /// one call it is scheduled to update in *preview* updates.
    pub fn run_hn_once<H: AppHandler<UpdateArgs>>(&self, handler: H) -> OnUpdateHandle {
        let mut u = UPDATES_SV.write();
        u.update_ext.insert(UpdateFlags::UPDATE);
        u.send_awake();
        Self::push_handler(u.pos_handlers.get_mut(), true, handler, true)
    }

    /// Create a preview update handler.
    ///
    /// The `handler` is called every time the app updates, just before the UI updates. It can be any of the non-async [`AppHandler`],
    /// use the [`app_hn!`] or [`app_hn_once!`] macros to declare the closure. You must avoid using async handlers because UI bound async
    /// tasks cause app updates to awake, so it is very easy to lock the app in a constant sequence of updates. You can use [`run`](Self::run)
    /// to start an async app context task.
    ///
    /// Returns an [`OnUpdateHandle`] that can be used to unsubscribe, you can also unsubscribe from inside the handler by calling
    /// [`unsubscribe`](crate::handler::AppWeakHandle::unsubscribe) in the third parameter of [`app_hn!`] or [`async_app_hn!`].
    ///
    /// [`app_hn_once!`]: macro@crate::handler::app_hn_once
    /// [`app_hn!`]: macro@crate::handler::app_hn
    /// [`async_app_hn!`]: macro@crate::handler::async_app_hn
    pub fn on_pre_update<H>(&self, handler: H) -> OnUpdateHandle
    where
        H: AppHandler<UpdateArgs>,
    {
        let u = UPDATES_SV.read();
        let r = Self::push_handler(&mut u.pre_handlers.lock(), true, handler, false);
        r
    }

    /// Create an update handler.
    ///
    /// The `handler` is called every time the app updates, just after the UI updates. It can be any of the non-async [`AppHandler`],
    /// use the [`app_hn!`] or [`app_hn_once!`] macros to declare the closure. You must avoid using async handlers because UI bound async
    /// tasks cause app updates to awake, so it is very easy to lock the app in a constant sequence of updates. You can use [`run`](Self::run)
    /// to start an async app context task.
    ///
    /// Returns an [`OnUpdateHandle`] that can be used to unsubscribe, you can also unsubscribe from inside the handler by calling
    /// [`unsubscribe`](crate::handler::AppWeakHandle::unsubscribe) in the third parameter of [`app_hn!`] or [`async_app_hn!`].
    ///
    /// [`app_hn!`]: macro@crate::handler::app_hn
    /// [`app_hn_once!`]: macro@crate::handler::app_hn_once
    /// [`async_app_hn!`]: macro@crate::handler::async_app_hn
    pub fn on_update<H>(&self, handler: H) -> OnUpdateHandle
    where
        H: AppHandler<UpdateArgs>,
    {
        let u = UPDATES_SV.read();
        let r = Self::push_handler(&mut u.pos_handlers.lock(), false, handler, false);
        r
    }

    fn push_handler<H>(entries: &mut Vec<UpdateHandler>, is_preview: bool, mut handler: H, force_once: bool) -> OnUpdateHandle
    where
        H: AppHandler<UpdateArgs>,
    {
        let (handle_owner, handle) = OnUpdateHandle::new();
        entries.push(UpdateHandler {
            handle: handle_owner,
            count: 0,
            handler: Box::new(move |args, handle| {
                let handler_args = AppHandlerArgs { handle, is_preview };
                handler.event(args, &handler_args);
                if force_once {
                    handler_args.handle.unsubscribe();
                }
            }),
        });
        handle
    }

    pub(crate) fn on_pre_updates(&self) {
        let _s = tracing::trace_span!("UPDATES.on_pre_updates");
        let mut handlers = mem::take(UPDATES_SV.write().pre_handlers.get_mut());
        Self::retain_updates(&mut handlers);

        let mut u = UPDATES_SV.write();
        handlers.append(u.pre_handlers.get_mut());
        *u.pre_handlers.get_mut() = handlers;
    }

    pub(crate) fn on_updates(&self) {
        let _s = tracing::trace_span!("UPDATES.on_updates");
        let mut handlers = mem::take(UPDATES_SV.write().pos_handlers.get_mut());
        Self::retain_updates(&mut handlers);

        let mut u = UPDATES_SV.write();
        handlers.append(u.pos_handlers.get_mut());
        *u.pos_handlers.get_mut() = handlers;
    }

    fn retain_updates(handlers: &mut Vec<UpdateHandler>) {
        handlers.retain_mut(|e| {
            !e.handle.is_dropped() && {
                e.count = e.count.wrapping_add(1);
                (e.handler)(&UpdateArgs { count: e.count }, &e.handle.weak_handle());
                !e.handle.is_dropped()
            }
        });
    }

    /// Returns (update_ext, update_widgets)
    pub(super) fn take_update(&self) -> (bool, WidgetUpdates) {
        let mut u = UPDATES_SV.write();

        let ext = u.update_ext.contains(UpdateFlags::UPDATE);
        u.update_ext.remove(UpdateFlags::UPDATE);

        (
            ext,
            WidgetUpdates {
                delivery_list: mem::take(&mut u.update_widgets),
            },
        )
    }

    /// Returns (info_ext, info_widgets)
    pub(super) fn take_info(&self) -> (bool, InfoUpdates) {
        let mut u = UPDATES_SV.write();

        let ext = u.update_ext.contains(UpdateFlags::INFO);
        u.update_ext.remove(UpdateFlags::INFO);

        (
            ext,
            InfoUpdates {
                delivery_list: mem::take(&mut u.info_widgets),
            },
        )
    }

    /// Returns (layout_ext, layout_widgets)
    pub(super) fn take_layout(&self) -> (bool, LayoutUpdates) {
        let mut u = UPDATES_SV.write();

        let ext = u.update_ext.contains(UpdateFlags::LAYOUT);
        u.update_ext.remove(UpdateFlags::LAYOUT);

        (
            ext,
            LayoutUpdates {
                delivery_list: mem::take(&mut u.layout_widgets),
            },
        )
    }

    /// Returns (render_ext, render_widgets, render_update_widgets)
    pub(super) fn take_render(&self) -> (bool, RenderUpdates, RenderUpdates) {
        let mut u = UPDATES_SV.write();

        let ext = u.update_ext.intersects(UpdateFlags::RENDER | UpdateFlags::RENDER_UPDATE);
        u.update_ext.remove(UpdateFlags::RENDER | UpdateFlags::RENDER_UPDATE);

        (
            ext,
            RenderUpdates {
                delivery_list: mem::take(&mut u.render_widgets),
            },
            RenderUpdates {
                delivery_list: mem::take(&mut u.render_update_widgets),
            },
        )
    }

    pub(crate) fn handler_lens(&self) -> (usize, usize) {
        let u = UPDATES_SV.read();
        let r = (u.pre_handlers.lock().len(), u.pos_handlers.lock().len());
        r
    }
    pub(crate) fn new_update_handlers(&self, pre_from: usize, pos_from: usize) -> Vec<Box<dyn Fn() -> bool>> {
        let u = UPDATES_SV.read();
        let r = u
            .pre_handlers
            .lock()
            .iter()
            .skip(pre_from)
            .chain(u.pos_handlers.lock().iter().skip(pos_from))
            .map(|h| h.handle.weak_handle())
            .map(|h| {
                let r: Box<dyn Fn() -> bool> = Box::new(move || h.upgrade().is_some());
                r
            })
            .collect();
        r
    }
}

app_local! {
    static UPDATES_SV: UpdatesService = UpdatesService::new();
}
struct UpdatesService {
    event_sender: Option<AppEventSender>,

    update_ext: UpdateFlags,
    update_widgets: UpdateDeliveryList,
    info_widgets: UpdateDeliveryList,
    layout_widgets: UpdateDeliveryList,
    render_widgets: UpdateDeliveryList,
    render_update_widgets: UpdateDeliveryList,

    pre_handlers: Mutex<Vec<UpdateHandler>>,
    pos_handlers: Mutex<Vec<UpdateHandler>>,

    app_is_awake: bool,
    awake_pending: bool,
}
impl UpdatesService {
    fn new() -> Self {
        Self {
            event_sender: None,
            update_ext: UpdateFlags::empty(),
            update_widgets: UpdateDeliveryList::new_any(),
            info_widgets: UpdateDeliveryList::new_any(),
            layout_widgets: UpdateDeliveryList::new_any(),
            render_widgets: UpdateDeliveryList::new_any(),
            render_update_widgets: UpdateDeliveryList::new_any(),

            pre_handlers: Mutex::new(vec![]),
            pos_handlers: Mutex::new(vec![]),

            app_is_awake: false,
            awake_pending: false,
        }
    }

    fn send_awake(&mut self) {
        if !self.app_is_awake && !self.awake_pending {
            self.awake_pending = true;
            match self.event_sender.as_ref() {
                Some(s) => {
                    if let Err(AppDisconnected(())) = s.send_check_update() {
                        tracing::error!("no app connected to update");
                    }
                }
                None => {
                    tracing::error!("no app connected yet to update");
                }
            }
        }
    }

    fn app_awake(&mut self, wake: bool) {
        self.awake_pending = false;
        self.app_is_awake = wake;
    }
}

/// Updates that must be reacted by an app owner.
#[derive(Debug, Default)]
pub struct ContextUpdates {
    /// Events to notify.
    ///
    /// When this is not empty [`update`](Self::update) is `true`.
    pub events: Vec<EventUpdate>,

    /// Update requested.
    ///
    /// When this is `true`, [`update_widgets`](Self::update_widgets)
    /// may contain widgets, if not then only app extensions must update.
    pub update: bool,

    /// Info rebuild requested.
    ///
    /// When this is `true`, [`info_widgets`](Self::info_widgets)
    /// may contain widgets, if not then only app extensions must update.
    pub info: bool,

    /// Layout requested.
    ///
    /// When this is `true`, [`layout_widgets`](Self::layout_widgets)
    /// may contain widgets, if not then only app extensions must update.
    pub layout: bool,

    /// Render requested.
    ///
    /// When this is `true`, [`render_widgets`](Self::render_widgets) or [`render_update_widgets`](Self::render_update_widgets)
    /// may contain widgets, if not then only app extensions must update.
    pub render: bool,

    /// Update targets.
    ///
    /// When this is not empty [`update`](Self::update) is `true`.
    pub update_widgets: WidgetUpdates,

    /// Info rebuild targets.
    ///
    /// When this is not empty [`info`](Self::info) is `true`.
    pub info_widgets: InfoUpdates,

    /// Layout targets.
    ///
    /// When this is not empty [`layout`](Self::layout) is `true`.
    pub layout_widgets: LayoutUpdates,

    /// Full render targets.
    ///
    /// When this is not empty [`render`](Self::render) is `true`.
    pub render_widgets: RenderUpdates,

    /// Render update targets.
    ///
    /// When this is not empty [`render`](Self::render) is `true`.
    pub render_update_widgets: RenderUpdates,
}
impl ContextUpdates {
    /// If has events, update, layout or render was requested.
    pub fn has_updates(&self) -> bool {
        !self.events.is_empty() || self.update || self.info || self.layout || self.render
    }
}
impl std::ops::BitOrAssign for ContextUpdates {
    fn bitor_assign(&mut self, rhs: Self) {
        self.events.extend(rhs.events);
        self.update |= rhs.update;
        self.update_widgets.extend(rhs.update_widgets);
        self.info |= rhs.info;
        self.info_widgets.extend(rhs.info_widgets);
        self.layout |= rhs.layout;
        self.layout_widgets.extend(rhs.layout_widgets);
        self.render |= rhs.render;
        self.render_widgets.extend(rhs.render_widgets);
        self.render_update_widgets.extend(rhs.render_update_widgets);
    }
}
impl std::ops::BitOr for ContextUpdates {
    type Output = Self;

    fn bitor(mut self, rhs: Self) -> Self {
        self |= rhs;
        self
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, bytemuck::NoUninit)]
    #[repr(transparent)]
    pub(crate) struct UpdateFlags: u8 {
        const REINIT =        0b1000_0000;
        const INFO =          0b0001_0000;
        const UPDATE =        0b0000_0001;
        const LAYOUT =        0b0000_0010;
        const RENDER =        0b0000_0100;
        const RENDER_UPDATE = 0b0000_1000;
    }
}

/// Represents an [`on_pre_update`](UPDATES::on_pre_update) or [`on_update`](UPDATES::on_update) handler.
///
/// Drop all clones of this handle to drop the binding, or call [`perm`](Self::perm) to drop the handle
/// but keep the handler alive for the duration of the app.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[repr(transparent)]
#[must_use = "dropping the handle unsubscribes update handler"]
pub struct OnUpdateHandle(Handle<()>);
impl OnUpdateHandle {
    fn new() -> (HandleOwner<()>, OnUpdateHandle) {
        let (owner, handle) = Handle::new(());
        (owner, OnUpdateHandle(handle))
    }

    /// Create a handle to nothing, the handle always in the *unsubscribed* state.
    ///
    /// Note that `Option<OnUpdateHandle>` takes up the same space as `OnUpdateHandle` and avoids an allocation.
    pub fn dummy() -> Self {
        OnUpdateHandle(Handle::dummy(()))
    }

    /// Drop the handle but does **not** unsubscribe.
    ///
    /// The handler stays in memory for the duration of the app or until another handle calls [`unsubscribe`](Self::unsubscribe.)
    pub fn perm(self) {
        self.0.perm();
    }

    /// If another handle has called [`perm`](Self::perm).
    /// If `true` the var binding will stay active until the app exits, unless [`unsubscribe`](Self::unsubscribe) is called.
    pub fn is_permanent(&self) -> bool {
        self.0.is_permanent()
    }

    /// Drops the handle and forces the handler to drop.
    pub fn unsubscribe(self) {
        self.0.force_drop()
    }

    /// If another handle has called [`unsubscribe`](Self::unsubscribe).
    ///
    /// The handler is already dropped or will be dropped in the next app update, this is irreversible.
    pub fn is_unsubscribed(&self) -> bool {
        self.0.is_dropped()
    }

    /// Create a weak handle.
    pub fn downgrade(&self) -> WeakOnUpdateHandle {
        WeakOnUpdateHandle(self.0.downgrade())
    }
}

/// Weak [`OnUpdateHandle`].
#[derive(Clone, PartialEq, Eq, Hash, Default, Debug)]
pub struct WeakOnUpdateHandle(WeakHandle<()>);
impl WeakOnUpdateHandle {
    /// New weak handle that does not upgrade.
    pub fn new() -> Self {
        Self(WeakHandle::new())
    }

    /// Gets the strong handle if it is still subscribed.
    pub fn upgrade(&self) -> Option<OnUpdateHandle> {
        self.0.upgrade().map(OnUpdateHandle)
    }
}

/// Specify what app extension and widget operation must be run to satisfy an update request targeting an widget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateOp {
    /// The [`AppExtension::update_preview`], [`AppExtension::update_ui`] and [`AppExtension::update`] are called in order,
    /// this is a normal update cycle.
    ///
    /// The [`UiNode::update`] is called for the target widget, parent widgets and any other widget that requested update
    /// in the same cycle. This call happens inside [`AppExtension::update_ui`].
    ///
    /// [`AppExtension::update_preview`]: crate::AppExtension::update_preview
    /// [`AppExtension::update_ui`]: crate::AppExtension::update_ui
    /// [`AppExtension::update`]: crate::AppExtension::update
    Update,
    /// The normal [`Update`] cycle runs, and after the info tree of windows that inited or deinited widgets are rebuild
    /// by calling [`UiNode::info`].  The target widget is also flagged for rebuild.
    ///
    /// [`Update`]: UpdateOp::Render
    Info,
    /// The [`AppExtension::layout`] is called the an update cycle happens without generating anymore update requests.
    ///
    /// The [`UiNode::layout`] is called for the widget target, parent widgets and any other widget that depends on
    /// layout metrics that have changed or that also requested layout update.
    ///
    /// [`AppExtension::layout`]: crate::AppExtension::layout
    Layout,
    /// The [`AppExtension::render`] is called after an update and layout cycle happens generating anymore requests for update or layout.
    ///
    /// The [`UiNode::render`] is called for the target widget, parent widgets and all other widgets that also requested render
    /// or that requested [`RenderUpdate`] in the same window.
    ///
    /// [`RenderUpdate`]: UpdateOp::RenderUpdate
    /// [`AppExtension::render`]: crate::AppExtension::render
    Render,
    /// Same behavior as [`Render`], except that windows where all widgets only requested render update are rendered
    /// using [`UiNode::render_update`] instead of the full render.
    ///
    /// This OP is upgraded to [`Render`] if any other widget requests a full render in the same window.
    ///
    /// [`Render`]: UpdateOp::Render
    RenderUpdate,
}

/// Arguments for an [`on_pre_update`](UPDATES::on_pre_update), [`on_update`](UPDATES::on_update) or [`run`](UPDATES::run) handler.
#[derive(Debug, Clone, Copy)]
pub struct UpdateArgs {
    /// Number of times the handler was called.
    pub count: usize,
}

struct UpdateHandler {
    handle: HandleOwner<()>,
    count: usize,
    handler: Box<dyn FnMut(&UpdateArgs, &dyn AppWeakHandle) + Send>,
}
