use notify::Watcher as _;
use parking_lot::Mutex;
use std::{
    collections::{HashMap, hash_map},
    io, mem,
    path::{Path, PathBuf},
    time::Duration,
};
use zng_handle::{Handle, HandleOwner};
use zng_unit::TimeUnits;

use crate::{WatcherHandle, fs_event};

pub struct Watchers {
    dirs: HashMap<PathBuf, DirWatcher>,
    watcher: Mutex<Box<dyn notify::Watcher + Send>>, // mutex for Sync only
    // watcher for paths that the system watcher cannot watch yet.
    error_watcher: Option<PollWatcher>,
    poll_interval: Duration,
}
impl Watchers {
    pub fn new() -> Self {
        Self {
            dirs: HashMap::default(),
            watcher: Mutex::new(Box::new(notify::NullWatcher)),
            error_watcher: None,
            poll_interval: 1.secs(),
        }
    }

    pub fn watch(&mut self, file: PathBuf) -> WatcherHandle {
        self.watch_insert(file, WatchMode::File(std::ffi::OsString::new()))
    }

    pub fn watch_dir(&mut self, dir: PathBuf, recursive: bool) -> WatcherHandle {
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
            if m == &mode
                && let Some(h) = handle.weak_handle().upgrade()
            {
                return WatcherHandle(h);
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

    pub fn set_poll_interval(&mut self, interval: Duration) {
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

    pub fn init(&mut self) {
        *self.watcher.get_mut() = match notify::recommended_watcher(super::notify_watcher_handler()) {
            Ok(w) => Box::new(w),
            Err(e) => {
                tracing::error!("error creating watcher\n{e}\nfallback to slow poll watcher");
                match PollWatcher::new(
                    super::notify_watcher_handler(),
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

    pub fn deinit(&mut self) {
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
                super::notify_watcher_handler(),
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

    pub fn allow(&mut self, r: &fs_event::Event) -> bool {
        if let notify::EventKind::Access(_) = r.kind
            && !r.need_rescan()
        {
            return false;
        }

        for (dir, w) in &mut self.dirs {
            let mut matched = false;

            'modes: for (mode, _) in &w.modes {
                match mode {
                    WatchMode::File(f) => {
                        for path in &r.paths {
                            if let Some(name) = path.file_name()
                                && name == f
                                && let Some(path) = path.parent()
                                && path == dir
                            {
                                // matched `dir/exact`
                                matched = true;
                                break 'modes;
                            }
                        }
                    }
                    WatchMode::Children => {
                        for path in &r.paths {
                            if let Some(path) = path.parent()
                                && path == dir
                            {
                                // matched `dir/*`
                                matched = true;
                                break 'modes;
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
        if self.sender.send(msg).is_err()
            && let Some(worker) = self.worker.take()
            && let Err(panic) = worker.join()
        {
            std::panic::resume_unwind(panic);
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
