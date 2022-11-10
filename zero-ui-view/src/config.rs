use crate::{AnimationsConfig, FontAntiAliasing, KeyRepeatConfig, MultiClickConfig};
use std::time::Duration;

/// Create a hidden window that listens to Windows config change events.
#[cfg(windows)]
pub(crate) fn spawn_listener(event_loop: crate::AppEventSender) {
    config_listener(event_loop);
    /*
    std::thread::Builder::new()
    .name("config_listener".to_owned())
    .spawn(move || config_listener(event_loop))
    .unwrap();
    */
}
#[cfg(windows)]
fn config_listener(event_loop: crate::AppEventSender) {
    let _span = tracing::trace_span!("config_listener").entered();

    use crate::AppEvent;
    use windows_sys::core::*;
    use windows_sys::Win32::{Foundation::GetLastError, UI::WindowsAndMessaging::*};
    use zero_ui_view_api::Event;

    use crate::util;

    let class_name: PCWSTR = windows_sys::w!("zero-ui-view::config_listener");

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

    let r = util::set_raw_windows_event_handler(window, u32::from_ne_bytes(*b"cevl") as _, move |_, msg, wparam, _| {
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
                _ => None,
            },
            WM_DISPLAYCHANGE => {
                let _ = event_loop.send(AppEvent::RefreshMonitors);
                Some(0)
            }
            _ => None,
        }
    });
    if !r {
        panic!("error 0x{:x}", unsafe { GetLastError() })
    }
}

/// Gets the system text anti-aliasing config.
#[cfg(windows)]
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
#[cfg(not(windows))]
pub fn font_aa() -> FontAntiAliasing {
    tracing::error!("`text_aa` not implemented for this OS, will use default");
    FontAntiAliasing::Subpixel
}

/// Gets the "double-click" settings.
#[cfg(windows)]
pub fn multi_click_config() -> MultiClickConfig {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    use zero_ui_view_api::units::*;

    unsafe {
        MultiClickConfig {
            time: Duration::from_millis(u64::from(GetDoubleClickTime())),
            area: DipSize::new(
                Dip::new(GetSystemMetrics(SM_CXDOUBLECLK).abs() as i32),
                Dip::new(GetSystemMetrics(SM_CYDOUBLECLK).abs() as i32),
            ),
        }
    }
}

#[cfg(not(windows))]
pub fn multi_click_config() -> MultiClickConfig {
    tracing::error!("`multi_click_config` not implemented for this OS, will use default");
    MultiClickConfig::default()
}

#[cfg(windows)]
pub fn animations_config() -> AnimationsConfig {
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::System::WindowsProgramming::INFINITE;
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
#[cfg(not(windows))]
pub fn animations_config() -> AnimationsConfig {
    // see https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-reduced-motion
    // for other config sources
    tracing::error!("`animations_enabled` not implemented for this OS, will use default");
    AnimationsConfig::default()
}

#[cfg(windows)]
pub fn key_repeat_config() -> KeyRepeatConfig {
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

#[cfg(not(windows))]
pub fn key_repeat_config() -> KeyRepeatConfig {
    tracing::error!("`key_repeat_config` not implemented for this OS, will use default");
    KeyRepeatConfig::default()
}
