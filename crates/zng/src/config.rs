#![cfg(feature = "config")]

//! Config service, sources and other types.
//!
//! The configuration service [`CONFIG`] separates config using from config writing. A config
//! is a variable of a serializable type, widgets and other components request a config using an unique text name and
//! then simply use the variable like any other. The app optionally sets one or more config sources that are automatically
//! updated when a config variable changes and are monitored for changes that are propagated back to the config variables.
//!
//! # Sources
//!
//! The default config source is the [`MemoryConfig`] that only lives for the app process lifetime, this can
//! be used to connect different UI components, more importantly it also means that the [`CONFIG`] service always works.
//!
//! ```
//! use zng::prelude::*;
//!
//! fn txt_input() -> UiNode {
//!     TextInput!(CONFIG.get("example-txt", Txt::from("")))
//! }
//!
//! fn txt_display() -> UiNode {
//!     Text!(CONFIG.get("example-txt", Txt::from("")))
//! }
//!
//! # fn main() { }
//! # fn demo() {
//! # let _scope = APP.defaults();
//! # let _ =
//! Container! {
//!     child = txt_input();
//!     child_bottom = txt_display(), 20;
//! }
//! # ; }
//! ```
//!
//! The example above uses a config key `"example-txt"`, no config source is set so this config will only last for the
//! duration of the app instance, but both widgets are synchronized because they are bound to the same config.
//!
//! The example below setups a [`JsonConfig`] that persists the configs to a JSON file. The file updates when
//! a config variable is modified and the variables are updated when the file is modified externally.
//!
//! ```
//! # use zng::prelude::*;
//! # fn main() { }
//! # fn demo() {
//! # let _scope = APP.defaults();
//! let cfg = zng::config::JsonConfig::sync("target/tmp/example.config.json");
//! CONFIG.load(cfg);
//! # }
//! ```
//!
//! ## Other Sources
//!
//! The JSON, TOML, YAML and RON are available behind a feature flags, you can also implement your own source.
//!
//! Some *meta* sources are also provided, they enables composite sources, such as having two sources,
//! *default config* and *user config* where the user config file only records the non-default values.
//!
//! The next example demonstrates a more complex setup:
//!
//! ```
//! use zng::config::*;
//!
//! fn load_config() -> Box<dyn FallbackConfigReset> {
//!     // config file for the app, keys with prefix "main." are saved here.
//!     let user_cfg = JsonConfig::sync("target/tmp/example.config.json");
//!     // entries not found in `user_cfg` bind to this file first before going to embedded fallback.
//!     let default_cfg = ReadOnlyConfig::new(JsonConfig::sync("examples/config/res/defaults.json"));
//!
//!     // the app settings.
//!     let main_cfg = FallbackConfig::new(user_cfg, default_cfg);
//!
//!     // Clone a ref that can be used to reset specific entries.
//!     let main_ref = main_cfg.clone_boxed();
//!
//!     // any other configs (Window::save_state for example)
//!     let other_cfg = JsonConfig::sync("target/tmp/example.config.other.json");
//!
//!     CONFIG.load(SwitchConfig::new().with_prefix("main.", main_cfg).with_prefix("", other_cfg));
//!
//!     main_ref
//! }
//! ```
//!
//! # Full API
//!
//! See [`zng_ext_config`] for the full config API.

pub use zng_ext_config::{
    AnyConfig, CONFIG, Config, ConfigKey, ConfigStatus, ConfigValue, FallbackConfig, FallbackConfigReset, MemoryConfig, RawConfigValue,
    ReadOnlyConfig, SwapConfig, SwitchConfig,
};

#[cfg(feature = "window")]
pub use zng_wgt_window::{SaveState, save_state_node};

#[cfg(feature = "config_json")]
pub use zng_ext_config::JsonConfig;

#[cfg(feature = "config_ron")]
pub use zng_ext_config::RonConfig;

#[cfg(feature = "config_toml")]
pub use zng_ext_config::TomlConfig;

#[cfg(feature = "config_yaml")]
pub use zng_ext_config::YamlConfig;

/// Settings are the config the user can directly edit, this module implements a basic settings data model.
///
/// # Full API
///
/// See [`zng_ext_config::settings`] for the full settings API.
pub mod settings {
    pub use zng_ext_config::settings::{
        CategoriesBuilder, Category, CategoryBuilder, CategoryId, SETTINGS, Setting, SettingBuilder, SettingsBuilder,
    };
    pub use zng_wgt_input::cmd::{SETTINGS_CMD, on_pre_settings, on_settings};

    /// Settings editor widget.
    ///
    /// # Full API
    ///
    /// See [`zng_wgt_settings`] for the full settings editor API.
    #[cfg(feature = "settings_editor")]
    pub mod editor {
        pub use zng_wgt_settings::{
            CategoriesListArgs, CategoryHeaderArgs, CategoryItemArgs, SettingArgs, SettingBuilderEditorExt, SettingsArgs, SettingsCtxExt,
            SettingsEditor, categories_list_fn, category_header_fn, category_item_fn, setting_fn, settings_fn,
        };
    }
}
