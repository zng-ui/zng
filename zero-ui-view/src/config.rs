use crate::{MultiClickConfig, TextAntiAliasing};
use std::time::Duration;

#[cfg(windows)]
use crate::AppEvent;

/// Create a hidden window that listen to Windows config change events.
#[cfg(windows)]
pub(crate) fn config_listener(
    event_loop: impl crate::AppEventSender,
    window_target: &glutin::event_loop::EventLoopWindowTarget<AppEvent>,
) -> glutin::window::Window {
    tracing::trace!("config_listener");

    use glutin::window::WindowBuilder;
    use winapi::um::winuser::*;
    use zero_ui_view_api::Event;

    use crate::util;

    let w = WindowBuilder::new()
        .with_title("config-event-listener")
        .with_visible(false)
        .build(window_target)
        .unwrap();

    util::set_raw_windows_event_handler(&w, u32::from_ne_bytes(*b"cevl") as _, move |_, msg, wparam, _| {
        let notify = |ev| {
            let _ = event_loop.send(AppEvent::Notify(ev));
            Some(0)
        };
        match msg {
            WM_FONTCHANGE => notify(Event::FontsChanged),
            WM_SETTINGCHANGE => match wparam as u32 {
                SPI_SETFONTSMOOTHING | SPI_SETFONTSMOOTHINGTYPE => notify(Event::TextAaChanged(text_aa())),
                SPI_SETDOUBLECLICKTIME | SPI_SETDOUBLECLKWIDTH | SPI_SETDOUBLECLKHEIGHT => {
                    notify(Event::MultiClickConfigChanged(multi_click_config()))
                }
                SPI_SETCLIENTAREAANIMATION => notify(Event::AnimationEnabledChanged(animation_enabled())),
                SPI_SETKEYBOARDDELAY => notify(Event::KeyRepeatDelayChanged(key_repeat_delay())),
                _ => None,
            },
            WM_DISPLAYCHANGE => {
                let _ = event_loop.send(AppEvent::RefreshMonitors);
                Some(0)
            }
            _ => None,
        }
    });

    w
}

/// Gets the system text anti-aliasing config.
#[cfg(windows)]
pub fn text_aa() -> TextAntiAliasing {
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::winuser::*;

    unsafe {
        let mut enabled = 0;
        let mut smoothing_type: u32 = 0;

        if SystemParametersInfoW(SPI_GETFONTSMOOTHING, 0, &mut enabled as *mut _ as *mut _, 0) == 0 {
            tracing::error!("SPI_GETFONTSMOOTHING error: {:X}", GetLastError());
            return TextAntiAliasing::Mono;
        }
        if enabled == 0 {
            return TextAntiAliasing::Mono;
        }

        if SystemParametersInfoW(SPI_GETFONTSMOOTHINGTYPE, 0, &mut smoothing_type as *mut _ as *mut _, 0) == 0 {
            tracing::error!("SPI_GETFONTSMOOTHINGTYPE error: {:X}", GetLastError());
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
    tracing::error!("`text_aa` not implemented for this OS, will use default");
    TextAntiAliasing::Subpixel
}

/// Gets the "double-click" settings.
#[cfg(windows)]
pub fn multi_click_config() -> MultiClickConfig {
    use winapi::um::winuser::*;
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
pub fn animation_enabled() -> bool {
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::winuser::*;

    unsafe {
        let mut enabled = true;

        if SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, &mut enabled as *mut _ as *mut _, 0) == 0 {
            tracing::error!("SPI_GETCLIENTAREAANIMATION error: {:X}", GetLastError());
            return true;
        }

        enabled
    }
}
#[cfg(not(windows))]
pub fn animation_enabled() -> bool {
    tracing::error!("`animation_enabled` not implemented for this OS, will use default");
    true
}

#[cfg(windows)]
pub fn key_repeat_delay() -> Duration {
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::winuser::*;

    unsafe {
        let mut index = 0;

        if SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, &mut index as *mut _ as *mut _, 0) == 0 {
            tracing::error!("SPI_GETCLIENTAREAANIMATION error: {:X}", GetLastError());
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
