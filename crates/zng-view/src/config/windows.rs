use zng_unit::Rgba;
use zng_view_api::config::{
    AnimationsConfig, ChromeConfig, ColorScheme, ColorsConfig, FontAntiAliasing, KeyRepeatConfig, LocaleConfig, MultiClickConfig,
    TouchConfig,
};

/// Create a hidden window that listens to Windows config change events.
pub(crate) fn spawn_listener(event_loop: crate::AppEventSender) -> Option<Box<dyn FnOnce()>> {
    config_listener(event_loop);
    /*
    std::thread::Builder::new()
    .name("config_listener".to_owned())
    .spawn(move || config_listener(event_loop))
    .unwrap();
    */
    None
}
fn config_listener(event_loop: crate::AppEventSender) {
    let _span = tracing::trace_span!("config_listener").entered();

    use crate::AppEvent;
    use windows_sys::{
        core::*,
        Win32::{
            Foundation::GetLastError,
            System::{
                Power::{RegisterPowerSettingNotification, UnregisterPowerSettingNotification},
                SystemServices::GUID_SESSION_DISPLAY_STATUS,
            },
            UI::WindowsAndMessaging::*,
        },
    };
    use zng_view_api::Event;

    use crate::util;

    let class_name: PCWSTR = windows_sys::w!("zng-view::config_listener");

    unsafe {
        let class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: Default::default(),
            lpfnWndProc: Some(util::minimal_wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: util::get_instance_handle(),
            hIcon: Default::default(),
            hCursor: Default::default(), // must be null in order for cursor state to work properly
            hbrBackground: Default::default(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name,
            hIconSm: Default::default(),
        };

        let r = RegisterClassExW(&class);
        if r == 0 {
            panic!("error 0x{:x}", GetLastError())
        }
    }

    let window = unsafe {
        let r = CreateWindowExW(
            WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_LAYERED | WS_EX_TOOLWINDOW,
            class_name,
            std::ptr::null(),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            0,
            0,
            util::get_instance_handle(),
            std::ptr::null(),
        );
        if r == 0 {
            panic!("error 0x{:x}", GetLastError())
        }
        r
    };

    let mut power_listener_handle = unsafe {
        //
        RegisterPowerSettingNotification(window, &GUID_SESSION_DISPLAY_STATUS, 0)
    };

    let mut colors_cfg = colors_config();

    let r = util::set_raw_windows_event_handler(window, u32::from_ne_bytes(*b"cevl") as _, move |_, msg, wparam, lparam| {
        let notify = |ev| {
            let _ = event_loop.send(AppEvent::Notify(ev));
            Some(0)
        };
        match msg {
            WM_FONTCHANGE => notify(Event::FontsChanged),
            WM_SETTINGCHANGE => match wparam as _ {
                SPI_SETFONTSMOOTHING | SPI_SETFONTSMOOTHINGTYPE => notify(Event::FontAaChanged(font_aa())),
                SPI_SETDOUBLECLICKTIME | SPI_SETDOUBLECLKWIDTH | SPI_SETDOUBLECLKHEIGHT => {
                    notify(Event::MultiClickConfigChanged(multi_click_config()))
                }
                SPI_SETCLIENTAREAANIMATION => notify(Event::AnimationsConfigChanged(animations_config())),
                SPI_SETKEYBOARDDELAY | SPI_SETKEYBOARDSPEED => notify(Event::KeyRepeatConfigChanged(key_repeat_config())),
                0 if lparam != 0 => {
                    let p_str = lparam as windows_sys::core::PSTR;
                    let b_str = unsafe {
                        let len = windows_sys::Win32::Globalization::lstrlenA(p_str);
                        std::slice::from_raw_parts(p_str, len as _)
                    };
                    match b_str {
                        b"i" | b"intl" => notify(Event::LocaleChanged(locale_config())),
                        _ => None,
                    }
                }
                _ => {
                    // winit does this too for color scheme
                    let cfg = colors_config();
                    if cfg != colors_cfg {
                        colors_cfg = cfg;
                        notify(Event::ColorsConfigChanged(cfg));
                        Some(0)
                    } else {
                        None
                    }
                }
            },
            WM_DISPLAYCHANGE => {
                let _ = event_loop.send(AppEvent::RefreshMonitors);
                Some(0)
            }
            WM_POWERBROADCAST => {
                if wparam == PBT_POWERSETTINGCHANGE as usize {
                    let _ = event_loop.send(AppEvent::MonitorPowerChanged);
                }
                Some(0)
            }
            WM_DESTROY => {
                let h = std::mem::take(&mut power_listener_handle);
                if h != 0 {
                    unsafe {
                        UnregisterPowerSettingNotification(h);
                    };
                }
                None
            }
            _ => None,
        }
    });
    if !r {
        panic!("error 0x{:x}", unsafe { GetLastError() })
    }
}

/// Gets the system text anti-aliasing config.
pub fn font_aa() -> FontAntiAliasing {
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    unsafe {
        let mut enabled = 0;
        let mut smoothing_type: u32 = 0;

        if SystemParametersInfoW(SPI_GETFONTSMOOTHING, 0, &mut enabled as *mut _ as *mut _, 0) == 0 {
            tracing::error!("SPI_GETFONTSMOOTHING error: {:?}", GetLastError());
            return FontAntiAliasing::Mono;
        }
        if enabled == 0 {
            return FontAntiAliasing::Mono;
        }

        if SystemParametersInfoW(SPI_GETFONTSMOOTHINGTYPE, 0, &mut smoothing_type as *mut _ as *mut _, 0) == 0 {
            tracing::error!("SPI_GETFONTSMOOTHINGTYPE error: {:?}", GetLastError());
            return FontAntiAliasing::Mono;
        }

        if smoothing_type == FE_FONTSMOOTHINGCLEARTYPE {
            FontAntiAliasing::Subpixel
        } else {
            FontAntiAliasing::Alpha
        }
    }
}

/// Gets the "double-click" settings.
pub fn multi_click_config() -> MultiClickConfig {
    use std::time::Duration;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    use zng_unit::*;

    unsafe {
        MultiClickConfig {
            time: Duration::from_millis(u64::from(GetDoubleClickTime())),
            area: DipSize::new(
                Dip::new(GetSystemMetrics(SM_CXDOUBLECLK).abs()),
                Dip::new(GetSystemMetrics(SM_CYDOUBLECLK).abs()),
            ),
        }
    }
}

pub fn animations_config() -> AnimationsConfig {
    use std::time::Duration;
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::System::Threading::INFINITE;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    let enabled = unsafe {
        let mut enabled = true;

        if SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, &mut enabled as *mut _ as *mut _, 0) == 0 {
            tracing::error!("SPI_GETCLIENTAREAANIMATION error: {:?}", GetLastError());
            enabled = true;
        }

        enabled
    };

    let blink_time = unsafe { GetCaretBlinkTime() };
    let blink_time = if blink_time == INFINITE {
        Duration::MAX
    } else {
        Duration::from_millis(blink_time as _)
    };

    let blink_timeout = unsafe {
        let mut timeout = 5000;

        if SystemParametersInfoW(SPI_GETCARETTIMEOUT, 0, &mut timeout as *mut _ as *mut _, 0) == 0 {
            tracing::error!("SPI_GETCARETTIMEOUT error: {:?}", GetLastError());
            timeout = 5000;
        }

        timeout
    };
    let blink_timeout = if blink_timeout == INFINITE {
        Duration::MAX
    } else {
        Duration::from_millis(blink_timeout as _)
    };

    AnimationsConfig {
        enabled,
        caret_blink_interval: blink_time,
        caret_blink_timeout: blink_timeout,
    }
}

