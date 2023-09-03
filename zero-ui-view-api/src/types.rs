use crate::units::*;
use crate::DisplayList;
use crate::FrameValueUpdate;
use crate::IpcBytes;
use serde::{Deserialize, Serialize};
use std::mem;
use std::ops;
use std::sync::Arc;
use std::time::Duration;
use std::{fmt, path::PathBuf};
use webrender_api::*;

macro_rules! declare_id {
    ($(
        $(#[$docs:meta])+
        pub struct $Id:ident(_);
    )+) => {$(
        $(#[$docs])+
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $Id(u32);

        impl $Id {
            /// Dummy ID, zero.
            pub const INVALID: Self = Self(0);

            /// Create the first valid ID.
            pub const fn first() -> Self {
                Self(1)
            }

            /// Create the next ID.
            ///
            /// IDs wrap around to [`first`] when the entire `u32` space is used, it is never `INVALID`.
            ///
            /// [`first`]: Self::first
            #[must_use]
            pub const fn next(self) -> Self {
                let r = Self(self.0.wrapping_add(1));
                if r.0 == Self::INVALID.0 {
                    Self::first()
                } else {
                    r
                }
            }

            /// Replace self with [`next`] and returns.
            ///
            /// [`next`]: Self::next
            #[must_use]
            pub fn incr(&mut self) -> Self {
                std::mem::replace(self, self.next())
            }

            /// Get the raw ID.
            pub const fn get(self) -> u32 {
                self.0
            }

            /// Create an ID using a custom value.
            ///
            /// Note that only the documented process must generate IDs, and that it must only
            /// generate IDs using this function or the [`next`] function.
            ///
            /// If the `id` is zero it will still be [`INVALID`] and handled differently by the other process,
            /// zero is never valid.
            ///
            /// [`next`]: Self::next
            /// [`INVALID`]: Self::INVALID
            pub const fn from_raw(id: u32) -> Self {
                Self(id)
            }
        }
    )+};
}

declare_id! {
    /// Window ID in channel.
    ///
    /// In the View Process this is mapped to a system id.
    ///
    /// In the App Process this is an unique id that survives View crashes.
    ///
    /// The App Process defines the ID.
    pub struct WindowId(_);

    /// Device ID in channel.
    ///
    /// In the View Process this is mapped to a system id.
    ///
    /// In the App Process this is mapped to an unique id, but does not survived View crashes.
    ///
    /// The View Process defines the ID.
    pub struct DeviceId(_);

    /// Monitor screen ID in channel.
    ///
    /// In the View Process this is mapped to a system id.
    ///
    /// In the App Process this is mapped to an unique id, but does not survived View crashes.
    ///
    /// The View Process defines the ID.
    pub struct MonitorId(_);

    /// Id of a decoded image in the cache.
    ///
    /// The View Process defines the ID.
    pub struct ImageId(_);

    /// View-process generation, starts at one and changes every respawn, it is never zero.
    ///
    /// The View Process defines the ID.
    pub struct ViewProcessGen(_);

    /// Identifies a frame request for collaborative resize in [`WindowChanged`].
    ///
    /// The View Process defines the ID.
    pub struct FrameWaitId(_);

    /// Identifies an ongoing async native dialog with the user.
    pub struct DialogId(_);
}

/// Identifier for a specific analog axis on some device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AxisId(pub u32);

/// Identifier for a specific button on some device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ButtonId(pub u32);

/// Identifier for a continuous touch contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TouchId(pub u64);

/// Identifier of a frame or frame update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, bytemuck::NoUninit)]
#[repr(C)]
pub struct FrameId(u32, u32);
impl FrameId {
    /// Dummy frame ID.
    pub const INVALID: FrameId = FrameId(u32::MAX, u32::MAX);

    /// Create first frame id of a window.
    pub fn first() -> FrameId {
        FrameId(0, 0)
    }

    /// Create the next full frame ID after the current one.
    pub fn next(self) -> FrameId {
        let mut id = self.0.wrapping_add(1);
        if id == u32::MAX {
            id = 0;
        }
        FrameId(id, 0)
    }

    /// Create the next update frame ID after the current one.
    pub fn next_update(self) -> FrameId {
        let mut id = self.1.wrapping_add(1);
        if id == u32::MAX {
            id = 0;
        }
        FrameId(self.0, id)
    }

    /// Get the raw ID.
    pub fn get(self) -> u64 {
        (self.0 as u64) << 32 | (self.1 as u64)
    }

    /// Get the full frame ID.
    pub fn epoch(self) -> Epoch {
        Epoch(self.0)
    }

    /// Get the frame update ID.
    pub fn update(self) -> u32 {
        self.1
    }
}

/// Pixels-per-inch of each dimension of an image.
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ImagePpi {
    /// Pixels-per-inch in the X dimension.
    pub x: f32,
    /// Pixels-per-inch in the Y dimension.
    pub y: f32,
}
impl ImagePpi {
    ///
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// New equal in both dimensions.
    pub const fn splat(xy: f32) -> Self {
        Self::new(xy, xy)
    }
}
impl Default for ImagePpi {
    /// 96.0
    fn default() -> Self {
        Self::splat(96.0)
    }
}
impl fmt::Debug for ImagePpi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() || self.x != self.y {
            f.debug_struct("ImagePpi").field("x", &self.x).field("y", &self.y).finish()
        } else {
            write!(f, "{}", self.x)
        }
    }
}
impl From<f32> for ImagePpi {
    fn from(xy: f32) -> Self {
        ImagePpi::splat(xy)
    }
}
impl From<(f32, f32)> for ImagePpi {
    fn from((x, y): (f32, f32)) -> Self {
        ImagePpi::new(x, y)
    }
}

/// State a [`Key`] has entered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyState {
    /// The key was pressed.
    Pressed,
    /// The key was released.
    Released,
}

/// State a [`MouseButton`] has entered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ButtonState {
    /// The button was pressed.
    Pressed,
    /// The button was released.
    Released,
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

/// Describes touch-screen input state.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum TouchPhase {
    /// A finger touched the screen.
    Start,
    /// A finger moved on the screen.
    Move,
    /// A finger was lifted from the screen.
    End,
    /// The system cancelled tracking for the touch.
    Cancel,
}

/// Describes a difference in the mouse scroll wheel state.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MouseScrollDelta {
    /// Amount in lines or rows to scroll in the horizontal
    /// and vertical directions.
    ///
    /// Positive values indicate rightwards movement in the X axis and downwards movement in the Y axis.
    LineDelta(f32, f32),
    /// Amount in pixels to scroll in the horizontal and
    /// vertical direction.
    ///
    /// Scroll events are expressed as a pixel delta if
    /// supported by the device (eg. a touchpad) and
    /// platform.
    PixelDelta(f32, f32),
}
impl MouseScrollDelta {
    /// Gets the sign status of x and y.
    ///
    /// Positive values indicate rightwards movement in the X axis and downwards movement in the Y axis.
    pub fn is_sign_positive(self) -> euclid::BoolVector2D {
        match self {
            MouseScrollDelta::LineDelta(x, y) | MouseScrollDelta::PixelDelta(x, y) => euclid::BoolVector2D {
                x: x.is_sign_positive(),
                y: y.is_sign_positive(),
            },
        }
    }

    /// Gets the sign status of x and y.
    ///
    /// Negative values indicate leftwards movement in the X axis and upwards movement in the Y axis.
    pub fn is_sign_negative(self) -> euclid::BoolVector2D {
        self.is_sign_positive().not()
    }

    /// Gets the pixel delta, line delta is converted using the `line_size`.
    pub fn delta(self, line_size: euclid::Size2D<f32, Px>) -> euclid::Vector2D<f32, Px> {
        match self {
            MouseScrollDelta::LineDelta(x, y) => euclid::vec2(line_size.width * x, line_size.height * y),
            MouseScrollDelta::PixelDelta(x, y) => euclid::vec2(x, y),
        }
    }
}

/// Contains the platform-native physical key identifier
///
/// The exact values vary from platform to platform (which is part of why this is a per-platform
/// enum), but the values are primarily tied to the key's physical location on the keyboard.
///
/// This enum is primarily used to store raw keycodes when Winit doesn't map a given native
/// physical key identifier to a meaningful [`KeyCode`] variant. In the presence of identifiers we
/// haven't mapped for you yet, this lets you use use [`KeyCode`] to:
///
/// - Correctly match key press and release events.
/// - On non-web platforms, support assigning key binds to virtually any key through a UI.
#[derive(Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum NativeKeyCode {
    /// Implementer did not identify system or scancode.
    Unidentified,
    /// An Android "scancode".
    Android(u32),
    /// A macOS "scancode".
    MacOS(u16),
    /// A Windows "scancode".
    Windows(u16),
    /// An XKB "keycode".
    Xkb(u32),
}
impl NativeKeyCode {
    /// Gets the variant name.
    pub fn name(self) -> &'static str {
        serde_variant::to_variant_name(&self).unwrap_or("")
    }
}
impl std::fmt::Debug for NativeKeyCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "NativeKeyCode::")?;
        }

        use NativeKeyCode::{Android, MacOS, Unidentified, Windows, Xkb};
        let mut debug_tuple;
        match self {
            Unidentified => {
                debug_tuple = f.debug_tuple("Unidentified");
            }
            Android(code) => {
                debug_tuple = f.debug_tuple("Android");
                debug_tuple.field(&format_args!("0x{code:04X}"));
            }
            MacOS(code) => {
                debug_tuple = f.debug_tuple("MacOS");
                debug_tuple.field(&format_args!("0x{code:04X}"));
            }
            Windows(code) => {
                debug_tuple = f.debug_tuple("Windows");
                debug_tuple.field(&format_args!("0x{code:04X}"));
            }
            Xkb(code) => {
                debug_tuple = f.debug_tuple("Xkb");
                debug_tuple.field(&format_args!("0x{code:04X}"));
            }
        }
        debug_tuple.finish()
    }
}

