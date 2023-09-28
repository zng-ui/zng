use std::{cell::Cell, sync::Arc};

use rayon::ThreadPoolBuilder;
use winit::{event::ElementState, monitor::MonitorHandle};
use zero_ui_view_api::access::AccessNodeId;
use zero_ui_view_api::clipboard as clipboard_api;
use zero_ui_view_api::{
    config::ColorScheme,
    keyboard::{Key, KeyCode, KeyState, NativeKeyCode},
    mouse::{ButtonState, MouseButton, MouseScrollDelta},
    touch::{TouchForce, TouchPhase},
    units::*,
    window::{CursorIcon, MonitorInfo, VideoMode},
};

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
pub fn set_raw_windows_event_handler<H>(hwnd: windows_sys::Win32::Foundation::HWND, subclass_id: usize, handler: H) -> bool
where
    H: FnMut(
            windows_sys::Win32::Foundation::HWND,
            u32,
            windows_sys::Win32::Foundation::WPARAM,
            windows_sys::Win32::Foundation::LPARAM,
        ) -> Option<windows_sys::Win32::Foundation::LRESULT>
        + 'static,
{
    let data = Box::new(handler);
    unsafe {
        windows_sys::Win32::UI::Shell::SetWindowSubclass(hwnd, Some(subclass_raw_event_proc::<H>), subclass_id, Box::into_raw(data) as _)
            != 0
    }
}

#[cfg(windows)]
unsafe extern "system" fn subclass_raw_event_proc<H>(
    hwnd: windows_sys::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows_sys::Win32::Foundation::WPARAM,
    lparam: windows_sys::Win32::Foundation::LPARAM,
    _id: usize,
    data: usize,
) -> windows_sys::Win32::Foundation::LRESULT
where
    H: FnMut(
            windows_sys::Win32::Foundation::HWND,
            u32,
            windows_sys::Win32::Foundation::WPARAM,
            windows_sys::Win32::Foundation::LPARAM,
        ) -> Option<windows_sys::Win32::Foundation::LRESULT>
        + 'static,
{
    use windows_sys::Win32::UI::WindowsAndMessaging::WM_DESTROY;
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
                windows_sys::Win32::UI::Shell::DefSubclassProc(hwnd, msg, wparam, lparam)
            }
        }
    }
}

#[cfg(windows)]
pub(crate) fn unregister_raw_input() {
    use windows_sys::Win32::Devices::HumanInterfaceDevice::{HID_USAGE_GENERIC_KEYBOARD, HID_USAGE_GENERIC_MOUSE, HID_USAGE_PAGE_GENERIC};
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::Input::{RAWINPUTDEVICE, RIDEV_REMOVE};

    let flags = RIDEV_REMOVE;

    let devices: [RAWINPUTDEVICE; 2] = [
        RAWINPUTDEVICE {
            usUsagePage: HID_USAGE_PAGE_GENERIC,
            usUsage: HID_USAGE_GENERIC_MOUSE,
            dwFlags: flags,
            hwndTarget: HWND::default(),
        },
        RAWINPUTDEVICE {
            usUsagePage: HID_USAGE_PAGE_GENERIC,
            usUsage: HID_USAGE_GENERIC_KEYBOARD,
            dwFlags: flags,
            hwndTarget: HWND::default(),
        },
    ];

    let device_size = std::mem::size_of::<RAWINPUTDEVICE>() as _;

    let ok = unsafe { windows_sys::Win32::UI::Input::RegisterRawInputDevices(devices.as_ptr(), devices.len() as _, device_size) != 0 };

    if !ok {
        let e = unsafe { windows_sys::Win32::Foundation::GetLastError() };
        panic!("failed `unregister_raw_input`, {e:?}");
    }
}

/// Conversion from `winit` logical units to [`Dip`].
///
/// All conversions are 1 to 1.
pub(crate) trait WinitToDip {
    /// `Self` equivalent in [`Dip`] units.
    type AsDip;

    /// Returns `self` in [`Dip`] units.
    fn to_dip(self) -> Self::AsDip;
}

/// Conversion from `winit` physical units to [`Dip`].
///
/// All conversions are 1 to 1.
pub(crate) trait WinitToPx {
    /// `Self` equivalent in [`Px`] units.
    type AsPx;

    /// Returns `self` in [`Px`] units.
    fn to_px(self) -> Self::AsPx;
}

/// Conversion from [`Dip`] to `winit` logical units.
pub(crate) trait DipToWinit {
    /// `Self` equivalent in `winit` logical units.
    type AsWinit;

    /// Returns `self` in `winit` logical units.
    fn to_winit(self) -> Self::AsWinit;
}

/// Conversion from [`Px`] to `winit` physical units.
pub(crate) trait PxToWinit {
    /// `Self` equivalent in `winit` logical units.
    type AsWinit;

    /// Returns `self` in `winit` logical units.
    fn to_winit(self) -> Self::AsWinit;
}

impl PxToWinit for PxSize {
    type AsWinit = winit::dpi::PhysicalSize<u32>;

    fn to_winit(self) -> Self::AsWinit {
        winit::dpi::PhysicalSize::new(self.width.0 as _, self.height.0 as _)
    }
}
impl PxToWinit for PxPoint {
    type AsWinit = winit::dpi::PhysicalPosition<i32>;

