use std::{
    io, mem,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use atomic::{Atomic, Ordering};
use zng_app::{
    APP, DInstant, INSTANT, hn_once,
    timer::{DeadlineHandle, TIMERS},
    view_process::raw_events::LOW_MEMORY_EVENT,
};
use zng_app_context::{LocalContext, app_local};
use zng_clone_move::clmv;
use zng_unit::TimeUnits;
use zng_var::{VARS, Var, VarUpdateId, VarValue, var};

use crate::{
    FS_CHANGES_EVENT, FsChange, FsChangeNote, FsChangeNoteHandle, FsChangesArgs, WatchFile, WatcherHandle, WatcherReadStatus,
    WatcherSyncStatus, WriteFile, fs_event,
};

mod watchers;
use watchers::*;

mod read_to_var;
use read_to_var::*;

mod sync_with_var;
use sync_with_var::*;

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

    read_to_var: Vec<ReadToVar>,
    sync_with_var: Vec<SyncWithVar>,

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
            read_to_var: vec![],
            sync_with_var: vec![],
            notes: vec![],
        };

        FS_CHANGES_EVENT
            .hook(|a| {
                WATCHER_SV.write().event(a);
                true
            })
            .perm();

        LOW_MEMORY_EVENT
            .hook(|_| {
                WATCHER_SV.write().low_memory();
                true
            })
            .perm();

        APP.on_deinit(|_| {
            let mut flush = WATCHER_SV.write().shutdown();
            for v in &mut flush {
                v.flush_shutdown();
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
        s.sync_debounce
            .hook(|_| {
                WATCHER_SV.write().update_sync();
                true
            })
            .perm();

        s.watcher.init();
        s
    }

    pub fn update_read(&mut self) {
        self.read_to_var.retain_mut(|f| f.retain());
    }

    pub fn update_sync(&mut self) {
        let sync_debounce = self.sync_debounce.get();
        self.sync_with_var.retain_mut(|f| f.retain(sync_debounce));
    }

    pub fn event(&mut self, args: &FsChangesArgs) {
        self.read_to_var.retain_mut(|f| f.on_event(args));
        self.sync_with_var.retain_mut(|f| f.on_event(args));
    }

    pub fn low_memory(&mut self) {
        self.read_to_var.retain_mut(|v| v.retain());
        let sync_debounce = self.sync_debounce.get();
        self.sync_with_var.retain_mut(|v| v.retain(sync_debounce));
    }

    pub fn watch(&mut self, file: PathBuf) -> WatcherHandle {
        self.watcher.watch(file)
    }

    pub fn watch_dir(&mut self, dir: PathBuf, recursive: bool) -> WatcherHandle {
        self.watcher.watch_dir(dir, recursive)
    }

    pub fn read<O: VarValue>(
        &mut self,
        file: PathBuf,
        init: O,
        read: impl FnMut(io::Result<WatchFile>) -> Option<O> + Send + 'static,
    ) -> Var<O> {
        let handle = self.watch(file.clone());
        fn open(p: &Path) -> io::Result<WatchFile> {
            WatchFile::open(p)
        }
        let (read, var) = ReadToVar::new(handle, file, init, open, read, || {});
        self.read_to_var.push(read);
        var
    }

    pub fn read_status<O, S, E>(
        &mut self,
        file: PathBuf,
        init: O,
        mut read: impl FnMut(io::Result<WatchFile>) -> Result<Option<O>, E> + Send + 'static,
    ) -> (Var<O>, Var<S>)
    where
        O: VarValue,
        S: WatcherReadStatus<E>,
    {
        let handle = self.watch(file.clone());
        fn open(p: &Path) -> io::Result<WatchFile> {
            WatchFile::open(p)
        }
        let status = var(S::reading());

        let (read, var) = ReadToVar::new(
            handle,
            file,
            init,
            open,
            // read
            clmv!(status, |d| {
                status.set(S::reading());
                match read(d) {
                    Ok(r) => {
                        if r.is_none() {
                            status.set(S::idle());
                        }
                        r
                    }
                    Err(e) => {
                        status.set(S::read_error(e));
                        None
                    }
                }
            }),
            // on_modify
            clmv!(status, || {
                status.set(S::idle());
            }),
        );
        self.read_to_var.push(read);

        (var, status.read_only())
    }

    pub fn read_dir<O: VarValue>(
        &mut self,
        dir: PathBuf,
        recursive: bool,
        init: O,
        read: impl FnMut(walkdir::WalkDir) -> Option<O> + Send + 'static,
    ) -> Var<O> {
        let handle = self.watch_dir(dir.clone(), recursive);
        fn open(p: &Path) -> walkdir::WalkDir {
            walkdir::WalkDir::new(p).min_depth(1).max_depth(1)
        }
        fn open_recursive(p: &Path) -> walkdir::WalkDir {
            walkdir::WalkDir::new(p).min_depth(1)
        }
        let (read, var) = ReadToVar::new(handle, dir, init, if recursive { open_recursive } else { open }, read, || {});
        self.read_to_var.push(read);
        var
    }
    pub fn read_dir_status<O, S, E>(
        &mut self,
        dir: PathBuf,
        recursive: bool,
        init: O,
        mut read: impl FnMut(walkdir::WalkDir) -> Result<Option<O>, E> + Send + 'static,
    ) -> (Var<O>, Var<S>)
    where
        O: VarValue,
        S: WatcherReadStatus<E>,
    {
        let status = var(S::reading());

        let handle = self.watch_dir(dir.clone(), recursive);
        fn open(p: &Path) -> walkdir::WalkDir {
            walkdir::WalkDir::new(p).min_depth(1).max_depth(1)
        }
        fn open_recursive(p: &Path) -> walkdir::WalkDir {
            walkdir::WalkDir::new(p).min_depth(1)
        }

        let (read, var) = ReadToVar::new(
            handle,
            dir,
            init,
            if recursive { open_recursive } else { open },
            // read
            clmv!(status, |d| {
                status.set(S::reading());
                match read(d) {
                    Ok(r) => {
                        if r.is_none() {
                            status.set(S::idle());
                        }
                        r
                    }
                    Err(e) => {
                        status.set(S::read_error(e));
                        None
                    }
                }
            }),
            // on_modify
            clmv!(status, || {
                status.set(S::idle());
            }),
        );
        self.read_to_var.push(read);

        (var, status.read_only())
    }

    pub fn sync<O: VarValue>(
        &mut self,
        file: PathBuf,
        init: O,
        read: impl FnMut(io::Result<WatchFile>) -> Option<O> + Send + 'static,
        mut write: impl FnMut(O, io::Result<WriteFile>) + Send + 'static,
    ) -> Var<O> {
        let handle = self.watch(file.clone());

        let (sync, var) = SyncWithVar::new(handle, file, init, read, move |o, _, f| write(o, f), |_| {});
        self.sync_with_var.push(sync);
        var
    }

    pub fn sync_status<O, S, ER, EW>(
        &mut self,
        file: PathBuf,
        init: O,
        mut read: impl FnMut(io::Result<WatchFile>) -> Result<Option<O>, ER> + Send + 'static,
        mut write: impl FnMut(O, io::Result<WriteFile>) -> Result<(), EW> + Send + 'static,
    ) -> (Var<O>, Var<S>)
    where
        O: VarValue,
        S: WatcherSyncStatus<ER, EW>,
    {
        let handle = self.watch(file.clone());
        let latest_write = Arc::new(Atomic::new(VarUpdateId::never()));

        let status = var(S::reading());
        let (sync, var) = SyncWithVar::new(
            handle,
            file,
            init,
            // read
            clmv!(status, |f| {
                status.set(S::reading());
                match read(f) {
                    Ok(r) => {
                        if r.is_none() {
                            status.set(S::idle());
                        }
                        r
                    }
                    Err(e) => {
                        status.set(S::read_error(e));
                        None
                    }
                }
            }),
            // write
            clmv!(status, latest_write, |o, o_id, f| {
                status.set(S::writing()); // init write
                match write(o, f) {
                    Ok(()) => {
                        if latest_write.load(Ordering::Relaxed) == o_id {
                            status.set(S::idle());
                        }
                    }
                    Err(e) => {
                        status.set(S::write_error(e));
                    }
                }
            }),
            // hook&modify
            clmv!(status, |is_read| {
                status.set(if is_read {
                    S::idle()
                } else {
                    let id = VARS.update_id();
                    latest_write.store(id, Ordering::Relaxed);

                    S::writing()
                });
            }),
        );

        self.sync_with_var.push(sync);

        (var, status.read_only())
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
    pub(crate) fn shutdown(&mut self) -> Vec<SyncWithVar> {
        self.watcher.deinit();
        mem::take(&mut self.sync_with_var)
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