/// Represents the location of a physical key.
///
/// This mostly conforms to the UI Events Specification's [`KeyboardEvent.code`] with a few
/// exceptions:
/// - The keys that the specification calls "MetaLeft" and "MetaRight" are named "SuperLeft" and
///   "SuperRight" here.
/// - The key that the specification calls "Super" is reported as `Unidentified` here.
/// - The `Unidentified` variant here, can still identify a key through it's `NativeKeyCode`.
///
/// [`KeyboardEvent.code`]: https://w3c.github.io/uievents-code/#code-value-tables
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u16)]
pub enum KeyCode {
    // source: https://docs.rs/winit/0.29.0-beta.0/src/winit/keyboard.rs.html#201-649
    //
    //
    /// This variant is used when the key cannot be translated to any other variant.
    ///
    /// The native keycode is provided (if available) so you're able to more reliably match
    /// key-press and key-release events by hashing the [`KeyCode`]. It is also possible to use
    /// this for key-binds for non-standard keys, but such key-binds are tied to a given platform.
    Unidentified(NativeKeyCode),
    /// <kbd>`</kbd> on a US keyboard. This is also called a backtick or grave.
    /// This is the <kbd>半角</kbd>/<kbd>全角</kbd>/<kbd>漢字</kbd>
    /// (hankaku/zenkaku/kanji) key on Japanese keyboards
    Backquote = 1,
    /// Used for both the US <kbd>\\</kbd> (on the 101-key layout) and also for the key
    /// located between the <kbd>"</kbd> and <kbd>Enter</kbd> keys on row C of the 102-,
    /// 104- and 106-key layouts.
    /// Labeled <kbd>#</kbd> on a UK (102) keyboard.
    Backslash,
    /// <kbd>[</kbd> on a US keyboard.
    BracketLeft,
    /// <kbd>]</kbd> on a US keyboard.
    BracketRight,
    /// <kbd>,</kbd> on a US keyboard.
    Comma,
    /// <kbd>0</kbd> on a US keyboard.
    Digit0,
    /// <kbd>1</kbd> on a US keyboard.
    Digit1,
    /// <kbd>2</kbd> on a US keyboard.
    Digit2,
    /// <kbd>3</kbd> on a US keyboard.
    Digit3,
    /// <kbd>4</kbd> on a US keyboard.
    Digit4,
    /// <kbd>5</kbd> on a US keyboard.
    Digit5,
    /// <kbd>6</kbd> on a US keyboard.
    Digit6,
    /// <kbd>7</kbd> on a US keyboard.
    Digit7,
    /// <kbd>8</kbd> on a US keyboard.
    Digit8,
    /// <kbd>9</kbd> on a US keyboard.
    Digit9,
    /// <kbd>=</kbd> on a US keyboard.
    Equal,
    /// Located between the left <kbd>Shift</kbd> and <kbd>Z</kbd> keys.
    /// Labeled <kbd>\\</kbd> on a UK keyboard.
    IntlBackslash,
    /// Located between the <kbd>/</kbd> and right <kbd>Shift</kbd> keys.
    /// Labeled <kbd>\\</kbd> (ro) on a Japanese keyboard.
    IntlRo,
    /// Located between the <kbd>=</kbd> and <kbd>Backspace</kbd> keys.
    /// Labeled <kbd>¥</kbd> (yen) on a Japanese keyboard. <kbd>\\</kbd> on a
    /// Russian keyboard.
    IntlYen,
    /// <kbd>a</kbd> on a US keyboard.
    /// Labeled <kbd>q</kbd> on an AZERTY (e.g., French) keyboard.
    KeyA,
    /// <kbd>b</kbd> on a US keyboard.
    KeyB,
    /// <kbd>c</kbd> on a US keyboard.
    KeyC,
    /// <kbd>d</kbd> on a US keyboard.
    KeyD,
    /// <kbd>e</kbd> on a US keyboard.
    KeyE,
    /// <kbd>f</kbd> on a US keyboard.
    KeyF,
    /// <kbd>g</kbd> on a US keyboard.
    KeyG,
    /// <kbd>h</kbd> on a US keyboard.
    KeyH,
    /// <kbd>i</kbd> on a US keyboard.
    KeyI,
    /// <kbd>j</kbd> on a US keyboard.
    KeyJ,
    /// <kbd>k</kbd> on a US keyboard.
    KeyK,
    /// <kbd>l</kbd> on a US keyboard.
    KeyL,
    /// <kbd>m</kbd> on a US keyboard.
    KeyM,
    /// <kbd>n</kbd> on a US keyboard.
    KeyN,
    /// <kbd>o</kbd> on a US keyboard.
    KeyO,
    /// <kbd>p</kbd> on a US keyboard.
    KeyP,
    /// <kbd>q</kbd> on a US keyboard.
    /// Labeled <kbd>a</kbd> on an AZERTY (e.g., French) keyboard.
    KeyQ,
    /// <kbd>r</kbd> on a US keyboard.
    KeyR,
    /// <kbd>s</kbd> on a US keyboard.
    KeyS,
    /// <kbd>t</kbd> on a US keyboard.
    KeyT,
    /// <kbd>u</kbd> on a US keyboard.
    KeyU,
    /// <kbd>v</kbd> on a US keyboard.
    KeyV,
    /// <kbd>w</kbd> on a US keyboard.
    /// Labeled <kbd>z</kbd> on an AZERTY (e.g., French) keyboard.
    KeyW,
    /// <kbd>x</kbd> on a US keyboard.
    KeyX,
    /// <kbd>y</kbd> on a US keyboard.
    /// Labeled <kbd>z</kbd> on a QWERTZ (e.g., German) keyboard.
    KeyY,
    /// <kbd>z</kbd> on a US keyboard.
    /// Labeled <kbd>w</kbd> on an AZERTY (e.g., French) keyboard, and <kbd>y</kbd> on a
    /// QWERTZ (e.g., German) keyboard.
    KeyZ,
    /// <kbd>-</kbd> on a US keyboard.
    Minus,
    /// <kbd>.</kbd> on a US keyboard.
    Period,
    /// <kbd>'</kbd> on a US keyboard.
    Quote,
    /// <kbd>;</kbd> on a US keyboard.
    Semicolon,
    /// <kbd>/</kbd> on a US keyboard.
    Slash,
    /// <kbd>Alt</kbd>, <kbd>Option</kbd>, or <kbd>⌥</kbd>.
    AltLeft,
    /// <kbd>Alt</kbd>, <kbd>Option</kbd>, or <kbd>⌥</kbd>.
    /// This is labeled <kbd>AltGr</kbd> on many keyboard layouts.
    AltRight,
    /// <kbd>Backspace</kbd> or <kbd>⌫</kbd>.
    /// Labeled <kbd>Delete</kbd> on Apple keyboards.
    Backspace,
    /// <kbd>CapsLock</kbd> or <kbd>⇪</kbd>
    CapsLock,
    /// The application context menu key, which is typically found between the right
    /// <kbd>Super</kbd> key and the right <kbd>Ctrl</kbd> key.
    ContextMenu,
    /// <kbd>Ctrl</kbd> or <kbd>⌃</kbd>
    CtrlLeft,
    /// <kbd>Ctrl</kbd> or <kbd>⌃</kbd>
    CtrlRight,
    /// <kbd>Enter</kbd> or <kbd>↵</kbd>. Labeled <kbd>Return</kbd> on Apple keyboards.
    Enter,
    /// The Windows, <kbd>⌘</kbd>, <kbd>Command</kbd>, or other OS symbol key.
    SuperLeft,
    /// The Windows, <kbd>⌘</kbd>, <kbd>Command</kbd>, or other OS symbol key.
    SuperRight,
    /// <kbd>Shift</kbd> or <kbd>⇧</kbd>
    ShiftLeft,
    /// <kbd>Shift</kbd> or <kbd>⇧</kbd>
    ShiftRight,
    /// <kbd> </kbd> (space)
    Space,
    /// <kbd>Tab</kbd> or <kbd>⇥</kbd>
    Tab,
    /// Japanese: <kbd>変</kbd> (henkan)
    Convert,
    /// Japanese: <kbd>カタカナ</kbd>/<kbd>ひらがな</kbd>/<kbd>ローマ字</kbd> (katakana/hiragana/romaji)
    KanaMode,
    /// Korean: HangulMode <kbd>한/영</kbd> (han/yeong)
    ///
    /// Japanese (Mac keyboard): <kbd>か</kbd> (kana)
    Lang1,
    /// Korean: Hanja <kbd>한</kbd> (hanja)
    ///
    /// Japanese (Mac keyboard): <kbd>英</kbd> (eisu)
    Lang2,
    /// Japanese (word-processing keyboard): Katakana
    Lang3,
    /// Japanese (word-processing keyboard): Hiragana
    Lang4,
    /// Japanese (word-processing keyboard): Zenkaku/Hankaku
    Lang5,
    /// Japanese: <kbd>無変換</kbd> (muhenkan)
    NonConvert,
    /// <kbd>⌦</kbd>. The forward delete key.
    /// Note that on Apple keyboards, the key labelled <kbd>Delete</kbd> on the main part of
    /// the keyboard is encoded as [`Backspace`].
    ///
    /// [`Backspace`]: Self::Backspace
    Delete,
    /// <kbd>Page Down</kbd>, <kbd>End</kbd>, or <kbd>↘</kbd>
    End,
    /// <kbd>Help</kbd>. Not present on standard PC keyboards.
    Help,
    /// <kbd>Home</kbd> or <kbd>↖</kbd>
    Home,
    /// <kbd>Insert</kbd> or <kbd>Ins</kbd>. Not present on Apple keyboards.
    Insert,
    /// <kbd>Page Down</kbd>, <kbd>PgDn</kbd>, or <kbd>⇟</kbd>
    PageDown,
    /// <kbd>Page Up</kbd>, <kbd>PgUp</kbd>, or <kbd>⇞</kbd>
    PageUp,
    /// <kbd>↓</kbd>
    ArrowDown,
    /// <kbd>←</kbd>
    ArrowLeft,
    /// <kbd>→</kbd>
    ArrowRight,
    /// <kbd>↑</kbd>
    ArrowUp,
    /// On the Mac, this is used for the numpad <kbd>Clear</kbd> key.
    NumLock,
    /// <kbd>0 Ins</kbd> on a keyboard. <kbd>0</kbd> on a phone or remote control
    Numpad0,
    /// <kbd>1 End</kbd> on a keyboard. <kbd>1</kbd> or <kbd>1 QZ</kbd> on a phone or remote control
    Numpad1,
    /// <kbd>2 ↓</kbd> on a keyboard. <kbd>2 ABC</kbd> on a phone or remote control
    Numpad2,
    /// <kbd>3 PgDn</kbd> on a keyboard. <kbd>3 DEF</kbd> on a phone or remote control
    Numpad3,
    /// <kbd>4 ←</kbd> on a keyboard. <kbd>4 GHI</kbd> on a phone or remote control
    Numpad4,
    /// <kbd>5</kbd> on a keyboard. <kbd>5 JKL</kbd> on a phone or remote control
    Numpad5,
    /// <kbd>6 →</kbd> on a keyboard. <kbd>6 MNO</kbd> on a phone or remote control
    Numpad6,
    /// <kbd>7 Home</kbd> on a keyboard. <kbd>7 PQRS</kbd> or <kbd>7 PRS</kbd> on a phone
    /// or remote control
    Numpad7,
    /// <kbd>8 ↑</kbd> on a keyboard. <kbd>8 TUV</kbd> on a phone or remote control
    Numpad8,
    /// <kbd>9 PgUp</kbd> on a keyboard. <kbd>9 WXYZ</kbd> or <kbd>9 WXY</kbd> on a phone
    /// or remote control
    Numpad9,
    /// <kbd>+</kbd>
    NumpadAdd,
    /// Found on the Microsoft Natural Keyboard.
    NumpadBackspace,
    /// <kbd>C</kbd> or <kbd>A</kbd> (All Clear). Also for use with numpads that have a
    /// <kbd>Clear</kbd> key that is separate from the <kbd>NumLock</kbd> key. On the Mac, the
    /// numpad <kbd>Clear</kbd> key is encoded as [`NumLock`].
    ///
    /// [`NumLock`]: Self::NumLock
    NumpadClear,
    /// <kbd>C</kbd> (Clear Entry)
    NumpadClearEntry,
    /// <kbd>,</kbd> (thousands separator). For locales where the thousands separator
    /// is a "." (e.g., Brazil), this key may generate a <kbd>.</kbd>.
    NumpadComma,
    /// <kbd>. Del</kbd>. For locales where the decimal separator is "," (e.g.,
    /// Brazil), this key may generate a <kbd>,</kbd>.
    NumpadDecimal,
    /// <kbd>/</kbd>
    NumpadDivide,
    /// <kbd>↵</kbd>
    NumpadEnter,
    /// <kbd>=</kbd>
    NumpadEqual,
    /// <kbd>#</kbd> on a phone or remote control device. This key is typically found
    /// below the <kbd>9</kbd> key and to the right of the <kbd>0</kbd> key.
    NumpadHash,
    /// <kbd>M</kbd> Add current entry to the value stored in memory.
    NumpadMemoryAdd,
    /// <kbd>M</kbd> Clear the value stored in memory.
    NumpadMemoryClear,
    /// <kbd>M</kbd> Replace the current entry with the value stored in memory.
    NumpadMemoryRecall,
    /// <kbd>M</kbd> Replace the value stored in memory with the current entry.
    NumpadMemoryStore,
    /// <kbd>M</kbd> Subtract current entry from the value stored in memory.
    NumpadMemorySubtract,
    /// <kbd>*</kbd> on a keyboard. For use with numpads that provide mathematical
    /// operations (<kbd>+</kbd>, <kbd>-</kbd> <kbd>*</kbd> and <kbd>/</kbd>).
    ///
    /// Use `NumpadStar` for the <kbd>*</kbd> key on phones and remote controls.
    NumpadMultiply,
    /// <kbd>(</kbd> Found on the Microsoft Natural Keyboard.
    NumpadParenLeft,
    /// <kbd>)</kbd> Found on the Microsoft Natural Keyboard.
    NumpadParenRight,
    /// <kbd>*</kbd> on a phone or remote control device.
    ///
    /// This key is typically found below the <kbd>7</kbd> key and to the left of
    /// the <kbd>0</kbd> key.
    ///
    /// Use <kbd>"NumpadMultiply"</kbd> for the <kbd>*</kbd> key on
    /// numeric keypads.
    NumpadStar,
    /// <kbd>-</kbd>
    NumpadSubtract,
    /// <kbd>Esc</kbd> or <kbd>⎋</kbd>
    Escape,
    /// <kbd>Fn</kbd> This is typically a hardware key that does not generate a separate code.
    Fn,
    /// <kbd>FLock</kbd> or <kbd>FnLock</kbd>. Function Lock key. Found on the Microsoft
    /// Natural Keyboard.
    FnLock,
    /// <kbd>PrtScr SysRq</kbd> or <kbd>Print Screen</kbd>
    PrintScreen,
    /// <kbd>Scroll Lock</kbd>
    ScrollLock,
    /// <kbd>Pause Break</kbd>
    Pause,
    /// Some laptops place this key to the left of the <kbd>↑</kbd> key.
    ///
    /// This also the "back" button (triangle) on Android.
    BrowserBack,
    ///
    BrowserFavorites,
    /// Some laptops place this key to the right of the <kbd>↑</kbd> key.
    BrowserForward,
    /// The "home" button on Android.
    BrowserHome,
    ///
    BrowserRefresh,
    ///
    BrowserSearch,
    ///
    BrowserStop,
    /// <kbd>Eject</kbd> or <kbd>⏏</kbd>. This key is placed in the function section on some Apple
    /// keyboards.
    Eject,
    /// Sometimes labelled <kbd>My Computer</kbd> on the keyboard
    LaunchApp1,
    /// Sometimes labelled <kbd>Calculator</kbd> on the keyboard
    LaunchApp2,
    /// <kbd>✉</kbd>
    LaunchMail,
    /// <kbd>⏯</kbd>
    MediaPlayPause,
    ///
    MediaSelect,
    /// <kbd>⏹</kbd>
    MediaStop,
    /// <kbd>⏭</kbd>
    MediaTrackNext,
    /// <kbd>⏮</kbd>
    MediaTrackPrevious,
    /// This key is placed in the function section on some Apple keyboards, replacing the
    /// <kbd>Eject</kbd> key.
    Power,
    ///
    Sleep,
    ///
    AudioVolumeDown,
    ///
    AudioVolumeMute,
    ///
    AudioVolumeUp,
    ///
    WakeUp,
    /// Legacy modifier key. Also called "Super" in certain places.
    Meta,
    /// Legacy modifier key.
    Hyper,
    ///
    Turbo,
    ///
    Abort,
    ///
    Resume,
    ///
    Suspend,
    /// Found on Sun’s USB keyboard.
    Again,
    /// Found on Sun’s USB keyboard.
    Copy,
    /// Found on Sun’s USB keyboard.
    Cut,
    /// Found on Sun’s USB keyboard.
    Find,
    /// Found on Sun’s USB keyboard.
    Open,
    /// Found on Sun’s USB keyboard.
    Paste,
    /// Found on Sun’s USB keyboard.
    Props,
    /// Found on Sun’s USB keyboard.
    Select,
    /// Found on Sun’s USB keyboard.
    Undo,
    /// Use for dedicated <kbd>ひらがな</kbd> key found on some Japanese word processing keyboards.
    Hiragana,
    /// Use for dedicated <kbd>カタカナ</kbd> key found on some Japanese word processing keyboards.
    Katakana,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F1,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F2,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F3,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F4,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F5,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F6,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F7,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F8,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F9,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F10,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F11,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F12,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F13,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F14,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F15,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F16,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F17,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F18,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F19,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F20,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F21,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F22,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F23,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F24,
    /// General-purpose function key.
    F25,
    /// General-purpose function key.
    F26,
    /// General-purpose function key.
    F27,
    /// General-purpose function key.
    F28,
    /// General-purpose function key.
    F29,
    /// General-purpose function key.
    F30,
    /// General-purpose function key.
    F31,
    /// General-purpose function key.
    F32,
    /// General-purpose function key.
    F33,
    /// General-purpose function key.
    F34,
    /// General-purpose function key.
    F35,
}
impl KeyCode {
    /// If key-code is fully unidentified ([`NativeKeyCode::Unidentified`]).
    pub fn is_unidentified(&self) -> bool {
        matches!(self, KeyCode::Unidentified(NativeKeyCode::Unidentified))
    }

    /// If the keycode represents a known and identified modifier.
    pub fn is_modifier(&self) -> bool {
        matches!(
            self,
            KeyCode::AltLeft
                | KeyCode::AltRight
                | KeyCode::CtrlLeft
                | KeyCode::CtrlRight
                | KeyCode::ShiftLeft
                | KeyCode::ShiftRight
                | KeyCode::SuperLeft
                | KeyCode::SuperRight
                | KeyCode::CapsLock
                | KeyCode::Fn
                | KeyCode::FnLock
                | KeyCode::Meta
                | KeyCode::NumLock
                | KeyCode::ScrollLock
                | KeyCode::Hyper
        )
    }

    /// If the key if for IME composition actions as defined by [w3].
    ///
    /// [w3]: https://www.w3.org/TR/uievents-key/#keys-composition
    pub fn is_composition(&self) -> bool {
        matches!(self, |KeyCode::Convert| KeyCode::NonConvert
            | KeyCode::Hiragana
            | KeyCode::KanaMode
            | KeyCode::Katakana)
    }

    /// If the key is for an edit action as defined by [w3].
    ///
    /// [w3]: https://www.w3.org/TR/uievents-key/#keys-editing
    pub fn is_editing(&self) -> bool {
        matches!(
            self,
            KeyCode::Backspace | KeyCode::Cut | KeyCode::Delete | KeyCode::Insert | KeyCode::Paste | KeyCode::Undo
        )
    }

    /// If the key is for an general UI action as defined by [w3].
    ///
    /// [w3]: https://www.w3.org/TR/uievents-key/#keys-ui
    pub fn is_ui(&self) -> bool {
        matches!(
            self,
            KeyCode::Again
                | KeyCode::ContextMenu
                | KeyCode::Escape
                | KeyCode::Find
                | KeyCode::Help
                | KeyCode::Pause
                | KeyCode::Props
                | KeyCode::Select
        )
    }

    /// If the key is for an general device action as defined by [w3].
    ///
    /// [w3]: https://www.w3.org/TR/uievents-key/#keys-device
    pub fn is_device(&self) -> bool {
        matches!(self, |KeyCode::Eject| KeyCode::Power | KeyCode::PrintScreen | KeyCode::WakeUp)
    }

    /// If the key is one of the general purpose function keys.
    pub fn is_function(&self) -> bool {
        matches!(
            self,
            KeyCode::F1
                | KeyCode::F2
                | KeyCode::F3
                | KeyCode::F4
                | KeyCode::F5
                | KeyCode::F6
                | KeyCode::F7
                | KeyCode::F8
                | KeyCode::F9
                | KeyCode::F10
                | KeyCode::F11
                | KeyCode::F12
                | KeyCode::F13
                | KeyCode::F14
                | KeyCode::F15
                | KeyCode::F16
                | KeyCode::F17
                | KeyCode::F18
                | KeyCode::F19
                | KeyCode::F20
                | KeyCode::F21
                | KeyCode::F22
                | KeyCode::F23
                | KeyCode::F24
                | KeyCode::F25
                | KeyCode::F26
                | KeyCode::F27
                | KeyCode::F28
                | KeyCode::F29
                | KeyCode::F30
                | KeyCode::F31
                | KeyCode::F32
                | KeyCode::F33
                | KeyCode::F34
                | KeyCode::F35
        )
    }

    /// If the key is for an multimedia control as defined by [w3].
    ///
    /// [w3]: https://www.w3.org/TR/uievents-key/#keys-multimedia
    pub fn is_multimedia(&self) -> bool {
        matches!(
            self,
            KeyCode::MediaPlayPause | KeyCode::MediaStop | KeyCode::MediaTrackNext | KeyCode::MediaTrackPrevious | KeyCode::Open
        )
    }

    /// If the key is for an audio control as defined by [w3].
    ///
    /// [w3]: https://www.w3.org/TR/uievents-key/#keys-audio
    pub fn is_audio(&self) -> bool {
        matches!(self, KeyCode::AudioVolumeDown | KeyCode::AudioVolumeUp | KeyCode::AudioVolumeMute)
    }

    /// If the key is for launching an application.
    pub fn is_launch(&self) -> bool {
        matches!(self, KeyCode::LaunchMail)
    }

    /// If the key is for a browser control.
    pub fn is_browser(&self) -> bool {
        matches!(
            self,
            KeyCode::BrowserBack
                | KeyCode::BrowserFavorites
                | KeyCode::BrowserForward
                | KeyCode::BrowserHome
                | KeyCode::BrowserRefresh
                | KeyCode::BrowserSearch
                | KeyCode::BrowserStop
        )
    }

    /// Iterate over all identified values.
    ///
    /// The first value is `Backquote` the last is `F35`.
    pub fn all_identified() -> impl ExactSizeIterator<Item = KeyCode> + DoubleEndedIterator {
        unsafe {
            // SAFETY: this is safe because the variants are without associated data.
            let e: (u16, [u8; 9]) = mem::transmute(KeyCode::F35);
            (1..=e.0).map(|n| mem::transmute((n, [0u8; 9])))
        }
    }

    /// Gets the key a a static str.
    pub fn name(self) -> &'static str {
        serde_variant::to_variant_name(&self).unwrap_or("")
    }
}
/// Gets the identified key name or `Unidentified`
impl std::str::FromStr for KeyCode {
    type Err = KeyCode;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for v in Self::all_identified() {
            if v.name() == s {
                return Ok(v);
            }
        }
        Err(KeyCode::Unidentified(NativeKeyCode::Unidentified))
    }
}
impl fmt::Debug for KeyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "KeyCode::")?;
        }
        let name = self.name();
        match self {
            Self::Unidentified(u) => write!(f, "{name}({u:?})"),
            _ => write!(f, "{name}"),
        }
    }
}

/// Key represents the meaning of a key press.
///
/// This mostly conforms to the UI Events Specification's [`KeyboardEvent.key`] with a few
/// exceptions:
/// - The `Super` variant here, is named `Meta` in the aforementioned specification. (There's
///   another key which the specification calls `Super`. That does not exist here.)
/// - The `Space` variant here, can be identified by the character it generates in the
///   specification.
/// - The `Dead` variant here, can specify the character which is inserted when pressing the
///   dead-key twice.
///
/// [`KeyboardEvent.key`]: https://w3c.github.io/uievents-key/
#[non_exhaustive]
#[derive(Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u16)]
pub enum Key {
    /// A key that corresponds to the character typed by the user, taking into account the
    /// user’s current locale setting, and any system-level keyboard mapping overrides that are in
    /// effect.
    Char(char),

    /// A key string that corresponds to the character typed by the user, taking into account the
    /// user’s current locale setting, and any system-level keyboard mapping overrides that are in
    /// effect.
    Str(Arc<str>),

    /// This variant is used when the key cannot be translated to any other variant.
    ///
    /// You can try using the [`KeyCode`] to identify the key.
    Unidentified,

    /// Contains the text representation of the dead-key when available.
    ///
    /// ## Platform-specific
    /// - **Web:** Always contains `None`
    Dead(Option<char>),

