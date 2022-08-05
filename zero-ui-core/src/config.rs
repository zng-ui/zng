//! Config manager.
//!
//! The [`ConfigManager`] is an [app extension], it
//! is included in the [default app] and manages the [`Config`] service that can be used to store and retrieve
//! state that is persisted between application runs.
//!
//! [app extension]: crate::app::AppExtension
//! [default app]: crate::app::App::default

use std::{
    cell::Cell,
    collections::{hash_map::Entry, HashMap, HashSet},
    error::Error,
    rc::Rc,
};

use crate::{
    app::{AppEventSender, AppExtReceiver, AppExtSender, AppExtension},
    context::*,
    service::*,
    text::Text,
    var::*,
};

use serde_json::value::Value as JsonValue;

/// Application extension that manages the app configuration access point ([`Config`]).
///
/// Note that this extension does not implement a [`ConfigBackend`], it just manages whatever backend is installed and
/// the watcher variables.
#[derive(Default)]
pub struct ConfigManager {}
impl AppExtension for ConfigManager {
    fn init(&mut self, ctx: &mut AppContext) {
        ctx.services.register(Config::new(ctx.updates.sender()));
    }

    fn update_preview(&mut self, ctx: &mut AppContext) {
        let config = Config::req(ctx.services);

        // run once tasks
        for task in config.once_tasks.drain(..) {
            task(ctx.vars, &config.status);
        }

        // collect backend updates
        let mut read = HashSet::new();
        let mut read_all = false;
        if let Some((_, backend_tasks)) = &config.backend {
            while let Ok(task) = backend_tasks.try_recv() {
                match task {
                    ConfigBackendUpdate::Refresh(key) => {
                        if !read_all {
                            read.insert(key);
                        }
                    }
                    ConfigBackendUpdate::RefreshAll => read_all = true,
                }
            }
        }

        // run retained tasks
        config.tasks.retain_mut(|t| t(ctx.vars, &config.status));

        // Update config vars:
        // - Remove dropped vars.
        // - React to var assigns.
        // - Apply backend requests.
        let mut var_tasks = vec![];
        config.vars.retain(|key, var| match var.upgrade(ctx.vars) {
            Some((any_var, write)) => {
                if write {
                    // var was set by the user, start a write task.
                    var_tasks.push(var.write(ConfigVarTaskArgs {
                        vars: ctx.vars,
                        key,
                        var: any_var,
                    }));
                } else if read_all || read.contains(key) {
                    // backend notified a potential change, start a read task.
                    var_tasks.push(var.read(ConfigVarTaskArgs {
                        vars: ctx.vars,
                        key,
                        var: any_var,
                    }));
                }
                true // retain var
            }
            None => false, // var was dropped, remove entry
        });

        for task in var_tasks {
            task(config);
        }
    }
}

/// Key to a persistent config in [`Config`].
pub type ConfigKey = Text;

/// A type that can be a [`Config`] value.
///
/// # Trait Alias
///
/// This trait is used like a type alias for traits and is
/// already implemented for all types it applies to.
pub trait ConfigValue: VarValue + PartialEq + serde::Serialize + serde::de::DeserializeOwned {}
impl<T: VarValue + PartialEq + serde::Serialize + serde::de::DeserializeOwned> ConfigValue for T {}

/// Return `true` to retain, `false` to drop.
type ConfigTask = Box<dyn FnMut(&Vars, &RcVar<ConfigStatus>) -> bool>;
type OnceConfigTask = Box<dyn FnOnce(&Vars, &RcVar<ConfigStatus>)>;

/// Represents the config of the app.
///
/// This type does not implement any config scheme, a [`ConfigBackend`] must be installed to enable persistence, without a backend
/// only the config variables work.
#[derive(Service)]
pub struct Config {
    update: AppEventSender,
    backend: Option<(Box<dyn ConfigBackend>, AppExtReceiver<ConfigBackendUpdate>)>,
    vars: HashMap<ConfigKey, ConfigVar>,

    status: RcVar<ConfigStatus>,

    once_tasks: Vec<OnceConfigTask>,
    tasks: Vec<ConfigTask>,
}
impl Config {
    fn new(update: AppEventSender) -> Self {
        Config {
            update,
            backend: None,
            vars: HashMap::new(),

            status: var(ConfigStatus::default()),

            once_tasks: vec![],
            tasks: vec![],
        }
    }

