#![cfg(all(
    feature = "trace_recorder",
    not(any(target_arch = "wasm32", target_os = "android", target_os = "ios"))
))]

//! Trace recording and data model.
//!
//! All tracing instrumentation in Zng projects is done using the `tracing` crate, this module uses the `tracing-chrome` crate
//! to record traces that can be viewed in `chrome://tracing` or `ui.perfetto.dev` and can be parsed to the [`Trace`] data model.

use std::{
    collections::HashMap,
    fmt,
    io::{self, Read},
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use parking_lot::Mutex;
use serde::Deserialize as _;
use tracing_subscriber::{filter::EnvFilter, layer::SubscriberExt as _, util::SubscriberInitExt as _};
use zng_txt::{ToTxt as _, Txt};

/// Represents a recorded trace.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Trace {
    /// Traced app processes.
    pub processes: Vec<ProcessTrace>,
}

/// Represents a single app process in a recorded trace.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ProcessTrace {
    /// System process ID.
    pub pid: u64,

    /// Process name.
    pub name: Txt,

    /// Traced threads on the process.
    pub threads: Vec<ThreadTrace>,

    /// Process start instant.
    ///
    /// This time stamp is system dependent, if the system time changes before a second app process starts it can show as starting first.
    ///
    /// If [`SystemTime::UNIX_EPOCH`] if the recorder does not support time.
    pub start: SystemTime,
}

/// Represents a single thread in an app process in a recorded trace.
#[derive(Clone)]
#[non_exhaustive]
pub struct ThreadTrace {
    /// Thread name.
    pub name: Txt,

    /// Events that happened on the thread.
    pub events: Vec<EventTrace>,
    /// Spans started and ended on the thread.
    pub spans: Vec<SpanTrace>,
}
impl fmt::Debug for ThreadTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ThreadTrace")
            .field("name", &self.name)
            .field("events.len()", &self.events.len())
            .field("spans.len()", &self.spans.len())
            .finish()
    }
}

/// Represents a traced event.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct EventTrace {
    /// Event info.
    pub info: Info,
    /// Moment from the recording start when this event happened.
    pub instant: Duration,
}

/// Represents a traced span.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SpanTrace {
    /// Span info.
    pub info: Info,

    /// Moment from the recording start when this span started.
    pub start: Duration,
    /// Moment from the recording start when this span ended.
    pub end: Duration,
}

/// Common info traced about events and spans.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Info {
    /// Event or span name.
    pub name: Txt,
    /// Categories.
    ///
    /// Zng recordings usually write two categories, "target" and "level".
    pub categories: Vec<Txt>,
    /// File where the event or span was traced.
    pub file: Txt,
    /// Code line where the event or span was traced.
    pub line: u32,
    /// Custom args traced with the event or span.
    pub args: HashMap<Txt, Txt>,
}

impl Trace {
    /// Read and parse a Chrome JSON Array format trace.
    ///
    /// See [`parse_chrome_trace`] for more details.
    /// 
    /// [`parse_chrome_trace`]: Self::parse_chrome_trace
    pub fn read_chrome_trace(json_path: impl AsRef<Path>) -> io::Result<Self> {
        let json = std::fs::read_to_string(json_path)?;
        let trace = Self::parse_chrome_trace(&json)?;
        Ok(trace)
    }

