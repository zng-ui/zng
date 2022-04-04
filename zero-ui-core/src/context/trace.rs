use std::{
    any::type_name,
    collections::{hash_map, HashMap},
    fmt,
    sync::Arc,
};

use parking_lot::Mutex;
use tracing::span;

use crate::{app::AppExtension, event::Event, var::VarValue, window::WindowId, WidgetId};

pub(crate) struct UpdatesTrace {
    context: Mutex<UpdateContext>,
    trace: Arc<Mutex<Vec<UpdateTrace>>>,

    widgets_stack: Mutex<Vec<WidgetId>>,
    properties_stack: Mutex<Vec<String>>,
}
impl tracing::subscriber::Subscriber for UpdatesTrace {
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        metadata.target() == Self::UPDATES_TARGET
    }

    fn new_span(&self, span: &span::Attributes<'_>) -> span::Id {
        match span.metadata().name() {
            "PROPERTY" => {
                let name = visit_str(|v| span.record(v), "name");
                let mut ctx = self.context.lock();

                if let Some(p) = ctx.property.replace(name) {
                    self.properties_stack.lock().push(p);
                }

                span::Id::from_u64(1)
            }
            "WIDGET" => {
                let id = visit_u64(|v| span.record(v), "id").unwrap();
                if id == 0 {
                    panic!()
                }
                let id = unsafe { WidgetId::from_raw(id) };

                let mut ctx = self.context.lock();
                if let Some(p) = ctx.widget_id.replace(id) {
                    self.widgets_stack.lock().push(p);
                }
                span::Id::from_u64(2)
            }
            "WINDOW" => {
                let id = visit_u64(|v| span.record(v), "id").unwrap() as u32;
                if id == 0 {
                    panic!()
                }
                let id = unsafe { WindowId::from_raw(id) };
                self.context.lock().window_id = Some(id);

                span::Id::from_u64(3)
            }
            "APP-EXTENSION" => {
                let name = visit_str(|v| span.record(v), "type_name");
                self.context.lock().app_extension = Some(name);
                span::Id::from_u64(4)
            }
            _ => span::Id::from_u64(u64::MAX),
        }
    }

    fn record(&self, _span: &span::Id, _values: &span::Record<'_>) {}

    fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {
        let action = match visit_str(|v| event.record(v), "kind").as_str() {
            "VAR" => UpdateAction::Var {
                type_name: visit_str(|v| event.record(v), "type_name"),
            },
            "EVENT" => UpdateAction::Event {
                type_name: visit_str(|v| event.record(v), "type_name"),
            },
            "UPDATE" => UpdateAction::Update,
            "LAYOUT" => UpdateAction::Layout,
            _ => return,
        };

        let ctx = self.context.lock().clone();

        let entry = UpdateTrace { ctx, action };
        self.trace.lock().push(entry);
    }

    fn enter(&self, _span: &span::Id) {}

    fn exit(&self, span: &span::Id) {
        let mut ctx = self.context.lock();
        if span == &span::Id::from_u64(1) {
            ctx.property = self.properties_stack.lock().pop();
        } else if span == &span::Id::from_u64(2) {
            ctx.widget_id = self.widgets_stack.lock().pop();
        } else if span == &span::Id::from_u64(3) {
            ctx.window_id = None;
        } else if span == &span::Id::from_u64(4) {
            ctx.app_extension = None;
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
            properties_stack: Mutex::new(Vec::with_capacity(100)),
        }
    }

    /// Opens an app extension span.
    #[inline(always)]
    pub fn extension_span<E: AppExtension>() -> tracing::span::EnteredSpan {
        tracing::trace_span!(target: UpdatesTrace::UPDATES_TARGET, "APP-EXTENSION", type_name = type_name::<E>()).entered()
    }

    /// Opens a window span.
    #[inline(always)]
    pub fn window_span(id: WindowId) -> tracing::span::EnteredSpan {
        tracing::trace_span!(target: UpdatesTrace::UPDATES_TARGET, "WINDOW", id = id.get() as u64).entered()
    }

    /// Opens a widget span.
    #[inline(always)]
    pub fn widget_span(id: WidgetId) -> tracing::span::EnteredSpan {
        tracing::trace_span!(target: UpdatesTrace::UPDATES_TARGET, "WIDGET", id = id.get()).entered()
    }

    /// Opens a property span.
    #[inline(always)]
    pub fn property_span(name: &'static str) -> tracing::span::EnteredSpan {
        tracing::trace_span!(target: UpdatesTrace::UPDATES_TARGET, "PROPERTY", name).entered()
    }

    /// Log a direct update request.
    #[inline(always)]
    pub fn log_update() {
        tracing::event!(target: UpdatesTrace::UPDATES_TARGET, tracing::Level::TRACE, { kind = "UPDATE" });
    }

    /// Log a direct layout request.
    #[inline(always)]
    pub fn log_layout() {
        tracing::event!(target: UpdatesTrace::UPDATES_TARGET, tracing::Level::TRACE, { kind = "LAYOUT" });
    }

    /// Log a var update request.
    #[inline(always)]
    pub fn log_var<T: VarValue>() {
        tracing::event!(
            target: UpdatesTrace::UPDATES_TARGET,
            tracing::Level::TRACE,
            { kind = "VAR", type_name = type_name::<T>() }
        );
    }

    /// Log an event update request.
    #[inline(always)]
    pub fn log_event<E: Event>() {
        tracing::event!(
            target: UpdatesTrace::UPDATES_TARGET,
            tracing::Level::TRACE,
            { kind = "EVENT", type_name = type_name::<E>() }
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

    /// Displays the top 10 most frequent update sources in the trace.
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
        for (t, c) in frequencies.into_iter().take(10) {
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
    widget_id: Option<WidgetId>,
    property: Option<String>,
}
impl fmt::Display for UpdateContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(e) = &self.app_extension {
            write!(f, "{}//", e.rsplit("::").next().unwrap())?;
        } else {
            write!(f, "<unknown>//")?;
        }
        if let Some(w) = self.window_id {
            write!(f, "{w}/?/")?;
        }
        if let Some(w) = self.widget_id {
            write!(f, "{w}")?;
        }
        if let Some(p) = &self.property {
            write!(f, "::{p}")?;
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
}
impl fmt::Display for UpdateAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UpdateAction::Update => write!(f, "update"),
            UpdateAction::Layout => write!(f, "layout"),
            UpdateAction::Var { type_name } => write!(f, "update var of type {type_name}"),
            UpdateAction::Event { type_name } => write!(f, "update event {type_name}"),
        }
    }
}

fn visit_str(record: impl FnOnce(&mut dyn tracing::field::Visit), name: &str) -> String {
    struct Visitor<'a> {
        name: &'a str,
        result: String,
    }
    impl<'a> tracing::field::Visit for Visitor<'a> {
        fn record_debug(&mut self, _field: &tracing::field::Field, _value: &dyn std::fmt::Debug) {}
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
