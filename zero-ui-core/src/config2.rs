use std::{any::Any, borrow::Cow, collections::HashMap, fmt, io, ops, path::PathBuf, sync::Arc};

use crate::{
    clmv,
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
        todo!("!!:")
    }

    /// All active errors.
    ///
    /// Errors are cleared when the same operation that causes then succeeds.
    ///
    /// The returned variable is read/write unless the config is read-only.
    pub fn errors(&self) -> BoxedVar<ConfigErrors> {
        todo!("!!:")
    }

    /// Binds the `var` to the `key` on this config.
    ///
    /// When the `var` updates the `key` is set, when the stored `key` updates the variable is updated.
    ///
    /// If the `key` is not present in the config it is set on init, if the `key` is present the `var` is set on init.
    pub fn bind<T: ConfigValue>(&self, key: impl Into<ConfigKey>, var: impl Var<T>) -> VarHandles {
        todo!("!!:")
    }

    /// Gets a variable that is bound to the config `key`.
    ///
    /// This is equivalent to using [`bind`] on a new variable, except `default` will be called only if the `key`
    /// is not present in the config on init.
    ///
    /// If the config is
    ///
    /// [`bind`]: Self::bind
    pub fn var<T: ConfigValue>(&self, key: impl Into<ConfigKey>, default: impl Fn() -> T) -> BoxedVar<T> {
        todo!("!!:")
    }
}
// !!: TODO, implement Config for CONFIG

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

    /// Binds the weak typed `var` to the `key` on this config.
    ///
    /// The binding is dropped if the returned handles are dropped.
    ///
    /// This method is used when `T` cannot be passed because the map is behind a dynamic reference,
    /// if the backend is not JSON it must convert the value from the in memory representation to JSON.
    fn bind_json(&self, key: ConfigKey, var: BoxedVar<serde_json::Value>) -> VarHandles;

    /// Gets a weak typed variable to the config `key`.
    ///
    /// This method is used when `T` cannot be passed because the map is behind a dynamic reference,
    /// if the backend is not JSON it must convert the value from the in memory representation to JSON.
    fn var_json(&self, key: ConfigKey, default: &dyn Fn() -> serde_json::Value) -> BoxedVar<serde_json::Value>;
}

/// Represents one or more config sources.
pub trait Config: AnyConfig {
    /// Binds the `var` to the `key` on this config.
    ///
    /// When the `var` updates the `key` is set, when the stored `key` updates the variable is updated.
    ///
    /// If the `key` is not present in the config it is set on init, if the `key` is present the `var` is set on init.
    fn bind<T: ConfigValue>(&self, key: impl Into<ConfigKey>, var: impl Var<T>) -> VarHandles {
        let key = key.into();
        let wk_errors = self.errors().downgrade();
        let json_var = var
            .filter_map_bidi(
                // T -> JSON
                clmv!(key, wk_errors, |typed| {
                    match serde_json::to_value(typed) {
                        Ok(json) => {
                            if let Some(errors) = wk_errors.upgrade() {
                                if errors.with(|e| e.entry(&key).next().is_some()) {
                                    let _ = errors.modify(|e| e.clear_entry(&key));
                                }
                            }
                            Some(json)
                        }
                        Err(e) => {
                            if let Some(errors) = wk_errors.upgrade() {
                                let _ = errors.modify(|es| es.push(ConfigError::new_get(key, e)));
                            }
                            None
                        }
                    }
                }),
                // JSON -> T
                clmv!(key, |json| match serde_json::from_value(json.clone()) {
                    Ok(typed) => {
                        if let Some(errors) = wk_errors.upgrade() {
                            if errors.with(|e| e.entry(&key).next().is_some()) {
                                let _ = errors.modify(|es| es.clear_entry(&key));
                            }
                        }
                        Some(typed)
                    }
                    Err(e) => {
                        if let Some(errors) = wk_errors.upgrade() {
                            let _ = errors.modify(|es| es.push(ConfigError::new_get(key, e)));
                        }
                        None
                    }
                }),
                || serde_json::Value::Null,
            )
            .boxed();

        self.bind_json(key, json_var)
    }

