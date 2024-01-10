//! Config service, sources and types.
//!
//! # Full API
//!
//! See [`zero_ui_ext_config`] for the full config API.

pub use zero_ui_ext_config::{
    AnyConfig, Config, ConfigKey, ConfigMap, ConfigStatus, ConfigValue, ConfigVars, FallbackConfig, FallbackConfigReset, JsonConfig,
    MemoryConfig, RawConfigValue, ReadOnlyConfig, RonConfig, SwapConfig, SwitchConfig, SyncConfig, TomlConfig, YamlConfig, CONFIG,
};
