#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! File system events and service.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
// suppress nag about very simple boxed closure signatures.
#![allow(clippy::type_complexity)]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::{
    collections::{hash_map, HashMap},
    fmt, fs,
    io::{self, Write as _},
    mem, ops,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64},
        Arc,
    },
    thread,
    time::{Duration, SystemTime},
};

use atomic::{Atomic, Ordering};
use fs4::FileExt;
use notify::Watcher;
use parking_lot::Mutex;
use path_absolutize::Absolutize;
use zng_app::{
    app_hn_once,
    event::{event, event_args, EventHandle},
    handler::{AppHandler, FilterAppHandler},
    timer::{DeadlineHandle, TIMERS},
    update::EventUpdate,
    view_process::raw_events::LOW_MEMORY_EVENT,
    AppExtension, DInstant, INSTANT,
};
use zng_app_context::{app_local, LocalContext};
use zng_clone_move::clmv;
use zng_handle::{Handle, HandleOwner};
use zng_txt::Txt;
use zng_unit::TimeUnits;
use zng_var::{
    types::WeakArcVar, var, AnyVar, AnyVarHookArgs, AnyWeakVar, ArcVar, ReadOnlyArcVar, Var, VarUpdateId, VarValue, WeakVar, VARS,
};

use zng_task as task;

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

    fn event_preview(&mut self, update: &mut EventUpdate) {
        if let Some(args) = FS_CHANGES_EVENT.on(update) {
            WATCHER_SV.write().event(args);
        } else if LOW_MEMORY_EVENT.on(update).is_some() {
            WATCHER_SV.write().low_memory();
        }
    }

    fn update_preview(&mut self) {
        WATCHER_SV.write().update();
    }

    fn deinit(&mut self) {
        let mut flush = WATCHER_SV.write().shutdown();
        for v in &mut flush {
            v.flush_shutdown();
        }
    }
}

/// File system watcher service.
///
/// This is mostly a wrapper around the [`notify`](https://docs.rs/notify) crate, integrating it with events and variables.
pub struct WATCHER;
impl WATCHER {
    /// Gets a read-write variable that defines interval awaited between each [`FS_CHANGES_EVENT`]. If
    /// a watched path is constantly changing an event will be emitted every elapse of this interval,
    /// the event args will contain a list of all the changes observed during the interval.
    ///
    /// Note that the first event notifies immediately, only subsequent events within this interval are debounced.
    ///
    /// Is `100.ms()` by default.
    pub fn debounce(&self) -> ArcVar<Duration> {
        WATCHER_SV.read().debounce.clone()
    }

    /// Gets a read-write variable that defines interval awaited between each [`sync`] write.
    ///
    /// Is `100.ms()` by default.
    ///
    /// [`sync`]: WATCHER::sync
    pub fn sync_debounce(&self) -> ArcVar<Duration> {
        WATCHER_SV.read().debounce.clone()
    }

    /// Gets a read-write variable that defines the fallback poll watcher interval.
    ///
    /// When an efficient watcher cannot be used a poll watcher fallback is used, the poll watcher reads
    /// the directory or path every elapse of this interval. The poll watcher is also used for paths that
    /// do not exist yet, that is also affected by this interval.
    ///
    /// Is `1.secs()` by default.
    pub fn poll_interval(&self) -> ArcVar<Duration> {
        WATCHER_SV.read().poll_interval.clone()
    }

