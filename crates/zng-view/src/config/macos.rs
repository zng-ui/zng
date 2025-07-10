use objc2_app_kit::*;
use objc2_foundation::*;
use zng_unit::{Rgba, TimeUnits as _};
use zng_view_api::config::{
    AnimationsConfig, ChromeConfig, ColorScheme, ColorsConfig, FontAntiAliasing, KeyRepeatConfig, MultiClickConfig, TouchConfig,
};

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
    KeyRepeatConfig::new(
        (unsafe { NSEvent::keyRepeatDelay() } as f32).ms(),
        (unsafe { NSEvent::keyRepeatInterval() } as f32).ms(),
    )
}

pub fn touch_config() -> TouchConfig {
    // macOS does not provide touch config
    TouchConfig::default()
}

pub fn colors_config() -> ColorsConfig {
    if macos_major_version() < 11 {
        tracing::warn!("color scheme and accent only implemented for macOS >=11");
        return ColorsConfig::default();
    }

    let appearance = unsafe { NSAppearance::currentDrawingAppearance() };

    // source: winit
    fn dark_appearance_name() -> &'static NSString {
        // Don't use the static `NSAppearanceNameDarkAqua` to allow linking on macOS < 10.14
        ns_string!("NSAppearanceNameDarkAqua")
    }
    let best_match = appearance.bestMatchFromAppearancesWithNames(&NSArray::from_id_slice(&[
        unsafe { NSAppearanceNameAqua.copy() },
        dark_appearance_name().copy(),
    ]));
    let scheme = if let Some(best_match) = best_match {
        if *best_match == *dark_appearance_name() {
            ColorScheme::Dark
        } else {
            ColorScheme::Light
        }
    } else {
        tracing::warn!("failed to determine macOS color scheme");
        ColorScheme::Light
    };

    let accent = unsafe {
        let a = NSColor::controlAccentColor();
        if let Some(a) = a.colorUsingColorSpace(&NSColorSpace::deviceRGBColorSpace()) {
            Rgba::new(a.redComponent(), a.greenComponent(), a.blueComponent(), a.alphaComponent())
        } else {
            tracing::warn!("failed to determine macOS accent color");
            ColorsConfig::default().accent
        }
    };
    ColorsConfig::new(scheme, accent)
}

pub fn locale_config() -> zng_view_api::config::LocaleConfig {
    super::other::locale_config()
}

pub fn chrome_config() -> ChromeConfig {
    ChromeConfig::default()
}

pub fn spawn_listener(l: crate::AppEventSender) -> Option<Box<dyn FnOnce()>> {
    super::other::spawn_listener(l)
}

fn macos_major_version() -> u32 {
    let output = match std::process::Command::new("sw_vers").arg("-productVersion").output() {
        Ok(o) => o,
        Err(e) => {
            tracing::error!("cannot retrieve macos version, {e}");
            return 0;
        }
    };

    if output.status.success() {
        let ver = String::from_utf8_lossy(&output.stdout);
        match ver.trim().split('.').next().unwrap_or("").parse() {
            Ok(v) => v,
            Err(_) => {
                tracing::error!("cannot parse macos version {ver:?}");
                0
            }
        }
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        tracing::error!("cannot retrieve macos version, {}", err.trim());
        0
    }
}
