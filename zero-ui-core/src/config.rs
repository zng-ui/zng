//!: Config service and sources.

use std::{
    any::Any,
    borrow::Cow,
    collections::{hash_map, HashMap},
    fmt, io, ops,
    path::PathBuf,
    sync::Arc,
};

use atomic::{Atomic, Ordering};
use parking_lot::Mutex;

use crate::{
    app_local, clmv,
    fs_watcher::{WatchFile, WriteFile, WATCHER},
    text::Txt,
    var::*,
};

/// Represents the app main config.
///
/// Config sources must be loaded using [`CONFIG.load`], otherwise the config only leaves for the
/// duration of the app instance.
pub struct CONFIG;
impl CONFIG {
    ///  Replace the config source.
    ///
    /// Variables and bindings survive source replacement, updating to the new value or setting the new source
    /// if the key is not present in the new source.
    pub fn load(&self, source: impl AnyConfig) {
        CONFIG_SV.write().load(source)
    }

    /// Gets a read-only variable that changes to `true` when all key variables
    /// are set to the new source after it finishes loading in another thread.
    pub fn is_loaded(&self) -> BoxedVar<bool> {
        CONFIG_SV.read().is_loaded()
    }

    /// Gets a variable that is bound to the config `key`.
    ///
    /// The same variable is returned for multiple requests of the same key. If the loaded config is not read-only the
    /// returned variable can be set to update the config source.
    ///
    /// The `default` closure is used to generate a value if the key is not found in the config, the default value
    /// is not inserted in the config, the key is inserted or replaced only when the returned variable updates. Note
    /// that the `default` closure may be used even if the key is already in the config, depending on the config implementation.
    pub fn get<T: ConfigValue>(&self, key: impl Into<ConfigKey>, default: impl FnOnce() -> T) -> BoxedVar<T> {
        CONFIG_SV.write().get(key.into(), default)
    }
}
impl AnyConfig for CONFIG {
    fn errors(&self) -> BoxedVar<ConfigErrors> {
        CONFIG_SV.read().errors()
    }

    fn get_json(&mut self, key: ConfigKey, default: serde_json::Value, shared: bool) -> BoxedVar<serde_json::Value> {
        CONFIG_SV.write().get_json(key, default, shared)
    }

    fn contains_key(&self, key: &ConfigKey) -> bool {
        CONFIG_SV.read().contains_key(key)
    }

    fn is_loaded(&self) -> BoxedVar<bool> {
        CONFIG.is_loaded()
    }
}
impl Config for CONFIG {
    fn get<T: ConfigValue>(&mut self, key: impl Into<ConfigKey>, default: impl FnOnce() -> T) -> BoxedVar<T> {
        CONFIG.get(key, default)
    }
}

app_local! {
    static CONFIG_SV: SwapConfig = SwapConfig::new();
}

/// Unique key to a config entry.
pub type ConfigKey = Txt;

/// Marker trait for types that can stored in a [`Config`].
///
/// This trait is already implemented for types it applies.
pub trait ConfigValue: VarValue + serde::Serialize + serde::de::DeserializeOwned {}
impl<T: VarValue + serde::Serialize + serde::de::DeserializeOwned> ConfigValue for T {}

/// Represents a full config map in memory.
///
/// This can be used with [`SyncConfig`] to implement a full config.
pub trait ConfigMap: VarValue {
    /// New empty map.
    fn empty() -> Self;

    /// Read a map from the file.
    ///
    /// This method runs in unblocked context.
    fn read(file: WatchFile) -> io::Result<Self>;
    /// Write the map to a file.
    ///
    /// This method runs in unblocked context.
    fn write(self, file: &mut WriteFile) -> io::Result<()>;

    /// Gets the weak typed value.
    ///
    /// This method is used when `T` cannot be passed because the map is behind a dynamic reference,
    /// if the backend is not JSON it must convert the value from the in memory representation to JSON.
    ///
    /// This method can run in blocking contexts, work with in memory storage only.
    fn get_json(&self, key: &ConfigKey) -> Result<Option<serde_json::Value>, Arc<dyn std::error::Error + Send + Sync>>;

    /// Sets the weak typed value.
    ///
    /// This method is used when `T` cannot be passed because the map is behind a dynamic reference,
    /// if the backend is not JSON it must convert to the in memory representation.
    ///
    /// If `map` is dereferenced mutable a write task will, if possible check if the entry already has the same value
    /// before mutating the map to avoid a potentially expensive IO write.
    ///
    /// This method can run in blocking contexts, work with in memory storage only.
    fn set_json(map: &mut Cow<Self>, key: ConfigKey, value: serde_json::Value) -> Result<(), Arc<dyn std::error::Error + Send + Sync>>;

    /// Returns if the key in config.
    ///
    /// This method can run in blocking contexts, work with in memory storage only.
    fn contains_key(&self, key: &ConfigKey) -> bool;

    /// Get the value if present.
    ///
    /// This method can run in blocking contexts, work with in memory storage only.
    fn get<O: ConfigValue>(&self, key: &ConfigKey) -> Result<Option<O>, Arc<dyn std::error::Error + Send + Sync>> {
        if let Some(value) = self.get_json(key)? {
            match serde_json::from_value(value) {
                Ok(s) => Ok(Some(s)),
                Err(e) => Err(Arc::new(e)),
            }
        } else {
            Ok(None)
        }
    }