    /// Maximum time the service keeps the process alive to process pending IO operations when the app shuts down.
    ///
    /// Is 1 minute by default.
    pub fn shutdown_timeout(&self) -> ArcVar<Duration> {
        WATCHER_SV.read().shutdown_timeout.clone()
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
    /// The directory will be watched using an OS specific efficient watcher provided by the [`notify`](https://docs.rs/notify) crate. If there is
    /// any error creating the watcher, such as if the directory does not exist yet a slower polling watcher will retry periodically    
    /// until the efficient watcher can be created or the handle is dropped.
    pub fn watch_dir(&self, dir: impl Into<PathBuf>, recursive: bool) -> WatcherHandle {
        WATCHER_SV.write().watch_dir(dir.into(), recursive)
    }

    /// Read a file into a variable, the `init` value will start the variable and the `read` closure will be called
    /// once immediately and every time the file changes, if the closure returns `Some(O)` the variable updates with the new value.
    ///
    /// Dropping the variable drops the read watch. The `read` closure is non-blocking, it is called in a [`task::wait`]
    /// background thread.
    ///
    /// [`task::wait`]: zng_task::wait
    pub fn read<O: VarValue>(
        &self,
        file: impl Into<PathBuf>,
        init: O,
        read: impl FnMut(io::Result<WatchFile>) -> Option<O> + Send + 'static,
    ) -> ReadOnlyArcVar<O> {
        WATCHER_SV.write().read(file.into(), init, read)
    }

    /// Same operation as [`read`] but also tracks the operation status in a second var.
    ///
    /// The status variable is set to [`WatcherReadStatus::reading`] as soon as `read` starts and
    /// is set to [`WatcherReadStatus::idle`] when read returns. If read returns a value the status
    /// only updates to idle  when the new value is available on the var, or because read the same value.
    ///
    /// [`read`]: Self::read
    pub fn read_status<O, S, E>(
        &self,
        file: impl Into<PathBuf>,
        init: O,
        read: impl FnMut(io::Result<WatchFile>) -> Result<Option<O>, E> + Send + 'static,
    ) -> (ReadOnlyArcVar<O>, ReadOnlyArcVar<S>)
    where
        O: VarValue,
        S: WatcherReadStatus<E>,
    {
        WATCHER_SV.write().read_status(file.into(), init, read)
    }

    /// Read a directory into a variable, the `init` value will start the variable and the `read` closure will be called
    /// once immediately and every time any changes happen inside the dir, if the closure returns `Some(O)` the variable updates with the new value.
    ///
    /// The `read` closure parameter is a directory walker from the [`walkdir`](https://docs.rs/walkdir) crate.
    ///
    /// The directory walker is pre-configured to skip the `dir` itself and to have a max-depth of 1 if not `recursive`, these configs can.
    ///
    /// Dropping the variable drops the read watch. The `read` closure is non-blocking, it is called in a [`task::wait`]
    /// background thread.
    ///
    /// [`task::wait`]: zng_task::wait
    pub fn read_dir<O: VarValue>(
        &self,
        dir: impl Into<PathBuf>,
        recursive: bool,
        init: O,
        read: impl FnMut(walkdir::WalkDir) -> Option<O> + Send + 'static,
    ) -> ReadOnlyArcVar<O> {
        WATCHER_SV.write().read_dir(dir.into(), recursive, init, read)
    }

    /// Same operation as [`read_dir`] but also tracks the operation status in a second var.
    ///
    /// The status variable is set to [`WatcherReadStatus::reading`] as soon as `read` starts and
    /// is set to [`WatcherReadStatus::idle`] when read returns. If read returns a value the status
    /// only updates to idle when the new value is available on the var, or because read the same value.
    ///
    /// [`read_dir`]: Self::read_dir
    pub fn read_dir_status<O, S, E>(
        &self,
        dir: impl Into<PathBuf>,
        recursive: bool,
        init: O,
        read: impl FnMut(walkdir::WalkDir) -> Result<Option<O>, E> + Send + 'static,
    ) -> (ReadOnlyArcVar<O>, ReadOnlyArcVar<S>)
    where
        O: VarValue,
        S: WatcherReadStatus<E>,
    {
        WATCHER_SV.write().read_dir_status(dir.into(), recursive, init, read)
    }

    /// Bind a file with a variable, the `file` will be `read` when it changes and be `write` when the variable changes,
    /// writes are only applied on success and will not cause a `read` on the same sync task. The `init` value is used to
    /// create the variable, if the `file` exists it will be `read` once at the beginning.
    ///
    /// Dropping the variable drops the read watch. The `read` and `write` closures are non-blocking, they are called in a [`task::wait`]
    /// background thread.
    ///
    /// # Sync
    ///
    /// The file synchronization ensures that the file is only actually modified when writing is finished by writing
    /// to a temporary file and committing a replace only if the write succeeded. The file is write-locked for the duration
    /// of `write` call, but the contents are not touched until commit. See [`WriteFile`] for more details.
    ///
    /// The [`FsWatcherManager`] blocks on app exit until all writes commit or cancel.
    ///
    /// ## Read Errors
    ///
    /// Not-found errors are handled by the watcher by calling `write` using the current variable value, other read errors
    /// are passed to `read`. If `read` returns a value for an error the `write` closure is called to override the file,
    /// otherwise only the variable is set and this variable update does not cause a `write`.
    ///
    /// ## Write Errors
    ///
    /// If `write` fails the file is not touched and the temporary file is removed, if the file path
    /// does not exit all missing parent folders and the file will be created automatically before the `write`
    /// call.
    ///
    /// Note that [`WriteFile::commit`] must be called to flush the temporary file and attempt to rename
    /// it, if the file is dropped without commit it will cancel and log an error, you must call [`WriteFile::cancel`]
    /// to correctly avoid writing.
    ///
    /// If the cleanup after commit fails the error is logged and ignored.
    ///
    /// If write fails to even create the file and/or acquire a write lock on it this error is the input for
    /// the `write` closure.
    ///
    /// ## Error Handling
    ///
    /// You can call services or set other variables from inside the `read` and `write` closures, this can be
    /// used to get a signal out that perhaps drops the sync var (to stop watching), alert the user that the
    /// file is out of sync and initiate some sort of recovery routine.
    ///
    /// If the file synchronization is not important you can just ignore it, the watcher will try again
    /// on the next variable or file update.
    ///
    /// ## Status
    ///
    /// Note that `read` and `write` run in background task threads, so if you are tracking the operation
    /// status in a separate variable you may end-up with synchronization bugs between th status variable
    /// and the actual result variable, you can use [`sync_status`] to implement racing-free status tracking.
    ///
    /// [`sync_status`]: Self::sync_status
    /// [`task::wait`]: zng_task::wait
    pub fn sync<O: VarValue>(
        &self,
        file: impl Into<PathBuf>,
        init: O,
        read: impl FnMut(io::Result<WatchFile>) -> Option<O> + Send + 'static,
        write: impl FnMut(O, io::Result<WriteFile>) + Send + 'static,
    ) -> ArcVar<O> {
        WATCHER_SV.write().sync(file.into(), init, read, write)
    }

    /// Same operation as [`sync`] but also tracks the operation status in a second var.
    ///
    /// The status variable is set to [`WatcherReadStatus::reading`] as soon as `read` starts and
    /// is set to [`WatcherReadStatus::idle`] when read returns. If read returns a value the status
    /// only updates to idle when the new sync value is available, or because read the same value.
    ///
    /// The status variable is set to [`WatcherSyncStatus::writing`] as soon as it updates and
    /// is set to [`WatcherReadStatus::idle`] only when the new sync value is available, either
    /// by update or because read the same value.
    ///
    /// [`sync`]: Self::sync
    pub fn sync_status<O, S, ER, EW>(
        &self,
        file: impl Into<PathBuf>,
        init: O,
        read: impl FnMut(io::Result<WatchFile>) -> Result<Option<O>, ER> + Send + 'static,
        write: impl FnMut(O, io::Result<WriteFile>) -> Result<(), EW> + Send + 'static,
    ) -> (ArcVar<O>, ReadOnlyArcVar<S>)
    where
        O: VarValue,
        S: WatcherSyncStatus<ER, EW>,
    {
        WATCHER_SV.write().sync_status(file.into(), init, read, write)
    }

    /// Watch `file` and calls `handler` every time it changes.
    ///
    /// Note that the `handler` is blocking, use [`async_app_hn!`] and [`task::wait`] to run IO without
    /// blocking the app.
    ///
    /// [`async_app_hn!`]: macro@zng_app::handler::async_app_hn
    /// [`task::wait`]: zng_task::wait
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
    ///
    /// [`async_app_hn!`]: macro@zng_app::handler::async_app_hn
    /// [`task::wait`]: zng_task::wait
    pub fn on_dir_changed(&self, dir: impl Into<PathBuf>, recursive: bool, handler: impl AppHandler<FsChangesArgs>) -> EventHandle {
        let dir = dir.into();
        let handle = self.watch_dir(dir.clone(), recursive);
        FS_CHANGES_EVENT.on_event(FilterAppHandler::new(handler, move |args| {
            let _handle = &handle;
            args.events_for_path(&dir).next().is_some()
        }))
    }

    /// Push a `note` that will be cloned on all subsequent change events until it the returned handle is dropped.
    ///
    /// This can be used to tag all events that happened over a period of time, something you can't do just
    /// by receiving the events due to async delays caused by debounce.
    ///
    /// Note that the underlying system events the [`notify`](https://docs.rs/notify) crate uses are not guaranteed to be synchronous.
    pub fn annotate(&self, note: Arc<dyn FsChangeNote>) -> FsChangeNoteHandle {
        WATCHER_SV.write().annotate(note)
    }
}

/// Represents a status type for [`WATCHER.sync_status`].
///
/// [`WATCHER.sync_status`]: WATCHER::sync_status
pub trait WatcherSyncStatus<ER = io::Error, EW = io::Error>: WatcherReadStatus<ER> {
    /// New writing value.
    fn writing() -> Self;
    /// New write error value.
    fn write_error(e: EW) -> Self;
}

/// Represents a status type for [`WATCHER`] read-only operations.
pub trait WatcherReadStatus<ER = io::Error>: VarValue + PartialEq {
    /// New idle value.
    fn idle() -> Self;
    /// New reading value.
    fn reading() -> Self;
    /// New read error value.
    fn read_error(e: ER) -> Self;
}

/// Represents an open read-only file provided by [`WATCHER.read`].
///
/// This type is a thin wrapper around the [`std::fs::File`] with some convenience parsing methods.
///
/// [`WATCHER.read`]: WATCHER::read
#[derive(Debug)]
pub struct WatchFile(fs::File);
impl WatchFile {
    /// Open read the file.
    pub fn open(file: impl AsRef<Path>) -> io::Result<Self> {
        Self::try_open_non_empty(file.as_ref(), true)
    }
    fn try_open_non_empty(path: &Path, retry: bool) -> io::Result<Self> {
        let file = fs::File::open(path)?;

        if retry && file.metadata()?.len() == 0 {
            // some apps create an empty file unlocked, then write.
            let _ = file;
            std::thread::sleep(5.ms());
            return Self::try_open_non_empty(path, false);
        }

        lock_shared(&file, Duration::from_secs(10))?;
        Ok(Self(file))
    }

