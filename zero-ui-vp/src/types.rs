use crate::units::*;
use bitflags::*;
use serde::{Deserialize, Serialize};
pub use serde_bytes::ByteBuf;
use std::time::Duration;
use std::{fmt, path::PathBuf};
use webrender_api::{BuiltDisplayListDescriptor, ColorF, Epoch, HitTestResult, PipelineId};

/// Window ID in channel.
///
/// In the View Process this is mapped to a system id.
///
/// In the App Process this is mapped to a unique id that survives View crashes.
///
/// Zero is never an ID.
pub type WinId = u32;

/// Device ID in channel.
///
/// In the View Process this is mapped to a system id.
///
/// In the App Process this is mapped to a unique id, but does not survived View crashes.
///
/// Zero is never an ID.
pub type DevId = u32;

/// Monitor screen ID in channel.
///
/// In the View Process this is mapped to a system id.
///
/// In the App Process this is mapped to a unique id, but does not survived View crashes.
///
/// Zero is never an ID.
pub type MonId = u32;

/// View-process generation, starts at one and changes every respawn, it is never zero.
pub type ViewProcessGen = u32;

/// Hardware-dependent keyboard scan code.
pub type ScanCode = u32;

/// Identifier for a specific analog axis on some device.
pub type AxisId = u32;

/// Identifier for a specific button on some device.
pub type ButtonId = u32;

/// State a [`Key`] has entered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyState {
    /// The key was pressed.
    Pressed,
    /// The key was released.
    Released,
}
#[cfg(feature = "full")]
impl From<glutin::event::ElementState> for KeyState {
    fn from(s: glutin::event::ElementState) -> Self {
        match s {
            glutin::event::ElementState::Pressed => KeyState::Pressed,
            glutin::event::ElementState::Released => KeyState::Released,
        }
    }
}

/// State a [`MouseButton`] has entered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ButtonState {
    /// The button was pressed.
    Pressed,
    /// The button was released.
    Released,
}
#[cfg(feature = "full")]
impl From<glutin::event::ElementState> for ButtonState {
    fn from(s: glutin::event::ElementState) -> Self {
        match s {
            glutin::event::ElementState::Pressed => ButtonState::Pressed,
            glutin::event::ElementState::Released => ButtonState::Released,
        }
    }
}

bitflags! {
    /// Represents the current state of the keyboard modifiers.
    ///
    /// Each flag represents a modifier and is set if this modifier is active.
    #[derive(Default, Serialize, Deserialize)]
    pub struct ModifiersState: u32 {
        // left and right modifiers are currently commented out, but we should be able to support
        // them in a future release
        /// The "shift" key.
        const SHIFT = 0b100;
        // const LSHIFT = 0b010 << 0;
        // const RSHIFT = 0b001 << 0;
        /// The "control" key.
        const CTRL = 0b100 << 3;
        // const LCTRL = 0b010 << 3;
        // const RCTRL = 0b001 << 3;
        /// The "alt" key.
        const ALT = 0b100 << 6;
        // const LALT = 0b010 << 6;
        // const RALT = 0b001 << 6;
        /// This is the "windows" key on PC and "command" key on Mac.
        const LOGO = 0b100 << 9;
        // const LLOGO = 0b010 << 9;
        // const RLOGO = 0b001 << 9;
    }
}
impl ModifiersState {
    /// Returns `true` if the shift key is pressed.
    pub fn shift(&self) -> bool {
        self.intersects(Self::SHIFT)
    }
    /// Returns `true` if the control key is pressed.
    pub fn ctrl(&self) -> bool {
        self.intersects(Self::CTRL)
    }
    /// Returns `true` if the alt key is pressed.
    pub fn alt(&self) -> bool {
        self.intersects(Self::ALT)
    }
    /// Returns `true` if the logo key is pressed.
    pub fn logo(&self) -> bool {
        self.intersects(Self::LOGO)
    }
}
#[cfg(feature = "full")]
impl From<glutin::event::ModifiersState> for ModifiersState {
    fn from(s: glutin::event::ModifiersState) -> Self {
        Self::from_bits(s.bits()).unwrap()
    }
}

