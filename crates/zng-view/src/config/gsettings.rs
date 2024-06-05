use zng_view_api::config::{AnimationsConfig, ColorScheme, FontAntiAliasing, KeyRepeatConfig, LocaleConfig, MultiClickConfig, TouchConfig};

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

pub fn color_scheme_config() -> ColorScheme {
    super::other::color_scheme_config()
}

pub fn locale_config() -> LocaleConfig {
    // sys_locale
    super::other::locale_config()
}

pub fn spawn_listener(event_loop: crate::AppEventSender) {
    super::other::spawn_listener(event_loop)
}
