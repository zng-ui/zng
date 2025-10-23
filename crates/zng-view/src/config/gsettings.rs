use std::{io::BufRead as _, time::Duration};

use zng_unit::{Rgba, TimeUnits as _};
use zng_view_api::{
    Event,
    config::{
        AnimationsConfig, ChromeConfig, ColorScheme, ColorsConfig, FontAntiAliasing, KeyRepeatConfig, LocaleConfig, MultiClickConfig,
        TouchConfig,
    },
};

use crate::AppEvent;

pub fn font_aa() -> FontAntiAliasing {
    super::other::font_aa()
}

pub fn multi_click_config() -> MultiClickConfig {
    let mut cfg = MultiClickConfig::default();
    if let Some(d) = gsettings_uint("org.gnome.desktop.peripherals.mouse", "double-click") {
        cfg.time = d.ms();
    }
    cfg
}

pub fn animations_config() -> AnimationsConfig {
    let mut cfg = AnimationsConfig::default();
    if let Some(e) = gsettings_bool("org.gnome.desktop.interface", "enable-animations") {
        cfg.enabled = e;
    }
    if let Some(d) = gsettings_uint("org.gnome.desktop.interface", "cursor-blink-time") {
        cfg.caret_blink_interval = (d / 2).ms();
    }
    if let Some(e) = gsettings_bool("org.gnome.desktop.interface", "cursor-blink")
        && !e
    {
        cfg.caret_blink_interval = Duration::MAX;
    }

    cfg
}

pub fn key_repeat_config() -> KeyRepeatConfig {
    let mut cfg = KeyRepeatConfig::default();
    if let Some(d) = gsettings_uint("org.gnome.desktop.peripherals.keyboard", "delay") {
        cfg.start_delay = d.ms();
    }
    if let Some(d) = gsettings_uint("org.gnome.desktop.peripherals.keyboard", "repeat-interval") {
        cfg.interval = d.ms();
    }
    cfg
}

pub fn touch_config() -> TouchConfig {
    // Gnome does not provide touch config
    TouchConfig::default()
}

pub fn colors_config() -> ColorsConfig {
    let scheme = match gsettings("org.gnome.desktop.interface", "color-scheme") {
        Some(cs) => {
            if cs.contains("dark") {
                ColorScheme::Dark
            } else {
                ColorScheme::Light
            }
        }
        None => ColorScheme::Light,
    };

    // the color value is not in any config, need to parse theme name
    let theme = gsettings("org.gnome.desktop.interface", "gtk-theme");
    let theme = match theme.as_ref() {
        Some(n) => n
            .trim_matches('\'')
            .split('-')
            .find(|p| !["Yaru", "dark"].contains(p))
            .unwrap_or(""),
        None => "?",
    };
    // see https://github.com/ubuntu/yaru/blob/6e28865e0ce55c0f95d17a25871618b1660e97b5/common/accent-colors.scss.in
    let accent = match theme {
        "" => Rgba::new(233, 84, 32, 255),               // #E95420
        "bark" => Rgba::new(120, 120, 89, 255),          // #787859
        "sage" => Rgba::new(101, 123, 105, 255),         // #657b69
        "olive" => Rgba::new(75, 133, 1, 255),           // #4B8501
        "viridian" => Rgba::new(3, 135, 91, 255),        // #03875b
        "prussiangreen" => Rgba::new(48, 130, 128, 255), // #308280
        "blue" => Rgba::new(0, 115, 229, 255),           // #0073E5
        "purple" => Rgba::new(119, 100, 216, 255),       // #7764d8
        "magenta" => Rgba::new(179, 76, 179, 255),       // #b34cb3
        "red" => Rgba::new(218, 52, 80, 255),            // #DA3450
        _ => ColorsConfig::default().accent,
    };

    ColorsConfig::new(scheme, accent)
}

pub fn locale_config() -> LocaleConfig {
    // sys_locale
    super::other::locale_config()
}