    /// Install a config backend, replaces the previous backend.
    pub fn set_backend(&mut self, mut backend: impl ConfigBackend) {
        let (sender, receiver) = self.update.ext_channel();
        if !self.vars.is_empty() {
            let _ = sender.send(ConfigBackendUpdate::RefreshAll);
        }

        backend.init(sender);
        self.backend = Some((Box::new(backend), receiver));
    }

    /// Gets a variable that tracks the backend write tasks.
    pub fn status(&self) -> ReadOnlyRcVar<ConfigStatus> {
        self.status.clone().into_read_only()
    }

    /// Read the config value currently associated with the `key` if it is of the same type.
    ///
    /// Returns a [`ResponseVar`] that will update once when the value finishes reading.
    pub fn read<K, T>(&mut self, key: K) -> ResponseVar<Option<T>>
    where
        K: Into<ConfigKey>,
        T: ConfigValue,
    {
        self.read_impl(key.into())
    }
    fn read_impl<T>(&mut self, key: ConfigKey) -> ResponseVar<Option<T>>
    where
        T: ConfigValue,
    {
        // channel with the caller.
        let (responder, rsp) = response_var();

        self.read_raw(key, move |vars, r| {
            responder.respond(vars, r);
        });

        rsp
    }
    fn read_raw<T, R>(&mut self, key: ConfigKey, respond: R)
    where
        T: ConfigValue,
        R: FnOnce(&Vars, Option<T>) + 'static,
    {
        if let Some((backend, _)) = &mut self.backend {
            // channel with the backend.
            let (sender, receiver) = self.update.ext_channel_bounded(1);
            backend.read(key, sender);

            // bind two channels.
            let mut respond = Some(respond);
            self.tasks.push(Box::new(move |vars, _| {
                match receiver.try_recv() {
                    Ok(Ok(r)) => {
                        let respond = respond.take().unwrap();
                        respond(vars, r.and_then(|v| serde_json::from_value(v).ok()));
                        false
                    }
                    Err(None) => true, // retain
                    Ok(Err(_)) | Err(_) => {
                        let respond = respond.take().unwrap();
                        respond(vars, None);
                        false
                    }
                }
            }));
        } else {
            // no backend, just respond with `None`.
            self.once_tasks.push(Box::new(move |vars, _| {
                respond(vars, None);
            }));
            let _ = self.update.send_ext_update();
        }
    }

    /// Write the config value associated with the `key`.
    pub fn write<K, T>(&mut self, key: K, value: T)
    where
        K: Into<ConfigKey>,
        T: ConfigValue,
    {
        self.write_impl(key.into(), value)
    }
    fn write_impl<T>(&mut self, key: ConfigKey, value: T)
    where
        T: ConfigValue,
    {
        // register variable update if the entry is observed.
        let key = match self.vars.entry(key) {
            Entry::Occupied(entry) => {
                let key = entry.key().clone();
                if let Some(var) = entry.get().downcast::<T>() {
                    let value = value.clone();

                    self.once_tasks.push(Box::new(move |vars, _| {
                        var.modify(vars, move |mut v| {
                            if v.value != value {
                                v.value = value;
                                v.write.set(false);
                            }
                        });
                    }));

                    let _ = self.update.send_ext_update();
                } else {
                    // not observed anymore or changed type.
                    entry.remove();
                }
                key
            }
            Entry::Vacant(e) => e.into_key(),
        };

        // serialize and request write.
        self.write_backend(key, value);
    }
    fn write_backend<T>(&mut self, key: ConfigKey, value: T)
    where
        T: ConfigValue,
    {
        if let Some((backend, _)) = &mut self.backend {
            match serde_json::value::to_value(value) {
                Ok(json) => {
                    let (sx, rx) = self.update.ext_channel_bounded(1);
                    backend.write(key, json, sx);

                    let mut count = 0;
                    self.tasks.push(Box::new(move |vars, status| {
                        match rx.try_recv() {
                            Ok(r) => {
                                status.modify(vars, move |mut s| {
                                    s.pending -= count;
                                    if let Err(e) = r {
                                        s.last_error = Some(e.into());
                                    }
                                });
                                false // task finished
                            }
                            Err(None) => {
                                if count == 0 {
                                    // first try, add pending.
                                    count = 1;
                                    status.modify(vars, |mut s| s.pending += 1);
                                }
                                true // retain
                            }
                            Err(Some(e)) => {
                                status.modify(vars, move |mut s| {
                                    s.pending -= count;
                                    s.last_error = Some(Rc::new(e))
                                });
                                false // task finished
                            }
                        }
                    }));
                }
                Err(e) => {
                    self.once_tasks.push(Box::new(move |vars, status| {
                        status.modify(vars, move |mut s| s.last_error = Some(Rc::new(e)));
                    }));
                }
            }
        }
    }

