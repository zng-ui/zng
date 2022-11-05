use super::*;
use crate::{crate_util::panic_str, units::*};
use std::{
    fs,
    io::{BufReader, BufWriter, Write},
    path::PathBuf,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

/// Builder for [`ConfigFile`].
pub struct ConfigFileBuilder {
    pretty: bool,
    debounce: Duration,
    #[cfg(any(test, doc, feature = "test_util"))]
    read_delay: Option<Duration>,
}
impl Default for ConfigFileBuilder {
    fn default() -> Self {
        Self {
            pretty: true,
            debounce: 1.secs(),
            #[cfg(any(test, doc, feature = "test_util"))]
            read_delay: None,
        }
    }
}
impl ConfigFileBuilder {
    /// If the JSON is formatted when written, is `true` by default.
    pub fn with_pretty(mut self, pretty: bool) -> Self {
        self.pretty = pretty;
        self
    }

    /// Write and reload debounce delay, is `1.secs()` by default.
    pub fn with_debounce(mut self, delay: Duration) -> Self {
        self.debounce = delay;
        self
    }

    /// Read test delay, is not set by default.
    #[cfg(any(test, doc, feature = "test_util"))]
    pub fn with_read_sleep(mut self, delay: Duration) -> Self {
        self.read_delay = Some(delay);
        self
    }

    /// Build the config file, will read and write from the `json_file`.
    pub fn build(self, json_file: impl Into<PathBuf>) -> ConfigFile {
        ConfigFile {
            file: json_file.into(),
            thread: None,
            update: None,
            pretty: self.pretty,
            debounce: self.debounce,
            last_panic: None,
            panic_count: 0,
            is_shutdown: false,

            #[cfg(any(test, doc, feature = "test_util"))]
            read_delay: self.read_delay,
        }
    }
}

/// Config source that read and writes to a single JSON file.
pub struct ConfigFile {
    file: PathBuf,
    thread: Option<(JoinHandle<()>, flume::Sender<Request>)>,
    update: Option<AppExtSender<ConfigSourceUpdate>>,
    pretty: bool,
    debounce: Duration,
    last_panic: Option<Instant>,
    panic_count: usize,
    is_shutdown: bool,

    #[cfg(any(test, doc, feature = "test_util"))]
    read_delay: Option<Duration>,
}
impl ConfigFile {
    /// Start building a config file.
    pub fn builder() -> ConfigFileBuilder {
        Default::default()
    }

    /// New with default config.
    pub fn new(json_file: impl Into<PathBuf>) -> Self {
        Self::builder().build(json_file)
    }

    fn send(&mut self, request: Request) {
        if self.is_shutdown {
            // worker thread is permanently shutdown, can happen in case of repeated panics, or
            match request {
                Request::Read { rsp, .. } => {
                    let _ = rsp.send(Err(ConfigError::new_str("config worker is shutdown")));
                }
                Request::Write { rsp, .. } => {
                    let _ = rsp.send(Err(ConfigError::new_str("config worker is shutdown")));
                }
                Request::Remove { rsp, .. } => {
                    let _ = rsp.send(Err(ConfigError::new_str("config worker is shutdown")));
                }
                Request::Shutdown => {}
            }
        } else if let Some((_, sx)) = &self.thread {
            // worker thread is running, send request

            if sx.send(request).is_err() {
                // worker thread disconnected, can only be due to panic.

                // get panic.
                let thread = self.thread.take().unwrap().0;
                let panic = thread.join().unwrap_err();

                // respawn 5 times inside 1 minute, in case the error is recoverable.
                let now = Instant::now();
                if let Some(last) = self.last_panic {
                    if now.duration_since(last) < 1.minutes() {
                        self.panic_count += 1;
                    } else {
                        self.panic_count = 1;
                    }
                } else {
                    self.panic_count = 1;
                }
                self.last_panic = Some(now);

                if self.panic_count > 5 {
                    self.is_shutdown = true;
                    let update = self.update.as_ref().unwrap();
                    update
                        .send(ConfigSourceUpdate::InternalError(ConfigError::new_str(format!(
                            "config thread panic 5 times in 1 minute, deactivating\nlast panic: {:?}",
                            panic_str(&panic)
                        ))))
                        .unwrap();
                } else {
                    let update = self.update.as_ref().unwrap();
                    update
                        .send(ConfigSourceUpdate::InternalError(ConfigError::new_str(format!(
                            "config thread panic, {:?}",
                            panic_str(&panic)
                        ))))
                        .unwrap();
                    update.send(ConfigSourceUpdate::RefreshAll).unwrap();
                }
            }
        } else {
            // spawn worker thread

            let (sx, rx) = flume::unbounded();
            sx.send(request).unwrap();
            let file = self.file.clone();
            let pretty = self.pretty;
            let debounce = self.debounce;
            #[cfg(any(test, doc, feature = "test_util"))]
            let read_delay = self.read_delay;
            let update = self.update.as_ref().unwrap().clone();

            let handle = thread::Builder::new()
                .name("ConfigFile".to_owned())
                .spawn(move || {
                    if let Some(dir) = file.parent() {
                        if let Err(e) = fs::create_dir_all(dir) {
                            if e.kind() != std::io::ErrorKind::AlreadyExists {
                                panic!("failed to create missing config dir")
                            }
                        }
                    }

                    // load
                    let mut data: HashMap<Text, JsonValue>;
                    let mut data_version;
                    {
                        let mut file = fs::OpenOptions::new()
                            .read(true)
                            .write(true)
                            .create(true)
                            .open(&file)
                            .expect("failed to crate or open config file");

                        let meta = file.metadata().expect("failed to read file metadata");
                        data = if meta.len() > 0 {
                            serde_json::from_reader(&mut BufReader::new(&mut file)).unwrap()
                        } else {
                            HashMap::new()
                        };
                        data_version = meta.modified().ok();
                    };

                    let mut oldest_pending = Instant::now();
                    let mut pending_writes = vec![];
                    let mut write_fails = 0;

                    let mut last_data_version_check = Instant::now();

                    let mut run = true;

                    while run {
                        match rx.recv_timeout(if write_fails > 0 { 1.secs() } else { debounce }) {
                            Ok(request) => match request {
                                Request::Read { key, rsp } => {
                                    let r = data.get(&key).cloned();

                                    #[cfg(any(test, doc, feature = "test_util"))]
                                    if let Some(delay) = read_delay {
                                        crate::task::spawn(async move {
                                            crate::task::deadline(delay).await;
                                            rsp.send(Ok(r)).unwrap();
                                        });
                                    } else {
                                        rsp.send(Ok(r)).unwrap()
                                    }

                                    #[cfg(not(any(test, doc, feature = "test_util")))]
                                    rsp.send(Ok(r)).unwrap();
                                }
                                Request::Write { key, value, rsp } => {
                                    // update entry, but wait for next debounce write.
                                    let write = match data.entry(key) {
                                        Entry::Occupied(mut e) => {
                                            if e.get() != &value {
                                                *e.get_mut() = value;
                                                true
                                            } else {
                                                false
                                            }
                                        }
                                        Entry::Vacant(e) => {
                                            e.insert(value);
                                            true
                                        }
                                    };
                                    if write {
                                        if pending_writes.is_empty() {
                                            oldest_pending = Instant::now();
                                        }
                                        pending_writes.push(rsp);
                                    } else {
                                        rsp.send(Ok(())).unwrap();
                                    }
                                }
                                Request::Remove { key, rsp } => {
                                    if data.remove(&key).is_some() {
                                        if pending_writes.is_empty() {
                                            oldest_pending = Instant::now();
                                        }
                                        pending_writes.push(rsp);
                                    } else {
                                        rsp.send(Ok(())).unwrap();
                                    }
                                }
                                Request::Shutdown => {
                                    // stop running will flush
                                    run = false;
                                }
                            },
                            Err(flume::RecvTimeoutError::Timeout) => {}
                            Err(flume::RecvTimeoutError::Disconnected) => panic!("disconnected"),
                        }

                        if (!pending_writes.is_empty() || write_fails > 0) && (!run || (oldest_pending.elapsed()) >= debounce) {
                            // write debounce elapsed, or is shutting-down, or is trying to recover from write error.

                            // try write
                            let write_result: Result<(), ConfigError> = (|| {
                                let mut file = fs::OpenOptions::new().write(true).create(true).truncate(true).open(&file)?;
                                let file_buf = BufWriter::new(&mut file);
                                if pretty {
                                    serde_json::to_writer_pretty(file_buf, &data)?;
                                } else {
                                    serde_json::to_writer(file_buf, &data)?;
                                };
                                file.flush()?;

                                data_version = file.metadata().unwrap().modified().ok();
                                last_data_version_check = Instant::now();

                                Ok(())
                            })();

                            // notify write listeners
                            for request in pending_writes.drain(..) {
                                let _ = request.send(write_result.clone());
                            }

                            // track error recovery
                            if write_result.is_err() {
                                write_fails += 1;
                                if write_fails > 5 {
                                    // causes a respawn or worker shutdown.
                                    panic!("write failed 5 times in 5 seconds");
                                }
                            } else {
                                write_fails = 0;
                            }
                        }

                        if last_data_version_check.elapsed() >= debounce {
                            // file watcher update.

                            if let Ok(m) = fs::metadata(&file) {
                                let version = m.modified().ok();

                                if data_version != version {
                                    // try reload.
                                    let d: Result<HashMap<Text, JsonValue>, ConfigError> = (|| {
                                        let mut file = fs::File::open(&file)?;
                                        let map = serde_json::from_reader(&mut BufReader::new(&mut file))?;
                                        Ok(map)
                                    })();
                                    if let Ok(d) = d {
                                        for (key, value) in d {
                                            match data.entry(key) {
                                                Entry::Occupied(mut e) => {
                                                    if e.get() != &value {
                                                        // key changed
                                                        *e.get_mut() = value;
                                                        update.send(ConfigSourceUpdate::Refresh(e.key().clone())).unwrap();
                                                    }
                                                }
                                                Entry::Vacant(e) => {
                                                    // new key
                                                    e.insert(value);
                                                }
                                            }
                                        }
                                    }
                                }

                                data_version = version;
                                last_data_version_check = Instant::now();
                            }
                        }
                    }
                })
                .expect("failed to spawn ConfigFile worker thread");

            self.thread = Some((handle, sx));
        }
    }
}
impl ConfigSource for ConfigFile {
    fn init(&mut self, sender: AppExtSender<ConfigSourceUpdate>) {
        self.update = Some(sender);
    }

    fn deinit(&mut self) {
        if let Some((thread, sender)) = self.thread.take() {
            self.is_shutdown = true;
            let _ = sender.send(Request::Shutdown);
            let _ = thread.join();
        }
    }

    fn read(&mut self, key: ConfigKey) -> BoxedFut<Result<Option<JsonValue>, ConfigError>> {
        let (rsp, rcv) = flume::bounded(1);
        self.send(Request::Read { key, rsp });

        Box::pin(async move { rcv.recv_async().await? })
    }

    fn write(&mut self, key: ConfigKey, value: JsonValue) -> BoxedFut<Result<(), ConfigError>> {
        let (rsp, rcv) = flume::bounded(1);
        self.send(Request::Write { key, value, rsp });

        Box::pin(async move { rcv.recv_async().await? })
    }

    fn remove(&mut self, key: ConfigKey) -> BoxedFut<Result<(), ConfigError>> {
        let (rsp, rcv) = flume::bounded(1);
        self.send(Request::Remove { key, rsp });

        Box::pin(async move { rcv.recv_async().await? })
    }
}

enum Request {
    Read {
        key: ConfigKey,
        rsp: flume::Sender<Result<Option<JsonValue>, ConfigError>>,
    },
    Write {
        key: ConfigKey,
        value: JsonValue,
        rsp: flume::Sender<Result<(), ConfigError>>,
    },
    Remove {
        key: ConfigKey,
        rsp: flume::Sender<Result<(), ConfigError>>,
    },
    Shutdown,
}
