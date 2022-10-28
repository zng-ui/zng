use std::{
    any::type_name,
    collections::{hash_map, HashMap},
    fmt,
    sync::Arc,
};

use parking_lot::Mutex;
use tracing::span;

use crate::{
    app::AppExtension,
    event::{Event, EventArgs},
    var::VarValue,
    widget_instance::{TraceNode, UiNode, WidgetId},
    window::WindowId,
};

use super::InfoContext;

/// Extension methods for infinite loop diagnostics.
///
/// You can also use [`updates_trace_span`] to define a custom scope inside a node, and [`updates_trace_event`]
/// to log a custom entry.
///
/// Note that traces are only recorded if the "inspector" feature is active and a tracing subscriber is installed.
pub trait UpdatesTraceUiNodeExt {
    /// Defines a custom span.
    #[allow(clippy::type_complexity)]
    fn instrument<S: Into<String>>(
        self,
        tag: S,
    ) -> TraceNode<Self, Box<dyn Fn(&mut InfoContext, &'static str) -> tracing::span::EnteredSpan>>
    where
        Self: Sized;
}
impl<U: UiNode> UpdatesTraceUiNodeExt for U {
    fn instrument<S: Into<String>>(
        self,
        tag: S,
    ) -> TraceNode<Self, Box<dyn Fn(&mut InfoContext, &'static str) -> tracing::span::EnteredSpan>> {
        #[cfg(inspector)]
        {
            let tag = tag.into();
            TraceNode::new(self, Box::new(move |_ctx, node_mtd| UpdatesTrace::custom_span(&tag, node_mtd)))
        }
        #[cfg(not(inspector))]
        {
            let _ = tag;
            TraceNode::new(self, Box::new(|_, _| tracing::Span::none().entered()))
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

    fn new_span(&self, span: &span::Attributes<'_>) -> span::Id {
        let r = match span.metadata().name() {
            "property" | "constructor" => {
                let name = visit_str(|v| span.record(v), "name");
                let mut ctx = self.context.lock();

                if let Some(p) = ctx.node_parent.replace(name) {
                    self.node_parents_stack.lock().push(p);
                }
                if let Some(p) = ctx.tag.replace(String::new()) {
                    self.tags_stack.lock().push(p);
                }

                span::Id::from_u64(1)
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

                if let Some(p) = ctx.node_parent.replace("new".to_owned()) {
                    self.node_parents_stack.lock().push(p);
                }

                if let Some(p) = ctx.tag.replace(String::new()) {
                    self.tags_stack.lock().push(p);
                }

                span::Id::from_u64(2)
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

                span::Id::from_u64(3)
            }
            "AppExtension" => {
                let name = visit_str(|v| span.record(v), "name");

                let mut ctx = self.context.lock();
                ctx.app_extension = Some(name);

                if let Some(p) = ctx.tag.replace(String::new()) {
                    self.tags_stack.lock().push(p);
                }

                span::Id::from_u64(4)
            }
            "tag" => {
                let tag = visit_str(|v| span.record(v), "tag");
                let mut ctx = self.context.lock();
                if let Some(p) = ctx.tag.replace(tag) {
                    self.tags_stack.lock().push(p);
                }
                span::Id::from_u64(5)
            }
            _ => span::Id::from_u64(u64::MAX),
        };
        // println!("{}", self.context.lock());
        r
    }

    fn record(&self, _span: &span::Id, _values: &span::Record<'_>) {}

    fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id) {}

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
        if ctx.app_extension.is_none() {
            return;
        }

        let entry = UpdateTrace { ctx, action };
        self.trace.lock().push(entry);
    }

    fn enter(&self, _span: &span::Id) {}

    fn exit(&self, span: &span::Id) {
        let mut ctx = self.context.lock();
        if span == &span::Id::from_u64(1) {
            ctx.node_parent = self.node_parents_stack.lock().pop();
            ctx.tag = self.tags_stack.lock().pop();
        } else if span == &span::Id::from_u64(2) {
            ctx.widget = self.widgets_stack.lock().pop();
            ctx.node_parent = self.node_parents_stack.lock().pop();
            ctx.tag = self.tags_stack.lock().pop();
        } else if span == &span::Id::from_u64(3) {
            ctx.window_id = None;
            ctx.tag = self.tags_stack.lock().pop();
        } else if span == &span::Id::from_u64(4) {
            ctx.app_extension = None;
            ctx.tag = self.tags_stack.lock().pop();
        } else if span == &span::Id::from_u64(5) {
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
        tracing::trace_span!(target: UpdatesTrace::UPDATES_TARGET, "AppExtension", name = type_name::<E>(), %ext_mtd).entered()
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
    pub fn log_var<T: VarValue>() {
        tracing::event!(
            target: UpdatesTrace::UPDATES_TARGET,
            tracing::Level::TRACE,
            { kind = "update var", type_name = type_name::<T>() }
        );
    }

    /// Log an event update request.
    pub fn log_event<A: EventArgs>(event: Event<A>) {
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
            write!(f, "/{w}")?;
        }
        if let Some((id, name)) = &self.widget {
            write!(f, "/{name}#{id}")?;
        }
        if let Some(p) = &self.node_parent {
            write!(f, "/{p}")?;
        }
        if let Some(t) = &self.tag {
            if !t.is_empty() {
                write!(f, "/{t}")?;
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