/// Describes a button of a mouse controller.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum MouseButton {
    /// Left button.
    Left,
    /// Right button.
    Right,
    /// Middle button.
    Middle,
    /// Any other button.
    Other(u16),
}
#[cfg(feature = "full")]
impl From<glutin::event::MouseButton> for MouseButton {
    fn from(btn: glutin::event::MouseButton) -> Self {
        match btn {
            glutin::event::MouseButton::Left => Self::Left,
            glutin::event::MouseButton::Right => Self::Right,
            glutin::event::MouseButton::Middle => Self::Middle,
            glutin::event::MouseButton::Other(btn) => Self::Other(btn),
        }
    }
}

/// Describes touch-screen input state.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum TouchPhase {
    /// A finger touched the screen.
    Started,
    /// A finger moved on the screen.
    Moved,
    /// A finger was lifted from the screen.
    Ended,
    /// The system cancelled tracking for the touch.
    Cancelled,
}
#[cfg(feature = "full")]
impl From<glutin::event::TouchPhase> for TouchPhase {
    fn from(s: glutin::event::TouchPhase) -> Self {
        match s {
            glutin::event::TouchPhase::Started => Self::Started,
            glutin::event::TouchPhase::Moved => Self::Moved,
            glutin::event::TouchPhase::Ended => Self::Ended,
            glutin::event::TouchPhase::Cancelled => Self::Cancelled,
        }
    }
}

/// Describes a difference in the mouse scroll wheel state.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MouseScrollDelta {
    /// Amount in lines or rows to scroll in the horizontal
    /// and vertical directions.
    ///
    /// Positive values indicate movement forward
    /// (away from the user) or rightwards.
    LineDelta(f32, f32),
    /// Amount in pixels to scroll in the horizontal and
    /// vertical direction.
    ///
    /// Scroll events are expressed as a PixelDelta if
    /// supported by the device (eg. a touchpad) and
    /// platform.
    PixelDelta(f32, f32),
}
#[cfg(feature = "full")]
impl From<glutin::event::MouseScrollDelta> for MouseScrollDelta {
    fn from(s: glutin::event::MouseScrollDelta) -> Self {
        match s {
            glutin::event::MouseScrollDelta::LineDelta(x, y) => Self::LineDelta(x, y),
            glutin::event::MouseScrollDelta::PixelDelta(d) => Self::PixelDelta(d.x as f32, d.y as f32),
        }
    }
}

/// Symbolic name for a keyboard key.
#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
#[repr(u32)]
#[allow(missing_docs)] // some of these are self-explanatory.
pub enum Key {
    /// The '1' key over the letters.
    Key1,
    /// The '2' key over the letters.
    Key2,
    /// The '3' key over the letters.
    Key3,
    /// The '4' key over the letters.
    Key4,
    /// The '5' key over the letters.
    Key5,
    /// The '6' key over the letters.
    Key6,
    /// The '7' key over the letters.
    Key7,
    /// The '8' key over the letters.
    Key8,
    /// The '9' key over the letters.
    Key9,
    /// The '0' key over the 'O' and 'P' keys.
    Key0,

    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    /// The Escape key, next to F1.
    Escape,

    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,

    /// Print Screen/SysRq.
    PrtScr,
    ScrollLock,
    /// Pause/Break key, next to Scroll lock.
    Pause,

    /// `Insert`, next to Backspace.
    Insert,
    Home,
    Delete,
    End,
    PageDown,
    PageUp,

    Left,
    Up,
    Right,
    Down,

    /// The Backspace key, right over Enter.
    Backspace,
    /// The Return key.
    Enter,
    /// The space bar.
    Space,

    /// The "Compose" key on Linux.
    Compose,

    Caret,

    NumLock,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    NumpadDivide,
    NumpadDecimal,
    NumpadComma,
    NumpadEnter,
    NumpadEquals,
    NumpadMultiply,
    NumpadSubtract,