    /// Gets a variable that updates every time the config associated with `key` changes and writes the config
    /// every time it changes. This is equivalent of a two-way binding between the config storage and the variable.
    ///
    /// If the config is not already observed the `default_value` is used to generate a variable that will update with the current value
    /// after it is read.
    pub fn var<K, T, D>(&mut self, key: K, default_value: D) -> impl Var<T>
    where
        K: Into<ConfigKey>,
        T: ConfigValue,
        D: FnOnce() -> T,
    {
        self.var_with_source(key.into(), default_value).map_ref_bidi(
            |v| &v.value,
            |v| {
                v.write.set(true);
                &mut v.value
            },
        )
    }

    /// Binds a variable that updates every time the config associated with `key` changes and writes the config
    /// every time it changes. If the `target` is dropped the binding is dropped.
    ///
    /// If the config is not already observed the `default_value` is used to generate a variable that will update with the current value
    /// after it is read.
    pub fn bind<Vw: WithVars, K: Into<ConfigKey>, T: ConfigValue, D: FnOnce() -> T, V: Var<T>>(
        &mut self,
        vars: &Vw,
        key: K,
        default_value: D,
        target: &V,
    ) -> VarBindingHandle {
        let source = self.var_with_source(key.into(), default_value);
        vars.with_vars(|vars| {
            if let Some(target) = target.actual_var(vars).downgrade() {
                vars.bind(move |vars, binding| {
                    if let Some(target) = target.upgrade() {
                        if let Some(v) = source.get_new(vars) {
                            // backend updated, notify
                            let _ = target.set_ne(vars, v.value.clone());
                        }
                        if let Some(value) = target.clone_new(vars) {
                            // user updated, write
                            source.modify(vars, move |mut v| {
                                if v.value != value {
                                    Cell::set(&v.write, true);
                                    v.value = value;
                                }
                            });
                        }
                    } else {
                        // dropped target, drop binding
                        binding.unbind();
                    }
                })
            } else {
                VarBindingHandle::dummy()
            }
        })
    }

    fn var_with_source<T: ConfigValue>(&mut self, key: ConfigKey, default_value: impl FnOnce() -> T) -> RcVar<ValueWithSource<T>> {
        let refresh;

        let r = match self.vars.entry(key) {
            Entry::Occupied(mut entry) => {
                if let Some(var) = entry.get().downcast::<T>() {
                    return var; // already observed and is the same type.
                }

                // entry stale or for the wrong type:

                // re-insert observer
                let (cfg_var, var) = ConfigVar::new(default_value());
                *entry.get_mut() = cfg_var;

                // and refresh the value.
                refresh = (entry.key().clone(), var.clone());

                var
            }
            Entry::Vacant(entry) => {
                let (cfg_var, var) = ConfigVar::new(default_value());

                refresh = (entry.key().clone(), var.clone());

                entry.insert(cfg_var);

                var
            }
        };

        let (key, var) = refresh;
        let value = self.read::<_, T>(key);
        self.tasks.push(Box::new(move |vars, _| {
            if let Some(rsp) = value.rsp_clone(vars) {
                if let Some(value) = rsp {
                    var.modify(vars, move |mut v| {
                        if v.value != value {
                            v.value = value;
                            v.write.set(false);
                        }
                    });
                }
                false // task finished
            } else {
                true // retain
            }
        }));

        r
    }
}

type VarUpdateTask = Box<dyn FnOnce(&mut Config)>;

/// ConfigVar actual value, tracks if updates need to be send to backend.
#[derive(Debug, Clone, PartialEq)]
struct ValueWithSource<T: ConfigValue> {
    value: T,
    write: Rc<Cell<bool>>,
}

