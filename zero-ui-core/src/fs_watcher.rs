//! File system events and service.

use std::{
    fs::{self, File},
    io, mem,
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc},
    time::{Duration, Instant},
};

use atomic::Ordering;
use parking_lot::Mutex;

use crate::{
    app::AppExtension,
    context::app_local,
    crate_util::HandleOwner,
    event::{event, event_args, EventHandle},
    handler::{app_hn_once, AppHandler, FilterAppHandler},
    task,
    timer::{DeadlineHandle, TIMERS},
    units::*,
    var::*,
};

/// Application extension that provides file system change events and service.
///
/// # Events
///
/// Events this extension provides.
///
/// * [`FS_CHANGES_EVENT`]
///
/// # Services
///
/// Services this extension provides.
///
/// * [`WATCHER`]
#[derive(Default)]
pub struct FsWatcherManager {}
impl AppExtension for FsWatcherManager {
    fn init(&mut self) {
        WATCHER_SV.write().init_watcher();
    }

    fn event_preview(&mut self, update: &mut crate::event::EventUpdate) {
        if let Some(args) = FS_CHANGES_EVENT.on(update) {
            WATCHER_SV.write().event(args);
        }
    }

    fn update_preview(&mut self) {
        WATCHER_SV.write().update();
    }
}

/// File system watcher service.
///
/// This is mostly a wrapper around the [`notify`] crate, integrating it with events and variables.
pub struct WATCHER;
impl WATCHER {
    /// Gets a read-write variable that interval awaited before a [`FS_CHANGES_EVENT`] is emitted. If
    /// a watched path is constantly changing an event will be emitted every elapse of this interval,
    /// the event args will contain a list of all the changes observed during the interval.
    ///
    /// Is `100.ms()` by default, this helps secure the app against being overwelmed, and to detect
    /// file changes when the file is temporarly removed and another file move to have its name.
    pub fn debounce(&self) -> ArcVar<Duration> {
        WATCHER_SV.read().debounce.clone()
    }

    /// When an efficient watcher cannot be used a poll watcher fallback is used, the poll watcher reads
    /// the directory or path every elapse of this interval.
    ///
    /// Is `10.secs()` by default.
    pub fn poll_interval(&self) -> ArcVar<Duration> {
        WATCHER_SV.read().poll_interval.clone()
    }

    /// Enable file change events for the `file`.
    ///
    /// Returns a handle that will stop the file watch when dropped, if there is no other active handler for the same file.
    ///
    /// Note that this is implemented by actually watching the parent directory and filtering the events, this is done
    /// to ensure the watcher survives operations that remove the file and then move another file to the same path.
    ///
    /// See [`watch_dir`] for more details.
    ///
    /// [`watch_dir`]: WATCHER::watch_dir
    pub fn watch(&self, file: impl Into<PathBuf>) -> WatcherHandle {
        WATCHER_SV.write().watch(file.into())
    }

    /// Enable file change events for files inside `dir`, also include inner directories if `recursive` is `true`.
    ///
    /// Returns a handle that will stop the dir watch when dropped, if there is no other active handler for the same directory.
    ///
    /// The directory will be watched using an OS specific efficient watcher provided by the [`notify`] crate. If there is
    /// any error creating the watcher, such as if the directory does not exist yet a slower polling watcher will retry periodically    
    /// until the efficient watcher can be created or the handle is dropped.
    pub fn watch_dir(&self, dir: impl Into<PathBuf>, recursive: bool) -> WatcherHandle {
        WATCHER_SV.write().watch_dir(dir.into(), recursive)
    }

    /// Read a file into a variable, the `init` value will start the variable and the `read` closure will be called
    /// once imediatly and every time the file changes, if the closure returns `Some(O)` the variable updates with the new value.
    ///
    /// Dropping the variable drops the read watch. The `read` closure is non-blocking, it is called in a [`task::wait`]
    /// background thread.
    pub fn read<O: VarValue>(
        &self,
        file: impl Into<PathBuf>,
        init: O,
        read: impl FnMut(io::Result<File>) -> Option<O> + Send + 'static,
    ) -> ReadOnlyArcVar<O> {
        let path = file.into();
        let handle = self.watch(path);
        let (read, var) = ReadToVar::new(handle, path, init, std::fs::File::open, read);
        WATCHER_SV.write().read_to_var.push(read);
        var
    }

    /// Read a directory into a variable,  the `init` value will start the variable and the `read` closure will be called
    /// once imediatly and every time any changes happen inside the dir, if the closure returns `Some(O)` the variable updates with the new value.
    ///
    /// Dropping the variable drops the read watch. The `read` closure is non-blocking, it is called in a [`task::wait`]
    /// background thread.
    pub fn read_dir<O: VarValue>(
        &self,
        dir: impl Into<PathBuf>,
        recursive: bool,
        init: O,
        read: impl FnMut(io::Result<fs::ReadDir>) -> Option<O> + Send + 'static,
    ) -> ReadOnlyArcVar<O> {
        let path = dir.into();
        let handle = self.watch_dir(path, recursive);
        let (read, var) = ReadToVar::new(handle, path, init, std::fs::read_dir, read);
        WATCHER_SV.write().read_to_var.push(read);
        var
    }