    AbntC1,
    AbntC2,
    Apostrophe,
    Apps,
    Asterisk,
    At,
    Ax,
    Backslash,
    Calculator,
    CapsLock,
    Colon,
    Comma,
    Convert,
    Equals,
    Grave,
    Kana,
    Kanji,
    /// Left Alt
    LAlt,
    LBracket,
    /// Left Control
    LCtrl,
    /// Left Shift
    LShift,
    LLogo,
    Mail,
    MediaSelect,
    MediaStop,
    Minus,
    Mute,
    MyComputer,
    // also called "Next"
    NavigateForward,
    // also called "Prior"
    NavigateBackward,
    NextTrack,
    NoConvert,
    Oem102,
    /// The '.' key, also called a dot.
    Period,
    PlayPause,
    Plus,
    Power,
    PrevTrack,
    /// Right Alt.
    RAlt,
    RBracket,
    RControl,
    RShift,
    RLogo,
    Semicolon,
    Slash,
    Sleep,
    Stop,
    Sysrq,
    Tab,
    Underline,
    Unlabeled,
    VolumeDown,
    VolumeUp,
    Wake,
    WebBack,
    WebFavorites,
    WebForward,
    WebHome,
    WebRefresh,
    WebSearch,
    WebStop,
    Yen,
    Copy,
    Paste,
    Cut,
}
impl Key {
    /// If the key is a modifier key.
    pub fn is_modifier(self) -> bool {
        matches!(
            self,
            Key::LAlt | Key::LCtrl | Key::LShift | Key::LLogo | Key::RAlt | Key::RControl | Key::RShift | Key::RLogo
        )
    }

    /// If the key is left alt or right alt.
    pub fn is_alt(self) -> bool {
        matches!(self, Key::LAlt | Key::RAlt)
    }

    /// If the key is left ctrl or right ctrl.
    pub fn is_ctrl(self) -> bool {
        matches!(self, Key::LCtrl | Key::RControl)
    }

    /// If the key is left shift or right shift.
    pub fn is_shift(self) -> bool {
        matches!(self, Key::LShift | Key::RShift)
    }

    /// If the key is left logo or right logo.
    pub fn is_logo(self) -> bool {
        matches!(self, Key::LLogo | Key::RLogo)
    }

    /// If the key is a numpad key, includes numlock.
    pub fn is_numpad(self) -> bool {
        let key = self as u32;
        key >= Key::NumLock as u32 && key <= Key::NumpadSubtract as u32
    }
}
#[cfg(feature = "full")]
use glutin::event::VirtualKeyCode as VKey;
#[cfg(feature = "full")]
impl From<VKey> for Key {
    fn from(v_key: VKey) -> Self {
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
            VKey::RControl => Key::RControl,
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
}
#[cfg(feature = "full")]
impl From<Key> for VKey {
    fn from(key: Key) -> Self {
        // SAFETY: This is safe if `From<VKey> for Key` is safe.
        unsafe { std::mem::transmute(key) }
    }
}

/// Describes the appearance of the mouse cursor.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CursorIcon {
    /// The platform-dependent default cursor.
    Default,
    /// A simple crosshair.
    Crosshair,
    /// A hand (often used to indicate links in web browsers).
    Hand,
    /// Self explanatory.
    Arrow,
    /// Indicates something is to be moved.
    Move,
    /// Indicates horizontal text that may be selected or edited.
    Text,
    /// Program busy indicator.
    Wait,
    /// Help indicator (often rendered as a "?")
    Help,
    /// Progress indicator. Shows that processing is being done. But in contrast
    /// with "Wait" the user may still interact with the program. Often rendered
    /// as a spinning beach ball, or an arrow with a watch or hourglass.
    Progress,

    /// Cursor showing that something cannot be done.
    NotAllowed,
    /// Indicates that a context menu is available.
    ContextMenu,
    /// Indicates a table cell or set of cells can be selected.
    Cell,
    /// Indicates vertical text that may be selected or edited.
    VerticalText,
    /// Indicates an alias or shortcut is to be created.
    Alias,
    /// Indicates something is to be copied.
    Copy,
    /// An item may not be dropped at the current location.
    NoDrop,
    /// Indicates something can be grabbed.
    Grab,
    /// Indicates something is grabbed.
    Grabbing,
    /// Something can be scrolled in any direction (panned).
    AllScroll,
    /// Something can be zoomed (magnified) in.
    ZoomIn,
    /// Something can be zoomed (magnified) out.
    ZoomOut,

