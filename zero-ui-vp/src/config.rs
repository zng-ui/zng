use crate::{AppEvent, MultiClickConfig, TextAntiAliasing};
use std::time::Duration;
use winapi::um::winuser::*;

/// Create a hidden window that listen to Windows config change events.
#[cfg(windows)]
pub(crate) fn config_listener(ctx: &crate::Context) -> glutin::window::Window {
    use glutin::window::WindowBuilder;

    use crate::Ev;

    let w = WindowBuilder::new()
        .with_title("config-event-listener")
        .with_visible(false)
        .build(ctx.window_target)
        .unwrap();

    let event_proxy = ctx.event_loop.clone();
    set_raw_windows_event_handler(&w, u32::from_ne_bytes(*b"cevl") as _, move |_, msg, wparam, _| {
        let notify = |ev| {
            let _ = event_proxy.send_event(AppEvent::Notify(ev));
            Some(0)
        };
        match msg {
            WM_FONTCHANGE => notify(Ev::FontsChanged),
            WM_SETTINGCHANGE => match wparam as u32 {
                SPI_SETFONTSMOOTHING | SPI_SETFONTSMOOTHINGTYPE => notify(Ev::TextAaChanged(text_aa())),
                SPI_SETDOUBLECLICKTIME | SPI_SETDOUBLECLKWIDTH | SPI_SETDOUBLECLKHEIGHT => {
                    notify(Ev::MultiClickConfigChanged(multi_click_config()))
                }
                SPI_SETCLIENTAREAANIMATION => notify(Ev::AnimationEnabledChanged(animation_enabled())),
                SPI_SETKEYBOARDDELAY => notify(Ev::KeyRepeatDelayChanged(key_repeat_delay())),
                _ => None,
            },
            WM_DISPLAYCHANGE => {
                let _ = event_proxy.send_event(AppEvent::RefreshMonitors);
                Some(0)
            }
            _ => None,
        }
    });

    w
}

/// Sets a window subclass that calls a raw event handler.
///
/// Use this to receive Windows OS events not covered in [`raw_events`].
///
/// Returns if adding a subclass handler succeeded.
///
/// # Handler
///
/// The handler inputs are the first 4 arguments of a [`SUBCLASSPROC`].
/// You can use closure capture to include extra data.
///
/// The handler must return `Some(LRESULT)` to stop the propagation of a specific message.
///
/// The handler is dropped after it receives the `WM_DESTROY` message.
///
/// # Panics
///
/// Panics in headless mode.
///
/// [`raw_events`]: crate::app::raw_events
/// [`SUBCLASSPROC`]: https://docs.microsoft.com/en-us/windows/win32/api/commctrl/nc-commctrl-subclassproc
#[cfg(windows)]
pub fn set_raw_windows_event_handler<
    H: FnMut(
            winapi::shared::windef::HWND,
            winapi::shared::minwindef::UINT,
            winapi::shared::minwindef::WPARAM,
            winapi::shared::minwindef::LPARAM,
        ) -> Option<winapi::shared::minwindef::LRESULT>
        + 'static,
>(
    window: &glutin::window::Window,
    subclass_id: winapi::shared::basetsd::UINT_PTR,
    handler: H,
) -> bool {
    use glutin::platform::windows::WindowExtWindows;

    let hwnd = window.hwnd() as winapi::shared::windef::HWND;
    let data = Box::new(handler);
    unsafe {
        winapi::um::commctrl::SetWindowSubclass(
            hwnd,
            Some(subclass_raw_event_proc::<H>),
            subclass_id,
            Box::into_raw(data) as winapi::shared::basetsd::DWORD_PTR,
        ) != 0
    }
}
#[cfg(windows)]
unsafe extern "system" fn subclass_raw_event_proc<
    H: FnMut(
            winapi::shared::windef::HWND,
            winapi::shared::minwindef::UINT,
            winapi::shared::minwindef::WPARAM,
            winapi::shared::minwindef::LPARAM,
        ) -> Option<winapi::shared::minwindef::LRESULT>
        + 'static,
