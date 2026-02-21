use std::{mem, path::PathBuf, sync::Arc, time::Duration};

use parking_lot::{Condvar, Mutex};
use zng_app::{
    APP, DInstant, INSTANT, hn_once,
    timer::{DeadlineHandle, TIMERS},
};
use zng_app_context::{LocalContext, app_local};
use zng_unit::TimeUnits;
use zng_var::{Var, var};

use crate::{FS_CHANGES_EVENT, FsChange, FsChangeNote, FsChangeNoteHandle, FsChangesArgs, WatcherHandle, fs_event};

mod watchers;
use watchers::*;

// mod read_to_var;
// use read_to_var::*;

// mod sync_with_var;
// use sync_with_var::*;

app_local! {
    pub(crate) static WATCHER_SV: WatcherService = WatcherService::new();
}

pub(crate) struct WatcherService {
    pub debounce: Var<Duration>,
    pub sync_debounce: Var<Duration>,
    pub poll_interval: Var<Duration>,
    pub shutdown_timeout: Var<Duration>,

    watcher: Watchers,

    debounce_oldest: DInstant,
    debounce_buffer: Vec<FsChange>,
    debounce_timer: Option<DeadlineHandle>,

    sync_writing: Vec<std::sync::Weak<SyncFlushLock>>,

    notes: Vec<std::sync::Weak<Arc<dyn FsChangeNote>>>,
}
impl WatcherService {
    fn new() -> Self {
        let mut s = Self {
            debounce: var(100.ms()),
            sync_debounce: var(100.ms()),
            poll_interval: var(1.secs()),
            shutdown_timeout: var(1.minutes()),
            watcher: Watchers::new(),
            debounce_oldest: INSTANT.now(),
            debounce_buffer: vec![],
            debounce_timer: None,
            sync_writing: vec![],
            notes: vec![],
        };

        APP.on_deinit(|_| {
            WATCHER_SV.write().shutdown();

            // await writes
            let w = mem::take(&mut WATCHER_SV.write().sync_writing);
            for l in w {
                if let Some(l) = l.upgrade() {
                    l.wait_flush();
                }
            }
        });

        s.poll_interval
            .hook(|n| {
                WATCHER_SV.write().watcher.set_poll_interval(*n.value());
                true
            })
            .perm();

        s.debounce
            .hook(|n| {
                let mut s = WATCHER_SV.write();
                if s.debounce_oldest.elapsed() >= *n.value() {
                    s.notify();
                }
                true
            })
            .perm();

        s.watcher.init();
        s
    }

    pub fn watch(&mut self, file: PathBuf) -> WatcherHandle {
        self.watcher.watch(file)
    }

    pub fn watch_dir(&mut self, dir: PathBuf, recursive: bool) -> WatcherHandle {
        self.watcher.watch_dir(dir, recursive)
    }

    fn on_watcher(&mut self, r: Result<fs_event::Event, fs_event::Error>) {
        if let Ok(r) = &r
            && !self.watcher.allow(r)
        {
            // file parent watcher, file not affected.
            return;
        }

        let notify = self.debounce_oldest.elapsed() >= self.debounce.get();

        let mut notes = Vec::with_capacity(self.notes.len());
        self.notes.retain(|n| match n.upgrade() {
            Some(n) => {
                notes.push(Arc::clone(&*n));
                true
            }
            None => false,
        });

        self.debounce_buffer.push(FsChange {
            notes,
            event: r.map_err(Arc::new),
        });

        if notify {
            self.notify();
        } else if self.debounce_timer.is_none() {
            self.debounce_timer = Some(TIMERS.on_deadline(
                self.debounce.get(),
                hn_once!(|_| {
                    WATCHER_SV.write().on_debounce_timer();
                }),
            ));
        }
    }

    pub fn annotate(&mut self, note: Arc<dyn FsChangeNote>) -> FsChangeNoteHandle {
        let handle = Arc::new(note);
        self.notes.push(Arc::downgrade(&handle));
        FsChangeNoteHandle(handle)
    }

    fn on_debounce_timer(&mut self) {
        if !self.debounce_buffer.is_empty() {
            self.notify();
        }
    }

    fn notify(&mut self) {
        let changes = mem::take(&mut self.debounce_buffer);
        let now = INSTANT.now();
        self.debounce_oldest = now;
        self.debounce_timer = None;

        FS_CHANGES_EVENT.notify(FsChangesArgs::new(now, Default::default(), changes));
    }

    /// Deinit watcher, returns items to flush without a service lock.
    pub(crate) fn shutdown(&mut self) {
        self.watcher.deinit();
    }

    pub(crate) fn push_sync_flush(&mut self, f: &Arc<SyncFlushLock>) {
        if self.sync_writing.len() > 50 {
            self.sync_writing.retain(|f| f.strong_count() > 0);
        }
        self.sync_writing.push(Arc::downgrade(f));
    }
}
fn notify_watcher_handler() -> impl notify::EventHandler {
    let mut ctx = LocalContext::capture();
    move |r| {
        ctx.with_context(|| {
            // this is an attempt to workaround https://github.com/notify-rs/notify/issues/463
            // we have observed deadlocks inside notify code with a debugger
            zng_task::spawn(async move { WATCHER_SV.write().on_watcher(r) });
        })
    }
}

pub(crate) struct SyncFlushLock {
    debouncing: (Mutex<bool>, Condvar),
}
// on_deinit methods
impl SyncFlushLock {
    fn wait_flush(&self) {
        // unblock debounce lock early
        {
            let mut is_deinit = self.debouncing.0.lock();
            *is_deinit = true;
            self.debouncing.1.notify_one();
        }
        // block until write finishes
        drop(self.debouncing.0.lock());
    }
}
// task methods
impl SyncFlushLock {
    pub(crate) fn new() -> Self {
        Self {
            debouncing: (Mutex::new(false), Condvar::new()),
        }
    }

    pub(crate) fn begin_write(&self, debounce: Duration) -> impl Drop {
        let mut is_deinit = self.debouncing.0.lock();
        if !*is_deinit {
            // if not deinit signaled block until debounce timeout, or until deinit
            self.debouncing.1.wait_for(&mut is_deinit, debounce);
        }
        // caller must cover the write op with this guard
        is_deinit
    }
}