    /// Set the value, if you avoid calling [`Cow::to_mut`] the map is not written.
    ///
    /// If `map` is dereferenced mutable a write task will, if possible check if the entry already has the same value
    /// before mutating the map to avoid a potentially expensive IO write.
    ///
    /// This method can run in blocking contexts, work with in memory storage only.
    fn set<O: ConfigValue>(map: &mut Cow<Self>, key: ConfigKey, value: O) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
        match serde_json::to_value(value) {
            Ok(s) => Self::set_json(map, key, s),
            Err(e) => Err(Arc::new(e)),
        }
    }
}

/// Represents one or more config sources behind a dynamic reference.
///
/// See [`Config`] for the full trait.
pub trait AnyConfig: Send + Any {
    /// Gets a read-only variable that changes to `true` when all key variables
    /// are set to the new source after it finishes loading in another thread.
    fn is_loaded(&self) -> BoxedVar<bool>;

    /// All active errors.
    ///
    /// Errors are cleared when the same operation that causes then succeeds.
    ///
    /// The returned variable is read/write unless the config is read-only.
    fn errors(&self) -> BoxedVar<ConfigErrors>;

    /// Errors filtered to only read/write errors.
    fn io_errors(&self) -> BoxedVar<ConfigErrors> {
        self.errors().map(|e| ConfigErrors(e.io().cloned().collect())).boxed()
    }

    /// Errors filtered to only get/set for the `key`.
    fn entry_errors(&self, key: ConfigKey) -> BoxedVar<ConfigErrors> {
        self.errors().map(move |e| ConfigErrors(e.entry(&key).cloned().collect())).boxed()
    }

    /// Gets a weak typed variable to the config `key`.
    ///
    /// This method is used when `T` cannot be passed because the config is behind a dynamic reference,
    /// if the backend is not JSON it must convert the value from the in memory representation to JSON.
    ///
    /// If `shared` is `true` and the key was already requested it the same var is returned, if `false`
    /// a new variable is always generated. Note that if you have two different variables for the same
    /// key they will go out-of-sync as updates from setting one variable do not propagate to the other.
    ///
    /// The `default` value is used to if the key is not found in the config, the default value
    /// is not inserted in the config, the key is inserted or replaced only when the returned variable updates.
    fn get_json(&mut self, key: ConfigKey, default: serde_json::Value, shared: bool) -> BoxedVar<serde_json::Value>;

    /// Returns if the `key` already has the key in the backing storage.
    ///
    /// Both [`get_json`] and [`get`] methods don't insert the key on request, the key is inserted on the
    /// first time the returned variable updates.
    ///
    /// [`get_json`]: AnyConfig::get_json
    /// [`get`]: Config::get
    fn contains_key(&self, key: &ConfigKey) -> bool;
}

/// Represents one or more config sources.
pub trait Config: AnyConfig {
    /// Gets a variable that is bound to the config `key`.
    ///
    /// The same variable is returned for multiple requests of the same key. If the loaded config is not read-only the
    /// returned variable can be set to update the config source.
    ///
    /// The `default` closure is used to generate a value if the key is not found in the config, the default value
    /// is not inserted in the config, the key is inserted or replaced only when the returned variable updates. Note
    /// that the `default` closure may be used even if the key is already in the config, depending on the config implementation.
    fn get<T: ConfigValue>(&mut self, key: impl Into<ConfigKey>, default: impl FnOnce() -> T) -> BoxedVar<T> {
        let key = key.into();
        let default = default();
        let json_var = self.get_json(key.clone(), serde_json::to_value(&default).unwrap_or(serde_json::Value::Null), true);
        let wk_errors = self.errors().downgrade();

        json_var
            .filter_map_bidi(
                // JSON -> T
                clmv!(key, wk_errors, |json| match serde_json::from_value(json.clone()) {
                    Ok(typed) => {
                        if let Some(errors) = wk_errors.upgrade() {
                            if errors.with(|e| e.entry(&key).next().is_some()) {
                                let _ = errors.modify(clmv!(key, |e| e.to_mut().clear_entry(&key)));
                            }
                        }
                        Some(typed)
                    }
                    Err(e) => {
                        tracing::error!("get config get({key:?}) error, {e:?}");
                        if let Some(errors) = wk_errors.upgrade() {
                            let _ = errors.modify(clmv!(key, |es| es.to_mut().push(ConfigError::new_get(key, e))));
                        }
                        None
                    }
                }),
                // T -> JSON
                clmv!(key, wk_errors, |typed| {
                    match serde_json::to_value(typed) {
                        Ok(json) => {
                            if let Some(errors) = wk_errors.upgrade() {
                                if errors.with(|e| e.entry(&key).next().is_some()) {
                                    let _ = errors.modify(clmv!(key, |e| e.to_mut().clear_entry(&key)));
                                }
                            }
                            Some(json)
                        }
                        Err(e) => {
                            tracing::error!("get config set({key:?}) error, {e:?}");
                            if let Some(errors) = wk_errors.upgrade() {
                                let _ = errors.modify(clmv!(key, |es| es.to_mut().push(ConfigError::new_set(key, e))));
                            }
                            None
                        }
                    }
                }),
                move || default.clone(),
            )
            .boxed()
    }
}

impl ConfigMap for HashMap<ConfigKey, serde_json::Value> {
    fn empty() -> Self {
        Self::new()
    }

    fn read(mut file: WatchFile) -> io::Result<Self> {
        file.json().map_err(Into::into)
    }

    fn write(self, file: &mut WriteFile) -> io::Result<()> {
        file.write_json(&self, true).map_err(Into::into)
    }