    /// The `Alt` (Alternative) key.
    ///
    /// This key enables the alternate modifier function for interpreting concurrent or subsequent
    /// keyboard input. This key value is also used for the Apple <kbd>Option</kbd> key.
    Alt = 4,
    /// The Alternate Graphics (<kbd>AltGr</kbd> or <kbd>AltGraph</kbd>) key.
    ///
    /// This key is used enable the ISO Level 3 shift modifier (the standard `Shift` key is the
    /// level 2 modifier).
    AltGraph,
    /// The `Caps Lock` (Capital) key.
    ///
    /// Toggle capital character lock function for interpreting subsequent keyboard input event.
    CapsLock,
    /// The `Control` or `Ctrl` key.
    ///
    /// Used to enable control modifier function for interpreting concurrent or subsequent keyboard
    /// input.
    Ctrl,
    /// The Function switch `Fn` key. Activating this key simultaneously with another key changes
    /// that key’s value to an alternate character or function. This key is often handled directly
    /// in the keyboard hardware and does not usually generate key events.
    Fn,
    /// The Function-Lock (`FnLock` or `F-Lock`) key. Activating this key switches the mode of the
    /// keyboard to changes some keys' values to an alternate character or function. This key is
    /// often handled directly in the keyboard hardware and does not usually generate key events.
    FnLock,
    /// The `NumLock` or Number Lock key. Used to toggle numpad mode function for interpreting
    /// subsequent keyboard input.
    NumLock,
    /// Toggle between scrolling and cursor movement modes.
    ScrollLock,
    /// Used to enable shift modifier function for interpreting concurrent or subsequent keyboard
    /// input.
    Shift,
    /// The Symbol modifier key (used on some virtual keyboards).
    Symbol,
    ///
    SymbolLock,
    /// Legacy modifier key. Also called "Super" in certain places.
    Meta,
    /// Legacy modifier key.
    Hyper,
    /// Used to enable "super" modifier function for interpreting concurrent or subsequent keyboard
    /// input. This key value is used for the "Windows Logo" key and the Apple `Command` or `⌘` key.
    ///
    /// Note: In some contexts (e.g. the Web) this is referred to as the "Meta" key.
    Super,
    /// The `Enter` or `↵` key. Used to activate current selection or accept current input. This key
    /// value is also used for the `Return` (Macintosh numpad) key. This key value is also used for
    /// the Android `KEYCODE_DPAD_CENTER`.
    Enter,
    /// The Horizontal Tabulation `Tab` key.
    Tab,
    /// Used in text to insert a space between words. Usually located below the character keys.
    Space,
    /// Navigate or traverse downward. (`KEYCODE_DPAD_DOWN`)
    ArrowDown,
    /// Navigate or traverse leftward. (`KEYCODE_DPAD_LEFT`)
    ArrowLeft,
    /// Navigate or traverse rightward. (`KEYCODE_DPAD_RIGHT`)
    ArrowRight,
    /// Navigate or traverse upward. (`KEYCODE_DPAD_UP`)
    ArrowUp,
    /// The End key, used with keyboard entry to go to the end of content (`KEYCODE_MOVE_END`).
    End,
    /// The Home key, used with keyboard entry, to go to start of content (`KEYCODE_MOVE_HOME`).
    /// For the mobile phone `Home` key (which goes to the phone’s main screen), use [`GoHome`].
    ///
    /// [`GoHome`]: Self::GoHome
    Home,
    /// Scroll down or display next page of content.
    PageDown,
    /// Scroll up or display previous page of content.
    PageUp,
    /// Used to remove the character to the left of the cursor. This key value is also used for
    /// the key labeled `Delete` on MacOS keyboards.
    Backspace,
    /// Remove the currently selected input.
    Clear,
    /// Copy the current selection. (`APPCOMMAND_COPY`)
    Copy,
    /// The Cursor Select key.
    CrSel,
    /// Cut the current selection. (`APPCOMMAND_CUT`)
    Cut,
    /// Used to delete the character to the right of the cursor. This key value is also used for the
    /// key labeled `Delete` on MacOS keyboards when `Fn` is active.
    Delete,
    /// The Erase to End of Field key. This key deletes all characters from the current cursor
    /// position to the end of the current field.
    EraseEof,
    /// The Extend Selection key.
    ExSel,
    /// Toggle between text modes for insertion or overtyping.
    /// (`KEYCODE_INSERT`)
    Insert,
    /// The Paste key. (`APPCOMMAND_PASTE`)
    Paste,
    /// Redo the last action. (`APPCOMMAND_REDO`)
    Redo,
    /// Undo the last action. (`APPCOMMAND_UNDO`)
    Undo,
    /// The Accept (Commit, OK) key. Accept current option or input method sequence conversion.
    Accept,
    /// Redo or repeat an action.
    Again,
    /// The Attention (Attn) key.
    Attn,
    ///
    Cancel,
    /// Show the application’s context menu.
    /// This key is commonly found between the right `Super` key and the right `Ctrl` key.
    ContextMenu,
    /// The `Esc` key. This key was originally used to initiate an escape sequence, but is
    /// now more generally used to exit or "escape" the current context, such as closing a dialog
    /// or exiting full screen mode.
    Escape,
    ///
    Execute,
    /// Open the Find dialog. (`APPCOMMAND_FIND`)
    Find,
    /// Open a help dialog or toggle display of help information. (`APPCOMMAND_HELP`,
    /// `KEYCODE_HELP`)
    Help,
    /// Pause the current state or application (as appropriate).
    ///
    /// Note: Do not use this value for the `Pause` button on media controllers. Use `"MediaPause"`
    /// instead.
    Pause,
    /// Play or resume the current state or application (as appropriate).
    ///
    /// Note: Do not use this value for the `Play` button on media controllers. Use `"MediaPlay"`
    /// instead.
    Play,
    /// The properties (Props) key.
    Props,
    ///
    Select,
    /// The ZoomIn key. (`KEYCODE_ZOOM_IN`)
    ZoomIn,
    /// The ZoomOut key. (`KEYCODE_ZOOM_OUT`)
    ZoomOut,
    /// The Brightness Down key. Typically controls the display brightness.
    /// (`KEYCODE_BRIGHTNESS_DOWN`)
    BrightnessDown,
    /// The Brightness Up key. Typically controls the display brightness. (`KEYCODE_BRIGHTNESS_UP`)
    BrightnessUp,
    /// Toggle removable media to eject (open) and insert (close) state. (`KEYCODE_MEDIA_EJECT`)
    Eject,
    ///
    LogOff,
    /// Toggle power state. (`KEYCODE_POWER`)
    /// Note: Note: Some devices might not expose this key to the operating environment.
    Power,
    /// The `PowerOff` key. Sometime called `PowerDown`.
    PowerOff,
    /// Initiate print-screen function.
    PrintScreen,
    /// The Hibernate key. This key saves the current state of the computer to disk so that it can
    /// be restored. The computer will then shutdown.
    Hibernate,
    /// The Standby key. This key turns off the display and places the computer into a low-power
    /// mode without completely shutting down. It is sometimes labelled `Suspend` or `Sleep` key.
    /// (`KEYCODE_SLEEP`)
    Standby,
    /// The WakeUp key. (`KEYCODE_WAKEUP`)
    WakeUp,
    /// Initiate the multi-candidate mode.
    AllCandidates,
    ///
    Alphanumeric,
    /// Initiate the Code Input mode to allow characters to be entered by
    /// their code points.
    CodeInput,
    /// The Compose key, also known as "Multi_key" on the X Window System. This key acts in a
    /// manner similar to a dead key, triggering a mode where subsequent key presses are combined to
    /// produce a different character.
    Compose,
    /// Convert the current input method sequence.
    Convert,
    /// The Final Mode `Final` key used on some Asian keyboards, to enable the final mode for IMEs.
    FinalMode,
    /// Switch to the first character group. (ISO/IEC 9995)
    GroupFirst,
    /// Switch to the last character group. (ISO/IEC 9995)
    GroupLast,
    /// Switch to the next character group. (ISO/IEC 9995)
    GroupNext,
    /// Switch to the previous character group. (ISO/IEC 9995)
    GroupPrevious,
    /// Toggle between or cycle through input modes of IMEs.
    ModeChange,
    ///
    NextCandidate,
    /// Accept current input method sequence without
    /// conversion in IMEs.
    NonConvert,
    ///
    PreviousCandidate,
    ///
    Process,
    ///
    SingleCandidate,
    /// Toggle between Hangul and English modes.
    HangulMode,
    ///
    HanjaMode,
    ///
    JunjaMode,
    /// The Eisu key. This key may close the IME, but its purpose is defined by the current IME.
    /// (`KEYCODE_EISU`)
    Eisu,
    /// The (Half-Width) Characters key.
    Hankaku,
    /// The Hiragana (Japanese Kana characters) key.
    Hiragana,
    /// The Hiragana/Katakana toggle key. (`KEYCODE_KATAKANA_HIRAGANA`)
    HiraganaKatakana,
    /// The Kana Mode (Kana Lock) key. This key is used to enter hiragana mode (typically from
    /// romaji mode).
    KanaMode,
    /// The Kanji (Japanese name for ideographic characters of Chinese origin) Mode key. This key is
    /// typically used to switch to a hiragana keyboard for the purpose of converting input into
    /// kanji. (`KEYCODE_KANA`)
    KanjiMode,
    /// The Katakana (Japanese Kana characters) key.
    Katakana,
    /// The Roman characters function key.
    Romaji,
    /// The Zenkaku (Full-Width) Characters key.
    Zenkaku,
    /// The Zenkaku/Hankaku (full-width/half-width) toggle key. (`KEYCODE_ZENKAKU_HANKAKU`)
    ZenkakuHankaku,
    /// General purpose virtual function key, as index 1.
    Soft1,
    /// General purpose virtual function key, as index 2.
    Soft2,
    /// General purpose virtual function key, as index 3.
    Soft3,
    /// General purpose virtual function key, as index 4.
    Soft4,
    /// Select next (numerically or logically) lower channel. (`APPCOMMAND_MEDIA_CHANNEL_DOWN`,
    /// `KEYCODE_CHANNEL_DOWN`)
    ChannelDown,
    /// Select next (numerically or logically) higher channel. (`APPCOMMAND_MEDIA_CHANNEL_UP`,
    /// `KEYCODE_CHANNEL_UP`)
    ChannelUp,
    /// Close the current document or message (Note: This doesn’t close the application).
    /// (`APPCOMMAND_CLOSE`)
    Close,
    /// Open an editor to forward the current message. (`APPCOMMAND_FORWARD_MAIL`)
    MailForward,
    /// Open an editor to reply to the current message. (`APPCOMMAND_REPLY_TO_MAIL`)
    MailReply,
    /// Send the current message. (`APPCOMMAND_SEND_MAIL`)
    MailSend,
    /// Close the current media, for example to close a CD or DVD tray. (`KEYCODE_MEDIA_CLOSE`)
    MediaClose,
    /// Initiate or continue forward playback at faster than normal speed, or increase speed if
    /// already fast forwarding. (`APPCOMMAND_MEDIA_FAST_FORWARD`, `KEYCODE_MEDIA_FAST_FORWARD`)
    MediaFastForward,
    /// Pause the currently playing media. (`APPCOMMAND_MEDIA_PAUSE`, `KEYCODE_MEDIA_PAUSE`)
    ///
    /// Note: Media controller devices should use this value rather than `"Pause"` for their pause
    /// keys.
    MediaPause,
    /// Initiate or continue media playback at normal speed, if not currently playing at normal
    /// speed. (`APPCOMMAND_MEDIA_PLAY`, `KEYCODE_MEDIA_PLAY`)
    MediaPlay,
    /// Toggle media between play and pause states. (`APPCOMMAND_MEDIA_PLAY_PAUSE`,
    /// `KEYCODE_MEDIA_PLAY_PAUSE`)
    MediaPlayPause,
    /// Initiate or resume recording of currently selected media. (`APPCOMMAND_MEDIA_RECORD`,
    /// `KEYCODE_MEDIA_RECORD`)
    MediaRecord,
    /// Initiate or continue reverse playback at faster than normal speed, or increase speed if
    /// already rewinding. (`APPCOMMAND_MEDIA_REWIND`, `KEYCODE_MEDIA_REWIND`)
    MediaRewind,
    /// Stop media playing, pausing, forwarding, rewinding, or recording, if not already stopped.
    /// (`APPCOMMAND_MEDIA_STOP`, `KEYCODE_MEDIA_STOP`)
    MediaStop,
    /// Seek to next media or program track. (`APPCOMMAND_MEDIA_NEXTTRACK`, `KEYCODE_MEDIA_NEXT`)
    MediaTrackNext,
    /// Seek to previous media or program track. (`APPCOMMAND_MEDIA_PREVIOUSTRACK`,
    /// `KEYCODE_MEDIA_PREVIOUS`)
    MediaTrackPrevious,
    /// Open a new document or message. (`APPCOMMAND_NEW`)
    New,
    /// Open an existing document or message. (`APPCOMMAND_OPEN`)
    Open,
    /// Print the current document or message. (`APPCOMMAND_PRINT`)
    Print,
    /// Save the current document or message. (`APPCOMMAND_SAVE`)
    Save,
    /// Spellcheck the current document or selection. (`APPCOMMAND_SPELL_CHECK`)
    SpellCheck,
    /// The `11` key found on media numpads that
    /// have buttons from `1` ... `12`.
    Key11,
    /// The `12` key found on media numpads that
    /// have buttons from `1` ... `12`.
    Key12,
    /// Adjust audio balance leftward. (`VK_AUDIO_BALANCE_LEFT`)
    AudioBalanceLeft,
    /// Adjust audio balance rightward. (`VK_AUDIO_BALANCE_RIGHT`)
    AudioBalanceRight,
    /// Decrease audio bass boost or cycle down through bass boost states. (`APPCOMMAND_BASS_DOWN`,
    /// `VK_BASS_BOOST_DOWN`)
    AudioBassBoostDown,
    /// Toggle bass boost on/off. (`APPCOMMAND_BASS_BOOST`)
    AudioBassBoostToggle,
    /// Increase audio bass boost or cycle up through bass boost states. (`APPCOMMAND_BASS_UP`,
    /// `VK_BASS_BOOST_UP`)
    AudioBassBoostUp,
    /// Adjust audio fader towards front. (`VK_FADER_FRONT`)
    AudioFaderFront,
    /// Adjust audio fader towards rear. (`VK_FADER_REAR`)
    AudioFaderRear,
    /// Advance surround audio mode to next available mode. (`VK_SURROUND_MODE_NEXT`)
    AudioSurroundModeNext,
    /// Decrease treble. (`APPCOMMAND_TREBLE_DOWN`)
    AudioTrebleDown,
    /// Increase treble. (`APPCOMMAND_TREBLE_UP`)
    AudioTrebleUp,
    /// Decrease audio volume. (`APPCOMMAND_VOLUME_DOWN`, `KEYCODE_VOLUME_DOWN`)
    AudioVolumeDown,
    /// Increase audio volume. (`APPCOMMAND_VOLUME_UP`, `KEYCODE_VOLUME_UP`)
    AudioVolumeUp,
    /// Toggle between muted state and prior volume level. (`APPCOMMAND_VOLUME_MUTE`,
    /// `KEYCODE_VOLUME_MUTE`)
    AudioVolumeMute,
    /// Toggle the microphone on/off. (`APPCOMMAND_MIC_ON_OFF_TOGGLE`)
    MicrophoneToggle,
    /// Decrease microphone volume. (`APPCOMMAND_MICROPHONE_VOLUME_DOWN`)
    MicrophoneVolumeDown,
    /// Increase microphone volume. (`APPCOMMAND_MICROPHONE_VOLUME_UP`)
    MicrophoneVolumeUp,
    /// Mute the microphone. (`APPCOMMAND_MICROPHONE_VOLUME_MUTE`, `KEYCODE_MUTE`)
    MicrophoneVolumeMute,
    /// Show correction list when a word is incorrectly identified. (`APPCOMMAND_CORRECTION_LIST`)
    SpeechCorrectionList,
    /// Toggle between dictation mode and command/control mode.
    /// (`APPCOMMAND_DICTATE_OR_COMMAND_CONTROL_TOGGLE`)
    SpeechInputToggle,
    /// The first generic "LaunchApplication" key. This is commonly associated with launching "My
    /// Computer", and may have a computer symbol on the key. (`APPCOMMAND_LAUNCH_APP1`)
    LaunchApplication1,
    /// The second generic "LaunchApplication" key. This is commonly associated with launching
    /// "Calculator", and may have a calculator symbol on the key. (`APPCOMMAND_LAUNCH_APP2`,
    /// `KEYCODE_CALCULATOR`)
    LaunchApplication2,
    /// The "Calendar" key. (`KEYCODE_CALENDAR`)
    LaunchCalendar,
    /// The "Contacts" key. (`KEYCODE_CONTACTS`)
    LaunchContacts,
    /// The "Mail" key. (`APPCOMMAND_LAUNCH_MAIL`)
    LaunchMail,
    /// The "Media Player" key. (`APPCOMMAND_LAUNCH_MEDIA_SELECT`)
    LaunchMediaPlayer,
    ///
    LaunchMusicPlayer,
    ///
    LaunchPhone,
    ///
    LaunchScreenSaver,
    ///
    LaunchSpreadsheet,
    ///
    LaunchWebBrowser,
    ///
    LaunchWebCam,
    ///
    LaunchWordProcessor,
    /// Navigate to previous content or page in current history. (`APPCOMMAND_BROWSER_BACKWARD`)
    BrowserBack,
    /// Open the list of browser favorites. (`APPCOMMAND_BROWSER_FAVORITES`)
    BrowserFavorites,
    /// Navigate to next content or page in current history. (`APPCOMMAND_BROWSER_FORWARD`)
    BrowserForward,
    /// Go to the user’s preferred home page. (`APPCOMMAND_BROWSER_HOME`)
    BrowserHome,
    /// Refresh the current page or content. (`APPCOMMAND_BROWSER_REFRESH`)
    BrowserRefresh,
    /// Call up the user’s preferred search page. (`APPCOMMAND_BROWSER_SEARCH`)
    BrowserSearch,
    /// Stop loading the current page or content. (`APPCOMMAND_BROWSER_STOP`)
    BrowserStop,
    /// The Application switch key, which provides a list of recent apps to switch between.
    /// (`KEYCODE_APP_SWITCH`)
    AppSwitch,
    /// The Call key. (`KEYCODE_CALL`)
    Call,
    /// The Camera key. (`KEYCODE_CAMERA`)
    Camera,
    /// The Camera focus key. (`KEYCODE_FOCUS`)
    CameraFocus,
    /// The End Call key. (`KEYCODE_ENDCALL`)
    EndCall,
    /// The Back key. (`KEYCODE_BACK`)
    GoBack,
    /// The Home key, which goes to the phone’s main screen. (`KEYCODE_HOME`)
    GoHome,
    /// The Headset Hook key. (`KEYCODE_HEADSETHOOK`)
    HeadsetHook,
    ///
    LastNumberRedial,
    /// The Notification key. (`KEYCODE_NOTIFICATION`)
    Notification,
    /// Toggle between manner mode state: silent, vibrate, ring, ... (`KEYCODE_MANNER_MODE`)
    MannerMode,
    ///
    VoiceDial,
    /// Switch to viewing TV. (`KEYCODE_TV`)
    TV,
    /// TV 3D Mode. (`KEYCODE_3D_MODE`)
    TV3DMode,
    /// Toggle between antenna and cable input. (`KEYCODE_TV_ANTENNA_CABLE`)
    TVAntennaCable,
    /// Audio description. (`KEYCODE_TV_AUDIO_DESCRIPTION`)
    TVAudioDescription,
    /// Audio description mixing volume down. (`KEYCODE_TV_AUDIO_DESCRIPTION_MIX_DOWN`)
    TVAudioDescriptionMixDown,
    /// Audio description mixing volume up. (`KEYCODE_TV_AUDIO_DESCRIPTION_MIX_UP`)
    TVAudioDescriptionMixUp,
    /// Contents menu. (`KEYCODE_TV_CONTENTS_MENU`)
    TVContentsMenu,
    /// Contents menu. (`KEYCODE_TV_DATA_SERVICE`)
    TVDataService,
    /// Switch the input mode on an external TV. (`KEYCODE_TV_INPUT`)
    TVInput,
    /// Switch to component input #1. (`KEYCODE_TV_INPUT_COMPONENT_1`)
    TVInputComponent1,
    /// Switch to component input #2. (`KEYCODE_TV_INPUT_COMPONENT_2`)
    TVInputComponent2,
    /// Switch to composite input #1. (`KEYCODE_TV_INPUT_COMPOSITE_1`)
    TVInputComposite1,
    /// Switch to composite input #2. (`KEYCODE_TV_INPUT_COMPOSITE_2`)
    TVInputComposite2,
    /// Switch to HDMI input #1. (`KEYCODE_TV_INPUT_HDMI_1`)
    TVInputHDMI1,
    /// Switch to HDMI input #2. (`KEYCODE_TV_INPUT_HDMI_2`)
    TVInputHDMI2,
    /// Switch to HDMI input #3. (`KEYCODE_TV_INPUT_HDMI_3`)
    TVInputHDMI3,
    /// Switch to HDMI input #4. (`KEYCODE_TV_INPUT_HDMI_4`)
    TVInputHDMI4,
    /// Switch to VGA input #1. (`KEYCODE_TV_INPUT_VGA_1`)
    TVInputVGA1,
    /// Media context menu. (`KEYCODE_TV_MEDIA_CONTEXT_MENU`)
    TVMediaContext,
    /// Toggle network. (`KEYCODE_TV_NETWORK`)
    TVNetwork,
    /// Number entry. (`KEYCODE_TV_NUMBER_ENTRY`)
    TVNumberEntry,
    /// Toggle the power on an external TV. (`KEYCODE_TV_POWER`)
    TVPower,
    /// Radio. (`KEYCODE_TV_RADIO_SERVICE`)
    TVRadioService,
    /// Satellite. (`KEYCODE_TV_SATELLITE`)
    TVSatellite,
    /// Broadcast Satellite. (`KEYCODE_TV_SATELLITE_BS`)
    TVSatelliteBS,
    /// Communication Satellite. (`KEYCODE_TV_SATELLITE_CS`)
    TVSatelliteCS,
    /// Toggle between available satellites. (`KEYCODE_TV_SATELLITE_SERVICE`)
    TVSatelliteToggle,
    /// Analog Terrestrial. (`KEYCODE_TV_TERRESTRIAL_ANALOG`)
    TVTerrestrialAnalog,
    /// Digital Terrestrial. (`KEYCODE_TV_TERRESTRIAL_DIGITAL`)
    TVTerrestrialDigital,
    /// Timer programming. (`KEYCODE_TV_TIMER_PROGRAMMING`)
    TVTimer,
    /// Switch the input mode on an external AVR (audio/video receiver). (`KEYCODE_AVR_INPUT`)
    AVRInput,
    /// Toggle the power on an external AVR (audio/video receiver). (`KEYCODE_AVR_POWER`)
    AVRPower,
    /// General purpose color-coded media function key, as index 0 (red). (`VK_COLORED_KEY_0`,
    /// `KEYCODE_PROG_RED`)
    ColorF0Red,
    /// General purpose color-coded media function key, as index 1 (green). (`VK_COLORED_KEY_1`,
    /// `KEYCODE_PROG_GREEN`)
    ColorF1Green,
    /// General purpose color-coded media function key, as index 2 (yellow). (`VK_COLORED_KEY_2`,
    /// `KEYCODE_PROG_YELLOW`)
    ColorF2Yellow,
    /// General purpose color-coded media function key, as index 3 (blue). (`VK_COLORED_KEY_3`,
    /// `KEYCODE_PROG_BLUE`)
    ColorF3Blue,
    /// General purpose color-coded media function key, as index 4 (grey). (`VK_COLORED_KEY_4`)
    ColorF4Grey,
    /// General purpose color-coded media function key, as index 5 (brown). (`VK_COLORED_KEY_5`)
    ColorF5Brown,
    /// Toggle the display of Closed Captions. (`VK_CC`, `KEYCODE_CAPTIONS`)
    ClosedCaptionToggle,
    /// Adjust brightness of device, by toggling between or cycling through states. (`VK_DIMMER`)
    Dimmer,
    /// Swap video sources. (`VK_DISPLAY_SWAP`)
    DisplaySwap,
    /// Select Digital Video Recorder. (`KEYCODE_DVR`)
    DVR,
    /// Exit the current application. (`VK_EXIT`)
    Exit,
    /// Clear program or content stored as favorite 0. (`VK_CLEAR_FAVORITE_0`)
    FavoriteClear0,
    /// Clear program or content stored as favorite 1. (`VK_CLEAR_FAVORITE_1`)
    FavoriteClear1,
    /// Clear program or content stored as favorite 2. (`VK_CLEAR_FAVORITE_2`)
    FavoriteClear2,
    /// Clear program or content stored as favorite 3. (`VK_CLEAR_FAVORITE_3`)
    FavoriteClear3,
    /// Select (recall) program or content stored as favorite 0. (`VK_RECALL_FAVORITE_0`)
    FavoriteRecall0,
    /// Select (recall) program or content stored as favorite 1. (`VK_RECALL_FAVORITE_1`)
    FavoriteRecall1,
    /// Select (recall) program or content stored as favorite 2. (`VK_RECALL_FAVORITE_2`)
    FavoriteRecall2,
    /// Select (recall) program or content stored as favorite 3. (`VK_RECALL_FAVORITE_3`)
    FavoriteRecall3,
    /// Store current program or content as favorite 0. (`VK_STORE_FAVORITE_0`)
    FavoriteStore0,
    /// Store current program or content as favorite 1. (`VK_STORE_FAVORITE_1`)
    FavoriteStore1,
    /// Store current program or content as favorite 2. (`VK_STORE_FAVORITE_2`)
    FavoriteStore2,
    /// Store current program or content as favorite 3. (`VK_STORE_FAVORITE_3`)
    FavoriteStore3,
    /// Toggle display of program or content guide. (`VK_GUIDE`, `KEYCODE_GUIDE`)
    Guide,
    /// If guide is active and displayed, then display next day’s content. (`VK_NEXT_DAY`)
    GuideNextDay,
    /// If guide is active and displayed, then display previous day’s content. (`VK_PREV_DAY`)
    GuidePreviousDay,
    /// Toggle display of information about currently selected context or media. (`VK_INFO`,
    /// `KEYCODE_INFO`)
    Info,
    /// Toggle instant replay. (`VK_INSTANT_REPLAY`)
    InstantReplay,
    /// Launch linked content, if available and appropriate. (`VK_LINK`)
    Link,
    /// List the current program. (`VK_LIST`)
    ListProgram,
    /// Toggle display listing of currently available live content or programs. (`VK_LIVE`)
    LiveContent,
    /// Lock or unlock current content or program. (`VK_LOCK`)
    Lock,
    /// Show a list of media applications: audio/video players and image viewers. (`VK_APPS`)
    ///
    /// Note: Do not confuse this key value with the Windows' `VK_APPS` / `VK_CONTEXT_MENU` key,
    /// which is encoded as `"ContextMenu"`.
    MediaApps,
    /// Audio track key. (`KEYCODE_MEDIA_AUDIO_TRACK`)
    MediaAudioTrack,
    /// Select previously selected channel or media. (`VK_LAST`, `KEYCODE_LAST_CHANNEL`)
    MediaLast,
    /// Skip backward to next content or program. (`KEYCODE_MEDIA_SKIP_BACKWARD`)
    MediaSkipBackward,
    /// Skip forward to next content or program. (`VK_SKIP`, `KEYCODE_MEDIA_SKIP_FORWARD`)
    MediaSkipForward,
    /// Step backward to next content or program. (`KEYCODE_MEDIA_STEP_BACKWARD`)
    MediaStepBackward,
    /// Step forward to next content or program. (`KEYCODE_MEDIA_STEP_FORWARD`)
    MediaStepForward,
    /// Media top menu. (`KEYCODE_MEDIA_TOP_MENU`)
    MediaTopMenu,
    /// Navigate in. (`KEYCODE_NAVIGATE_IN`)
    NavigateIn,
    /// Navigate to next key. (`KEYCODE_NAVIGATE_NEXT`)
    NavigateNext,
    /// Navigate out. (`KEYCODE_NAVIGATE_OUT`)
    NavigateOut,
    /// Navigate to previous key. (`KEYCODE_NAVIGATE_PREVIOUS`)
    NavigatePrevious,
    /// Cycle to next favorite channel (in favorites list). (`VK_NEXT_FAVORITE_CHANNEL`)
    NextFavoriteChannel,
    /// Cycle to next user profile (if there are multiple user profiles). (`VK_USER`)
    NextUserProfile,
    /// Access on-demand content or programs. (`VK_ON_DEMAND`)
    OnDemand,
    /// Pairing key to pair devices. (`KEYCODE_PAIRING`)
    Pairing,
    /// Move picture-in-picture window down. (`VK_PINP_DOWN`)
    PinPDown,
    /// Move picture-in-picture window. (`VK_PINP_MOVE`)
    PinPMove,
    /// Toggle display of picture-in-picture window. (`VK_PINP_TOGGLE`)
    PinPToggle,
    /// Move picture-in-picture window up. (`VK_PINP_UP`)
    PinPUp,
    /// Decrease media playback speed. (`VK_PLAY_SPEED_DOWN`)
    PlaySpeedDown,
    /// Reset playback to normal speed. (`VK_PLAY_SPEED_RESET`)
    PlaySpeedReset,
    /// Increase media playback speed. (`VK_PLAY_SPEED_UP`)
    PlaySpeedUp,
    /// Toggle random media or content shuffle mode. (`VK_RANDOM_TOGGLE`)
    RandomToggle,
    /// Not a physical key, but this key code is sent when the remote control battery is low.
    /// (`VK_RC_LOW_BATTERY`)
    RcLowBattery,
    /// Toggle or cycle between media recording speeds. (`VK_RECORD_SPEED_NEXT`)
    RecordSpeedNext,
    /// Toggle RF (radio frequency) input bypass mode (pass RF input directly to the RF output).
    /// (`VK_RF_BYPASS`)
    RfBypass,
    /// Toggle scan channels mode. (`VK_SCAN_CHANNELS_TOGGLE`)
    ScanChannelsToggle,
    /// Advance display screen mode to next available mode. (`VK_SCREEN_MODE_NEXT`)
    ScreenModeNext,
    /// Toggle display of device settings screen. (`VK_SETTINGS`, `KEYCODE_SETTINGS`)
    Settings,
    /// Toggle split screen mode. (`VK_SPLIT_SCREEN_TOGGLE`)
    SplitScreenToggle,
    /// Switch the input mode on an external STB (set top box). (`KEYCODE_STB_INPUT`)
    STBInput,
    /// Toggle the power on an external STB (set top box). (`KEYCODE_STB_POWER`)
    STBPower,
    /// Toggle display of subtitles, if available. (`VK_SUBTITLE`)
    Subtitle,
    /// Toggle display of teletext, if available (`VK_TELETEXT`, `KEYCODE_TV_TELETEXT`).
    Teletext,
    /// Advance video mode to next available mode. (`VK_VIDEO_MODE_NEXT`)
    VideoModeNext,
    /// Cause device to identify itself in some manner, e.g., audibly or visibly. (`VK_WINK`)
    Wink,
    /// Toggle between full-screen and scaled content, or alter magnification level. (`VK_ZOOM`,
    /// `KEYCODE_TV_ZOOM_MODE`)
    ZoomToggle,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F1,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F2,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F3,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F4,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F5,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F6,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F7,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F8,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F9,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F10,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F11,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F12,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F13,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F14,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F15,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F16,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F17,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F18,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F19,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F20,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F21,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F22,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F23,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F24,
    /// General-purpose function key.
    F25,
    /// General-purpose function key.
    F26,
    /// General-purpose function key.
    F27,
    /// General-purpose function key.
    F28,
    /// General-purpose function key.
    F29,
    /// General-purpose function key.
    F30,
    /// General-purpose function key.
    F31,
    /// General-purpose function key.
    F32,
    /// General-purpose function key.
    F33,
    /// General-purpose function key.
    F34,
    /// General-purpose function key.
    F35,
}
impl Key {
    /// If the key is a modifier as defined by [w3].
    ///
    /// [w3]: https://www.w3.org/TR/uievents-key/#keys-modifier
    pub fn is_modifier(&self) -> bool {
        matches!(
            self,
            Key::Ctrl
                | Key::Alt
                | Key::AltGraph
                | Key::CapsLock
                | Key::Fn
                | Key::FnLock
                | Key::Meta
                | Key::NumLock
                | Key::ScrollLock
                | Key::Shift
                | Key::Symbol
                | Key::SymbolLock
                | Key::Super
                | Key::Hyper
        )
    }