    /// Read the file contents as a text string.
    pub fn text(&mut self) -> io::Result<Txt> {
        self.string().map(Txt::from)
    }

    /// Read the file contents as a string.
    pub fn string(&mut self) -> io::Result<String> {
        use std::io::Read;
        let mut s = String::new();
        self.0.read_to_string(&mut s)?;
        Ok(s)
    }

    /// Deserialize the file contents as JSON.
    #[cfg(feature = "json")]
    pub fn json<O>(&mut self) -> serde_json::Result<O>
    where
        O: serde::de::DeserializeOwned,
    {
        serde_json::from_reader(io::BufReader::new(&mut self.0))
    }

    /// Deserialize the file contents as TOML.
    #[cfg(feature = "toml")]
    pub fn toml<O>(&mut self) -> io::Result<O>
    where
        O: serde::de::DeserializeOwned,
    {
        use std::io::Read;
        let mut buf = io::BufReader::new(&mut self.0);

        let mut toml_str = String::new();
        buf.read_to_string(&mut toml_str)?;

        toml::de::from_str(&toml_str).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Deserialize the file content as RON.
    #[cfg(feature = "ron")]
    pub fn ron<O>(&mut self) -> Result<O, ron::de::SpannedError>
    where
        O: serde::de::DeserializeOwned,
    {
        ron::de::from_reader(io::BufReader::new(&mut self.0))
    }

    /// Deserialize the file content as YAML.
    #[cfg(feature = "yaml")]
    pub fn yaml<O>(&mut self) -> serde_yaml::Result<O>
    where
        O: serde::de::DeserializeOwned,
    {
        serde_yaml::from_reader(io::BufReader::new(&mut self.0))
    }

    /// Read file and parse it.
    pub fn parse<O: std::str::FromStr>(&mut self) -> Result<O, WatchFileParseError<O::Err>> {
        use std::io::Read;
        let mut s = String::new();
        self.0.read_to_string(&mut s)?;
        O::from_str(&s).map_err(WatchFileParseError::Parse)
    }
}
impl ops::Deref for WatchFile {
    type Target = fs::File;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl ops::DerefMut for WatchFile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Drop for WatchFile {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

const TRANSACTION_GUID: &str = "6eIw3bYMS0uKaQMkTIQacQ";
const TRANSACTION_LOCK_EXT: &str = "6eIw3bYMS0uKaQMkTIQacQ-lock.tmp";

/// Represents an open write file provided by [`WATCHER.sync`].
///
/// This type actually writes to a temporary file and rename it over the actual file on commit only.
/// The dereferenced [`fs::File`] is the temporary file, not the actual one.
///
/// # Transaction
///
/// To minimize the risk of file corruption exclusive locks are used, both the target file and the temp file
/// are locked. An empty lock file is also used to cover the moment when both files are unlocked for the rename operation
/// and the moment the temp file is acquired.
///
/// The temp file is the actual file path with file extension replaced with `{path/.file-name.ext}.{GUID}-{n}.tmp`, the `n` is a
/// number from 0 to 999, if a temp file exists unlocked it will be reused.
///
/// The lock file is `{path/.file-name.ext}.{GUID}-lock.tmp`. Note that this
/// lock file only helps for apps that use [`WriteFile`], but even without it the risk is minimal as the slow
/// write operations are already flushed when it is time to commit.
///
/// [`WATCHER.sync`]: WATCHER::sync
pub struct WriteFile {
    temp_file: Option<fs::File>,
    actual_file: Option<fs::File>,
    transaction_lock: Option<fs::File>,

    actual_path: PathBuf,
    temp_path: PathBuf,
    transaction_path: PathBuf,

    cleaned: bool,
}
impl Drop for WriteFile {
    fn drop(&mut self) {
        if !self.cleaned {
            tracing::error!("dropped sync write file without commit or cancel");
            self.clean();
        }
    }
}
impl ops::Deref for WriteFile {
    type Target = fs::File;

    fn deref(&self) -> &Self::Target {
        self.temp_file.as_ref().unwrap()
    }
}
impl ops::DerefMut for WriteFile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.temp_file.as_mut().unwrap()
    }
}
impl WriteFile {
    /// Open or create the file.
    pub fn open(path: PathBuf) -> io::Result<Self> {
        let actual_path = path.absolutize()?.into_owned();
        if !actual_path.exists() {
            if let Some(parent) = actual_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let hidden_name = match actual_path.file_name() {
            Some(n) => format!(".{}", n.to_string_lossy()),
            None => return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected file name")),
        };

        let transaction_path = actual_path.with_file_name(format!("{hidden_name}.{TRANSACTION_LOCK_EXT}"));
        let transaction_lock = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&transaction_path)?;

        const TIMEOUT: Duration = Duration::from_secs(10);

        lock_exclusive(&transaction_lock, TIMEOUT)?;

        let actual_file = fs::OpenOptions::new().write(true).create(true).truncate(false).open(&actual_path)?;
        lock_exclusive(&actual_file, TIMEOUT)?;

        let mut n = 0;
        let mut temp_path = actual_path.with_file_name(format!("{hidden_name}.{TRANSACTION_GUID}-{n}.tmp"));
        let temp_file = loop {
            if let Ok(f) = fs::OpenOptions::new().write(true).create(true).truncate(true).open(&temp_path) {
                if f.try_lock_exclusive().is_ok() {
                    break f;
                }
            }

            n += 1;
            temp_path = actual_path.with_file_name(format!("{hidden_name}.{TRANSACTION_GUID}-{n}.tmp"));
            n += 1;
            if n > 1000 {
                return Err(io::Error::new(io::ErrorKind::AlreadyExists, "cannot create temporary file"));
            }
        };

        Ok(Self {
            actual_file: Some(actual_file),
            temp_file: Some(temp_file),
            transaction_lock: Some(transaction_lock),
            actual_path,
            temp_path,
            transaction_path,
            cleaned: false,
        })
    }

