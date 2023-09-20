use std::{cell::Cell, sync::Arc};

use rayon::ThreadPoolBuilder;
use winit::{event::ElementState, monitor::MonitorHandle};
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

    let target = AccessNodeId(request.target.0.get() as u64);

    Some(zero_ui_view_api::Event::AccessCommand {
        window: window_id,
        target,
        command: match request.action {
            Action::Default => AccessCommand::Default,
            Action::Focus => AccessCommand::Focus,
            Action::Blur => AccessCommand::Blur,
            Action::Collapse => AccessCommand::Collapse,
            Action::Expand => AccessCommand::Expand,
            Action::CustomAction => return None, // Figure out pattern to extract ApiExtension from data
            Action::Decrement => AccessCommand::Decrement,
            Action::Increment => AccessCommand::Increment,
            Action::HideTooltip => AccessCommand::HideToolTip,
            Action::ShowTooltip => AccessCommand::ShowToolTip,
            Action::InvalidateTree => return None,      // TODO
            Action::LoadInlineTextBoxes => return None, // TODO
            Action::ReplaceSelectedText => {
                if let Some(accesskit::ActionData::Value(s)) = request.data {
                    AccessCommand::ReplaceSelectedText(s.to_string())
                } else {
                    AccessCommand::ReplaceSelectedText(String::new())
                }
            }
            Action::ScrollBackward => AccessCommand::ScrollUp,
            Action::ScrollDown => AccessCommand::ScrollDown,
            Action::ScrollForward => AccessCommand::ScrollDown,
            Action::ScrollLeft => AccessCommand::ScrollLeft,
            Action::ScrollRight => AccessCommand::ScrollRight,
            Action::ScrollUp => AccessCommand::ScrollUp,
            Action::ScrollIntoView => return None, // TODO dip or px?
            Action::ScrollToPoint => return None,
            Action::SetScrollOffset => return None, // TODO, value range
            Action::SetTextSelection => {
                if let Some(accesskit::ActionData::SetTextSelection(s)) = request.data {
                    AccessCommand::SelectText {
                        start: (AccessNodeId(s.anchor.node.0.get() as u64), s.anchor.character_index),
                        caret: (AccessNodeId(s.focus.node.0.get() as u64), s.focus.character_index),
                    }
                } else {
                    return None;
                }
            }
            Action::SetSequentialFocusNavigationStartingPoint => AccessCommand::SetNextTabStart,
            Action::SetValue => match request.data {
                Some(accesskit::ActionData::Value(s)) => AccessCommand::SetValueString(s.to_string()),
                Some(accesskit::ActionData::NumericValue(n)) => AccessCommand::SetValueNumber(n),
                _ => return None,
            },
            Action::ShowContextMenu => AccessCommand::ShowContextMenu,
        },
    })
}
