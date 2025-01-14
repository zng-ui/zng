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
//! be used to connect different UI components, more importantly it also means that the [`CONFIG`] service always works
//! so widgets can just set configs in case a persisting source is setup.
//!
//! ```
//! use zng::prelude::*;
//!
//! fn txt_input() -> impl UiNode {
//!     TextInput!(CONFIG.get("example.txt", Txt::from("")))
//! }
//!
//! fn txt_display() -> impl UiNode {
//!     Text!(CONFIG.get("example.txt", Txt::from("")))
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
//! The example above uses a config `"example.txt"`, the text will be wiped when the app is closed, but the app
//! components are ready in case they are used in an app that enables persistent config.
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
//! The JSON format is available by default, TOML, YAML and RON are also available behind a feature flags, you can
//! also implement your own source.
//!
//! Apart from config sources that represents a format some *meta* sources are provided, they enables composite sources,
//! such as having two sources app default and user where the user config file only records the non-default values.
//!
//! The crate example `examples/config.rs` demonstrates a more complex setup:
//!
//! ```
//! use zng::config::*;
//!
//! fn load_config() -> Box<dyn FallbackConfigReset> {
//!     // config file for the app, keys with prefix "main." are saved here.
//!     let user_cfg = JsonConfig::sync("target/tmp/example.config.json");
//!     // entries not found in `user_cfg` bind to this file first before going to embedded fallback.
//!     let default_cfg = ReadOnlyConfig::new(JsonConfig::sync("examples/res/config/defaults.json"));
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
    AnyConfig, Config, ConfigKey, ConfigStatus, ConfigValue, FallbackConfig, FallbackConfigReset, JsonConfig, MemoryConfig, RawConfigValue,
    ReadOnlyConfig, SwapConfig, SwitchConfig, CONFIG,
};

#[cfg(feature = "window")]
pub use zng_wgt_window::{save_state_node, SaveState};

#[cfg(feature = "ron")]
pub use zng_ext_config::RonConfig;

#[cfg(feature = "toml")]
pub use zng_ext_config::TomlConfig;

#[cfg(feature = "yaml")]
pub use zng_ext_config::YamlConfig;

/// Settings are the config the user can directly edit, this module implements a basic settings data model.
///
/// # Full API
///
/// See [`zng_ext_config::settings`] for the full settings API.
pub mod settings {
    pub use zng_ext_config::settings::{
        CategoriesBuilder, Category, CategoryBuilder, CategoryId, Setting, SettingBuilder, SettingsBuilder, SETTINGS,
    };
    pub use zng_wgt_input::cmd::{on_pre_settings, on_settings, SETTINGS_CMD};

    /// Settings editor widget.
    ///
    /// # Full API
    ///
    /// See [`zng_wgt_settings`] for the full settings editor API.
    #[cfg(feature = "settings_editor")]
    pub mod editor {
        pub use zng_wgt_settings::{
            categories_list_fn, category_header_fn, category_item_fn, setting_fn, settings_fn, CategoriesListArgs, CategoryHeaderArgs,
            CategoryItemArgs, SettingArgs, SettingBuilderEditorExt, SettingsArgs, SettingsCtxExt, SettingsEditor,
        };
    }
}