struct ConfigVar {
    var: Box<dyn AnyWeakVar>,
    write: Rc<Cell<bool>>,
    run_task: Box<dyn Fn(ConfigVarTask, ConfigVarTaskArgs) -> VarUpdateTask>,
}
impl ConfigVar {
    fn new<T: ConfigValue>(initial_value: T) -> (Self, RcVar<ValueWithSource<T>>) {
        let write = Rc::new(Cell::new(false));
        let var = var(ValueWithSource {
            value: initial_value,
            write: write.clone(),
        });
        let r = ConfigVar {
            var: var.downgrade().into_any(),
            write,
            run_task: Box::new(ConfigVar::run_task_impl::<T>),
        };
        (r, var)
    }

    /// Returns var and if it needs to write.
    fn upgrade(&mut self, vars: &Vars) -> Option<(Box<dyn AnyVar>, bool)> {
        self.var.upgrade_any().map(|v| {
            let write = self.write.get() && v.is_new_any(vars);
            (v, write)
        })
    }

    fn downcast<T: ConfigValue>(&self) -> Option<RcVar<ValueWithSource<T>>> {
        self.var.as_any().downcast_ref::<types::WeakRcVar<ValueWithSource<T>>>()?.upgrade()
    }

    fn read(&self, args: ConfigVarTaskArgs) -> VarUpdateTask {
        (self.run_task)(ConfigVarTask::Read, args)
    }
    fn write(&self, args: ConfigVarTaskArgs) -> VarUpdateTask {
        (self.run_task)(ConfigVarTask::Write, args)
    }
    fn run_task_impl<T: ConfigValue>(task: ConfigVarTask, args: ConfigVarTaskArgs) -> VarUpdateTask {
        if let Some(var) = args.var.as_any().downcast_ref::<RcVar<ValueWithSource<T>>>() {
            match task {
                ConfigVarTask::Read => {
                    let key = args.key.clone();
                    let var = var.clone();
                    Box::new(move |config| {
                        config.read_raw::<T, _>(key, move |vars, value| {
                            if let Some(value) = value {
                                var.modify(vars, move |mut v| {
                                    if v.value != value {
                                        v.value = value;
                                        v.write.set(false);
                                    }
                                });
                            }
                        });
                    })
                }
                ConfigVarTask::Write => {
                    let key = args.key.clone();
                    let value = var.get_clone(args.vars).value;
                    Box::new(move |config| {
                        config.write_backend(key, value);
                    })
                }
            }
        } else {
            Box::new(|_| {})
        }
    }
}

struct ConfigVarTaskArgs<'a> {
    vars: &'a Vars,
    key: &'a ConfigKey,
    var: Box<dyn AnyVar>,
}

enum ConfigVarTask {
    Read,
    Write,
}

/// Current [`Config`] status.
#[derive(Debug, Clone, Default)]
pub struct ConfigStatus {
    /// Number of pending writes.
    pub pending: usize,

    /// Last write error.
    pub last_error: Option<Rc<dyn Error + Send>>,
}

/// Represents an implementation of [`Config`].
pub trait ConfigBackend: 'static {
    /// Called once when the backend is installed.
    fn init(&mut self, observer: AppExtSender<ConfigBackendUpdate>);

    /// Send a read request for the most recent value associated with `key` in the persistent storage.
    ///
    /// The `rsp` channel must be used once to send back the result.
    fn read(&mut self, key: ConfigKey, rsp: AppExtSender<Result<Option<JsonValue>, Box<dyn Error + Send>>>);
    /// Send a write request to set the `value` for `key` on the persistent storage.
    ///
    /// The `rsp` channel must be used once to send back the result.
    fn write(&mut self, key: ConfigKey, value: JsonValue, rsp: AppExtSender<Result<(), Box<dyn Error + Send>>>);
}

/// External updates in a [`ConfigBackend`].
#[derive(Clone, Debug)]
pub enum ConfigBackendUpdate {
    /// Value associated with the key may have changed from an external event, **not** a write operation.
    Refresh(ConfigKey),
    /// All values may have changed.
    RefreshAll,
}

mod file_backend {
    use super::*;
    use crate::{crate_util::panic_str, units::*};
    use std::{
        fs,
        io::{BufReader, BufWriter, Seek, SeekFrom},
        path::PathBuf,
        thread::{self, JoinHandle},
        time::{Instant, Duration},
    };

