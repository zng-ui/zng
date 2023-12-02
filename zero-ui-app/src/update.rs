//! App updates API.

use std::{
    collections::{hash_map, HashMap},
    fmt, mem,
    sync::Arc,
};

use parking_lot::Mutex;
use zero_ui_unique_id::IdSet;

use crate::{
    event::{AnyEvent, AnyEventArgs},
    AppExtension, WidgetId, WindowId,
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
            if any || self.subscribers.contains(w.id()) {
                any = true;
                self.widgets.insert(w.id());
            }
        }
        if any {
            self.windows.insert(wgt.tree().window_id());
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
    pub fn fulfill_search<'a, 'b, P>(&'a mut self, windows: impl Iterator<Item = &'b P>)
    where
        P: WidgetSearchProvider,
    {
        for window in windows {
            self.search.retain(|w| {
                if let Some(w) = window.search_widget(*w) {
                    for w in w.widget_and_ancestors() {
                        self.widgets.insert(w.id());
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
    type WidgetIter: Iterator<Item = WidgetId>;

    /// The window parent.
    fn window_id(&self) -> WindowId;
    /// Iterate over the widget, parent, grandparent, .., root.
    fn widget_and_ancestors(&self) -> Self::WidgetIter;
}

/// Provides a query API on all widgets of a window.
pub trait WidgetSearchProvider {
    /// Found widget type.
    type Result: WidgetPathProvider;

    /// Search widget.
    fn search_widget(&self, id: WidgetId) -> Option<&Self::Result>;
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

    /// Calls `handle` if the event targets the widget and propagation is not stopped.
    pub fn with_widget<H, R>(&self, widget_id: WidgetId, handle: H) -> Option<R>
    where
        H: FnOnce() -> R,
    {
        if self.delivery_list.enter_widget(widget_id) {
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
    delivery_list: UpdateDeliveryList,
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

    /// Calls `handle` if update was requested for the window.
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

    /// Calls `handle` if update was requested for the widget.
    pub fn with_widget<H, R>(&self, widget_id: WidgetId, handle: H) -> Option<R>
    where
        H: FnOnce() -> R,
    {
        if self.delivery_list.enter_widget(widget_id) {
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
    delivery_list: UpdateDeliveryList,
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

    /// Opens an app extension span.
    pub fn extension_span<E: AppExtension>(ext_mtd: &'static str) -> tracing::span::EnteredSpan {
        tracing::trace_span!(target: UpdatesTrace::UPDATES_TARGET, "AppExtension", name = pretty_type_name::pretty_type_name::<E>(), %ext_mtd).entered()
    }

    /// Opens a window span.
    pub fn window_span(id: WindowId) -> tracing::span::EnteredSpan {
        tracing::trace_span!(target: UpdatesTrace::UPDATES_TARGET, "Window", %id, raw_id = id.get() as u64).entered()
    }

    /// Opens a widget span.
    #[cfg(trace_widget)]
    pub fn widget_span(id: WidgetId, name: &'static str, node_mtd: &'static str) -> tracing::span::EnteredSpan {
        tracing::trace_span!(target: UpdatesTrace::UPDATES_TARGET, "widget", %id, raw_id = id.get(), name, %node_mtd).entered()
    }

    /// Opens a property span.
    #[cfg(trace_wgt_item)]
    pub fn property_span(name: &'static str, node_mtd: &'static str) -> tracing::span::EnteredSpan {
        tracing::trace_span!(target: UpdatesTrace::UPDATES_TARGET, "property", name, %node_mtd).entered()
    }

    /// Opens an intrinsic span.
    #[cfg(trace_wgt_item)]
    pub fn intrinsic_span(name: &'static str, node_mtd: &'static str) -> tracing::span::EnteredSpan {
        tracing::trace_span!(target: UpdatesTrace::UPDATES_TARGET, "intrinsic", name, %node_mtd).entered()
    }

    /// Opens a custom named span.
    pub fn custom_span(name: &str, node_mtd: &'static str) -> tracing::span::EnteredSpan {
        #[cfg(inspector)]
        {
            tracing::trace_span!(target: UpdatesTrace::UPDATES_TARGET, "tag", %name, %node_mtd).entered()
        }
        #[cfg(not(inspector))]
        {
            let _ = (name, node_mtd);
            tracing::Span::none().entered()
        }
    }

    /// Log a direct update request.
    pub fn log_update() {
        tracing::event!(target: UpdatesTrace::UPDATES_TARGET, tracing::Level::TRACE, {
            kind = "update request"
        });
    }

    /// Log a direct layout request.
    pub fn log_layout() {
        tracing::event!(target: UpdatesTrace::UPDATES_TARGET, tracing::Level::TRACE, {
            kind = "layout request"
        });
    }

    /// Log a custom event.
    pub fn log_custom(tag: &str) {
        tracing::event!(
            target: UpdatesTrace::UPDATES_TARGET,
            tracing::Level::TRACE,
            { kind = "custom", %tag }
        );
    }

    /// Log a var update request.
    pub fn log_var(type_name: &str) {
        tracing::event!(
            target: UpdatesTrace::UPDATES_TARGET,
            tracing::Level::TRACE,
            { kind = "update var", type_name = pretty_type_name::pretty_type_name_str(type_name) }
        );
    }

    /// Log an event update request.
    pub fn log_event(event: AnyEvent) {
        tracing::event!(
            target: UpdatesTrace::UPDATES_TARGET,
            tracing::Level::TRACE,
            { kind = "notify event", type_name = event.name() }
        );
    }

    /// Run `action` collecting a trace of what caused updates.
    pub fn collect_trace<R>(trace: &mut Vec<UpdateTrace>, action: impl FnOnce() -> R) -> R {
        let tracer = UpdatesTrace::new();
        let result = Arc::clone(&tracer.trace);
        let r = tracing::subscriber::with_default(tracer, action);
        trace.extend(Arc::try_unwrap(result).unwrap().into_inner());
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
