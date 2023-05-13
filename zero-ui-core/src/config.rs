//!: Config service and sources.

use std::{
    any::Any,
    borrow::Cow,
    collections::{hash_map, HashMap},
    fmt, io, ops,
    sync::Arc,
};

use crate::{
    app_local, clmv,
    fs_watcher::{WatchFile, WriteFile},
    text::Txt,
    var::*,
};

mod fallback;
pub use fallback::*;

mod json;
pub use json::*;

mod swap;
pub use swap::*;

mod sync;
pub use sync::*;

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

    fn get_raw(&mut self, key: ConfigKey, default: RawConfigValue, shared: bool) -> BoxedVar<RawConfigValue> {
        CONFIG_SV.write().get_raw(key, default, shared)
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

/// Represents any entry type in a config.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RawConfigValue(pub serde_json::Value);
impl RawConfigValue {
    /// Serialize to the raw config format.
    pub fn serialize<T: serde::Serialize>(value: T) -> Result<Self, serde_json::Error> {
        serde_json::to_value(value).map(Self)
    }

    /// Deserialize from the raw config format.
    pub fn deserialize<T: serde::de::DeserializeOwned>(self) -> Result<T, serde_json::Error> {
        serde_json::from_value(self.0)
    }
}

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
    /// the backend must convert the value from the in memory representation to [`RawConfigValue`].
    ///
    /// This method can run in blocking contexts, work with in memory storage only.
    fn get_raw(&self, key: &ConfigKey) -> Result<Option<RawConfigValue>, Arc<dyn std::error::Error + Send + Sync>>;

    /// Sets the weak typed value.
    ///
    /// This method is used when `T` cannot be passed because the map is behind a dynamic reference,
    /// the backend must convert to the in memory representation.
    ///
    /// If `map` is dereferenced mutable a write task will, if possible check if the entry already has the same value
    /// before mutating the map to avoid a potentially expensive IO write.
    ///
    /// This method can run in blocking contexts, work with in memory storage only.
    fn set_raw(map: &mut Cow<Self>, key: ConfigKey, value: RawConfigValue) -> Result<(), Arc<dyn std::error::Error + Send + Sync>>;

    /// Returns if the key in config.
    ///
    /// This method can run in blocking contexts, work with in memory storage only.
    fn contains_key(&self, key: &ConfigKey) -> bool;

    /// Get the value if present.
    ///
    /// This method can run in blocking contexts, work with in memory storage only.
    fn get<O: ConfigValue>(&self, key: &ConfigKey) -> Result<Option<O>, Arc<dyn std::error::Error + Send + Sync>> {
        if let Some(value) = self.get_raw(key)? {
            match RawConfigValue::deserialize(value) {
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
        match RawConfigValue::serialize(value) {
            Ok(s) => Self::set_raw(map, key, s),
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
    /// the backend must convert the value from the in memory representation to [`RawConfigValue`].
    ///
    /// If `shared` is `true` and the key was already requested it the same var is returned, if `false`
    /// a new variable is always generated. Note that if you have two different variables for the same
    /// key they will go out-of-sync as updates from setting one variable do not propagate to the other.
    ///
    /// The `default` value is used to if the key is not found in the config, the default value
    /// is not inserted in the config, the key is inserted or replaced only when the returned variable updates.
    fn get_raw(&mut self, key: ConfigKey, default: RawConfigValue, shared: bool) -> BoxedVar<RawConfigValue>;

    /// Returns if the `key` already has the key in the backing storage.
    ///
    /// Both [`get_raw`] and [`get`] methods don't insert the key on request, the key is inserted on the
    /// first time the returned variable updates.
    ///
    /// [`get_raw`]: AnyConfig::get_raw
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
        let raw_var = self.get_raw(key.clone(), RawConfigValue::serialize(&default).unwrap(), true);
        let wk_errors = self.errors().downgrade();

        raw_var
            .filter_map_bidi(
                // Raw -> T
                clmv!(key, wk_errors, |raw| match RawConfigValue::deserialize(raw.clone()) {
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
                // T -> Raw
                clmv!(key, wk_errors, |typed| {
                    match RawConfigValue::serialize(typed) {
                        Ok(raw) => {
                            if let Some(errors) = wk_errors.upgrade() {
                                if errors.with(|e| e.entry(&key).next().is_some()) {
                                    let _ = errors.modify(clmv!(key, |e| e.to_mut().clear_entry(&key)));
                                }
                            }
                            Some(raw)
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

    fn get_raw(&mut self, key: ConfigKey, default: RawConfigValue, shared: bool) -> BoxedVar<RawConfigValue> {
        self.cfg.get_raw(key, default, shared).read_only()
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

/// Config without any backing store.
pub struct NilConfig;

impl AnyConfig for NilConfig {
    fn errors(&self) -> BoxedVar<ConfigErrors> {
        LocalVar(ConfigErrors::default()).boxed()
    }
    fn get_raw(&mut self, _: ConfigKey, default: RawConfigValue, _: bool) -> BoxedVar<RawConfigValue> {
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

        let source_var = source.get_raw(key.clone(), RawConfigValue::serialize(var.get()).unwrap(), false);

        match RawConfigValue::deserialize::<T>(source_var.get()) {
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
            // Raw -> T
            clmv!(key, errors, |raw| {
                match RawConfigValue::deserialize(raw.clone()) {
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
            // T -> Raw
            clmv!(key, errors, source_var, |value| {
                let _strong_ref = &source_var;
                match RawConfigValue::serialize(value) {
                    Ok(raw) => {
                        if errors.with(|e| e.entry(&key).next().is_some()) {
                            errors.modify(clmv!(key, |e| e.to_mut().clear_entry(&key)));
                        }
                        Some(raw)
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
