use objc2_app_kit::*;
use zng_unit::TimeUnits as _;
use zng_view_api::config::{AnimationsConfig, ColorScheme, FontAntiAliasing, KeyRepeatConfig, MultiClickConfig, TouchConfig};

pub fn font_aa() -> FontAntiAliasing {
    super::other::font_aa()
}

pub fn multi_click_config() -> MultiClickConfig {
    let mut cfg = MultiClickConfig::default();
    cfg.time = (unsafe { NSEvent::doubleClickInterval() } as f32).ms();
    cfg
}

pub fn animations_config() -> AnimationsConfig {
    super::other::animations_config()
}

pub fn key_repeat_config() -> KeyRepeatConfig {
    let mut cfg = KeyRepeatConfig::default();
    cfg.start_delay = (unsafe { NSEvent::keyRepeatDelay() } as f32).ms();
    cfg.interval = (unsafe { NSEvent::keyRepeatInterval() } as f32).ms();
    cfg
}

pub fn touch_config() -> TouchConfig {
    super::other::touch_config()
}

pub fn color_scheme_config() -> ColorScheme {
    super::other::color_scheme_config()
}

#[cfg(not(windows))]
pub fn locale_config() -> zng_view_api::config::LocaleConfig {
    super::other::locale_config()
}

pub fn spawn_listener(l: crate::AppEventSender) -> Option<Box<dyn FnOnce()>> {
    super::other::spawn_listener(l)
}