    /// If the key is a white space as defined by [w3].
    ///
    /// [w3]: https://www.w3.org/TR/uievents-key/#keys-whitespace
    pub fn is_white_space(&self) -> bool {
        matches!(self, Key::Tab | Key::Space)
    }

    /// If the key is for an edit action as defined by [w3].
    ///
    /// [w3]: https://www.w3.org/TR/uievents-key/#keys-editing
    pub fn is_editing(&self) -> bool {
        matches!(
            self,
            Key::Backspace
                | Key::Clear
                | Key::CrSel
                | Key::Cut
                | Key::Delete
                | Key::EraseEof
                | Key::ExSel
                | Key::Insert
                | Key::Paste
                | Key::Redo
                | Key::Undo
        )
    }

    /// If the key is for an general UI action as defined by [w3].
    ///
    /// [w3]: https://www.w3.org/TR/uievents-key/#keys-ui
    pub fn is_ui(&self) -> bool {
        matches!(
            self,
            Key::Accept
                | Key::Again
                | Key::Attn
                | Key::Cancel
                | Key::ContextMenu
                | Key::Escape
                | Key::Execute
                | Key::Find
                | Key::Help
                | Key::Pause
                | Key::Play
                | Key::Props
                | Key::Select
                | Key::ZoomIn
                | Key::ZoomOut
        )
    }

    /// If the key is for an general device action as defined by [w3].
    ///
    /// [w3]: https://www.w3.org/TR/uievents-key/#keys-device
    pub fn is_device(&self) -> bool {
        matches!(
            self,
            Key::BrightnessDown
                | Key::BrightnessUp
                | Key::Eject
                | Key::LogOff
                | Key::Power
                | Key::PowerOff
                | Key::PrintScreen
                | Key::Hibernate
                | Key::Standby
                | Key::WakeUp
        )
    }

    /// If the key if for IME composition actions as defined by [w3].
    ///
    /// [w3]: https://www.w3.org/TR/uievents-key/#keys-composition
    pub fn is_composition(&self) -> bool {
        matches!(
            self,
            Key::AllCandidates
                | Key::Alphanumeric
                | Key::CodeInput
                | Key::Compose
                | Key::Convert
                | Key::Dead(_)
                | Key::FinalMode
                | Key::GroupFirst
                | Key::GroupLast
                | Key::GroupNext
                | Key::GroupPrevious
                | Key::ModeChange
                | Key::NextCandidate
                | Key::NonConvert
                | Key::PreviousCandidate
                | Key::Process
                | Key::SingleCandidate
                | Key::HangulMode
                | Key::HanjaMode
                | Key::JunjaMode
                | Key::Eisu
                | Key::Hankaku
                | Key::Hiragana
                | Key::HiraganaKatakana
                | Key::KanaMode
                | Key::KanjiMode
                | Key::Katakana
                | Key::Romaji
                | Key::Zenkaku
                | Key::ZenkakuHankaku
        )
    }

    /// If the key is one of the general purpose function keys.
    pub fn is_function(&self) -> bool {
        matches!(
            self,
            Key::F1
                | Key::F2
                | Key::F3
                | Key::F4
                | Key::F5
                | Key::F6
                | Key::F7
                | Key::F8
                | Key::F9
                | Key::F10
                | Key::F11
                | Key::F12
                | Key::F13
                | Key::F14
                | Key::F15
                | Key::F16
                | Key::F17
                | Key::F18
                | Key::F19
                | Key::F20
                | Key::F21
                | Key::F22
                | Key::F23
                | Key::F24
                | Key::F25
                | Key::F26
                | Key::F27
                | Key::F28
                | Key::F29
                | Key::F30
                | Key::F31
                | Key::F32
                | Key::F33
                | Key::F34
                | Key::F35
                | Key::Soft1
                | Key::Soft2
                | Key::Soft3
                | Key::Soft4
        )
    }

    /// If the key is for an multimedia control as defined by [w3].
    ///
    /// [w3]: https://www.w3.org/TR/uievents-key/#keys-multimedia
    pub fn is_multimedia(&self) -> bool {
        matches!(
            self,
            Key::ChannelDown
                | Key::ChannelUp
                | Key::Close
                | Key::MailForward
                | Key::MailReply
                | Key::MailSend
                | Key::MediaClose
                | Key::MediaFastForward
                | Key::MediaPause
                | Key::MediaPlay
                | Key::MediaPlayPause
                | Key::MediaRecord
                | Key::MediaRewind
                | Key::MediaStop
                | Key::MediaTrackNext
                | Key::MediaTrackPrevious
                | Key::New
                | Key::Open
                | Key::Print
                | Key::Save
                | Key::SpellCheck
        )
    }

    /// If the key is for an audio control as defined by [w3].
    ///
    /// [w3]: https://www.w3.org/TR/uievents-key/#keys-audio
    pub fn is_audio(&self) -> bool {
        matches!(
            self,
            Key::AudioBalanceLeft
                | Key::AudioBalanceRight
                | Key::AudioBassBoostDown
                | Key::AudioBassBoostToggle
                | Key::AudioBassBoostUp
                | Key::AudioFaderFront
                | Key::AudioFaderRear
                | Key::AudioSurroundModeNext
                | Key::AudioTrebleDown
                | Key::AudioTrebleUp
                | Key::AudioVolumeDown
                | Key::AudioVolumeUp
                | Key::AudioVolumeMute
                | Key::MicrophoneToggle
                | Key::MicrophoneVolumeDown
                | Key::MicrophoneVolumeUp
                | Key::MicrophoneVolumeMute
        )
    }

    /// If the key is for a speech correction control as defined by [w3].
    ///
    /// [w3]:https://www.w3.org/TR/uievents-key/#keys-speech
    pub fn is_speech(&self) -> bool {
        matches!(self, Key::SpeechCorrectionList | Key::SpeechInputToggle)
    }

    /// If the key is for launching an application.
    pub fn is_launch(&self) -> bool {
        matches!(
            self,
            Key::LaunchApplication1
                | Key::LaunchApplication2
                | Key::LaunchCalendar
                | Key::LaunchContacts
                | Key::LaunchMail
                | Key::LaunchMediaPlayer
                | Key::LaunchMusicPlayer
                | Key::LaunchPhone
                | Key::LaunchScreenSaver
                | Key::LaunchSpreadsheet
                | Key::LaunchWebBrowser
                | Key::LaunchWebCam
                | Key::LaunchWordProcessor
        )
    }

    /// If the key is for a browser control.
    pub fn is_browser(&self) -> bool {
        matches!(
            self,
            Key::BrowserBack
                | Key::BrowserFavorites
                | Key::BrowserForward
                | Key::BrowserHome
                | Key::BrowserRefresh
                | Key::BrowserSearch
                | Key::BrowserStop
        )
    }

    /// If the key is from a mobile phone as defined by [w3].
    ///
    /// [w3]: https://www.w3.org/TR/uievents-key/#keys-mobile
    pub fn is_mobile(&self) -> bool {
        matches!(
            self,
            Key::AppSwitch
                | Key::Call
                | Key::Camera
                | Key::CameraFocus
                | Key::EndCall
                | Key::GoBack
                | Key::GoHome
                | Key::HeadsetHook
                | Key::LastNumberRedial
                | Key::Notification
                | Key::MannerMode
                | Key::VoiceDial
        )
    }

