use std::cell::Cell;

use winit::{event::ElementState, monitor::MonitorHandle};
use zero_ui_view_api::{
    units::*, ButtonState, CursorIcon, Key, KeyState, MonitorInfo, MouseButton, MouseScrollDelta, TouchForce, TouchPhase, VideoMode,
    WindowTheme,
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
pub fn set_raw_windows_event_handler<H>(hwnd: windows::Win32::Foundation::HWND, subclass_id: usize, handler: H) -> bool
where
    H: FnMut(
            windows::Win32::Foundation::HWND,
            u32,
            windows::Win32::Foundation::WPARAM,
            windows::Win32::Foundation::LPARAM,
        ) -> Option<windows::Win32::Foundation::LRESULT>
        + 'static,
{
    use windows::Win32::Foundation::BOOL;
    let data = Box::new(handler);
    unsafe {
        windows::Win32::UI::Shell::SetWindowSubclass(hwnd, Some(subclass_raw_event_proc::<H>), subclass_id, Box::into_raw(data) as _)
            != BOOL(0)
    }
}

#[cfg(windows)]
unsafe extern "system" fn subclass_raw_event_proc<H>(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
    _id: usize,
    data: usize,
) -> windows::Win32::Foundation::LRESULT
where
    H: FnMut(
            windows::Win32::Foundation::HWND,
            u32,
            windows::Win32::Foundation::WPARAM,
            windows::Win32::Foundation::LPARAM,
        ) -> Option<windows::Win32::Foundation::LRESULT>
        + 'static,
{
    use windows::Win32::UI::WindowsAndMessaging::WM_DESTROY;
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
                windows::Win32::UI::Shell::DefSubclassProc(hwnd, msg, wparam, lparam)
            }
        }
    }
}