    /// Watch `file` and calls `handler` every time it changes.
    ///
    /// Note that the `handler` is blocking, use [`async_app_hn!`] and [`task::wait`] to run IO without
    /// blocking the app.
    pub fn on_file_changed(&self, file: impl Into<PathBuf>, handler: impl AppHandler<FsChangesArgs>) -> EventHandle {
        let file = file.into();
        let handle = self.watch(file.clone());
        FS_CHANGES_EVENT.on_event(FilterAppHandler::new(handler, move |args| {
            let _handle = handle;
            args.events_for_path(&file).is_some()
        }))
    }

    /// Watch `dir` and calls `handler` every time something inside it changes.
    ///
    /// Note that the `handler` is blocking, use [`async_app_hn!`] and [`task::wait`] to run IO without
    /// blocking the app.
    pub fn on_dir_changed(&self, dir: impl Into<PathBuf>, recursive: bool, handler: impl AppHandler<FsChangesArgs>) -> EventHandle {
        let dir = dir.into();
        let handle = self.watch_dir(dir.clone(), recursive);
        FS_CHANGES_EVENT.on_event(FilterAppHandler::new(handler, move |args| {
            let _handle = handle;
            args.events_for_path(&dir).is_some()
        }))
    }
}

event_args! {
     /// [`FS_CHANGES_EVENT`] arguments.
    pub struct FsChangesArgs {
        /// Timestamp of the first result in `changes`. This is roughly the `timestamp` minus the [`WATCHER.debounce`]
        /// interval.
        ///
        /// [`WATCHER.debounce`]: WATCHER::debounce
        pub first_change_ts: Instant,

        /// All notify changes since the last event.
        pub changes: Arc<Vec<notify::Result<notify::Event>>>,

        ..

        /// None, only app level handlers receive this event.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            let _ = list;
        }
    }
}
impl FsChangesArgs {
    /// Iterate over all change events.
    pub fn events(&self) -> impl Iterator<Item = &notify::Event> + '_ {
        self.changes.iter().filter_map(|r| r.as_ref().ok())
    }

    /// Iterate over all file watcher errors.
    pub fn errors(&self) -> impl Iterator<Item = &notify::Error> + '_ {
        self.changes.iter().filter_map(|r| r.as_ref().err())
    }

    /// Iterate over all change events that affects paths selected by the `glob` pattern.
    pub fn events_for(&self, glob: &str) -> Result<impl Iterator<Item = &notify::Event> + '_, glob::PatternError> {
        let glob = glob::Pattern::new(glob)?;
        Ok(self.events().filter(move |ev| ev.paths.iter().any(|p| glob.matches_path(p))))
    }

    /// Iterate over all change events that affects paths that are equal to `path` or inside it.
    pub fn events_for_path<'a>(&'a self, path: &'a Path) -> impl Iterator<Item = &notify::Event> + 'a {
        self.events().filter(move |ev| ev.paths.iter().any(|p| p.starts_with(path)))
    }
}

event! {
    /// Event sent by the [`WATCHER`] service on directories or files that are watched.
    pub static FS_CHANGES_EVENT: FsChangesArgs;
}

/// Represents an active file or directory watcher in [`WATCHER`].
#[derive(Clone)]
#[must_use = "the watcher is dropped if the handle is dropped"]
pub struct WatcherHandle(crate::crate_util::Handle<()>);

impl WatcherHandle {
    fn new() -> (HandleOwner<()>, Self) {
        let (owner, handle) = crate::crate_util::Handle::new(());
        (owner, Self(handle))
    }

    /// Handle to no watcher.
    pub fn dummy() -> Self {
        Self(crate::crate_util::Handle::dummy(()))
    }

    /// If [`perm`](Self::perm) was called in another clone of this handle.
    ///
    /// If `true` the resource will stay in memory for the duration of the app, unless [`force_drop`](Self::force_drop)
    /// is also called.
    pub fn is_permanent(&self) -> bool {
        self.0.is_permanent()
    }

    /// Force drops the watcher, meaning it will be dropped even if there are other handles active.
    pub fn force_drop(self) {
        self.0.force_drop()
    }

    /// If the watcher is dropped.
    pub fn is_dropped(&self) -> bool {
        self.0.is_dropped()
    }

    /// Drop the handle without dropping the watcher, the watcher will stay active for the
    /// duration of the app process.
    pub fn perm(self) {
        self.0.perm()
    }
}

