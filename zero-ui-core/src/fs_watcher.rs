//! File system events and service.

use std::{
    fs::{self, File},
    io,
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use atomic::Ordering;
use parking_lot::Mutex;

use crate::{
    app::AppExtension,
    context::app_local,
    crate_util::HandleOwner,
    event::{event, event_args, EventHandle},
    handler::{async_app_hn, AppHandler},
    task,
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
    fn update_preview(&mut self) {
        let sv = WATCHER_SV.read();
        if let Some(new) = sv.debounce.get_new() {
            todo!()
        }
        if let Some(new) = sv.poll_interval.get_new() {
            todo!()
        }
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
        todo!()
    }

    /// When an efficient watcher cannot be used a poll watcher fallback is used, the poll watcher reads
    /// the directory or path every elapse of this interval.
    ///
    /// Is `10.secs()` by default.
    pub fn poll_interval(&self) -> ArcVar<Duration> {
        todo!()
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
        todo!("!!: watch the parent dir and filter, to avoid issues when the file is temporarily removed and replaced with another")
    }

    /// Enable file change events for files inside `dir`, also include inner directories if `recursive` is `true`.
    ///
    /// Returns a handle that will stop the dir watch when dropped, if there is no other active handler for the same directory.
    ///
    /// The directory will be watched using an OS specific efficient watcher provided by the [`notify`] crate. If there is
    /// any error creating the watcher, such as if the directory does not exist yet a slower polling watcher will retry periodically    
    /// until the efficient watcher can be created or the handle is dropped.
    pub fn watch_dir(&self, dir: impl Into<PathBuf>, recursive: bool) -> WatcherHandle {
        todo!("!!: HANDLE")
    }

    /// Read a file into a variable, the `init` value will start the variable and the `read` closure will be called
    /// every time the file changes, if the closure returns `Some(O)` the variable updates with the new value.
    ///
    /// Dropping the variable drops the read watch. The `read` closure is non-blocking, it is called in a [`task::wait`]
    /// background thread.
    pub fn read<O: VarValue>(
        &self,
        file: impl Into<PathBuf>,
        init: O,
        read: impl FnMut(io::Result<File>) -> Option<O> + Send + 'static,
    ) -> ReadOnlyArcVar<O> {
        // !!: TODO
        ReadFile::new(file.into(), init, read).1
    }

    /// Read a directory into a variable,  the `init` value will start the variable and the `read` closure will be called
    /// every time any changes happen inside the dir, if the closure returns `Some(O)` the variable updates with the new value.
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
        todo!()
    }

    /// Watch `file` and calls `handler` every time it changes.
    ///
    /// Note that the `handler` is blocking, use [`async_app_hn!`] and [`task::wait`] to run IO without
    /// blocking the app.
    pub fn on_file_changed(&self, file: impl Into<PathBuf>, handler: impl AppHandler<PathChangedArgs>) -> EventHandle {
        todo!()
    }

    /// Watch `dir` and calls `handler` every time something inside it changes.
    ///
    /// Note that the `handler` is blocking, use [`async_app_hn!`] and [`task::wait`] to run IO without
    /// blocking the app.
    pub fn on_dir_changed(&self, dir: impl Into<PathBuf>, recursive: bool, handler: impl AppHandler<PathChangedArgs>) -> EventHandle {
        todo!()
    }
}

event_args! {
     /// [`FS_CHANGES_EVENT`] arguments.
    pub struct PathChangedArgs {
        ..

        /// None, only app level handlers receive this event.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            let _ = list;
        }
    }
}

event! {
    /// Event sent by the [`WATCHER`] service on directories or files that are watched.
    pub static FS_CHANGES_EVENT: PathChangedArgs;
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
    static WATCHER_SV: WatcherService = WatcherService {
        debounce: var(100.ms()),
        poll_interval: var(10.secs()),
    };
}

struct WatcherService {
    debounce: ArcVar<Duration>,
    poll_interval: ArcVar<Duration>,
}

struct ReadFile {
    read: Box<dyn Fn(Arc<AtomicBool>, WatcherHandle)>,
    pending: Arc<AtomicBool>,
    handle: WatcherHandle,
}
impl ReadFile {
    fn new<O: VarValue>(path: PathBuf, init: O, read: impl FnMut(fs::File) -> Option<O> + Send + 'static) -> (Self, ReadOnlyArcVar<O>) {
        let handle = WATCHER.watch(path.clone());
        let path = Arc::new(path);
        let var = var(init);

        let pending = Arc::new(AtomicBool::new(false));
        let read = Arc::new(Mutex::new(read));
        let wk_var = var.downgrade();
        let read = Box::new(move |pending, handle| {
            if wk_var.strong_count() == 0 {
                handle.force_drop();
                return;
            }
            if pending.load(Ordering::Relaxed) {
                task::spawn_wait(clmv!(read, wk_var, path, || {
                    let mut read = read.lock();
                    while pending.swap(Ordering::Relaxed, false) {
                        if let Some(update) = read(fs::File::open(path.as_path())) {
                            if let Some(var) = wk_var.upgrade() {
                                var.set(update);
                            } else {
                                handle.unsubscribe();
                            }
                        }
                    }
                }));
            }
        });

        (Self { read, pending, handle }, var.read_only())
    }
}