    fn get_json(&self, key: &ConfigKey) -> Result<Option<serde_json::Value>, Arc<dyn std::error::Error + Send + Sync>> {
        Ok(self.get(key).cloned())
    }

    fn set_json(map: &mut Cow<Self>, key: ConfigKey, value: serde_json::Value) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
        if map.get(&key) != Some(&value) {
            map.to_mut().insert(key, value);
        }
        Ok(())
    }

    fn contains_key(&self, key: &ConfigKey) -> bool {
        self.contains_key(key)
    }

    fn get<O: ConfigValue>(&self, key: &ConfigKey) -> Result<Option<O>, Arc<dyn std::error::Error + Send + Sync>> {
        if let Some(value) = self.get_json(key)? {
            match serde_json::from_value(value) {
                Ok(s) => Ok(Some(s)),
                Err(e) => Err(Arc::new(e)),
            }
        } else {
            Ok(None)
        }
    }

    fn set<O: ConfigValue>(map: &mut Cow<Self>, key: ConfigKey, value: O) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
        match serde_json::to_value(value) {
            Ok(s) => Self::set_json(map, key, s),
            Err(e) => Err(Arc::new(e)),
        }
    }
}

/// Config source that auto syncs with file.
///
/// The [`WATCHER.sync`] is used to synchronize with the file, this type implements the binding
/// for each key.
///
/// [`WATCHER.sync`]: WATCHER::sync
pub struct SyncConfig<M: ConfigMap> {
    sync_var: ArcVar<M>,
    is_loaded: ArcVar<bool>,
    errors: ArcVar<ConfigErrors>,
    shared: ConfigVars,
}
impl<M: ConfigMap> SyncConfig<M> {
    /// Open write the `file`
    pub fn sync(file: impl Into<PathBuf>) -> Self {
        let is_loaded = var(false);
        let errors = var(ConfigErrors::default());
        let sync_var = WATCHER.sync(
            file,
            M::empty(),
            clmv!(is_loaded, errors, |r| {
                match (|| M::read(r?))() {
                    Ok(ok) => {
                        if errors.with(|e| e.io().next().is_some()) {
                            errors.modify(|e| e.to_mut().clear_io());
                        }
                        if !is_loaded.get() {
                            is_loaded.set(true);
                        }
                        Some(ok)
                    }
                    Err(e) => {
                        if is_loaded.get() {
                            is_loaded.set(false);
                        }
                        tracing::error!("sync config read error, {e:?}");
                        errors.modify(|es| es.to_mut().push(ConfigError::new_read(e)));
                        None
                    }
                }
            }),
            clmv!(is_loaded, errors, |map, w| {
                match (|| {
                    let mut w = w?;
                    map.write(&mut w)?;
                    w.commit()
                })() {
                    Ok(()) => {
                        if errors.with(|e| e.io().next().is_some()) {
                            errors.modify(|e| e.to_mut().clear_io());
                        }
                        if !is_loaded.get() {
                            is_loaded.set(true);
                        }
                    }
                    Err(e) => {
                        if is_loaded.get() {
                            is_loaded.set(false);
                        }
                        tracing::error!("sync config write error, {e:?}");
                        errors.modify(|es| es.to_mut().push(ConfigError::new_write(e)));
                    }
                }
            }),
        );

        Self {
            sync_var,
            errors,
            is_loaded,
            shared: ConfigVars::default(),
        }
    }

    fn get_new_json(
        sync_var: &ArcVar<M>,
        errors: &ArcVar<ConfigErrors>,
        key: ConfigKey,
        default: serde_json::Value,
    ) -> BoxedVar<serde_json::Value> {
        // init var to already present value, or default.
        let var = match sync_var.with(|m| ConfigMap::get_json(m, &key)) {
            Ok(json) => {
                // get ok, clear any entry errors
                if errors.with(|e| e.entry(&key).next().is_some()) {
                    errors.modify(clmv!(key, |e| e.to_mut().clear_entry(&key)));
                }

                match json {
                    Some(json) => var(json),
                    None => var(default),
                }
            }
            Err(e) => {
                // get error
                tracing::error!("sync config get({key:?}) error, {e:?}");
                errors.modify(clmv!(key, |es| es.to_mut().push(ConfigError::new_get(key, e))));
                var(default)
            }
        };

        // bind entry var

        // config -> entry
        let wk_var = var.downgrade();
        let last_update = Atomic::new(VarUpdateId::never());
        sync_var
            .hook(Box::new(clmv!(errors, key, |map| {
                let update_id = VARS.update_id();
                if update_id == last_update.load(Ordering::Relaxed) {
                    return true;
                }
                last_update.store(update_id, Ordering::Relaxed);
                if let Some(var) = wk_var.upgrade() {
                    match map.as_any().downcast_ref::<M>().unwrap().get_json(&key) {
                        Ok(json) => {
                            // get ok
                            if errors.with(|e| e.entry(&key).next().is_some()) {
                                errors.modify(clmv!(key, |e| e.to_mut().clear_entry(&key)));
                            }

                            if let Some(json) = json {
                                var.set(json);
                            }
                            // else backend lost entry but did not report as error.
                        }
                        Err(e) => {
                            // get error
                            tracing::error!("sync config get({key:?}) error, {e:?}");
                            errors.modify(clmv!(key, |es| es.to_mut().push(ConfigError::new_get(key, e))));
                        }
                    }
                    // retain hook
                    true
                } else {
                    // entry var dropped, drop hook
                    false
                }
            })))
            .perm();

        // entry -> config
        let wk_sync_var = sync_var.downgrade();
        let last_update = Atomic::new(VarUpdateId::never());
        var.hook(Box::new(clmv!(errors, |value| {
            let update_id = VARS.update_id();
            if update_id == last_update.load(Ordering::Relaxed) {
                return true;
            }
            last_update.store(update_id, Ordering::Relaxed);
            if let Some(sync_var) = wk_sync_var.upgrade() {
                let json = value.as_any().downcast_ref::<serde_json::Value>().unwrap().clone();
                sync_var.modify(clmv!(key, errors, |m| {
                    // set, only if actually changed
                    match ConfigMap::set_json(m, key.clone(), json) {
                        Ok(()) => {
                            // set ok
                            if errors.with(|e| e.entry(&key).next().is_some()) {
                                errors.modify(move |e| e.to_mut().clear_entry(&key));
                            }
                        }
                        Err(e) => {
                            // set error
                            tracing::error!("sync config set({key:?}) error, {e:?}");
                            errors.modify(|es| es.to_mut().push(ConfigError::new_set(key, e)));
                        }
                    }
                }));

                // retain hook
                true
            } else {
                // config dropped, drop hook
                false
            }
        })))
        .perm();

        var.boxed()
    }