    /// If the key is from a TV control as defined by [w3].
    ///
    /// [w3]: https://www.w3.org/TR/uievents-key/#keys-tv
    pub fn is_tv(&self) -> bool {
        matches!(
            self,
            Key::TV
                | Key::TV3DMode
                | Key::TVAntennaCable
                | Key::TVAudioDescription
                | Key::TVAudioDescriptionMixDown
                | Key::TVAudioDescriptionMixUp
                | Key::TVContentsMenu
                | Key::TVDataService
                | Key::TVInput
                | Key::TVInputComponent1
                | Key::TVInputComponent2
                | Key::TVInputComposite1
                | Key::TVInputComposite2
                | Key::TVInputHDMI1
                | Key::TVInputHDMI2
                | Key::TVInputHDMI3
                | Key::TVInputHDMI4
                | Key::TVInputVGA1
                | Key::TVMediaContext
                | Key::TVNetwork
                | Key::TVNumberEntry
                | Key::TVPower
                | Key::TVRadioService
                | Key::TVSatellite
                | Key::TVSatelliteBS
                | Key::TVSatelliteCS
                | Key::TVSatelliteToggle
                | Key::TVTerrestrialAnalog
                | Key::TVTerrestrialDigital
                | Key::TVTimer
        )
    }

    /// If the key is for a media controller as defined by [w3].
    ///
    /// [w3]: https://www.w3.org/TR/uievents-key/#keys-media-controller
    pub fn is_media_controller(&self) -> bool {
        matches!(
            self,
            Key::AVRInput
                | Key::AVRPower
                | Key::ColorF0Red
                | Key::ColorF1Green
                | Key::ColorF2Yellow
                | Key::ColorF3Blue
                | Key::ColorF4Grey
                | Key::ColorF5Brown
                | Key::ClosedCaptionToggle
                | Key::Dimmer
                | Key::DisplaySwap
                | Key::DVR
                | Key::Exit
                | Key::FavoriteClear0
                | Key::FavoriteClear1
                | Key::FavoriteClear2
                | Key::FavoriteClear3
                | Key::FavoriteRecall0
                | Key::FavoriteRecall1
                | Key::FavoriteRecall2
                | Key::FavoriteRecall3
                | Key::FavoriteStore0
                | Key::FavoriteStore1
                | Key::FavoriteStore2
                | Key::FavoriteStore3
                | Key::Guide
                | Key::GuideNextDay
                | Key::GuidePreviousDay
                | Key::Info
                | Key::InstantReplay
                | Key::Link
                | Key::ListProgram
                | Key::LiveContent
                | Key::Lock
                | Key::MediaApps
                | Key::MediaAudioTrack
                | Key::MediaLast
                | Key::MediaSkipBackward
                | Key::MediaSkipForward
                | Key::MediaStepBackward
                | Key::MediaStepForward
                | Key::MediaTopMenu
                | Key::NavigateIn
                | Key::NavigateNext
                | Key::NavigateOut
                | Key::NavigatePrevious
                | Key::NextFavoriteChannel
                | Key::NextUserProfile
                | Key::OnDemand
                | Key::Pairing
                | Key::PinPDown
                | Key::PinPMove
                | Key::PinPToggle
                | Key::PinPUp
                | Key::PlaySpeedDown
                | Key::PlaySpeedReset
                | Key::PlaySpeedUp
                | Key::RandomToggle
                | Key::RcLowBattery
                | Key::RecordSpeedNext
                | Key::RfBypass
                | Key::ScanChannelsToggle
                | Key::ScreenModeNext
                | Key::Settings
                | Key::SplitScreenToggle
                | Key::STBInput
                | Key::STBPower
                | Key::Subtitle
                | Key::Teletext
                | Key::VideoModeNext
                | Key::Wink
                | Key::ZoomToggle
        )
    }

    /// Gets the variant name.
    pub fn name(&self) -> &'static str {
        serde_variant::to_variant_name(self).unwrap_or("")
    }

    /// Gets the named key, or `Char` or `Str`.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        let mut n = s.chars();
        if let Some(c) = n.next() {
            if n.next().is_none() {
                return Self::Char(c);
            }
        }

        for v in Self::all_named() {
            if v.name() == s {
                return v;
            }
        }

        Self::Str(s.to_owned().into())
    }

    /// Iterate over all values from `Alt` to `F35`.
    pub fn all_named() -> impl ExactSizeIterator<Item = Key> + DoubleEndedIterator {
        unsafe {
            // SAFETY: this is safe because all variants from `Alt` are without associated data.
            let s: (u16, [u8; 22]) = mem::transmute(Key::Alt);
            let e: (u16, [u8; 22]) = mem::transmute(Key::F35);
            (s.0..=e.0).map(|n| mem::transmute((n, [0u8; 22])))
        }
    }
}
impl std::str::FromStr for Key {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_str(s))
    }
}
impl fmt::Debug for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "Key::")?;
        }
        let name = self.name();
        match self {
            Self::Char(c) => write!(f, "{name}({c:?})"),
            Self::Str(s) => write!(f, "{name}({:?})", s.as_ref()),
            Self::Dead(c) => write!(f, "{name}({c:?})"),
            _ => write!(f, "{name}"),
        }
    }
}

/// Describes the appearance of the mouse cursor.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum CursorIcon {
    /// The platform-dependent default cursor.
    #[default]
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

impl CursorIcon {
    /// All cursor icons.
    pub const ALL: &'static [CursorIcon] = &[
        CursorIcon::Default,
        CursorIcon::Crosshair,
        CursorIcon::Hand,
        CursorIcon::Arrow,
        CursorIcon::Move,
        CursorIcon::Text,
        CursorIcon::Wait,
        CursorIcon::Help,
        CursorIcon::Progress,
        CursorIcon::NotAllowed,
        CursorIcon::ContextMenu,
        CursorIcon::Cell,
        CursorIcon::VerticalText,
        CursorIcon::Alias,
        CursorIcon::Copy,
        CursorIcon::NoDrop,
        CursorIcon::Grab,
        CursorIcon::Grabbing,
        CursorIcon::AllScroll,
        CursorIcon::ZoomIn,
        CursorIcon::ZoomOut,
        CursorIcon::EResize,
        CursorIcon::NResize,
        CursorIcon::NeResize,
        CursorIcon::NwResize,
        CursorIcon::SResize,
        CursorIcon::SeResize,
        CursorIcon::SwResize,
        CursorIcon::WResize,
        CursorIcon::EwResize,
        CursorIcon::NsResize,
        CursorIcon::NeswResize,
        CursorIcon::NwseResize,
        CursorIcon::ColResize,
        CursorIcon::RowResize,
    ];

    /// Estimated icon size and click spot in that size.
    pub fn size_and_spot(&self) -> (DipSize, DipPoint) {
        fn splat(s: f32, rel_pt: f32) -> (DipSize, DipPoint) {
            size(s, s, rel_pt, rel_pt)
        }
        fn size(w: f32, h: f32, rel_x: f32, rel_y: f32) -> (DipSize, DipPoint) {
            (
                DipSize::new(Dip::new_f32(w), Dip::new_f32(h)),
                DipPoint::new(Dip::new_f32(w * rel_x), Dip::new_f32(h * rel_y)),
            )
        }

        match self {
            CursorIcon::Crosshair
            | CursorIcon::Move
            | CursorIcon::Wait
            | CursorIcon::NotAllowed
            | CursorIcon::NoDrop
            | CursorIcon::Cell
            | CursorIcon::Grab
            | CursorIcon::Grabbing
            | CursorIcon::AllScroll => splat(20.0, 0.5),
            CursorIcon::Text | CursorIcon::NResize | CursorIcon::SResize | CursorIcon::NsResize => size(8.0, 20.0, 0.5, 0.5),
            CursorIcon::VerticalText | CursorIcon::EResize | CursorIcon::WResize | CursorIcon::EwResize => size(20.0, 8.0, 0.5, 0.5),
            _ => splat(20.0, 0.0),
        }
    }
}

/// Window state after a resize.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Default)]
pub enum WindowState {
    /// Window is visible but does not fill the screen.
    #[default]
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
impl WindowState {
    /// Returns `true` if `self` matches [`Fullscreen`] or [`Exclusive`].
    ///
    /// [`Fullscreen`]: WindowState::Fullscreen
    /// [`Exclusive`]: WindowState::Exclusive
    pub fn is_fullscreen(self) -> bool {
        matches!(self, Self::Fullscreen | Self::Exclusive)
    }
}

/// [`Event::FrameRendered`] payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventFrameRendered {
    /// Window that was rendered.
    pub window: WindowId,
    /// Frame that was rendered.
    pub frame: FrameId,
    /// Frame image, if one was requested with the frame request.
    pub frame_image: Option<ImageLoadedData>,
}

/// [`Event::WindowChanged`] payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowChanged {
    // note that this payload is handled by `Event::coalesce`, add new fields there too.
    //
    /// Window that has changed state.
    pub window: WindowId,

    /// Window new state, is `None` if the window state did not change.
    pub state: Option<WindowStateAll>,

    /// Window new global position, is `None` if the window position did not change.
    ///
    /// The values are the global position and the position in the monitor.
    pub position: Option<(PxPoint, DipPoint)>,

    /// Window new monitor.
    ///
    /// The window's monitor change when it is moved enough so that most of the
    /// client area is in the new monitor screen.
    pub monitor: Option<MonitorId>,

    /// The window new size, is `None` if the window size did not change.
    pub size: Option<DipSize>,

    /// If the view-process is blocking the event loop for a time waiting for a frame for the new `size` this
    /// ID must be send with the frame to signal that it is the frame for the new size.
    ///
    /// Event loop implementations can use this to resize without visible artifacts
    /// like the clear color flashing on the window corners, there is a timeout to this delay but it
    /// can be a noticeable stutter, a [`render`] or [`render_update`] request for the window unblocks the loop early
    /// to continue the resize operation.
    ///
    /// [`render`]: crate::Api::render
    /// [`render_update`]: crate::Api::render_update
    pub frame_wait_id: Option<FrameWaitId>,

    /// What caused the change, end-user/OS modifying the window or the app.
    pub cause: EventCause,
}
impl WindowChanged {
    /// Create an event that represents window move.
    pub fn moved(window: WindowId, global_position: PxPoint, position: DipPoint, cause: EventCause) -> Self {
        WindowChanged {
            window,
            state: None,
            position: Some((global_position, position)),
            monitor: None,
            size: None,
            frame_wait_id: None,
            cause,
        }
    }

    /// Create an event that represents window parent monitor change.
    pub fn monitor_changed(window: WindowId, monitor: MonitorId, cause: EventCause) -> Self {
        WindowChanged {
            window,
            state: None,
            position: None,
            monitor: Some(monitor),
            size: None,
            frame_wait_id: None,
            cause,
        }
    }

    /// Create an event that represents window resized.
    pub fn resized(window: WindowId, size: DipSize, cause: EventCause, frame_wait_id: Option<FrameWaitId>) -> Self {
        WindowChanged {
            window,
            state: None,
            position: None,
            monitor: None,
            size: Some(size),
            frame_wait_id,
            cause,
        }
    }

    /// Create an event that represents [`WindowStateAll`] change.
    pub fn state_changed(window: WindowId, state: WindowStateAll, cause: EventCause) -> Self {
        WindowChanged {
            window,
            state: Some(state),
            position: None,
            monitor: None,
            size: None,
            frame_wait_id: None,
            cause,
        }
    }
}