    /// Indicate that the right vertical edge is to be moved left/right.
    EResize,
    /// Indicates that the top horizontal edge is to be moved up/down.
    NResize,
    /// Indicates that top-right corner is to be moved.
    NeResize,
    /// Indicates that the top-left corner is to be moved.
    NwResize,
    /// Indicates that the bottom vertical edge is to be moved up/down.
    SResize,
    /// Indicates that the bottom-right corner is to be moved.
    SeResize,
    /// Indicates that the bottom-left corner is to be moved.
    SwResize,
    /// Indicates that the left vertical edge is to be moved left/right.
    WResize,
    /// Indicates that the any of the vertical edges is to be moved left/right.
    EwResize,
    /// Indicates that the any of the horizontal edges is to be moved up/down.
    NsResize,
    /// Indicates that the top-right or bottom-left corners are to be moved.
    NeswResize,
    /// Indicates that the top-left or bottom-right corners are to be moved.
    NwseResize,
    /// Indicates that the item/column can be resized horizontally.
    ColResize,
    /// Indicates that the item/row can be resized vertically.
    RowResize,
}
impl Default for CursorIcon {
    fn default() -> Self {
        CursorIcon::Default
    }
}
#[cfg(feature = "full")]
use glutin::window::CursorIcon as WCursorIcon;

#[cfg(feature = "full")]
impl From<WCursorIcon> for CursorIcon {
    fn from(s: WCursorIcon) -> Self {
        let _assert = match s {
            WCursorIcon::Default => CursorIcon::Default,
            WCursorIcon::Crosshair => CursorIcon::Crosshair,
            WCursorIcon::Hand => CursorIcon::Hand,
            WCursorIcon::Arrow => CursorIcon::Arrow,
            WCursorIcon::Move => CursorIcon::Move,
            WCursorIcon::Text => CursorIcon::Text,
            WCursorIcon::Wait => CursorIcon::Wait,
            WCursorIcon::Help => CursorIcon::Help,
            WCursorIcon::Progress => CursorIcon::Progress,
            WCursorIcon::NotAllowed => CursorIcon::NotAllowed,
            WCursorIcon::ContextMenu => CursorIcon::ContextMenu,
            WCursorIcon::Cell => CursorIcon::Cell,
            WCursorIcon::VerticalText => CursorIcon::VerticalText,
            WCursorIcon::Alias => CursorIcon::Alias,
            WCursorIcon::Copy => CursorIcon::Copy,
            WCursorIcon::NoDrop => CursorIcon::NoDrop,
            WCursorIcon::Grab => CursorIcon::Grab,
            WCursorIcon::Grabbing => CursorIcon::Grabbing,
            WCursorIcon::AllScroll => CursorIcon::AllScroll,
            WCursorIcon::ZoomIn => CursorIcon::ZoomIn,
            WCursorIcon::ZoomOut => CursorIcon::ZoomOut,
            WCursorIcon::EResize => CursorIcon::EResize,
            WCursorIcon::NResize => CursorIcon::NResize,
            WCursorIcon::NeResize => CursorIcon::NeResize,
            WCursorIcon::NwResize => CursorIcon::NwResize,
            WCursorIcon::SResize => CursorIcon::SResize,
            WCursorIcon::SeResize => CursorIcon::SeResize,
            WCursorIcon::SwResize => CursorIcon::SwResize,
            WCursorIcon::WResize => CursorIcon::WResize,
            WCursorIcon::EwResize => CursorIcon::EwResize,
            WCursorIcon::NsResize => CursorIcon::NsResize,
            WCursorIcon::NeswResize => CursorIcon::NeswResize,
            WCursorIcon::NwseResize => CursorIcon::NwseResize,
            WCursorIcon::ColResize => CursorIcon::ColResize,
            WCursorIcon::RowResize => CursorIcon::RowResize,
        };

        // SAFETY: If the `match` above compiles then we have an exact copy of VKey.
        unsafe { std::mem::transmute(s) }
    }
}
#[cfg(feature = "full")]
impl From<CursorIcon> for WCursorIcon {
    fn from(c: CursorIcon) -> Self {
        // SAFETY: This is safe if `From<WCursorIcon> for CursorIcon` is safe.
        unsafe { std::mem::transmute(c) }
    }
}