#[cfg(windows)]
pub(crate) fn unregister_raw_input() {
    use windows::Win32::Devices::HumanInterfaceDevice::{HID_USAGE_GENERIC_KEYBOARD, HID_USAGE_GENERIC_MOUSE, HID_USAGE_PAGE_GENERIC};
    use windows::Win32::Foundation::{BOOL, HWND};
    use windows::Win32::UI::Input::{RAWINPUTDEVICE, RIDEV_REMOVE};

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

    let ok = unsafe { windows::Win32::UI::Input::RegisterRawInputDevices(&devices, device_size) != BOOL(0) };

    if !ok {
        let e = unsafe { windows::Win32::Foundation::GetLastError() };
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
        winit::event::TouchPhase::Started => TouchPhase::Started,
        winit::event::TouchPhase::Moved => TouchPhase::Moved,
        winit::event::TouchPhase::Ended => TouchPhase::Ended,
        winit::event::TouchPhase::Cancelled => TouchPhase::Cancelled,
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

pub(crate) fn winit_theme_to_zui(t: winit::window::Theme) -> WindowTheme {
    match t {
        winit::window::Theme::Light => WindowTheme::Light,
        winit::window::Theme::Dark => WindowTheme::Dark,
    }
}

use winit::event::VirtualKeyCode as VKey;
pub(crate) fn v_key_to_key(v_key: VKey) -> Key {
    #[cfg(debug_assertions)]
    let _assert = match v_key {
        VKey::Key1 => Key::Key1,
        VKey::Key2 => Key::Key2,
        VKey::Key3 => Key::Key3,
        VKey::Key4 => Key::Key4,
        VKey::Key5 => Key::Key5,
        VKey::Key6 => Key::Key6,
        VKey::Key7 => Key::Key7,
        VKey::Key8 => Key::Key8,
        VKey::Key9 => Key::Key9,
        VKey::Key0 => Key::Key0,
        VKey::A => Key::A,
        VKey::B => Key::B,
        VKey::C => Key::C,
        VKey::D => Key::D,
        VKey::E => Key::E,
        VKey::F => Key::F,
        VKey::G => Key::G,
        VKey::H => Key::H,
        VKey::I => Key::I,
        VKey::J => Key::J,
        VKey::K => Key::K,
        VKey::L => Key::L,
        VKey::M => Key::M,
        VKey::N => Key::N,
        VKey::O => Key::O,
        VKey::P => Key::P,
        VKey::Q => Key::Q,
        VKey::R => Key::R,
        VKey::S => Key::S,
        VKey::T => Key::T,
        VKey::U => Key::U,
        VKey::V => Key::V,
        VKey::W => Key::W,
        VKey::X => Key::X,
        VKey::Y => Key::Y,
        VKey::Z => Key::Z,
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
        VKey::Snapshot => Key::PrtScr,
        VKey::Scroll => Key::ScrollLock,
        VKey::Pause => Key::Pause,
        VKey::Insert => Key::Insert,
        VKey::Home => Key::Home,
        VKey::Delete => Key::Delete,
        VKey::End => Key::End,
        VKey::PageDown => Key::PageDown,
        VKey::PageUp => Key::PageUp,
        VKey::Left => Key::Left,
        VKey::Up => Key::Up,
        VKey::Right => Key::Right,
        VKey::Down => Key::Down,
        VKey::Back => Key::Backspace,
        VKey::Return => Key::Enter,
        VKey::Space => Key::Space,
        VKey::Compose => Key::Compose,
        VKey::Caret => Key::Caret,
        VKey::Numlock => Key::NumLock,
        VKey::Numpad0 => Key::Numpad0,
        VKey::Numpad1 => Key::Numpad1,
        VKey::Numpad2 => Key::Numpad2,
        VKey::Numpad3 => Key::Numpad3,
        VKey::Numpad4 => Key::Numpad4,
        VKey::Numpad5 => Key::Numpad5,
        VKey::Numpad6 => Key::Numpad6,
        VKey::Numpad7 => Key::Numpad7,
        VKey::Numpad8 => Key::Numpad8,
        VKey::Numpad9 => Key::Numpad9,
        VKey::NumpadAdd => Key::NumpadAdd,
        VKey::NumpadDivide => Key::NumpadDivide,
        VKey::NumpadDecimal => Key::NumpadDecimal,
        VKey::NumpadComma => Key::NumpadComma,
        VKey::NumpadEnter => Key::NumpadEnter,
        VKey::NumpadEquals => Key::NumpadEquals,
        VKey::NumpadMultiply => Key::NumpadMultiply,
        VKey::NumpadSubtract => Key::NumpadSubtract,
        VKey::AbntC1 => Key::AbntC1,
        VKey::AbntC2 => Key::AbntC2,
        VKey::Apostrophe => Key::Apostrophe,
        VKey::Apps => Key::Apps,
        VKey::Asterisk => Key::Asterisk,
        VKey::At => Key::At,
        VKey::Ax => Key::Ax,
        VKey::Backslash => Key::Backslash,
        VKey::Calculator => Key::Calculator,
        VKey::Capital => Key::CapsLock,
        VKey::Colon => Key::Colon,
        VKey::Comma => Key::Comma,
        VKey::Convert => Key::Convert,
        VKey::Equals => Key::Equals,
        VKey::Grave => Key::Grave,
        VKey::Kana => Key::Kana,
        VKey::Kanji => Key::Kanji,
        VKey::LAlt => Key::LAlt,
        VKey::LBracket => Key::LBracket,
        VKey::LControl => Key::LCtrl,
        VKey::LShift => Key::LShift,
        VKey::LWin => Key::LLogo,
        VKey::Mail => Key::Mail,
        VKey::MediaSelect => Key::MediaSelect,
        VKey::MediaStop => Key::MediaStop,
        VKey::Minus => Key::Minus,
        VKey::Mute => Key::Mute,
        VKey::MyComputer => Key::MyComputer,
        VKey::NavigateForward => Key::NavigateForward,
        VKey::NavigateBackward => Key::NavigateBackward,
        VKey::NextTrack => Key::NextTrack,
        VKey::NoConvert => Key::NoConvert,
        VKey::OEM102 => Key::Oem102,
        VKey::Period => Key::Period,
        VKey::PlayPause => Key::PlayPause,
        VKey::Plus => Key::Plus,
        VKey::Power => Key::Power,
        VKey::PrevTrack => Key::PrevTrack,
        VKey::RAlt => Key::RAlt,
        VKey::RBracket => Key::RBracket,
        VKey::RControl => Key::RCtrl,
        VKey::RShift => Key::RShift,
        VKey::RWin => Key::RLogo,
        VKey::Semicolon => Key::Semicolon,
        VKey::Slash => Key::Slash,
        VKey::Sleep => Key::Sleep,
        VKey::Stop => Key::Stop,
        VKey::Sysrq => Key::Sysrq,
        VKey::Tab => Key::Tab,
        VKey::Underline => Key::Underline,
        VKey::Unlabeled => Key::Unlabeled,
        VKey::VolumeDown => Key::VolumeDown,
        VKey::VolumeUp => Key::VolumeUp,
        VKey::Wake => Key::Wake,
        VKey::WebBack => Key::WebBack,
        VKey::WebFavorites => Key::WebFavorites,
        VKey::WebForward => Key::WebForward,
        VKey::WebHome => Key::WebHome,
        VKey::WebRefresh => Key::WebRefresh,
        VKey::WebSearch => Key::WebSearch,
        VKey::WebStop => Key::WebStop,
        VKey::Yen => Key::Yen,
        VKey::Copy => Key::Copy,
        VKey::Paste => Key::Paste,
        VKey::Cut => Key::Cut,
    };
    // SAFETY: If the `match` above compiles then we have an exact copy of VKey.
    unsafe { std::mem::transmute(v_key) }
}

thread_local! {
    static SUPPRESS: Cell<bool>  = Cell::new(false);
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
        Some(s) => *s,
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
    window: windows::Win32::Foundation::HWND,
    message: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    unsafe { windows::Win32::UI::WindowsAndMessaging::DefWindowProcW(window, message, wparam, lparam) }
}

#[cfg(windows)]
pub fn get_instance_handle() -> windows::Win32::Foundation::HINSTANCE {
    // Gets the instance handle by taking the address of the
    // pseudo-variable created by the Microsoft linker:
    // https://devblogs.microsoft.com/oldnewthing/20041025-00/?p=37483

    // This is preferred over GetModuleHandle(NULL) because it also works in DLLs:
    // https://stackoverflow.com/questions/21718027/getmodulehandlenull-vs-hinstance

    extern "C" {
        static __ImageBase: windows::Win32::System::SystemServices::IMAGE_DOS_HEADER;
    }

    unsafe { windows::Win32::Foundation::HINSTANCE((&__ImageBase) as *const _ as _) }
}
