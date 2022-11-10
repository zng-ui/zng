use std::{
    cell::Cell,
    fmt,
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::Duration,
};

use rustc_hash::FxHashMap;
use tracing::{
    field::{Field, Visit},
    span, Subscriber,
};
use v_jsonescape::escape;

pub use tracing::Level;

use crate::mpsc;

/// Arguments for the filter closure of [`record_profile`].
pub struct FilterArgs<'a> {
    /// If entry represents a tracing span. If false it is a log event.
    pub is_span: bool,

    /// Verbosity level.
    pub level: Level,
    /// Event or span name.
    pub name: &'static str,
    /// Event or span  target.
    pub target: &'static str,
    /// File where the event or span where declared.
    pub file: Option<&'static str>,
    /// Line of declaration in [`file`].
    ///
    /// [`file`]: FilterArgs::file
    pub line: Option<u32>,
    /// Arguments for the span or event.
    pub args: &'a FxHashMap<&'static str, String>,

    /// Duration in microseconds. Is zero for events.
    pub duration: u64,
}
impl<'a> FilterArgs<'a> {
    /// If is [`Level::TRACE`].
    pub fn is_trace(&self) -> bool {
        self.level == Level::TRACE
    }

    /// If is [`Level::DEBUG`].
    pub fn is_debug(&self) -> bool {
        self.level == Level::DEBUG
    }

    /// If is [`Level::INFO`].
    pub fn is_info(&self) -> bool {
        self.level == Level::INFO
    }

    /// If is [`Level::WARN`].
    pub fn is_warn(&self) -> bool {
        self.level == Level::WARN
    }

    /// If is [`Level::ERROR`].
    pub fn is_error(&self) -> bool {
        self.level == Level::ERROR
    }
}