/// Window state after a resize.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum WindowState {
    /// Window is visible but does not fill the screen.
    Normal,
    /// Window is only visible as an icon in the taskbar.
    Minimized,
    /// Window fills the screen, but not the parts reserved by the system, like the taskbar.
    Maximized,
    /// Window is chromeless and completely fills the screen, including over parts reserved by the system.
    Fullscreen,
    /// Window has exclusive access to the video output, so only the window content is visible.
    Exclusive,
}
impl Default for WindowState {
    fn default() -> Self {
        WindowState::Normal
    }
}
impl WindowState {
    /// Returns `true` if `self` matches [`Fullscreen`] or [`Exclusive`].
    ///
    /// [`Fullscreen`]: WindowState::Fullscreen
    /// [`Exclusive`]: WindowState::Exclusive
    pub fn is_fullscreen(self) -> bool {
        matches!(self, Self::Fullscreen | Self::Exclusive)
    }
}

/// System/User events sent from the View Process.
#[repr(u32)]
#[derive(Debug, Serialize, Deserialize)]
pub enum Ev {
    /// The view-process crashed and respawned, all resources must be rebuild.
    ///
    /// The [`ViewProcessGen`] is the new generation, after the respawn.
    Respawned(ViewProcessGen),
    /// The event channel disconnected, probably because the view-process crashed.
    ///
    /// The [`ViewProcessGen`] is the generation of the view-process that was lost, it must be passed to
    /// [`Controller::handle_disconnect`].
    ///
    /// [`Controller::handle_disconnect`]: crate::Controller::handle_disconnect
    Disconnected(ViewProcessGen),
    /// A sequence of events that happened at the *same time* finished sending.
    ///
    /// The same device action can generate multiple events, this event is send after
    /// each such sequence of window and device events, even if it only one event.
    EventsCleared,

    /// A frame finished rendering.
    ///
    /// `EventsCleared` is not send after this event.
    FrameRendered(WinId, Epoch),

    /// Window maximized/minimized/restored.
    ///
    /// The [`EventCause`] can be used to identify a state change initiated by the app.
    WindowStateChanged(WinId, WindowState, EventCause),

    /// The size of the window has changed. Contains the client area’s new dimensions and the window state.
    ///
    /// The [`EventCause`] can be used to identify a resize initiated by the app.
    WindowResized(WinId, DipSize, EventCause),
    /// The position of the window has changed. Contains the window’s new position.
    ///
    /// The [`EventCause`] can be used to identify a move initiated by the app.
    WindowMoved(WinId, DipPoint, EventCause),
    /// A file has been dropped into the window.
    ///
    /// When the user drops multiple files at once, this event will be emitted for each file separately.
    DroppedFile(WinId, PathBuf),
    /// A file is being hovered over the window.
    ///
    /// When the user hovers multiple files at once, this event will be emitted for each file separately.
    HoveredFile(WinId, PathBuf),
    /// A file was hovered, but has exited the window.
    ///
    /// There will be a single event triggered even if multiple files were hovered.
    HoveredFileCancelled(WinId),
    /// The window received a Unicode character.
    ReceivedCharacter(WinId, char),
    /// The window gained or lost focus.
    ///
    /// The parameter is true if the window has gained focus, and false if it has lost focus.
    Focused(WinId, bool),
    /// An event from the keyboard has been received.
    KeyboardInput(WinId, DevId, ScanCode, KeyState, Option<Key>),
    /// The keyboard modifiers have changed.
    ModifiersChanged(WinId, ModifiersState),
    /// The cursor has moved on the window.
    ///
    /// Contains a hit-test of the point and the frame epoch that was hit.
    CursorMoved(WinId, DevId, DipPoint, HitTestResult, Epoch),