    /// Write the text string.
    pub fn write_text(&mut self, txt: &str) -> io::Result<()> {
        self.write_all(txt.as_bytes())
    }

    /// Serialize and write.
    ///
    /// If `pretty` is `true` the JSON is formatted for human reading.
    #[cfg(feature = "json")]
    pub fn write_json<O: serde::Serialize>(&mut self, value: &O, pretty: bool) -> io::Result<()> {
        let mut buf = io::BufWriter::new(ops::DerefMut::deref_mut(self));
        if pretty {
            serde_json::to_writer_pretty(&mut buf, value)?;
        } else {
            serde_json::to_writer(&mut buf, value)?;
        }
        buf.flush()
    }

    /// Serialize and write.
    ///
    /// If `pretty` is `true` the TOML is formatted for human reading.
    #[cfg(feature = "toml")]
    pub fn write_toml<O: serde::Serialize>(&mut self, value: &O, pretty: bool) -> io::Result<()> {
        let toml = if pretty {
            toml::ser::to_string_pretty(value)
        } else {
            toml::ser::to_string(value)
        }
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        self.write_all(toml.as_bytes())
    }

    /// Serialize and write.
    ///
    /// If `pretty` is `true` the RON if formatted for human reading using the default pretty config.
    #[cfg(feature = "ron")]
    pub fn write_ron<O: serde::Serialize>(&mut self, value: &O, pretty: bool) -> io::Result<()> {
        let mut buf = io::BufWriter::new(ops::DerefMut::deref_mut(self));
        if pretty {
            ron::ser::to_writer_pretty(&mut buf, value, Default::default()).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        } else {
            ron::ser::to_writer(&mut buf, value).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        }
        buf.flush()
    }