/// System/User events sent from the View Process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    /// View process is online.
    ///
    /// The [`ViewProcessGen`] is the generation of the new view-process, it must be passed to
    /// [`Controller::handle_inited`].
    ///
    /// [`Controller::handle_inited`]: crate::Controller::handle_inited
    Inited {
        /// View-process generation, changes after respawns and is never zero.
        generation: ViewProcessGen,
        /// If the view-process is a respawn from a previous crashed process.
        is_respawn: bool,

        /// Available monitors.
        available_monitors: Vec<(MonitorId, MonitorInfo)>,
        /// System multi-click config.
        multi_click_config: MultiClickConfig,
        /// System keyboard pressed key repeat start delay config.
        key_repeat_config: KeyRepeatConfig,
        /// System touch config.
        touch_config: TouchConfig,
        /// System font anti-aliasing config.
        font_aa: FontAntiAliasing,
        /// System animations config.
        animations_config: AnimationsConfig,
        /// System locale config.
        locale_config: LocaleConfig,
        /// System preferred color scheme.
        color_scheme: ColorScheme,
        /// API extensions implemented by the view-process.
        ///
        /// The extension IDs will stay valid for the duration of the view-process.
        extensions: ApiExtensions,
    },

    /// The event channel disconnected, probably because the view-process crashed.
    ///
    /// The [`ViewProcessGen`] is the generation of the view-process that was lost, it must be passed to
    /// [`Controller::handle_disconnect`].
    ///
    /// [`Controller::handle_disconnect`]: crate::Controller::handle_disconnect
    Disconnected(ViewProcessGen),

    /// Window, context and renderer have finished initializing and is ready to receive commands.
    WindowOpened(WindowId, WindowOpenData),

    /// Headless context and renderer have finished initializing and is ready to receive commands.
    HeadlessOpened(WindowId, HeadlessOpenData),

    /// Window open or headless context open request failed.
    WindowOrHeadlessOpenError {
        /// Id from the request.
        id: WindowId,
        /// Error message.
        error: String,
    },

    /// A frame finished rendering.
    ///
    /// `EventsCleared` is not send after this event.
    FrameRendered(EventFrameRendered),

    /// Window moved, resized, or minimized/maximized etc.
    ///
    /// This event coalesces events usually named `WindowMoved`, `WindowResized` and `WindowStateChanged` into a
    /// single event to simplify tracking composite changes, for example, the window changes size and position
    /// when maximized, this can be trivially observed with this event.
    ///
    /// The [`EventCause`] can be used to identify a state change initiated by the app.
    WindowChanged(WindowChanged),

    /// A file has been dropped into the window.
    ///
    /// When the user drops multiple files at once, this event will be emitted for each file separately.
    DroppedFile {
        /// Window that received the file drop.
        window: WindowId,
        /// Path to the file that was dropped.
        file: PathBuf,
    },
    /// A file is being hovered over the window.
    ///
    /// When the user hovers multiple files at once, this event will be emitted for each file separately.
    HoveredFile {
        /// Window that was hovered by drag-drop.
        window: WindowId,
        /// Path to the file being dragged.
        file: PathBuf,
    },
    /// A file was hovered, but has exited the window.
    ///
    /// There will be a single event triggered even if multiple files were hovered.
    HoveredFileCancelled(WindowId),

    /// App window(s) focus changed.
    FocusChanged {
        /// Window that lost focus.
        prev: Option<WindowId>,
        /// Window that got focus.
        new: Option<WindowId>,
    },
    /// An event from the keyboard has been received.
    ///
    /// This event is only send if the window is focused, all pressed keys should be considered released
    /// after [`FocusChanged`] to `None`. Modifier keys receive special treatment, after they are pressed,
    /// the modifier key state is monitored directly so that the `Released` event is always send, unless the
    /// focus changed to none.
    ///
    /// [`FocusChanged`]: Self::FocusChanged
    KeyboardInput {
        /// Window that received the key event.
        window: WindowId,
        /// Device that generated the key event.
        device: DeviceId,
        /// Physical key.
        key_code: KeyCode,
        /// If the key was pressed or released.
        state: KeyState,

        /// Semantic key.
        ///
        /// Pressing `Shift+A` key will produce `Key::Char('a')` in QWERT keyboards, the modifiers are not applied.
        key: Option<Key>,
        /// Semantic key modified by the current active modifiers.
        ///
        /// Pressing `Shift+A` key will produce `Key::Char('A')` in QWERT keyboards, the modifiers are applied.
        key_modified: Option<Key>,
        /// Text typed.
        ///
        /// This is only set during [`KeyState::Pressed`] of a key that generates text.
        ///
        /// This is usually the `key_modified` char, but is also `'\r'` for `Key::Enter`. On Windows when a dead key was
        /// pressed earlier but cannot be combined with the character from this key press, the produced text
        /// will consist of two characters: the dead-key-character followed by the character resulting from this key press.
        text: String,
    },
    /// The cursor has moved on the window.
    ///
    /// This event can be coalesced, i.e. multiple cursor moves packed into the same event.
    CursorMoved {
        /// Window that received the cursor move.
        window: WindowId,
        /// Device that generated the cursor move.
        device: DeviceId,

        /// Cursor positions in between the previous event and this one.
        coalesced_pos: Vec<DipPoint>,

        /// Cursor position, relative to the window top-left in device independent pixels.
        position: DipPoint,
    },

    /// The cursor has entered the window.
    CursorEntered {
        /// Window that now is hovered by the cursor.
        window: WindowId,
        /// Device that generated the cursor move event.
        device: DeviceId,
    },
    /// The cursor has left the window.
    CursorLeft {
        /// Window that is no longer hovered by the cursor.
        window: WindowId,
        /// Device that generated the cursor move event.
        device: DeviceId,
    },
    /// A mouse wheel movement or touchpad scroll occurred.
    MouseWheel {
        /// Window that was hovered by the cursor when the mouse wheel was used.
        window: WindowId,
        /// Device that generated the mouse wheel event.
        device: DeviceId,
        /// Delta of change in the mouse scroll wheel state.
        delta: MouseScrollDelta,
        /// Touch state if the device that generated the event is a touchpad.
        phase: TouchPhase,
    },
    /// An mouse button press has been received.
    MouseInput {
        /// Window that was hovered by the cursor when the mouse button was used.
        window: WindowId,
        /// Mouse device that generated the event.
        device: DeviceId,
        /// If the button was pressed or released.
        state: ButtonState,
        /// The mouse button.
        button: MouseButton,
    },
    /// Touchpad pressure event.
    TouchpadPressure {
        /// Window that was hovered when the touchpad was touched.
        window: WindowId,
        /// Touchpad device.
        device: DeviceId,
        /// Pressure level between 0 and 1.
        pressure: f32,
        /// Click level.
        stage: i64,
    },
    /// Motion on some analog axis. May report data redundant to other, more specific events.
    AxisMotion {
        /// Window that was focused when the motion was realized.
        window: WindowId,
        /// Analog device.
        device: DeviceId,
        /// Axis.
        axis: AxisId,
        /// Motion value.
        value: f64,
    },
    /// Touch event has been received.
    Touch {
        /// Window that was touched.
        window: WindowId,
        /// Touch device.
        device: DeviceId,

        /// Coalesced touch updates, never empty.
        touches: Vec<TouchUpdate>,
    },
    /// The monitor’s scale factor has changed.
    ScaleFactorChanged {
        /// Monitor that has changed.
        monitor: MonitorId,
        /// Windows affected by this change.
        ///
        /// Note that a window's scale factor can also change if it is moved to another monitor,
        /// the [`Event::WindowChanged`] event notifies this using the [`WindowChanged::monitor`].
        windows: Vec<WindowId>,
        /// The new scale factor.
        scale_factor: f32,
    },

    /// The available monitors have changed.
    MonitorsChanged(Vec<(MonitorId, MonitorInfo)>),

    /// The preferred color scheme for a window has changed.
    ColorSchemeChanged(WindowId, ColorScheme),
    /// The window has been requested to close.
    WindowCloseRequested(WindowId),
    /// The window has closed.
    WindowClosed(WindowId),

    /// An image resource already decoded size and PPI.
    ImageMetadataLoaded {
        /// The image that started loading.
        image: ImageId,
        /// The image pixel size.
        size: PxSize,
        /// The image pixels-per-inch metadata.
        ppi: Option<ImagePpi>,
        /// The image is a single channel R8.
        is_mask: bool,
    },
    /// An image resource finished decoding.
    ImageLoaded(ImageLoadedData),
    /// An image resource, progressively decoded has decoded more bytes.
    ImagePartiallyLoaded {
        /// The image that has decoded more pixels.
        image: ImageId,
        /// The size of the decoded pixels, can be different then the image size if the
        /// image is not *interlaced*.
        partial_size: PxSize,
        /// The image pixels-per-inch metadata.
        ppi: Option<ImagePpi>,
        /// If the decoded pixels so-far are all opaque (255 alpha).
        is_opaque: bool,
        /// If the decoded pixels so-far are a single channel.
        is_mask: bool,
        /// Updated BGRA8 pre-multiplied pixel buffer or R8 if `is_mask`. This includes all the pixels
        /// decoded so-far.
        partial_pixels: IpcBytes,
    },
    /// An image resource failed to decode, the image ID is not valid.
    ImageLoadError {
        /// The image that failed to decode.
        image: ImageId,
        /// The error message.
        error: String,
    },
    /// An image finished encoding.
    ImageEncoded {
        /// The image that finished encoding.
        image: ImageId,
        /// The format of the encoded data.
        format: String,
        /// The encoded image data.
        data: IpcBytes,
    },
    /// An image failed to encode.
    ImageEncodeError {
        /// The image that failed to encode.
        image: ImageId,
        /// The encoded format that was requested.
        format: String,
        /// The error message.
        error: String,
    },

    /// An image generated from a rendered frame is ready.
    FrameImageReady {
        /// Window that had pixels copied.
        window: WindowId,
        /// The frame that was rendered when the pixels where copied.
        frame: FrameId,
        /// The frame image.
        image: ImageId,
        /// The pixel selection relative to the top-left.
        selection: PxRect,
    },

    // Config events
    /// System fonts have changed.
    FontsChanged,
    /// System text-antialiasing configuration has changed.
    FontAaChanged(FontAntiAliasing),
    /// System double-click definition changed.
    MultiClickConfigChanged(MultiClickConfig),
    /// System animations config changed.
    AnimationsConfigChanged(AnimationsConfig),
    /// System definition of pressed key repeat event changed.
    KeyRepeatConfigChanged(KeyRepeatConfig),
    /// System touch config changed.
    TouchConfigChanged(TouchConfig),
    /// System locale changed.
    LocaleChanged(LocaleConfig),

    // Raw device events
    /// Device added or installed.
    DeviceAdded(DeviceId),
    /// Device removed.
    DeviceRemoved(DeviceId),
    /// Mouse pointer motion.
    ///
    /// The values if the delta of movement (x, y), not position.
    DeviceMouseMotion {
        /// Device that generated the event.
        device: DeviceId,
        /// Delta of change in the cursor position.
        delta: euclid::Vector2D<f64, ()>,
    },
    /// Mouse scroll wheel turn.
    DeviceMouseWheel {
        /// Mouse device that generated the event.
        device: DeviceId,
        /// Delta of change in the mouse scroll wheel state.
        delta: MouseScrollDelta,
    },
    /// Motion on some analog axis.
    ///
    /// This includes the mouse device and any other that fits.
    DeviceMotion {
        /// Device that generated the event.
        device: DeviceId,
        /// Device dependent axis of the motion.
        axis: AxisId,
        /// Device dependent value.
        value: f64,
    },
    /// Device button press or release.
    DeviceButton {
        /// Device that generated the event.
        device: DeviceId,
        /// Device dependent button that was used.
        button: ButtonId,
        /// If the button was pressed or released.
        state: ButtonState,
    },
    /// Device key press or release.
    DeviceKey {
        /// Device that generated the key event.
        device: DeviceId,
        /// Physical key.
        key_code: KeyCode,
        /// If the key was pressed or released.
        state: KeyState,
    },
    /// User responded to a native message dialog.
    MsgDialogResponse(DialogId, MsgDialogResponse),
    /// User responded to a native file dialog.
    FileDialogResponse(DialogId, FileDialogResponse),

    /// Represents a custom event send by the extension.
    ExtensionEvent(ApiExtensionId, ApiExtensionPayload),
}
impl Event {
    /// Change `self` to incorporate `other` or returns `other` if both events cannot be coalesced.
    #[allow(clippy::result_large_err)]
    pub fn coalesce(&mut self, other: Event) -> Result<(), Event> {
        use Event::*;

        match (self, other) {
            (
                CursorMoved {
                    window,
                    device,
                    coalesced_pos,
                    position,
                },
                CursorMoved {
                    window: n_window,
                    device: n_device,
                    coalesced_pos: n_coal_pos,
                    position: n_pos,
                },
            ) if *window == n_window && *device == n_device => {
                coalesced_pos.push(*position);
                coalesced_pos.extend(n_coal_pos);
                *position = n_pos;
            }
            // raw mouse motion.
            (
                DeviceMouseMotion { device, delta },
                DeviceMouseMotion {
                    device: n_device,
                    delta: n_delta,
                },
            ) if *device == n_device => {
                *delta += n_delta;
            }

            // wheel scroll.
            (
                MouseWheel {
                    window,
                    device,
                    delta: MouseScrollDelta::LineDelta(delta_x, delta_y),
                    phase,
                },
                MouseWheel {
                    window: n_window,
                    device: n_device,
                    delta: MouseScrollDelta::LineDelta(n_delta_x, n_delta_y),
                    phase: n_phase,
                },
            ) if *window == n_window && *device == n_device && *phase == n_phase => {
                *delta_x += n_delta_x;
                *delta_y += n_delta_y;
            }

            // trackpad scroll-move.
            (
                MouseWheel {
                    window,
                    device,
                    delta: MouseScrollDelta::PixelDelta(delta_x, delta_y),
                    phase,
                },
                MouseWheel {
                    window: n_window,
                    device: n_device,
                    delta: MouseScrollDelta::PixelDelta(n_delta_x, n_delta_y),
                    phase: n_phase,
                },
            ) if *window == n_window && *device == n_device && *phase == n_phase => {
                *delta_x += n_delta_x;
                *delta_y += n_delta_y;
            }

            // raw wheel scroll.
            (
                DeviceMouseWheel {
                    device,
                    delta: MouseScrollDelta::LineDelta(delta_x, delta_y),
                },
                DeviceMouseWheel {
                    device: n_device,
                    delta: MouseScrollDelta::LineDelta(n_delta_x, n_delta_y),
                },
            ) if *device == n_device => {
                *delta_x += n_delta_x;
                *delta_y += n_delta_y;
            }

            // raw trackpad scroll-move.
            (
                DeviceMouseWheel {
                    device,
                    delta: MouseScrollDelta::PixelDelta(delta_x, delta_y),
                },
                DeviceMouseWheel {
                    device: n_device,
                    delta: MouseScrollDelta::PixelDelta(n_delta_x, n_delta_y),
                },
            ) if *device == n_device => {
                *delta_x += n_delta_x;
                *delta_y += n_delta_y;
            }

            // touch
            (
                Touch { window, device, touches },
                Touch {
                    window: n_window,
                    device: n_device,
                    touches: mut n_touches,
                },
            ) if *window == n_window && *device == n_device => {
                touches.append(&mut n_touches);
            }

            // window changed.
            (WindowChanged(change), WindowChanged(n_change))
                if change.window == n_change.window && change.cause == n_change.cause && change.frame_wait_id.is_none() =>
            {
                if n_change.state.is_some() {
                    change.state = n_change.state;
                }

                if n_change.position.is_some() {
                    change.position = n_change.position;
                }

                if n_change.monitor.is_some() {
                    change.monitor = n_change.monitor;
                }

                if n_change.size.is_some() {
                    change.size = n_change.size;
                }

                change.frame_wait_id = n_change.frame_wait_id;
            }
            // window focus changed.
            (FocusChanged { prev, new }, FocusChanged { prev: n_prev, new: n_new })
                if prev.is_some() && new.is_none() && n_prev.is_none() && n_new.is_some() =>
            {
                *new = n_new;
            }
            // scale factor.
            (
                ScaleFactorChanged {
                    monitor,
                    windows,
                    scale_factor,
                },
                ScaleFactorChanged {
                    monitor: n_monitor,
                    windows: n_windows,
                    scale_factor: n_scale_factor,
                },
            ) if *monitor == n_monitor => {
                for w in n_windows {
                    if !windows.contains(&w) {
                        windows.push(w);
                    }
                }
                *scale_factor = n_scale_factor;
            }
            // fonts changed.
            (FontsChanged, FontsChanged) => {}
            // text aa.
            (FontAaChanged(config), FontAaChanged(n_config)) => {
                *config = n_config;
            }
            // double-click timeout.
            (MultiClickConfigChanged(config), MultiClickConfigChanged(n_config)) => {
                *config = n_config;
            }
            // touch config.
            (TouchConfigChanged(config), TouchConfigChanged(n_config)) => {
                *config = n_config;
            }
            // animation enabled and caret speed.
            (AnimationsConfigChanged(config), AnimationsConfigChanged(n_config)) => {
                *config = n_config;
            }
            // key repeat delay and speed.
            (KeyRepeatConfigChanged(config), KeyRepeatConfigChanged(n_config)) => {
                *config = n_config;
            }
            // locale
            (LocaleChanged(config), LocaleChanged(n_config)) => {
                *config = n_config;
            }
            (_, e) => return Err(e),
        }
        Ok(())
    }
}

/// Identify a new touch contact or a contact update.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct TouchUpdate {
    /// Identify a continuous touch contact or *finger*.
    ///
    /// Multiple points of contact can happen in the same device at the same time,
    /// this ID identifies each uninterrupted contact. IDs are unique only among other concurrent touches
    /// on the same device, after a touch is ended an ID may be reused.
    pub touch: TouchId,
    /// Touch phase for the `id`.
    pub phase: TouchPhase,
    /// Touch center, relative to the window top-left in device independent pixels.
    pub position: DipPoint,
    /// Touch pressure force and angle.
    pub force: Option<TouchForce>,
}

/// Cause of a window state change.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventCause {
    /// Operating system or end-user affected the window.
    System,
    /// App affected the window.
    App,
}

/// Describes the force of a touch event.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TouchForce {
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

/// Color scheme preference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorScheme {
    /// Dark foreground, light background.
    Light,

    /// Light foreground, dark background.
    Dark,
}
impl Default for ColorScheme {
    /// Light.
    fn default() -> Self {
        ColorScheme::Light
    }
}

/// Text anti-aliasing.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontAntiAliasing {
    /// Uses the operating system configuration.
    Default,
    /// Sub-pixel anti-aliasing if a fast implementation is available, otherwise uses `Alpha`.
    Subpixel,
    /// Alpha blending anti-aliasing.
    Alpha,
    /// Disable anti-aliasing.
    Mono,
}
impl Default for FontAntiAliasing {
    fn default() -> Self {
        Self::Default
    }
}
impl fmt::Debug for FontAntiAliasing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "FontAntiAliasing::")?;
        }
        match self {
            FontAntiAliasing::Default => write!(f, "Default"),
            FontAntiAliasing::Subpixel => write!(f, "Subpixel"),
            FontAntiAliasing::Alpha => write!(f, "Alpha"),
            FontAntiAliasing::Mono => write!(f, "Mono"),
        }
    }
}

/// The View-Process disconnected or has not finished initializing, try again after the *inited* event.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub struct ViewProcessOffline;
impl fmt::Display for ViewProcessOffline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "view-process disconnected or is initing, try again after the init event")
    }
}
impl std::error::Error for ViewProcessOffline {}

/// View Process IPC result.
pub(crate) type VpResult<T> = std::result::Result<T, ViewProcessOffline>;

/// Frame image capture request.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameCapture {
    /// Don't capture the frame.
    #[default]
    None,
    /// Captures a full BGRA8 image.
    Full,
    /// Captures an A8 mask image.
    Mask(ImageMaskMode),
}

/// Data for rendering a new frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameRequest {
    /// ID of the new frame.
    pub id: FrameId,
    /// Pipeline Tag.
    pub pipeline_id: PipelineId,

    /// Frame clear color.
    pub clear_color: ColorF,

    /// Display list.
    pub display_list: DisplayList,

    /// Create an image or mask from this rendered frame.
    ///
    /// The [`Event::FrameImageReady`] is sent with the image.
    pub capture: FrameCapture,

    /// Identifies this frame as the response to the [`WindowChanged`] resized frame request.
    pub wait_id: Option<FrameWaitId>,
}
impl FrameRequest {
    /// Compute webrender analysis info.
    pub fn render_reasons(&self) -> RenderReasons {
        let mut reasons = RenderReasons::SCENE;

        if self.capture != FrameCapture::None {
            reasons |= RenderReasons::SNAPSHOT;
        }

        reasons
    }
}

/// Data for rendering a new frame that is derived from the current frame.
#[derive(Clone, Serialize, Deserialize)]
pub struct FrameUpdateRequest {
    /// ID of the new frame.
    pub id: FrameId,

    /// Bound transforms.
    pub transforms: Vec<FrameValueUpdate<PxTransform>>,
    /// Bound floats.
    pub floats: Vec<FrameValueUpdate<f32>>,
    /// Bound colors.
    pub colors: Vec<FrameValueUpdate<ColorF>>,

    /// Render update extension key and payload.
    pub extensions: Vec<(ApiExtensionId, ApiExtensionPayload)>,

    /// New clear color.
    pub clear_color: Option<ColorF>,

    /// Create an image or mask from this rendered frame.
    ///
    /// The [`Event::FrameImageReady`] is send with the image.
    pub capture: FrameCapture,

    /// Identifies this frame as the response to the [`WindowChanged`] resized frame request.
    pub wait_id: Option<FrameWaitId>,
}
impl FrameUpdateRequest {
    /// A request that does nothing, apart from re-rendering the frame.
    pub fn empty(id: FrameId) -> FrameUpdateRequest {
        FrameUpdateRequest {
            id,
            transforms: vec![],
            floats: vec![],
            colors: vec![],
            extensions: vec![],
            clear_color: None,
            capture: FrameCapture::None,
            wait_id: None,
        }
    }

    /// If some property updates are requested.
    pub fn has_bounds(&self) -> bool {
        !(self.transforms.is_empty() && self.floats.is_empty() && self.colors.is_empty())
    }

    /// If this request does not do anything, apart from notifying
    /// a new frame if send to the renderer.
    pub fn is_empty(&self) -> bool {
        !self.has_bounds() && self.extensions.is_empty() && self.clear_color.is_none() && self.capture != FrameCapture::None
    }

    /// Compute webrender analysis info.
    pub fn render_reasons(&self) -> RenderReasons {
        let mut reasons = RenderReasons::empty();

        if self.has_bounds() {
            reasons |= RenderReasons::ANIMATED_PROPERTY;
        }

        if self.capture != FrameCapture::None {
            reasons |= RenderReasons::SNAPSHOT;
        }

        if self.clear_color.is_some() {
            reasons |= RenderReasons::CONFIG_CHANGE;
        }

        reasons
    }
}
impl fmt::Debug for FrameUpdateRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FrameUpdateRequest")
            .field("id", &self.id)
            .field("transforms", &self.transforms)
            .field("floats", &self.floats)
            .field("colors", &self.colors)
            .field("clear_color", &self.clear_color)
            .field("capture", &self.capture)
            .finish()
    }
}

/// Configuration of a new window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowRequest {
    /// ID that will identify the new window.
    pub id: WindowId,
    /// Title text.
    pub title: String,

    /// Window state, position, size and restore rectangle.
    pub state: WindowStateAll,

    /// Lock-in kiosk mode.
    ///
    /// If `true` the app-process will only set fullscreen states, never hide or minimize the window, never
    /// make the window chrome visible and only request an opaque window. The view-process implementer is expected
    /// to also never exit the fullscreen state, even temporally.
    ///
    /// The app-process does not expect the view-process to configure the operating system to run in kiosk mode, but
    /// if possible to detect the view-process can assert that it is running in kiosk mode, logging an error if the assert fails.
    pub kiosk: bool,

    /// If the initial position should be provided the operating system,
    /// if this is not possible the `state.restore_rect.origin` is used.
    pub default_position: bool,

    /// Video mode used when the window is in exclusive state.
    pub video_mode: VideoMode,

    /// Window visibility.
    pub visible: bool,
    /// Window taskbar icon visibility.
    pub taskbar_visible: bool,
    /// If the window is "top-most".
    pub always_on_top: bool,
    /// If the user can move the window.
    pub movable: bool,
    /// If the user can resize the window.
    pub resizable: bool,
    /// Window icon.
    pub icon: Option<ImageId>,
    /// Window cursor icon and visibility.
    pub cursor: Option<CursorIcon>,
    /// If the window is see-through in pixels that are not fully opaque.
    pub transparent: bool,

    /// If all or most frames will be *screenshotted*.
    ///
    /// If `false` all resources for capturing frame images
    /// are discarded after each screenshot request.
    pub capture_mode: bool,

    /// Render mode preference for this window.
    pub render_mode: RenderMode,

    /// Focus request indicator on init.
    pub focus_indicator: Option<FocusIndicator>,

    /// Ensures the window is focused after open, if not set the initial focus is decided by
    /// the windows manager, usually focusing the new window only if the process that causes the window has focus.
    pub focus: bool,

    /// Config for renderer extensions.
    pub extensions: Vec<(ApiExtensionId, ApiExtensionPayload)>,
}
impl WindowRequest {
    /// Corrects invalid values if [`kiosk`] is `true`.
    ///
    /// An error is logged for each invalid value.
    ///
    /// [`kiosk`]: Self::kiosk
    pub fn enforce_kiosk(&mut self) {
        if self.kiosk {
            if !self.state.state.is_fullscreen() {
                tracing::error!("window in `kiosk` mode did not request fullscreen");
                self.state.state = WindowState::Exclusive;
            }
            if self.state.chrome_visible {
                tracing::error!("window in `kiosk` mode request chrome");
                self.state.chrome_visible = false;
            }
            if !self.visible {
                tracing::error!("window in `kiosk` mode can only be visible");
                self.visible = true;
            }
        }
    }
}

