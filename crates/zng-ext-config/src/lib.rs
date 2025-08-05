#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Config service and sources.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
// suppress nag about very simple boxed closure signatures.
#![expect(clippy::type_complexity)]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

mod serde_value;

mod fallback;
pub use fallback::*;

mod swap;
pub use swap::*;

mod switch;
pub use switch::*;

mod sync;
pub use sync::*;

#[cfg(feature = "json")]
mod json;
#[cfg(feature = "json")]
pub use json::*;

#[cfg(feature = "toml")]
mod toml;
#[cfg(feature = "toml")]
pub use self::toml::*;

#[cfg(feature = "ron")]
mod ron;
#[cfg(feature = "ron")]
pub use self::ron::*;

#[cfg(feature = "yaml")]
mod yaml;
#[cfg(feature = "yaml")]
pub use self::yaml::*;

pub mod settings;

use std::{
    any::Any,
    collections::{HashMap, hash_map},
    fmt, io,
    sync::Arc,
};

use zng_app::{AppExtension, update::EventUpdate, view_process::raw_events::LOW_MEMORY_EVENT};
use zng_app_context::app_local;
use zng_clone_move::clmv;
use zng_ext_fs_watcher::{WatchFile, WatcherReadStatus, WatcherSyncStatus, WriteFile};
use zng_task as task;
use zng_txt::Txt;
use zng_var::{Var, VarHandles, VarValue, WeakVar, const_var, var};

/// Application extension that provides mouse events and service.
///
/// # Services
///
/// Services this extension provides.
///
/// * [`CONFIG`]
#[derive(Default)]
#[non_exhaustive]
pub struct ConfigManager {}

impl AppExtension for ConfigManager {
    fn event_preview(&mut self, update: &mut EventUpdate) {
        if LOW_MEMORY_EVENT.on(update).is_some() {
            CONFIG_SV.write().low_memory();
        }
    }
}

/// Represents the app main config.
///
/// Config sources must be loaded using [`CONFIG.load`], otherwise the config only lives for the
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
    pub fn status(&self) -> Var<ConfigStatus> {
        CONFIG_SV.read().status()
    }

    /// Wait until [`status`] is idle (not loading nor saving).
    ///
    /// [`status`]: Self::status
    pub async fn wait_idle(&self) {
        task::yield_now().await; // in case a `load` request was just made
        self.status().wait_match(|s| s.is_idle()).await;
    }

    /// Gets a variable that is bound to the config `key`.
    ///
    /// The same variable is returned for multiple requests of the same key. If the loaded config is not read-only the
    /// returned variable can be set to update the config source.
    ///
    /// The `default` value is used if the key is not found in the config, the default value
    /// is not inserted in the config, the key is inserted or replaced only when the returned variable updates.
    pub fn get<T: ConfigValue>(&self, key: impl Into<ConfigKey>, default: T) -> Var<T> {
        CONFIG_SV.write().get(key.into(), default, false)
    }

    /// Gets a variable that is bound to the config `key`, the `value` is set and if the `key` was
    /// not present it is also inserted on the config.
    pub fn insert<T: ConfigValue>(&self, key: impl Into<ConfigKey>, value: T) -> Var<T> {
        CONFIG_SV.write().get(key.into(), value, true)
    }
}
impl AnyConfig for CONFIG {
    fn get_raw(&mut self, key: ConfigKey, default: RawConfigValue, insert: bool) -> Var<RawConfigValue> {
        CONFIG_SV.write().get_raw(key, default, insert)
    }

    fn contains_key(&mut self, key: ConfigKey) -> Var<bool> {
        CONFIG_SV.write().contains_key(key)
    }

    fn status(&self) -> Var<ConfigStatus> {
        CONFIG.status()
    }

    fn remove(&mut self, key: &ConfigKey) -> bool {
        CONFIG_SV.write().remove(key)
    }

