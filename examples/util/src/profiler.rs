use std::{
    cell::Cell,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    thread,
};

use rustc_hash::FxHashMap;
use tracing::{span, Level, Subscriber};
use v_jsonescape::escape;

/// Start recording trace level spans and events.
///
/// Call [`Recording::finish`] to stop recording and wait flush.
///
/// Profiles can be viewed using the `chrome://tracing` app.
pub fn record_profile(path: impl AsRef<Path>) -> Recording {
    let mut file = BufWriter::new(File::create(path).unwrap());
    let (sender, recv) = flume::unbounded();

    let worker = thread::spawn(move || {
        let mut spans = FxHashMap::<span::Id, Span>::default();

        struct Span {
            name: &'static str,
            target: &'static str,
            file: Option<&'static str>,
            line: Option<u32>,
        }

        let pid = std::process::id();

        // specs: https://docs.google.com/document/d/1CvAClvFfyA5R-PhYUmn5OOQtYMH4h6I0nSsKchNAySU/preview#heading=h.lpfof2aylapb

        write!(&mut file, "[").unwrap();
        let mut comma = "";
        loop {
            match recv.recv().unwrap() {
                Msg::Event {
                    tid,
                    name,
                    target,
                    file: c_file,
                    line,
                    ts,
                } => {
                    write!(
                        &mut file,
                        r#"{}{{"pid":{},"tid":{},"ts":{},"ph":"i","name":"{}","args":{{"target":"{}""#,
                        comma,
                        pid,
                        tid,
                        ts,
                        escape(name),
                        escape(target)
                    )
                    .unwrap();
                    if let Some(f) = c_file {
                        write!(&mut file, r#","file":"{}""#, escape(f)).unwrap();
                    }
                    if let Some(l) = line {
                        write!(&mut file, r#","line":{}"#, l).unwrap();
                    }
                    write!(&mut file, "}}}}").unwrap();
                    comma = ",";
                }
                Msg::Enter { id, tid, ts } => {
                    let span = spans.get(&id).unwrap();
                    write!(
                        &mut file,
                        r#"{}{{"pid":{},"tid":{},"name":"{}","ph":"B","ts":{},"args":{{"target":"{}""#,
                        comma,
                        pid,
                        tid,
                        escape(span.name),
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
                    write!(&mut file, "}}}}").unwrap();
                    comma = ",";
                }
                Msg::Exit { id, tid, ts } => {
                    let _ = id;
                    write!(&mut file, r#"{}{{"pid":{},"tid":{},"ph":"E","ts":{}}}"#, comma, pid, tid, ts).unwrap();
                    comma = ",";
                }
                Msg::NewSpan {
                    id,
                    name,
                    target,
                    file,
                    line,
                } => {
                    spans.insert(id, Span { name, target, file, line });
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
        write!(&mut file, "]").unwrap();

        file.flush().unwrap();
    });

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
        name: &'static str,
        target: &'static str,
        file: Option<&'static str>,
        line: Option<u32>,
    },

    Event {
        tid: u64,
        name: &'static str,
        target: &'static str,
        file: Option<&'static str>,
        line: Option<u32>,
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
                self.sender
                    .send(Msg::ThreadInfo {
                        id: tid,
                        name: thread::current()
                            .name()
                            .map(|n| escape(n).to_string())
                            .unwrap_or_else(|| format!("<{:?}>", tid)),
                    })
                    .unwrap();
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

        self.sender
            .send(Msg::NewSpan {
                id: id.clone(),
                name: meta.name(),
                target: meta.target(),
                file: meta.file(),
                line: meta.line(),
            })
            .unwrap();

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

        self.sender
            .send(Msg::Event {
                tid,
                name: meta.name(),
                target: meta.target(),
                file: meta.file(),
                line: meta.line(),
                ts,
            })
            .unwrap();
    }

    fn enter(&self, span: &span::Id) {
        let ts = time_ns();

        let tid = self.thread_id();

        self.sender.send(Msg::Enter { id: span.clone(), tid, ts }).unwrap();
    }

    fn exit(&self, span: &span::Id) {
        let ts = time_ns();

        let tid = self.thread_id();

        self.sender.send(Msg::Exit { id: span.clone(), tid, ts }).unwrap();
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