/// Represents the properties of a window that affect its position, size and state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowStateAll {
    /// The window state.
    pub state: WindowState,

    /// Position across monitors.
    ///
    /// This is mostly used to find a monitor to resolve the `restore_rect` in.
    pub global_position: PxPoint,

    /// Position and size of the window in the `Normal` state.
    ///
    /// The position is relative to the monitor.
    pub restore_rect: DipRect,

    /// What state the window goes too when "restored".
    ///
    /// The *restore* state that the window must be set to be restored, if the [current state] is [`Maximized`], [`Fullscreen`] or [`Exclusive`]
    /// the restore state is [`Normal`], if the [current state] is [`Minimized`] the restore state is the previous state.
    ///
    /// When the restore state is [`Normal`] the [`restore_rect`] defines the window position and size.
    ///
    ///
    /// [current state]: Self::state
    /// [`Maximized`]: WindowState::Maximized
    /// [`Fullscreen`]: WindowState::Fullscreen
    /// [`Exclusive`]: WindowState::Exclusive
    /// [`Normal`]: WindowState::Normal
    /// [`Minimized`]: WindowState::Minimized
    /// [`restore_rect`]: Self::restore_rect
    pub restore_state: WindowState,

    /// Minimal `Normal` size allowed.
    pub min_size: DipSize,
    /// Maximum `Normal` size allowed.
    pub max_size: DipSize,

    /// If the system provided outer-border and title-bar is visible.
    ///
    /// This is also called the "decoration" or "chrome" of the window.
    pub chrome_visible: bool,
}
impl WindowStateAll {
    /// Clamp the `restore_rect.size` to `min_size` and `max_size`.
    pub fn clamp_size(&mut self) {
        self.restore_rect.size = self.restore_rect.size.min(self.max_size).max(self.min_size)
    }

    /// Compute a value for [`restore_state`] given the previous [`state`] in `self` and the `new_state` and update the [`state`].
    ///
    /// [`restore_state`]: Self::restore_state
    /// [`state`]: Self::state
    pub fn set_state(&mut self, new_state: WindowState) {
        self.restore_state = Self::compute_restore_state(self.restore_state, self.state, new_state);
        self.state = new_state;
    }

    /// Compute a value for [`restore_state`] given the previous `prev_state` and the new [`state`] in `self`.
    ///
    /// [`restore_state`]: Self::restore_state
    /// [`state`]: Self::state
    pub fn set_restore_state_from(&mut self, prev_state: WindowState) {
        self.restore_state = Self::compute_restore_state(self.restore_state, prev_state, self.state);
    }

    fn compute_restore_state(restore_state: WindowState, prev_state: WindowState, new_state: WindowState) -> WindowState {
        if new_state == WindowState::Minimized {
            // restore to previous state from minimized.
            if prev_state != WindowState::Minimized {
                prev_state
            } else {
                WindowState::Normal
            }
        } else if new_state.is_fullscreen() && !prev_state.is_fullscreen() {
            // restore to maximized or normal from fullscreen.
            if prev_state == WindowState::Maximized {
                WindowState::Maximized
            } else {
                WindowState::Normal
            }
        } else if new_state == WindowState::Maximized {
            WindowState::Normal
        } else {
            // Fullscreen to/from Exclusive keeps the previous restore_state.
            restore_state
        }
    }
}

/// Render backend preference.
///
/// This is mostly a trade-off between performance and power consumption, but the cold startup time can also be a
/// concern, both `Dedicated` and `Integrated` load the system OpenGL driver, depending on the installed
/// drivers and hardware this can take up to 500ms in rare cases, in most systems this delay stays around 100ms
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RenderMode {
    /// Prefer the *best* dedicated GPU, probably the best performance after initialization, but also the
    /// most power consumption.
    ///
    /// Falls-back to `Integrated`, then `Software`.
    Dedicated,

    /// Prefer the integrated *GPU*, probably the best power consumption and good performance for most GUI applications,
    /// this is the default value.
    ///
    /// Falls-back to `Dedicated`, then `Software`.
    Integrated,

    /// Use a software render fallback, this has the best compatibility and best initialization time. This is probably the
    /// best pick for one frame render tasks and small windows where the initialization time of a GPU context may not offset
    /// the render time gains.
    ///
    /// If the view-process implementation has no software fallback it may use one of the GPUs.
    Software,
}
impl Default for RenderMode {
    /// [`RenderMode::Integrated`].
    fn default() -> Self {
        RenderMode::Integrated
    }
}
impl RenderMode {
    /// Returns fallbacks that view-process implementers will try if `self` is not available.
    pub fn fallbacks(self) -> [RenderMode; 2] {
        use RenderMode::*;
        match self {
            Dedicated => [Integrated, Software],
            Integrated => [Dedicated, Software],
            Software => [Integrated, Dedicated],
        }
    }

    /// Returns `self` plus [`fallbacks`].
    ///
    /// [`fallbacks`]: Self::fallbacks
    pub fn with_fallbacks(self) -> [RenderMode; 3] {
        let [f0, f1] = self.fallbacks();
        [self, f0, f1]
    }
}

/// Configuration of a new headless surface.
///
/// Headless surfaces are always [`capture_mode`] enabled.
///
/// [`capture_mode`]: WindowRequest::capture_mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadlessRequest {
    /// ID that will identify the new headless surface.
    ///
    /// The surface is identified by a [`WindowId`] so that some API methods
    /// can apply to both windows or surfaces, no actual window is created.
    pub id: WindowId,

    /// Scale for the layout units in this config.
    pub scale_factor: f32,

    /// Surface area (viewport size).
    pub size: DipSize,

    /// Render mode preference for this headless surface.
    pub render_mode: RenderMode,

    /// Config for renderer extensions.
    pub extensions: Vec<(ApiExtensionId, ApiExtensionPayload)>,
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
    pub fn dip_size(&self) -> DipSize {
        self.size.to_dip(self.scale_factor)
    }
}

/// Exclusive video mode info.
///
/// You can get this values from [`MonitorInfo::video_modes`]. Note that when setting the
/// video mode the actual system mode is selected by approximation, closest `size`, then `bit_depth` then `refresh_rate`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoMode {
    /// Resolution of this video mode.
    pub size: PxSize,
    /// The bit depth of this video mode, as in how many bits you have available per color.
    /// This is generally 24 bits or 32 bits on modern systems, depending on whether the alpha channel is counted or not.
    pub bit_depth: u16,
    /// The refresh rate of this video mode, in millihertz.
    pub refresh_rate: u32,
}
impl Default for VideoMode {
    fn default() -> Self {
        Self::MAX
    }
}
impl VideoMode {
    /// Default value, matches with the largest size, greatest bit-depth and refresh rate.
    pub const MAX: VideoMode = VideoMode {
        size: PxSize::new(Px::MAX, Px::MAX),
        bit_depth: u16::MAX,
        refresh_rate: u32::MAX,
    };
}
impl fmt::Display for VideoMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == Self::MAX {
            write!(f, "MAX")
        } else {
            write!(
                f,
                "{}x{}, {}, {}hz",
                self.size.width.0,
                self.size.height.0,
                self.bit_depth,
                (self.refresh_rate as f32 * 0.001).round()
            )
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
    pub area: DipSize,
}
impl Default for MultiClickConfig {
    /// `500ms` and `4, 4`.
    fn default() -> Self {
        Self {
            time: Duration::from_millis(500),
            area: DipSize::splat(Dip::new(4)),
        }
    }
}

/// System settings needed to implementing touch gestures.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Deserialize)]
pub struct TouchConfig {
    /// Maximum (x, y) distance between a touch start and end that generates a touch click.
    ///
    /// Area can be disregarded if the touch is not ambiguous. This usually defines the initial lag
    /// for a single finger drag gesture.
    pub tap_area: DipSize,

    /// Maximum (x, y) distance that a subsequent touch click is linked with the previous one as a double click.
    ///
    /// Area can be disregarded if the touch is not ambiguous.
    pub double_tap_area: DipSize,

    /// Maximum time between start and end in the `tap_area` that generates a touch click.
    ///
    /// Time can be disregarded if the touch is not ambiguous. This usually defines the *long press* delay.
    pub tap_max_time: Duration,

    /// Maximum time between taps that generates a double click.
    pub double_tap_max_time: Duration,

    /// Minimum velocity that can be considered a fling gesture, in dip per seconds.
    pub min_fling_velocity: Dip,

    /// Fling velocity ceiling, in dip per seconds.
    pub max_fling_velocity: Dip,
}
impl Default for TouchConfig {
    fn default() -> Self {
        Self {
            tap_area: DipSize::splat(Dip::new(8)),
            double_tap_area: DipSize::splat(Dip::new(28)),
            tap_max_time: Duration::from_millis(300),
            double_tap_max_time: Duration::from_millis(500),
            min_fling_velocity: Dip::new(50),
            max_fling_velocity: Dip::new(8000),
        }
    }
}

/// System settings that define the key pressed repeat.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Deserialize)]
pub struct KeyRepeatConfig {
    /// Delay before repeat starts.
    pub start_delay: Duration,
    /// Delay before each repeat event after the first.
    pub interval: Duration,
}
impl Default for KeyRepeatConfig {
    /// 600ms, 100ms.
    fn default() -> Self {
        Self {
            start_delay: Duration::from_millis(600),
            interval: Duration::from_millis(100),
        }
    }
}

/// System settings that control animations.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Deserialize)]
pub struct AnimationsConfig {
    /// If animation are enabled.
    ///
    /// People with photo-sensitive epilepsy usually disable animations system wide.
    pub enabled: bool,

    /// Interval of the caret blink animation.
    pub caret_blink_interval: Duration,
    /// Duration after which the blink animation stops.
    pub caret_blink_timeout: Duration,
}
impl Default for AnimationsConfig {
    /// true, 530ms, 5s.
    fn default() -> Self {
        Self {
            enabled: true,
            caret_blink_interval: Duration::from_millis(530),
            caret_blink_timeout: Duration::from_secs(5),
        }
    }
}

/// System settings that define the locale.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, Deserialize, Default)]
pub struct LocaleConfig {
    /// BCP-47 language tags, if the locale can be obtained.
    pub langs: Vec<String>,
}

/// Defines how the A8 image mask pixels are to be derived from a source mask image.
#[derive(Debug, Copy, Clone, Serialize, PartialEq, Eq, Hash, Deserialize, Default)]
pub enum ImageMaskMode {
    /// Alpha channel.
    ///
    /// If the image has no alpha channel masks by `Luminance`.
    #[default]
    A,
    /// Blue channel.
    ///
    /// If the image has no color channel fallback to monochrome channel, or `A`.
    B,
    /// Green channel.
    ///
    /// If the image has no color channel fallback to monochrome channel, or `A`.
    G,
    /// Red channel.
    ///
    /// If the image has no color channel fallback to monochrome channel, or `A`.
    R,
    /// Relative luminance.
    ///
    /// If the image has no color channel fallback to monochrome channel, or `A`.
    Luminance,
}

/// Represent a image load/decode request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageRequest<D> {
    /// Image data format.
    pub format: ImageDataFormat,
    /// Image data.
    ///
    /// Bytes layout depends on the `format`, data structure is [`IpcBytes`] or [`IpcBytesReceiver`] in the view API.
    ///
    /// [`IpcBytesReceiver`]: crate::IpcBytesReceiver
    pub data: D,
    /// Maximum allowed decoded size.
    ///
    /// View-process will avoid decoding and return an error if the image decoded to BGRA (4 bytes) exceeds this size.
    /// This limit applies to the image before the `resize_to_fit`.
    pub max_decoded_len: u64,
    /// A size constraints to apply after the image is decoded. The image is resized so both dimensions fit inside
    /// the constraints, the image aspect ratio is preserved.
    pub downscale: Option<ImageDownscale>,
    /// Convert or decode the image into a single channel mask (R8).
    pub mask: Option<ImageMaskMode>,
}

/// Defines how an image is downscaled after decoding.
///
/// The image aspect ratio is preserved in both modes, the image is not upsized, if it already fits the size
/// constraints if will not be resized.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ImageDownscale {
    /// Image is downscaled so that both dimensions fit inside the size.
    Fit(PxSize),
    /// Image is downscaled so that at least one dimension fits inside the size.
    Fill(PxSize),
}
impl From<PxSize> for ImageDownscale {
    /// Fit
    fn from(fit: PxSize) -> Self {
        ImageDownscale::Fit(fit)
    }
}
impl From<Px> for ImageDownscale {
    /// Fit splat
    fn from(fit: Px) -> Self {
        ImageDownscale::Fit(PxSize::splat(fit))
    }
}

/// Format of the image bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImageDataFormat {
    /// Decoded BGRA8.
    ///
    /// This is the internal image format, it indicates the image data
    /// is already decoded and must only be entered into the cache.
    Bgra8 {
        /// Size in pixels.
        size: PxSize,
        /// Pixels-per-inch of the image.
        ppi: Option<ImagePpi>,
    },

    /// Decoded A8.
    ///
    /// This is the internal mask format it indicates the mask data
    /// is already decoded and must only be entered into the cache.
    A8 {
        /// Size in pixels.
        size: PxSize,
    },

    /// The image is encoded, a file extension that maybe identifies
    /// the format is known.
    FileExtension(String),

    /// The image is encoded, MIME type that maybe identifies the format is known.
    MimeType(String),

    /// The image is encoded, a decoder will be selected using the "magic number"
    /// on the beginning of the bytes buffer.
    Unknown,
}
impl From<String> for ImageDataFormat {
    fn from(ext_or_mime: String) -> Self {
        if ext_or_mime.contains('/') {
            ImageDataFormat::MimeType(ext_or_mime)
        } else {
            ImageDataFormat::FileExtension(ext_or_mime)
        }
    }
}
impl From<&str> for ImageDataFormat {
    fn from(ext_or_mime: &str) -> Self {
        ext_or_mime.to_owned().into()
    }
}
impl From<PxSize> for ImageDataFormat {
    fn from(bgra8_size: PxSize) -> Self {
        ImageDataFormat::Bgra8 {
            size: bgra8_size,
            ppi: None,
        }
    }
}
impl PartialEq for ImageDataFormat {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::FileExtension(l0), Self::FileExtension(r0)) => l0 == r0,
            (Self::MimeType(l0), Self::MimeType(r0)) => l0 == r0,
            (Self::Bgra8 { size: s0, ppi: p0 }, Self::Bgra8 { size: s1, ppi: p1 }) => s0 == s1 && ppi_key(*p0) == ppi_key(*p1),
            (Self::Unknown, Self::Unknown) => true,
            _ => false,
        }
    }
}
impl Eq for ImageDataFormat {}
impl std::hash::Hash for ImageDataFormat {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            ImageDataFormat::Bgra8 { size, ppi } => {
                size.hash(state);
                ppi_key(*ppi).hash(state);
            }
            ImageDataFormat::A8 { size } => {
                size.hash(state);
            }
            ImageDataFormat::FileExtension(ext) => ext.hash(state),
            ImageDataFormat::MimeType(mt) => mt.hash(state),
            ImageDataFormat::Unknown => {}
        }
    }
}

fn ppi_key(ppi: Option<ImagePpi>) -> Option<(u16, u16)> {
    ppi.map(|s| ((s.x * 3.0) as u16, (s.y * 3.0) as u16))
}

/// Represents a successfully decoded image.
///
/// See [`Event::ImageLoaded`].
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageLoadedData {
    /// Image ID.
    pub id: ImageId,
    /// Pixel size.
    pub size: PxSize,
    /// Pixel-per-inch metadata.
    pub ppi: Option<ImagePpi>,
    /// If all pixels have an alpha value of 255.
    pub is_opaque: bool,
    /// If the `pixels` are in a single channel (A8).
    pub is_mask: bool,
    /// Reference to the BGRA8 pre-multiplied image pixels or the A8 pixels if `is_mask`.
    pub pixels: IpcBytes,
}
impl fmt::Debug for ImageLoadedData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImageLoadedData")
            .field("id", &self.id)
            .field("size", &self.size)
            .field("ppi", &self.ppi)
            .field("is_opaque", &self.is_opaque)
            .field("is_mask", &self.is_mask)
            .field("pixels", &format_args!("<{} bytes shared memory>", self.pixels.len()))
            .finish()
    }
}

/// Information about a successfully opened window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowOpenData {
    /// Window renderer ID namespace.
    pub id_namespace: webrender_api::IdNamespace,
    /// Window renderer pipeline.
    pub pipeline_id: webrender_api::PipelineId,

    /// Window complete state.
    pub state: WindowStateAll,

    /// Monitor that contains the window, if any.
    pub monitor: Option<MonitorId>,

    /// Final top-left offset of the window (excluding outer chrome).
    ///
    /// The values are the global position and the position in the monitor.
    pub position: (PxPoint, DipPoint),
    /// Final dimensions of the client area of the window (excluding outer chrome).
    pub size: DipSize,

    /// Final scale factor.
    pub scale_factor: f32,

    /// Actual render mode, can be different from the requested mode if it is not available.
    pub render_mode: RenderMode,

    /// Preferred color scheme.
    pub color_scheme: ColorScheme,
}

/// Information about a successfully opened headless surface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadlessOpenData {
    /// Window renderer ID namespace.
    pub id_namespace: webrender_api::IdNamespace,
    /// Window renderer pipeline.
    pub pipeline_id: webrender_api::PipelineId,

    /// Actual render mode, can be different from the requested mode if it is not available.
    pub render_mode: RenderMode,
}
impl HeadlessOpenData {
    /// Create an *invalid* result, for when the surface can not be opened.
    pub fn invalid() -> Self {
        HeadlessOpenData {
            id_namespace: webrender_api::IdNamespace(0),
            pipeline_id: webrender_api::PipelineId::dummy(),
            render_mode: RenderMode::Software,
        }
    }

    /// If any of the data is invalid.
    pub fn is_invalid(&self) -> bool {
        let invalid = Self::invalid();
        self.pipeline_id == invalid.pipeline_id || self.id_namespace == invalid.id_namespace
    }
}

/// Represents a focus request indicator.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FocusIndicator {
    /// Activate critical focus request.
    Critical,
    /// Activate informational focus request.
    Info,
}

/// Custom serialized data, in a format defined by the extension.
///
/// Note that the bytes here should represent a serialized small `struct` only, you
/// can add an [`IpcBytes`] or [`IpcBytesReceiver`] field to this struct to transfer
/// large payloads.
///
/// [`IpcBytesReceiver`]: crate::ipc::IpcBytesReceiver
#[derive(Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ApiExtensionPayload(#[serde(with = "serde_bytes")] pub Vec<u8>);
impl ApiExtensionPayload {
    /// Serialize the payload.
    pub fn serialize<T: Serialize>(payload: &T) -> bincode::Result<Self> {
        bincode::serialize(payload).map(Self)
    }

