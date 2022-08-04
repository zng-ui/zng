//! Config manager.
//!
//! The [`ConfigManager`] is an [app extension], it
//! is included in the [default app] and manages the [`Config`] service that can be used to store and retrieve
//! state that is persisted between application runs.
//!
//! [app extension]: crate::app::AppExtension
//! [default app]: crate::app::App::default

use std::{
    any::Any,
    collections::{hash_map::Entry, HashMap},
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
pub struct ConfigManager {}
impl AppExtension for ConfigManager {
    fn init(&mut self, ctx: &mut AppContext) {
        ctx.services.register(Config::new(ctx.updates.sender()));
    }

    fn update_preview(&mut self, ctx: &mut AppContext) {
        todo!()
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

/// Represents the config of the app.
///
/// This type does not implement any config scheme, a [`ConfigBackend`] must be installed to enable persistence, without a backend
/// only the config variables work.
#[derive(Service)]
pub struct Config {
    update: AppEventSender,
    backend: Option<(Box<dyn ConfigBackend>, AppExtReceiver<ConfigBackendUpdate>)>,
    vars: HashMap<ConfigKey, Box<dyn Any>>,

    status: RcVar<ConfigStatus>,

    once_tasks: Vec<Box<dyn FnOnce(&Vars)>>,
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
            sender.send(ConfigBackendUpdate::ExternalChangeAll);
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
        todo!()
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
                if let Some(var) = entry.get().downcast_ref::<types::WeakRcVar<T>>().and_then(|v| v.upgrade()) {
                    let value = value.clone();

                    self.once_tasks.push(Box::new(move |vars| {
                        var.set_ne(vars, value);
                    }));

                    let _ = self.update.send_ext_update();
                } else {
                    // not observed anymore.
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
                            Err(flume::TryRecvError::Empty) => {
                                if count == 0 {
                                    // first try, add pending.
                                    count = 1;
                                    status.modify(vars, |mut s| s.pending += 1);
                                }
                                true // retain
                            }
                            Err(e) => {
                                status.modify(vars, move |mut s| {
                                    s.pending -= count;
                                    s.last_error = Some(Rc::new(e))
                                });
                                false // task finished
                            }
                        }
                    }));
                }
                Err(e) => tracing::error!("failed to serialize config for key `{:?}`, error: {:?}", key, e),
            }
        }
    }

    /// Gets a variable that updates every time the config associated with `key` changes and updates the config
    /// every time it changes.
    ///
    /// This is equivalent of a two-way binding between the config storage and the variable.
    pub fn var<K, T, D>(&mut self, key: K, default_value: D) -> RcVar<T>
    where
        K: Into<ConfigKey>,
        T: ConfigValue,
        D: FnOnce() -> T,
    {
        self.var_impl(key.into(), default_value)
    }
    fn var_impl<T: ConfigValue>(&mut self, key: ConfigKey, default_value: impl FnOnce() -> T) -> RcVar<T> {
        match self.vars.entry(key) {
            Entry::Occupied(mut entry) => {
                if let Some(var) = entry.get().downcast_ref::<types::WeakRcVar<T>>() {
                    if let Some(var) = var.upgrade() {
                        return var; // already observed and is the same type.
                    }
                }

                // entry stale or for the wrong type:

                // re-insert observer
                let var = var(default_value());
                *entry.get_mut() = Box::new(var.downgrade());

                // and refresh the value.
                let bound_var = var.clone();
                let value = self.read::<T>(entry.key().clone());
                self.tasks.push(Box::new(move |vars, _| {
                    if let Some(rsp) = value.rsp_clone(vars) {
                        if let Some(value) = rsp {
                            bound_var.set_ne(vars, value);
                        }
                        false // task finished
                    } else {
                        true // retain
                    }
                }));

                var
            }
            Entry::Vacant(mut entry) => {
                let var = var(default_value());
                entry.insert(Box::new(var.downgrade()));

                var
            }
        }
    }
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