    fn get_new<T: ConfigValue>(
        sync_var: &ArcVar<M>,
        errors: &ArcVar<ConfigErrors>,
        key: impl Into<ConfigKey>,
        default: impl FnOnce() -> T,
    ) -> BoxedVar<T> {
        // init var to already present value, or default.
        let key = key.into();
        let var = match sync_var.with(|m| ConfigMap::get::<T>(m, &key)) {
            Ok(value) => {
                if errors.with(|e| e.entry(&key).next().is_some()) {
                    errors.modify(clmv!(key, |e| e.to_mut().clear_entry(&key)));
                }
                match value {
                    Some(val) => var(val),
                    None => var(default()),
                }
            }
            Err(e) => {
                tracing::error!("sync config get({key:?}) error, {e:?}");
                errors.modify(clmv!(key, |es| es.to_mut().push(ConfigError::new_get(key, e))));
                var(default())
            }
        };

        // bind entry var

        // config -> entry
        let wk_var = var.downgrade();
        sync_var
            .hook(Box::new(clmv!(errors, key, |map| {
                if let Some(var) = wk_var.upgrade() {
                    match map.as_any().downcast_ref::<M>().unwrap().get::<T>(&key) {
                        Ok(value) => {
                            if errors.with(|e| e.entry(&key).next().is_some()) {
                                errors.modify(clmv!(key, |e| e.to_mut().clear_entry(&key)));
                            }

                            if let Some(value) = value {
                                var.set(value);
                            }
                        }
                        Err(e) => {
                            tracing::error!("sync config get({key:?}) error, {e:?}");
                            errors.modify(clmv!(key, |es| es.to_mut().push(ConfigError::new_get(key, e))));
                        }
                    }
                    true
                } else {
                    false
                }
            })))
            .perm();

        // entry -> config
        let wk_sync_var = sync_var.downgrade();
        var.hook(Box::new(clmv!(errors, |value| {
            if let Some(sync_var) = wk_sync_var.upgrade() {
                let value = value.as_any().downcast_ref::<T>().unwrap().clone();
                sync_var.modify(clmv!(key, errors, |m| {
                    match ConfigMap::set(m, key.clone(), value) {
                        Ok(()) => {
                            if errors.with(|e| e.entry(&key).next().is_some()) {
                                errors.modify(clmv!(key, |e| e.to_mut().clear_entry(&key)));
                            }
                        }
                        Err(e) => {
                            tracing::error!("sync config set({key:?}) error, {e:?}");
                            errors.modify(clmv!(key, |es| es.to_mut().push(ConfigError::new_set(key, e))));
                        }
                    }
                }));
                true
            } else {
                false
            }
        })))
        .perm();

        var.boxed()
    }
}
impl<M: ConfigMap> AnyConfig for SyncConfig<M> {
    fn errors(&self) -> BoxedVar<ConfigErrors> {
        self.errors.clone().boxed()
    }

    fn get_json(&mut self, key: ConfigKey, default: serde_json::Value, shared: bool) -> BoxedVar<serde_json::Value> {
        if shared {
            self.shared
                .get_or_bind(key, |key| Self::get_new_json(&self.sync_var, &self.errors, key.clone(), default))
        } else {
            Self::get_new_json(&self.sync_var, &self.errors, key, default)
        }
    }

    fn contains_key(&self, key: &ConfigKey) -> bool {
        self.sync_var.with(|q| q.contains_key(key))
    }

    fn is_loaded(&self) -> BoxedVar<bool> {
        self.is_loaded.clone().boxed()
    }
}
impl<M: ConfigMap> Config for SyncConfig<M> {
    fn get<T: ConfigValue>(&mut self, key: impl Into<ConfigKey>, default: impl FnOnce() -> T) -> BoxedVar<T> {
        self.shared
            .get_or_bind(key.into(), |key| Self::get_new(&self.sync_var, &self.errors, key.clone(), default))
    }
}

/// Represents a config source that synchronizes with a JSON file.
pub type JsonConfig = SyncConfig<HashMap<ConfigKey, serde_json::Value>>;

