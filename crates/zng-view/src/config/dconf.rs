use std::{io::BufRead as _, time::Duration};

use zng_unit::TimeUnits as _;
use zng_view_api::{
    config::{AnimationsConfig, ColorScheme, FontAntiAliasing, KeyRepeatConfig, LocaleConfig, MultiClickConfig, TouchConfig},
    Event,
};

use crate::AppEvent;

pub fn font_aa() -> FontAntiAliasing {
    super::other::font_aa()
}

pub fn multi_click_config() -> MultiClickConfig {
    let mut cfg = MultiClickConfig::default();
    if let Some(d) = dconf_uint("/org/gnome/desktop/peripherals/mouse/double-click") {
        cfg.time = d.ms();
    }
    cfg
}

pub fn animations_config() -> AnimationsConfig {
    let mut cfg = AnimationsConfig::default();
    if let Some(e) = dconf_bool("/org/gnome/desktop/interface/enable-animations") {
        cfg.enabled = e;
    }
    if let Some(d) = dconf_uint("/org/gnome/desktop/interface/cursor-blink-time") {
        cfg.caret_blink_interval = d.ms();
    }
    if let Some(e) = dconf_bool("/org/gnome/desktop/interface/cursor-blink") {
        if !e {
            cfg.caret_blink_interval = Duration::MAX;
        }
    }
    cfg
}

pub fn key_repeat_config() -> KeyRepeatConfig {
    let mut cfg = KeyRepeatConfig::default();
    if let Some(d) = dconf_uint("/org/gnome/desktop/peripherals/keyboard/delay") {
        cfg.start_delay = d.ms();
    }
    if let Some(d) = dconf_uint("/org/gnome/desktop/peripherals/keyboard/repeat-interval") {
        cfg.interval = d.ms();
    }
    cfg
}

pub fn touch_config() -> TouchConfig {
    super::other::touch_config()
}

pub fn color_scheme_config() -> ColorScheme {
    match dconf("/org/gnome/desktop/interface/color-scheme") {
        Some(cs) => {
            if cs.contains("dark") {
                ColorScheme::Dark
            } else {
                ColorScheme::Light
            }
        }
        None => ColorScheme::Light,
    }
}

pub fn locale_config() -> LocaleConfig {
    // sys_locale
    super::other::locale_config()
}

fn on_change(key: &str, s: &crate::AppEventSender) {
    // println!("{key}"); // to discover keys, uncomment and change the config in system config app.

    match key {
        "/org/gnome/desktop/interface/color-scheme" => {
            let _ = s.send(AppEvent::ColorSchemeConfigChanged);
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

fn dconf_bool(key: &str) -> Option<bool> {
    let s = dconf(key)?;
    match s.parse::<bool>() {
        Ok(b) => Some(b),
        Err(e) => {
            tracing::error!("unexpected value for {key} '{s}', parse error: {e}");
            None
        }
    }
}

fn dconf_uint(key: &str) -> Option<u64> {
    let s = dconf(key)?;
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

fn dconf(key: &str) -> Option<String> {
    let out = std::process::Command::new("dconf").arg("read").arg(key).output();
    match out {
        Ok(s) => {
            if s.status.success() {
                Some(String::from_utf8_lossy(&s.stdout).trim().to_owned())
            } else {
                let e = String::from_utf8_lossy(&s.stderr);
                tracing::error!("dconf read {key} error, {}", e.lines().next().unwrap_or_default());
                None
            }
        }
        Err(e) => {
            tracing::error!("cannot run dconf, {e}");
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
    std::thread::spawn(move || {
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
    });

    Some(Box::new(move || {
        let _ = w.kill();
        let _ = w.wait();
    }))
}