pub fn chrome_config() -> ChromeConfig {
    let is_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
    // VSCode/Electron, it changes XDG_CURRENT_DESKTOP to "Unity" and sets ORIGINAL_XDG_CURRENT_DESKTOP,
    // so running from VSCode gets the wrong value.
    let is_gnome = std::env::var("ORIGINAL_XDG_CURRENT_DESKTOP")
        .or_else(|_| std::env::var("XDG_CURRENT_DESKTOP"))
        .is_ok_and(|val| val.contains("GNOME"));
    ChromeConfig::new(is_gnome, !(is_wayland && is_gnome))
}

fn on_change(key: &str, s: &crate::AppEventSender) {
    // println!("{key}"); // to discover keys, uncomment and change the config in system config app.

    match key {
        "/org/gnome/desktop/interface/color-scheme" | "/org/gnome/desktop/interface/gtk-theme" => {
            let _ = s.send(AppEvent::Notify(Event::ColorsConfigChanged(colors_config())));
        }
        "/org/gnome/desktop/peripherals/keyboard/delay" | "/org/gnome/desktop/peripherals/keyboard/repeat-interval" => {
            let _ = s.send(AppEvent::Notify(Event::KeyRepeatConfigChanged(key_repeat_config())));
        }
        "/org/gnome/desktop/peripherals/mouse/double-click" => {
            let _ = s.send(AppEvent::Notify(Event::MultiClickConfigChanged(multi_click_config())));
        }
        "/org/gnome/desktop/interface/enable-animations"
        | "/org/gnome/desktop/interface/cursor-blink-time"
        | "/org/gnome/desktop/interface/cursor-blink" => {
            let _ = s.send(AppEvent::Notify(Event::AnimationsConfigChanged(animations_config())));
        }
        _ => {}
    }
}

fn gsettings_bool(schema: &str, key: &str) -> Option<bool> {
    let s = gsettings(schema, key)?;
    match s.parse::<bool>() {
        Ok(b) => Some(b),
        Err(e) => {
            tracing::error!("unexpected value for {key} '{s}', parse error: {e}");
            None
        }
    }
}

fn gsettings_uint(schema: &str, key: &str) -> Option<u64> {
    let s = gsettings(schema, key)?;
    let s = if let Some((t, i)) = s.rsplit_once(' ') {
        if !t.starts_with("uint") {
            tracing::error!("unexpected value for {key} '{s}'");
            return None;
        }
        i
    } else {
        s.as_str()
    };
    match s.parse::<u64>() {
        Ok(i) => Some(i),
        Err(e) => {
            tracing::error!("unexpected value for {key} '{s}', parse error: {e}");
            None
        }
    }
}

fn gsettings(schema: &str, key: &str) -> Option<String> {
    let out = std::process::Command::new("gsettings").arg("get").arg(schema).arg(key).output();
    match out {
        Ok(s) => {
            if s.status.success() {
                Some(String::from_utf8_lossy(&s.stdout).trim().to_owned())
            } else {
                let e = String::from_utf8_lossy(&s.stderr);
                tracing::error!("gsettings read {key} error, {}", e.lines().next().unwrap_or_default());
                None
            }
        }
        Err(e) => {
            tracing::error!("cannot run gsettings, {e}");
            None
        }
    }
}

pub fn spawn_listener(event_loop: crate::AppEventSender) -> Option<Box<dyn FnOnce()>> {
    let mut w = std::process::Command::new("dconf");
    w.arg("watch")
        .arg("/")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());

    let mut w = match w.spawn() {
        Ok(w) => w,
        Err(e) => {
            tracing::error!("cannot monitor config, dconf did not spawn, {e}");
            return None;
        }
    };
    let stdout = w.stdout.take().unwrap();
    std::thread::Builder::new()
        .name("dconf-watcher".into())
        .stack_size(256 * 1024)
        .spawn(move || {
            for line in std::io::BufReader::new(stdout).lines() {
                match line {
                    Ok(l) => {
                        if l.starts_with('/') {
                            on_change(&l, &event_loop);
                        }
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
        })
        .expect("failed to spawn thread");

    Some(Box::new(move || {
        let _ = w.kill();
        let _ = w.wait();
    }))
}