    fn to_winit(self) -> Self::AsWinit {
        winit::dpi::PhysicalPosition::new(self.x.0, self.y.0)
    }
}

impl DipToWinit for DipPoint {
    type AsWinit = winit::dpi::LogicalPosition<f32>;

    fn to_winit(self) -> Self::AsWinit {
        winit::dpi::LogicalPosition::new(self.x.to_f32(), self.y.to_f32())
    }
}

impl WinitToDip for winit::dpi::LogicalPosition<f64> {
    type AsDip = DipPoint;

    fn to_dip(self) -> Self::AsDip {
        DipPoint::new(Dip::new_f32(self.x as f32), Dip::new_f32(self.y as f32))
    }
}

impl WinitToPx for winit::dpi::PhysicalPosition<i32> {
    type AsPx = PxPoint;

    fn to_px(self) -> Self::AsPx {
        PxPoint::new(Px(self.x), Px(self.y))
    }
}

impl WinitToPx for winit::dpi::PhysicalPosition<f64> {
    type AsPx = PxPoint;

    fn to_px(self) -> Self::AsPx {
        PxPoint::new(Px(self.x as i32), Px(self.y as i32))
    }
}

impl DipToWinit for DipSize {
    type AsWinit = winit::dpi::LogicalSize<f32>;

    fn to_winit(self) -> Self::AsWinit {
        winit::dpi::LogicalSize::new(self.width.to_f32(), self.height.to_f32())
    }
}

impl WinitToDip for winit::dpi::LogicalSize<f64> {
    type AsDip = DipSize;

    fn to_dip(self) -> Self::AsDip {
        DipSize::new(Dip::new_f32(self.width as f32), Dip::new_f32(self.height as f32))
    }
}

impl WinitToPx for winit::dpi::PhysicalSize<u32> {
    type AsPx = PxSize;

    fn to_px(self) -> Self::AsPx {
        PxSize::new(Px(self.width as i32), Px(self.height as i32))
    }
}

pub trait CursorToWinit {
    fn to_winit(self) -> winit::window::CursorIcon;
}
impl CursorToWinit for CursorIcon {
    fn to_winit(self) -> winit::window::CursorIcon {
        use winit::window::CursorIcon::*;
        match self {
            CursorIcon::Default => Default,
            CursorIcon::Crosshair => Crosshair,
            CursorIcon::Hand => Hand,
            CursorIcon::Arrow => Arrow,
            CursorIcon::Move => Move,
            CursorIcon::Text => Text,
            CursorIcon::Wait => Wait,
            CursorIcon::Help => Help,
            CursorIcon::Progress => Progress,
            CursorIcon::NotAllowed => NotAllowed,
            CursorIcon::ContextMenu => ContextMenu,
            CursorIcon::Cell => Cell,
            CursorIcon::VerticalText => VerticalText,
            CursorIcon::Alias => Alias,
            CursorIcon::Copy => Copy,
            CursorIcon::NoDrop => NoDrop,
            CursorIcon::Grab => Grab,
            CursorIcon::Grabbing => Grabbing,
            CursorIcon::AllScroll => AllScroll,
            CursorIcon::ZoomIn => ZoomIn,
            CursorIcon::ZoomOut => ZoomOut,
            CursorIcon::EResize => EResize,
            CursorIcon::NResize => NResize,
            CursorIcon::NeResize => NeResize,
            CursorIcon::NwResize => NwResize,
            CursorIcon::SResize => SResize,
            CursorIcon::SeResize => SeResize,
            CursorIcon::SwResize => SwResize,
            CursorIcon::WResize => WResize,
            CursorIcon::EwResize => EwResize,
            CursorIcon::NsResize => NsResize,
            CursorIcon::NeswResize => NeswResize,
            CursorIcon::NwseResize => NwseResize,
            CursorIcon::ColResize => ColResize,
            CursorIcon::RowResize => RowResize,
        }
    }
}

pub(crate) fn monitor_handle_to_info(handle: &MonitorHandle) -> MonitorInfo {
    let position = handle.position().to_px();
    let size = handle.size().to_px();
    MonitorInfo {
        name: handle.name().unwrap_or_default(),
        position,
        size,
        scale_factor: handle.scale_factor() as f32,
        video_modes: handle.video_modes().map(glutin_video_mode_to_video_mode).collect(),
        is_primary: false,
    }
}

pub(crate) fn glutin_video_mode_to_video_mode(v: winit::monitor::VideoMode) -> VideoMode {
    let size = v.size();
    VideoMode {
        size: PxSize::new(Px(size.width as i32), Px(size.height as i32)),
        bit_depth: v.bit_depth(),
        refresh_rate: v.refresh_rate_millihertz(),
    }
}

pub(crate) fn element_state_to_key_state(s: ElementState) -> KeyState {
    match s {
        ElementState::Pressed => KeyState::Pressed,
        ElementState::Released => KeyState::Released,
    }
}

pub(crate) fn element_state_to_button_state(s: ElementState) -> ButtonState {
    match s {
        ElementState::Pressed => ButtonState::Pressed,
        ElementState::Released => ButtonState::Released,
    }
}

pub(crate) fn winit_mouse_wheel_delta_to_zui(w: winit::event::MouseScrollDelta) -> MouseScrollDelta {
    match w {
        winit::event::MouseScrollDelta::LineDelta(x, y) => MouseScrollDelta::LineDelta(x, y),
        winit::event::MouseScrollDelta::PixelDelta(d) => MouseScrollDelta::PixelDelta(d.x as f32, d.y as f32),
    }
}

