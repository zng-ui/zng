use std::{sync::Arc, any::type_name};

use parking_lot::Mutex;
use tracing::span;

use crate::{window::WindowId, WidgetId, event::Event, var::{VarValue, Var}};

pub(super) struct UpdatesTrace {
    context: Mutex<UpdateContext>,
    trace: Arc<Mutex<Vec<UpdateTrace>>>,
}
impl tracing::subscriber::Subscriber for UpdatesTrace {
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        metadata.target() == Self::UPDATES_TARGET
    }

    fn new_span(&self, span: &span::Attributes<'_>) -> span::Id {
        match span.metadata().name() {
            "PROPERTY" => {
                let name = visit_str(|v|span.record(v), "name");
                self.context.lock().property = Some(name);
            }
            "WIDGET" => {
                let id = visit_u64(|v|span.record(v), "id").unwrap();
                if id == 0 { panic!() }
                let id = unsafe { WidgetId::from_raw(id) };
                self.context.lock().widget_id = Some(id);
            }
            "WINDOW" => {
                let id = visit_u64(|v|span.record(v), "id").unwrap() as u32;
                if id == 0 { panic!() }
                let id = unsafe { WindowId::from_raw(id) };
                self.context.lock().window_id = Some(id);
            }
            _ => return span::Id::from_u64(1),
        }
        span::Id::from_u64(0)
    }

    fn record(&self, _span: &span::Id, _values: &span::Record<'_>) {}

    fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {
        let action = match event.metadata().name() {
            "VAR" => UpdateAction::Var { type_name: visit_str(|v| event.record(v), "type_name") },
            "EVENT" => UpdateAction::Event { type_name: visit_str(|v| event.record(v), "type_name") },
            "UPDATE" => UpdateAction::Update,
            "LAYOUT" => UpdateAction::Layout,
            _ => return,
        };
        let entry = UpdateTrace {
            ctx: self.context.lock().clone(),
            action,
        };
        self.trace.lock().push(entry);
    }

    fn enter(&self, _span: &span::Id) {}

    fn exit(&self, span: &span::Id) {
        if span == &span::Id::from_u64(0) {
            let mut ctx = self.context.lock();
            if ctx.property.is_some() {
                ctx.property = None;
            } else if ctx.widget_id.is_some() {
                ctx.widget_id = None;
            } else {
                ctx.window_id.take().unwrap();
            }
        }
    }
}
impl UpdatesTrace {
    const UPDATES_TARGET: &'static str = "zero-ui-updates";

    fn new() -> Self {
        UpdatesTrace {
            context: Mutex::new(UpdateContext::default()),
            trace: Arc::new(Mutex::new(Vec::with_capacity(100))),
        }
    }

    /// Opens a window span.
    #[inline(always)]
    pub fn window_span(id: WindowId) -> tracing::span::EnteredSpan {
        tracing::trace_span!("WINDOW", id=id.get() as u64).entered()
    }

    /// Opens a widget span.
    #[inline(always)]
    pub fn widget_span(id: WidgetId) -> tracing::span::EnteredSpan {
        tracing::trace_span!("WIDGET", id=id.get()).entered()
    }

    /// Opens a property span.
    #[inline(always)]
    pub fn property_span(name: &'static str) -> tracing::span::EnteredSpan {
        tracing::trace_span!("PROPERTY", name).entered()
    }

    /// Log a direct update request.
    #[inline(always)]
    pub fn log_update() {
        tracing::event!(target: "UPDATE", tracing::Level::TRACE, {});
    }

    /// Log a direct layout request.
    #[inline(always)]
    pub fn log_layout() {
        tracing::event!(target: "LAYOUT", tracing::Level::TRACE, {});
    }

    /// Log a var update request.
    #[inline(always)]
    pub fn log_var<T: VarValue, V: Var<T>>(var: &V) {
        tracing::event!(target: "VAR", tracing::Level::TRACE, { type_name=type_name::<V>() });
    }

    /// Log an event update request.
    #[inline(always)]
    pub fn log_event<E: Event>(event: &E) {
        tracing::event!(target: "EVENT", tracing::Level::TRACE, { type_name=type_name::<E>() });
    }

    /// Run `action` collecting a trace of what caused updates.
    pub fn collect_trace(action: impl FnOnce()) -> Vec<UpdateTrace> {
        let tracer = UpdatesTrace::new();
        let result = Arc::clone(&tracer.trace);
        tracing::subscriber::with_default(tracer, action);

        Arc::try_unwrap(result).unwrap().into_inner()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub(super) struct UpdateContext {
    pub window_id: Option<WindowId>,
    pub widget_id: Option<WidgetId>,
    pub property: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) struct UpdateTrace {
    pub ctx: UpdateContext,
    pub action: UpdateAction,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) enum UpdateAction {
    Update,
    Layout,
    Var { type_name: String },
    Event { type_name: String },
}

fn visit_str(record: impl FnOnce(&mut dyn tracing::field::Visit), name: &str) -> String {
    struct Visitor<'a> {
        name: &'a str,
        result: String,
    }
    impl<'a> tracing::field::Visit for Visitor<'a> {
        fn record_debug(&mut self, _field: &tracing::field::Field, _value: &dyn std::fmt::Debug) {            
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
        fn record_debug(&mut self, _field: &tracing::field::Field, _value: &dyn std::fmt::Debug) {            
        }
        fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
            if field.name() == self.name {
                self.result = Some(value)
            }
        }
    }

    let mut visitor = Visitor {
        name,
        result: None,
    };
    record(&mut visitor);
    visitor.result
}