    /// Parse a Chrome JSON Array format trace.
    ///
    /// Only supports the "phases" emitted by `tracing-chrome` in `TraceStyle::Threaded` mode, those are `B, E, i, M` for `M` only
    /// supports `thread_name` metadata. Also parses the custom messages that define the process name and start timestamp as defined
    /// by the `zng::app::trace_recorder` documentation.
    pub fn parse_chrome_trace(json: &str) -> io::Result<Self> {
        fn invalid_data(msg: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> io::Error {
            io::Error::new(io::ErrorKind::InvalidData, msg)
        }

        // skip the array opening
        let json = json.trim_start();
        if !json.starts_with('[') {
            return Err(invalid_data("expected JSON array"));
        }
        let json = &json[1..];

        enum Phase {
            Begin,
            End,
            Event,
        }
        struct Entry {
            phase: Phase,
            pid: u64,
            tid: u64,
            ts: Duration,
            name: Txt,
            cat: Vec<Txt>,
            file: Txt,
            line: u32,
            args: HashMap<Txt, Txt>,
        }
        let mut process_sys_pid = HashMap::new();
        let mut process_names = HashMap::new();
        let mut process_record_start = HashMap::new();
        let mut thread_names = HashMap::new();
        let mut entries = vec![];

        let mut reader = std::io::Cursor::new(json.as_bytes());
        loop {
            // skip white space and commas to the next object
            let mut pos = reader.position();
            let mut buf = [0u8];
            while reader.read(&mut buf).is_ok() {
                if !b" \r\n\t,".contains(&buf[0]) {
                    break;
                }
                pos = reader.position();
            }
            reader.set_position(pos);
            let mut de = serde_json::Deserializer::from_reader(&mut reader);
            match serde_json::Value::deserialize(&mut de) {
                Ok(entry) => match entry {
                    serde_json::Value::Object(map) => {
                        let pid = match map.get("pid") {
                            Some(serde_json::Value::Number(n)) => match n.as_u64() {
                                Some(pid) => pid,
                                None => return Err(invalid_data("expected \"pid\"")),
                            },
                            _ => return Err(invalid_data("expected \"pid\"")),
                        };
                        let tid = match map.get("tid") {
                            Some(serde_json::Value::Number(n)) => match n.as_u64() {
                                Some(tid) => tid,
                                None => return Err(invalid_data("expected \"tid\"")),
                            },
                            _ => return Err(invalid_data("expected \"tid\"")),
                        };
                        let name = match map.get("name") {
                            Some(serde_json::Value::String(name)) => name.to_txt(),
                            _ => return Err(invalid_data("expected \"name\"")),
                        };
                        let args: HashMap<Txt, Txt> = match map.get("args") {
                            Some(a) => match serde_json::from_value(a.clone()) {
                                Ok(a) => a,
                                Err(e) => {
                                    tracing::error!("only simple text args are supported, {e}");
                                    continue;
                                }
                            },
                            _ => HashMap::new(),
                        };
                        let phase = match map.get("ph") {
                            Some(serde_json::Value::String(ph)) => match ph.as_str() {
                                "B" => Phase::Begin,
                                "E" => Phase::End,
                                "i" => Phase::Event,
                                "M" => {
                                    if name == "thread_name" {
                                        if let Some(n) = args.get("name") {
                                            thread_names.insert(tid, n.to_txt());
                                        }
                                    }
                                    continue;
                                }
                                u => {
                                    tracing::error!("ignoring unknown or unsupported phase `{u:?}`");
                                    continue;
                                }
                            },
                            _ => return Err(invalid_data("expected \"ph\"")),
                        };

                        let ts = match map.get("ts") {
                            Some(serde_json::Value::Number(ts)) => match ts.as_f64() {
                                Some(ts) => Duration::from_nanos((ts * 1000.0).round() as u64),
                                None => return Err(invalid_data("expected \"ts\"")),
                            },
                            _ => return Err(invalid_data("expected \"ts\"")),
                        };
                        let cat = match map.get("cat") {
                            Some(serde_json::Value::String(cat)) => cat.split(',').map(|c| c.trim().to_txt()).collect(),
                            _ => vec![],
                        };
                        let file = match map.get(".file") {
                            Some(serde_json::Value::String(file)) => file.to_txt(),
                            _ => Txt::from_static(""),
                        };
                        let line = match map.get(".line") {
                            Some(serde_json::Value::Number(line)) => line.as_u64().unwrap_or(0) as u32,
                            _ => 0,
                        };

                        if let Some(msg) = args.get("message") {
                            if let Some(process_ts) = msg.strip_prefix("zng-record-start: ") {
                                if let Ok(process_ts) = process_ts.parse::<u64>() {
                                    process_record_start.insert(pid, SystemTime::UNIX_EPOCH + Duration::from_micros(process_ts));
                                }
                            } else if let Some(rest) = msg.strip_prefix("pid: ") {
                                if let Some((sys_pid, p_name)) = rest.split_once(", name: ") {
                                    if let Ok(sys_pid) = sys_pid.parse::<u64>() {
                                        process_sys_pid.insert(pid, sys_pid);
                                        process_names.insert(pid, p_name.to_txt());
                                    }
                                }
                            }
                        }

                        entries.push(Entry {
                            phase,
                            pid,
                            tid,
                            ts,
                            name,
                            cat,
                            file,
                            line,
                            args,
                        });
                    }
                    _ => return Err(invalid_data("expected JSON array of objects")),
                },
                Err(_) => {
                    // EOF
                    break;
                }
            }
        }

        let mut out = Trace { processes: vec![] };

        for entry in entries {
            let sys_pid = *process_sys_pid.entry(entry.pid).or_insert(entry.pid);
            let process = if let Some(p) = out.processes.iter_mut().find(|p| p.pid == sys_pid) {
                p
            } else {
                out.processes.push(ProcessTrace {
                    pid: sys_pid,
                    name: process_names.entry(entry.pid).or_insert_with(|| sys_pid.to_txt()).clone(),
                    threads: vec![],
                    start: process_record_start.get(&entry.pid).copied().unwrap_or(SystemTime::UNIX_EPOCH),
                });
                out.processes.last_mut().unwrap()
            };

            let thread_name = thread_names.entry(entry.tid).or_insert_with(|| entry.tid.to_txt()).clone();
            let thread = if let Some(t) = process.threads.iter_mut().find(|t| t.name == thread_name) {
                t
            } else {
                process.threads.push(ThreadTrace {
                    name: thread_name,
                    events: vec![],
                    spans: vec![],
                });
                process.threads.last_mut().unwrap()
            };

            fn entry_to_info(entry: Entry) -> Info {
                Info {
                    name: entry.name,
                    categories: entry.cat,
                    file: entry.file,
                    line: entry.line,
                    args: entry.args,
                }
            }

            match entry.phase {
                Phase::Begin => thread.spans.push(SpanTrace {
                    start: entry.ts,
                    end: entry.ts,
                    info: entry_to_info(entry),
                }),
                Phase::End => {
                    let end = entry.ts;
                    let info = entry_to_info(entry);
                    if let Some(open) = thread.spans.iter_mut().rev().find(|s| s.start == s.end && s.info.name == info.name) {
                        open.end = end;
                        open.info.merge(info);
                    }
                }
                Phase::Event => thread.events.push(EventTrace {
                    instant: entry.ts,
                    info: entry_to_info(entry),
                }),
            }
        }

        Ok(out)
    }

    /// Convert the trace to Chrome JSON Array format.
    pub fn to_chrome_trace(&self) -> Txt {
        todo!("!!:")
    }

    /// Convert and write the trace to Chrome JSON Array format.
    pub fn write_chrome_trace(&self, json_path: impl AsRef<Path>) -> io::Result<()> {
        std::fs::write(json_path, self.to_chrome_trace().as_str().as_bytes())
    }

    /// Merge `other` into this.
    pub fn merge(&mut self, other: Self) {
        for p in other.processes {
            if let Some(ep) = self.processes.iter_mut().find(|ep| ep.pid == p.pid && ep.name == p.name) {
                ep.merge(p);
            } else {
                self.processes.push(p);
            }
        }
    }

    /// Sort processes processes and threads by start time then name, events by instant and spans by start.
    pub fn sort(&mut self) {
        self.processes.sort_by(|a, b| a.start.cmp(&b.start).then(a.name.cmp(&b.name)));
        for p in &mut self.processes {
            p.sort();
        }
    }
}

impl ProcessTrace {
    /// Merge `other` into this.
    pub fn merge(&mut self, other: Self) {
        for t in other.threads {
            if let Some(et) = self.threads.iter_mut().find(|et| et.name == t.name) {
                et.merge(t);
            } else {
                self.threads.push(t);
            }
        }
    }