pub(crate) fn winit_touch_phase_to_zui(w: winit::event::TouchPhase) -> TouchPhase {
    match w {
        winit::event::TouchPhase::Started => TouchPhase::Start,
        winit::event::TouchPhase::Moved => TouchPhase::Move,
        winit::event::TouchPhase::Ended => TouchPhase::End,
        winit::event::TouchPhase::Cancelled => TouchPhase::Cancel,
    }
}

pub(crate) fn winit_force_to_zui(f: winit::event::Force) -> TouchForce {
    match f {
        winit::event::Force::Calibrated {
            force,
            max_possible_force,
            altitude_angle,
        } => TouchForce::Calibrated {
            force,
            max_possible_force,
            altitude_angle,
        },
        winit::event::Force::Normalized(f) => TouchForce::Normalized(f),
    }
}

pub(crate) fn winit_mouse_button_to_zui(b: winit::event::MouseButton) -> MouseButton {
    match b {
        winit::event::MouseButton::Left => MouseButton::Left,
        winit::event::MouseButton::Right => MouseButton::Right,
        winit::event::MouseButton::Middle => MouseButton::Middle,
        winit::event::MouseButton::Other(btn) => MouseButton::Other(btn),
    }
}

pub(crate) fn winit_theme_to_zui(t: winit::window::Theme) -> ColorScheme {
    match t {
        winit::window::Theme::Light => ColorScheme::Light,
        winit::window::Theme::Dark => ColorScheme::Dark,
    }
}