pub fn key_repeat_config() -> KeyRepeatConfig {
    use std::time::Duration;
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    let start_delay = unsafe {
        let mut index = 0;

        if SystemParametersInfoW(SPI_GETKEYBOARDDELAY, 0, &mut index as *mut _ as *mut _, 0) == 0 {
            tracing::error!("SPI_GETKEYBOARDDELAY error: {:?}", GetLastError());
            Duration::from_millis(600)
        } else {
            /*
                ..which is a value in the range from 0 (approximately 250 ms delay) through 3 (approximately 1 second delay).
                The actual delay associated with each value may vary depending on the hardware.

                source: SPI_GETKEYBOARDDELAY entry in https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-systemparametersinfow
            */
            Duration::from_millis(match index {
                0 => 250,
                1 => 500,
                2 => 750,
                3 => 1000,
                _ => 600,
            })
        }
    };

    let speed = unsafe {
        let mut index = 0;

        if SystemParametersInfoW(SPI_GETKEYBOARDSPEED, 0, &mut index as *mut _ as *mut _, 0) == 0 {
            tracing::error!("SPI_GETKEYBOARDSPEED error: {:?}", GetLastError());
            Duration::from_millis(100)
        } else {
            /*
                ..which is a value in the range from 0 (approximately 2.5 repetitions per second) through 31
                (approximately 30 repetitions per second). The actual repeat rates are hardware-dependent and may
                vary from a linear scale by as much as 20%

                source: SPI_GETKEYBOARDSPEED entry in https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-systemparametersinfow
            */
            let min = 0.0;
            let max = 31.0;
            let t_min = 2.5;
            let t_max = 30.0;
            let i = index as f32;
            let t = (i - min) / (max - min) * (t_max - t_min) + t_min;

            Duration::from_secs_f32(1.0 / t)
        }
    };

    KeyRepeatConfig {
        start_delay,
        interval: speed,
    }
}