    /// Sort threads by name, events by instant and spans by start.
    pub fn sort(&mut self) {
        self.threads.sort_by(|a, b| a.start().cmp(&b.start()).then(a.name.cmp(&b.name)));
        for t in &mut self.threads {
            t.sort();
        }
    }
}

impl ThreadTrace {
    /// Gets the minimum event or span start in the thread.
    pub fn start(&self) -> Duration {
        self.events
            .iter()
            .map(|e| e.instant)
            .min()
            .unwrap_or(Duration::MAX)
            .min(self.spans.iter().map(|e| e.start).min().unwrap_or(Duration::MAX))
    }

    /// Merge `other` into this.
    pub fn merge(&mut self, mut other: Self) {
        self.events.append(&mut other.events);
        self.spans.append(&mut other.spans);
    }

    /// Sort events by instant and spans by start.
    pub fn sort(&mut self) {
        self.events.sort_by(|a, b| a.instant.cmp(&b.instant));
        self.spans.sort_by(|a, b| a.start.cmp(&b.start));
    }
}

impl Info {
    /// Merge `other` into this.
    pub fn merge(&mut self, info: Info) {
        if !info.file.is_empty() {
            self.file = info.file;
            self.line = info.line;
        }
        self.args.extend(info.args);
    }
}

/// Starts recording, stops on process exit or on [`stop_recording`].
///
/// Note that this is called automatically on startup if the `ZNG_RECORD_TRACE` environment variable is set and that is
/// the recommended way of enabling recording as it record all processes not just the calling process.
///
/// # Config and Output
///
/// See the `zng::app::trace_recorder` module documentation for details on how to configure the recording and the output file structure.
///
/// # Panics
///
/// Panics if another `tracing` subscriber was already inited.
///
/// Note that this can cause panics on any subsequent attempt to init subscribers, no other log subscriber must run after recording starts,
/// including attempts to restart recording after stopping.
///
/// Panics cannot write to the output dir.
pub fn start_recording(output_dir: Option<PathBuf>) {
    let mut rec = recording();
    if rec.is_some() {
        // already recording
        return;
    }

    let process_start = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .expect("cannot define process start timestamp")
        .as_micros();

    let output_dir = output_dir.unwrap_or_else(|| std::env::current_dir().expect("`current_dir` error").join("zng-trace"));

    // first process sets the timestamp
    let timestamp = match std::env::var("ZNG_RECORD_TRACE_TIMESTAMP") {
        Ok(t) => t,
        Err(_) => {
            let t = process_start.to_string();
            // SAFETY: safe, only read by this pure Rust code in subsequent started processes.
            unsafe {
                std::env::set_var("ZNG_RECORD_TRACE_TIMESTAMP", t.clone());
            }
            t
        }
    };

    let output_dir = output_dir.join(timestamp);
    std::fs::create_dir_all(&output_dir).expect("cannot create `output_dir`");
    let output_file = output_dir.join(format!("{}.json", std::process::id()));

    let (chrome_layer, guard) = tracing_chrome::ChromeLayerBuilder::new()
        .include_args(true)
        .file(output_file)
        .category_fn(Box::new(|es| match es {
            tracing_chrome::EventOrSpan::Event(event) => format!("{},{}", event.metadata().target(), event.metadata().level()),
            tracing_chrome::EventOrSpan::Span(span_ref) => format!("{},{}", span_ref.metadata().target(), span_ref.metadata().level()),
        }))
        .build();
    *rec = Some(guard);

    let env_layer = EnvFilter::try_from_env("ZNG_RECORD_TRACE_FILTER")
        .or_else(|_| EnvFilter::try_from_default_env())
        .unwrap_or_else(|_| EnvFilter::new("trace"));

    tracing_subscriber::registry().with(env_layer).with(chrome_layer).init();
    zng_env::on_process_exit(|_| stop_recording());

    tracing::info!("zng-record-start: {process_start}");
}

/// Stops recording and flushes.
///
/// Note that this is called automatically on process exit.
pub fn stop_recording() {
    *recording() = None;
}

zng_env::on_process_start!(|_| {
    if std::env::var("ZNG_RECORD_TRACE").is_ok() {
        start_recording(std::env::var("ZNG_RECORD_TRACE_DIR").ok().map(PathBuf::from));
    }
});

zng_app_context::hot_static! {
    static RECORDING: Mutex<Option<tracing_chrome::FlushGuard>> = Mutex::new(None);
}
fn recording() -> parking_lot::MutexGuard<'static, Option<tracing_chrome::FlushGuard>> {
    zng_app_context::hot_static_ref!(RECORDING).lock()
}