    /// Serialize and write.
    #[cfg(feature = "yaml")]
    pub fn write_yaml<O: serde::Serialize>(&mut self, value: &O) -> io::Result<()> {
        let mut buf = io::BufWriter::new(ops::DerefMut::deref_mut(self));
        serde_yaml::to_writer(&mut buf, value).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        buf.flush()
    }

    /// Commit write, flush and replace the actual file with the new one.
    pub fn commit(mut self) -> io::Result<()> {
        let r = self.replace_actual();
        self.clean();
        r
    }

    /// Cancel write, the file will not be updated.
    pub fn cancel(mut self) {
        self.clean();
    }

    fn replace_actual(&mut self) -> io::Result<()> {
        let mut temp_file = self.temp_file.take().unwrap();
        temp_file.flush()?;
        temp_file.sync_all()?;

        unlock_ok(&temp_file).unwrap();
        drop(temp_file);

        let actual_file = self.actual_file.take().unwrap();
        unlock_ok(&actual_file)?;
        drop(actual_file);

        let mut retries = 0;
        loop {
            // commit by replacing the actual_path with already on disk temp_path file.
            match fs::rename(&self.temp_path, &self.actual_path) {
                Ok(()) => {
                    break;
                }
                Err(e) => match e.kind() {
                    io::ErrorKind::PermissionDenied => {
                        if retries == 5 {
                            // Give-up, we manage to write lock both temp and actual just
                            // before this, but now we can't replace actual and remove temp.
                            // Hardware issue? Or another process holding a lock for 1s+50ms*5.
                            return Err(e);
                        } else if retries > 0 {
                            // Second+ retries:
                            //
                            // probably a system issue.
                            //
                            // Windows sporadically returns ACCESS_DENIED for kernel!SetRenameInformationFile in
                            // other apps that use the same save pattern (write-tmp -> close-tmp -> rename).
                            // see GIMP issue: https://gitlab.gnome.org/GNOME/gimp/-/issues/1370
                            //
                            // I used procmon to trace all file operations, there is no other app trying to use
                            // the temp and actual files when the ACCESS_DENIED occurs, both files are unlocked and
                            // closed before the rename calls start. This might be a Windows bug.
                            std::thread::sleep(30.ms());
                        } else {
                            // first retry:
                            //
                            // probably another process reading the `actual_path`.
                            //
                            // Reacquire a write lock and unlock, just to wait the external app.
                            match std::fs::File::options().write(true).open(&self.actual_path) {
                                Ok(f) => {
                                    if lock_exclusive(&f, 10.secs()).is_ok() {
                                        // acquired actual ok, retry
                                        let _ = unlock_ok(&f);
                                    }
                                }
                                Err(e) => match e.kind() {
                                    io::ErrorKind::NotFound => {
                                        // all good, rename will create actual
                                        continue;
                                    }
                                    _ => {
                                        // unknown error, let retry handle it
                                        std::thread::sleep(30.ms());
                                    }
                                },
                            }
                        }

                        retries += 1;
                    }
                    _ => return Err(e),
                },
            }
        }

        Ok(())
    }