    /// Simple [`ConfigBackend`] that writes all settings to a JSON file.
    pub struct ConfigFile {
        file: PathBuf,
        thread: Option<(JoinHandle<()>, flume::Sender<Request>)>,
        update: Option<AppExtSender<ConfigBackendUpdate>>,
        pretty: bool,
        delay: Duration,
        last_panic: Option<Instant>,
        panic_count: usize,
    }
    impl ConfigFile {
        /// New with the path to the JSON config file.
        pub fn new(json_file: impl Into<PathBuf>, pretty: bool) -> Self {
            ConfigFile {
                file: json_file.into(),
                thread: None,
                update: None,
                pretty,
                delay: 1.secs(),
                last_panic: None,
                panic_count: 0,
            }
        }

        fn send(&mut self, request: Request) {
            if let Some((_, sx)) = &self.thread {
                if sx.send(request).is_err() {
                    let thread = self.thread.take().unwrap().0;
                    let panic = thread.join().unwrap_err();

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
                        std::panic::resume_unwind(panic);
                    } else {
                        tracing::error!("config thread panic, {:?}", panic_str(&panic));
                        self.update.as_ref().unwrap().send(ConfigBackendUpdate::RefreshAll).unwrap();
                    }
                }
            } else {
                let (sx, rx) = flume::unbounded();
                sx.send(request).unwrap();
                let file = self.file.clone();
                let pretty = self.pretty;
                let delay = self.delay;
                let handle = thread::spawn(move || {
                    let mut file = fs::OpenOptions::new().read(true).write(true).create(true).open(file).unwrap();
                    let mut data: HashMap<String, JsonValue> = if file.metadata().unwrap().len() == 0 {
                        HashMap::new()
                    } else {
                        serde_json::from_reader(&mut BufReader::new(&mut file)).unwrap()
                    };

                    let mut last_write = Instant::now();
                    let mut pending_writes = vec![];

                    loop {
                        match rx.recv_timeout(delay) {
                            Ok(request) => {
                                match request {
                                    Request::Read { key, rsp } => rsp.send(Ok(data.get(&key.into_owned()).cloned())).unwrap(),
                                    Request::Write { key, value, rsp } => {
                                        let write = match data.entry(key.into_owned()) {
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
                                            pending_writes.push(rsp);
                                        } else {
                                            rsp.send(Ok(())).unwrap();
                                        }
                                    },
                                }
                            },
                            Err(flume::RecvTimeoutError::Timeout) => {}
                            Err(flume::RecvTimeoutError::Disconnected) => panic!("disconnected"),
                        }

                        if !pending_writes.is_empty() && last_write.elapsed() >= delay {
                            let write_result: Result<(), Box<dyn Error + Send + Sync>> = (|| {
                                file.seek(SeekFrom::Start(0))?;
                                file.set_len(0)?;
                                
                                if pretty {
                                    serde_json::to_writer_pretty(BufWriter::new(&mut file), &data)?;
                                } else {
                                    serde_json::to_writer(BufWriter::new(&mut file), &data)?;
                                };

                                Ok(())
                            })();

                            for request in pending_writes {
                                let _ = request.send(write_result);
                            }
                        }
                    }
                });

                self.thread = Some((handle, sx));
            }
        }
    }
    impl ConfigBackend for ConfigFile {
        fn init(&mut self, sender: AppExtSender<ConfigBackendUpdate>) {
            self.update = Some(sender);
        }

        fn read(&mut self, key: ConfigKey, rsp: AppExtSender<Result<Option<JsonValue>, Box<dyn Error + Send>>>) {
            self.send(Request::Read { key, rsp })
        }

        fn write(&mut self, key: ConfigKey, value: JsonValue, rsp: AppExtSender<Result<(), Box<dyn Error + Send>>>) {
            self.send(Request::Write { key, value, rsp })
        }
    }

    enum Request {
        Read {
            key: ConfigKey,
            rsp: AppExtSender<Result<Option<JsonValue>, Box<dyn Error + Send>>>,
        },
        Write {
            key: ConfigKey,
            value: JsonValue,
            rsp: AppExtSender<Result<(), Box<dyn Error + Send>>>,
        },
    }
}
pub use file_backend::ConfigFile;