use winit::event::VirtualKeyCode as VKey;
pub(crate) fn v_key_to_key(v_key: VKey) -> Key {
    match v_key {
        // this is temporary until winit releases (chars are localized by the system)
        VKey::Key1 => Key::Char('1'),
        VKey::Key2 => Key::Char('2'),
        VKey::Key3 => Key::Char('3'),
        VKey::Key4 => Key::Char('4'),
        VKey::Key5 => Key::Char('5'),
        VKey::Key6 => Key::Char('6'),
        VKey::Key7 => Key::Char('7'),
        VKey::Key8 => Key::Char('8'),
        VKey::Key9 => Key::Char('9'),
        VKey::Key0 => Key::Char('0'),
        VKey::A => Key::Char('A'),
        VKey::B => Key::Char('B'),
        VKey::C => Key::Char('C'),
        VKey::D => Key::Char('D'),
        VKey::E => Key::Char('E'),
        VKey::F => Key::Char('F'),
        VKey::G => Key::Char('G'),
        VKey::H => Key::Char('H'),
        VKey::I => Key::Char('I'),
        VKey::J => Key::Char('J'),
        VKey::K => Key::Char('K'),
        VKey::L => Key::Char('L'),
        VKey::M => Key::Char('M'),
        VKey::N => Key::Char('N'),
        VKey::O => Key::Char('O'),
        VKey::P => Key::Char('P'),
        VKey::Q => Key::Char('Q'),
        VKey::R => Key::Char('R'),
        VKey::S => Key::Char('S'),
        VKey::T => Key::Char('T'),
        VKey::U => Key::Char('U'),
        VKey::V => Key::Char('V'),
        VKey::W => Key::Char('W'),
        VKey::X => Key::Char('X'),
        VKey::Y => Key::Char('Y'),
        VKey::Z => Key::Char('Z'),
        VKey::Escape => Key::Escape,
        VKey::F1 => Key::F1,
        VKey::F2 => Key::F2,
        VKey::F3 => Key::F3,
        VKey::F4 => Key::F4,
        VKey::F5 => Key::F5,
        VKey::F6 => Key::F6,
        VKey::F7 => Key::F7,
        VKey::F8 => Key::F8,
        VKey::F9 => Key::F9,
        VKey::F10 => Key::F10,
        VKey::F11 => Key::F11,
        VKey::F12 => Key::F12,
        VKey::F13 => Key::F13,
        VKey::F14 => Key::F14,
        VKey::F15 => Key::F15,
        VKey::F16 => Key::F16,
        VKey::F17 => Key::F17,
        VKey::F18 => Key::F18,
        VKey::F19 => Key::F19,
        VKey::F20 => Key::F20,
        VKey::F21 => Key::F21,
        VKey::F22 => Key::F22,
        VKey::F23 => Key::F23,
        VKey::F24 => Key::F24,
        VKey::Snapshot => Key::PrintScreen,
        VKey::Scroll => Key::ScrollLock,
        VKey::Pause => Key::Pause,
        VKey::Insert => Key::Insert,
        VKey::Home => Key::Home,
        VKey::Delete => Key::Delete,
        VKey::End => Key::End,
        VKey::PageDown => Key::PageDown,
        VKey::PageUp => Key::PageUp,
        VKey::Left => Key::ArrowLeft,
        VKey::Up => Key::ArrowUp,
        VKey::Right => Key::ArrowRight,
        VKey::Down => Key::ArrowDown,
        VKey::Back => Key::Backspace,
        VKey::Return => Key::Enter,
        VKey::Space => Key::Space,
        VKey::Compose => Key::Compose,
        VKey::Caret => Key::Unidentified,
        VKey::Numlock => Key::NumLock,
        VKey::Numpad0 => Key::Char('0'),
        VKey::Numpad1 => Key::Char('1'),
        VKey::Numpad2 => Key::Char('2'),
        VKey::Numpad3 => Key::Char('3'),
        VKey::Numpad4 => Key::Char('4'),
        VKey::Numpad5 => Key::Char('5'),
        VKey::Numpad6 => Key::Char('6'),
        VKey::Numpad7 => Key::Char('7'),
        VKey::Numpad8 => Key::Char('8'),
        VKey::Numpad9 => Key::Char('9'),
        VKey::NumpadAdd => Key::Char('+'),
        VKey::NumpadDivide => Key::Char('/'),
        VKey::NumpadDecimal => Key::Char('.'),
        VKey::NumpadComma => Key::Char(','),
        VKey::NumpadEnter => Key::Enter,
        VKey::NumpadEquals => Key::Char('='),
        VKey::NumpadMultiply => Key::Char('*'),
        VKey::NumpadSubtract => Key::Char('-'),
        VKey::AbntC1 => Key::Char('/'),
        VKey::AbntC2 => Key::Char('ç'),
        VKey::Apostrophe => Key::Char('\''),
        VKey::Apps => Key::AppSwitch,
        VKey::Asterisk => Key::Char('*'),
        VKey::At => Key::Char('@'),
        VKey::Ax => Key::Unidentified,
        VKey::Backslash => Key::Char('\\'),
        VKey::Calculator => Key::LaunchApplication2,
        VKey::Capital => Key::Unidentified,
        VKey::Colon => Key::Char(':'),
        VKey::Comma => Key::Char(','),
        VKey::Convert => Key::Convert,
        VKey::Equals => Key::Char('='),
        VKey::Grave => Key::Char('`'),
        VKey::Kana => Key::KanaMode,
        VKey::Kanji => Key::KanjiMode,
        VKey::LAlt => Key::Alt,
        VKey::LBracket => Key::Char('['),
        VKey::LControl => Key::Ctrl,
        VKey::LShift => Key::Shift,
        VKey::LWin => Key::Super,
        VKey::Mail => Key::LaunchMail,
        VKey::MediaSelect => Key::MediaApps,
        VKey::MediaStop => Key::MediaStop,
        VKey::Minus => Key::Char('-'),
        VKey::Mute => Key::AudioVolumeMute,
        VKey::MyComputer => Key::Unidentified,
        VKey::NavigateForward => Key::NavigateNext,
        VKey::NavigateBackward => Key::NavigatePrevious,
        VKey::NextTrack => Key::MediaTrackNext,
        VKey::NoConvert => Key::NonConvert,
        VKey::OEM102 => Key::Unidentified,
        VKey::Period => Key::Char('.'),
        VKey::PlayPause => Key::MediaPlayPause,
        VKey::Plus => Key::Char('+'),
        VKey::Power => Key::Power,
        VKey::PrevTrack => Key::MediaTrackPrevious,
        VKey::RAlt => Key::AltGraph,
        VKey::RBracket => Key::Char(']'),
        VKey::RControl => Key::Ctrl,
        VKey::RShift => Key::Shift,
        VKey::RWin => Key::Super,
        VKey::Semicolon => Key::Char(';'),
        VKey::Slash => Key::Char('/'),
        VKey::Sleep => Key::Standby,
        VKey::Stop => Key::MediaStop,
        VKey::Sysrq => Key::PrintScreen,
        VKey::Tab => Key::Tab,
        VKey::Underline => Key::Char('_'),
        VKey::Unlabeled => Key::Unidentified,
        VKey::VolumeDown => Key::AudioVolumeDown,
        VKey::VolumeUp => Key::AudioVolumeUp,
        VKey::Wake => Key::WakeUp,
        VKey::WebBack => Key::BrowserBack,
        VKey::WebFavorites => Key::BrowserFavorites,
        VKey::WebForward => Key::BrowserForward,
        VKey::WebHome => Key::BrowserHome,
        VKey::WebRefresh => Key::BrowserRefresh,
        VKey::WebSearch => Key::BrowserSearch,
        VKey::WebStop => Key::BrowserStop,
        VKey::Yen => Key::Char('¥'),
        VKey::Copy => Key::Copy,
        VKey::Paste => Key::Paste,
        VKey::Cut => Key::Cut,
    }
}

pub(crate) fn scan_code_to_key(scan: winit::event::ScanCode) -> KeyCode {
    let k = if cfg!(windows) {
        NativeKeyCode::Windows(scan as _)
    } else if cfg!(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    )) {
        NativeKeyCode::Xkb(scan as _)
    } else if cfg!(target_os = "macos") {
        NativeKeyCode::MacOS(scan as _)
    } else if cfg!(target_os = "android") {
        NativeKeyCode::Android(scan as _)
    } else {
        NativeKeyCode::Unidentified
    };
    KeyCode::Unidentified(k)
}

thread_local! {
    static SUPPRESS: Cell<bool> = const { Cell::new(false) };
}

/// If `true` our custom panic hook must not log anything.
pub(crate) fn suppress_panic() -> bool {
    SUPPRESS.with(|s| s.get())
}