    fn clean(&mut self) {
        self.cleaned = true;

        if let Some(tmp) = self.temp_file.take() {
            let _ = tmp.unlock();
        }
        if let Err(e) = fs::remove_file(&self.temp_path) {
            tracing::debug!("failed to cleanup temp file, {e}")
        }

        if let Some(file) = self.actual_file.take() {
            let _ = file.unlock();
        }

        let transaction = self.transaction_lock.take().unwrap();
        let _ = transaction.unlock();
        let _ = fs::remove_file(&self.transaction_path);
    }
}

/// Error for [`WatchFile::parse`].
#[derive(Debug)]
pub enum WatchFileParseError<E> {
    /// Error reading the file.
    Io(io::Error),
    /// Error parsing the file.
    Parse(E),
}
impl<E> From<io::Error> for WatchFileParseError<E> {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}
impl<E: fmt::Display> fmt::Display for WatchFileParseError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WatchFileParseError::Io(e) => write!(f, "read error, {e}"),
            WatchFileParseError::Parse(e) => write!(f, "parse error, {e}"),
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for WatchFileParseError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WatchFileParseError::Io(e) => Some(e),
            WatchFileParseError::Parse(e) => Some(e),
        }
    }
}

/// Represents a [`FsChange`] note.
///
/// This trait is already implemented for types it applies.
#[diagnostic::on_unimplemented(note = "`FsChangeNote` is implemented for all `T: Debug + Any + Send + Sync`")]
pub trait FsChangeNote: fmt::Debug + std::any::Any + Send + Sync {
    /// Access any.
    fn as_any(&self) -> &dyn std::any::Any;
}
impl<T: fmt::Debug + std::any::Any + Send + Sync> FsChangeNote for T {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Handle that holds a [`WATCHER.annotate`] note.
///
/// [`WATCHER.annotate`]: WATCHER::annotate
#[derive(Clone)]
#[must_use = "the note is removed when the handle is dropped"]
pub struct FsChangeNoteHandle(#[allow(dead_code)] Arc<Arc<dyn FsChangeNote>>);

/// Annotation for file watcher events and var update tags.
///
/// Identifies the [`WATCHER.sync`] file that is currently being written to.
///
/// [`WATCHER.sync`]: WATCHER::sync
#[derive(Debug, PartialEq, Eq)]
pub struct WatcherSyncWriteNote(PathBuf);
impl WatcherSyncWriteNote {
    /// Deref.
    pub fn as_path(&self) -> &Path {
        self
    }
}
impl ops::Deref for WatcherSyncWriteNote {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.0.as_path()
    }
}

/// File system change event types.
///
/// The event for each change is available in [`FsChange::event`].
///
/// This module re-exports types from the [`notify`](https://docs.rs/notify) crate.
pub mod fs_event {
    pub use notify::event::{
        AccessKind, AccessMode, CreateKind, DataChange, Event, EventKind, MetadataKind, ModifyKind, RemoveKind, RenameMode,
    };
    pub use notify::{Error, ErrorKind};
}

/// Represents a single file system change, annotated.
#[derive(Debug)]
pub struct FsChange {
    /// All [`WATCHER.annotate`] that where set when this event happened.
    ///
    /// [`WATCHER.annotate`]: WATCHER::annotate
    pub notes: Vec<Arc<dyn FsChangeNote>>,

