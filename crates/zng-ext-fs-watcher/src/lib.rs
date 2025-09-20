#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! File system events and service.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
// suppress nag about very simple boxed closure signatures.
#![expect(clippy::type_complexity)]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::{
    fmt, fs,
    io::{self, Write as _},
    ops,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use path_absolutize::Absolutize;
use zng_app::{
    AppExtension,
    event::{EventHandle, event, event_args},
    handler::{AppHandler, FilterAppHandler},
    update::EventUpdate,
    view_process::raw_events::LOW_MEMORY_EVENT,
};
use zng_handle::Handle;
use zng_txt::Txt;
use zng_unit::TimeUnits;
use zng_var::{Var, VarValue};

mod service;
use service::*;

mod lock;
use lock::*;

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
#[non_exhaustive]
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
///
/// # Panics
///
/// This service requires the [`FsWatcherManager`] extension to work, methods of this service panics if the extension is not part of the app.
pub struct WATCHER;
impl WATCHER {
    /// Gets a read-write variable that defines interval awaited between each [`FS_CHANGES_EVENT`]. If
    /// a watched path is constantly changing an event will be emitted every elapse of this interval,
    /// the event args will contain a list of all the changes observed during the interval.
    ///
    /// Note that the first event notifies immediately, only subsequent events within this interval are debounced.
    ///
    /// Is `100.ms()` by default.
    pub fn debounce(&self) -> Var<Duration> {
        WATCHER_SV.read().debounce.clone()
    }

    /// Gets a read-write variable that defines interval awaited between each [`sync`] write.
    ///
    /// Is `100.ms()` by default.
    ///
    /// [`sync`]: WATCHER::sync
    pub fn sync_debounce(&self) -> Var<Duration> {
        WATCHER_SV.read().debounce.clone()
    }

    /// Gets a read-write variable that defines the fallback poll watcher interval.
    ///
    /// When an efficient watcher cannot be used a poll watcher fallback is used, the poll watcher reads
    /// the directory or path every elapse of this interval. The poll watcher is also used for paths that
    /// do not exist yet, that is also affected by this interval.
    ///
    /// Is `1.secs()` by default.
    pub fn poll_interval(&self) -> Var<Duration> {
        WATCHER_SV.read().poll_interval.clone()
    }

    /// Maximum time the service keeps the process alive to finish pending IO operations when the app shuts down.
    ///
    /// Is 1 minute by default.
    pub fn shutdown_timeout(&self) -> Var<Duration> {
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
    ) -> Var<O> {
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
    ) -> (Var<O>, Var<S>)
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
    ) -> Var<O> {
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
    ) -> (Var<O>, Var<S>)
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
    /// The file synchronization ensures that the file is only actually modified when write is finished by writing
    /// to a temporary file and committing a replace only if the write succeeded. The file is write-locked for the duration
    /// of `write` call, but the contents are not touched until commit. See [`WriteFile`] for more details.
    ///
    /// The [`FsWatcherManager`] blocks on app exit until all writes commit or cancel. See [`WATCHER::shutdown_timeout`] for
    /// more details.
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
    /// used to get a signal out to handle the error, perhaps drop the sync var (to stop watching), alert the user that the
    /// file is out of sync and initiate some sort of recovery routine.
    ///
    /// If the file synchronization is not important you can just ignore it, the watcher will try again
    /// on the next variable or file update.
    ///
    /// ## Status
    ///
    /// Note that `read` and `write` run in background task threads, so if you are tracking the operation
    /// status in a separate variable you may end-up with synchronization bugs between the status variable
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
    ) -> Var<O> {
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
    ) -> (Var<O>, Var<S>)
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

    /// Push a `note` that will be cloned on all subsequent change events until the returned handle is dropped.
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
        let _ = FileExt::unlock(&self.0);
    }
}

const TRANSACTION_GUID: &str = "6eIw3bYMS0uKaQMkTIQacQ";
const TRANSACTION_LOCK_EXT: &str = "6eIw3bYMS0uKaQMkTIQacQ-lock.tmp";

/// Represents an open write file provided by [`WATCHER.sync`].
///
/// This struct writes to a temporary file and renames it over the actual file on commit only.
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
        if !actual_path.exists()
            && let Some(parent) = actual_path.parent()
        {
            std::fs::create_dir_all(parent)?;
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
            if let Ok(f) = fs::OpenOptions::new().write(true).create(true).truncate(true).open(&temp_path)
                && let Ok(true) = f.try_lock_exclusive()
            {
                break f;
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
        let buf = io::BufWriter::new(ops::DerefMut::deref_mut(self));
        struct Ffs<'a> {
            w: io::BufWriter<&'a mut fs::File>,
        }
        impl fmt::Write for Ffs<'_> {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                self.w.write_all(s.as_bytes()).map_err(|_| fmt::Error)
            }
        }
        let mut buf = Ffs { w: buf };
        if pretty {
            ron::ser::to_writer_pretty(&mut buf, value, Default::default()).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        } else {
            ron::ser::to_writer(&mut buf, value).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        }
        buf.w.flush()
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
                            // Give-up, we managed to write lock both temp and actual just
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
            let _ = FileExt::unlock(&tmp);
        }
        if let Err(e) = fs::remove_file(&self.temp_path) {
            tracing::debug!("failed to cleanup temp file, {e}")
        }

        if let Some(file) = self.actual_file.take() {
            let _ = FileExt::unlock(&file);
        }

        let transaction = self.transaction_lock.take().unwrap();
        let _ = FileExt::unlock(&transaction);
        let _ = fs::remove_file(&self.transaction_path);
    }
}

/// Error for [`WatchFile::parse`].
#[derive(Debug)]
#[non_exhaustive]
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
/// This trait is already implemented for all types it applies.
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
pub struct FsChangeNoteHandle(#[expect(dead_code)] Arc<Arc<dyn FsChangeNote>>);

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
#[non_exhaustive]
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
    pub fn changes_for_path<'a>(&'a self, path: &'a Path) -> impl Iterator<Item = &'a FsChange> + 'a {
        self.changes.iter().filter(move |c| c.is_for_path(path))
    }

    /// Iterate over all change events that affects that are equal to `path` or inside it.
    pub fn events_for(&self, glob: &str) -> Result<impl Iterator<Item = &fs_event::Event> + '_, glob::PatternError> {
        let glob = glob::Pattern::new(glob)?;
        Ok(self.events().filter(move |ev| ev.paths.iter().any(|p| glob.matches_path(p))))
    }

    /// Iterate over all change events that affects paths that are equal to `path` or inside it.
    pub fn events_for_path<'a>(&'a self, path: &'a Path) -> impl Iterator<Item = &'a fs_event::Event> + 'a {
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