pub fn touch_config() -> TouchConfig {
    super::other::touch_config()
}

pub fn chrome_config() -> ChromeConfig {
    ChromeConfig::default()
}

pub fn colors_config() -> ColorsConfig {
    // source: winit

    use std::mem;

    use windows_sys::{
        core::PCSTR,
        Win32::{
            System::LibraryLoader::{GetProcAddress, LoadLibraryA},
            UI::{
                Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTA},
                WindowsAndMessaging::{SystemParametersInfoA, SPI_GETHIGHCONTRAST},
            },
        },
    };

    fn should_apps_use_dark_mode() -> bool {
        type ShouldAppsUseDarkMode = unsafe extern "system" fn() -> bool;
        const UXTHEME_SHOULDAPPSUSEDARKMODE_ORDINAL: PCSTR = 132 as PCSTR;

        let module = unsafe { LoadLibraryA("uxtheme.dll\0".as_ptr()) };

        if module == 0 {
            return false;
        }

        let handle = unsafe { GetProcAddress(module, UXTHEME_SHOULDAPPSUSEDARKMODE_ORDINAL) };

        if let Some(f) = handle {
            unsafe {
                let f: ShouldAppsUseDarkMode = mem::transmute(f);
                f()
            }
        } else {
            false
        }
    }

    fn is_high_contrast() -> bool {
        use std::ptr;

        let mut hc = HIGHCONTRASTA {
            cbSize: 0,
            dwFlags: 0,
            lpszDefaultScheme: ptr::null_mut(),
        };

        let ok = unsafe { SystemParametersInfoA(SPI_GETHIGHCONTRAST, std::mem::size_of_val(&hc) as _, &mut hc as *mut _ as _, 0) };

        ok != 0 && hc.dwFlags & HCF_HIGHCONTRASTON == HCF_HIGHCONTRASTON
    }

    let scheme = if should_apps_use_dark_mode() && !is_high_contrast() {
        ColorScheme::Dark
    } else {
        ColorScheme::Light
    };

    let accent = windows::UI::ViewManagement::UISettings::new()
        .ok()
        .and_then(|ui| ui.GetColorValue(windows::UI::ViewManagement::UIColorType::Accent).ok())
        .map(|a| Rgba::new(a.R, a.G, a.B, a.A))
        .unwrap_or_else(|| ColorsConfig::default().accent);

    ColorsConfig { scheme, accent }
}

pub(crate) fn locale_config() -> LocaleConfig {
    // sys_locale returns only one lang, this code adapted from `whoami` crate and Firefox.

    use windows::System::UserProfile::GlobalizationPreferences;
    use windows_sys::Win32::{
        Foundation::FALSE,
        Globalization::{GetUserPreferredUILanguages, MUI_LANGUAGE_NAME},
    };
    use zng_txt::Txt;

    // Try newer WinRT COM API (Windows8+)
    if let Ok(r) = GlobalizationPreferences::Languages() {
        let r: Vec<_> = r.into_iter().map(|l| Txt::from_str(&l.to_string_lossy())).collect();
        if !r.is_empty() {
            return LocaleConfig { langs: r };
        }
    }

    let mut num_languages = 0;
    let mut buffer_size = 0;
    let mut buffer;

    unsafe {
        if GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut num_languages, std::ptr::null_mut(), &mut buffer_size) == FALSE {
            return LocaleConfig::default();
        }

        buffer = Vec::with_capacity(buffer_size as usize);

        if GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut num_languages, buffer.as_mut_ptr(), &mut buffer_size) == FALSE {
            return LocaleConfig::default();
        }

        buffer.set_len(buffer_size as usize);
    }

    // We know it ends in two null characters.
    buffer.pop();
    buffer.pop();

    LocaleConfig {
        langs: String::from_utf16_lossy(&buffer).split('\0').map(Txt::from_str).collect(),
    }
}