    /// The cursor has entered the window.
    CursorEntered(WinId, DevId),
    /// The cursor has left the window.
    CursorLeft(WinId, DevId),
    /// A mouse wheel movement or touchpad scroll occurred.
    MouseWheel(WinId, DevId, MouseScrollDelta, TouchPhase),
    /// An mouse button press has been received.
    MouseInput(WinId, DevId, ButtonState, MouseButton),
    /// Touchpad pressure event.
    TouchpadPressure(WinId, DevId, f32, i64),
    /// Motion on some analog axis. May report data redundant to other, more specific events.
    AxisMotion(WinId, DevId, AxisId, f64),
    /// Touch event has been received.
    Touch(WinId, DevId, TouchPhase, DipPoint, Option<Force>, u64),
    /// The window’s scale factor has changed.
    ScaleFactorChanged(WinId, f32),

    /// The available monitors have changed.
    MonitorsChanged(Vec<(MonId, MonitorInfo)>),

    /// The system window theme has changed.
    WindowThemeChanged(WinId, WindowTheme),
    /// The window has been requested to close.
    WindowCloseRequested(WinId),
    /// The window has closed.
    WindowClosed(WinId),

    // Config events
    /// System fonts have changed.
    FontsChanged,
    /// System text-antialiasing configuration has changed.
    TextAaChanged(TextAntiAliasing),
    /// System double-click definition changed.
    MultiClickConfigChanged(MultiClickConfig),
    /// System animation enabled config changed.
    AnimationEnabledChanged(bool),
    /// System definition of pressed key repeat event changed.
    KeyRepeatDelayChanged(Duration),

    // Raw device events
    /// Device added or installed.
    DeviceAdded(DevId),
    /// Device removed.
    DeviceRemoved(DevId),
    /// Mouse pointer motion.
    ///
    /// The values if the delta of movement (x, y), not position.
    DeviceMouseMotion(DevId, (f64, f64)),
    /// Mouse scroll wheel turn.
    DeviceMouseWheel(DevId, MouseScrollDelta),
    /// Motion on some analog axis.
    ///
    /// This includes the mouse device and any other that fits.
    DeviceMotion(DevId, AxisId, f64),
    /// Device button press or release.
    DeviceButton(DevId, ButtonId, ButtonState),
    /// Device key press or release.
    DeviceKey(DevId, ScanCode, KeyState, Option<Key>),
    /// Device Unicode character input.
    DeviceText(DevId, char),
}

/// Cause of a window state change.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventCause {
    /// Operating system or end-user affected the window.
    System,
    /// App affected the window.
    App,
}

/// Describes the force of a touch event
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Force {
    /// On iOS, the force is calibrated so that the same number corresponds to
    /// roughly the same amount of pressure on the screen regardless of the
    /// device.
    Calibrated {
        /// The force of the touch, where a value of 1.0 represents the force of
        /// an average touch (predetermined by the system, not user-specific).
        ///
        /// The force reported by Apple Pencil is measured along the axis of the
        /// pencil. If you want a force perpendicular to the device, you need to
        /// calculate this value using the `altitude_angle` value.
        force: f64,
        /// The maximum possible force for a touch.
        ///
        /// The value of this field is sufficiently high to provide a wide
        /// dynamic range for values of the `force` field.
        max_possible_force: f64,
        /// The altitude (in radians) of the stylus.
        ///
        /// A value of 0 radians indicates that the stylus is parallel to the
        /// surface. The value of this property is Pi/2 when the stylus is
        /// perpendicular to the surface.
        altitude_angle: Option<f64>,
    },
    /// If the platform reports the force as normalized, we have no way of
    /// knowing how much pressure 1.0 corresponds to – we know it's the maximum
    /// amount of force, but as to how much force, you might either have to
    /// press really really hard, or not hard at all, depending on the device.
    Normalized(f64),
}
#[cfg(feature = "full")]
impl From<glutin::event::Force> for Force {
    fn from(f: glutin::event::Force) -> Self {
        match f {
            glutin::event::Force::Calibrated {
                force,
                max_possible_force,
                altitude_angle,
            } => Force::Calibrated {
                force,
                max_possible_force,
                altitude_angle,
            },
            glutin::event::Force::Normalized(f) => Force::Normalized(f),
        }
    }
}

/// OS theme.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum WindowTheme {
    /// Dark text on light background.
    Light,

    /// Light text on dark background.
    Dark,
}
#[cfg(feature = "full")]
impl From<glutin::window::Theme> for WindowTheme {
    fn from(t: glutin::window::Theme) -> Self {
        match t {
            glutin::window::Theme::Light => WindowTheme::Light,
            glutin::window::Theme::Dark => WindowTheme::Dark,
        }
    }
}