/// Like [`std::panic::catch_unwind`], but flags [`suppress_panic`] for our custom panic hook.
pub(crate) fn catch_supress<T>(f: impl FnOnce() -> T + std::panic::UnwindSafe) -> std::thread::Result<T> {
    SUPPRESS.with(|s| s.set(true));
    let _cleanup = RunOnDrop::new(|| SUPPRESS.with(|s| s.set(false)));
    std::panic::catch_unwind(f)
}

pub(crate) fn panic_msg(payload: &dyn std::any::Any) -> &str {
    match payload.downcast_ref::<&'static str>() {
        Some(s) => s,
        None => match payload.downcast_ref::<String>() {
            Some(s) => &s[..],
            None => "Box<dyn Any>",
        },
    }
}

struct RunOnDrop<F: FnOnce()>(Option<F>);
impl<F: FnOnce()> RunOnDrop<F> {
    pub fn new(clean: F) -> Self {
        RunOnDrop(Some(clean))
    }
}
impl<F: FnOnce()> Drop for RunOnDrop<F> {
    fn drop(&mut self) {
        if let Some(clean) = self.0.take() {
            clean();
        }
    }
}

#[cfg(windows)]
pub(crate) extern "system" fn minimal_wndproc(
    window: windows_sys::Win32::Foundation::HWND,
    message: u32,
    wparam: windows_sys::Win32::Foundation::WPARAM,
    lparam: windows_sys::Win32::Foundation::LPARAM,
) -> windows_sys::Win32::Foundation::LRESULT {
    unsafe { windows_sys::Win32::UI::WindowsAndMessaging::DefWindowProcW(window, message, wparam, lparam) }
}

#[cfg(windows)]
pub fn get_instance_handle() -> winit::platform::windows::HINSTANCE {
    // Gets the instance handle by taking the address of the
    // pseudo-variable created by the Microsoft linker:
    // https://devblogs.microsoft.com/oldnewthing/20041025-00/?p=37483

    // This is preferred over GetModuleHandle(NULL) because it also works in DLLs:
    // https://stackoverflow.com/questions/21718027/getmodulehandlenull-vs-hinstance

    extern "C" {
        static __ImageBase: windows_sys::Win32::System::SystemServices::IMAGE_DOS_HEADER;
    }

    unsafe { (&__ImageBase) as *const _ as _ }
}

#[cfg(windows)]
pub mod taskbar_com {
    // copied from winit

    #![allow(non_snake_case)]
    #![allow(non_upper_case_globals)]

    use std::ffi::c_void;

    use windows_sys::{
        core::{IUnknown, GUID, HRESULT},
        Win32::Foundation::{BOOL, HWND},
    };

    #[repr(C)]
    pub struct IUnknownVtbl {
        pub QueryInterface: unsafe extern "system" fn(This: *mut IUnknown, riid: *const GUID, ppvObject: *mut *mut c_void) -> HRESULT,
        pub AddRef: unsafe extern "system" fn(This: *mut IUnknown) -> u32,
        pub Release: unsafe extern "system" fn(This: *mut IUnknown) -> u32,
    }

    #[repr(C)]
    pub struct ITaskbarListVtbl {
        pub parent: IUnknownVtbl,
        pub HrInit: unsafe extern "system" fn(This: *mut ITaskbarList) -> HRESULT,
        pub AddTab: unsafe extern "system" fn(This: *mut ITaskbarList, hwnd: HWND) -> HRESULT,
        pub DeleteTab: unsafe extern "system" fn(This: *mut ITaskbarList, hwnd: HWND) -> HRESULT,
        pub ActivateTab: unsafe extern "system" fn(This: *mut ITaskbarList, hwnd: HWND) -> HRESULT,
        pub SetActiveAlt: unsafe extern "system" fn(This: *mut ITaskbarList, hwnd: HWND) -> HRESULT,
    }

    #[repr(C)]
    pub struct ITaskbarList {
        pub lpVtbl: *const ITaskbarListVtbl,
    }

    #[repr(C)]
    pub struct ITaskbarList2Vtbl {
        pub parent: ITaskbarListVtbl,
        pub MarkFullscreenWindow: unsafe extern "system" fn(This: *mut ITaskbarList2, hwnd: HWND, fFullscreen: BOOL) -> HRESULT,
    }

    #[repr(C)]
    pub struct ITaskbarList2 {
        pub lpVtbl: *const ITaskbarList2Vtbl,
    }

    pub const CLSID_TaskbarList: GUID = GUID {
        data1: 0x56fdf344,
        data2: 0xfd6d,
        data3: 0x11d0,
        data4: [0x95, 0x8a, 0x00, 0x60, 0x97, 0xc9, 0xa0, 0x90],
    };

    pub const IID_ITaskbarList2: GUID = GUID {
        data1: 0x602d4995,
        data2: 0xb13a,
        data3: 0x429b,
        data4: [0xa6, 0x6e, 0x19, 0x35, 0xe4, 0x4f, 0x43, 0x17],
    };
}

pub(crate) fn wr_workers() -> Arc<rayon::ThreadPool> {
    // see: webrender/src/renderer/init.rs#L547
    //
    // we need the workers instance before renderer init for the extensions, but this
    // means that we removed some Webrender profiler instrumentation.
    let worker = ThreadPoolBuilder::new().thread_name(|idx| format!("WRWorker#{}", idx)).build();
    Arc::new(worker.unwrap())
}