    fn low_memory(&mut self) {
        CONFIG_SV.write().low_memory()
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
#[diagnostic::on_unimplemented(note = "`ConfigValue` is implemented for all `T: VarValue + Serialize + DeserializeOwned`")]
pub trait ConfigValue: VarValue + serde::Serialize + serde::de::DeserializeOwned {}
impl<T: VarValue + serde::Serialize + serde::de::DeserializeOwned> ConfigValue for T {}

/// Represents any entry type in a config.
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct RawConfigValue(pub serde_value::Value);
impl RawConfigValue {
    /// Serialize to the raw config format.
    pub fn serialize<T: serde::Serialize>(value: T) -> Result<Self, serde_value::SerializerError> {
        serde_value::to_value(value).map(Self)
    }

    /// Deserialize from the raw config format.
    pub fn deserialize<T: serde::de::DeserializeOwned>(self) -> Result<T, serde_value::DeserializerError> {
        T::deserialize(self.0)
    }
}

/// Represents one or more config sources behind a dynamic reference.
///
/// See [`Config`] for the full trait.
pub trait AnyConfig: Send + Any {
    /// Gets a read-only variable that represents the IO status of the config.
    fn status(&self) -> Var<ConfigStatus>;

    /// Gets a weak typed variable to the config `key`.
    ///
    /// This method is used when `T` cannot be passed because the config is behind a dynamic reference,
    /// the backend must convert the value from the in memory representation to [`RawConfigValue`].
    ///
    /// The `default` value is used if the key is not found in the config, the default value
    /// is only inserted in the config if `insert`, otherwise the key is inserted or replaced only when the returned variable changes.
    fn get_raw(&mut self, key: ConfigKey, default: RawConfigValue, insert: bool) -> Var<RawConfigValue>;

    /// Gets a read-only variable that tracks if an entry for the `key` is in the backing storage.
    fn contains_key(&mut self, key: ConfigKey) -> Var<bool>;

    /// Removes the `key` from the backing storage.
    ///
    /// Any active config variable for the key will continue to work normally, retaining the last config value and
    /// re-inserting the key if assigned a new value.
    ///
    /// Returns `true` if the key was found and will be removed in the next app update.
    /// Returns `false` if the key was not found or the config is read-only.
    fn remove(&mut self, key: &ConfigKey) -> bool;

    /// Cleanup and flush RAM caches.
    fn low_memory(&mut self);
}

/// Represents one or more config sources.
///
/// This trait is already implemented for all [`AnyConfig`] implementers.
pub trait Config: AnyConfig {
    /// Gets a variable that is bound to the config `key`.
    ///
    /// The same variable is returned for multiple requests of the same key. If the loaded config is not read-only the
    /// returned variable can be set to update the config source.
    ///
    /// The `default` value is used if the key is not found in the config, the default value
    /// is only inserted in the config if `insert`, otherwise the key is inserted or replaced only when the returned variable changes.
    fn get<T: ConfigValue>(&mut self, key: impl Into<ConfigKey>, default: T, insert: bool) -> Var<T>;
}
impl<C: AnyConfig> Config for C {
    fn get<T: ConfigValue>(&mut self, key: impl Into<ConfigKey>, default: T, insert: bool) -> Var<T> {
        get_impl(self, insert, key.into(), default)
    }
}
fn get_impl<T: ConfigValue, C: AnyConfig>(source: &mut C, insert: bool, key: ConfigKey, default: T) -> Var<T> {
    source
        .get_raw(key, RawConfigValue::serialize(&default).unwrap(), insert)
        .filter_map_bidi(
            move |raw| match raw.clone().deserialize() {
                Ok(v) => Some(v),
                Err(e) => {
                    #[cfg(debug_assertions)]
                    tracing::error!(
                        "failed to get config as `{}`, raw value was {:?}, {e}",
                        std::any::type_name::<T>(),
                        raw
                    );
                    #[cfg(not(debug_assertions))]
                    tracing::error!("failed to get config, {e}");
                    None
                }
            },
            |v| match RawConfigValue::serialize(v) {
                Ok(v) => Some(v),
                Err(e) => {
                    tracing::error!("failed to set config, {e}");
                    None
                }
            },
            move || default.clone(),
        )
}

/// Config wrapper that only provides read-only variables from the inner config.
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
    fn get_raw(&mut self, key: ConfigKey, default: RawConfigValue, _: bool) -> Var<RawConfigValue> {
        self.cfg.get_raw(key, default, false).read_only()
    }

    fn contains_key(&mut self, key: ConfigKey) -> Var<bool> {
        self.cfg.contains_key(key)
    }

    fn status(&self) -> Var<ConfigStatus> {
        self.cfg.status()
    }

    fn remove(&mut self, _key: &ConfigKey) -> bool {
        false
    }

    fn low_memory(&mut self) {
        self.cfg.low_memory()
    }
}

/// Memory only config.
///
/// Values are retained in memory even if all variables to the key are dropped, but they are lost when the process ends.
#[derive(Default)]
pub struct MemoryConfig {
    values: HashMap<ConfigKey, Var<RawConfigValue>>,
    contains: HashMap<ConfigKey, WeakVar<bool>>,
}

impl AnyConfig for MemoryConfig {
    fn status(&self) -> Var<ConfigStatus> {
        const_var(ConfigStatus::Loaded)
    }

