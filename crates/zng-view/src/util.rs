use std::any::Any;
use std::backtrace::Backtrace;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::{cell::Cell, sync::Arc};
use std::{fmt, ops};

use rayon::ThreadPoolBuilder;
use webrender::api as wr;
use winit::event_loop::ActiveEventLoop;
use winit::{event::ElementState, monitor::MonitorHandle};
use zng_txt::{ToTxt, Txt};
use zng_unit::*;
use zng_view_api::access::AccessNodeId;
use zng_view_api::keyboard::{KeyLocation, NativeKeyCode};
use zng_view_api::window::{FrameCapture, FrameRequest, FrameUpdateRequest, ResizeDirection, WindowButton};
use zng_view_api::{
    keyboard::{Key, KeyCode, KeyState},
    mouse::{ButtonState, MouseButton, MouseScrollDelta},
    touch::{TouchForce, TouchPhase},
    window::{CursorIcon, MonitorInfo, VideoMode},
};

#[cfg(not(target_os = "android"))]
use zng_view_api::clipboard as clipboard_api;

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
            let mut handler = unsafe { Box::from_raw(data as *mut H) };
            handler(hwnd, msg, wparam, lparam).unwrap_or_default()
        }

        msg => {
            let handler = unsafe { &mut *(data as *mut H) };
            if let Some(r) = handler(hwnd, msg, wparam, lparam) {
                r
            } else {
                unsafe { windows_sys::Win32::UI::Shell::DefSubclassProc(hwnd, msg, wparam, lparam) }
            }
        }
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
pub trait ResizeDirectionToWinit {
    fn to_winit(self) -> winit::window::ResizeDirection;
}
impl ResizeDirectionToWinit for ResizeDirection {
    fn to_winit(self) -> winit::window::ResizeDirection {
        use winit::window::ResizeDirection::*;
        match self {
            ResizeDirection::East => East,
            ResizeDirection::North => North,
            ResizeDirection::NorthEast => NorthEast,
            ResizeDirection::NorthWest => NorthWest,
            ResizeDirection::South => South,
            ResizeDirection::SouthEast => SouthEast,
            ResizeDirection::SouthWest => SouthWest,
            ResizeDirection::West => West,
        }
    }
}

pub trait WindowButtonsToWinit {
    fn to_winit(self) -> winit::window::WindowButtons;
}
impl WindowButtonsToWinit for WindowButton {
    fn to_winit(self) -> winit::window::WindowButtons {
        let mut r = winit::window::WindowButtons::empty();
        r.set(winit::window::WindowButtons::CLOSE, self.contains(WindowButton::CLOSE));
        r.set(winit::window::WindowButtons::MINIMIZE, self.contains(WindowButton::MINIMIZE));
        r.set(winit::window::WindowButtons::MAXIMIZE, self.contains(WindowButton::MAXIMIZE));

        r
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
            CursorIcon::ContextMenu => ContextMenu,
            CursorIcon::Help => Help,
            CursorIcon::Pointer => Pointer,
            CursorIcon::Progress => Progress,
            CursorIcon::Wait => Wait,
            CursorIcon::Cell => Cell,
            CursorIcon::Crosshair => Crosshair,
            CursorIcon::Text => Text,
            CursorIcon::VerticalText => VerticalText,
            CursorIcon::Alias => Alias,
            CursorIcon::Copy => Copy,
            CursorIcon::Move => Move,
            CursorIcon::NoDrop => NoDrop,
            CursorIcon::NotAllowed => NotAllowed,
            CursorIcon::Grab => Grab,
            CursorIcon::Grabbing => Grabbing,
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
            CursorIcon::AllScroll => AllScroll,
            CursorIcon::ZoomIn => ZoomIn,
            CursorIcon::ZoomOut => ZoomOut,
            _ => Default,
        }
    }
}

pub(crate) fn monitor_handle_to_info(handle: &MonitorHandle) -> MonitorInfo {
    let position = handle.position().to_px();
    let size = handle.size().to_px();
    MonitorInfo::new(
        Txt::from_str(&handle.name().unwrap_or_default()),
        position,
        size,
        Factor(handle.scale_factor() as _),
        handle.video_modes().map(glutin_video_mode_to_video_mode).collect(),
        false,
    )
}

