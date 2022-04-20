use crate::{FontAntiAliasing, MultiClickConfig};
use std::time::Duration;

/// Create a hidden window that listens to Windows config change events.
#[cfg(windows)]
pub(crate) fn spawn_listener(event_loop: impl crate::AppEventSender) {
    config_listener(event_loop);
    /*
    std::thread::Builder::new()
    .name("config_listener".to_owned())
    .spawn(move || config_listener(event_loop))
    .unwrap();
    */
}
#[cfg(windows)]
fn config_listener(event_loop: impl crate::AppEventSender) {
    let _span = tracing::trace_span!("config_listener").entered();

    use crate::AppEvent;
    use std::ptr;
    use windows::core::*;
    use windows::Win32::{
        Foundation::{GetLastError, LRESULT},
        System::LibraryLoader,
        UI::WindowsAndMessaging::*,
    };
    use zero_ui_view_api::Event;

    use crate::util;

    let class_name: Param<PCWSTR> = "zero-ui-view::config_listener".into_param();

    unsafe {
        let class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: Default::default(),
            lpfnWndProc: Some(util::minimal_wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: LibraryLoader::GetModuleHandleW(None),
            hIcon: Default::default(),
            hCursor: Default::default(), // must be null in order for cursor state to work properly
            hbrBackground: Default::default(),
            lpszMenuName: Default::default(),
            lpszClassName: class_name.abi(),
            hIconSm: Default::default(),
        };

        let r = RegisterClassExW(&class);
        if r == 0 {
            GetLastError().ok().unwrap();
        }
    }

    let window = unsafe {
        let r = CreateWindowExW(
            WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_LAYERED | WS_EX_TOOLWINDOW,
            class_name.abi(),
            None,
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            LibraryLoader::GetModuleHandleW(None),
            ptr::null(),
        );
        if r.0 == 0 {
            GetLastError().ok().unwrap();
        }
        r
    };

    let r = util::set_raw_windows_event_handler(window, u32::from_ne_bytes(*b"cevl") as _, move |_, msg, wparam, _| {
        let notify = |ev| {
            let _ = event_loop.send(AppEvent::Notify(ev));
            Some(LRESULT(0))
        };
        match msg {
            WM_FONTCHANGE => notify(Event::FontsChanged),
            WM_SETTINGCHANGE => match SYSTEM_PARAMETERS_INFO_ACTION(wparam.0 as _) {
                SPI_SETFONTSMOOTHING | SPI_SETFONTSMOOTHINGTYPE => notify(Event::FontAaChanged(font_aa())),
                SPI_SETDOUBLECLICKTIME | SPI_SETDOUBLECLKWIDTH | SPI_SETDOUBLECLKHEIGHT => {
                    notify(Event::MultiClickConfigChanged(multi_click_config()))
                }
                SPI_SETCLIENTAREAANIMATION => notify(Event::AnimationsEnabledChanged(animations_enabled())),
                SPI_SETKEYBOARDDELAY => notify(Event::KeyRepeatDelayChanged(key_repeat_delay())),
                _ => None,
            },
            WM_DISPLAYCHANGE => {
                let _ = event_loop.send(AppEvent::RefreshMonitors);
                Some(LRESULT(0))
            }
            _ => None,
        }
    });
    if !r {
        unsafe { GetLastError().ok().unwrap() }
    }
}

/// Gets the system text anti-aliasing config.
#[cfg(windows)]
pub fn font_aa() -> FontAntiAliasing {
    use windows::Win32::Foundation::{GetLastError, BOOL};
    use windows::Win32::UI::WindowsAndMessaging::*;

    unsafe {
        let mut enabled = 0;
        let mut smoothing_type: u32 = 0;

        if SystemParametersInfoW(
            SPI_GETFONTSMOOTHING,
            0,
            &mut enabled as *mut _ as *mut _,
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        ) == BOOL(0)
        {
            tracing::error!("SPI_GETFONTSMOOTHING error: {:?}", GetLastError());
            return FontAntiAliasing::Mono;
        }
        if enabled == 0 {
            return FontAntiAliasing::Mono;
        }

        if SystemParametersInfoW(
            SPI_GETFONTSMOOTHINGTYPE,
            0,
            &mut smoothing_type as *mut _ as *mut _,
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        ) == BOOL(0)
        {
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
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    use windows::Win32::UI::WindowsAndMessaging::*;
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
pub fn animations_enabled() -> bool {
    use windows::Win32::Foundation::{GetLastError, BOOL};
    use windows::Win32::UI::WindowsAndMessaging::*;

    unsafe {
        let mut enabled = true;

        if SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            &mut enabled as *mut _ as *mut _,
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        ) == BOOL(0)
        {
            tracing::error!("SPI_GETCLIENTAREAANIMATION error: {:?}", GetLastError());
            return true;
        }

        enabled
    }
}
#[cfg(not(windows))]
pub fn animations_enabled() -> bool {
    // see https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-reduced-motion
    // for other config sources
    tracing::error!("`animations_enabled` not implemented for this OS, will use default");
    true
}

#[cfg(windows)]
pub fn key_repeat_delay() -> Duration {
    use windows::Win32::Foundation::{GetLastError, BOOL};
    use windows::Win32::UI::WindowsAndMessaging::*;

    unsafe {
        let mut index = 0;

        if SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            &mut index as *mut _ as *mut _,
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        ) == BOOL(0)
        {
            tracing::error!("SPI_GETCLIENTAREAANIMATION error: {:?}", GetLastError());
            return Duration::from_millis(600);
        }

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
}

#[cfg(not(windows))]
pub fn key_repeat_delay() -> Duration {
    tracing::error!("`key_repeat_delay` not implemented for this OS, will use default");
    Duration::from_millis(600)
}
