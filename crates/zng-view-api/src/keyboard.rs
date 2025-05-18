//! Keyboard types.

use std::{fmt, mem};

use serde::{Deserialize, Serialize};
use zng_txt::Txt;

/// The location of the key on the keyboard.
///
/// Certain physical keys on the keyboard can have the same value, but are in different locations.
/// For instance, the Shift key can be on the left or right side of the keyboard, or the number
/// keys can be above the letters or on the numpad. This enum allows the user to differentiate
/// them.
#[derive(Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum KeyLocation {
    /// The key is in its "normal" location on the keyboard.
    ///
    /// For instance, the "1" key above the "Q" key on a QWERTY keyboard will use this location.
    /// This invariant is also returned when the location of the key cannot be identified.
    Standard,

    /// The key is on the left side of the keyboard.
    ///
    /// For instance, the left Shift key below the Caps Lock key on a QWERTY keyboard will use this
    /// location.
    Left,

    /// The key is on the right side of the keyboard.
    ///
    /// For instance, the right Shift key below the Enter key on a QWERTY keyboard will use this
    /// location.
    Right,

    /// The key is on the numpad.
    ///
    /// For instance, the "1" key on the numpad will use this location.
    Numpad,
}
impl KeyLocation {
    /// Gets the variant name.
    pub fn name(self) -> &'static str {
        serde_variant::to_variant_name(&self).unwrap_or("")
    }
}
impl std::fmt::Debug for KeyLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "KeyLocation::")?;
        }
        write!(f, "{}", self.name())
    }
}

/// Contains the platform-native physical key identifier
///
/// The exact values vary from platform to platform (which is part of why this is a per-platform
/// enum), but the values are primarily tied to the key's physical location on the keyboard.
///
/// This enum is primarily used to store raw key codes when Winit doesn't map a given native
/// physical key identifier to a meaningful [`KeyCode`] variant. In the presence of identifiers we
/// haven't mapped for you yet, this lets you use [`KeyCode`] to:
///
/// - Correctly match key press and release events.
/// - On non-web platforms, support assigning key binds to virtually any key through a UI.
#[derive(Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
#[non_exhaustive]
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
#[derive(Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u16)]
pub enum KeyCode {
    /* source: https://docs.rs/winit/0.29.0-beta.0/src/winit/keyboard.rs.html#201-649 */
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
    /// This is labelled <kbd>AltGr</kbd> on many keyboard layouts.
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
    /// Browser Favorites key.
    BrowserFavorites,
    /// Some laptops place this key to the right of the <kbd>↑</kbd> key.
    BrowserForward,
    /// The "home" button on Android.
    BrowserHome,
    /// Browser Refresh key.
    BrowserRefresh,
    /// Browser Search key.
    BrowserSearch,
    /// Browser Search key.
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
    /// Select Media key.
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
    /// Computer Sleep key.
    Sleep,
    /// Volume Down key.
    AudioVolumeDown,
    /// Volume Mute key.
    AudioVolumeMute,
    /// Volume Up key.
    AudioVolumeUp,
    /// Wakes up the device if it is not already awake.
    WakeUp,
    /// Legacy modifier key. Also called "Super" in certain places.
    Meta,
    /// Legacy modifier key.
    Hyper,
    /// Legacy under-clock key.
    Turbo,
    /// Legacy abort key.
    Abort,
    /// Legacy resume key.
    Resume,
    /// Legacy suspend key.
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
impl Clone for KeyCode {
    fn clone(&self) -> Self {
        *self
    }
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

    /// Gets the key as a static str.
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
#[derive(PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u16)]
pub enum Key {
    /// A key that corresponds to the character typed by the user, taking into account the
    /// user’s current locale setting, and any system-level keyboard mapping overrides that are in
    /// effect.
    Char(char),

    /// A key string that corresponds to the character typed by the user, taking into account the
    /// user’s current locale setting, and any system-level keyboard mapping overrides that are in
    /// effect.
    Str(Txt),

    /// This variant is used when the key cannot be translated to any other variant.
    ///
    /// You can try using the [`KeyCode`] to identify the key.
    Unidentified,