/// Window icon.
#[derive(Clone, Serialize, Deserialize)]
pub struct Icon {
    /// RGBA8 data.
    pub rgba: ByteBuf,
    /// Pixel width.
    pub width: u32,
    /// Pixel height.
    pub height: u32,
}
impl fmt::Debug for Icon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Icon")
            .field("rgba", &format_args!("<{} bytes>", self.rgba.len()))
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

/// Text anti-aliasing.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAntiAliasing {
    /// Uses the operating system configuration.
    Default,
    /// Sub-pixel anti-aliasing if a fast implementation is available, otherwise uses `Alpha`.
    Subpixel,
    /// Alpha blending anti-aliasing.
    Alpha,
    /// Disable anti-aliasing.
    Mono,
}
impl Default for TextAntiAliasing {
    fn default() -> Self {
        Self::Default
    }
}
impl fmt::Debug for TextAntiAliasing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "TextAntiAliasing::")?;
        }
        match self {
            TextAntiAliasing::Default => write!(f, "Default"),
            TextAntiAliasing::Subpixel => write!(f, "Subpixel"),
            TextAntiAliasing::Alpha => write!(f, "Alpha"),
            TextAntiAliasing::Mono => write!(f, "Mono"),
        }
    }
}

/// The View-Process crashed and respawned, all resources must be recreated.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub struct Respawned;
impl fmt::Display for Respawned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "view-process crashed and respawned, all resources must be rebuild")
    }
}
impl std::error::Error for Respawned {}

/// View Process IPC result.
pub type Result<T> = std::result::Result<T, Respawned>;

/// Data for rendering a new frame.
#[derive(Clone, Serialize, Deserialize)]
pub struct FrameRequest {
    /// Frame Tag.
    pub id: Epoch,
    /// Pipeline Tag.
    pub pipeline_id: PipelineId,

    /// Frame clear color.
    pub clear_color: ColorF,

    /// Display list, split in serializable parts.
    pub display_list: (ByteBuf, BuiltDisplayListDescriptor),
}
impl fmt::Debug for FrameRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FrameRequest")
            .field("id", &self.id)
            .field("pipeline_id", &self.pipeline_id)
            .finish_non_exhaustive()
    }
}

/// Configuration of a window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    /// Title text.
    pub title: String,
    /// Top-left offset, including the chrome (outer-position).
    pub pos: Option<DipPoint>,
    /// Content size (inner-size).
    pub size: DipSize,
    ///Initial window state.
    pub state: WindowState,

    /// Minimal size allowed.
    pub min_size: DipSize,
    /// Maximum size allowed.
    pub max_size: DipSize,

    /// Window visibility.
    pub visible: bool,
    /// Window taskbar icon visibility.
    pub taskbar_visible: bool,
    /// Window chrome visibility (decoration-visibility).
    pub chrome_visible: bool,
    /// In Windows, if `Alt+F4` does **not** causes a close request and instead causes a key-press event.
    pub allow_alt_f4: bool,
    /// If the window is "top-most".
    pub always_on_top: bool,
    /// If the user can move the window.
    pub movable: bool,
    /// If the user can resize the window.
    pub resizable: bool,
    /// Window icon.
    pub icon: Option<Icon>,
    /// If the window is see-through.
    pub transparent: bool,

    /// Text anti-aliasing.
    pub text_aa: TextAntiAliasing,
}

/// Configuration of a headless surface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadlessConfig {
    /// Scale for the layout units in this config.
    pub scale_factor: f32,

    /// Surface area (viewport size).
    pub size: DipSize,

    /// Text anti-aliasing.
    pub text_aa: TextAntiAliasing,
}

/// BGRA8 pixel data copied from a rendered frame.
#[derive(Clone, Serialize, Deserialize)]
pub struct FramePixels {
    /// Width in pixels.
    pub width: Px,
    /// Height in pixels.
    pub height: Px,

    /// BGRA8 data, bottom-to-top.
    pub bgra: ByteBuf,