/// Config wrapper that only updates variables from config source changes.
pub struct ReadOnlyConfig<C: Config> {
    cfg: C,
}
impl<C: Config> ReadOnlyConfig<C> {
    /// New reading from `cfg`.
    pub fn new(cfg: C) -> Self {
        Self { cfg }
    }
}
impl<C: Config> AnyConfig for ReadOnlyConfig<C> {
    fn errors(&self) -> BoxedVar<ConfigErrors> {
        self.cfg.errors().read_only()
    }

    fn get_json(&mut self, key: ConfigKey, default: serde_json::Value, shared: bool) -> BoxedVar<serde_json::Value> {
        self.cfg.get_json(key, default, shared).read_only()
    }

    fn contains_key(&self, key: &ConfigKey) -> bool {
        self.cfg.contains_key(key)
    }

    fn is_loaded(&self) -> BoxedVar<bool> {
        self.cfg.is_loaded()
    }
}
impl<C: Config> Config for ReadOnlyConfig<C> {
    fn get<T: ConfigValue>(&mut self, key: impl Into<ConfigKey>, default: impl FnOnce() -> T) -> BoxedVar<T> {
        self.cfg.get(key.into(), default).read_only()
    }
}

/// Represents a config source that is read and written too, when a key is not present in the source
/// the fallback variable is used, but if that variable is modified the key is inserted in the primary config.
pub struct FallbackConfig<S: Config, F: Config> {
    fallback: F,
    over: S,
}
impl<S: Config, F: Config> FallbackConfig<S, F> {
    /// New config.
    pub fn new(fallback: F, over: S) -> Self {
        Self { fallback, over }
    }
}
impl<S: Config, F: Config> AnyConfig for FallbackConfig<S, F> {
    fn is_loaded(&self) -> BoxedVar<bool> {
        self.fallback.is_loaded()
    }

    fn errors(&self) -> BoxedVar<ConfigErrors> {
        merge_var!(self.fallback.errors(), self.over.errors(), |a, b| {
            if a.is_empty() {
                return b.clone();
            }
            if b.is_empty() {
                return a.clone();
            }
            let mut r = a.clone();
            for b in b.iter() {
                r.push(b.clone());
            }
            r
        })
        .boxed()
    }

    fn get_json(&mut self, key: ConfigKey, default: serde_json::Value, shared: bool) -> BoxedVar<serde_json::Value> {
        let over = self.over.get_json(key.clone(), default.clone(), shared);
        if self.over.contains_key(&key) {
            return over;
        }

        let fallback = self.fallback.get_json(key, default, shared);
        let result = var(fallback.get());

        #[derive(Clone, Copy)]
        enum State {
            Fallback,
            FallbackUpdated,
            Over,
            OverUpdated,
        }
        let state = Arc::new(atomic::Atomic::new(State::Fallback));

        // hook fallback, signal `result` that an update is flowing from the fallback.
        let wk_result = result.downgrade();
        fallback
            .hook(Box::new(clmv!(state, |value| {
                match state.load(atomic::Ordering::Relaxed) {
                    State::Over | State::OverUpdated => {
                        // result -> over
                        return false;
                    }
                    _ => {}
                }

                // fallback -> result
                if let Some(result) = wk_result.upgrade() {
                    state.store(State::FallbackUpdated, atomic::Ordering::Relaxed);
                    result.set(value.as_any().downcast_ref::<serde_json::Value>().unwrap().clone());
                    true
                } else {
                    // weak-ref to avoid circular ref.
                    false
                }
            })))
            .perm();

        // hook over, signals `result` that an update is flowing from the override.
        let wk_result = result.downgrade();
        over.hook(Box::new(clmv!(state, |value| {
            match state.load(atomic::Ordering::Relaxed) {
                State::OverUpdated => {
                    // result -> over
                    state.store(State::Over, atomic::Ordering::Relaxed);
                }
                _ => {
                    // over -> result
                    let value = value.as_any().downcast_ref::<serde_json::Value>().unwrap();
                    state.store(State::OverUpdated, atomic::Ordering::Relaxed);
                    if let Some(result) = wk_result.upgrade() {
                        result.set(value.clone());
                    } else {
                        // weak-ref to avoid circular ref.
                        return false;
                    }
                }
            }

            true
        })))
        .perm();

        // hook result, on first callback not caused by `fallback` drops it and changes to `over`.
        let fallback = Mutex::new(Some(fallback));
        result
            .hook(Box::new(move |value| {
                match state.load(atomic::Ordering::Relaxed) {
                    State::Fallback => {
                        // result -> over(first)
                        state.store(State::Over, atomic::Ordering::Relaxed);
                        *fallback.lock() = None;
                        let value = value.as_any().downcast_ref::<serde_json::Value>().unwrap().clone();
                        let _ = over.set_ne(value);
                    }
                    State::FallbackUpdated => {
                        // fallback -> result
                        state.store(State::Fallback, atomic::Ordering::Relaxed);
                    }
                    State::Over => {
                        // result -> over
                        let value = value.as_any().downcast_ref::<serde_json::Value>().unwrap().clone();
                        let _ = over.set_ne(value);
                    }
                    State::OverUpdated => {
                        // over -> result
                        state.store(State::Over, atomic::Ordering::Relaxed);
                    }
                }
                true
            }))
            .perm();

        result.boxed()
    }