pub(crate) fn glutin_video_mode_to_video_mode(v: winit::monitor::VideoModeHandle) -> VideoMode {
    let size = v.size();
    VideoMode::new(
        PxSize::new(Px(size.width as i32), Px(size.height as i32)),
        v.bit_depth(),
        v.refresh_rate_millihertz(),
    )
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

pub(crate) fn winit_mouse_wheel_delta_to_zng(w: winit::event::MouseScrollDelta) -> MouseScrollDelta {
    match w {
        winit::event::MouseScrollDelta::LineDelta(x, y) => MouseScrollDelta::LineDelta(x, y),
        winit::event::MouseScrollDelta::PixelDelta(d) => MouseScrollDelta::PixelDelta(d.x as f32, d.y as f32),
    }
}

pub(crate) fn winit_touch_phase_to_zng(w: winit::event::TouchPhase) -> TouchPhase {
    match w {
        winit::event::TouchPhase::Started => TouchPhase::Start,
        winit::event::TouchPhase::Moved => TouchPhase::Move,
        winit::event::TouchPhase::Ended => TouchPhase::End,
        winit::event::TouchPhase::Cancelled => TouchPhase::Cancel,
    }
}

pub(crate) fn winit_force_to_zng(f: winit::event::Force) -> TouchForce {
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

pub(crate) fn winit_mouse_button_to_zng(b: winit::event::MouseButton) -> MouseButton {
    match b {
        winit::event::MouseButton::Left => MouseButton::Left,
        winit::event::MouseButton::Right => MouseButton::Right,
        winit::event::MouseButton::Middle => MouseButton::Middle,
        winit::event::MouseButton::Back => MouseButton::Back,
        winit::event::MouseButton::Forward => MouseButton::Forward,
        winit::event::MouseButton::Other(btn) => MouseButton::Other(btn),
    }
}

#[cfg(windows)]
pub(crate) fn winit_to_hwnd(window: &winit::window::Window) -> isize {
    use raw_window_handle::HasWindowHandle as _;

    match window.window_handle().unwrap().as_raw() {
        raw_window_handle::RawWindowHandle::Win32(w) => w.hwnd.get() as _,
        _ => unreachable!(),
    }
}

pub(crate) fn winit_key_location_to_zng(t: winit::keyboard::KeyLocation) -> KeyLocation {
    match t {
        winit::keyboard::KeyLocation::Standard => KeyLocation::Standard,
        winit::keyboard::KeyLocation::Left => KeyLocation::Left,
        winit::keyboard::KeyLocation::Right => KeyLocation::Right,
        winit::keyboard::KeyLocation::Numpad => KeyLocation::Numpad,
    }
}

use winit::keyboard::{Key as WinitKey, NamedKey as WinitNamedKey};
pub(crate) fn winit_key_to_key(key: WinitKey) -> Key {
    match key {
        WinitKey::Named(k) => match k {
            WinitNamedKey::Alt => Key::Alt,
            WinitNamedKey::AltGraph => Key::AltGraph,
            WinitNamedKey::CapsLock => Key::CapsLock,
            WinitNamedKey::Control => Key::Ctrl,
            WinitNamedKey::Fn => Key::Fn,
            WinitNamedKey::FnLock => Key::FnLock,
            WinitNamedKey::NumLock => Key::NumLock,
            WinitNamedKey::ScrollLock => Key::ScrollLock,
            WinitNamedKey::Shift => Key::Shift,
            WinitNamedKey::Symbol => Key::Symbol,
            WinitNamedKey::SymbolLock => Key::SymbolLock,
            WinitNamedKey::Meta => Key::Meta,
            WinitNamedKey::Hyper => Key::Hyper,
            WinitNamedKey::Super => Key::Super,
            WinitNamedKey::Enter => Key::Enter,
            WinitNamedKey::Tab => Key::Tab,
            WinitNamedKey::Space => Key::Space,
            WinitNamedKey::ArrowDown => Key::ArrowDown,
            WinitNamedKey::ArrowLeft => Key::ArrowLeft,
            WinitNamedKey::ArrowRight => Key::ArrowRight,
            WinitNamedKey::ArrowUp => Key::ArrowUp,
            WinitNamedKey::End => Key::End,
            WinitNamedKey::Home => Key::Home,
            WinitNamedKey::PageDown => Key::PageDown,
            WinitNamedKey::PageUp => Key::PageUp,
            WinitNamedKey::Backspace => Key::Backspace,
            WinitNamedKey::Clear => Key::Clear,
            WinitNamedKey::Copy => Key::Copy,
            WinitNamedKey::CrSel => Key::CrSel,
            WinitNamedKey::Cut => Key::Cut,
            WinitNamedKey::Delete => Key::Delete,
            WinitNamedKey::EraseEof => Key::EraseEof,
            WinitNamedKey::ExSel => Key::ExSel,
            WinitNamedKey::Insert => Key::Insert,
            WinitNamedKey::Paste => Key::Paste,
            WinitNamedKey::Redo => Key::Redo,
            WinitNamedKey::Undo => Key::Undo,
            WinitNamedKey::Accept => Key::Accept,
            WinitNamedKey::Again => Key::Again,
            WinitNamedKey::Attn => Key::Attn,
            WinitNamedKey::Cancel => Key::Cancel,
            WinitNamedKey::ContextMenu => Key::ContextMenu,
            WinitNamedKey::Escape => Key::Escape,
            WinitNamedKey::Execute => Key::Execute,
            WinitNamedKey::Find => Key::Find,
            WinitNamedKey::Help => Key::Help,
            WinitNamedKey::Pause => Key::Pause,
            WinitNamedKey::Play => Key::Play,
            WinitNamedKey::Props => Key::Props,
            WinitNamedKey::Select => Key::Select,
            WinitNamedKey::ZoomIn => Key::ZoomIn,
            WinitNamedKey::ZoomOut => Key::ZoomOut,
            WinitNamedKey::BrightnessDown => Key::BrightnessDown,
            WinitNamedKey::BrightnessUp => Key::BrightnessUp,
            WinitNamedKey::Eject => Key::Eject,
            WinitNamedKey::LogOff => Key::LogOff,
            WinitNamedKey::Power => Key::Power,
            WinitNamedKey::PowerOff => Key::PowerOff,
            WinitNamedKey::PrintScreen => Key::PrintScreen,
            WinitNamedKey::Hibernate => Key::Hibernate,
            WinitNamedKey::Standby => Key::Standby,
            WinitNamedKey::WakeUp => Key::WakeUp,
            WinitNamedKey::AllCandidates => Key::AllCandidates,
            WinitNamedKey::Alphanumeric => Key::Alphanumeric,
            WinitNamedKey::CodeInput => Key::CodeInput,
            WinitNamedKey::Compose => Key::Compose,
            WinitNamedKey::Convert => Key::Convert,
            WinitNamedKey::FinalMode => Key::FinalMode,
            WinitNamedKey::GroupFirst => Key::GroupFirst,
            WinitNamedKey::GroupLast => Key::GroupLast,
            WinitNamedKey::GroupNext => Key::GroupNext,
            WinitNamedKey::GroupPrevious => Key::GroupPrevious,
            WinitNamedKey::ModeChange => Key::ModeChange,
            WinitNamedKey::NextCandidate => Key::NextCandidate,
            WinitNamedKey::NonConvert => Key::NonConvert,
            WinitNamedKey::PreviousCandidate => Key::PreviousCandidate,
            WinitNamedKey::Process => Key::Process,
            WinitNamedKey::SingleCandidate => Key::SingleCandidate,
            WinitNamedKey::HangulMode => Key::HangulMode,
            WinitNamedKey::HanjaMode => Key::HanjaMode,
            WinitNamedKey::JunjaMode => Key::JunjaMode,
            WinitNamedKey::Eisu => Key::Eisu,
            WinitNamedKey::Hankaku => Key::Hankaku,
            WinitNamedKey::Hiragana => Key::Hiragana,
            WinitNamedKey::HiraganaKatakana => Key::HiraganaKatakana,
            WinitNamedKey::KanaMode => Key::KanaMode,
            WinitNamedKey::KanjiMode => Key::KanjiMode,
            WinitNamedKey::Katakana => Key::Katakana,
            WinitNamedKey::Romaji => Key::Romaji,
            WinitNamedKey::Zenkaku => Key::Zenkaku,
            WinitNamedKey::ZenkakuHankaku => Key::ZenkakuHankaku,
            WinitNamedKey::Soft1 => Key::Soft1,
            WinitNamedKey::Soft2 => Key::Soft2,
            WinitNamedKey::Soft3 => Key::Soft3,
            WinitNamedKey::Soft4 => Key::Soft4,
            WinitNamedKey::ChannelDown => Key::ChannelDown,
            WinitNamedKey::ChannelUp => Key::ChannelUp,
            WinitNamedKey::Close => Key::Close,
            WinitNamedKey::MailForward => Key::MailForward,
            WinitNamedKey::MailReply => Key::MailReply,
            WinitNamedKey::MailSend => Key::MailSend,
            WinitNamedKey::MediaClose => Key::MediaClose,
            WinitNamedKey::MediaFastForward => Key::MediaFastForward,
            WinitNamedKey::MediaPause => Key::MediaPause,
            WinitNamedKey::MediaPlay => Key::MediaPlay,
            WinitNamedKey::MediaPlayPause => Key::MediaPlayPause,
            WinitNamedKey::MediaRecord => Key::MediaRecord,
            WinitNamedKey::MediaRewind => Key::MediaRewind,
            WinitNamedKey::MediaStop => Key::MediaStop,
            WinitNamedKey::MediaTrackNext => Key::MediaTrackNext,
            WinitNamedKey::MediaTrackPrevious => Key::MediaTrackPrevious,
            WinitNamedKey::New => Key::New,
            WinitNamedKey::Open => Key::Open,
            WinitNamedKey::Print => Key::Print,
            WinitNamedKey::Save => Key::Save,
            WinitNamedKey::SpellCheck => Key::SpellCheck,
            WinitNamedKey::Key11 => Key::Key11,
            WinitNamedKey::Key12 => Key::Key12,
            WinitNamedKey::AudioBalanceLeft => Key::AudioBalanceLeft,
            WinitNamedKey::AudioBalanceRight => Key::AudioBalanceRight,
            WinitNamedKey::AudioBassBoostDown => Key::AudioBassBoostDown,
            WinitNamedKey::AudioBassBoostToggle => Key::AudioBassBoostToggle,
            WinitNamedKey::AudioBassBoostUp => Key::AudioBassBoostUp,
            WinitNamedKey::AudioFaderFront => Key::AudioFaderFront,
            WinitNamedKey::AudioFaderRear => Key::AudioFaderRear,
            WinitNamedKey::AudioSurroundModeNext => Key::AudioSurroundModeNext,
            WinitNamedKey::AudioTrebleDown => Key::AudioTrebleDown,
            WinitNamedKey::AudioTrebleUp => Key::AudioTrebleUp,
            WinitNamedKey::AudioVolumeDown => Key::AudioVolumeDown,
            WinitNamedKey::AudioVolumeUp => Key::AudioVolumeUp,
            WinitNamedKey::AudioVolumeMute => Key::AudioVolumeMute,
            WinitNamedKey::MicrophoneToggle => Key::MicrophoneToggle,
            WinitNamedKey::MicrophoneVolumeDown => Key::MicrophoneVolumeDown,
            WinitNamedKey::MicrophoneVolumeUp => Key::MicrophoneVolumeUp,
            WinitNamedKey::MicrophoneVolumeMute => Key::MicrophoneVolumeMute,
            WinitNamedKey::SpeechCorrectionList => Key::SpeechCorrectionList,
            WinitNamedKey::SpeechInputToggle => Key::SpeechInputToggle,
            WinitNamedKey::LaunchApplication1 => Key::LaunchApplication1,
            WinitNamedKey::LaunchApplication2 => Key::LaunchApplication2,
            WinitNamedKey::LaunchCalendar => Key::LaunchCalendar,
            WinitNamedKey::LaunchContacts => Key::LaunchContacts,
            WinitNamedKey::LaunchMail => Key::LaunchMail,
            WinitNamedKey::LaunchMediaPlayer => Key::LaunchMediaPlayer,
            WinitNamedKey::LaunchMusicPlayer => Key::LaunchMusicPlayer,
            WinitNamedKey::LaunchPhone => Key::LaunchPhone,
            WinitNamedKey::LaunchScreenSaver => Key::LaunchScreenSaver,
            WinitNamedKey::LaunchSpreadsheet => Key::LaunchSpreadsheet,
            WinitNamedKey::LaunchWebBrowser => Key::LaunchWebBrowser,
            WinitNamedKey::LaunchWebCam => Key::LaunchWebCam,
            WinitNamedKey::LaunchWordProcessor => Key::LaunchWordProcessor,
            WinitNamedKey::BrowserBack => Key::BrowserBack,
            WinitNamedKey::BrowserFavorites => Key::BrowserFavorites,
            WinitNamedKey::BrowserForward => Key::BrowserForward,
            WinitNamedKey::BrowserHome => Key::BrowserHome,
            WinitNamedKey::BrowserRefresh => Key::BrowserRefresh,
            WinitNamedKey::BrowserSearch => Key::BrowserSearch,
            WinitNamedKey::BrowserStop => Key::BrowserStop,
            WinitNamedKey::AppSwitch => Key::AppSwitch,
            WinitNamedKey::Call => Key::Call,
            WinitNamedKey::Camera => Key::Camera,
            WinitNamedKey::CameraFocus => Key::CameraFocus,
            WinitNamedKey::EndCall => Key::EndCall,
            WinitNamedKey::GoBack => Key::GoBack,
            WinitNamedKey::GoHome => Key::GoHome,
            WinitNamedKey::HeadsetHook => Key::HeadsetHook,
            WinitNamedKey::LastNumberRedial => Key::LastNumberRedial,
            WinitNamedKey::Notification => Key::Notification,
            WinitNamedKey::MannerMode => Key::MannerMode,
            WinitNamedKey::VoiceDial => Key::VoiceDial,
            WinitNamedKey::TV => Key::TV,
            WinitNamedKey::TV3DMode => Key::TV3DMode,
            WinitNamedKey::TVAntennaCable => Key::TVAntennaCable,
            WinitNamedKey::TVAudioDescription => Key::TVAudioDescription,
            WinitNamedKey::TVAudioDescriptionMixDown => Key::TVAudioDescriptionMixDown,
            WinitNamedKey::TVAudioDescriptionMixUp => Key::TVAudioDescriptionMixUp,
            WinitNamedKey::TVContentsMenu => Key::TVContentsMenu,
            WinitNamedKey::TVDataService => Key::TVDataService,
            WinitNamedKey::TVInput => Key::TVInput,
            WinitNamedKey::TVInputComponent1 => Key::TVInputComponent1,
            WinitNamedKey::TVInputComponent2 => Key::TVInputComponent2,
            WinitNamedKey::TVInputComposite1 => Key::TVInputComposite1,
            WinitNamedKey::TVInputComposite2 => Key::TVInputComposite2,
            WinitNamedKey::TVInputHDMI1 => Key::TVInputHDMI1,
            WinitNamedKey::TVInputHDMI2 => Key::TVInputHDMI2,
            WinitNamedKey::TVInputHDMI3 => Key::TVInputHDMI3,
            WinitNamedKey::TVInputHDMI4 => Key::TVInputHDMI4,
            WinitNamedKey::TVInputVGA1 => Key::TVInputVGA1,
            WinitNamedKey::TVMediaContext => Key::TVMediaContext,
            WinitNamedKey::TVNetwork => Key::TVNetwork,
            WinitNamedKey::TVNumberEntry => Key::TVNumberEntry,
            WinitNamedKey::TVPower => Key::TVPower,
            WinitNamedKey::TVRadioService => Key::TVRadioService,
            WinitNamedKey::TVSatellite => Key::TVSatellite,
            WinitNamedKey::TVSatelliteBS => Key::TVSatelliteBS,
            WinitNamedKey::TVSatelliteCS => Key::TVSatelliteCS,
            WinitNamedKey::TVSatelliteToggle => Key::TVSatelliteToggle,
            WinitNamedKey::TVTerrestrialAnalog => Key::TVTerrestrialAnalog,
            WinitNamedKey::TVTerrestrialDigital => Key::TVTerrestrialDigital,
            WinitNamedKey::TVTimer => Key::TVTimer,
            WinitNamedKey::AVRInput => Key::AVRInput,
            WinitNamedKey::AVRPower => Key::AVRPower,
            WinitNamedKey::ColorF0Red => Key::ColorF0Red,
            WinitNamedKey::ColorF1Green => Key::ColorF1Green,
            WinitNamedKey::ColorF2Yellow => Key::ColorF2Yellow,
            WinitNamedKey::ColorF3Blue => Key::ColorF3Blue,
            WinitNamedKey::ColorF4Grey => Key::ColorF4Grey,
            WinitNamedKey::ColorF5Brown => Key::ColorF5Brown,
            WinitNamedKey::ClosedCaptionToggle => Key::ClosedCaptionToggle,
            WinitNamedKey::Dimmer => Key::Dimmer,
            WinitNamedKey::DisplaySwap => Key::DisplaySwap,
            WinitNamedKey::DVR => Key::DVR,
            WinitNamedKey::Exit => Key::Exit,
            WinitNamedKey::FavoriteClear0 => Key::FavoriteClear0,
            WinitNamedKey::FavoriteClear1 => Key::FavoriteClear1,
            WinitNamedKey::FavoriteClear2 => Key::FavoriteClear2,
            WinitNamedKey::FavoriteClear3 => Key::FavoriteClear3,
            WinitNamedKey::FavoriteRecall0 => Key::FavoriteRecall0,
            WinitNamedKey::FavoriteRecall1 => Key::FavoriteRecall1,
            WinitNamedKey::FavoriteRecall2 => Key::FavoriteRecall2,
            WinitNamedKey::FavoriteRecall3 => Key::FavoriteRecall3,
            WinitNamedKey::FavoriteStore0 => Key::FavoriteStore0,
            WinitNamedKey::FavoriteStore1 => Key::FavoriteStore1,
            WinitNamedKey::FavoriteStore2 => Key::FavoriteStore2,
            WinitNamedKey::FavoriteStore3 => Key::FavoriteStore3,
            WinitNamedKey::Guide => Key::Guide,
            WinitNamedKey::GuideNextDay => Key::GuideNextDay,
            WinitNamedKey::GuidePreviousDay => Key::GuidePreviousDay,
            WinitNamedKey::Info => Key::Info,
            WinitNamedKey::InstantReplay => Key::InstantReplay,
            WinitNamedKey::Link => Key::Link,
            WinitNamedKey::ListProgram => Key::ListProgram,
            WinitNamedKey::LiveContent => Key::LiveContent,
            WinitNamedKey::Lock => Key::Lock,
            WinitNamedKey::MediaApps => Key::MediaApps,
            WinitNamedKey::MediaAudioTrack => Key::MediaAudioTrack,
            WinitNamedKey::MediaLast => Key::MediaLast,
            WinitNamedKey::MediaSkipBackward => Key::MediaSkipBackward,
            WinitNamedKey::MediaSkipForward => Key::MediaSkipForward,
            WinitNamedKey::MediaStepBackward => Key::MediaStepBackward,
            WinitNamedKey::MediaStepForward => Key::MediaStepForward,
            WinitNamedKey::MediaTopMenu => Key::MediaTopMenu,
            WinitNamedKey::NavigateIn => Key::NavigateIn,
            WinitNamedKey::NavigateNext => Key::NavigateNext,
            WinitNamedKey::NavigateOut => Key::NavigateOut,
            WinitNamedKey::NavigatePrevious => Key::NavigatePrevious,
            WinitNamedKey::NextFavoriteChannel => Key::NextFavoriteChannel,
            WinitNamedKey::NextUserProfile => Key::NextUserProfile,
            WinitNamedKey::OnDemand => Key::OnDemand,
            WinitNamedKey::Pairing => Key::Pairing,
            WinitNamedKey::PinPDown => Key::PinPDown,
            WinitNamedKey::PinPMove => Key::PinPMove,
            WinitNamedKey::PinPToggle => Key::PinPToggle,
            WinitNamedKey::PinPUp => Key::PinPUp,
            WinitNamedKey::PlaySpeedDown => Key::PlaySpeedDown,
            WinitNamedKey::PlaySpeedReset => Key::PlaySpeedReset,
            WinitNamedKey::PlaySpeedUp => Key::PlaySpeedUp,
            WinitNamedKey::RandomToggle => Key::RandomToggle,
            WinitNamedKey::RcLowBattery => Key::RcLowBattery,
            WinitNamedKey::RecordSpeedNext => Key::RecordSpeedNext,
            WinitNamedKey::RfBypass => Key::RfBypass,
            WinitNamedKey::ScanChannelsToggle => Key::ScanChannelsToggle,
            WinitNamedKey::ScreenModeNext => Key::ScreenModeNext,
            WinitNamedKey::Settings => Key::Settings,
            WinitNamedKey::SplitScreenToggle => Key::SplitScreenToggle,
            WinitNamedKey::STBInput => Key::STBInput,
            WinitNamedKey::STBPower => Key::STBPower,
            WinitNamedKey::Subtitle => Key::Subtitle,
            WinitNamedKey::Teletext => Key::Teletext,
            WinitNamedKey::VideoModeNext => Key::VideoModeNext,
            WinitNamedKey::Wink => Key::Wink,
            WinitNamedKey::ZoomToggle => Key::ZoomToggle,
            WinitNamedKey::F1 => Key::F1,
            WinitNamedKey::F2 => Key::F2,
            WinitNamedKey::F3 => Key::F3,
            WinitNamedKey::F4 => Key::F4,
            WinitNamedKey::F5 => Key::F5,
            WinitNamedKey::F6 => Key::F6,
            WinitNamedKey::F7 => Key::F7,
            WinitNamedKey::F8 => Key::F8,
            WinitNamedKey::F9 => Key::F9,
            WinitNamedKey::F10 => Key::F10,
            WinitNamedKey::F11 => Key::F11,
            WinitNamedKey::F12 => Key::F12,
            WinitNamedKey::F13 => Key::F13,
            WinitNamedKey::F14 => Key::F14,
            WinitNamedKey::F15 => Key::F15,
            WinitNamedKey::F16 => Key::F16,
            WinitNamedKey::F17 => Key::F17,
            WinitNamedKey::F18 => Key::F18,
            WinitNamedKey::F19 => Key::F19,
            WinitNamedKey::F20 => Key::F20,
            WinitNamedKey::F21 => Key::F21,
            WinitNamedKey::F22 => Key::F22,
            WinitNamedKey::F23 => Key::F23,
            WinitNamedKey::F24 => Key::F24,
            WinitNamedKey::F25 => Key::F25,
            WinitNamedKey::F26 => Key::F26,
            WinitNamedKey::F27 => Key::F27,
            WinitNamedKey::F28 => Key::F28,
            WinitNamedKey::F29 => Key::F29,
            WinitNamedKey::F30 => Key::F30,
            WinitNamedKey::F31 => Key::F31,
            WinitNamedKey::F32 => Key::F32,
            WinitNamedKey::F33 => Key::F33,
            WinitNamedKey::F34 => Key::F34,
            WinitNamedKey::F35 => Key::F35,
            k => {
                tracing::error!("matched unknown key code `{k:?}`");
                Key::Unidentified
            }
        },
        WinitKey::Character(c) => {
            let mut chars = c.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => Key::Char(c),
                _ => Key::Str(c.as_str().to_owned().into()),
            }
        }
        WinitKey::Unidentified(_) => Key::Unidentified,
        WinitKey::Dead(k) => Key::Dead(k),
    }
}

use winit::keyboard::KeyCode as WinitKeyCode;
pub(crate) fn winit_key_code_to_key_code(key: WinitKeyCode) -> KeyCode {
    match key {
        WinitKeyCode::Backquote => KeyCode::Backquote,
        WinitKeyCode::Backslash => KeyCode::Backslash,
        WinitKeyCode::BracketLeft => KeyCode::BracketLeft,
        WinitKeyCode::BracketRight => KeyCode::BracketRight,
        WinitKeyCode::Comma => KeyCode::Comma,
        WinitKeyCode::Digit0 => KeyCode::Digit0,
        WinitKeyCode::Digit1 => KeyCode::Digit1,
        WinitKeyCode::Digit2 => KeyCode::Digit2,
        WinitKeyCode::Digit3 => KeyCode::Digit3,
        WinitKeyCode::Digit4 => KeyCode::Digit4,
        WinitKeyCode::Digit5 => KeyCode::Digit5,
        WinitKeyCode::Digit6 => KeyCode::Digit6,
        WinitKeyCode::Digit7 => KeyCode::Digit7,
        WinitKeyCode::Digit8 => KeyCode::Digit8,
        WinitKeyCode::Digit9 => KeyCode::Digit9,
        WinitKeyCode::Equal => KeyCode::Equal,
        WinitKeyCode::IntlBackslash => KeyCode::IntlBackslash,
        WinitKeyCode::IntlRo => KeyCode::IntlRo,
        WinitKeyCode::IntlYen => KeyCode::IntlYen,
        WinitKeyCode::KeyA => KeyCode::KeyA,
        WinitKeyCode::KeyB => KeyCode::KeyB,
        WinitKeyCode::KeyC => KeyCode::KeyC,
        WinitKeyCode::KeyD => KeyCode::KeyD,
        WinitKeyCode::KeyE => KeyCode::KeyE,
        WinitKeyCode::KeyF => KeyCode::KeyF,
        WinitKeyCode::KeyG => KeyCode::KeyG,
        WinitKeyCode::KeyH => KeyCode::KeyH,
        WinitKeyCode::KeyI => KeyCode::KeyI,
        WinitKeyCode::KeyJ => KeyCode::KeyJ,
        WinitKeyCode::KeyK => KeyCode::KeyK,
        WinitKeyCode::KeyL => KeyCode::KeyL,
        WinitKeyCode::KeyM => KeyCode::KeyM,
        WinitKeyCode::KeyN => KeyCode::KeyN,
        WinitKeyCode::KeyO => KeyCode::KeyO,
        WinitKeyCode::KeyP => KeyCode::KeyP,
        WinitKeyCode::KeyQ => KeyCode::KeyQ,
        WinitKeyCode::KeyR => KeyCode::KeyR,
        WinitKeyCode::KeyS => KeyCode::KeyS,
        WinitKeyCode::KeyT => KeyCode::KeyT,
        WinitKeyCode::KeyU => KeyCode::KeyU,
        WinitKeyCode::KeyV => KeyCode::KeyV,
        WinitKeyCode::KeyW => KeyCode::KeyW,
        WinitKeyCode::KeyX => KeyCode::KeyX,
        WinitKeyCode::KeyY => KeyCode::KeyY,
        WinitKeyCode::KeyZ => KeyCode::KeyZ,
        WinitKeyCode::Minus => KeyCode::Minus,
        WinitKeyCode::Period => KeyCode::Period,
        WinitKeyCode::Quote => KeyCode::Quote,
        WinitKeyCode::Semicolon => KeyCode::Semicolon,
        WinitKeyCode::Slash => KeyCode::Slash,
        WinitKeyCode::AltLeft => KeyCode::AltLeft,
        WinitKeyCode::AltRight => KeyCode::AltRight,
        WinitKeyCode::Backspace => KeyCode::Backspace,
        WinitKeyCode::CapsLock => KeyCode::CapsLock,
        WinitKeyCode::ContextMenu => KeyCode::ContextMenu,
        WinitKeyCode::ControlLeft => KeyCode::CtrlLeft,
        WinitKeyCode::ControlRight => KeyCode::CtrlRight,
        WinitKeyCode::Enter => KeyCode::Enter,
        WinitKeyCode::SuperLeft => KeyCode::SuperLeft,
        WinitKeyCode::SuperRight => KeyCode::SuperRight,
        WinitKeyCode::ShiftLeft => KeyCode::ShiftLeft,
        WinitKeyCode::ShiftRight => KeyCode::ShiftRight,
        WinitKeyCode::Space => KeyCode::Space,
        WinitKeyCode::Tab => KeyCode::Tab,
        WinitKeyCode::Convert => KeyCode::Convert,
        WinitKeyCode::KanaMode => KeyCode::KanaMode,
        WinitKeyCode::Lang1 => KeyCode::Lang1,
        WinitKeyCode::Lang2 => KeyCode::Lang2,
        WinitKeyCode::Lang3 => KeyCode::Lang3,
        WinitKeyCode::Lang4 => KeyCode::Lang4,
        WinitKeyCode::Lang5 => KeyCode::Lang5,
        WinitKeyCode::NonConvert => KeyCode::NonConvert,
        WinitKeyCode::Delete => KeyCode::Delete,
        WinitKeyCode::End => KeyCode::End,
        WinitKeyCode::Help => KeyCode::Help,
        WinitKeyCode::Home => KeyCode::Home,
        WinitKeyCode::Insert => KeyCode::Insert,
        WinitKeyCode::PageDown => KeyCode::PageDown,
        WinitKeyCode::PageUp => KeyCode::PageUp,
        WinitKeyCode::ArrowDown => KeyCode::ArrowDown,
        WinitKeyCode::ArrowLeft => KeyCode::ArrowLeft,
        WinitKeyCode::ArrowRight => KeyCode::ArrowRight,
        WinitKeyCode::ArrowUp => KeyCode::ArrowUp,
        WinitKeyCode::NumLock => KeyCode::NumLock,
        WinitKeyCode::Numpad0 => KeyCode::Numpad0,
        WinitKeyCode::Numpad1 => KeyCode::Numpad1,
        WinitKeyCode::Numpad2 => KeyCode::Numpad2,
        WinitKeyCode::Numpad3 => KeyCode::Numpad3,
        WinitKeyCode::Numpad4 => KeyCode::Numpad4,
        WinitKeyCode::Numpad5 => KeyCode::Numpad5,
        WinitKeyCode::Numpad6 => KeyCode::Numpad6,
        WinitKeyCode::Numpad7 => KeyCode::Numpad7,
        WinitKeyCode::Numpad8 => KeyCode::Numpad8,
        WinitKeyCode::Numpad9 => KeyCode::Numpad9,
        WinitKeyCode::NumpadAdd => KeyCode::NumpadAdd,
        WinitKeyCode::NumpadBackspace => KeyCode::NumpadBackspace,
        WinitKeyCode::NumpadClear => KeyCode::NumpadClear,
        WinitKeyCode::NumpadClearEntry => KeyCode::NumpadClearEntry,
        WinitKeyCode::NumpadComma => KeyCode::NumpadComma,
        WinitKeyCode::NumpadDecimal => KeyCode::NumpadDecimal,
        WinitKeyCode::NumpadDivide => KeyCode::NumpadDivide,
        WinitKeyCode::NumpadEnter => KeyCode::NumpadEnter,
        WinitKeyCode::NumpadEqual => KeyCode::NumpadEqual,
        WinitKeyCode::NumpadHash => KeyCode::NumpadHash,
        WinitKeyCode::NumpadMemoryAdd => KeyCode::NumpadMemoryAdd,
        WinitKeyCode::NumpadMemoryClear => KeyCode::NumpadMemoryClear,
        WinitKeyCode::NumpadMemoryRecall => KeyCode::NumpadMemoryRecall,
        WinitKeyCode::NumpadMemoryStore => KeyCode::NumpadMemoryStore,
        WinitKeyCode::NumpadMemorySubtract => KeyCode::NumpadMemorySubtract,
        WinitKeyCode::NumpadMultiply => KeyCode::NumpadMultiply,
        WinitKeyCode::NumpadParenLeft => KeyCode::NumpadParenLeft,
        WinitKeyCode::NumpadParenRight => KeyCode::NumpadParenRight,
        WinitKeyCode::NumpadStar => KeyCode::NumpadStar,
        WinitKeyCode::NumpadSubtract => KeyCode::NumpadSubtract,
        WinitKeyCode::Escape => KeyCode::Escape,
        WinitKeyCode::Fn => KeyCode::Fn,
        WinitKeyCode::FnLock => KeyCode::FnLock,
        WinitKeyCode::PrintScreen => KeyCode::PrintScreen,
        WinitKeyCode::ScrollLock => KeyCode::ScrollLock,
        WinitKeyCode::Pause => KeyCode::Pause,
        WinitKeyCode::BrowserBack => KeyCode::BrowserBack,
        WinitKeyCode::BrowserFavorites => KeyCode::BrowserFavorites,
        WinitKeyCode::BrowserForward => KeyCode::BrowserForward,
        WinitKeyCode::BrowserHome => KeyCode::BrowserHome,
        WinitKeyCode::BrowserRefresh => KeyCode::BrowserRefresh,
        WinitKeyCode::BrowserSearch => KeyCode::BrowserSearch,
        WinitKeyCode::BrowserStop => KeyCode::BrowserStop,
        WinitKeyCode::Eject => KeyCode::Eject,
        WinitKeyCode::LaunchApp1 => KeyCode::LaunchApp1,
        WinitKeyCode::LaunchApp2 => KeyCode::LaunchApp2,
        WinitKeyCode::LaunchMail => KeyCode::LaunchMail,
        WinitKeyCode::MediaPlayPause => KeyCode::MediaPlayPause,
        WinitKeyCode::MediaSelect => KeyCode::MediaSelect,
        WinitKeyCode::MediaStop => KeyCode::MediaStop,
        WinitKeyCode::MediaTrackNext => KeyCode::MediaTrackNext,
        WinitKeyCode::MediaTrackPrevious => KeyCode::MediaTrackPrevious,
        WinitKeyCode::Power => KeyCode::Power,
        WinitKeyCode::Sleep => KeyCode::Sleep,
        WinitKeyCode::AudioVolumeDown => KeyCode::AudioVolumeDown,
        WinitKeyCode::AudioVolumeMute => KeyCode::AudioVolumeMute,
        WinitKeyCode::AudioVolumeUp => KeyCode::AudioVolumeUp,
        WinitKeyCode::WakeUp => KeyCode::WakeUp,
        WinitKeyCode::Meta => KeyCode::Meta,
        WinitKeyCode::Hyper => KeyCode::Hyper,
        WinitKeyCode::Turbo => KeyCode::Turbo,
        WinitKeyCode::Abort => KeyCode::Abort,
        WinitKeyCode::Resume => KeyCode::Resume,
        WinitKeyCode::Suspend => KeyCode::Suspend,
        WinitKeyCode::Again => KeyCode::Again,
        WinitKeyCode::Copy => KeyCode::Copy,
        WinitKeyCode::Cut => KeyCode::Cut,
        WinitKeyCode::Find => KeyCode::Find,
        WinitKeyCode::Open => KeyCode::Open,
        WinitKeyCode::Paste => KeyCode::Paste,
        WinitKeyCode::Props => KeyCode::Props,
        WinitKeyCode::Select => KeyCode::Select,
        WinitKeyCode::Undo => KeyCode::Undo,
        WinitKeyCode::Hiragana => KeyCode::Hiragana,
        WinitKeyCode::Katakana => KeyCode::Katakana,
        WinitKeyCode::F1 => KeyCode::F1,
        WinitKeyCode::F2 => KeyCode::F2,
        WinitKeyCode::F3 => KeyCode::F3,
        WinitKeyCode::F4 => KeyCode::F4,
        WinitKeyCode::F5 => KeyCode::F5,
        WinitKeyCode::F6 => KeyCode::F6,
        WinitKeyCode::F7 => KeyCode::F7,
        WinitKeyCode::F8 => KeyCode::F8,
        WinitKeyCode::F9 => KeyCode::F9,
        WinitKeyCode::F10 => KeyCode::F10,
        WinitKeyCode::F11 => KeyCode::F11,
        WinitKeyCode::F12 => KeyCode::F12,
        WinitKeyCode::F13 => KeyCode::F13,
        WinitKeyCode::F14 => KeyCode::F14,
        WinitKeyCode::F15 => KeyCode::F15,
        WinitKeyCode::F16 => KeyCode::F16,
        WinitKeyCode::F17 => KeyCode::F17,
        WinitKeyCode::F18 => KeyCode::F18,
        WinitKeyCode::F19 => KeyCode::F19,
        WinitKeyCode::F20 => KeyCode::F20,
        WinitKeyCode::F21 => KeyCode::F21,
        WinitKeyCode::F22 => KeyCode::F22,
        WinitKeyCode::F23 => KeyCode::F23,
        WinitKeyCode::F24 => KeyCode::F24,
        WinitKeyCode::F25 => KeyCode::F25,
        WinitKeyCode::F26 => KeyCode::F26,
        WinitKeyCode::F27 => KeyCode::F27,
        WinitKeyCode::F28 => KeyCode::F28,
        WinitKeyCode::F29 => KeyCode::F29,
        WinitKeyCode::F30 => KeyCode::F30,
        WinitKeyCode::F31 => KeyCode::F31,
        WinitKeyCode::F32 => KeyCode::F32,
        WinitKeyCode::F33 => KeyCode::F33,
        WinitKeyCode::F34 => KeyCode::F34,
        WinitKeyCode::F35 => KeyCode::F35,
        key => {
            tracing::error!("matched unknown key code `{key:?}`");
            KeyCode::Unidentified(NativeKeyCode::Unidentified)
        }
    }
}

use winit::keyboard::PhysicalKey as WinitPhysicalKey;
pub(crate) fn winit_physical_key_to_key_code(key: WinitPhysicalKey) -> KeyCode {
    match key {
        WinitPhysicalKey::Code(code) => winit_key_code_to_key_code(code),
        WinitPhysicalKey::Unidentified(u) => match u {
            winit::keyboard::NativeKeyCode::Unidentified => KeyCode::Unidentified(NativeKeyCode::Unidentified),
            winit::keyboard::NativeKeyCode::Android(c) => KeyCode::Unidentified(NativeKeyCode::Android(c)),
            winit::keyboard::NativeKeyCode::MacOS(c) => KeyCode::Unidentified(NativeKeyCode::MacOS(c)),
            winit::keyboard::NativeKeyCode::Windows(c) => KeyCode::Unidentified(NativeKeyCode::Windows(c)),
            winit::keyboard::NativeKeyCode::Xkb(c) => KeyCode::Unidentified(NativeKeyCode::Xkb(c)),
        },
    }
}

thread_local! {
    static SUPPRESS: Cell<bool> = const { Cell::new(false) };
    static SUPPRESSED_PANIC: RefCell<Option<SuppressedPanic>> = const { RefCell::new(None) };
}

/// If `true` our custom panic hook must not log anything.
#[cfg(ipc)]
pub(crate) fn suppress_panic() -> bool {
    SUPPRESS.get()
}
#[cfg(ipc)]
pub(crate) fn set_suppressed_panic(panic: SuppressedPanic) {
    SUPPRESSED_PANIC.set(Some(panic));
}

#[derive(Debug)]
pub(crate) struct SuppressedPanic {
    pub thread: Txt,
    pub msg: Txt,
    pub file: Txt,
    pub line: u32,
    pub column: u32,
    pub backtrace: Backtrace,
}
impl SuppressedPanic {
    #[cfg(ipc)]
    pub fn from_hook(info: &std::panic::PanicHookInfo, backtrace: Backtrace) -> Self {
        let current_thread = std::thread::current();
        let thread = current_thread.name().unwrap_or("<unnamed>");
        let msg = Self::payload(info.payload());

        let (file, line, column) = if let Some(l) = info.location() {
            (l.file(), l.line(), l.column())
        } else {
            ("<unknown>", 0, 0)
        };
        Self {
            thread: thread.to_txt(),
            msg,
            file: file.to_txt(),
            line,
            column,
            backtrace,
        }
    }

    pub fn from_catch(p: Box<dyn Any>) -> Self {
        Self {
            thread: Txt::from("<unknown>"),
            msg: Self::payload(&*p),
            file: Txt::from("<unknown>"),
            line: 0,
            column: 0,
            backtrace: Backtrace::disabled(),
        }
    }

    fn payload(p: &dyn Any) -> Txt {
        match p.downcast_ref::<&'static str>() {
            Some(s) => s,
            None => match p.downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<dyn Any>",
            },
        }
        .to_txt()
    }
}
impl std::error::Error for SuppressedPanic {}
impl fmt::Display for SuppressedPanic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "thread '{}' panicked at {}:{}:{}:\n{}\n{}",
            self.thread, self.file, self.line, self.column, self.msg, self.backtrace,
        )
    }
}

