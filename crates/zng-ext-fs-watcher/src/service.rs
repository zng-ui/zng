use std::{
    collections::{HashMap, hash_map},
    io, mem,
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, SystemTime},
};

use atomic::{Atomic, Ordering};
use notify::Watcher;
use parking_lot::Mutex;
use path_absolutize::Absolutize;
use zng_app::{
    DInstant, INSTANT, app_hn_once,
    timer::{DeadlineHandle, TIMERS},
};
use zng_app_context::{LocalContext, app_local};
use zng_clone_move::clmv;
use zng_handle::{Handle, HandleOwner};
use zng_unit::TimeUnits;
use zng_var::{AnyVarHookArgs, VARS, Var, VarUpdateId, VarValue, WeakVar, var};

use crate::{
    FS_CHANGES_EVENT, FsChange, FsChangeNote, FsChangeNoteHandle, FsChangesArgs, WATCHER, WatchFile, WatcherHandle, WatcherReadStatus,
    WatcherSyncStatus, WatcherSyncWriteNote, WriteFile, fs_event,
};

use zng_task as task;

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
        Self {
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
        }
    }

    pub fn init_watcher(&mut self) {
        self.watcher.init();
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

    pub fn update(&mut self) {
        if let Some(n) = self.poll_interval.get_new() {
            self.watcher.set_poll_interval(n);
        }
        if !self.debounce_buffer.is_empty() {
            if let Some(n) = self.debounce.get_new() {
                if self.debounce_oldest.elapsed() >= n {
                    self.notify();
                }
            }
        }
        self.read_to_var.retain_mut(|f| f.retain());
        let sync_debounce = self.sync_debounce.get();
        self.sync_with_var.retain_mut(|f| f.retain(sync_debounce));
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
        if let Ok(r) = &r {
            if !self.watcher.allow(r) {
                // file parent watcher, file not affected.
                return;
            }
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

        self.debounce_buffer.push(FsChange { notes, event: r });

        if notify {
            self.notify();
        } else if self.debounce_timer.is_none() {
            self.debounce_timer = Some(TIMERS.on_deadline(
                self.debounce.get(),
                app_hn_once!(|_| {
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
    move |r| ctx.with_context(|| WATCHER_SV.write().on_watcher(r))
}

struct ReadToVar {
    read: Box<dyn Fn(&Arc<AtomicBool>, &WatcherHandle, ReadEvent) + Send + Sync>,
    pending: Arc<AtomicBool>,
    handle: WatcherHandle,
}
impl ReadToVar {
    fn new<O: VarValue, R: 'static>(
        handle: WatcherHandle,
        mut path: PathBuf,
        init: O,
        load: fn(&Path) -> R,
        read: impl FnMut(R) -> Option<O> + Send + 'static,
        on_modify: impl Fn() + Send + Sync + 'static,
    ) -> (Self, Var<O>) {
        if let Ok(p) = path.absolutize() {
            path = p.into_owned();
        }
        let path = Arc::new(path);
        let var = var(init);
        let on_modify = Arc::new(on_modify);

        let pending = Arc::new(AtomicBool::new(false));
        let read = Arc::new(Mutex::new(read));
        let wk_var = var.downgrade();

        // read task "drains" pending, drops handle if the var is dropped.
        let read = Box::new(move |pending: &Arc<AtomicBool>, handle: &WatcherHandle, ev: ReadEvent| {
            if wk_var.strong_count() == 0 {
                handle.clone().force_drop();
                return;
            }

            let spawn = match ev {
                ReadEvent::Update => false,
                ReadEvent::Event(args) => !pending.load(Ordering::Relaxed) && args.events_for_path(&path).next().is_some(),
                ReadEvent::Init => true,
            };

            if !spawn {
                return;
            }

            pending.store(true, Ordering::Relaxed);
            if read.try_lock().is_none() {
                // another task already running.
                return;
            }
            task::spawn_wait(clmv!(read, wk_var, path, handle, pending, on_modify, || {
                let mut read = read.lock();
                while pending.swap(false, Ordering::Relaxed) {
                    if let Some(update) = read(load(path.as_path())) {
                        if let Some(var) = wk_var.upgrade() {
                            var.modify(clmv!(on_modify, |vm| {
                                vm.set(update);
                                on_modify();
                            }));
                        } else {
                            // var dropped
                            handle.force_drop();
                            break;
                        }
                    }
                }
            }));
        });
        read(&pending, &handle, ReadEvent::Init);

        (Self { read, pending, handle }, var.read_only())
    }

    /// Match the event and flag variable update.
    ///
    /// Returns if the variable is still alive.
    pub fn on_event(&mut self, args: &FsChangesArgs) -> bool {
        if !self.handle.is_dropped() {
            (self.read)(&self.pending, &self.handle, ReadEvent::Event(args));
        }
        !self.handle.is_dropped()
    }

    /// Returns if the variable is still alive.
    fn retain(&mut self) -> bool {
        if !self.handle.is_dropped() {
            (self.read)(&self.pending, &self.handle, ReadEvent::Update);
        }
        !self.handle.is_dropped()
    }
}
enum ReadEvent<'a> {
    Update,
    Event(&'a FsChangesArgs),
    Init,
}

pub(crate) struct SyncWithVar {
    task: Box<dyn Fn(&WatcherHandle, SyncEvent) + Send + Sync>,
    handle: WatcherHandle,
}
impl SyncWithVar {
    fn new<O, R, W, U>(handle: WatcherHandle, mut file: PathBuf, init: O, read: R, write: W, var_hook_and_modify: U) -> (Self, Var<O>)
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
                    if var.is_new() && !latest_from_read.load(Ordering::Relaxed) {
                        debounce = Some(sync_debounce);
                        pending |= WRITE;
                    } else {
                        return;
                    }
                }
                SyncEvent::Event(args) => {
                    if args.rescan() {
                        pending |= READ;
                    } else {
                        'ev: for ev in args.changes_for_path(&path) {
                            for note in ev.notes::<WatcherSyncWriteNote>() {
                                if path.as_path() == note.as_path() {
                                    // we caused this event
                                    continue 'ev;
                                }
                            }

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
                    let timeout = WATCHER_SV.read().shutdown_timeout.get();
                    if task_data.read_write.try_lock_for(timeout).is_none() {
                        tracing::error!("not all io operations finished on shutdown, timeout after {timeout:?}");
                    }
                    return;
                }
            };
            drop(var);

            task_data.pending.fetch_or(pending, Ordering::Relaxed);

            if task_data.read_write.try_lock().is_none() {
                // another spawn is already applying
                return;
            }
            task::spawn_wait(clmv!(task_data, path, var_hook_and_modify, handle, || {
                let mut read_write = task_data.read_write.lock();
                let (read, write) = &mut *read_write;

                loop {
                    let pending = task_data.pending.swap(0, Ordering::Relaxed);

                    if pending == WRITE {
                        if let Some(d) = debounce {
                            let now_ms = SystemTime::now()
                                .duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64;
                            let prev_ms = task_data.last_write.load(Ordering::Relaxed);
                            let elapsed = Duration::from_millis(now_ms - prev_ms);
                            if elapsed < d {
                                std::thread::sleep(d - elapsed);
                            }
                            task_data.last_write.store(now_ms, Ordering::Relaxed);
                        }

                        let (id, value) = if let Some(var) = task_data.wk_var.upgrade() {
                            (var.last_update(), var.get())
                        } else {
                            handle.force_drop();
                            return;
                        };

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

                        if let Some(update) = read(WatchFile::open(path.as_path())) {
                            if let Some(var) = task_data.wk_var.upgrade() {
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
    fn retain(&mut self, sync_debounce: Duration) -> bool {
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

struct Watchers {
    dirs: HashMap<PathBuf, DirWatcher>,
    watcher: Mutex<Box<dyn notify::Watcher + Send>>, // mutex for Sync only
    // watcher for paths that the system watcher cannot watch yet.
    error_watcher: Option<PollWatcher>,
    poll_interval: Duration,
}
impl Watchers {
    fn new() -> Self {
        Self {
            dirs: HashMap::default(),
            watcher: Mutex::new(Box::new(notify::NullWatcher)),
            error_watcher: None,
            poll_interval: 1.secs(),
        }
    }

    fn watch(&mut self, file: PathBuf) -> WatcherHandle {
        self.watch_insert(file, WatchMode::File(std::ffi::OsString::new()))
    }

    fn watch_dir(&mut self, dir: PathBuf, recursive: bool) -> WatcherHandle {
        self.watch_insert(dir, if recursive { WatchMode::Descendants } else { WatchMode::Children })
    }

    /// path can still contain the file name if mode is `WatchMode::File("")`
    fn watch_insert(&mut self, mut path: PathBuf, mut mode: WatchMode) -> WatcherHandle {
        use path_absolutize::*;
        path = match path.absolutize() {
            Ok(p) => p.to_path_buf(),
            Err(e) => {
                tracing::error!("cannot watch `{}`, failed to absolutize `{}`", path.display(), e);
                return WatcherHandle::dummy();
            }
        };

        if let WatchMode::File(name) = &mut mode {
            if let Some(n) = path.file_name() {
                *name = n.to_os_string();
                path.pop();
            } else {
                tracing::error!("cannot watch file `{}`", path.display());
                return WatcherHandle::dummy();
            }
        }

        let w = self.dirs.entry(path.clone()).or_default();

        for (m, handle) in &w.modes {
            if m == &mode {
                if let Some(h) = handle.weak_handle().upgrade() {
                    return WatcherHandle(h);
                }
            }
        }

        let (owner, handle) = Handle::new(());

        let recursive = matches!(&mode, WatchMode::Descendants);

        if w.modes.is_empty() {
            if Self::inner_watch_dir(&mut **self.watcher.get_mut(), &path, recursive).is_err() {
                Self::inner_watch_error_dir(&mut self.error_watcher, &path, recursive, self.poll_interval);
                w.is_in_error_watcher = true;
            }
        } else {
            let was_recursive = w.recursive();
            if !was_recursive && recursive {
                let watcher = &mut **self.watcher.get_mut();

                if mem::take(&mut w.is_in_error_watcher) {
                    Self::inner_unwatch_dir(self.error_watcher.as_mut().unwrap(), &path);
                } else {
                    Self::inner_unwatch_dir(watcher, &path);
                }
                if Self::inner_watch_dir(watcher, &path, recursive).is_err() {
                    Self::inner_watch_error_dir(&mut self.error_watcher, &path, recursive, self.poll_interval);
                }
            }
        }

        w.modes.push((mode, owner));

        WatcherHandle(handle)
    }

    fn cleanup(&mut self) {
        let watcher = &mut **self.watcher.get_mut();
        self.dirs.retain(|k, v| {
            let r = v.retain();
            if !r {
                if v.is_in_error_watcher {
                    Self::inner_unwatch_dir(self.error_watcher.as_mut().unwrap(), k);
                } else {
                    Self::inner_unwatch_dir(watcher, k);
                }
            }
            r
        })
    }

    fn set_poll_interval(&mut self, interval: Duration) {
        self.poll_interval = interval;
        if let Err(e) = self
            .watcher
            .get_mut()
            .configure(notify::Config::default().with_poll_interval(interval))
        {
            tracing::error!("error setting the watcher poll interval: {e}");
        }
        if let Some(w) = &mut self.error_watcher {
            w.configure(notify::Config::default().with_poll_interval(interval)).unwrap();
        }
    }

    fn init(&mut self) {
        *self.watcher.get_mut() = match notify::recommended_watcher(notify_watcher_handler()) {
            Ok(w) => Box::new(w),
            Err(e) => {
                tracing::error!("error creating watcher\n{e}\nfallback to slow poll watcher");
                match PollWatcher::new(
                    notify_watcher_handler(),
                    notify::Config::default().with_poll_interval(self.poll_interval),
                ) {
                    Ok(w) => Box::new(w),
                    Err(e) => {
                        tracing::error!("error creating poll watcher\n{e}\nfs watching disabled");
                        Box::new(notify::NullWatcher)
                    }
                }
            }
        };

        self.cleanup();

        let watcher = &mut **self.watcher.get_mut();
        for (dir, w) in &mut self.dirs {
            let recursive = w.recursive();
            if Self::inner_watch_dir(watcher, dir.as_path(), recursive).is_err() {
                Self::inner_watch_error_dir(&mut self.error_watcher, dir, recursive, self.poll_interval);
                w.is_in_error_watcher = true;
            }
        }
    }

    fn deinit(&mut self) {
        *self.watcher.get_mut() = Box::new(notify::NullWatcher);
    }

    /// Returns Ok, or Err `PathNotFound` or `MaxFilesWatch` that can be handled using the fallback watcher.
    fn inner_watch_dir(watcher: &mut dyn notify::Watcher, dir: &Path, recursive: bool) -> Result<(), notify::ErrorKind> {
        let recursive = if recursive {
            notify::RecursiveMode::Recursive
        } else {
            notify::RecursiveMode::NonRecursive
        };
        if let Err(e) = watcher.watch(dir, recursive) {
            match e.kind {
                notify::ErrorKind::Generic(e) => {
                    if dir.try_exists().unwrap_or(true) {
                        tracing::error!("cannot watch dir `{}`, {e}", dir.display())
                    } else {
                        return Err(notify::ErrorKind::PathNotFound);
                    }
                }
                notify::ErrorKind::Io(e) => {
                    if let io::ErrorKind::NotFound = e.kind() {
                        return Err(notify::ErrorKind::PathNotFound);
                    } else if dir.try_exists().unwrap_or(true) {
                        tracing::error!("cannot watch dir `{}`, {e}", dir.display())
                    } else {
                        return Err(notify::ErrorKind::PathNotFound);
                    }
                }
                e @ notify::ErrorKind::PathNotFound | e @ notify::ErrorKind::MaxFilesWatch => return Err(e),
                notify::ErrorKind::InvalidConfig(e) => unreachable!("{e:?}"),
                notify::ErrorKind::WatchNotFound => unreachable!(),
            }
        }
        Ok(())
    }

    fn inner_watch_error_dir(watcher: &mut Option<PollWatcher>, dir: &Path, recursive: bool, poll_interval: Duration) {
        let watcher = watcher.get_or_insert_with(|| {
            PollWatcher::new(
                notify_watcher_handler(),
                notify::Config::default().with_poll_interval(poll_interval),
            )
            .unwrap()
        });
        Self::inner_watch_dir(watcher, dir, recursive).unwrap();
    }

    fn inner_unwatch_dir(watcher: &mut dyn notify::Watcher, dir: &Path) {
        if let Err(e) = watcher.unwatch(dir) {
            match e.kind {
                notify::ErrorKind::Generic(e) => {
                    tracing::error!("cannot unwatch dir `{}`, {e}", dir.display());
                }
                notify::ErrorKind::Io(e) => {
                    tracing::error!("cannot unwatch dir `{}`, {e}", dir.display());
                }
                notify::ErrorKind::PathNotFound => {}  // ok?
                notify::ErrorKind::WatchNotFound => {} // ok
                notify::ErrorKind::InvalidConfig(_) => unreachable!(),
                notify::ErrorKind::MaxFilesWatch => unreachable!(),
            }
        }
    }

    fn allow(&mut self, r: &fs_event::Event) -> bool {
        if let notify::EventKind::Access(_) = r.kind {
            if !r.need_rescan() {
                return false;
            }
        }

        for (dir, w) in &mut self.dirs {
            let mut matched = false;

            'modes: for (mode, _) in &w.modes {
                match mode {
                    WatchMode::File(f) => {
                        for path in &r.paths {
                            if let Some(name) = path.file_name() {
                                if name == f {
                                    if let Some(path) = path.parent() {
                                        if path == dir {
                                            // matched `dir/exact`
                                            matched = true;
                                            break 'modes;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    WatchMode::Children => {
                        for path in &r.paths {
                            if let Some(path) = path.parent() {
                                if path == dir {
                                    // matched `dir/*`
                                    matched = true;
                                    break 'modes;
                                }
                            }
                        }
                    }
                    WatchMode::Descendants => {
                        for path in &r.paths {
                            if path.starts_with(dir) {
                                // matched `dir/**`
                                matched = true;
                                break 'modes;
                            }
                        }
                    }
                }
            }

            if matched {
                if mem::take(&mut w.is_in_error_watcher) {
                    // poll watcher managed to reach the path without error, try to move to the
                    // more performant system watcher.
                    Self::inner_unwatch_dir(self.error_watcher.as_mut().unwrap(), dir);
                    let recursive = w.recursive();
                    if Self::inner_watch_dir(&mut **self.watcher.get_mut(), dir, recursive).is_err() {
                        // failed again
                        Self::inner_watch_error_dir(&mut self.error_watcher, dir, recursive, self.poll_interval);
                        w.is_in_error_watcher = true;
                    }
                }
                return true;
            }
        }
        false
    }
}

#[derive(PartialEq, Eq)]
enum WatchMode {
    File(std::ffi::OsString),
    Children,
    Descendants,
}

#[derive(Default)]
struct DirWatcher {
    is_in_error_watcher: bool,
    modes: Vec<(WatchMode, HandleOwner<()>)>,
}
impl DirWatcher {
    fn recursive(&self) -> bool {
        self.modes.iter().any(|m| matches!(&m.0, WatchMode::Descendants))
    }

    fn retain(&mut self) -> bool {
        self.modes.retain(|(_, h)| !h.is_dropped());
        !self.modes.is_empty()
    }
}

enum PollMsg {
    Watch(PathBuf, bool),
    Unwatch(PathBuf),
    SetConfig(notify::Config),
}

/// Polling watcher.
///
/// We don't use the `notify` poll watcher to ignore path not found.
struct PollWatcher {
    sender: flume::Sender<PollMsg>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl PollWatcher {
    fn send_msg(&mut self, msg: PollMsg) {
        if self.sender.send(msg).is_err() {
            if let Some(worker) = self.worker.take() {
                if let Err(panic) = worker.join() {
                    std::panic::resume_unwind(panic);
                }
            }
        }
    }
}
impl notify::Watcher for PollWatcher {
    fn new<F: notify::EventHandler>(mut event_handler: F, mut config: notify::Config) -> notify::Result<Self>
    where
        Self: Sized,
    {
        let (sender, rcv) = flume::unbounded();
        let mut dirs = HashMap::<PathBuf, PollInfo, _>::new();
        let worker = std::thread::Builder::new()
            .name(String::from("poll-watcher"))
            .spawn(move || {
                loop {
                    match rcv.recv_timeout(config.poll_interval().unwrap_or_default()) {
                        Ok(msg) => match msg {
                            PollMsg::Watch(d, r) => {
                                let info = PollInfo::new(&d, r);
                                dirs.insert(d, info);
                            }
                            PollMsg::Unwatch(d) => {
                                if dirs.remove(&d).is_none() {
                                    event_handler.handle_event(Err(notify::Error {
                                        kind: notify::ErrorKind::WatchNotFound,
                                        paths: vec![d],
                                    }))
                                }
                            }
                            PollMsg::SetConfig(c) => config = c,
                        },
                        Err(e) => match e {
                            flume::RecvTimeoutError::Timeout => {}           // ok
                            flume::RecvTimeoutError::Disconnected => return, // stop thread
                        },
                    }

                    for (dir, info) in &mut dirs {
                        info.poll(dir, &mut event_handler);
                    }
                }
            })
            .expect("failed to spawn poll-watcher thread");

        Ok(Self {
            sender,
            worker: Some(worker),
        })
    }

    fn watch(&mut self, path: &Path, recursive_mode: notify::RecursiveMode) -> notify::Result<()> {
        let msg = PollMsg::Watch(path.to_path_buf(), matches!(recursive_mode, notify::RecursiveMode::Recursive));
        self.send_msg(msg);
        Ok(())
    }

    fn unwatch(&mut self, path: &Path) -> notify::Result<()> {
        let msg = PollMsg::Unwatch(path.to_path_buf());
        self.send_msg(msg);
        Ok(())
    }

    fn configure(&mut self, option: notify::Config) -> notify::Result<bool> {
        let msg = PollMsg::SetConfig(option);
        self.send_msg(msg);
        Ok(true)
    }

    fn kind() -> notify::WatcherKind
    where
        Self: Sized,
    {
        notify::WatcherKind::PollWatcher
    }
}
#[derive(Default)]
struct PollInfo {
    recursive: bool,
    paths: HashMap<PathBuf, PollEntry>,
    /// entries with `update_flag` not-eq this are removed.
    update_flag: bool,
}
struct PollEntry {
    modified: std::time::SystemTime,
    /// flipped by `recursive_update` if visited.
    update_flag: bool,
}
impl PollInfo {
    fn new(path: &Path, recursive: bool) -> Self {
        let mut paths = HashMap::new();

        for entry in walkdir::WalkDir::new(path)
            .min_depth(1)
            .max_depth(if recursive { usize::MAX } else { 1 })
            .into_iter()
            .flatten()
        {
            if let Some(modified) = entry.metadata().ok().and_then(|m| m.modified().ok()) {
                paths.insert(
                    entry.into_path(),
                    PollEntry {
                        modified,
                        update_flag: false,
                    },
                );
            }
        }

        Self {
            recursive,
            paths,
            update_flag: false,
        }
    }

    fn poll(&mut self, root: &Path, handler: &mut impl notify::EventHandler) {
        self.update_flag = !self.update_flag;
        for entry in walkdir::WalkDir::new(root)
            .min_depth(1)
            .max_depth(if self.recursive { usize::MAX } else { 1 })
            .into_iter()
            .flatten()
        {
            if let Some((is_dir, modified)) = entry.metadata().ok().and_then(|m| Some((m.is_dir(), m.modified().ok()?))) {
                match self.paths.entry(entry.into_path()) {
                    hash_map::Entry::Occupied(mut e) => {
                        let info = e.get_mut();
                        info.update_flag = self.update_flag;
                        if info.modified != modified {
                            info.modified = modified;

                            handler.handle_event(Ok(fs_event::Event {
                                kind: notify::EventKind::Modify(notify::event::ModifyKind::Metadata(
                                    notify::event::MetadataKind::WriteTime,
                                )),
                                paths: vec![e.key().clone()],
                                attrs: Default::default(),
                            }))
                        }
                    }
                    hash_map::Entry::Vacant(e) => {
                        handler.handle_event(Ok(fs_event::Event {
                            kind: notify::EventKind::Create(if is_dir {
                                notify::event::CreateKind::Folder
                            } else {
                                notify::event::CreateKind::File
                            }),
                            paths: vec![e.key().clone()],
                            attrs: Default::default(),
                        }));

                        e.insert(PollEntry {
                            modified,
                            update_flag: self.update_flag,
                        });
                    }
                }
            }
        }

        self.paths.retain(|k, e| {
            let retain = e.update_flag == self.update_flag;
            if !retain {
                handler.handle_event(Ok(fs_event::Event {
                    kind: notify::EventKind::Remove(notify::event::RemoveKind::Any),
                    paths: vec![k.clone()],
                    attrs: Default::default(),
                }));
            }
            retain
        });
    }
}