    fn get_raw(&mut self, key: ConfigKey, default: RawConfigValue, _insert: bool) -> Var<RawConfigValue> {
        match self.values.entry(key) {
            hash_map::Entry::Occupied(e) => e.get().clone(),
            hash_map::Entry::Vacant(e) => {
                let r = var(default);

                if let Some(v) = self.contains.get(e.key()) {
                    if let Some(v) = v.upgrade() {
                        v.set(true);
                    }
                }

                e.insert(r).clone()
            }
        }
    }

    fn contains_key(&mut self, key: ConfigKey) -> Var<bool> {
        match self.contains.entry(key) {
            hash_map::Entry::Occupied(mut e) => {
                if let Some(r) = e.get().upgrade() {
                    r
                } else {
                    let r = var(self.values.contains_key(e.key()));
                    e.insert(r.downgrade());
                    r
                }
            }
            hash_map::Entry::Vacant(e) => {
                let r = var(self.values.contains_key(e.key()));
                e.insert(r.downgrade());
                r
            }
        }
    }

    fn remove(&mut self, key: &ConfigKey) -> bool {
        if self.values.remove(key).is_some() {
            self.contains.retain(|_, v| v.strong_count() > 0);

            if let Some(v) = self.contains.get(key) {
                if let Some(v) = v.upgrade() {
                    v.set(false);
                }
            }
            true
        } else {
            false
        }
    }

    fn low_memory(&mut self) {
        self.contains.retain(|_, v| v.strong_count() > 0);
    }
}

struct ConfigVar<T: ConfigValue> {
    var: WeakVar<T>,
    binding: VarHandles,
}
impl<T: ConfigValue> ConfigVar<T> {
    fn new_any(var: WeakVar<T>, binding: VarHandles) -> Box<dyn AnyConfigVar> {
        Box::new(Self { var, binding })
    }
}
struct ConfigContainsVar {
    var: WeakVar<bool>,
    binding: VarHandles,
}

/// Map of configs already bound to a variable.
///
/// The map only holds a weak reference to the variables.
#[derive(Default)]
pub struct ConfigVars {
    values: HashMap<ConfigKey, Box<dyn AnyConfigVar>>,
    contains: HashMap<ConfigKey, ConfigContainsVar>,
}
impl ConfigVars {
    /// Gets the already bound variable or calls `bind` to generate a new binding.
    pub fn get_or_bind<T: ConfigValue>(&mut self, key: ConfigKey, bind: impl FnOnce(&ConfigKey) -> Var<T>) -> Var<T> {
        match self.values.entry(key) {
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
                let cfg = bind(e.key());

                let res = var(cfg.get());
                let binding = res.bind_map_bidi(
                    &cfg,
                    clmv!(cfg, |v| {
                        let _strong_ref = &cfg;
                        v.clone()
                    }),
                    Clone::clone,
                );

                e.insert(ConfigVar::new_any(res.downgrade(), binding));
                res
            }
            hash_map::Entry::Vacant(e) => {
                let cfg = bind(e.key());
                let res = var(cfg.get());
                let binding = res.bind_map_bidi(
                    &cfg,
                    clmv!(cfg, |v| {
                        let _strong_ref = &cfg;
                        v.clone()
                    }),
                    Clone::clone,
                );

                e.insert(ConfigVar::new_any(res.downgrade(), binding));
                res
            }
        }
    }