/// Like [`std::panic::catch_unwind`], but flags [`suppress_panic`] for our custom panic hook.
pub(crate) fn catch_suppress<T>(f: impl FnOnce() -> T + std::panic::UnwindSafe) -> Result<T, Box<SuppressedPanic>> {
    SUPPRESS.set(true);
    let _cleanup = RunOnDrop::new(|| SUPPRESS.set(false));
    std::panic::catch_unwind(f).map_err(|e| {
        SUPPRESSED_PANIC.with_borrow_mut(|p| match p.take() {
            Some(p) => Box::new(p),
            None => Box::new(SuppressedPanic::from_catch(e)),
        })
    })
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
pub fn get_instance_handle() -> isize {
    // HINSTANCE
    // Gets the instance handle by taking the address of the
    // pseudo-variable created by the Microsoft linker:
    // https://devblogs.microsoft.com/oldnewthing/20041025-00/?p=37483

    // This is preferred over GetModuleHandle(NULL) because it also works in DLLs:
    // https://stackoverflow.com/questions/21718027/getmodulehandlenull-vs-hinstance

    unsafe extern "C" {
        static __ImageBase: windows_sys::Win32::System::SystemServices::IMAGE_DOS_HEADER;
    }

    unsafe { (&__ImageBase) as *const _ as _ }
}

#[cfg(windows)]
pub mod taskbar_com {
    // copied from winit

    #![expect(non_snake_case)]
    #![expect(non_upper_case_globals)]

    use std::ffi::c_void;

    use windows_sys::{
        Win32::Foundation::{BOOL, HWND},
        core::{GUID, HRESULT, IUnknown},
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
    let worker = ThreadPoolBuilder::new().thread_name(|idx| format!("WRWorker#{idx}")).build();
    Arc::new(worker.unwrap())
}

#[cfg(not(any(windows, target_os = "android")))]
pub(crate) fn arboard_to_clip(e: arboard::Error) -> clipboard_api::ClipboardError {
    match e {
        arboard::Error::ContentNotAvailable => clipboard_api::ClipboardError::NotFound,
        arboard::Error::ClipboardNotSupported => clipboard_api::ClipboardError::NotSupported,
        e => clipboard_api::ClipboardError::Other(zng_txt::formatx!("{e:?}")),
    }
}

#[cfg(windows)]
pub(crate) fn clipboard_win_to_clip(e: clipboard_win::ErrorCode) -> clipboard_api::ClipboardError {
    use zng_txt::formatx;

    if e.raw_code() == 0 {
        // If GetClipboardData fails it returns a NULL, but GetLastError sometimes (always?) returns 0 (ERROR_SUCCESS)
        clipboard_api::ClipboardError::NotFound
    } else {
        clipboard_api::ClipboardError::Other(formatx!("{e:?}"))
    }
}

fn accesskit_to_px(length: f64) -> Px {
    Px(length.round() as _)
}

fn accesskit_point_to_px(p: accesskit::Point) -> PxPoint {
    PxPoint::new(accesskit_to_px(p.x), accesskit_to_px(p.y))
}

pub(crate) fn accesskit_to_event(
    window_id: zng_view_api::window::WindowId,
    request: accesskit::ActionRequest,
) -> Option<zng_view_api::Event> {
    use accesskit::Action;
    use zng_view_api::access::*;

    let target = AccessNodeId(request.target.0);

    Some(zng_view_api::Event::AccessCommand {
        window: window_id,
        target,
        command: match request.action {
            Action::Click => AccessCmd::Click(true),
            Action::ShowContextMenu => AccessCmd::Click(false),
            Action::Focus => AccessCmd::Focus(true),
            Action::SetSequentialFocusNavigationStartingPoint => AccessCmd::FocusNavOrigin,
            Action::Blur => AccessCmd::Focus(false),
            Action::Collapse => AccessCmd::SetExpanded(false),
            Action::Expand => AccessCmd::SetExpanded(true),
            Action::CustomAction => return None,
            Action::Decrement => AccessCmd::Increment(-1),
            Action::Increment => AccessCmd::Increment(1),
            Action::HideTooltip => AccessCmd::SetToolTipVis(false),
            Action::ShowTooltip => AccessCmd::SetToolTipVis(true),
            Action::ReplaceSelectedText => {
                if let Some(accesskit::ActionData::Value(s)) = request.data {
                    AccessCmd::ReplaceSelectedText(Txt::from_str(&s))
                } else {
                    AccessCmd::ReplaceSelectedText(Txt::from_str(""))
                }
            }
            Action::ScrollDown => AccessCmd::Scroll(ScrollCmd::PageDown),
            Action::ScrollLeft => AccessCmd::Scroll(ScrollCmd::PageLeft),
            Action::ScrollRight => AccessCmd::Scroll(ScrollCmd::PageRight),
            Action::ScrollUp => AccessCmd::Scroll(ScrollCmd::PageUp),
            Action::ScrollIntoView => AccessCmd::Scroll(ScrollCmd::ScrollTo),
            Action::ScrollToPoint => {
                if let Some(accesskit::ActionData::ScrollToPoint(p)) = request.data {
                    AccessCmd::Scroll(ScrollCmd::ScrollToRect(PxRect::new(accesskit_point_to_px(p), PxSize::splat(Px(1)))))
                } else {
                    return None;
                }
            }
            Action::SetScrollOffset => {
                if let Some(accesskit::ActionData::SetScrollOffset(o)) = request.data {
                    AccessCmd::Scroll(ScrollCmd::ScrollToRect(PxRect::new(accesskit_point_to_px(o), PxSize::splat(Px(1)))))
                } else {
                    return None;
                }
            }
            Action::SetTextSelection => {
                if let Some(accesskit::ActionData::SetTextSelection(s)) = request.data {
                    AccessCmd::SelectText {
                        start: (AccessNodeId(s.anchor.node.0), s.anchor.character_index),
                        caret: (AccessNodeId(s.focus.node.0), s.focus.character_index),
                    }
                } else {
                    return None;
                }
            }
            Action::SetValue => match request.data {
                Some(accesskit::ActionData::Value(s)) => AccessCmd::SetString(Txt::from_str(&s)),
                Some(accesskit::ActionData::NumericValue(n)) => AccessCmd::SetNumber(n),
                _ => return None,
            },
        },
    })
}

pub(crate) fn access_tree_update_to_kit(update: zng_view_api::access::AccessTreeUpdate) -> accesskit::TreeUpdate {
    let mut nodes = Vec::with_capacity(update.updates.iter().map(|t| t.len()).sum());

    for update in update.updates {
        access_node_to_kit(update.root(), &mut nodes);
    }

    accesskit::TreeUpdate {
        nodes,
        tree: update.full_root.map(|id| accesskit::Tree::new(access_id_to_kit(id))),
        focus: access_id_to_kit(update.focused),
    }
}

fn access_node_to_kit(
    node: zng_view_api::access::AccessNodeRef,
    output: &mut Vec<(accesskit::NodeId, accesskit::Node)>,
) -> accesskit::NodeId {
    let node_id = access_id_to_kit(node.id);
    let node_role = node.role.map(access_role_to_kit).unwrap_or(accesskit::Role::Unknown);
    let mut builder = accesskit::Node::new(node_role);

    // add bounds and transform
    if !node.size.is_empty() {
        let mut bounds = accesskit::Rect::new(0.0, 0.0, node.size.width.0 as f64, node.size.height.0 as f64);
        if !node.transform.is_identity() {
            if node.children_count() == 0
                && let PxTransform::Offset(o) = node.transform
            {
                let (x, y) = o.cast().to_tuple();
                bounds = bounds.with_origin(accesskit::Point::new(x, y));
            } else {
                let t = node.transform.to_transform().to_2d();
                builder.set_transform(accesskit::Affine::new(t.cast().to_array()));
            }
        }
        builder.set_bounds(bounds);
    }

    // add actions
    for cmd in &node.commands {
        use zng_view_api::access::AccessCmdName::*;

        match cmd {
            Click => {
                builder.add_action(accesskit::Action::Click);
                builder.add_action(accesskit::Action::ShowContextMenu);
            }
            Focus => {
                builder.add_action(accesskit::Action::Focus);
                builder.add_action(accesskit::Action::Blur);
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
                builder.add_action(accesskit::Action::ScrollUp);
                builder.add_action(accesskit::Action::ScrollLeft);
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
        use zng_view_api::access::{self, AccessState::*};

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
            Checked(b) => builder.set_toggled(match b {
                Some(true) => accesskit::Toggled::True,
                Some(false) => accesskit::Toggled::False,
                None => accesskit::Toggled::Mixed,
            }),
            Current(kind) => match kind {
                access::CurrentKind::Page => builder.set_aria_current(accesskit::AriaCurrent::Page),
                access::CurrentKind::Step => builder.set_aria_current(accesskit::AriaCurrent::Step),
                access::CurrentKind::Location => builder.set_aria_current(accesskit::AriaCurrent::Location),
                access::CurrentKind::Date => builder.set_aria_current(accesskit::AriaCurrent::Date),
                access::CurrentKind::Time => builder.set_aria_current(accesskit::AriaCurrent::Time),
                access::CurrentKind::Item => builder.set_aria_current(accesskit::AriaCurrent::True),
                _ => {}
            },
            Disabled => builder.set_disabled(),
            ErrorMessage(id) => builder.set_error_message(access_id_to_kit(*id)),
            Expanded(b) => builder.set_expanded(*b),
            Popup(pop) => match pop {
                access::Popup::Menu => builder.set_has_popup(accesskit::HasPopup::Menu),
                access::Popup::ListBox => builder.set_has_popup(accesskit::HasPopup::Listbox),
                access::Popup::Tree => builder.set_has_popup(accesskit::HasPopup::Tree),
                access::Popup::Grid => builder.set_has_popup(accesskit::HasPopup::Grid),
                access::Popup::Dialog => builder.set_has_popup(accesskit::HasPopup::Dialog),
                _ => {}
            },
            Invalid(i) => {
                if i.contains(access::Invalid::SPELLING) {
                    builder.set_invalid(accesskit::Invalid::Spelling)
                } else if i.contains(access::Invalid::GRAMMAR) {
                    builder.set_invalid(accesskit::Invalid::Grammar)
                } else if i.contains(access::Invalid::ANY) {
                    builder.set_invalid(accesskit::Invalid::True)
                }
            }
            Label(s) => builder.set_label(s.clone().into_owned().into_boxed_str()),
            Level(n) => builder.set_level(n.get() as usize),
            Modal => builder.set_modal(),
            MultiSelectable => builder.set_multiselectable(),
            Orientation(o) => match o {
                access::Orientation::Horizontal => builder.set_orientation(accesskit::Orientation::Horizontal),
                access::Orientation::Vertical => builder.set_orientation(accesskit::Orientation::Vertical),
            },
            Placeholder(p) => builder.set_placeholder(p.clone().into_owned().into_boxed_str()),
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
            ValueText(v) => builder.set_value(v.clone().into_owned().into_boxed_str()),
            Live { indicator, atomic, busy } => {
                builder.set_live(match indicator {
                    access::LiveIndicator::Assertive => accesskit::Live::Assertive,
                    access::LiveIndicator::OnlyFocused => accesskit::Live::Off,
                    access::LiveIndicator::Polite => accesskit::Live::Polite,
                    _ => accesskit::Live::Polite,
                });
                if *atomic {
                    builder.set_live_atomic();
                }
                if *busy {
                    builder.set_busy();
                }
            }
            ActiveDescendant(id) => builder.set_active_descendant(access_id_to_kit(*id)),
            ColCount(c) => builder.set_column_count(*c),
            ColIndex(i) => builder.set_column_index(*i),
            ColSpan(s) => builder.set_column_span(*s),
            Controls(ids) => builder.set_controls(ids.iter().copied().map(access_id_to_kit).collect::<Vec<_>>()),
            DescribedBy(ids) => builder.set_described_by(ids.iter().copied().map(access_id_to_kit).collect::<Vec<_>>()),
            Details(ids) => builder.set_details(ids.iter().copied().map(access_id_to_kit).collect::<Vec<_>>()),
            FlowTo(ids) => builder.set_flow_to(ids.iter().copied().map(access_id_to_kit).collect::<Vec<_>>()),
            LabelledBy(ids) => builder.set_labelled_by(ids.iter().copied().map(access_id_to_kit).collect::<Vec<_>>()),
            LabelledByChild => {
                let labelled_by: Vec<_> = if node.children.is_empty() {
                    node.children().map(|c| access_id_to_kit(c.id)).collect()
                } else {
                    node.children.iter().map(|id| access_id_to_kit(*id)).collect()
                };
                builder.set_labelled_by(labelled_by);
            }
            Owns(ids) => {
                for id in ids {
                    builder.push_child(access_id_to_kit(*id));
                }
            }
            ItemIndex(p) => builder.set_position_in_set(*p),
            RowCount(c) => builder.set_row_count(*c),
            RowIndex(i) => builder.set_row_index(*i),
            RowSpan(s) => builder.set_row_span(*s),
            ItemCount(s) => builder.set_size_of_set(*s),
            Lang(l) => builder.set_language(l.to_string()),

            ScrollHorizontal(x) => {
                builder.set_scroll_x(*x as f64);
                builder.set_scroll_x_min(0.0);
                builder.set_scroll_x_max(1.0);
            }
            ScrollVertical(y) => {
                builder.set_scroll_y(*y as f64);
                builder.set_scroll_y_min(0.0);
                builder.set_scroll_y_max(1.0);
            }
            _ => {}
        }
    }

    // add descendants
    if node.children.is_empty() {
        for child in node.children() {
            let child_id = access_node_to_kit(child, output);
            builder.push_child(child_id);
        }
    } else {
        for id in &node.children {
            builder.push_child(access_id_to_kit(*id));
        }
        for child in node.children() {
            let _ = access_node_to_kit(child, output);
        }
    }

    let node = builder;
    output.push((node_id, node));
    node_id
}

fn access_id_to_kit(id: AccessNodeId) -> accesskit::NodeId {
    accesskit::NodeId(id.0)
}

fn access_role_to_kit(role: zng_view_api::access::AccessRole) -> accesskit::Role {
    use accesskit::Role;
    use zng_view_api::access::AccessRole::*;
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
        Image => Role::Image,
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

pub(crate) fn frame_render_reasons(frame: &FrameRequest) -> wr::RenderReasons {
    let mut reasons = wr::RenderReasons::SCENE;

    if frame.capture != FrameCapture::None {
        reasons |= wr::RenderReasons::SNAPSHOT;
    }

    reasons
}

pub(crate) fn frame_update_render_reasons(update: &FrameUpdateRequest) -> wr::RenderReasons {
    let mut reasons = wr::RenderReasons::empty();

    if update.has_bounds() {
        reasons |= wr::RenderReasons::ANIMATED_PROPERTY;
    }

    if update.capture != FrameCapture::None {
        reasons |= wr::RenderReasons::SNAPSHOT;
    }

    if update.clear_color.is_some() {
        reasons |= wr::RenderReasons::CONFIG_CHANGE;
    }

    reasons
}

#[must_use = "call unset before drop"]
pub(crate) struct WinitEventLoop(*const ActiveEventLoop);
impl WinitEventLoop {
    pub fn set<'l>(&mut self, winit_loop: &'l ActiveEventLoop) -> WinitEventLoopGuard<'l> {
        self.0 = winit_loop;
        WinitEventLoopGuard {
            defused: false,
            _loop_lifetime: PhantomData,
        }
    }
}
impl Default for WinitEventLoop {
    fn default() -> Self {
        Self(std::ptr::null())
    }
}
impl ops::Deref for WinitEventLoop {
    type Target = ActiveEventLoop;

    fn deref(&self) -> &Self::Target {
        assert!(!self.0.is_null(), "winit event loop not active");
        // SAFETY: just checked, and can only set pointer with `set`
        unsafe { &*self.0 }
    }
}
pub(crate) struct WinitEventLoopGuard<'l> {
    defused: bool,
    _loop_lifetime: PhantomData<&'l ActiveEventLoop>,
}
impl WinitEventLoopGuard<'_> {
    pub fn unset(&mut self, l: &mut WinitEventLoop) {
        self.defused = true;
        l.0 = std::ptr::null();
    }
}
impl Drop for WinitEventLoopGuard<'_> {
    fn drop(&mut self) {
        if !self.defused {
            let msg = "unsafe pointer to winit ActiveEventLoop not cleared";
            if std::thread::panicking() {
                tracing::error!("{msg}");
            } else {
                panic!("{msg}");
            }
        }
    }
}