#[cfg(not(windows))]
pub(crate) fn arboard_to_clip(e: arboard::Error) -> clipboard_api::ClipboardError {
    match e {
        arboard::Error::ContentNotAvailable => clipboard_api::ClipboardError::NotFound,
        arboard::Error::ClipboardNotSupported => clipboard_api::ClipboardError::NotSupported,
        e => clipboard_api::ClipboardError::Other(format!("{e:?}")),
    }
}

#[cfg(windows)]
pub(crate) fn clipboard_win_to_clip(e: clipboard_win::SystemError) -> clipboard_api::ClipboardError {
    if e == clipboard_win::SystemError::unimplemented() {
        clipboard_api::ClipboardError::NotSupported
    } else {
        clipboard_api::ClipboardError::Other(format!("{e:?}"))
    }
}

pub(crate) fn accesskit_to_event(
    window_id: zero_ui_view_api::window::WindowId,
    request: accesskit::ActionRequest,
) -> Option<zero_ui_view_api::Event> {
    use accesskit::Action;
    use zero_ui_view_api::access::*;

    let target = AccessNodeId(request.target.0);

    Some(zero_ui_view_api::Event::AccessCommand {
        window: window_id,
        target,
        command: match request.action {
            Action::Default => AccessCommand::Click(true),
            Action::ShowContextMenu => AccessCommand::Click(false),
            Action::Focus => AccessCommand::Focus(true),
            Action::Blur => AccessCommand::Focus(false),
            Action::Collapse => AccessCommand::SetExpanded(false),
            Action::Expand => AccessCommand::SetExpanded(true),
            Action::CustomAction => return None,
            Action::Decrement => AccessCommand::Increment(-1),
            Action::Increment => AccessCommand::Increment(1),
            Action::HideTooltip => AccessCommand::SetToolTipVis(false),
            Action::ShowTooltip => AccessCommand::SetToolTipVis(true),
            Action::ReplaceSelectedText => {
                if let Some(accesskit::ActionData::Value(s)) = request.data {
                    AccessCommand::ReplaceSelectedText(s.to_string())
                } else {
                    AccessCommand::ReplaceSelectedText(String::new())
                }
            }
            Action::ScrollBackward => AccessCommand::Scroll(ScrollCommand::PageUp),
            Action::ScrollForward => AccessCommand::Scroll(ScrollCommand::PageDown),

            Action::ScrollDown => AccessCommand::Scroll(ScrollCommand::PageDown),
            Action::ScrollLeft => AccessCommand::Scroll(ScrollCommand::PageLeft),
            Action::ScrollRight => AccessCommand::Scroll(ScrollCommand::PageRight),
            Action::ScrollUp => AccessCommand::Scroll(ScrollCommand::PageUp),
            Action::ScrollIntoView => {
                if let Some(accesskit::ActionData::ScrollTargetRect(_r)) = request.data {
                    return None; // TODO, figure out units
                } else {
                    AccessCommand::Scroll(ScrollCommand::ScrollTo)
                }
            }
            Action::ScrollToPoint => {
                if let Some(accesskit::ActionData::ScrollToPoint(_p)) = request.data {
                    return None; // TODO, units
                } else {
                    return None;
                }
            }
            Action::SetScrollOffset => {
                if let Some(accesskit::ActionData::SetScrollOffset(_o)) = request.data {
                    return None; // TODO, value range
                } else {
                    return None;
                }
            }
            Action::SetTextSelection => {
                if let Some(accesskit::ActionData::SetTextSelection(s)) = request.data {
                    AccessCommand::SelectText {
                        start: (AccessNodeId(s.anchor.node.0), s.anchor.character_index),
                        caret: (AccessNodeId(s.focus.node.0), s.focus.character_index),
                    }
                } else {
                    return None;
                }
            }
            Action::SetSequentialFocusNavigationStartingPoint => AccessCommand::SetNextTabStart,
            Action::SetValue => match request.data {
                Some(accesskit::ActionData::Value(s)) => AccessCommand::SetString(s.to_string()),
                Some(accesskit::ActionData::NumericValue(n)) => AccessCommand::SetNumber(n),
                _ => return None,
            },
        },
    })
}

pub(crate) fn access_tree_init(root_id: AccessNodeId) -> accesskit::TreeUpdate {
    let root_id = access_id_to_kit(root_id);
    let mut classes = accesskit::NodeClassSet::new();
    let root = accesskit::NodeBuilder::new(accesskit::Role::Application).build(&mut classes);

    accesskit::TreeUpdate {
        nodes: vec![(root_id, root)],
        tree: Some(accesskit::Tree::new(root_id)),
        focus: root_id,
    }
}

pub(crate) fn access_tree_update_to_kit(update: zero_ui_view_api::access::AccessTreeUpdate) -> accesskit::TreeUpdate {
    let mut class_set = accesskit::NodeClassSet::new();
    let mut nodes = Vec::with_capacity(update.updates.iter().map(|t| t.len()).sum());

    for update in update.updates {
        access_node_to_kit(update.root(), &mut class_set, &mut nodes);
    }

    accesskit::TreeUpdate {
        nodes,
        tree: update.full_root.map(|id| accesskit::Tree::new(access_id_to_kit(id))),
        focus: access_id_to_kit(update.focused),
    }
}