    /// Contains the text representation of the dead-key when available.
    ///
    /// ## Platform-specific
    /// - **Web:** Always contains `None`
    Dead(Option<char>),

    /* SAFETY, no associated data after Alt, see `Key::all_named` */
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
    /// Toggle between normal keyboard and symbols keyboard.
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
    /// the key labelled `Delete` on MacOS keyboards.
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
    /// key labelled `Delete` on MacOS keyboards when `Fn` is active.
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
    /// Cancel key.
    Cancel,
    /// Show the application’s context menu.
    /// This key is commonly found between the right `Super` key and the right `Ctrl` key.
    ContextMenu,
    /// The `Esc` key. This key was originally used to initiate an escape sequence, but is
    /// now more generally used to exit or "escape" the current context, such as closing a dialog
    /// or exiting full screen mode.
    Escape,
    /// Execute key.
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
    /// Select key.
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
    /// Log-off key.
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
    /// Alphanumeric mode.
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
    /// Next IME candidate.
    NextCandidate,
    /// Accept current input method sequence without
    /// conversion in IMEs.
    NonConvert,
    /// Previous IME candidate.
    PreviousCandidate,
    /// IME key.
    Process,
    /// IME key.
    SingleCandidate,
    /// Toggle between Hangul and English modes.
    HangulMode,
    /// Toggle between Hanja and English modes.
    HanjaMode,
    /// Toggle between Junja and English modes.
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
    /// The "Music Player" key.
    LaunchMusicPlayer,
    /// The "Phone" key.
    LaunchPhone,
    /// The "Screen Saver" key.
    LaunchScreenSaver,
    /// The "Excel" key.
    LaunchSpreadsheet,
    /// The "Web Browser" key.
    LaunchWebBrowser,
    /// The "Webcam" key.
    LaunchWebCam,
    /// The "Word" key.
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
    /// "Last Number Redial" key
    LastNumberRedial,
    /// The Notification key. (`KEYCODE_NOTIFICATION`)
    Notification,
    /// Toggle between manner mode state: silent, vibrate, ring, ... (`KEYCODE_MANNER_MODE`)
    MannerMode,
    /// "Voice Dial" key.
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
    /// Toggle between fullscreen and scaled content, or alter magnification level. (`VK_ZOOM`,
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
impl Clone for Key {
    fn clone(&self) -> Self {
        key_clone(self)
    }
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
    #[expect(clippy::should_implement_trait)]
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
            // SAFETY: this is safe because all variants from `Alt` are without associated data and Key is `repr(u16)`.
            const KEY_DATA_SIZE: usize = mem::size_of::<Key>() - mem::size_of::<u16>();
            let s: (u16, [u8; KEY_DATA_SIZE]) = mem::transmute(Key::Alt);
            let e: (u16, [u8; KEY_DATA_SIZE]) = mem::transmute(Key::F35);
            (s.0..=e.0).map(|n| mem::transmute((n, [0u8; KEY_DATA_SIZE])))
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "Key::")?;
        }
        let name = self.name();
        match self {
            Self::Char(c) => write!(f, "{name}({c:?})"),
            Self::Str(s) => write!(f, "{name}({:?})", s.as_str()),
            Self::Dead(c) => write!(f, "{name}({c:?})"),
            _ => write!(f, "{name}"),
        }
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

// monomorphize
fn key_clone(key: &Key) -> Key {
    match key {
        Key::Char(arg0) => Key::Char(*arg0),
        Key::Str(arg0) => Key::Str(arg0.clone()),
        Key::Unidentified => Key::Unidentified,
        Key::Dead(arg0) => Key::Dead(*arg0),
        Key::Alt => Key::Alt,
        Key::AltGraph => Key::AltGraph,
        Key::CapsLock => Key::CapsLock,
        Key::Ctrl => Key::Ctrl,
        Key::Fn => Key::Fn,
        Key::FnLock => Key::FnLock,
        Key::NumLock => Key::NumLock,
        Key::ScrollLock => Key::ScrollLock,
        Key::Shift => Key::Shift,
        Key::Symbol => Key::Symbol,
        Key::SymbolLock => Key::SymbolLock,
        Key::Meta => Key::Meta,
        Key::Hyper => Key::Hyper,
        Key::Super => Key::Super,
        Key::Enter => Key::Enter,
        Key::Tab => Key::Tab,
        Key::Space => Key::Space,
        Key::ArrowDown => Key::ArrowDown,
        Key::ArrowLeft => Key::ArrowLeft,
        Key::ArrowRight => Key::ArrowRight,
        Key::ArrowUp => Key::ArrowUp,
        Key::End => Key::End,
        Key::Home => Key::Home,
        Key::PageDown => Key::PageDown,
        Key::PageUp => Key::PageUp,
        Key::Backspace => Key::Backspace,
        Key::Clear => Key::Clear,
        Key::Copy => Key::Copy,
        Key::CrSel => Key::CrSel,
        Key::Cut => Key::Cut,
        Key::Delete => Key::Delete,
        Key::EraseEof => Key::EraseEof,
        Key::ExSel => Key::ExSel,
        Key::Insert => Key::Insert,
        Key::Paste => Key::Paste,
        Key::Redo => Key::Redo,
        Key::Undo => Key::Undo,
        Key::Accept => Key::Accept,
        Key::Again => Key::Again,
        Key::Attn => Key::Attn,
        Key::Cancel => Key::Cancel,
        Key::ContextMenu => Key::ContextMenu,
        Key::Escape => Key::Escape,
        Key::Execute => Key::Execute,
        Key::Find => Key::Find,
        Key::Help => Key::Help,
        Key::Pause => Key::Pause,
        Key::Play => Key::Play,
        Key::Props => Key::Props,
        Key::Select => Key::Select,
        Key::ZoomIn => Key::ZoomIn,
        Key::ZoomOut => Key::ZoomOut,
        Key::BrightnessDown => Key::BrightnessDown,
        Key::BrightnessUp => Key::BrightnessUp,
        Key::Eject => Key::Eject,
        Key::LogOff => Key::LogOff,
        Key::Power => Key::Power,
        Key::PowerOff => Key::PowerOff,
        Key::PrintScreen => Key::PrintScreen,
        Key::Hibernate => Key::Hibernate,
        Key::Standby => Key::Standby,
        Key::WakeUp => Key::WakeUp,
        Key::AllCandidates => Key::AllCandidates,
        Key::Alphanumeric => Key::Alphanumeric,
        Key::CodeInput => Key::CodeInput,
        Key::Compose => Key::Compose,
        Key::Convert => Key::Convert,
        Key::FinalMode => Key::FinalMode,
        Key::GroupFirst => Key::GroupFirst,
        Key::GroupLast => Key::GroupLast,
        Key::GroupNext => Key::GroupNext,
        Key::GroupPrevious => Key::GroupPrevious,
        Key::ModeChange => Key::ModeChange,
        Key::NextCandidate => Key::NextCandidate,
        Key::NonConvert => Key::NonConvert,
        Key::PreviousCandidate => Key::PreviousCandidate,
        Key::Process => Key::Process,
        Key::SingleCandidate => Key::SingleCandidate,
        Key::HangulMode => Key::HangulMode,
        Key::HanjaMode => Key::HanjaMode,
        Key::JunjaMode => Key::JunjaMode,
        Key::Eisu => Key::Eisu,
        Key::Hankaku => Key::Hankaku,
        Key::Hiragana => Key::Hiragana,
        Key::HiraganaKatakana => Key::HiraganaKatakana,
        Key::KanaMode => Key::KanaMode,
        Key::KanjiMode => Key::KanjiMode,
        Key::Katakana => Key::Katakana,
        Key::Romaji => Key::Romaji,
        Key::Zenkaku => Key::Zenkaku,
        Key::ZenkakuHankaku => Key::ZenkakuHankaku,
        Key::Soft1 => Key::Soft1,
        Key::Soft2 => Key::Soft2,
        Key::Soft3 => Key::Soft3,
        Key::Soft4 => Key::Soft4,
        Key::ChannelDown => Key::ChannelDown,
        Key::ChannelUp => Key::ChannelUp,
        Key::Close => Key::Close,
        Key::MailForward => Key::MailForward,
        Key::MailReply => Key::MailReply,
        Key::MailSend => Key::MailSend,
        Key::MediaClose => Key::MediaClose,
        Key::MediaFastForward => Key::MediaFastForward,
        Key::MediaPause => Key::MediaPause,
        Key::MediaPlay => Key::MediaPlay,
        Key::MediaPlayPause => Key::MediaPlayPause,
        Key::MediaRecord => Key::MediaRecord,
        Key::MediaRewind => Key::MediaRewind,
        Key::MediaStop => Key::MediaStop,
        Key::MediaTrackNext => Key::MediaTrackNext,
        Key::MediaTrackPrevious => Key::MediaTrackPrevious,
        Key::New => Key::New,
        Key::Open => Key::Open,
        Key::Print => Key::Print,
        Key::Save => Key::Save,
        Key::SpellCheck => Key::SpellCheck,
        Key::Key11 => Key::Key11,
        Key::Key12 => Key::Key12,
        Key::AudioBalanceLeft => Key::AudioBalanceLeft,
        Key::AudioBalanceRight => Key::AudioBalanceRight,
        Key::AudioBassBoostDown => Key::AudioBassBoostDown,
        Key::AudioBassBoostToggle => Key::AudioBassBoostToggle,
        Key::AudioBassBoostUp => Key::AudioBassBoostUp,
        Key::AudioFaderFront => Key::AudioFaderFront,
        Key::AudioFaderRear => Key::AudioFaderRear,
        Key::AudioSurroundModeNext => Key::AudioSurroundModeNext,
        Key::AudioTrebleDown => Key::AudioTrebleDown,
        Key::AudioTrebleUp => Key::AudioTrebleUp,
        Key::AudioVolumeDown => Key::AudioVolumeDown,
        Key::AudioVolumeUp => Key::AudioVolumeUp,
        Key::AudioVolumeMute => Key::AudioVolumeMute,
        Key::MicrophoneToggle => Key::MicrophoneToggle,
        Key::MicrophoneVolumeDown => Key::MicrophoneVolumeDown,
        Key::MicrophoneVolumeUp => Key::MicrophoneVolumeUp,
        Key::MicrophoneVolumeMute => Key::MicrophoneVolumeMute,
        Key::SpeechCorrectionList => Key::SpeechCorrectionList,
        Key::SpeechInputToggle => Key::SpeechInputToggle,
        Key::LaunchApplication1 => Key::LaunchApplication1,
        Key::LaunchApplication2 => Key::LaunchApplication2,
        Key::LaunchCalendar => Key::LaunchCalendar,
        Key::LaunchContacts => Key::LaunchContacts,
        Key::LaunchMail => Key::LaunchMail,
        Key::LaunchMediaPlayer => Key::LaunchMediaPlayer,
        Key::LaunchMusicPlayer => Key::LaunchMusicPlayer,
        Key::LaunchPhone => Key::LaunchPhone,
        Key::LaunchScreenSaver => Key::LaunchScreenSaver,
        Key::LaunchSpreadsheet => Key::LaunchSpreadsheet,
        Key::LaunchWebBrowser => Key::LaunchWebBrowser,
        Key::LaunchWebCam => Key::LaunchWebCam,
        Key::LaunchWordProcessor => Key::LaunchWordProcessor,
        Key::BrowserBack => Key::BrowserBack,
        Key::BrowserFavorites => Key::BrowserFavorites,
        Key::BrowserForward => Key::BrowserForward,
        Key::BrowserHome => Key::BrowserHome,
        Key::BrowserRefresh => Key::BrowserRefresh,
        Key::BrowserSearch => Key::BrowserSearch,
        Key::BrowserStop => Key::BrowserStop,
        Key::AppSwitch => Key::AppSwitch,
        Key::Call => Key::Call,
        Key::Camera => Key::Camera,
        Key::CameraFocus => Key::CameraFocus,
        Key::EndCall => Key::EndCall,
        Key::GoBack => Key::GoBack,
        Key::GoHome => Key::GoHome,
        Key::HeadsetHook => Key::HeadsetHook,
        Key::LastNumberRedial => Key::LastNumberRedial,
        Key::Notification => Key::Notification,
        Key::MannerMode => Key::MannerMode,
        Key::VoiceDial => Key::VoiceDial,
        Key::TV => Key::TV,
        Key::TV3DMode => Key::TV3DMode,
        Key::TVAntennaCable => Key::TVAntennaCable,
        Key::TVAudioDescription => Key::TVAudioDescription,
        Key::TVAudioDescriptionMixDown => Key::TVAudioDescriptionMixDown,
        Key::TVAudioDescriptionMixUp => Key::TVAudioDescriptionMixUp,
        Key::TVContentsMenu => Key::TVContentsMenu,
        Key::TVDataService => Key::TVDataService,
        Key::TVInput => Key::TVInput,
        Key::TVInputComponent1 => Key::TVInputComponent1,
        Key::TVInputComponent2 => Key::TVInputComponent2,
        Key::TVInputComposite1 => Key::TVInputComposite1,
        Key::TVInputComposite2 => Key::TVInputComposite2,
        Key::TVInputHDMI1 => Key::TVInputHDMI1,
        Key::TVInputHDMI2 => Key::TVInputHDMI2,
        Key::TVInputHDMI3 => Key::TVInputHDMI3,
        Key::TVInputHDMI4 => Key::TVInputHDMI4,
        Key::TVInputVGA1 => Key::TVInputVGA1,
        Key::TVMediaContext => Key::TVMediaContext,
        Key::TVNetwork => Key::TVNetwork,
        Key::TVNumberEntry => Key::TVNumberEntry,
        Key::TVPower => Key::TVPower,
        Key::TVRadioService => Key::TVRadioService,
        Key::TVSatellite => Key::TVSatellite,
        Key::TVSatelliteBS => Key::TVSatelliteBS,
        Key::TVSatelliteCS => Key::TVSatelliteCS,
        Key::TVSatelliteToggle => Key::TVSatelliteToggle,
        Key::TVTerrestrialAnalog => Key::TVTerrestrialAnalog,
        Key::TVTerrestrialDigital => Key::TVTerrestrialDigital,
        Key::TVTimer => Key::TVTimer,
        Key::AVRInput => Key::AVRInput,
        Key::AVRPower => Key::AVRPower,
        Key::ColorF0Red => Key::ColorF0Red,
        Key::ColorF1Green => Key::ColorF1Green,
        Key::ColorF2Yellow => Key::ColorF2Yellow,
        Key::ColorF3Blue => Key::ColorF3Blue,
        Key::ColorF4Grey => Key::ColorF4Grey,
        Key::ColorF5Brown => Key::ColorF5Brown,
        Key::ClosedCaptionToggle => Key::ClosedCaptionToggle,
        Key::Dimmer => Key::Dimmer,
        Key::DisplaySwap => Key::DisplaySwap,
        Key::DVR => Key::DVR,
        Key::Exit => Key::Exit,
        Key::FavoriteClear0 => Key::FavoriteClear0,
        Key::FavoriteClear1 => Key::FavoriteClear1,
        Key::FavoriteClear2 => Key::FavoriteClear2,
        Key::FavoriteClear3 => Key::FavoriteClear3,
        Key::FavoriteRecall0 => Key::FavoriteRecall0,
        Key::FavoriteRecall1 => Key::FavoriteRecall1,
        Key::FavoriteRecall2 => Key::FavoriteRecall2,
        Key::FavoriteRecall3 => Key::FavoriteRecall3,
        Key::FavoriteStore0 => Key::FavoriteStore0,
        Key::FavoriteStore1 => Key::FavoriteStore1,
        Key::FavoriteStore2 => Key::FavoriteStore2,
        Key::FavoriteStore3 => Key::FavoriteStore3,
        Key::Guide => Key::Guide,
        Key::GuideNextDay => Key::GuideNextDay,
        Key::GuidePreviousDay => Key::GuidePreviousDay,
        Key::Info => Key::Info,
        Key::InstantReplay => Key::InstantReplay,
        Key::Link => Key::Link,
        Key::ListProgram => Key::ListProgram,
        Key::LiveContent => Key::LiveContent,
        Key::Lock => Key::Lock,
        Key::MediaApps => Key::MediaApps,
        Key::MediaAudioTrack => Key::MediaAudioTrack,
        Key::MediaLast => Key::MediaLast,
        Key::MediaSkipBackward => Key::MediaSkipBackward,
        Key::MediaSkipForward => Key::MediaSkipForward,
        Key::MediaStepBackward => Key::MediaStepBackward,
        Key::MediaStepForward => Key::MediaStepForward,
        Key::MediaTopMenu => Key::MediaTopMenu,
        Key::NavigateIn => Key::NavigateIn,
        Key::NavigateNext => Key::NavigateNext,
        Key::NavigateOut => Key::NavigateOut,
        Key::NavigatePrevious => Key::NavigatePrevious,
        Key::NextFavoriteChannel => Key::NextFavoriteChannel,
        Key::NextUserProfile => Key::NextUserProfile,
        Key::OnDemand => Key::OnDemand,
        Key::Pairing => Key::Pairing,
        Key::PinPDown => Key::PinPDown,
        Key::PinPMove => Key::PinPMove,
        Key::PinPToggle => Key::PinPToggle,
        Key::PinPUp => Key::PinPUp,
        Key::PlaySpeedDown => Key::PlaySpeedDown,
        Key::PlaySpeedReset => Key::PlaySpeedReset,
        Key::PlaySpeedUp => Key::PlaySpeedUp,
        Key::RandomToggle => Key::RandomToggle,
        Key::RcLowBattery => Key::RcLowBattery,
        Key::RecordSpeedNext => Key::RecordSpeedNext,
        Key::RfBypass => Key::RfBypass,
        Key::ScanChannelsToggle => Key::ScanChannelsToggle,
        Key::ScreenModeNext => Key::ScreenModeNext,
        Key::Settings => Key::Settings,
        Key::SplitScreenToggle => Key::SplitScreenToggle,
        Key::STBInput => Key::STBInput,
        Key::STBPower => Key::STBPower,
        Key::Subtitle => Key::Subtitle,
        Key::Teletext => Key::Teletext,
        Key::VideoModeNext => Key::VideoModeNext,
        Key::Wink => Key::Wink,
        Key::ZoomToggle => Key::ZoomToggle,
        Key::F1 => Key::F1,
        Key::F2 => Key::F2,
        Key::F3 => Key::F3,
        Key::F4 => Key::F4,
        Key::F5 => Key::F5,
        Key::F6 => Key::F6,
        Key::F7 => Key::F7,
        Key::F8 => Key::F8,
        Key::F9 => Key::F9,
        Key::F10 => Key::F10,
        Key::F11 => Key::F11,
        Key::F12 => Key::F12,
        Key::F13 => Key::F13,
        Key::F14 => Key::F14,
        Key::F15 => Key::F15,
        Key::F16 => Key::F16,
        Key::F17 => Key::F17,
        Key::F18 => Key::F18,
        Key::F19 => Key::F19,
        Key::F20 => Key::F20,
        Key::F21 => Key::F21,
        Key::F22 => Key::F22,
        Key::F23 => Key::F23,
        Key::F24 => Key::F24,
        Key::F25 => Key::F25,
        Key::F26 => Key::F26,
        Key::F27 => Key::F27,
        Key::F28 => Key::F28,
        Key::F29 => Key::F29,
        Key::F30 => Key::F30,
        Key::F31 => Key::F31,
        Key::F32 => Key::F32,
        Key::F33 => Key::F33,
        Key::F34 => Key::F34,
        Key::F35 => Key::F35,
    }
}
