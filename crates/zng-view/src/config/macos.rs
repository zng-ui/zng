use objc2_app_kit::*;
use zng_unit::TimeUnits as _;
use zng_view_api::config::{AnimationsConfig, ColorScheme, ColorsConfig, FontAntiAliasing, KeyRepeatConfig, MultiClickConfig, TouchConfig};

pub fn font_aa() -> FontAntiAliasing {
    super::other::font_aa()
}

pub fn multi_click_config() -> MultiClickConfig {
    MultiClickConfig {
        time: (unsafe { NSEvent::doubleClickInterval() } as f32).ms(),
        ..Default::default()
    }
}

pub fn animations_config() -> AnimationsConfig {
    super::other::animations_config()
}

pub fn key_repeat_config() -> KeyRepeatConfig {
    KeyRepeatConfig {
        start_delay: (unsafe { NSEvent::keyRepeatDelay() } as f32).ms(),
        interval: (unsafe { NSEvent::keyRepeatInterval() } as f32).ms(),
    }
}

pub fn touch_config() -> TouchConfig {
    super::other::touch_config()
}

pub fn colors_config() -> ColorsConfig {
    super::other::colors_config() // !!: TODO
}

#[cfg(not(windows))]
pub fn locale_config() -> zng_view_api::config::LocaleConfig {
    super::other::locale_config()
}

pub fn spawn_listener(l: crate::AppEventSender) -> Option<Box<dyn FnOnce()>> {
    super::other::spawn_listener(l)
}