    /// Bind the contains variable.
    pub fn get_or_bind_contains(&mut self, key: ConfigKey, bind: impl FnOnce(&ConfigKey) -> Var<bool>) -> Var<bool> {
        match self.contains.entry(key) {
            hash_map::Entry::Occupied(mut e) => {
                if let Some(res) = e.get().var.upgrade() {
                    return res;
                }

                let cfg = bind(e.key());
                let res = var(cfg.get());

                let binding = VarHandles::from([
                    cfg.bind(&res),
                    res.hook(move |_| {
                        let _strong_ref = &cfg;
                        true
                    }),
                ]);

                e.insert(ConfigContainsVar {
                    var: res.downgrade(),
                    binding,
                });

                res
            }
            hash_map::Entry::Vacant(e) => {
                let cfg = bind(e.key());
                let res = var(cfg.get());

                let binding = VarHandles::from([
                    cfg.bind(&res),
                    res.hook(move |_| {
                        let _strong_ref = &cfg;
                        true
                    }),
                ]);

                e.insert(ConfigContainsVar {
                    var: res.downgrade(),
                    binding,
                });

                res
            }
        }
    }

    /// Bind all variables to the new `source`.
    ///
    /// If the map entry is present in the `source` the variable is updated to the new value, if not the entry
    /// is inserted in the source. The variable is then bound to the source.
    pub fn rebind(&mut self, source: &mut dyn AnyConfig) {
        self.values.retain(|key, wk_var| wk_var.rebind(key, source));
        self.contains.retain(|key, wk_var| wk_var.rebind(key, source));
    }

    /// System warning low memory, flush caches.
    pub fn low_memory(&mut self) {
        self.values.retain(|_, v| v.can_upgrade());
        self.contains.retain(|_, v| v.var.strong_count() > 0)
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

        // var.set_from_map(source_var)
        var.modify(clmv!(source_var, key, |vm| {
            match RawConfigValue::deserialize::<T>(source_var.get()) {
                Ok(value) => {
                    vm.set(value);
                }
                Err(e) => {
                    // invalid data error
                    tracing::error!("rebind config get({key:?}) error, {e:?}");

                    // try to override
                    source_var.set(RawConfigValue::serialize(vm.value()).unwrap());
                }
            }
        }));

        let mut first = true;
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
                if std::mem::take(&mut first) {
                    return None; // skip value we just set.
                }

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
impl ConfigContainsVar {
    fn rebind(&mut self, key: &ConfigKey, source: &mut dyn AnyConfig) -> bool {
        if let Some(res) = self.var.upgrade() {
            let cfg = source.contains_key(key.clone());
            res.set_from(&cfg);

            self.binding = VarHandles::from([
                cfg.bind(&res),
                res.hook(move |_| {
                    let _strong_ref = &cfg;
                    true
                }),
            ]);

            true
        } else {
            false
        }
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
    LoadErrors(ConfigStatusError),
    /// Config last save failed.
    SaveErrors(ConfigStatusError),
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

    /// merge all `status`.
    pub fn merge_status(status: impl Iterator<Item = ConfigStatus>) -> ConfigStatus {
        let mut load_errors = vec![];
        let mut save_errors = vec![];
        let mut loading = false;
        let mut saving = false;
        for s in status {
            match s {
                ConfigStatus::Loaded => {}
                ConfigStatus::Loading => loading = true,
                ConfigStatus::Saving => saving = true,
                ConfigStatus::LoadErrors(e) => {
                    if load_errors.is_empty() {
                        load_errors = e;
                    } else {
                        load_errors.extend(e);
                    }
                }
                ConfigStatus::SaveErrors(e) => {
                    if save_errors.is_empty() {
                        save_errors = e;
                    } else {
                        save_errors.extend(e);
                    }
                }
            }
        }

        if loading {
            ConfigStatus::Loading
        } else if saving {
            ConfigStatus::Saving
        } else if !load_errors.is_empty() {
            ConfigStatus::LoadErrors(load_errors)
        } else if !save_errors.is_empty() {
            ConfigStatus::SaveErrors(save_errors)
        } else {
            ConfigStatus::Loaded
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
impl WatcherSyncStatus<ConfigStatusError, ConfigStatusError> for ConfigStatus {
    fn writing() -> Self {
        ConfigStatus::Saving
    }

    fn write_error(e: ConfigStatusError) -> Self {
        ConfigStatus::SaveErrors(e)
    }
}
impl WatcherReadStatus<ConfigStatusError> for ConfigStatus {
    fn idle() -> Self {
        ConfigStatus::Loaded
    }

    fn reading() -> Self {
        ConfigStatus::Loading
    }

    fn read_error(e: ConfigStatusError) -> Self {
        ConfigStatus::LoadErrors(e)
    }
}
type ConfigStatusError = Vec<Arc<dyn std::error::Error + Send + Sync>>;