    /// Scale factor when the frame was rendered.
    pub scale_factor: f32,

    /// If all alpha values are `255`.
    pub opaque: bool,
}
impl Default for FramePixels {
    fn default() -> Self {
        Self {
            width: Px(0),
            height: Px(0),
            bgra: ByteBuf::default(),
            scale_factor: 1.0,
            opaque: true,
        }
    }
}
impl fmt::Debug for FramePixels {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FramePixels")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("bgra", &format_args!("<{} bytes>", self.bgra.len()))
            .field("scale_factor", &self.scale_factor)
            .field("opaque", &self.opaque)
            .finish()
    }
}
impl FramePixels {
    /// Width in [`Dip`] units.
    pub fn width(&self) -> Dip {
        Dip::from_px(self.width, self.scale_factor)
    }

    /// Height in [`Dip`] units.
    pub fn height(&self) -> Dip {
        Dip::from_px(self.height, self.scale_factor)
    }
}

/// Information about a monitor screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorInfo {
    /// Readable name of the monitor.
    pub name: String,
    /// Top-left offset of the monitor region in the virtual screen, in pixels.
    pub position: PxPoint,
    /// Width/height of the monitor region in the virtual screen, in pixels.
    pub size: PxSize,
    /// The monitor scale factor.
    pub scale_factor: f32,
    /// Exclusive fullscreen video modes.
    pub video_modes: Vec<VideoMode>,

    /// If could determine this monitor is the primary.
    pub is_primary: bool,
}
impl MonitorInfo {
    /// Returns the `size` descaled using the `scale_factor`.
    #[inline]
    pub fn dip_size(&self) -> DipSize {
        self.size.to_dip(self.scale_factor)
    }
}
#[cfg(feature = "full")]
impl<'a> From<&'a glutin::monitor::MonitorHandle> for MonitorInfo {
    fn from(m: &'a glutin::monitor::MonitorHandle) -> Self {
        let position = m.position().to_px();
        let size = m.size().to_px();
        Self {
            name: m.name().unwrap_or_default(),
            position,
            size,
            scale_factor: m.scale_factor() as f32,
            video_modes: m.video_modes().map(Into::into).collect(),
            is_primary: false,
        }
    }
}
#[cfg(feature = "full")]
impl From<glutin::monitor::MonitorHandle> for MonitorInfo {
    fn from(m: glutin::monitor::MonitorHandle) -> Self {
        (&m).into()
    }
}

/// Exclusive video mode info.
///
/// You can get this values from [`MonitorInfo::video_modes`]. Note that when setting the
/// video mode the actual system mode is selected by approximation, closest `size`, then `bit_depth` then `refresh_rate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoMode {
    /// Resolution of this video mode.
    pub size: PxSize,
    /// the bit depth of this video mode, as in how many bits you have available per color.
    /// This is generally 24 bits or 32 bits on modern systems, depending on whether the alpha channel is counted or not.
    pub bit_depth: u16,
    /// The refresh rate of this video mode.
    ///
    /// Note: the returned refresh rate is an integer approximation, and you shouldn’t rely on this value to be exact.
    pub refresh_rate: u16,
}
#[cfg(feature = "full")]
impl From<glutin::monitor::VideoMode> for VideoMode {
    fn from(v: glutin::monitor::VideoMode) -> Self {
        let size = v.size();
        Self {
            size: PxSize::new(Px(size.width as i32), Px(size.height as i32)),
            bit_depth: v.bit_depth(),
            refresh_rate: v.refresh_rate(),
        }
    }
}

/// System settings needed for implementing double/triple clicks.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Deserialize)]
pub struct MultiClickConfig {
    /// Maximum time interval between clicks.
    ///
    /// Only repeated clicks within this time interval can count as double-clicks.
    pub time: Duration,

    /// Maximum (x, y) distance in pixels.
    ///
    /// Only repeated clicks that are within this distance of the first click can count as double-clicks.
    pub area: PxSize,
}
impl Default for MultiClickConfig {
    /// `500ms` and `4, 4`.
    fn default() -> Self {
        Self {
            time: Duration::from_millis(500),
            area: PxSize::new(Px(4), Px(4)),
        }
    }
}