fn access_node_to_kit(
    node: zero_ui_view_api::access::AccessNodeRef,
    class_set: &mut accesskit::NodeClassSet,
    output: &mut Vec<(accesskit::NodeId, accesskit::Node)>,
) -> accesskit::NodeId {
    let node_id = access_id_to_kit(node.id);
    let node_role = node.role.map(access_role_to_kit).unwrap_or(accesskit::Role::Unknown);
    let mut builder = accesskit::NodeBuilder::new(node_role);

    // add actions
    for cmd in &node.commands {
        use zero_ui_view_api::access::AccessCommandName::*;

        match cmd {
            Click => {
                builder.add_action(accesskit::Action::Default);
                builder.add_action(accesskit::Action::ShowContextMenu); // TODO, what if it does not?
            }
            Focus => {
                builder.add_action(accesskit::Action::Focus);
                builder.add_action(accesskit::Action::Blur);
            }
            SetNextTabStart => {
                builder.add_action(accesskit::Action::SetSequentialFocusNavigationStartingPoint);
            }
            SetExpanded => {
                builder.add_action(accesskit::Action::Expand);
                builder.add_action(accesskit::Action::Collapse);
            }
            Increment => {
                builder.add_action(accesskit::Action::Increment);
            }
            SetToolTipVis => {
                builder.add_action(accesskit::Action::ShowTooltip);
                builder.add_action(accesskit::Action::HideTooltip);
            }
            Scroll => {
                // TODO, what is can't scroll up?
                builder.add_action(accesskit::Action::ScrollBackward);
                builder.add_action(accesskit::Action::ScrollUp);
                builder.add_action(accesskit::Action::ScrollLeft);
                builder.add_action(accesskit::Action::ScrollForward);
                builder.add_action(accesskit::Action::ScrollDown);
                builder.add_action(accesskit::Action::ScrollRight);
                builder.add_action(accesskit::Action::ScrollIntoView);
                builder.add_action(accesskit::Action::ScrollToPoint);
            }
            ReplaceSelectedText => {
                builder.add_action(accesskit::Action::ReplaceSelectedText);
            }
            SelectText => {
                builder.add_action(accesskit::Action::SetTextSelection);
            }
            SetString => {
                builder.add_action(accesskit::Action::SetValue);
            }
            SetNumber => {
                builder.add_action(accesskit::Action::SetValue);
            }
            _ => {}
        }
    }

    // add state
    for state in &node.state {
        use zero_ui_view_api::access::{self, AccessState::*};

        match state {
            AutoComplete(s) => {
                if *s == access::AutoComplete::BOTH {
                    builder.set_auto_complete(accesskit::AutoComplete::Both)
                } else if *s == access::AutoComplete::INLINE {
                    builder.set_auto_complete(accesskit::AutoComplete::Inline)
                } else if *s == access::AutoComplete::LIST {
                    builder.set_auto_complete(accesskit::AutoComplete::List)
                }
            }
            Checked(b) => builder.set_checked(match b {
                Some(true) => accesskit::Checked::True,
                Some(false) => accesskit::Checked::False,
                None => accesskit::Checked::Mixed,
            }),
            Current(kind) => match kind {
                access::CurrentKind::Page => builder.set_aria_current(accesskit::AriaCurrent::Page),
                access::CurrentKind::Step => builder.set_aria_current(accesskit::AriaCurrent::Step),
                access::CurrentKind::Location => builder.set_aria_current(accesskit::AriaCurrent::Location),
                access::CurrentKind::Date => builder.set_aria_current(accesskit::AriaCurrent::Date),
                access::CurrentKind::Time => builder.set_aria_current(accesskit::AriaCurrent::Time),
                access::CurrentKind::Item => builder.set_aria_current(accesskit::AriaCurrent::True),
            },
            Disabled => builder.set_disabled(),
            ErrorMessage(id) => builder.set_error_message(access_id_to_kit(*id)),
            Expanded(b) => builder.set_expanded(*b),
            HasPopup(pop) => match pop {
                access::Popup::Menu => builder.set_has_popup(accesskit::HasPopup::Menu),
                access::Popup::ListBox => builder.set_has_popup(accesskit::HasPopup::Listbox),
                access::Popup::Tree => builder.set_has_popup(accesskit::HasPopup::Tree),
                access::Popup::Grid => builder.set_has_popup(accesskit::HasPopup::Grid),
                access::Popup::Dialog => builder.set_has_popup(accesskit::HasPopup::Dialog),
            },
            Invalid => builder.set_invalid(accesskit::Invalid::True),
            InvalidGrammar => builder.set_invalid(accesskit::Invalid::Grammar),
            InvalidSpelling => builder.set_invalid(accesskit::Invalid::Spelling),
            Label(s) => builder.set_name(s.clone().into_boxed_str()),
            Level(n) => builder.set_hierarchical_level(n.get() as usize),
            Modal => builder.set_modal(),
            MultiSelectable => builder.set_multiselectable(),
            Orientation(o) => match o {
                access::Orientation::Horizontal => builder.set_orientation(accesskit::Orientation::Horizontal),
                access::Orientation::Vertical => builder.set_orientation(accesskit::Orientation::Vertical),
            },
            Placeholder(p) => builder.set_placeholder(p.clone().into_boxed_str()),
            ReadOnly => builder.set_read_only(),
            Required => builder.set_required(),
            Selected => builder.set_selected(true),
            Sort(o) => match o {
                access::SortDirection::Ascending => builder.set_sort_direction(accesskit::SortDirection::Ascending),
                access::SortDirection::Descending => builder.set_sort_direction(accesskit::SortDirection::Descending),
            },
            ValueMax(m) => builder.set_max_numeric_value(*m),
            ValueMin(m) => builder.set_min_numeric_value(*m),
            Value(v) => builder.set_numeric_value(*v),
            ValueText(v) => builder.set_value(v.clone().into_boxed_str()),
            Live { indicator, atomic, busy } => {
                builder.set_live(match indicator {
                    access::LiveIndicator::Assertive => accesskit::Live::Assertive,
                    access::LiveIndicator::OnlyFocused => accesskit::Live::Off,
                    access::LiveIndicator::Polite => accesskit::Live::Polite,
                });
                if *atomic {
                    builder.set_live_atomic();
                }
                if *busy {
                    builder.set_busy();
                }
            }
            ActiveDescendant(id) => builder.set_active_descendant(access_id_to_kit(*id)),
            ColCount(c) => builder.set_table_column_count(*c),
            ColIndex(i) => builder.set_table_column_index(*i),
            ColSpan(s) => builder.set_table_cell_column_span(*s),
            Controls(ids) => builder.set_controls(ids.iter().copied().map(access_id_to_kit).collect::<Vec<_>>()),
            DescribedBy(ids) => builder.set_described_by(ids.iter().copied().map(access_id_to_kit).collect::<Vec<_>>()),
            Details(ids) => builder.set_details(ids.iter().copied().map(access_id_to_kit).collect::<Vec<_>>()),
            FlowTo(id) => builder.set_flow_to(vec![access_id_to_kit(*id)]),
            LabelledBy(ids) => builder.set_labelled_by(ids.iter().copied().map(access_id_to_kit).collect::<Vec<_>>()),
            Owns(ids) => {
                for id in ids {
                    builder.push_child(access_id_to_kit(*id));
                }
            }
            ItemIndex(p) => builder.set_position_in_set(*p),
            RowCount(c) => builder.set_table_row_count(*c),
            RowIndex(i) => builder.set_table_row_index(*i),
            RowSpan(s) => builder.set_table_cell_row_span(*s),
            ItemCount(s) => builder.set_size_of_set(*s),
            _ => {}
        }
    }

    // add descendants
    for child in node.children() {
        let child_id = access_node_to_kit(child, class_set, output);
        builder.push_child(child_id);
    }
    let node = builder.build(class_set);
    output.push((node_id, node));
    node_id
}

