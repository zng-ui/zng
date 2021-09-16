use crate::{AppEvent, MultiClickConfig, TextAntiAliasing};
use std::time::Duration;

/// Create a hidden window that listen to Windows config change events.
#[cfg(windows)]
pub(crate) fn config_listener(ctx: &crate::Context<glutin::event_loop::EventLoopProxy<crate::AppEvent>>) -> glutin::window::Window {
    use glutin::window::WindowBuilder;
    use winapi::um::winuser::*;

    use crate::{util, Ev};

    let w = WindowBuilder::new()
        .with_title("config-event-listener")
        .with_visible(false)
        .build(ctx.window_target)
        .unwrap();

    let event_proxy = ctx.event_loop.clone();
    util::set_raw_windows_event_handler(&w, u32::from_ne_bytes(*b"cevl") as _, move |_, msg, wparam, _| {
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

/// Gets the system text anti-aliasing config.
#[cfg(windows)]
pub fn text_aa() -> TextAntiAliasing {
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::winuser::*;

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
    use crate::units::*;
    use winapi::um::winuser::*;

    unsafe {
        MultiClickConfig {
            time: Duration::from_millis(u64::from(GetDoubleClickTime())),
            area: PxSize::new(
                Px(GetSystemMetrics(SM_CXDOUBLECLK).abs() as i32),
                Px(GetSystemMetrics(SM_CYDOUBLECLK).abs() as i32),
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
    use winapi::um::winuser::*;

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
    use winapi::um::winuser::*;

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