/// Start recording trace level spans and events.
///
/// Call [`Recording::finish`] to stop recording and wait flush.
///
/// Profiles can be viewed using the `chrome://tracing` app. Log events from the `log` crate are not recorded.
///
/// # Output
///
/// The `path` is a JSON file that will be written too as the profiler records, the extension will be set to `.json` or
/// `.json.gz` depending the `"deflate"` feature. Returns a [`Recording`] struct, you must call [`Recording::finish`] to stop recording and correctly
/// terminate the JSON file. If `finish` is not called the output file will not be valid JSON,
/// you can probably fix it manually in this case by removing the last incomplete event entry and adding
/// `]}`.
///
/// # About
///
/// The `about` array is a list of any key-value metadata to be included in the output.
///
/// # Filter
///
/// The `filter` closure takes a [`FilterArgs`] and returns `true` if the event or span is to be included in the profile.
///
/// # Special Attributes
///
/// If a span or event has an attribute `"name"` the value will be included in the trace entry title,
/// you can use this to dynamically generate a name.
///
/// If a span has an attribute `"thread"` or `"track"` the span will be recorded as the *virtual thread* named.
///
/// If a event has an attribute `"message"` the message is taken as a name.
pub fn record_profile(
    path: impl Into<PathBuf>,
    about: &[(&str, &dyn std::fmt::Display)],
    filter: impl FnMut(FilterArgs) -> bool + Send + 'static,
) -> Recording {
    record_profile_impl(path.into(), about, Box::new(filter))
}
fn record_profile_impl(
    mut path: PathBuf,
    about: &[(&str, &dyn std::fmt::Display)],
    mut filter: Box<dyn FnMut(FilterArgs) -> bool + Send>,
) -> Recording {
    path.set_extension("json");
    let path = path;

    #[allow(unused_mut)]
    let mut file = BufWriter::new(File::create(&path).unwrap());

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
        write!(&mut file, r#"{comma}"{}":"{}""#, escape(key), escape(&format!("{value}"))).unwrap();
        comma = ",";
    }
    write!(&mut file, r#"}},"traceEvents":["#).unwrap();

    let (sender, mut recv) = mpsc::unbounded();

    let worker = thread::Builder::new()
        .name("profiler".to_owned())
        .spawn(move || {
            struct Span {
                count: usize,
                name: &'static str,
                level: Level,
                target: &'static str,
                file: Option<&'static str>,
                line: Option<u32>,
                args: FxHashMap<&'static str, String>,

                open: Vec<(u64, u64)>,
            }
            let mut spans = FxHashMap::<span::Id, Span>::default();

            let pid = std::process::id();

            let mut comma = "";
            loop {
                let msg = recv.try_recv();
                if msg.is_none() {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }
                match msg.unwrap() {
                    Msg::Event {
                        tid,
                        level,
                        name,
                        target,
                        file: c_file,
                        line,
                        ts,
                        args,
                    } => {
                        let args = FxHashMap::from_iter(args);
                        if !filter(FilterArgs {
                            is_span: false,
                            level,
                            name,
                            target,
                            file: c_file,
                            line,
                            args: &args,
                            duration: 0,
                        }) {
                            continue;
                        }

                        write!(
                            &mut file,
                            r#"{comma}{{"pid":{pid},"tid":{tid},"ts":{ts},"ph":"i","name":"{name}","cat":"{cat}","args":{{"target":"{target}""#,
                            name = NameDisplay(name, &["name", "message"], &args),
                            cat = level,
                            target = escape(target)
                        )
                        .unwrap();
                        if let Some(f) = c_file {
                            write!(&mut file, r#","file":"{}""#, escape(f)).unwrap();
                        }
                        if let Some(l) = line {
                            write!(&mut file, r#","line":{l}"#).unwrap();
                        }
                        for (arg_name, arg_value) in &args {
                            write!(&mut file, r#","{}":{}"#, escape(arg_name), arg_value).unwrap();
                        }
                        write!(&mut file, "}}}}").unwrap();
                        comma = ",";
                    }
                    Msg::Enter { id, tid, ts } => {
                        let span = spans.get_mut(&id).unwrap();
                        span.open.push((tid, ts));
                    }
                    Msg::Exit { id, tid, ts } => {
                        let span = spans.get_mut(&id).unwrap();

                        let enter = span.open.iter().rposition(|(t, _)| *t == tid).unwrap();
                        let (_, start_ts) = span.open.remove(enter);

                        let dur = ts - start_ts;
                        if !filter(FilterArgs {
                            is_span: true,
                            level: span.level,
                            name: span.name,
                            target: span.target,
                            file: span.file,
                            line: span.line,
                            args: &span.args,
                            duration: dur,
                        }) {
                            continue;
                        }

                        write!(
                            &mut file,
                            r#"{comma}{{"pid":{pid},"tid":{tid},"name":"{name}", "cat":"{cat}","ph":"X","ts":{ts},"dur":{dur},"args":{{"target":"{target}""#,
                            tid = ThreadIdDisplay(tid, &span.args),
                            name = NameDisplay(span.name, &["name"], &span.args),
                            cat = span.level,
                            ts = start_ts,
                            target = escape(span.target)
                        )
                        .unwrap();
                        if let Some(f) = span.file {
                            write!(&mut file, r#","file":"{}""#, escape(f)).unwrap();
                        }
                        if let Some(l) = span.line {
                            write!(&mut file, r#","line":{l}"#).unwrap();
                        }
                        for (arg_name, arg_value) in &span.args {
                            write!(&mut file, r#","{}":{}"#, escape(arg_name), arg_value).unwrap();
                        }
                        write!(&mut file, "}}}}").unwrap();
                        comma = ",";
                    }
                    Msg::NewSpan {
                        id,
                        level,
                        name,
                        target,
                        file,
                        line,
                    } => {
                        spans.insert(
                            id,
                            Span {
                                count: 1,
                                level,
                                name,
                                target,
                                file,
                                line,
                                args: FxHashMap::default(),
                                open: vec![],
                            },
                        );
                    }
                    Msg::InsertArgs { id, key, value } => {
                        spans.get_mut(&id).unwrap().args.insert(key, value);
                    }
                    Msg::CloneSpan { id } => {
                        spans.get_mut(&id).unwrap().count += 1;
                    }
                    Msg::DropSpan { id } => {
                        if let std::collections::hash_map::Entry::Occupied(mut s) = spans.entry(id) {
                            s.get_mut().count -= 1;
                            if s.get_mut().count == 0 {
                                s.remove();
                            }
                        } else {
                            unreachable!()
                        }
                    }
                    Msg::ThreadInfo { id, name } => {
                        write!(
                            &mut file,
                            r#"{comma}{{"name":"thread_name","ph":"M","pid":{pid},"tid":{id},"args":{{"name":"{name}"}}}}"#,
                        )
                        .unwrap();
                        comma = ",";
                    }
                    Msg::Finish => {
                        println!("saving profile `{}`", path.display());
                        break
                    },
                }
            }
            write!(&mut file, "]}}").unwrap();

            file.flush().unwrap();
        })
        .unwrap();

    tracing::dispatcher::set_global_default(tracing::Dispatch::new(Profiler::new(sender.clone())))
        .unwrap_or_else(|_| panic!("tracing consumer already set, cannot log and profile at the same time"));

    Recording { sender, worker }
}

/// A running recording operation.
pub struct Recording {
    sender: mpsc::Sender<Msg>,
    worker: thread::JoinHandle<()>,
}
impl Recording {
    /// Stop recording and wait flush.
    pub fn finish(self) {
        self.sender.send(Msg::Finish);
        self.worker.join().unwrap();
    }
}

#[derive(Debug)]
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
    },
    InsertArgs {
        id: span::Id,
        key: &'static str,
        value: String,
    },
    CloneSpan {
        id: span::Id,
    },
    DropSpan {
        id: span::Id,
    },

    // trails implicit `InsertArgs` with the `0` index.
    Event {
        tid: u64,
        level: Level,
        name: &'static str,
        target: &'static str,
        file: Option<&'static str>,
        line: Option<u32>,
        ts: u64,
        args: Vec<(&'static str, String)>,
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
    sender: mpsc::Sender<Msg>,
}
impl Profiler {
    fn new(sender: mpsc::Sender<Msg>) -> Self {
        Profiler {
            id: AtomicU64::new(1), // 0 is event
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
                self.sender.send(Msg::ThreadInfo {
                    id: tid,
                    name: thread::current()
                        .name()
                        .map(|n| escape(n).to_string())
                        .unwrap_or_else(|| format!("<{tid:?}>")),
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

        self.sender.send(Msg::NewSpan {
            id: id.clone(),
            level: *meta.level(),
            name: meta.name(),
            target: meta.target(),
            file: meta.file(),
            line: meta.line(),
        });
        span.record(&mut span_values_sender(&id, &self.sender));

        id
    }

    fn clone_span(&self, id: &span::Id) -> span::Id {
        self.sender.send(Msg::CloneSpan { id: id.clone() });
        id.clone()
    }

    fn try_close(&self, id: span::Id) -> bool {
        self.sender.send(Msg::DropSpan { id });
        true
    }

    fn record(&self, id: &span::Id, values: &span::Record<'_>) {
        values.record(&mut span_values_sender(id, &self.sender));
    }

    fn record_follows_from(&self, span: &span::Id, follows: &span::Id) {
        let _ = (span, follows);
    }

    fn event(&self, event: &tracing::Event<'_>) {
        let ts = timestamp();

        let tid = self.thread_id();
        let meta = event.metadata();

        let mut args = vec![];
        event.record(&mut event_values_collector(&mut args));
        self.sender.send(Msg::Event {
            tid,
            level: *meta.level(),
            name: meta.name(),
            target: meta.target(),
            file: meta.file(),
            line: meta.line(),
            ts,
            args,
        });
    }

    fn enter(&self, span: &span::Id) {
        let ts = timestamp();

        let tid = self.thread_id();

        self.sender.send(Msg::Enter { id: span.clone(), tid, ts });
    }

    fn exit(&self, span: &span::Id) {
        let ts = timestamp();

        let tid = self.thread_id();

        self.sender.send(Msg::Exit { id: span.clone(), tid, ts });
    }
}

fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64
}

thread_local! {
    static THREAD_ID: Cell<Option<u64>> = Cell::new(None);
}

fn span_values_sender<'a>(id: &'a span::Id, sender: &'a mpsc::Sender<Msg>) -> RecordVisitor<impl FnMut(&'static str, String) + 'a> {
    RecordVisitor(|key, value| {
        sender.send(Msg::InsertArgs {
            id: id.clone(),
            key,
            value,
        });
    })
}

fn event_values_collector<'a>(args: &'a mut Vec<(&'static str, String)>) -> RecordVisitor<impl FnMut(&'static str, String) + 'a> {
    RecordVisitor(|key, value| {
        args.push((key, value));
    })
}

struct RecordVisitor<F>(F);
impl<F> RecordVisitor<F> {}
impl<F: FnMut(&'static str, String)> Visit for RecordVisitor<F> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let value = format!("{value:?}");
        let value = escape(&value);
        (self.0)(field.name(), format!(r#""{value}""#));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        (self.0)(field.name(), format!("{value}"));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        (self.0)(field.name(), format!("{value}"));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        (self.0)(field.name(), format!("{value}"));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        (self.0)(field.name(), format!("{value}"));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        let value = escape(value);
        (self.0)(field.name(), format!(r#""{value}""#));
    }
}

struct NameDisplay<'a>(&'static str, &'a [&'static str], &'a FxHashMap<&'static str, String>);
impl<'a> fmt::Display for NameDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for dyn_name_key in self.1 {
            if let Some(dyn_name) = self.2.get(dyn_name_key) {
                let dyn_name = &dyn_name[1..dyn_name.len() - 1]; // remove quotes
                return if self.0.is_empty() || self.0.contains(".rs:") {
                    write!(f, "{dyn_name}")
                } else {
                    write!(f, "{} ({})", escape(self.0), dyn_name)
                };
            }
        }

        if self.0.is_empty() {
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
            write!(f, "{v_thread}")
        } else if let Some(v_thread) = self.1.get("track") {
            write!(f, "{v_thread}")
        } else {
            write!(f, "{}", self.0)
        }
    }
}

/// Quick trace variable value spans with the variable as a thread and the value as spans.
///
/// # Syntax
///
/// ```
/// # macro_rules! trace_var { ($($tt:tt)*) => {} }
/// trace_var!(ctx.vars, ?var); // debug
/// trace_var!(ctx.vars, %var); // display
/// ```
#[macro_export]
macro_rules! trace_var {
    ($vars:expr, $tracing_display_or_debug:tt $var:ident) => {
        $var.trace_value($vars, |value| {
            tracing::trace_span!("", name = $tracing_display_or_debug value, track = stringify!($var)).entered()
        }).perm();
    };
}

/// Suppress tracing warnings from dependencies that we can't handle.
pub fn filter(level: &Level, metadata: &tracing::Metadata) -> bool {
    if metadata.level() > level {
        return false;
    }

    // suppress webrender warnings:
    //
    if metadata.target() == "webrender::device::gl" {
        // suppress vertex debug-only warnings.
        // see: https://bugzilla.mozilla.org/show_bug.cgi?id=1615342
        if metadata.line() == Some(2396) {
            return false;
        }

        // Suppress "Cropping texture upload Box2D((0, 0), (0, 1)) to None"
        // This happens when an empty frame is rendered.
        if metadata.line() == Some(4560) {
            return false;
        }
    }

    // suppress font-kit warnings:
    //
    if metadata.target() == "font_kit::loaders::freetype" {
        // Suppress "$fn(): found invalid platform ID $n"
        // This does not look fully implemented and generates a lot of warns
        // with the default Ubuntu font set all with valid platform IDs.
        if metadata.line() == Some(735) {
            return false;
        }
    }

    true
}