fn access_id_to_kit(id: AccessNodeId) -> accesskit::NodeId {
    accesskit::NodeId(id.0)
}

fn access_role_to_kit(role: zero_ui_view_api::access::AccessRole) -> accesskit::Role {
    use accesskit::Role;
    use zero_ui_view_api::access::AccessRole::*;
    match role {
        Button => Role::Button,
        CheckBox => Role::CheckBox,
        GridCell => Role::Cell,
        Link => Role::Link,
        MenuItem => Role::MenuItem,
        MenuItemCheckBox => Role::MenuItemCheckBox,
        MenuItemRadio => Role::MenuItemRadio,
        Option => Role::ListBoxOption,
        ProgressBar => Role::ProgressIndicator,
        Radio => Role::RadioButton,
        ScrollBar => Role::ScrollBar,
        SearchBox => Role::SearchInput,
        Slider => Role::Slider,
        SpinButton => Role::SpinButton,
        Switch => Role::Switch,
        Tab => Role::Tab,
        TabPanel => Role::TabPanel,
        TextInput => Role::TextInput,
        TreeItem => Role::TreeItem,
        ComboBox => Role::ComboBox,
        Grid => Role::Grid,
        ListBox => Role::ListBox,
        Menu => Role::Menu,
        MenuBar => Role::MenuBar,
        RadioGroup => Role::RadioGroup,
        TabList => Role::TabList,
        Tree => Role::Tree,
        TreeGrid => Role::TreeGrid,
        Application => Role::Application,
        Article => Role::Article,
        Cell => Role::Cell,
        ColumnHeader => Role::ColumnHeader,
        Definition => Role::Definition,
        Document => Role::Document,
        Feed => Role::Feed,
        Figure => Role::Figure,
        Group => Role::Group,
        Heading => Role::Heading,
        Img => Role::Image,
        List => Role::List,
        ListItem => Role::ListItem,
        Math => Role::Math,
        Note => Role::Note,
        Row => Role::Row,
        RowGroup => Role::RowGroup,
        RowHeader => Role::RowHeader,
        Separator => Role::Splitter,
        Table => Role::Table,
        Term => Role::Term,
        ToolBar => Role::Toolbar,
        ToolTip => Role::Tooltip,
        Banner => Role::Banner,
        Complementary => Role::Complementary,
        ContentInfo => Role::ContentInfo,
        Form => Role::Form,
        Main => Role::Main,
        Navigation => Role::Navigation,
        Region => Role::Region,
        Search => Role::Search,
        Alert => Role::Alert,
        Log => Role::Log,
        Marquee => Role::Marquee,
        Status => Role::Status,
        Timer => Role::Timer,
        AlertDialog => Role::AlertDialog,
        Dialog => Role::Dialog,
        _ => Role::Unknown,
    }
}
