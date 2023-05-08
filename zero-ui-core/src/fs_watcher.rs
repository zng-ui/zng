//! File system events and service.

use std::{
    fs::{self, File},
    io, mem,
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc},
    time::{Duration, Instant},
};

use atomic::Ordering;
use hashbrown::HashMap;
use notify::Watcher as _;
use parking_lot::Mutex;

use crate::{
    app::AppExtension,
    context::app_local,
    crate_util::{Handle, HandleOwner},
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
        let handle = self.watch(path.clone());
        fn open(p: &Path) -> io::Result<File> {
            std::fs::File::open(p)
        }
        let (read, var) = ReadToVar::new(handle, path, init, open, read);
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
        let handle = self.watch_dir(path.clone(), recursive);
        fn open(p: &Path) -> io::Result<fs::ReadDir> {
            std::fs::read_dir(p)
        }
        let (read, var) = ReadToVar::new(handle, path, init, open, read);
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
            let _handle = &handle;
            args.events_for_path(&file).next().is_some()
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
            let _handle = &handle;
            args.events_for_path(&dir).next().is_some()
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
pub struct WatcherHandle(Handle<()>);

impl WatcherHandle {
    /// Handle to no watcher.
    pub fn dummy() -> Self {
        Self(Handle::dummy(()))
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

    watcher: Watchers,

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
            watcher: Watchers::new(),
            debounce_oldest: Instant::now(),
            debounce_buffer: vec![],
            debounce_timer: None,
            read_to_var: vec![],
        }
    }

    fn init_watcher(&mut self) {
        self.watcher.init();
    }

    fn event(&mut self, args: &FsChangesArgs) {
        self.read_to_var.retain_mut(|f| f.on_event(args));
    }

    fn update(&mut self) {
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
    }

    fn watch(&mut self, file: PathBuf) -> WatcherHandle {
        self.watcher.watch(file)
    }

    fn watch_dir(&mut self, dir: PathBuf, recursive: bool) -> WatcherHandle {
        self.watcher.watch_dir(dir, recursive)
    }

    fn on_watcher(&mut self, r: notify::Result<notify::Event>) {
        if let Ok(r) = &r {
            if !self.watcher.allow(r) {
                // file parent watcher, file not affected.
                return;
            }
        }

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
    read: Box<dyn Fn(&Arc<AtomicBool>, &WatcherHandle, ReadEvent) + Send + Sync>,
    pending: Arc<AtomicBool>,
    handle: WatcherHandle,
}
impl ReadToVar {
    fn new<O: VarValue, R: 'static>(
        handle: WatcherHandle,
        path: PathBuf,
        init: O,
        load: fn(&Path) -> io::Result<R>,
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
                    if let Some(update) = read(load(path.as_path())) {
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
            poll_interval: 10.secs(),
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
                tracing::error!("cannot file watch `{}`", path.display());
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
        *self.watcher.get_mut() = match notify::recommended_watcher(notify_watcher_handle) {
            Ok(w) => Box::new(w),
            Err(e) => {
                tracing::error!("error creating watcher\n{e}\nfallback to slow poll watcher");
                match PollWatcher::new(
                    notify_watcher_handle,
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

    /// Returns Ok, or Err `PathNotFound` or `MaxFilesWatch` that can be handled using the fallback watcher.
    fn inner_watch_dir(watcher: &mut dyn notify::Watcher, dir: &Path, recursive: bool) -> Result<(), notify::ErrorKind> {
        let recursive = if recursive {
            notify::RecursiveMode::Recursive
        } else {
            notify::RecursiveMode::NonRecursive
        };
        if let Err(e) = watcher.watch(dir, recursive) {
            match e.kind {
                notify::ErrorKind::Generic(e) => tracing::error!("cannot watch `{}`, {e}", dir.display()),
                notify::ErrorKind::Io(e) => {
                    if let io::ErrorKind::NotFound = e.kind() {
                        return Err(notify::ErrorKind::PathNotFound);
                    } else {
                        tracing::error!("cannot watch `{}`, {e}", dir.display())
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
            PollWatcher::new(notify_watcher_handle, notify::Config::default().with_poll_interval(poll_interval)).unwrap()
        });
        Self::inner_watch_dir(watcher, dir, recursive).unwrap();
    }

    fn inner_unwatch_dir(watcher: &mut dyn notify::Watcher, dir: &Path) {
        if let Err(e) = watcher.unwatch(dir) {
            match e.kind {
                notify::ErrorKind::Generic(e) => {
                    tracing::error!("cannot unwatch `{}`, {e}", dir.display());
                }
                notify::ErrorKind::Io(e) => {
                    tracing::error!("cannot unwatch `{}`, {e}", dir.display());
                }
                notify::ErrorKind::PathNotFound => {}  // ok?
                notify::ErrorKind::WatchNotFound => {} // ok
                notify::ErrorKind::InvalidConfig(_) => unreachable!(),
                notify::ErrorKind::MaxFilesWatch => unreachable!(),
            }
        }
    }

    fn allow(&mut self, r: &notify::Event) -> bool {
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
        let mut dirs = HashMap::<PathBuf, PollInfo, _, _>::new();
        let worker = std::thread::Builder::new()
            .name(String::from("poll-watcher"))
            .spawn(move || loop {
                match rcv.recv_timeout(config.poll_interval()) {
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
        recursive_collect(&mut paths, path, recursive);
        Self {
            recursive,
            paths,
            update_flag: false,
        }
    }

    fn poll(&mut self, root: &Path, handler: &mut impl notify::EventHandler) {
        self.update_flag = !self.update_flag;
        recursive_update(&mut self.paths, root, self.recursive, self.update_flag, handler);

        self.paths.retain(|k, e| {
            let retain = e.update_flag == self.update_flag;
            if !retain {
                handler.handle_event(Ok(notify::Event {
                    kind: notify::EventKind::Remove(notify::event::RemoveKind::Any),
                    paths: vec![k.clone()],
                    attrs: Default::default(),
                }));
            }
            retain
        });
    }
}
fn recursive_collect(paths: &mut HashMap<PathBuf, PollEntry>, path: &Path, recursive: bool) {
    if let Ok(dirs) = std::fs::read_dir(path) {
        for entry in dirs.flatten() {
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    let path = entry.path();
                    if recursive && meta.is_dir() {
                        recursive_collect(paths, &path, recursive);
                    }
                    paths.insert(
                        path,
                        PollEntry {
                            modified,
                            update_flag: false,
                        },
                    );
                }
            }
        }
    }
}

fn recursive_update(
    paths: &mut HashMap<PathBuf, PollEntry>,
    path: &Path,
    recursive: bool,
    update_flag: bool,
    handler: &mut impl notify::EventHandler,
) {
    if let Ok(dirs) = std::fs::read_dir(path) {
        for entry in dirs.flatten() {
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    let path = entry.path();
                    if recursive && meta.is_dir() {
                        recursive_update(paths, &path, recursive, update_flag, handler);
                    }
                    match paths.entry(path) {
                        hashbrown::hash_map::Entry::Occupied(mut e) => {
                            let info = e.get_mut();
                            info.update_flag = update_flag;
                            if info.modified != modified {
                                info.modified = modified;

                                handler.handle_event(Ok(notify::Event {
                                    kind: notify::EventKind::Modify(notify::event::ModifyKind::Metadata(
                                        notify::event::MetadataKind::WriteTime,
                                    )),
                                    paths: vec![e.key().clone()],
                                    attrs: Default::default(),
                                }))
                            }
                        }
                        hashbrown::hash_map::Entry::Vacant(e) => {
                            handler.handle_event(Ok(notify::Event {
                                kind: notify::EventKind::Create(if meta.is_dir() {
                                    notify::event::CreateKind::Folder
                                } else {
                                    notify::event::CreateKind::File
                                }),
                                paths: vec![e.key().clone()],
                                attrs: Default::default(),
                            }));

                            e.insert(PollEntry { modified, update_flag });
                        }
                    }
                }
            }
        }
    }
}
