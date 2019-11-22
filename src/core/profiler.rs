#![cfg(feature = "app_profiler")]
use lazy_static::*;
use serde_json::*;

use std::cell::RefCell;
use std::fs::File;
use std::io::BufWriter;
use std::string::String;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;
use std::thread;
use time::precise_time_ns;

lazy_static! {
    static ref GLOBAL_PROFILER: Mutex<Profiler> = Mutex::new(Profiler::new());
}

thread_local!(static THREAD_PROFILER: RefCell<Option<ThreadProfiler>> = RefCell::new(None));

#[derive(Copy, Clone)]
struct ThreadId(usize);

struct ThreadInfo {
    name: String,
}

struct Sample {
    tid: ThreadId,
    name: String,
    t0: u64,
    t1: u64,
}

struct ThreadProfiler {
    id: ThreadId,
    tx: Sender<Sample>,
}

impl ThreadProfiler {
    fn push_sample(&self, name: String, t0: u64, t1: u64) {
        let sample = Sample {
            tid: self.id,
            name,
            t0,
            t1,
        };
        self.tx.send(sample).ok();
    }
}

struct Profiler {
    rx: Receiver<Sample>,
    tx: Sender<Sample>,
    threads: Vec<ThreadInfo>,
}

impl Profiler {
    fn new() -> Profiler {
        let (tx, rx) = channel();

        Profiler {
            rx,
            tx,
            threads: Vec::new(),
        }
    }

    fn register_thread(&mut self) {
        let id = ThreadId(self.threads.len());
        let name = match thread::current().name() {
            Some(s) => s.to_string(),
            None => format!("<unnamed-{}>", id.0),
        };

        self.threads.push(ThreadInfo { name });

        THREAD_PROFILER.with(|profiler| {
            assert!(profiler.borrow().is_none());

            let thread_profiler = ThreadProfiler {
                id,
                tx: self.tx.clone(),
            };

            *profiler.borrow_mut() = Some(thread_profiler);
        });
    }

    fn write_profile(&self, filename: &str) {
        // Stop reading samples that are written after
        // write_profile() is called.
        let start_time = precise_time_ns();
        let mut data = Vec::new();

        while let Ok(sample) = self.rx.try_recv() {
            if sample.t0 > start_time {
                break;
            }

            let thread_id = self.threads[sample.tid.0].name.as_str();
            let t0 = sample.t0 / 1000;
            let t1 = sample.t1 / 1000;

            data.push(json!({
                "pid": 0,
                "tid": thread_id,
                "name": sample.name,
                "ph": "B",
                "ts": t0
            }));

            data.push(json!({
                "pid": 0,
                "tid": thread_id,
                "ph": "E",
                "ts": t1
            }));
        }

        let f = BufWriter::new(File::create(filename).unwrap());
        serde_json::to_writer(f, &data).unwrap();
    }
}

pub struct ProfileScope {
    name: String,
    t0: u64,
}

impl ProfileScope {
    pub fn new(name: String) -> ProfileScope {
        let t0 = precise_time_ns();
        ProfileScope { name, t0 }
    }
}

impl Drop for ProfileScope {
    /// When the ProfileScope is dropped it records the
    /// length of time it was alive for and records it
    /// against the Profiler.
    fn drop(&mut self) {
        let t1 = precise_time_ns();

        THREAD_PROFILER.with(|profiler| match *profiler.borrow() {
            Some(ref profiler) => {
                profiler.push_sample(self.name.clone(), self.t0, t1);
            }
            None => {
                println!("ERROR: ProfileScope {} on unregistered thread!", self.name);
            }
        });
    }
}

/// Writes the global profile to a specific file.
pub fn write_profile(filename: &str) {
    GLOBAL_PROFILER.lock().unwrap().write_profile(filename);
}

/// Registers the current thread with the global profiler.
pub fn register_thread_with_profiler() {
    GLOBAL_PROFILER.lock().unwrap().register_thread();
}