    /// Gets a variable that is bound to the config `key`.
    ///
    /// This is equivalent to using [`bind`] on a new variable, except `default` will be called only if the `key`
    /// is not present in the config on init.
    ///
    /// [`bind`]: Self::bind
    fn var<T: ConfigValue>(&self, key: impl Into<ConfigKey>, default: impl Fn() -> T) -> BoxedVar<T> {
        let key = key.into();
        let json_var = self.var_json(key.clone(), &|| serde_json::to_value(default()).unwrap_or(serde_json::Value::Null));
        let wk_errors = self.errors().downgrade();

        json_var
            .filter_map_bidi(
                // JSON -> T
                clmv!(key, wk_errors, |json| match serde_json::from_value(json.clone()) {
                    Ok(typed) => {
                        if let Some(errors) = wk_errors.upgrade() {
                            if errors.with(|e| e.entry(&key).next().is_some()) {
                                let _ = errors.modify(|e| e.to_mut().clear_entry(&key));
                            }
                        }
                        Some(typed)
                    }
                    Err(e) => {
                        if let Some(errors) = wk_errors.upgrade() {
                            let _ = errors.modify(|es| es.to_mut().push(ConfigError::new_get(key, e)));
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
                                    let _ = errors.modify(|e| e.to_mut().clear_entry(&key));
                                }
                            }
                            Some(json)
                        }
                        Err(e) => {
                            if let Some(errors) = wk_errors.upgrade() {
                                let _ = errors.modify(|es| es.to_mut().push(ConfigError::new_get(key, e)));
                            }
                            None
                        }
                    }
                }),
                default,
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
            map.insert(key, value);
        }
        Ok(())
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
pub struct SyncConfig<M: ConfigMap> {
    sync_var: ArcVar<M>,
    errors: ArcVar<ConfigErrors>,
}
impl<M: ConfigMap> SyncConfig<M> {
    /// Open write the `file`
    pub fn sync(file: impl Into<PathBuf>) -> Self {
        let errors = var(ConfigErrors::default());
        let sync_var = WATCHER.sync(
            file,
            M::empty(),
            clmv!(errors, |r| {
                match (|| M::read(r?))() {
                    Ok(ok) => {
                        if errors.with(|e| e.io().next().is_some()) {
                            errors.modify(|e| e.clear_io());
                        }
                        Some(ok)
                    }
                    Err(e) => {
                        errors.modify(|es| es.push(ConfigError::new_read(e)));
                        None
                    }
                }
            }),
            clmv!(errors, |map, w| {
                match (|| {
                    let mut w = w?;
                    map.write(&mut w)?;
                    w.commit()
                })() {
                    Ok(()) => {
                        if errors.with(|e| e.io().next().is_some()) {
                            errors.modify(|e| e.clear_io());
                        }
                    }
                    Err(e) => {
                        errors.modify(|es| es.push(ConfigError::new_write(e)));
                    }
                }
            }),
        );

        Self { sync_var, errors }
    }

    fn bind_non_static<T: ConfigValue>(&self, key: ConfigKey, var: &impl Var<T>) -> (VarHandle, VarHandle) {
        let errors = &self.errors;

        let map_to_var = if var.capabilities().is_always_read_only() {
            VarHandle::dummy()
        } else {
            let wk_var = var.downgrade();
            self.sync_var.hook(Box::new(clmv!(errors, key, |map| {
                if let Some(var) = wk_var.upgrade() {
                    match map.as_any().downcast_ref::<M>().unwrap().get::<T>(&key) {
                        Ok(value) => {
                            if errors.with(|e| e.entry(&key).next().is_some()) {
                                errors.modify(|e| e.clear_entry(&key));
                            }

                            if let Some(value) = value {
                                let _ = var.set(value);
                            }
                        }
                        Err(e) => {
                            errors.modify(|es| es.push(ConfigError::new_get(key, e)));
                        }
                    }
                    true
                } else {
                    false
                }
            })))
        };

        let wk_sync_var = self.sync_var.downgrade();
        let var_to_map = var.hook(Box::new(clmv!(errors, |value| {
            if let Some(sync_var) = wk_sync_var.upgrade() {
                let value = value.as_any().downcast_ref::<T>().unwrap().clone();
                sync_var.modify(clmv!(key, errors, |m| {
                    match ConfigMap::set(m, key, value) {
                        Ok(()) => {
                            if errors.with(|e| e.entry(&key).next().is_some()) {
                                errors.modify(|e| e.clear_entry(&key));
                            }
                        }
                        Err(e) => {
                            errors.modify(|es| es.push(ConfigError::new_set(key, e)));
                        }
                    }
                }));
                true
            } else {
                false
            }
        })));

        (map_to_var, var_to_map)
    }
}
impl<M: ConfigMap> AnyConfig for SyncConfig<M> {
    fn errors(&self) -> BoxedVar<ConfigErrors> {
        self.errors.boxed()
    }

