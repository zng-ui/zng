#![cfg(feature = "trace_recorder")]

//! Trace recording and data model.
//!
//! All tracing instrumentation in Zng projects is done using the `tracing` crate, this module uses the `tracing-chrome` crate
//! to record traces that can be viewed in `chrome://tracing` or `ui.perfetto.dev` and can be parsed to the [`Trace`] data model.

use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use parking_lot::Mutex;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};
use zng_txt::Txt;

/// Represents a recorded trace.
#[non_exhaustive]
pub struct Trace {
    /// Traced app processes.
    pub processes: Vec<ProcessTrace>,
}

/// Represents a single app process in a recorded trace.
#[non_exhaustive]
pub struct ProcessTrace {
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
#[non_exhaustive]
pub struct ThreadTrace {
    /// Thread name.
    pub name: Txt,

    /// Events that happened on the thread.
    pub events: Vec<EventTrace>,
    /// Spans started and ended on the thread.
    pub spans: Vec<SpanTrace>,
}

/// Represents a traced event.
#[non_exhaustive]
pub struct EventTrace {
    /// Event info.
    pub info: Info,
    /// Moment from the recording start when this event happened.
    pub instant: Duration,
}

/// Represents a traced span.
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
pub struct Info {
    /// Event or span name.
    pub name: Txt,
    /// Function where the event or span was traced.
    pub target: Txt,
    /// File where the event or span was traced.
    pub file: Option<Txt>,
    /// Code line where the event or span was traced.
    pub line: Option<u32>,
    /// Custom args traced with the event or span.
    pub args: Option<HashMap<Txt, Txt>>,
}

impl Trace {
    /// Read and parse a Chrome JSON Array format trace.
    pub fn read_chrome_trace(json_path: impl AsRef<Path>) -> io::Result<Self> {
        let json = std::fs::read_to_string(json_path)?;
        let trace = Self::parse_chrome_trace(&json)?;
        Ok(trace)
    }

    /// Parse a Chrome JSON Array format trace.
    ///
    /// You can use the `tracing_chrome` crate to collect traces.
    pub fn parse_chrome_trace(json: &str) -> io::Result<Self> {
        todo!()
    }

    /// Convert the trace to Chrome JSON Array format.
    pub fn to_chrome_trace(&self) -> Txt {
        todo!()
    }

    /// Convert and write the trace to Chrome JSON Array format.
    pub fn write_chrome_trace(&self, json_path: impl AsRef<Path>) -> io::Result<()> {
        std::fs::write(json_path, self.to_chrome_trace().as_str().as_bytes())
    }

    /// Merge `other` into this.
    pub fn merge(&mut self, other: Self) {
        for p in other.processes {
            if let Some(ep) = self.processes.iter_mut().find(|ep| ep.name == p.name) {
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

/// Starts recording, stops on process exit or on [`stop_recording`].
///
/// Note that this is called automatically on startup if the `ZNG_RECORD_TRACE` environment variable is set and that is
/// the recommended way of recording traces.
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
        .build();
    *rec = Some(guard);

    tracing_subscriber::registry().with(chrome_layer).init();
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
    if let Ok(args) = std::env::var("ZNG_RECORD_TRACE") {
        // !!: TODO handle args
        start_recording(None);
    }
});

zng_app_context::hot_static! {
    static RECORDING: Mutex<Option<tracing_chrome::FlushGuard>> = Mutex::new(None);
}
fn recording() -> parking_lot::MutexGuard<'static, Option<tracing_chrome::FlushGuard>> {
    zng_app_context::hot_static_ref!(RECORDING).lock()
}
