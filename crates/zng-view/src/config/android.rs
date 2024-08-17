use crate::platform::android;
use objc2_app_kit::*;
use objc2_foundation::*;
use zng_unit::{Rgba, TimeUnits as _};
use zng_view_api::config::{AnimationsConfig, ColorScheme, ColorsConfig, FontAntiAliasing, KeyRepeatConfig, MultiClickConfig, TouchConfig};

pub fn font_aa() -> FontAntiAliasing {
    super::other::font_aa()
}

pub fn multi_click_config() -> MultiClickConfig {
    super::other::multi_click_config()
}

pub fn animations_config() -> AnimationsConfig {
    super::other::animations_config()
}

pub fn key_repeat_config() -> KeyRepeatConfig {
    super::other::key_repeat_config()
}

pub fn touch_config() -> TouchConfig {
    super::other::touch_config()
}

pub fn colors_config() -> ColorsConfig {
    use ndk::configuration::UiModeNight;
    ColorsConfig {
        scheme: match android::android_app().config().ui_mode_night() {
            UiModeNight::Any => ColorScheme::default(),
            UiModeNight::Yes => ColorScheme::Dark,
            UiModeNight::No => ColorScheme::Light,
        }
        // accent:
        ..ColorsConfig::default(),
    }
}

pub fn locale_config() -> zng_view_api::config::LocaleConfig {
    // sys_locale
    super::other::locale_config()
}

pub fn spawn_listener(l: crate::AppEventSender) -> Option<Box<dyn FnOnce()>> {
    super::other::spawn_listener(l)
}
