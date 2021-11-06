//! Performance profiling.
//!
//! Crate must be compiled with the `app_profiler`. See [`profile_scope!`] and [`write_profile`] for more details.
//!
//! Profiler can be viewed using the `chrome://tracing` app.
//!
//! [`profile_scope!`]: crate::profiler::profile_scope
//! [`write_profile`]: crate::profiler::write_profile

#[cfg(feature = "app_profiler")]
#[cfg_attr(doc_nightly, doc(cfg(feature = "app_profiler")))]
mod profiler_impl {
    use serde_json::*;

    use crate::text::Text;
    use flume::{unbounded, Receiver, Sender};
    use parking_lot::{const_mutex, Mutex};
    use std::cell::RefCell;
    use std::fs::File;
    use std::io::BufWriter;
    use std::string::String;
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    static GLOBAL_PROFILER: Mutex<Option<Profiler>> = const_mutex(None);

    thread_local!(static THREAD_PROFILER: RefCell<Option<ThreadProfiler>> = RefCell::new(None));

    #[derive(Copy, Clone)]
    struct ThreadId(usize);

    struct ThreadInfo {
        name: String,
    }

    struct Sample {
        tid: ThreadId,
        name: Text,
        t0: u64,
        t1: u64,
    }

    struct ThreadProfiler {
        id: ThreadId,
        tx: Sender<Sample>,
    }

    impl ThreadProfiler {
        fn push_sample(&self, name: Text, t0: u64, t1: u64) {
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
            let (tx, rx) = unbounded();

            Profiler {
                rx,
                tx,
                threads: Vec::new(),
            }
        }

        fn register_thread(&mut self) {
            let registered_name = THREAD_PROFILER.with(|profiler| {
                if profiler.borrow().is_none() {
                    let id = ThreadId(self.threads.len());

                    let thread_profiler = ThreadProfiler { id, tx: self.tx.clone() };
                    *profiler.borrow_mut() = Some(thread_profiler);

                    Some(match thread::current().name() {
                        Some(s) => s.to_string(),
                        None => format!("<unnamed-{}>", id.0),
                    })
                } else {
                    None
                }
            });
            if let Some(name) = registered_name {
                self.threads.push(ThreadInfo { name });
            }
        }

        fn write_profile(&self, filename: &str, ignore_0ms: bool) {
            // Stop reading samples that are written after
            // write_profile() is called.
            let start_time = precise_time_ns();
            let mut data = Vec::new();

            let p_id = std::process::id();

            while let Ok(sample) = self.rx.try_recv() {
                if sample.t0 > start_time {
                    break;
                }

                let thread_id = self.threads[sample.tid.0].name.as_str();
                let t0 = sample.t0 / 1000;
                let t1 = sample.t1 / 1000;

                if ignore_0ms && t0 == t1 {
                    continue;
                }

                data.push(json!({
                    "pid": p_id,
                    "tid": thread_id,
                    "name": sample.name.as_ref(),
                    "ph": "B",
                    "ts": t0
                }));

                data.push(json!({
                    "pid": p_id,
                    "tid": thread_id,
                    "ph": "E",
                    "ts": t1
                }));
            }

            let f = BufWriter::new(File::create(filename).unwrap());
            serde_json::to_writer(f, &data).unwrap();
        }
    }

    /// Named profile scope. The scope start time is when [`new`](ProfileScope::new) is called,
    /// the scope duration is the time it was alive.
    ///
    /// # Example
    ///
    /// ```
    /// # use zero_ui_core::profiler::ProfileScope;
    /// # fn do_thing() { }
    /// # fn do_another_thing() { }
    /// {
    ///     #[cfg(feature = "app_profiler")]
    ///     let _scope = ProfileScope::new("do-things");
    ///
    ///     do_thing();
    ///     do_another_thing();
    /// }
    /// ```
    ///
    /// # Macro
    ///
    /// For basic usage like in the example there is also the [`profile_scope!`](macro.profile_scope.html) macro.
    #[cfg_attr(doc_nightly, doc(cfg(feature = "app_profiler")))]
    pub struct ProfileScope {
        name: Text,
        t0: u64,
    }
    impl ProfileScope {
        /// Starts a new profile scope, the start time is when this method is called.
        pub fn new(name: impl Into<Text>) -> ProfileScope {
            let t0 = precise_time_ns();
            ProfileScope { name: name.into(), t0 }
        }
    }
    impl Drop for ProfileScope {
        /// When the `ProfileScope` is dropped it records the
        /// length of time it was alive for and records it
        /// against the Profiler.
        fn drop(&mut self) {
            let t1 = precise_time_ns();

            THREAD_PROFILER.with(|profiler| match *profiler.borrow() {
                Some(ref profiler) => {
                    profiler.push_sample(std::mem::take(&mut self.name), self.t0, t1);
                }
                None => {
                    println!("ERROR: ProfileScope {} on unregistered thread!", self.name);
                }
            });
        }
    }

    /// Writes the global profile to a specific file.
    #[inline]
    #[cfg_attr(doc_nightly, doc(cfg(feature = "app_profiler")))]
    pub fn write_profile(filename: &str, ignore_0ms: bool) {
        GLOBAL_PROFILER
            .lock()
            .get_or_insert_with(Profiler::new)
            .write_profile(filename, ignore_0ms);
    }

    /// Registers the current thread with the global profiler.
    ///
    /// Does nothing if the thread is already registered.
    #[inline]
    #[cfg_attr(doc_nightly, doc(cfg(feature = "app_profiler")))]
    pub fn register_thread_with_profiler() {
        GLOBAL_PROFILER.lock().get_or_insert_with(Profiler::new).register_thread();
    }

    fn precise_time_ns() -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64
    }
}

#[cfg(feature = "app_profiler")]
pub use profiler_impl::*;

///<span data-inline></span> Declares a [`ProfileScope`](crate::profiler::ProfileScope) variable if
/// the `app_profiler` feature is active.
///
/// # Example
///
/// If compiled with the `app_profiler` feature, this will register a "do-things" scope
/// that starts when the macro was called and has the duration of the block.
/// ```
/// # use zero_ui_core::profiler::profile_scope;
/// # fn main()
/// {
/// # fn do_thing() { }
/// # fn do_another_thing() { }
///     profile_scope!("do-things");
///
///     do_thing();
///     do_another_thing();
/// }
/// ```
///
/// You can also format strings:
/// ```
/// # use zero_ui_core::profiler::profile_scope;
/// # let thing = "";
/// profile_scope!("do-{}", thing);
/// ```
#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {
        #[cfg(feature = "app_profiler")]
        let _profile_scope =
        $crate::profiler::ProfileScope::new($name);
    };
    ($($args:tt)+) => {
        #[cfg(feature = "app_profiler")]
        let _profile_scope =
        $crate::profiler::ProfileScope::new(format!($($args)+));
    };
}
#[doc(inline)]
pub use crate::profile_scope;