>(
    hwnd: winapi::shared::windef::HWND,
    msg: winapi::shared::minwindef::UINT,
    wparam: winapi::shared::minwindef::WPARAM,
    lparam: winapi::shared::minwindef::LPARAM,
    _id: winapi::shared::basetsd::UINT_PTR,
    data: winapi::shared::basetsd::DWORD_PTR,
) -> winapi::shared::minwindef::LRESULT {
    match msg {
        WM_DESTROY => {
            // last call and cleanup.
            let mut handler = Box::from_raw(data as *mut H);
            handler(hwnd, msg, wparam, lparam).unwrap_or_default()
        }

        msg => {
            let handler = &mut *(data as *mut H);
            if let Some(r) = handler(hwnd, msg, wparam, lparam) {
                r
            } else {
                winapi::um::commctrl::DefSubclassProc(hwnd, msg, wparam, lparam)
            }
        }
    }
}

/// Gets the system text anti-aliasing config.
#[cfg(windows)]
pub fn text_aa() -> TextAntiAliasing {
    use winapi::um::errhandlingapi::GetLastError;

    unsafe {
        let mut enabled = 0;
        let mut smoothing_type: u32 = 0;

        if SystemParametersInfoW(SPI_GETFONTSMOOTHING, 0, &mut enabled as *mut _ as *mut _, 0) == 0 {
            log::error!("SPI_GETFONTSMOOTHING error: {:X}", GetLastError());
            return TextAntiAliasing::Mono;
        }
        if enabled == 0 {
            return TextAntiAliasing::Mono;
        }

        if SystemParametersInfoW(SPI_GETFONTSMOOTHINGTYPE, 0, &mut smoothing_type as *mut _ as *mut _, 0) == 0 {
            log::error!("SPI_GETFONTSMOOTHINGTYPE error: {:X}", GetLastError());
            return TextAntiAliasing::Mono;
        }

        if smoothing_type == FE_FONTSMOOTHINGCLEARTYPE {
            TextAntiAliasing::Subpixel
        } else {
            TextAntiAliasing::Alpha
        }
    }
}
#[cfg(not(windows))]
pub fn text_aa() -> TextAntiAliasing {
    // TODO
    TextAntiAliasing::Subpixel
}

/// Gets the "double-click" settings.
#[cfg(target_os = "windows")]
pub fn multi_click_config() -> MultiClickConfig {
    unsafe {
        MultiClickConfig {
            time: Duration::from_millis(u64::from(GetDoubleClickTime())),
            area: (
                GetSystemMetrics(SM_CXDOUBLECLK).abs() as u32,
                GetSystemMetrics(SM_CYDOUBLECLK).abs() as u32,
            ),
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn multi_click_time() -> MultiClickConfig {
    // TODO
    // https://stackoverflow.com/questions/50868129/how-to-get-double-click-time-interval-value-programmatically-on-linux
    // https://developer.apple.com/documentation/appkit/nsevent/1532495-mouseevent
    MultiClickConfig::default()
}

#[cfg(windows)]
pub fn animation_enabled() -> bool {
    use winapi::um::errhandlingapi::GetLastError;

    unsafe {
        let mut enabled = true;

        if SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, &mut enabled as *mut _ as *mut _, 0) == 0 {
            log::error!("SPI_GETCLIENTAREAANIMATION error: {:X}", GetLastError());
            return true;
        }

        enabled
    }
}
#[cfg(not(windows))]
pub fn animation_enabled() -> bool {
    true
}

#[cfg(windows)]
pub fn key_repeat_delay() -> Duration {
    use winapi::um::errhandlingapi::GetLastError;

    unsafe {
        let mut index = 0;

        if SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, &mut index as *mut _ as *mut _, 0) == 0 {
            log::error!("SPI_GETCLIENTAREAANIMATION error: {:X}", GetLastError());
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
    Duration::from_millis(600)
}
