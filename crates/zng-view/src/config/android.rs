use crate::platform::android;
use zng_unit::Rgba;
use zng_view_api::config::{
    AnimationsConfig, ChromeConfig, ColorScheme, ColorsConfig, FontAntiAliasing, KeyRepeatConfig, MultiClickConfig, TouchConfig,
};

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
    let scheme = match android::android_app().config().ui_mode_night() {
        UiModeNight::Yes => ColorScheme::Dark,
        UiModeNight::No => ColorScheme::Light,
        _ => ColorScheme::default(),
    };
    ColorsConfig {
        accent: match scheme {
            ColorScheme::Light => Rgba::new(3, 218, 197, 255),
            ColorScheme::Dark => Rgba::new(187, 134, 252, 255),
        },
        scheme,
    }
}

pub fn locale_config() -> zng_view_api::config::LocaleConfig {
    // sys_locale
    super::other::locale_config()
}

pub fn chrome_config() -> ChromeConfig {
    ChromeConfig {
        prefer_custom: false,
        provided: false,
    }
}

pub fn spawn_listener(l: crate::AppEventSender) -> Option<Box<dyn FnOnce()>> {
    super::other::spawn_listener(l)
}
