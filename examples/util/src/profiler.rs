use std::{
    cell::Cell,
    fmt,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    thread,
};

use rustc_hash::FxHashMap;
use tracing::{
    field::{Field, Visit},
    span, Level, Subscriber,
};
use v_jsonescape::escape;

/// Start recording trace level spans and events.
///
/// Call [`Recording::finish`] to stop recording and wait flush.
///
/// Profiles can be viewed using the `chrome://tracing` app. Log events from the `log` crate are not recorded.
///
/// # Output
///
/// The `path` is a JSON file that will be written too as the profiler records. Returns a
/// [`Recording`] struct, you must call [`Recording::finish`] to stop recording and correctly
/// terminate the JSON file. If `finish` is not called the output file will not be valid JSON,
/// you can probably fix it manually in this case by removing the last incomplete event entry and adding
/// `]}`.
///
/// # About
///
/// The `about` array is a list of any key-value metadata to be included in the output.
///
/// # Special Attributes
///
/// If a span or event has an attribute `"name"` the value will be included in the trace entry title,
/// you can use this to dynamically generate a name.
///
/// If a span has an attribute `"thread"` the span will be recorded as the *virtual thread* named.
pub fn record_profile(path: impl AsRef<Path>, about: &[(&str, &str)]) -> Recording {
    let mut file = BufWriter::new(File::create(path).unwrap());

    // specs: https://docs.google.com/document/d/1CvAClvFfyA5R-PhYUmn5OOQtYMH4h6I0nSsKchNAySU/preview#heading=h.lpfof2aylapb

    write!(
        &mut file,
        r#"{{"recorder":"{}-{}", "debug":{},"about":{{"#,
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        cfg!(debug_assertions),
    )
    .unwrap();
    let mut comma = "";
    for (key, value) in about {
        write!(&mut file, r#"{}"{}":"{}""#, comma, escape(key), escape(value)).unwrap();
        comma = ",";
    }
    write!(&mut file, r#"}},"traceEvents":["#).unwrap();

    let (sender, recv) = flume::unbounded();

    let worker = thread::Builder::new()
        .name("profiler".to_owned())
        .spawn(move || {
            let mut spans = FxHashMap::<span::Id, Span>::default();

            struct Span {
                name: &'static str,
                level: Level,
                target: &'static str,
                file: Option<&'static str>,
                line: Option<u32>,
                args: FxHashMap<&'static str, String>,
            }

            let pid = std::process::id();

            let mut comma = "";
            loop {
                match recv.recv().unwrap() {
                    Msg::Event {
                        tid,
                        level,
                        name,
                        target,
                        file: c_file,
                        line,
                        args,
                        ts,
                    } => {
                        write!(
                            &mut file,
                            r#"{}{{"pid":{},"tid":{},"ts":{},"ph":"i","name":"{}","cat":"{}","args":{{"target":"{}""#,
                            comma,
                            pid,
                            tid,
                            ts,
                            NameDisplay(name, &args),
                            level,
                            escape(target)
                        )
                        .unwrap();
                        if let Some(f) = c_file {
                            write!(&mut file, r#","file":"{}""#, escape(f)).unwrap();
                        }
                        if let Some(l) = line {
                            write!(&mut file, r#","line":{}"#, l).unwrap();
                        }
                        for (arg_name, arg_value) in args {
                            write!(&mut file, r#","{}":{}"#, escape(arg_name), arg_value).unwrap();
                        }
                        write!(&mut file, "}}}}").unwrap();
                        comma = ",";
                    }
                    Msg::Enter { id, tid, ts } => {
                        let span = spans.get(&id).unwrap();
                        write!(
                            &mut file,
                            r#"{}{{"pid":{},"tid":{},"name":"{}","cat":"{}","ph":"B","ts":{},"args":{{"target":"{}""#,
                            comma,
                            pid,
                            ThreadIdDisplay(tid, &span.args),
                            NameDisplay(span.name, &span.args),
                            span.level,
                            ts,
                            escape(span.target)
                        )
                        .unwrap();
                        if let Some(f) = span.file {
                            write!(&mut file, r#","file":"{}""#, escape(f)).unwrap();
                        }
                        if let Some(l) = span.line {
                            write!(&mut file, r#","line":{}"#, l).unwrap();
                        }
                        for (arg_name, arg_value) in &span.args {
                            write!(&mut file, r#","{}":{}"#, escape(arg_name), arg_value).unwrap();
                        }
                        write!(&mut file, "}}}}").unwrap();
                        comma = ",";
                    }
                    Msg::Exit { id, tid, ts } => {
                        let span = spans.get(&id).unwrap();
                        write!(
                            &mut file,
                            r#"{}{{"pid":{},"tid":{},"ph":"E","ts":{}}}"#,
                            comma,
                            pid,
                            ThreadIdDisplay(tid, &span.args),
                            ts
                        )
                        .unwrap();
                        comma = ",";
                    }
                    Msg::NewSpan {
                        id,
                        level,
                        name,
                        target,
                        file,
                        line,
                        args,
                    } => {
                        spans.insert(
                            id,
                            Span {
                                level,
                                name,
                                target,
                                file,
                                line,
                                args,
                            },
                        );
                    }
                    Msg::ThreadInfo { id, name } => {
                        write!(
                            &mut file,
                            r#"{}{{"name":"thread_name","ph":"M","pid":{},"tid":{},"args":{{"name":"{}"}}}}"#,
                            comma, pid, id, name
                        )
                        .unwrap();
                        comma = ",";
                    }
                    Msg::Finish => break,
                }
            }
            write!(&mut file, "]}}").unwrap();

            file.flush().unwrap();
        })
        .unwrap();

    tracing::dispatcher::set_global_default(tracing::Dispatch::new(Profiler::new(sender.clone()))).unwrap();

    Recording { sender, worker }
}

/// A running recording operation.
pub struct Recording {
    sender: flume::Sender<Msg>,
    worker: thread::JoinHandle<()>,
}
impl Recording {
    /// Stop recording and wait flush.
    pub fn finish(self) {
        self.sender.send(Msg::Finish).unwrap();
        self.worker.join().unwrap();
    }
}

enum Msg {
    ThreadInfo {
        id: u64,
        name: String,
    },

    NewSpan {
        id: span::Id,
        level: Level,
        name: &'static str,
        target: &'static str,
        file: Option<&'static str>,
        line: Option<u32>,
        args: FxHashMap<&'static str, String>,
    },

    Event {
        tid: u64,
        level: Level,
        name: &'static str,
        target: &'static str,
        file: Option<&'static str>,
        line: Option<u32>,
        args: FxHashMap<&'static str, String>,
        ts: u64,
    },

    Enter {
        id: span::Id,
        tid: u64,
        ts: u64,
    },
    Exit {
        id: span::Id,
        tid: u64,
        ts: u64,
    },

    Finish,
}

struct Profiler {
    id: AtomicU64,
    tid: AtomicU64,
    sender: flume::Sender<Msg>,
}
impl Profiler {
    fn new(sender: flume::Sender<Msg>) -> Self {
        Profiler {
            id: AtomicU64::new(1),
            tid: AtomicU64::new(1),
            sender,
        }
    }

    fn thread_id(&self) -> u64 {
        THREAD_ID.with(|id| {
            if let Some(id) = id.get() {
                id
            } else {
                let tid = self.tid.fetch_add(1, Ordering::Relaxed);
                id.set(Some(tid));
                let _ = self.sender.send(Msg::ThreadInfo {
                    id: tid,
                    name: thread::current()
                        .name()
                        .map(|n| escape(n).to_string())
                        .unwrap_or_else(|| format!("<{:?}>", tid)),
                });
                tid
            }
        })
    }
}
impl Subscriber for Profiler {
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        crate::filter(&Level::TRACE, metadata)
    }

    fn new_span(&self, span: &span::Attributes<'_>) -> span::Id {
        let id = span::Id::from_u64(self.id.fetch_add(1, Ordering::Relaxed));

        let meta = span.metadata();

        let mut args = FxHashMap::default();
        span.record(&mut RecordVisitor(&mut args));

        let _ = self.sender.send(Msg::NewSpan {
            id: id.clone(),
            level: *meta.level(),
            name: meta.name(),
            target: meta.target(),
            file: meta.file(),
            line: meta.line(),
            args,
        });

        id
    }

    fn record(&self, span: &span::Id, values: &span::Record<'_>) {
        let _ = (span, values);
    }

    fn record_follows_from(&self, span: &span::Id, follows: &span::Id) {
        let _ = (span, follows);
    }

    fn event(&self, event: &tracing::Event<'_>) {
        let ts = time_ns();

        let tid = self.thread_id();
        let meta = event.metadata();

        let mut args = FxHashMap::default();
        event.record(&mut RecordVisitor(&mut args));

        let _ = self.sender.send(Msg::Event {
            tid,
            level: *meta.level(),
            name: meta.name(),
            target: meta.target(),
            file: meta.file(),
            line: meta.line(),
            args,
            ts,
        });
    }

    fn enter(&self, span: &span::Id) {
        let ts = time_ns();

        let tid = self.thread_id();

        let _ = self.sender.send(Msg::Enter { id: span.clone(), tid, ts });
    }

    fn exit(&self, span: &span::Id) {
        let ts = time_ns();

        let tid = self.thread_id();

        let _ = self.sender.send(Msg::Exit { id: span.clone(), tid, ts });
    }
}

fn time_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64
}

thread_local! {
    static THREAD_ID: Cell<Option<u64>> = Cell::new(None);
}

struct RecordVisitor<'a>(&'a mut FxHashMap<&'static str, String>);
impl<'a> Visit for RecordVisitor<'a> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let value = format!("{:?}", value);
        let value = escape(&value);
        self.0.insert(field.name(), format!(r#""{}""#, value));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.0.insert(field.name(), format!("{}", value));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.0.insert(field.name(), format!("{}", value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.0.insert(field.name(), format!("{}", value));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.0.insert(field.name(), format!("{}", value));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        let value = escape(value);
        self.0.insert(field.name(), format!(r#""{}""#, value));
    }
}

struct NameDisplay<'a>(&'static str, &'a FxHashMap<&'static str, String>);
impl<'a> fmt::Display for NameDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(dyn_name) = self.1.get("name") {
            let dyn_name = dyn_name.trim_matches('"');
            if self.0.is_empty() {
                write!(f, "{}", dyn_name)
            } else {
                write!(f, "{} ({})", escape(self.0), dyn_name)
            }
        } else if self.0.is_empty() {
            write!(f, "<unnamed>")
        } else {
            write!(f, "{}", escape(self.0))
        }
    }
}

struct ThreadIdDisplay<'a>(u64, &'a FxHashMap<&'static str, String>);
impl<'a> fmt::Display for ThreadIdDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(v_thread) = self.1.get("thread") {
            write!(f, "{}", v_thread)
        } else {
            write!(f, "{}", self.0)
        }
    }
}
