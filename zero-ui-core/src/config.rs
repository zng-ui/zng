//! Config service and sources.

use std::{
    any::Any,
    borrow::Cow,
    collections::{hash_map, HashMap},
    fmt, io,
    sync::Arc,
};

use crate::{
    app::AppExtension,
    app_local, clmv,
    fs_watcher::{WatchFile, WriteFile},
    task,
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

#[cfg(feature = "toml")]
mod toml;
#[cfg(feature = "toml")]
pub use self::toml::*;

#[cfg(feature = "ron")]
mod ron;
#[cfg(feature = "ron")]
pub use self::ron::*;

/// Application extension that provides mouse events and service.
///
/// # Services
///
/// Services this extension provides.
///
/// * [`CONFIG`]
///
///
/// # Default
///
/// This extension is included in the [default app].
///
/// [default app]: crate::app::App::default
#[derive(Default)]
pub struct ConfigManager {}

impl AppExtension for ConfigManager {}

/// Represents the app main config.
///
/// Config sources must be loaded using [`CONFIG.load`], otherwise the config only leaves for the
/// duration of the app instance.
///
/// [`CONFIG.load`]: CONFIG::load
pub struct CONFIG;
impl CONFIG {
    /// Replace the config source.
    ///
    /// Variables and bindings survive source replacement, updating to the new value or setting the new source
    /// if the key is not present in the new source.
    pub fn load(&self, source: impl AnyConfig) {
        CONFIG_SV.write().load(source)
    }

    /// Gets a read-only variable that represents the IO status of the config.
    pub fn status(&self) -> BoxedVar<ConfigStatus> {
        CONFIG_SV.read().status()
    }

    /// Wait until [`status`] is idle (not loading nor saving).
    ///
    /// [`status`]: Self::status
    pub async fn wait_idle(&self) {
        task::yield_now().await; // in case a `load` request was just made

        let status = self.status();
        while !status.get().is_idle() {
            status.wait_is_new().await;
        }
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
    fn get_raw(&mut self, key: ConfigKey, default: RawConfigValue, shared: bool) -> BoxedVar<RawConfigValue> {
        CONFIG_SV.write().get_raw(key, default, shared)
    }

    fn contains_key(&self, key: &ConfigKey) -> bool {
        CONFIG_SV.read().contains_key(key)
    }

    fn status(&self) -> BoxedVar<ConfigStatus> {
        CONFIG.status()
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
    /// Gets a read-only variable that represents the IO status of the config.
    fn status(&self) -> BoxedVar<ConfigStatus>;

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
    fn get<T: ConfigValue>(&mut self, key: impl Into<ConfigKey>, default: impl FnOnce() -> T) -> BoxedVar<T>;
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
    fn get_raw(&mut self, key: ConfigKey, default: RawConfigValue, shared: bool) -> BoxedVar<RawConfigValue> {
        self.cfg.get_raw(key, default, shared).read_only()
    }

    fn contains_key(&self, key: &ConfigKey) -> bool {
        self.cfg.contains_key(key)
    }

    fn status(&self) -> BoxedVar<ConfigStatus> {
        self.cfg.status()
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
    fn get_raw(&mut self, _: ConfigKey, default: RawConfigValue, _: bool) -> BoxedVar<RawConfigValue> {
        LocalVar(default).boxed()
    }
    fn contains_key(&self, _: &ConfigKey) -> bool {
        false
    }

    fn status(&self) -> BoxedVar<ConfigStatus> {
        LocalVar(ConfigStatus::Loaded).boxed()
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
    ///
    /// Note that this means the variables bound from the previous source in [`get_or_bind`] **will be reused**,
    /// the previous source must be dropped before calling this method.
    ///
    /// [`get_or_bind`]: Self::get_or_bind
    pub fn rebind(&mut self, source: &mut dyn AnyConfig) {
        self.0.retain(|key, wk_var| wk_var.rebind(key, source));
    }
}
trait AnyConfigVar: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn can_upgrade(&self) -> bool;
    fn rebind(&mut self, key: &ConfigKey, source: &mut dyn AnyConfig) -> bool;
}
impl<T: ConfigValue> AnyConfigVar for ConfigVar<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn can_upgrade(&self) -> bool {
        self.var.strong_count() > 0
    }

    fn rebind(&mut self, key: &ConfigKey, source: &mut dyn AnyConfig) -> bool {
        let var = if let Some(var) = self.var.upgrade() {
            var
        } else {
            // no need to retain, will bind directly to new source if requested later.
            return false;
        };

        // get or insert the source var
        let source_var = source.get_raw(key.clone(), RawConfigValue::serialize(var.get()).unwrap(), false);

        match RawConfigValue::deserialize::<T>(source_var.get()) {
            Ok(value) => {
                let _ = var.set(value);
            }
            Err(e) => {
                // invalid data error
                tracing::error!("rebind config get({key:?}) error, {e:?}");

                // try to override
                let _ = source_var.set(RawConfigValue::serialize(var.get()).unwrap());
            }
        }

        self.binding = source_var.bind_filter_map_bidi(
            &var,
            // Raw -> T
            clmv!(key, |raw| {
                match RawConfigValue::deserialize(raw.clone()) {
                    Ok(value) => Some(value),
                    Err(e) => {
                        tracing::error!("rebind config get({key:?}) error, {e:?}");
                        None
                    }
                }
            }),
            // T -> Raw
            clmv!(key, source_var, |value| {
                let _strong_ref = &source_var;
                match RawConfigValue::serialize(value) {
                    Ok(raw) => Some(raw),
                    Err(e) => {
                        tracing::error!("rebind config set({key:?}) error, {e:?}");
                        None
                    }
                }
            }),
        );

        true
    }
}

/// Represents the current IO status of the config.
#[derive(Debug, Clone)]
pub enum ConfigStatus {
    /// Config is loaded.
    Loaded,
    /// Config is loading.
    Loading,
    /// Config is saving.
    Saving,
    /// Config last load failed.
    LoadErrors(Vec<Arc<dyn std::error::Error + Send + Sync>>),
    /// Config last save failed.
    SaveErrors(Vec<Arc<dyn std::error::Error + Send + Sync>>),
}
impl ConfigStatus {
    /// If status is not loading nor saving.
    pub fn is_idle(&self) -> bool {
        !matches!(self, Self::Loading | Self::Saving)
    }

    /// If status is load or save errors.
    pub fn is_err(&self) -> bool {
        matches!(self, ConfigStatus::LoadErrors(_) | ConfigStatus::SaveErrors(_))
    }

    /// Errors list.
    ///
    /// Note that [`is_err`] may be true even when this is empty.
    ///
    /// [`is_err`]: Self::is_err
    pub fn errors(&self) -> &[Arc<dyn std::error::Error + Send + Sync>] {
        match self {
            ConfigStatus::LoadErrors(e) => e,
            ConfigStatus::SaveErrors(e) => e,
            _ => &[],
        }
    }
}
impl fmt::Display for ConfigStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Loaded => Ok(()),
            Self::Loading => write!(f, "loading…"),
            Self::Saving => write!(f, "saving…"),
            Self::LoadErrors(e) => {
                writeln!(f, "read errors:")?;
                for e in e {
                    writeln!(f, "   {e}")?;
                }
                Ok(())
            }
            Self::SaveErrors(e) => {
                writeln!(f, "write errors:")?;
                for e in e {
                    writeln!(f, "   {e}")?;
                }
                Ok(())
            }
        }
    }
}
impl PartialEq for ConfigStatus {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::LoadErrors(a), Self::LoadErrors(b)) => a.is_empty() && b.is_empty(),
            (Self::SaveErrors(a), Self::SaveErrors(b)) => a.is_empty() && b.is_empty(),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}
impl Eq for ConfigStatus {}