#[cfg(feature = "image_png")]
pub(crate) use png_color::png_color_metadata_to_icc;

#[cfg(feature = "image_png")]
mod png_color {
    use lcms2::{CIExyY, CIExyYTRIPLE, Profile, ToneCurve};
    use png::{ScaledFloat, SourceChromaticities};

    fn is_srgb_equivalent(gamma: Option<ScaledFloat>, chromaticities: Option<SourceChromaticities>) -> bool {
        const SRGB_GAMMA: f32 = 1.0 / 2.2;
        const GAMMA_EPSILON: f32 = 0.01;

        const SRGB_WHITE_X: f32 = 0.3127;
        const SRGB_WHITE_Y: f32 = 0.3290;
        const SRGB_RED_X: f32 = 0.64;
        const SRGB_RED_Y: f32 = 0.33;
        const SRGB_GREEN_X: f32 = 0.30;
        const SRGB_GREEN_Y: f32 = 0.60;
        const SRGB_BLUE_X: f32 = 0.15;
        const SRGB_BLUE_Y: f32 = 0.06;
        const CHROMA_EPSILON: f32 = 0.001;

        let about_eq = |a: f32, b: f32, epsilon: f32| (a - b).abs() < epsilon;

        let gamma_matches = gamma.is_some_and(|g| about_eq(g.into_value(), SRGB_GAMMA, GAMMA_EPSILON));

        let chroma_matches = chromaticities.is_some_and(|c| {
            about_eq(c.white.0.into_value(), SRGB_WHITE_X, CHROMA_EPSILON)
                && about_eq(c.white.1.into_value(), SRGB_WHITE_Y, CHROMA_EPSILON)
                && about_eq(c.red.0.into_value(), SRGB_RED_X, CHROMA_EPSILON)
                && about_eq(c.red.1.into_value(), SRGB_RED_Y, CHROMA_EPSILON)
                && about_eq(c.green.0.into_value(), SRGB_GREEN_X, CHROMA_EPSILON)
                && about_eq(c.green.1.into_value(), SRGB_GREEN_Y, CHROMA_EPSILON)
                && about_eq(c.blue.0.into_value(), SRGB_BLUE_X, CHROMA_EPSILON)
                && about_eq(c.blue.1.into_value(), SRGB_BLUE_Y, CHROMA_EPSILON)
        });

        match (gamma.is_some(), chromaticities.is_some()) {
            (true, true) => gamma_matches && chroma_matches,
            (true, false) => gamma_matches,
            (false, true) => chroma_matches,
            (false, false) => false,
        }
    }

