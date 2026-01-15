use std::{
    io,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, SystemTime},
};

use crate::{FsChangesArgs, WATCHER, WatchFile, WatcherHandle, WatcherSyncWriteNote, WriteFile, service::WATCHER_SV};
use atomic::Atomic;
use parking_lot::Mutex;
use path_absolutize::Absolutize as _;
use zng_clone_move::clmv;
use zng_task as task;
use zng_unit::TimeUnits as _;
use zng_var::{AnyVarHookArgs, Var, VarUpdateId, VarValue, WeakVar, var};

#[cfg(target_has_atomic = "64")]
use std::sync::atomic::AtomicU64;

#[cfg(not(target_has_atomic = "64"))]
struct AtomicU64(Mutex<u64>);
#[cfg(not(target_has_atomic = "64"))]
impl AtomicU64 {
    pub const fn new(u: u64) -> Self {
        Self(Mutex::new(u))
    }

    pub fn load(&self, _: Ordering) -> u64 {
        *self.0.lock()
    }

    pub fn store(&self, _: Ordering, u: u64) {
        *self.0.lock() = u;
    }
}

pub struct SyncWithVar {
    task: Box<dyn Fn(&WatcherHandle, SyncEvent) + Send + Sync>,
    handle: WatcherHandle,
}
impl SyncWithVar {
    pub fn new<O, R, W, U>(handle: WatcherHandle, mut file: PathBuf, init: O, read: R, write: W, var_hook_and_modify: U) -> (Self, Var<O>)
    where
        O: VarValue,
        R: FnMut(io::Result<WatchFile>) -> Option<O> + Send + 'static,
        W: FnMut(O, VarUpdateId, io::Result<WriteFile>) + Send + 'static,
        U: Fn(bool) + Send + Sync + 'static,
    {
        if let Ok(p) = file.absolutize() {
            file = p.into_owned();
        }

        let path = Arc::new(WatcherSyncWriteNote(file));
        let latest_from_read = Arc::new(AtomicBool::new(false));

        let var_hook_and_modify = Arc::new(var_hook_and_modify);

        let var = var(init);
        var.as_any()
            .hook(clmv!(path, latest_from_read, var_hook_and_modify, |args: &AnyVarHookArgs| {
                let is_read = args.downcast_tags::<Arc<WatcherSyncWriteNote>>().any(|n| n == &path);
                latest_from_read.store(is_read, Ordering::Relaxed);
                var_hook_and_modify(is_read);
                true
            }))
            .perm();

        var.hook_drop(|| {
            WATCHER_SV.write().update_sync();
        })
        .perm();

        type PendingFlag = u8;
        const READ: PendingFlag = 0b01;
        const WRITE: PendingFlag = 0b11;

        struct TaskData<R, W, O: VarValue> {
            pending: Atomic<PendingFlag>,
            read_write: Mutex<(R, W)>,
            wk_var: WeakVar<O>,
            last_write: AtomicU64, // ms from epoch
        }
        let task_data = Arc::new(TaskData {
            pending: Atomic::new(0),
            read_write: Mutex::new((read, write)),
            wk_var: var.downgrade(),
            last_write: AtomicU64::new(0),
        });

        // task drains pending, drops handle if the var is dropped.
        let task = Box::new(move |handle: &WatcherHandle, ev: SyncEvent| {
            let var = match task_data.wk_var.upgrade() {
                Some(v) => v,
                None => {
                    handle.clone().force_drop();
                    return;
                }
            };

            let mut debounce = None;
            let mut pending = 0;

            match ev {
                SyncEvent::Update(sync_debounce) => {
                    if var.is_new() && !latest_from_read.load(Ordering::Acquire) {
                        // var updated, not from read
                        debounce = Some(sync_debounce);
                        pending |= WRITE;
                    } else {
                        return;
                    }
                }
                SyncEvent::Event(args) => {
                    if args.rescan() {
                        // file may have updated
                        pending |= READ;
                    } else {
                        'ev: for ev in args.changes_for_path(&path) {
                            for note in ev.notes::<WatcherSyncWriteNote>() {
                                if path.as_path() == note.as_path() {
                                    // we caused this event
                                    continue 'ev;
                                }
                            }

                            // file updated, not from write
                            pending |= READ;
                            break;
                        }
                        if pending == 0 {
                            return;
                        }
                    }
                }
                SyncEvent::Init => {
                    if path.exists() {
                        pending |= READ;
                    } else {
                        pending |= WRITE;
                    }
                }
                SyncEvent::FlushShutdown => {
                    // task is always "flushing", just await the timeout
                    let timeout = WATCHER_SV.read().shutdown_timeout.get();
                    if task_data.read_write.try_lock_for(timeout).is_none() {
                        tracing::error!("not all io operations finished on shutdown, timeout after {timeout:?}");
                    }
                    return;
                }
            };
            drop(var);

            task_data.pending.fetch_or(pending, Ordering::AcqRel);
            if task_data.read_write.try_lock().is_none() {
                // another spawn is already applying
                return;
            }
            task::spawn_wait(clmv!(task_data, path, var_hook_and_modify, handle, || {
                let mut read_write = match task_data.read_write.try_lock() {
                    Some(rw) => rw,
                    None => {
                        // another spawn raced over the external lock
                        return;
                    }
                };
                let (read, write) = &mut *read_write;
                let mut var_update_id_before_read = None;

                loop {
                    let pending = task_data.pending.swap(0, Ordering::AcqRel);

                    if pending == WRITE {
                        if let Some(d) = debounce {
                            let now_ms = SystemTime::now()
                                .duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64;
                            let prev_ms = task_data.last_write.load(Ordering::Relaxed);
                            let elapsed = (now_ms - prev_ms).ms();
                            if elapsed < d {
                                std::thread::sleep(d - elapsed);
                            }
                            task_data.last_write.store(now_ms, Ordering::Relaxed);
                        }

                        let (id, value) = if let Some(var) = task_data.wk_var.upgrade() {
                            // spin until read has applied
                            while var_update_id_before_read == Some(var.last_update()) {
                                std::thread::sleep(10.ms());
                            }
                            var_update_id_before_read = None;
                            (var.last_update(), var.get())
                        } else {
                            handle.force_drop();
                            return;
                        };

                        // write, annotate all changes observed during write with the `path`
                        // that is used to avoid READ caused by our own write
                        {
                            let _note = WATCHER.annotate(path.clone());
                            write(value, id, WriteFile::open(path.to_path_buf()));
                        }

                        if task_data.wk_var.strong_count() == 0 {
                            handle.force_drop();
                            return;
                        }
                    } else if pending == READ {
                        if task_data.wk_var.strong_count() == 0 {
                            handle.force_drop();
                            return;
                        }

                        // read, tags the var update with `path`
                        if let Some(update) = read(WatchFile::open(path.as_path())) {
                            if let Some(var) = task_data.wk_var.upgrade() {
                                var_update_id_before_read = Some(var.last_update());
                                var.modify(clmv!(path, var_hook_and_modify, |vm| {
                                    vm.set(update);
                                    vm.push_tag(path);
                                    var_hook_and_modify(true);
                                }));
                            } else {
                                handle.force_drop();
                                return;
                            }
                        }
                    } else {
                        return;
                    }
                }
            }));
        });

        task(&handle, SyncEvent::Init);

        (Self { task, handle }, var)
    }

    /// Match the event and flag variable update.
    ///
    /// Returns if the variable is still alive.
    pub fn on_event(&mut self, args: &FsChangesArgs) -> bool {
        if !self.handle.is_dropped() {
            (self.task)(&self.handle, SyncEvent::Event(args));
        }
        !self.handle.is_dropped()
    }

    /// Returns if the variable is still alive.
    pub fn retain(&mut self, sync_debounce: Duration) -> bool {
        if !self.handle.is_dropped() {
            (self.task)(&self.handle, SyncEvent::Update(sync_debounce));
        }
        !self.handle.is_dropped()
    }

    pub fn flush_shutdown(&mut self) {
        if !self.handle.is_dropped() {
            (self.task)(&self.handle, SyncEvent::FlushShutdown);
        }
    }
}
enum SyncEvent<'a> {
    Update(Duration),
    Event(&'a FsChangesArgs),
    Init,
    FlushShutdown,
}