    fn contains_key(&self, key: &ConfigKey) -> bool {
        self.fallback.contains_key(key) || self.over.contains_key(key)
    }
}
impl<S: Config, F: Config> Config for FallbackConfig<S, F> {
    fn get<T: ConfigValue>(&mut self, key: impl Into<ConfigKey>, default: impl FnOnce() -> T) -> BoxedVar<T> {
        let key = key.into();
        let default = default();
        let fallback = self.fallback.get(key.clone(), || default.clone());
        let over = var(None::<T>); // TODO, actually provided by self.source
        if over.with(|s| s.is_some()) {
            return self.over.get(key, move || default);
        }
        let result = var(fallback.get());

        #[derive(Clone, Copy)]
        enum State {
            Fallback,
            FallbackUpdated,
            Over,
            OverUpdated,
        }
        let state = Arc::new(atomic::Atomic::new(State::Fallback));

        // hook fallback, signal `result` that an update is flowing from the fallback.
        let wk_result = result.downgrade();
        fallback
            .hook(Box::new(clmv!(state, |value| {
                match state.load(atomic::Ordering::Relaxed) {
                    State::Over | State::OverUpdated => {
                        // result -> over
                        return false;
                    }
                    _ => {}
                }

                // fallback -> result
                if let Some(result) = wk_result.upgrade() {
                    state.store(State::FallbackUpdated, atomic::Ordering::Relaxed);
                    result.set(value.as_any().downcast_ref::<T>().unwrap().clone());
                    true
                } else {
                    // weak-ref to avoid circular ref.
                    false
                }
            })))
            .perm();

        // hook over, signals `result` that an update is flowing from the override.
        let wk_result = result.downgrade();
        over.hook(Box::new(clmv!(state, |value| {
            match state.load(atomic::Ordering::Relaxed) {
                State::OverUpdated => {
                    // result -> over
                    state.store(State::Over, atomic::Ordering::Relaxed);
                }
                _ => {
                    // over -> result
                    if let Some(value) = value.as_any().downcast_ref::<Option<T>>().unwrap() {
                        state.store(State::OverUpdated, atomic::Ordering::Relaxed);
                        if let Some(result) = wk_result.upgrade() {
                            result.set(value.clone());
                        } else {
                            // weak-ref to avoid circular ref.
                            return false;
                        }
                    }
                }
            }

            true
        })))
        .perm();

        // hook result, on first callback not caused by `fallback` drops it and changes to `over`.
        let fallback = Mutex::new(Some(fallback));
        result
            .hook(Box::new(move |value| {
                match state.load(atomic::Ordering::Relaxed) {
                    State::Fallback => {
                        // result -> over(first)
                        state.store(State::Over, atomic::Ordering::Relaxed);
                        *fallback.lock() = None;
                        over.set(Some(value.as_any().downcast_ref::<T>().unwrap().clone()));
                    }
                    State::FallbackUpdated => {
                        // fallback -> result
                        state.store(State::Fallback, atomic::Ordering::Relaxed);
                    }
                    State::Over => {
                        // result -> over
                        over.set(Some(value.as_any().downcast_ref::<T>().unwrap().clone()));
                    }
                    State::OverUpdated => {
                        // over -> result
                        state.store(State::Over, atomic::Ordering::Relaxed);
                    }
                }
                true
            }))
            .perm();

        result.boxed()
    }
}

/// Config without any backing store.
pub struct NilConfig;

impl AnyConfig for NilConfig {
    fn errors(&self) -> BoxedVar<ConfigErrors> {
        LocalVar(ConfigErrors::default()).boxed()
    }
    fn get_json(&mut self, _: ConfigKey, default: serde_json::Value, _: bool) -> BoxedVar<serde_json::Value> {
        LocalVar(default).boxed()
    }
    fn contains_key(&self, _: &ConfigKey) -> bool {
        false
    }

    fn is_loaded(&self) -> BoxedVar<bool> {
        LocalVar(true).boxed()
    }
}
impl Config for NilConfig {
    fn get<T: ConfigValue>(&mut self, _: impl Into<ConfigKey>, default: impl FnOnce() -> T) -> BoxedVar<T> {
        LocalVar(default()).boxed()
    }
}

/// Represents a config source that swap its backend without disconnecting any bound keys.
///
/// Note that the [`CONFIG`] service already uses this type internally.
pub struct SwapConfig {
    cfg: Mutex<Box<dyn AnyConfig>>,
    shared: ConfigVars,

    is_loaded: ArcVar<bool>,
    is_loaded_binding: VarHandle,
    errors: ArcVar<ConfigErrors>,
    errors_binding: VarHandle,
}

impl AnyConfig for SwapConfig {
    fn errors(&self) -> BoxedVar<ConfigErrors> {
        self.errors.clone().boxed()
    }

    fn get_json(&mut self, key: ConfigKey, default: serde_json::Value, shared: bool) -> BoxedVar<serde_json::Value> {
        if shared {
            self.shared
                .get_or_bind(key, |key| self.cfg.get_mut().get_json(key.clone(), default, false))
        } else {
            self.cfg.get_mut().get_json(key, default, false)
        }
    }

    fn contains_key(&self, key: &ConfigKey) -> bool {
        self.cfg.lock().contains_key(key)
    }

