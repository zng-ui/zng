use std::{io::BufRead as _, sync::Arc};

use zng_unit::TimeUnits as _;
use zng_view_api::{
    config::{AnimationsConfig, ColorScheme, FontAntiAliasing, KeyRepeatConfig, LocaleConfig, MultiClickConfig, TouchConfig},
    Event,
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

fn on_change(key: &str, event_loop: &crate::AppEventSender) {
    // println!("{key}"); // to discover keys, uncomment and change the config in system config app.

    match key {
        "/org/gnome/desktop/interface/color-scheme" => {
            let _ = event_loop.send(crate::AppEvent::ColorSchemeConfigChanged);
        }
        "/org/gnome/desktop/peripherals/keyboard/delay" | "/org/gnome/desktop/peripherals/keyboard/repeat-interval" => {
            let _ = event_loop.send(crate::AppEvent::Notify(Event::KeyRepeatConfigChanged(key_repeat_config())));
        }
        _ => {}
    }
}

fn dconf_uint(key: &str) -> Option<u64> {
    let s = dconf(key)?;
    if let Some((_, i)) = s.rsplit_once(' ') {
        match i.parse::<u64>() {
            Ok(i) => Some(i),
            Err(e) => {
                tracing::error!("unexpected value for {key} '{i}', parse error: {e}");
                None
            }
        }
    } else {
        tracing::error!("unexpected value for {key} '{s}'");
        None
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
    let w = match duct::cmd!("dconf", "watch", "/").reader() {
        Ok(w) => w,
        Err(e) => {
            tracing::error!("cannot monitor config, dconf did not spawn, {e}");
            return None;
        }
    };
    let w = Arc::new(w);
    let ww = w.clone();

    std::thread::spawn(move || {
        for line in std::io::BufReader::new(&*w).lines() {
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
        let _ = ww.kill();
    }))
}
