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
//! # let _ =
//! Container! {
//!     child = txt_input();
//!     child_spacing = 20;
//!     child_bottom = txt_display();
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
//!     let default_cfg = JsonConfig::read("examples/config/res/defaults.json");
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

/// Settings metadata model.
///
/// Settings are the [`CONFIG`] the user can directly edit, they have associated metadata such as display name and description,
/// and will usually be editable in a special settings window. This module provides a basic settings data model, with category grouping,
/// sorting and filtering. A default settings editor window is also provided.
///
/// ```
/// # use zng::config::settings::*;
/// # use zng::prelude::*;
/// fn register_categories() {
///     SETTINGS.register_categories(|c| {
///         c.entry("cat-example", |c| c.name("Example Settings"));
///     });
/// }
///
/// fn register_settings() {
///     SETTINGS.register(|s| {
///         s.entry("settings.value", "cat-example", |s| {
///             s.name("Value")
///                 .description("Example using EDITORS provided editor.")
///                 .value(Txt::default())
///         });
///         s.entry("settings.custom", "cat-example", |s| {
///             s.name("Custom")
///                 .description("Example using custom editor.")
///                 .editor_fn(wgt_fn!(|_setting| {
///                     TextInput! {
///                         txt = CONFIG.get("settings.custom", Txt::default());
///                     }
///                 }))
///         });
///     });
/// }
/// ```
///
/// The example above demonstrates how to register a settings category and two values, one using a default editor, the other
/// using a custom editor. When no [`editor_fn`] is set and the [`value`] config is called the [`zng::widget::EDITORS`] service is used
/// to find an editor for the config. The editor closure parameter is a [`Setting`] that is not used in this case, as the editor already
/// binds to the config directly.
///
/// Note that the `name` and `description` accepts variable inputs, in a full app use the [`l10n!`] macro to declare localized metadata.
///
/// In the default `APP` the [`SETTINGS_CMD`] command is handled and shows a [`settings::editor`] window that implements search, edit and reset
/// features. See the [config example] for a full demonstration of settings.
///
/// # Reset
///
/// Restoring settings to default is a common feature, you can simply update the value to a *default*, or with some setup, you can
/// actually remove the config from the user file.
///
/// ```
/// # use zng::config::*;
/// # use zng::config::settings::*;
/// # use zng::prelude::*;
/// #
/// fn load_config() -> Box<dyn FallbackConfigReset> {
///     // user edited config (settings.)
///     let user = JsonConfig::sync(zng::env::config("settings.json"));
///     let default = JsonConfig::read(zng::env::res("default-settings.json"));
///     let settings = FallbackConfig::new(user, default);
///     let settings_ref = settings.clone_boxed();
///
///     // any other configs (Window::save_state for example)
///     let other = JsonConfig::sync(zng::env::config("config.json"));
///
///     CONFIG.load(SwitchConfig::new().with_prefix("settings.", settings).with_prefix("", other));
///
///     settings_ref
/// }
/// fn register_settings(reset: Box<dyn FallbackConfigReset>) {
///     SETTINGS.register(move |s| {
///         s.entry("settings.value", "cat-example", |s| {
///             s.name("Value").value(Txt::default()).reset(reset.clone_boxed(), "settings.")
///         });
///     });
/// }
/// ```
///
/// The example above declares a config system with three files, the relevant ones are `"default-settings.json"` and `"settings.json"`.
/// The default file is deployed with the app resources and is read-only, the user file is created in the user data directory and
/// contains only the custom values set by the user. The [`FallbackConfigReset`] has access to the user file and can remove entries from it.
///
/// The [`FallbackConfigReset::can_reset`] variable tracks the presence of the config in the user file. The default settings widget
/// uses this to show a little reset arrow button that users can click to reset.
///
/// The [`FallbackConfig`] handles entry removal by updating the config variable back to the fallback file entry.
///
/// # Full API
///
/// See [`zng_ext_config::settings`] for the full settings API.
///
/// [`editor_fn`]: crate::config::settings::editor::SettingBuilderEditorExt::editor_fn
/// [`value`]: crate::config::settings::SettingBuilder::value
/// [`Setting`]: crate::config::settings::Setting
/// [`SETTINGS_CMD`]: crate::config::settings::SETTINGS_CMD
/// [`l10n!`]: crate::l10n::l10n
/// [config example]: https://github.com/zng-ui/zng/blob/main/examples/config/src/main.rs
pub mod settings {
    pub use zng_ext_config::settings::{
        CategoriesBuilder, Category, CategoryBuilder, CategoryId, SETTINGS, Setting, SettingBuilder, SettingsBuilder,
    };
    pub use zng_wgt_input::cmd::{SETTINGS_CMD, can_settings, on_pre_settings, on_settings};

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