    fn is_loaded(&self) -> BoxedVar<bool> {
        self.is_loaded.clone().boxed()
    }
}
impl Config for SwapConfig {
    fn get<T: ConfigValue>(&mut self, key: impl Into<ConfigKey>, default: impl FnOnce() -> T) -> BoxedVar<T> {
        self.shared.get_or_bind(key.into(), |key| {
            // not in shared, bind with source json var.

            let default = default();
            let source_var = self.cfg.get_mut().get_json(
                key.clone(),
                serde_json::to_value(&default).unwrap_or(serde_json::Value::Null),
                false,
            );
            let var = var(serde_json::from_value(source_var.get()).unwrap_or(default));

            let errors = &self.errors;

            source_var
                .bind_filter_map_bidi(
                    &var,
                    // JSON -> T
                    clmv!(key, errors, |json| {
                        match serde_json::from_value(json.clone()) {
                            Ok(value) => {
                                if errors.with(|e| e.entry(&key).next().is_some()) {
                                    errors.modify(clmv!(key, |e| e.to_mut().clear_entry(&key)));
                                }
                                Some(value)
                            }
                            Err(e) => {
                                tracing::error!("swap config get({key:?}) error, {e:?}");
                                errors.modify(clmv!(key, |es| es.to_mut().push(ConfigError::new_get(key, e))));
                                None
                            }
                        }
                    }),
                    // T -> JSON
                    clmv!(key, errors, source_var, |value| {
                        let _strong_ref = &source_var;

                        match serde_json::to_value(value) {
                            Ok(json) => {
                                if errors.with(|e| e.entry(&key).next().is_some()) {
                                    errors.modify(clmv!(key, |e| e.to_mut().clear_entry(&key)));
                                }
                                Some(json)
                            }
                            Err(e) => {
                                tracing::error!("swap config set({key:?}) error, {e:?}");
                                errors.modify(clmv!(key, |es| es.to_mut().push(ConfigError::new_set(key, e))));
                                None
                            }
                        }
                    }),
                )
                .perm();

            var.boxed()
        })
    }
}
impl SwapConfig {
    /// New with [`NilConfig`] backend.
    pub fn new() -> Self {
        Self {
            cfg: Mutex::new(Box::new(NilConfig)),
            errors: var(ConfigErrors::default()),
            shared: ConfigVars::default(),
            is_loaded: var(false),
            is_loaded_binding: VarHandle::dummy(),
            errors_binding: VarHandle::dummy(),
        }
    }

    /// Load the config.
    pub fn load(&mut self, cfg: impl AnyConfig) {
        self.replace_source(Box::new(cfg))
    }

    fn replace_source(&mut self, mut source: Box<dyn AnyConfig>) {
        let source_errors = source.errors();
        self.errors.set(source_errors.get());
        self.errors_binding = source_errors.bind(&self.errors);

        let source_is_loaded = source.is_loaded();
        self.is_loaded.set(source_is_loaded.get());
        self.is_loaded_binding = source_is_loaded.bind(&self.is_loaded);

        self.shared.rebind(&self.errors, &mut *source);

        *self.cfg.get_mut() = source;
    }
}
impl Default for SwapConfig {
    fn default() -> Self {
        Self::new()
    }
}
struct ConfigVar<T: ConfigValue> {
    var: BoxedWeakVar<T>,
    binding: VarHandles,
}
impl<T: ConfigValue> ConfigVar<T> {
    fn new_any(var: BoxedWeakVar<T>) -> Box<dyn AnyConfigVar> {
        Box::new(Self {
            var,
            binding: VarHandles::dummy(),
        })
    }
}

/// Map of configs already bound to a variable.
///
/// The map does only holds a weak reference to the variables.
#[derive(Default)]
pub struct ConfigVars(HashMap<ConfigKey, Box<dyn AnyConfigVar>>);
impl ConfigVars {
    /// Gets the already bound variable or calls `bind` to generate a new binding.
    pub fn get_or_bind<T: ConfigValue>(&mut self, key: ConfigKey, bind: impl FnOnce(&ConfigKey) -> BoxedVar<T>) -> BoxedVar<T> {
        match self.0.entry(key) {
            hash_map::Entry::Occupied(mut e) => {
                if e.get().can_upgrade() {
                    if let Some(x) = e.get().as_any().downcast_ref::<ConfigVar<T>>() {
                        if let Some(var) = x.var.upgrade() {
                            return var;
                        }
                    } else {
                        tracing::error!(
                            "cannot get key `{}` as `{}` because it is already requested with a different type",
                            e.key(),
                            std::any::type_name::<T>()
                        );
                        return bind(e.key());
                    }
                }
                // cannot upgrade
                let var = bind(e.key());
                e.insert(ConfigVar::new_any(var.downgrade()));
                var
            }
            hash_map::Entry::Vacant(e) => {
                let var = bind(e.key());
                e.insert(ConfigVar::new_any(var.downgrade()));
                var
            }
        }
    }