    /// Deserialize the payload.
    pub fn deserialize<T: serde::de::DeserializeOwned>(&self) -> Result<T, ApiExtensionRecvError> {
        if let Some((id, error)) = self.parse_invalid_request() {
            Err(ApiExtensionRecvError::InvalidRequest {
                extension_id: id,
                error: error.to_owned(),
            })
        } else if let Some(id) = self.parse_unknown_extension() {
            Err(ApiExtensionRecvError::UnknownExtension { extension_id: id })
        } else {
            bincode::deserialize(&self.0).map_err(ApiExtensionRecvError::Deserialize)
        }
    }

    /// Empty payload.
    pub const fn empty() -> Self {
        Self(vec![])
    }

    /// Value returned when an invalid extension is requested.
    ///
    /// Value is a string `"zero-ui-view-api.unknown_extension;id={extension_id}"`.
    pub fn unknown_extension(extension_id: ApiExtensionId) -> Self {
        Self(format!("zero-ui-view-api.unknown_extension;id={extension_id}").into_bytes())
    }

    /// Value returned when an invalid request is made for a valid extension key.
    ///
    /// Value is a string `"zero-ui-view-api.invalid_request;id={extension_id};error={error}"`.
    pub fn invalid_request(extension_id: ApiExtensionId, error: impl fmt::Display) -> Self {
        Self(format!("zero-ui-view-api.invalid_request;id={extension_id};error={error}").into_bytes())
    }

    /// If the payload is an [`unknown_extension`] error message, returns the key.
    ///
    /// if the payload starts with the invalid request header and the key cannot be retrieved the
    /// [`ApiExtensionId::INVALID`] is returned as the key.
    ///
    /// [`unknown_extension`]: Self::unknown_extension
    pub fn parse_unknown_extension(&self) -> Option<ApiExtensionId> {
        let p = self.0.strip_prefix(b"zero-ui-view-api.unknown_extension;")?;
        if let Some(p) = p.strip_prefix(b"id=") {
            if let Ok(id_str) = std::str::from_utf8(p) {
                return match id_str.parse::<ApiExtensionId>() {
                    Ok(id) => Some(id),
                    Err(id) => Some(id),
                };
            }
        }
        Some(ApiExtensionId::INVALID)
    }

    /// If the payload is an [`invalid_request`] error message, returns the key and error.
    ///
    /// if the payload starts with the invalid request header and the key cannot be retrieved the
    /// [`ApiExtensionId::INVALID`] is returned as the key and the error message will mention "corrupted payload".
    ///
    /// [`invalid_request`]: Self::invalid_request
    pub fn parse_invalid_request(&self) -> Option<(ApiExtensionId, &str)> {
        let p = self.0.strip_prefix(b"zero-ui-view-api.invalid_request;")?;
        if let Some(p) = p.strip_prefix(b"id=") {
            if let Some(id_end) = p.iter().position(|&b| b == b';') {
                if let Ok(id_str) = std::str::from_utf8(&p[..id_end]) {
                    let id = match id_str.parse::<ApiExtensionId>() {
                        Ok(id) => id,
                        Err(id) => id,
                    };
                    if let Some(p) = p[id_end..].strip_prefix(b";error=") {
                        if let Ok(err_str) = std::str::from_utf8(p) {
                            return Some((id, err_str));
                        }
                    }
                    return Some((id, "invalid request, corrupted payload, unknown error"));
                }
            }
        }
        Some((
            ApiExtensionId::INVALID,
            "invalid request, corrupted payload, unknown extension_id and error",
        ))
    }
}
impl fmt::Debug for ApiExtensionPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ExtensionPayload({} bytes)", self.0.len())
    }
}

/// Identifies an API extension and version.
///
/// Note that the version is part of the name, usually in the pattern "crate-name.extension.v2",
/// there are no minor versions, all different versions are considered breaking changes and
/// must be announced and supported by exact match only. You can still communicate non-breaking changes
/// by using the extension payload
#[derive(Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ApiExtensionName {
    name: String,
}
impl ApiExtensionName {
    /// New from unique name.
    ///
    /// The name must contain at least 1 characters, and match the pattern `[a-zA-Z][a-zA-Z0-9-_.]`.
    pub fn new(name: impl Into<String>) -> Result<Self, ApiExtensionNameError> {
        let name = name.into();
        Self::new_impl(name)
    }
    fn new_impl(name: String) -> Result<ApiExtensionName, ApiExtensionNameError> {
        if name.is_empty() {
            return Err(ApiExtensionNameError::NameCannotBeEmpty);
        }
        for (i, c) in name.char_indices() {
            if i == 0 {
                if !c.is_ascii_alphabetic() {
                    return Err(ApiExtensionNameError::NameCannotStartWithChar(c));
                }
            } else if !c.is_ascii_alphanumeric() && c != '_' && c != '-' && c != '.' {
                return Err(ApiExtensionNameError::NameInvalidChar(c));
            }
        }

        Ok(Self { name })
    }
}
impl fmt::Debug for ApiExtensionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.name, f)
    }
}
impl fmt::Display for ApiExtensionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.name, f)
    }
}
impl ops::Deref for ApiExtensionName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.name.as_str()
    }
}
impl From<&'static str> for ApiExtensionName {
    fn from(value: &'static str) -> Self {
        Self::new(value).unwrap()
    }
}

/// API extension invalid name.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ApiExtensionNameError {
    /// Name cannot empty `""`.
    NameCannotBeEmpty,
    /// Name can only start with ASCII alphabetic chars `[a-zA-Z]`.
    NameCannotStartWithChar(char),
    /// Name can only contains `[a-zA-Z0-9-_.]`.
    NameInvalidChar(char),
}
impl fmt::Display for ApiExtensionNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiExtensionNameError::NameCannotBeEmpty => write!(f, "API extension name cannot be empty"),
            ApiExtensionNameError::NameCannotStartWithChar(c) => {
                write!(f, "API cannot start with '{c}', name pattern `[a-zA-Z][a-zA-Z0-9-_.]`")
            }
            ApiExtensionNameError::NameInvalidChar(c) => write!(f, "API cannot contain '{c}', name pattern `[a-zA-Z][a-zA-Z0-9-_.]`"),
        }
    }
}
impl std::error::Error for ApiExtensionNameError {}

/// List of available API extensions.
#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ApiExtensions(Vec<ApiExtensionName>);
impl ops::Deref for ApiExtensions {
    type Target = [ApiExtensionName];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl ApiExtensions {
    /// New Empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the position of the `ext` in the list of available extensions. This index
    /// identifies the API extension in the [`Api::app_extension`] and [`Api::render_extension`].
    ///
    /// The key can be cached only for the duration of the view process, each view re-instantiation
    /// must query for the presence of the API extension again, and it may change position on the list.
    ///
    /// [`Api::app_extension`]: crate::Api::app_extension
    /// [`Api::render_extension`]: crate::Api::render_extension
    pub fn id(&self, ext: &ApiExtensionName) -> Option<ApiExtensionId> {
        self.0.iter().position(|e| e == ext).map(ApiExtensionId::from_index)
    }

    /// Push the `ext` to the list, if it is not already inserted.
    ///
    /// Returns `Ok(key)` if inserted or `Err(key)` is was already in list.
    pub fn insert(&mut self, ext: ApiExtensionName) -> Result<ApiExtensionId, ApiExtensionId> {
        if let Some(key) = self.id(&ext) {
            Err(key)
        } else {
            let key = self.0.len();
            self.0.push(ext);
            Ok(ApiExtensionId::from_index(key))
        }
    }
}

/// Identifies an [`ApiExtensionName`] in a list.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ApiExtensionId(u32);
impl fmt::Debug for ApiExtensionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == Self::INVALID {
            if f.alternate() {
                write!(f, "ApiExtensionId::")?;
            }
            write!(f, "INVALID")
        } else {
            write!(f, "ApiExtensionId({})", self.0 - 1)
        }
    }
}
impl fmt::Display for ApiExtensionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == Self::INVALID {
            write!(f, "invalid")
        } else {
            write!(f, "{}", self.0 - 1)
        }
    }
}
impl ApiExtensionId {
    /// Dummy ID.
    pub const INVALID: Self = Self(0);

    /// Gets the ID as a list index.
    ///
    /// # Panics
    ///
    /// Panics if called in `INVALID`.
    pub fn index(self) -> usize {
        self.0.checked_sub(1).expect("invalid id") as _
    }

    /// New ID from the index of an [`ApiExtensionName`] in a list.
    ///
    /// # Panics
    ///
    /// Panics if `idx > u32::MAX - 1`.
    pub fn from_index(idx: usize) -> Self {
        if idx > (u32::MAX - 1) as _ {
            panic!("index out-of-bounds")
        }
        Self(idx as u32 + 1)
    }
}
impl std::str::FromStr for ApiExtensionId {
    type Err = Self;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.parse::<u32>() {
            Ok(i) => {
                let r = Self::from_index(i as _);
                if r == Self::INVALID {
                    Err(r)
                } else {
                    Ok(r)
                }
            }
            Err(_) => Err(Self::INVALID),
        }
    }
}

/// Error in the response of an API extension call.
#[derive(Debug)]
pub enum ApiExtensionRecvError {
    /// Requested extension was not in the list of extensions.
    UnknownExtension {
        /// Extension that was requested.
        ///
        /// Is `INVALID` only if error message is corrupted.
        extension_id: ApiExtensionId,
    },
    /// Invalid request format.
    InvalidRequest {
        /// Extension that was requested.
        ///
        /// Is `INVALID` only if error message is corrupted.
        extension_id: ApiExtensionId,
        /// Message from the view-process.
        error: String,
    },
    /// Failed to deserialize to the expected response type.
    Deserialize(bincode::Error),
}
impl fmt::Display for ApiExtensionRecvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiExtensionRecvError::UnknownExtension { extension_id } => write!(f, "invalid API request for unknown id {extension_id:?}"),
            ApiExtensionRecvError::InvalidRequest { extension_id, error } => {
                write!(f, "invalid API request for extension id {extension_id:?}, {error}")
            }
            ApiExtensionRecvError::Deserialize(e) => write!(f, "API extension response failed to deserialize, {e}"),
        }
    }
}
impl std::error::Error for ApiExtensionRecvError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Self::Deserialize(e) = self {
            Some(e)
        } else {
            None
        }
    }
}

/// Defines a native message dialog.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct MsgDialog {
    /// Message dialog window title.
    pub title: String,
    /// Message text.
    pub message: String,
    /// Kind of message.
    pub icon: MsgDialogIcon,
    /// Message buttons.
    pub buttons: MsgDialogButtons,
}
impl Default for MsgDialog {
    fn default() -> Self {
        Self {
            title: String::new(),
            message: String::new(),
            icon: MsgDialogIcon::Info,
            buttons: MsgDialogButtons::Ok,
        }
    }
}

/// Icon of a message dialog.
///
/// Defines the overall *level* style of the dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MsgDialogIcon {
    /// Informational.
    Info,
    /// Warning.
    Warn,
    /// Error.
    Error,
}

/// Buttons of a message dialog.
///
/// Defines what kind of question the user is answering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MsgDialogButtons {
    /// Ok.
    ///
    /// Just a confirmation of message received.
    Ok,
    /// Ok or Cancel.
    ///
    /// Approve selected choice or cancel.
    OkCancel,
    /// Yes or No.
    YesNo,
}

/// Response to a message dialog.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MsgDialogResponse {
    ///
    Ok,
    ///
    Yes,
    ///
    No,
    ///
    Cancel,
    /// Failed to show the message.
    ///
    /// The associated string may contain debug information, caller should assume that native file dialogs
    /// are not available for the given window ID at the current view-process instance.
    Error(String),
}

/// Defines a native file dialog.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct FileDialog {
    /// Dialog window title.
    pub title: String,
    /// Selected directory when the dialog opens.
    pub starting_dir: PathBuf,
    /// Starting file name.
    pub starting_name: String,
    /// File extension filters.
    ///
    /// Syntax:
    ///
    /// ```txt
    /// Display Name|ext1;ext2|All Files|*
    /// ```
    ///
    /// You can use the [`push_filter`] method to create filters. Note that the extensions are
    /// not glob patterns, they must be an extension (without the dot prefix) or `*` for all files.
    ///
    /// [`push_filter`]: Self::push_filter
    pub filters: String,

    /// Defines the file  dialog looks and what kind of result is expected.
    pub kind: FileDialogKind,
}
impl FileDialog {
    /// Push a filter entry.
    pub fn push_filter(&mut self, display_name: &str, extensions: &[&str]) -> &mut Self {
        if !self.filters.is_empty() && !self.filters.ends_with('|') {
            self.filters.push('|');
        }

        let mut extensions: Vec<_> = extensions
            .iter()
            .copied()
            .filter(|&s| !s.contains('|') && !s.contains(';'))
            .collect();
        if extensions.is_empty() {
            extensions = vec!["*"];
        }

        let display_name = display_name.replace('|', " ");
        let display_name = display_name.trim();
        if !display_name.is_empty() {
            self.filters.push_str(display_name);
            self.filters.push_str(" (");
        }
        let mut prefix = "";
        for pat in &extensions {
            self.filters.push_str(prefix);
            prefix = ", ";
            self.filters.push_str("*.");
            self.filters.push_str(pat);
        }
        if !display_name.is_empty() {
            self.filters.push(')');
        }

        self.filters.push('|');

        prefix = "";
        for pat in extensions {
            self.filters.push_str(prefix);
            prefix = ";";
            self.filters.push_str(pat);
        }

        self
    }

    /// Iterate over filter entries and patterns.
    pub fn iter_filters(&self) -> impl Iterator<Item = (&str, impl Iterator<Item = &str>)> {
        struct Iter<'a> {
            filters: &'a str,
        }
        struct PatternIter<'a> {
            patterns: &'a str,
        }
        impl<'a> Iterator for Iter<'a> {
            type Item = (&'a str, PatternIter<'a>);

            fn next(&mut self) -> Option<Self::Item> {
                if let Some(i) = self.filters.find('|') {
                    let display_name = &self.filters[..i];
                    self.filters = &self.filters[i + 1..];

                    let patterns = if let Some(i) = self.filters.find('|') {
                        let pat = &self.filters[..i];
                        self.filters = &self.filters[i + 1..];
                        pat
                    } else {
                        let pat = self.filters;
                        self.filters = "";
                        pat
                    };

                    if !patterns.is_empty() {
                        Some((display_name.trim(), PatternIter { patterns }))
                    } else {
                        self.filters = "";
                        None
                    }
                } else {
                    self.filters = "";
                    None
                }
            }
        }
        impl<'a> Iterator for PatternIter<'a> {
            type Item = &'a str;

            fn next(&mut self) -> Option<Self::Item> {
                if let Some(i) = self.patterns.find(';') {
                    let pattern = &self.patterns[..i];
                    self.patterns = &self.patterns[i + 1..];
                    Some(pattern.trim())
                } else if !self.patterns.is_empty() {
                    let pat = self.patterns;
                    self.patterns = "";
                    Some(pat)
                } else {
                    self.patterns = "";
                    None
                }
            }
        }
        Iter {
            filters: self.filters.trim_start().trim_start_matches('|'),
        }
    }
}
impl Default for FileDialog {
    fn default() -> Self {
        FileDialog {
            title: String::new(),
            starting_dir: PathBuf::new(),
            starting_name: String::new(),
            filters: String::new(),
            kind: FileDialogKind::OpenFile,
        }
    }
}

/// Kind of file dialogs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum FileDialogKind {
    /// Pick one file for reading.
    OpenFile,
    /// Pick one or many files for reading.
    OpenFiles,
    /// Pick one directory for reading.
    SelectFolder,
    /// Pick one or many directories for reading.
    SelectFolders,
    /// Pick one file for writing.
    SaveFile,
}

/// Response to a message dialog.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum FileDialogResponse {
    /// Selected paths.
    ///
    /// Is never empty.
    Selected(Vec<PathBuf>),
    /// User did not select any path.
    Cancel,
    /// Failed to show the dialog.
    ///
    /// The associated string may contain debug information, caller should assume that native file dialogs
    /// are not available for the given window ID at the current view-process instance.
    Error(String),
}

/// Clipboard data.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ClipboardData {
    /// Text string.
    ///
    /// View-process can convert between [`String`] and the text formats of the platform.
    Text(String),
    /// Image data.
    ///
    /// View-process reads from clipboard in any format supported and starts an image decode task
    /// for the data, the [`ImageId`] may still be decoding when received. For writing the
    /// view-process will expect the image to already be loaded, the image will be encoded in
    /// a format compatible with the platform clipboard.
    Image(ImageId),
    /// List of paths.
    FileList(Vec<PathBuf>),
    /// Any data format supported only by the specific view-process implementation.
    Extension {
        /// Type key, must be in a format defined by the view-process.
        data_type: String,
        /// The raw data.
        data: IpcBytes,
    },
}

/// Clipboard data type.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ClipboardType {
    /// A [`ClipboardData::Text`].
    Text,
    /// A [`ClipboardData::Image`].
    Image,
    /// A [`ClipboardData::FileList`].
    FileList,
    /// A [`ClipboardData::Extension`].
    Extension(String),
}

/// Clipboard read/write error.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ClipboardError {
    /// Requested format is not set on the clipboard.
    NotFound,
    /// View-process or operating system does not support the data type.
    NotSupported,
    /// Other error.
    ///
    /// The string can be a debug description of the error, only suitable for logging.
    Other(String),
}
impl fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClipboardError::NotFound => write!(f, "clipboard does not contains requested format"),
            ClipboardError::NotSupported => write!(f, "clipboard implementation does not support the format"),
            ClipboardError::Other(_) => write!(f, "internal error"),
        }
    }
}
impl std::error::Error for ClipboardError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_filters() {
        let mut dlg = FileDialog {
            title: "".to_owned(),
            starting_dir: "".into(),
            starting_name: "".to_owned(),
            filters: "".to_owned(),
            kind: FileDialogKind::OpenFile,
        };

        let expected = "Display Name (*.abc, *.bca)|abc;bca|All Files (*.*)|*";

        dlg.push_filter("Display Name", &["abc", "bca"]).push_filter("All Files", &["*"]);
        assert_eq!(expected, dlg.filters);

        let expected = vec![("Display Name (*.abc, *.bca)", vec!["abc", "bca"]), ("All Files (*.*)", vec!["*"])];
        let parsed: Vec<(&str, Vec<&str>)> = dlg.iter_filters().map(|(n, p)| (n, p.collect())).collect();
        assert_eq!(expected, parsed);
    }

    #[test]
    fn key_code_iter() {
        let mut iter = KeyCode::all_identified();
        let first = iter.next().unwrap();
        assert_eq!(first, KeyCode::Backquote);

        for k in iter {
            assert_eq!(k.name(), &format!("{:?}", k));
        }
    }
}
