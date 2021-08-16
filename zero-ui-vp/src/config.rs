use crate::TextAntiAliasing;

/// Create a hidden window that listen to Windows config change events.
#[cfg(windows)]
pub(crate) fn config_listener(ctx: &crate::Context) -> glutin::window::Window {
    use glutin::window::WindowBuilder;

    let w = WindowBuilder::new()
        .with_title("config-event-listener")
        .with_visible(false)
        .build(ctx.window_target)
        .unwrap();

    let event_proxy = ctx.event_loop.clone();
    set_raw_windows_event_handler(&w, u32::from_ne_bytes(*b"cevl") as _, move |_, msg, wparam, _| {
        if msg == winapi::um::winuser::WM_FONTCHANGE {
            let _ = event_proxy.send_event(crate::AppEvent::SystemFontsChanged);
            Some(0)
        } else if msg == winapi::um::winuser::WM_SETTINGCHANGE {
            if wparam == winapi::um::winuser::SPI_GETFONTSMOOTHING as usize
                || wparam == winapi::um::winuser::SPI_GETFONTSMOOTHINGTYPE as usize
            {
                let _ = event_proxy.send_event(crate::AppEvent::SystemTextAaChanged(system_text_aa()));
                Some(0)
            } else {
                None
            }
        } else {
            None
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
        winapi::um::winuser::WM_DESTROY => {
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
pub fn system_text_aa() -> TextAntiAliasing {
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::winuser::{SystemParametersInfoW, FE_FONTSMOOTHINGCLEARTYPE, SPI_GETFONTSMOOTHING, SPI_GETFONTSMOOTHINGTYPE};

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
pub fn system_text_aa() -> TextAntiAliasing {
    // TODO
    TextAntiAliasing::Subpixel
}