    fn bind_json(&self, key: ConfigKey, var: BoxedVar<serde_json::Value>) -> VarHandles {
        let var = var.actual_var();

        match self.sync_var.with(|m| ConfigMap::get_json(m, &key)) {
            Ok(json) => match json {
                Some(json)=
            }
        }
    }

    fn var_json(&self, key: ConfigKey, default: &dyn Fn() -> serde_json::Value) -> BoxedVar<serde_json::Value> {
        todo!()
    }
}
impl<M: ConfigMap> Config for SyncConfig<M> {
    fn bind<T: ConfigValue>(&self, key: impl Into<ConfigKey>, var: impl Var<T>) -> VarHandles {
        let var = var.actual_var();

        let key = key.into();
        match self.sync_var.with(|m| ConfigMap::get::<T>(m, &key)) {
            Ok(value) => match value {
                Some(val) => {
                    let _ = var.set(val);
                }
                None => {
                    let value = var.get();
                    let error = &self.error;
                    self.sync_var.modify(clmv!(error, key, |map| {
                        if let Err(e) = ConfigMap::set(map, key, value) {
                            error.set(Some(Arc::new(e)));
                        }
                    }));
                }
            },
            Err(e) => {
                self.error.set(Some(Arc::new(e)));
            }
        }

        if var.capabilities().is_always_static() {
            return VarHandles::dummy();
        }

        let (a, b) = self.bind_non_static(key, &var);

        VarHandles::from_iter([a, b])
    }

    fn var<T: ConfigValue>(&self, key: impl Into<ConfigKey>, default: impl FnOnce() -> T) -> BoxedVar<T> {
        let key = key.into();
        let var = match self.sync_var.with(|m| ConfigMap::get::<T>(m, &key)) {
            Ok(value) => match value {
                Some(val) => var(val),
                None => {
                    let value = default();

                    var(value)
                }
            },
            Err(e) => {
                self.error.set(Some(Arc::new(e)));
                var(default())
            }
        };

        let (a, b) = self.bind_non_static(key, &var);
        a.perm();
        b.perm();

        var.boxed()
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

    fn bind_json(&self, key: ConfigKey, var: BoxedVar<serde_json::Value>) -> VarHandles {
        self.cfg.bind_json(key.into(), var.actual_var().read_only())
    }

    fn var_json(&self, key: ConfigKey, default: &dyn Fn() -> serde_json::Value) -> BoxedVar<serde_json::Value> {
        self.cfg.var_json(key.into(), default).read_only()
    }
}
impl<C: Config> Config for ReadOnlyConfig<C> {
    fn bind<T: ConfigValue>(&self, key: impl Into<ConfigKey>, var: impl Var<T>) -> VarHandles {
        self.cfg.bind(key.into(), var.actual_var().read_only())
    }

    fn var<T: ConfigValue>(&self, key: impl Into<ConfigKey>, default: impl Fn() -> T) -> BoxedVar<T> {
        self.cfg.var(key.into(), default).read_only()
    }
}

/// Represents a config source that is read and written too, when a key is not present in the source
/// the fallback variable is used, but if that variable is modified the key is inserted in the primary config.
pub struct ConfigWithFallback<S: Config, F: Config> {
    fallback: F,
    source: S,
    // !!: TODO
}

/// Represents a config source that swap its backend without disconnecting any bound keys.
///
/// Note that the [`CONFIG`] service already uses this type internally.
pub struct SwapConfig {
    // !!: TODO
}

/// Config error.
#[derive(Clone, Debug)]
pub enum ConfigError {
    /// Error reading from the external storage to memory.
    Read(Arc<io::Error>),
    /// Error writting from memory to the external storage.
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