    /// The actual notify event or error.
    pub event: Result<fs_event::Event, fs_event::Error>,
}
impl FsChange {
    /// If the change affects the `path`.
    pub fn is_for_path(&self, path: &Path) -> bool {
        if let Ok(ev) = &self.event {
            return ev.paths.iter().any(|p| p.starts_with(path));
        }
        false
    }

    /// If the change affects any path matched by the glob pattern.
    pub fn is_for_glob(&self, pattern: &glob::Pattern) -> bool {
        if let Ok(ev) = &self.event {
            return ev.paths.iter().any(|p| pattern.matches_path(p));
        }
        false
    }

    /// Iterate over all notes of the type `T`.
    pub fn notes<T: FsChangeNote>(&self) -> impl Iterator<Item = &T> {
        self.notes.iter().filter_map(|n| FsChangeNote::as_any(&**n).downcast_ref::<T>())
    }
}

event_args! {
     /// [`FS_CHANGES_EVENT`] arguments.
    pub struct FsChangesArgs {
        /// All notify changes since the last event.
        pub changes: Arc<Vec<FsChange>>,

        ..

        /// None, only app level handlers receive this event.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            let _ = list;
        }
    }
}
impl FsChangesArgs {
    /// Iterate over all change events.
    pub fn events(&self) -> impl Iterator<Item = &fs_event::Event> + '_ {
        self.changes.iter().filter_map(|r| r.event.as_ref().ok())
    }

    /// Iterate over all file watcher errors.
    pub fn errors(&self) -> impl Iterator<Item = &notify::Error> + '_ {
        self.changes.iter().filter_map(|r| r.event.as_ref().err())
    }

    /// Returns `true` is some events where lost.
    ///
    /// This indicates either a lapse in the events or a change in the filesystem such that events
    /// received so far can no longer be relied on to represent the state of the filesystem now.
    ///
    /// An application that simply reacts to file changes may not care about this. An application
    /// that keeps an in-memory representation of the filesystem will need to care, and will need
    /// to refresh that representation directly from the filesystem.
    pub fn rescan(&self) -> bool {
        self.events().any(|e| e.need_rescan())
    }

    /// Iterate over all changes that affects paths selected by the `glob` pattern.
    pub fn changes_for(&self, glob: &str) -> Result<impl Iterator<Item = &FsChange> + '_, glob::PatternError> {
        let glob = glob::Pattern::new(glob)?;
        Ok(self.changes.iter().filter(move |c| c.is_for_glob(&glob)))
    }

    /// Iterate over all changes that affects paths that are equal to `path` or inside it.
    pub fn changes_for_path<'a>(&'a self, path: &'a Path) -> impl Iterator<Item = &FsChange> + 'a {
        self.changes.iter().filter(move |c| c.is_for_path(path))
    }

    /// Iterate over all change events that affects that are equal to `path` or inside it.
    pub fn events_for(&self, glob: &str) -> Result<impl Iterator<Item = &fs_event::Event> + '_, glob::PatternError> {
        let glob = glob::Pattern::new(glob)?;
        Ok(self.events().filter(move |ev| ev.paths.iter().any(|p| glob.matches_path(p))))
    }

    /// Iterate over all change events that affects paths that are equal to `path` or inside it.
    pub fn events_for_path<'a>(&'a self, path: &'a Path) -> impl Iterator<Item = &fs_event::Event> + 'a {
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
    sync_debounce: ArcVar<Duration>,
    poll_interval: ArcVar<Duration>,
    shutdown_timeout: ArcVar<Duration>,

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

    fn init_watcher(&mut self) {
        self.watcher.init();
    }

    fn event(&mut self, args: &FsChangesArgs) {
        self.read_to_var.retain_mut(|f| f.on_event(args));
        self.sync_with_var.retain_mut(|f| f.on_event(args));
    }

    fn low_memory(&mut self) {
        self.read_to_var.retain_mut(|v| v.retain());
        let sync_debounce = self.sync_debounce.get();
        self.sync_with_var.retain_mut(|v| v.retain(sync_debounce));
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
        let sync_debounce = self.sync_debounce.get();
        self.sync_with_var.retain_mut(|f| f.retain(sync_debounce));
    }

    fn watch(&mut self, file: PathBuf) -> WatcherHandle {
        self.watcher.watch(file)
    }

    fn watch_dir(&mut self, dir: PathBuf, recursive: bool) -> WatcherHandle {
        self.watcher.watch_dir(dir, recursive)
    }

    fn read<O: VarValue>(
        &mut self,
        file: PathBuf,
        init: O,
        read: impl FnMut(io::Result<WatchFile>) -> Option<O> + Send + 'static,
    ) -> ReadOnlyArcVar<O> {
        let handle = self.watch(file.clone());
        fn open(p: &Path) -> io::Result<WatchFile> {
            WatchFile::open(p)
        }
        let (read, var) = ReadToVar::new(handle, file, init, open, read, || {});
        self.read_to_var.push(read);
        var
    }

    fn read_status<O, S, E>(
        &mut self,
        file: PathBuf,
        init: O,
        mut read: impl FnMut(io::Result<WatchFile>) -> Result<Option<O>, E> + Send + 'static,
    ) -> (ReadOnlyArcVar<O>, ReadOnlyArcVar<S>)
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

    fn read_dir<O: VarValue>(
        &mut self,
        dir: PathBuf,
        recursive: bool,
        init: O,
        read: impl FnMut(walkdir::WalkDir) -> Option<O> + Send + 'static,
    ) -> ReadOnlyArcVar<O> {
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
    fn read_dir_status<O, S, E>(
        &mut self,
        dir: PathBuf,
        recursive: bool,
        init: O,
        mut read: impl FnMut(walkdir::WalkDir) -> Result<Option<O>, E> + Send + 'static,
    ) -> (ReadOnlyArcVar<O>, ReadOnlyArcVar<S>)
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

    fn sync<O: VarValue>(
        &mut self,
        file: PathBuf,
        init: O,
        read: impl FnMut(io::Result<WatchFile>) -> Option<O> + Send + 'static,
        mut write: impl FnMut(O, io::Result<WriteFile>) + Send + 'static,
    ) -> ArcVar<O> {
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
    ) -> (ArcVar<O>, ReadOnlyArcVar<S>)
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

    fn annotate(&mut self, note: Arc<dyn FsChangeNote>) -> FsChangeNoteHandle {
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
    fn shutdown(&mut self) -> Vec<SyncWithVar> {
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
    ) -> (Self, ReadOnlyArcVar<O>) {
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

struct SyncWithVar {
    task: Box<dyn Fn(&WatcherHandle, SyncEvent) + Send + Sync>,
    handle: WatcherHandle,
}
impl SyncWithVar {
    fn new<O, R, W, U>(handle: WatcherHandle, mut file: PathBuf, init: O, read: R, write: W, var_hook_and_modify: U) -> (Self, ArcVar<O>)
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
        var.hook_any(Box::new(clmv!(
            path,
            latest_from_read,
            var_hook_and_modify,
            |args: &AnyVarHookArgs| {
                let is_read = args.downcast_tags::<Arc<WatcherSyncWriteNote>>().any(|n| n == &path);
                latest_from_read.store(is_read, Ordering::Relaxed);
                var_hook_and_modify(is_read);
                true
            }
        )))
        .perm();

        type PendingFlag = u8;
        const READ: PendingFlag = 0b01;
        const WRITE: PendingFlag = 0b11;

        struct TaskData<R, W, O: VarValue> {
            pending: Atomic<PendingFlag>,
            read_write: Mutex<(R, W)>,
            wk_var: WeakArcVar<O>,
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

    fn flush_shutdown(&mut self) {
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
            .spawn(move || loop {
                match rcv.recv_timeout(config.poll_interval_v2().unwrap_or_default()) {
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

/// Calls [`fs4::FileExt::lock_exclusive`] with a timeout.
fn lock_exclusive(file: &impl fs4::FileExt, timeout: Duration) -> std::io::Result<()> {
    lock_timeout(file, |f| f.try_lock_exclusive(), timeout)
}

/// Calls [`fs4::FileExt::lock_shared`] with a timeout.
fn lock_shared(file: &impl fs4::FileExt, timeout: Duration) -> std::io::Result<()> {
    lock_timeout(file, |f| f.try_lock_shared(), timeout)
}

fn lock_timeout<F: fs4::FileExt>(file: &F, try_lock: impl Fn(&F) -> std::io::Result<()>, mut timeout: Duration) -> std::io::Result<()> {
    let mut locked_error = None;
    loop {
        match try_lock(file) {
            Ok(()) => return Ok(()),
            Err(e) => {
                if e.kind() != std::io::ErrorKind::WouldBlock
                    && e.raw_os_error() != locked_error.get_or_insert_with(fs4::lock_contended_error).raw_os_error()
                {
                    return Err(e);
                }

                const INTERVAL: Duration = Duration::from_millis(10);
                timeout = timeout.saturating_sub(INTERVAL);
                if timeout.is_zero() {
                    return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, e));
                } else {
                    thread::sleep(INTERVAL.min(timeout));
                }
            }
        }
    }
}

fn unlock_ok(file: &impl fs4::FileExt) -> std::io::Result<()> {
    if let Err(e) = file.unlock() {
        if let Some(code) = e.raw_os_error() {
            #[cfg(windows)]
            if code == 158 {
                // ERROR_NOT_LOCKED
                return Ok(());
            }

            #[cfg(unix)]
            if code == 22 {
                // EINVAL
                return Ok(());
            }
        }

        Err(e)
    } else {
        Ok(())
    }
}