    pub(crate) fn png_color_metadata_to_icc(gamma: Option<ScaledFloat>, chromaticities: Option<SourceChromaticities>) -> Option<Profile> {
        if gamma.is_none() && chromaticities.is_none() {
            return None;
        }

        if is_srgb_equivalent(gamma, chromaticities) {
            return None;
        }

        let white_point = chromaticities.map_or_else(
            || CIExyY {
                x: 0.3127,
                y: 0.3290,
                Y: 1.0,
            },
            |c| CIExyY {
                x: c.white.0.into_value() as f64,
                y: c.white.1.into_value() as f64,
                Y: 1.0,
            },
        );

        let primaries = chromaticities.map_or_else(
            || CIExyYTRIPLE {
                Red: CIExyY { x: 0.64, y: 0.33, Y: 1.0 },
                Green: CIExyY { x: 0.30, y: 0.60, Y: 1.0 },
                Blue: CIExyY { x: 0.15, y: 0.06, Y: 1.0 },
            },
            |c| CIExyYTRIPLE {
                Red: CIExyY {
                    x: c.red.0.into_value() as f64,
                    y: c.red.1.into_value() as f64,
                    Y: 1.0,
                },
                Green: CIExyY {
                    x: c.green.0.into_value() as f64,
                    y: c.green.1.into_value() as f64,
                    Y: 1.0,
                },
                Blue: CIExyY {
                    x: c.blue.0.into_value() as f64,
                    y: c.blue.1.into_value() as f64,
                    Y: 1.0,
                },
            },
        );

        let display_gamma = match gamma.map(|g| g.into_value()) {
            Some(g) if (g - 1.0).abs() < 0.001 => 1.0,
            Some(g) => (1.0 / g) as f64,
            None => 2.2,
        };

        let tone_curve = ToneCurve::new(display_gamma);
        let curves = [&tone_curve, &tone_curve, &tone_curve];

        Profile::new_rgb(&white_point, &primaries, &curves).ok()
    }
}