    /// Bind all variables to the new `source`.
    ///
    /// If the map entry is present in the `source` the variable is updated to the new value, if not the entry
    /// is inserted in the source. The variable is then bound to the source.
    pub fn rebind(&mut self, errors: &ArcVar<ConfigErrors>, source: &mut dyn AnyConfig) {
        self.0.retain(|key, wk_var| wk_var.rebind(errors, key, source));
    }
}
trait AnyConfigVar: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn can_upgrade(&self) -> bool;
    fn rebind(&mut self, errors: &ArcVar<ConfigErrors>, key: &ConfigKey, source: &mut dyn AnyConfig) -> bool;
}
impl<T: ConfigValue> AnyConfigVar for ConfigVar<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn can_upgrade(&self) -> bool {
        self.var.strong_count() > 0
    }

    fn rebind(&mut self, errors: &ArcVar<ConfigErrors>, key: &ConfigKey, source: &mut dyn AnyConfig) -> bool {
        let var = if let Some(var) = self.var.upgrade() {
            var
        } else {
            return false;
        };

        let source_var = source.get_json(
            key.clone(),
            serde_json::to_value(var.get()).unwrap_or(serde_json::Value::Null),
            false,
        );

        match serde_json::from_value::<T>(source_var.get()) {
            Ok(value) => {
                let _ = var.set(value);
            }
            Err(e) => {
                tracing::error!("rebind config get({key:?}) error, {e:?}");
                errors.modify(clmv!(key, |es| es.to_mut().push(ConfigError::new_get(key, e))));
            }
        }

        self.binding = source_var.bind_filter_map_bidi(
            &var,
            // JSON -> T
            clmv!(key, errors, |json| {
                match serde_json::from_value(json.clone()) {
                    Ok(value) => {
                        if errors.with(|e| e.entry(&key).next().is_some()) {
                            errors.modify(clmv!(key, |e| e.to_mut().clear_entry(&key)));
                        }
                        Some(value)
                    }
                    Err(e) => {
                        tracing::error!("rebind config get({key:?}) error, {e:?}");
                        errors.modify(clmv!(key, |es| es.to_mut().push(ConfigError::new_get(key, e))));
                        None
                    }
                }
            }),
            // T -> JSON
            clmv!(key, errors, source_var, |value| {
                let _strong_ref = &source_var;
                match serde_json::to_value(value) {
                    Ok(json) => {
                        if errors.with(|e| e.entry(&key).next().is_some()) {
                            errors.modify(clmv!(key, |e| e.to_mut().clear_entry(&key)));
                        }
                        Some(json)
                    }
                    Err(e) => {
                        tracing::error!("rebind config set({key:?}) error, {e:?}");
                        errors.modify(clmv!(key, |es| es.to_mut().push(ConfigError::new_set(key, e))));
                        None
                    }
                }
            }),
        );

        true
    }
}

/// Config error.
#[derive(Clone, Debug)]
pub enum ConfigError {
    /// Error reading from the external storage to memory.
    Read(Arc<io::Error>),
    /// Error writing from memory to the external storage.
    Write(Arc<io::Error>),

    /// Error converting a key from memory to the final type.
    Get {
        /// Key.
        key: ConfigKey,
        /// Error.
        err: Arc<dyn std::error::Error + Send + Sync>,
    },
    /// Error converting from the final type to the memory format.
    Set {
        /// Key.
        key: ConfigKey,
        /// Error.
        err: Arc<dyn std::error::Error + Send + Sync>,
    },
}
#[cfg(test)]
fn _assert_var_value(cfg: ConfigError) -> impl VarValue {
    cfg
}
impl ConfigError {
    /// Reference the read or write error.
    pub fn io(&self) -> Option<&Arc<io::Error>> {
        match self {
            Self::Read(e) | Self::Write(e) => Some(e),
            _ => None,
        }
    }

    /// New read error.
    pub fn new_read(e: impl Into<io::Error>) -> Self {
        Self::Read(Arc::new(e.into()))
    }

    /// New write error.
    pub fn new_write(e: impl Into<io::Error>) -> Self {
        Self::Write(Arc::new(e.into()))
    }

    /// New get error.
    pub fn new_get(key: impl Into<ConfigKey>, e: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Get {
            key: key.into(),
            err: Arc::new(e),
        }
    }
    /// New set error.
    pub fn new_set(key: impl Into<ConfigKey>, e: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Set {
            key: key.into(),
            err: Arc::new(e),
        }
    }
}
impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Read(e) => write!(f, "config read error, {e}"),
            ConfigError::Write(e) => write!(f, "config write error, {e}"),
            ConfigError::Get { key, err } => write!(f, "config `{key}` get error, {err}"),
            ConfigError::Set { key, err } => write!(f, "config `{key}` set error, {err}"),
        }
    }
}
impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::Read(e) | ConfigError::Write(e) => Some(e),
            ConfigError::Get { err, .. } | ConfigError::Set { err, .. } => Some(err),
        }
    }
}

/// List of active errors in a config source.
#[derive(Debug, Clone, Default)]
pub struct ConfigErrors(pub Vec<ConfigError>);
impl ops::Deref for ConfigErrors {
    type Target = Vec<ConfigError>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl ops::DerefMut for ConfigErrors {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl ConfigErrors {
    /// Iterate over read and write errors.
    pub fn io(&self) -> impl Iterator<Item = &ConfigError> {
        self.iter().filter(|e| matches!(e, ConfigError::Read(_) | ConfigError::Write(_)))
    }

    /// Remove read and write errors.
    pub fn clear_io(&mut self) {
        self.retain(|e| !matches!(e, ConfigError::Read(_) | ConfigError::Write(_)));
    }

    /// Iterate over get and set errors for the key.
    pub fn entry<'a>(&'a self, key: &'a ConfigKey) -> impl Iterator<Item = &'a ConfigError> + 'a {
        let entry_key = key;
        self.iter()
            .filter(move |e| matches!(e, ConfigError::Get { key, .. } | ConfigError::Set { key, .. } if key == entry_key))
    }

    /// Remove get and set errors for the key.
    pub fn clear_entry(&mut self, key: &ConfigKey) {
        let entry_key = key;
        self.retain(move |e| !matches!(e, ConfigError::Get { key, .. } | ConfigError::Set { key, .. } if key == entry_key))
    }
}
impl fmt::Display for ConfigErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut prefix = "";
        for e in self.iter() {
            write!(f, "{prefix}{e}")?;
            prefix = "\n";
        }
        Ok(())
    }
}