app_local! {
    static WATCHER_SV: WatcherService = WatcherService::new();
}

struct WatcherService {
    debounce: ArcVar<Duration>,
    poll_interval: ArcVar<Duration>,

    watcher: Mutex<Box<dyn notify::Watcher + Send>>, // mutex for Sync only

    debounce_oldest: Instant,
    debounce_buffer: Vec<notify::Result<notify::Event>>,
    debounce_timer: Option<DeadlineHandle>,

    read_to_var: Vec<ReadToVar>,
}
impl WatcherService {
    fn new() -> Self {
        Self {
            debounce: var(100.ms()),
            poll_interval: var(10.secs()),
            watcher: Mutex::new(Box::new(notify::NullWatcher)),
            debounce_oldest: Instant::now(),
            debounce_buffer: vec![],
            debounce_timer: None,
            read_to_var: vec![],
        }
    }

    fn init_watcher(&mut self) {
        *self.watcher.get_mut() = match notify::recommended_watcher(notify_watcher_handle) {
            Ok(w) => Box::new(w),
            Err(e) => {
                tracing::error!("error creating watcher\n{e}\nfallback to slow poll watcher");
                match notify::PollWatcher::new(
                    notify_watcher_handle,
                    notify::Config::default().with_poll_interval(self.poll_interval.get()),
                ) {
                    Ok(w) => Box::new(w),
                    Err(e) => {
                        tracing::error!("error creating poll watcher\n{e}\nfs watching disabled");
                        Box::new(notify::NullWatcher)
                    }
                }
            }
        };
    }

    fn event(&mut self, args: &FsChangesArgs) {
        self.read_to_var.retain_mut(|f| f.on_event(args));
    }

    fn update(&mut self) {
        if let Some(n) = self.poll_interval.get_new() {
            self.watcher.get_mut().configure(notify::Config::default().with_poll_interval(n));
        }
        if !self.debounce_buffer.is_empty() {
            if let Some(n) = self.debounce.get_new() {
                if self.debounce_oldest.elapsed() >= n {
                    self.notify();
                }
            }
        }
        self.read_to_var.retain_mut(|f| f.retain());
    }

    fn watch(&mut self, file: PathBuf) -> WatcherHandle {
        let (owner, handle) = WatcherHandle::new();
        // !!: TODO
        handle
    }

    fn watch_dir(&mut self, dir: PathBuf, recursive: bool) -> WatcherHandle {
        let (owner, handle) = WatcherHandle::new();
        // !!: TODO
        handle
    }

    fn on_watcher(&mut self, r: notify::Result<notify::Event>) {
        let notify = !self.debounce_buffer.is_empty() && self.debounce_oldest.elapsed() >= self.debounce.get();

        self.debounce_buffer.push(r);

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

    fn on_debounce_timer(&mut self) {
        if !self.debounce_buffer.is_empty() {
            self.notify();
        }
    }

    fn notify(&mut self) {
        let changes = mem::take(&mut self.debounce_buffer);
        let now = Instant::now();
        let first_change_ts = mem::replace(&mut self.debounce_oldest, now);
        self.debounce_timer = None;

        FS_CHANGES_EVENT.notify(FsChangesArgs::new(now, Default::default(), first_change_ts, changes));
    }
}
fn notify_watcher_handle(r: notify::Result<notify::Event>) {
    WATCHER_SV.write().on_watcher(r)
}

struct ReadToVar {
    read: Box<dyn Fn(&Arc<AtomicBool>, &WatcherHandle, ReadEvent)>,
    pending: Arc<AtomicBool>,
    handle: WatcherHandle,
}
impl ReadToVar {
    fn new<O: VarValue, R>(
        handle: WatcherHandle,
        path: PathBuf,
        init: O,
        load: impl Fn(&Path) -> io::Result<R>,
        read: impl FnMut(io::Result<R>) -> Option<O> + Send + 'static,
    ) -> (Self, ReadOnlyArcVar<O>) {
        let path = Arc::new(path);
        let var = var(init);

        let pending = Arc::new(AtomicBool::new(false));
        let read = Arc::new(Mutex::new(read));
        let wk_var = var.downgrade();

        // read task "drains" pending, drops handle if the var is dropped.
        let read = Box::new(move |pending: &Arc<AtomicBool>, handle: &WatcherHandle, ev: ReadEvent| {
            if wk_var.strong_count() == 0 {
                handle.clone().force_drop();
                return;
            };

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
            task::spawn_wait(clmv!(read, wk_var, path, handle, pending, || {
                let mut read = read.lock();
                while pending.swap(false, Ordering::Relaxed) {
                    if let Some(update) = read(fs::File::open(path.as_path())) {
                        if let Some(var) = wk_var.upgrade() {
                            var.set(update);
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
