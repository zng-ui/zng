//! Config manager.
//!
//! The [`ConfigManager`] is an [app extension], it
//! is included in the [default app] and manages the [`Config`] service that can be used to store and retrieve
//! state that is persisted between application runs.
//!
//! [app extension]: crate::app::AppExtension
//! [default app]: crate::app::App::default

use std::{
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

        for task in config.once_tasks.drain(..) {
            task(ctx.vars, &config.status);
        }

        let mut read = HashSet::new();
        let mut read_all = false;

        if let Some((_, backend_tasks)) = &config.backend {
            while let Ok(task) = backend_tasks.try_recv() {
                match task {
                    ConfigBackendUpdate::ExternalChange(key) => {
                        if !read_all {
                            read.insert(key);
                        }
                    }
                    ConfigBackendUpdate::ExternalChangeAll => read_all = true,
                }
            }
        }

        config.tasks.retain_mut(|t| t(ctx.vars, &config.status));

        let mut var_tasks = vec![];
        config.vars.retain(|key, var| match var.upgrade(ctx.vars) {
            Some((any_var, write)) => {
                if write {
                    var_tasks.push(var.write(ConfigVarTaskArgs {
                        vars: ctx.vars,
                        key,
                        var: any_var,
                    }));
                } else if read_all || read.remove(key) {
                    var_tasks.push(var.read(ConfigVarTaskArgs {
                        vars: ctx.vars,
                        key,
                        var: any_var,
                    }));
                }
                true
            }
            None => false,
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
    pub fn install_backend(&mut self, mut backend: impl ConfigBackend) {
        let (sender, receiver) = self.update.ext_channel();
        if !self.vars.is_empty() {
            let _ = sender.send(ConfigBackendUpdate::ExternalChangeAll);
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
        if let Some((backend, _)) = &self.backend {
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
                        var.set_ne(vars, value);
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
        if let Some((backend, _)) = &self.backend {
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

    /// Gets a variable that updates every time the config associated with `key` changes and updates the config
    /// every time it changes.
    ///
    /// This is equivalent of a two-way binding between the config storage and the variable.
    ///
    /// If the config is not already observed the `default_value` is used to generate a variable that will update with the current value
    /// after it is read.
    pub fn var<K, T, D>(&mut self, key: K, default_value: D) -> ReadOnlyRcVar<T>
    where
        K: Into<ConfigKey>,
        T: ConfigValue,
        D: FnOnce() -> T,
    {
        self.var_impl(key.into(), default_value).into_read_only()
    }
    fn var_impl<T: ConfigValue>(&mut self, key: ConfigKey, default_value: impl FnOnce() -> T) -> RcVar<T> {
        let refresh;

        let r = match self.vars.entry(key) {
            Entry::Occupied(mut entry) => {
                if let Some(var) = entry.get().downcast::<T>() {
                    return var; // already observed and is the same type.
                }

                // entry stale or for the wrong type:

                // re-insert observer
                let var = var(default_value());
                *entry.get_mut() = ConfigVar::new(&var);

                // and refresh the value.
                refresh = (entry.key().clone(), var.clone());

                var
            }
            Entry::Vacant(entry) => {
                let var = var(default_value());

                refresh = (entry.key().clone(), var.clone());

                entry.insert(ConfigVar::new(&var));

                var
            }
        };

        let (key, var) = refresh;
        let value = self.read::<_, T>(key);
        self.tasks.push(Box::new(move |vars, _| {
            if let Some(rsp) = value.rsp_clone(vars) {
                if let Some(value) = rsp {
                    var.set_ne(vars, value);
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

struct ConfigVar {
    var: Box<dyn AnyWeakVar>,
    run_task: Box<dyn Fn(ConfigVarTask, ConfigVarTaskArgs) -> VarUpdateTask>,
}
impl ConfigVar {
    fn new<T: ConfigValue>(var: &RcVar<T>) -> Self {
        ConfigVar {
            var: var.downgrade().into_any(),
            run_task: Box::new(ConfigVar::run_task_impl::<T>),
        }
    }

    /// Returns var and if it needs to write.
    fn upgrade(&mut self, vars: &Vars) -> Option<(Box<dyn AnyVar>, bool)> {
        self.var.upgrade_any().map(|v| {
            let write = v.is_new_any(vars);
            (v, write)
        })
    }

    fn downcast<T: ConfigValue>(&self) -> Option<RcVar<T>> {
        self.var.as_any().downcast_ref::<types::WeakRcVar<T>>()?.upgrade()
    }

    fn read(&self, args: ConfigVarTaskArgs) -> VarUpdateTask {
        (self.run_task)(ConfigVarTask::Read, args)
    }
    fn write(&self, args: ConfigVarTaskArgs) -> VarUpdateTask {
        (self.run_task)(ConfigVarTask::Write, args)
    }
    fn run_task_impl<T: ConfigValue>(task: ConfigVarTask, args: ConfigVarTaskArgs) -> VarUpdateTask {
        if let Some(var) = args.var.as_any().downcast_ref::<RcVar<T>>() {
            match task {
                ConfigVarTask::Read => {
                    let key = args.key.clone();
                    let var = var.clone();
                    Box::new(move |config| {
                        config.read_raw::<T, _>(key, move |vars, value| {
                            if let Some(value) = value {
                                var.set_ne(vars, value);
                            }
                        });
                    })
                }
                ConfigVarTask::Write => {
                    let key = args.key.clone();
                    let value = var.get_clone(args.vars);
                    Box::new(move |config| {
                        config.write_impl(key, value);
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
    fn read(&self, key: ConfigKey, rsp: AppExtSender<Result<Option<JsonValue>, Box<dyn Error + Send>>>);
    /// Send a write request to set the `value` for `key` on the persistent storage.
    ///
    /// The `rsp` channel must be used once to send back the result.
    fn write(&self, key: ConfigKey, value: JsonValue, rsp: AppExtSender<Result<(), Box<dyn Error + Send>>>);
}

/// External updates in a [`ConfigBackend`].
#[derive(Clone, Debug)]
pub enum ConfigBackendUpdate {
    /// Value associated with the key may have changed from an external event, **not** a write operation.
    ExternalChange(ConfigKey),
    /// All values may have changed.
    ExternalChangeAll,
}
