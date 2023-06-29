use crate::units::*;
use crate::DisplayList;
use crate::FrameValueUpdate;
use crate::IpcBytes;
use serde::{Deserialize, Serialize};
use std::ops;
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

/// Hardware-dependent keyboard scan code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ScanCode(pub u32);

/// Identifier for a specific analog axis on some device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AxisId(pub u32);

/// Identifier for a specific button on some device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ButtonId(pub u32);

/// Identifier of a frame or frame update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FrameId(Epoch, u32);
impl FrameId {
    /// Dummy frame ID.
    pub const INVALID: FrameId = FrameId(Epoch(u32::MAX), u32::MAX);

    /// Create first frame id of a window.
    pub fn first() -> FrameId {
        FrameId(Epoch(0), 0)
    }

    /// Create the next full frame ID after the current one.
    pub fn next(self) -> FrameId {
        let mut id = self.0 .0.wrapping_add(1);
        if id == u32::MAX {
            id = 0;
        }
        FrameId(Epoch(id), 0)
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
        (self.0 .0 as u64) << 32 | (self.1 as u64)
    }

    /// Get the full frame ID.
    pub fn epoch(self) -> Epoch {
        self.0
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
    Started,
    /// A finger moved on the screen.
    Moved,
    /// A finger was lifted from the screen.
    Ended,
    /// The system cancelled tracking for the touch.
    Cancelled,
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
    RCtrl,
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
            Key::LAlt | Key::LCtrl | Key::LShift | Key::LLogo | Key::RAlt | Key::RCtrl | Key::RShift | Key::RLogo
        )
    }

    /// If the key is left alt or right alt.
    pub fn is_alt(self) -> bool {
        matches!(self, Key::LAlt | Key::RAlt)
    }

    /// If the key is left ctrl or right ctrl.
    pub fn is_ctrl(self) -> bool {
        matches!(self, Key::LCtrl | Key::RCtrl)
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
    /// Window that has changed state.
    pub window: WindowId,

    /// Window new state, is `None` if the window state did not change.
    pub state: Option<WindowStateAll>,

    /// Window new position, is `None` if the window position did not change.
    pub position: Option<DipPoint>,

    /// Window new monitor and its scale factor.
    ///
    /// The window's monitor change when it is moved enough so that most of the
    /// client area is in the new monitor screen.
    ///
    /// Note that the window's scale factor can also change by system settings, that change
    /// generates an [`Event::ScaleFactorChanged`] event only.
    pub monitor: Option<(MonitorId, f32)>,

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
    pub fn moved(window: WindowId, position: DipPoint, cause: EventCause) -> Self {
        WindowChanged {
            window,
            state: None,
            position: Some(position),
            monitor: None,
            size: None,
            frame_wait_id: None,
            cause,
        }
    }

    /// Create an event that represents window parent monitor change.
    pub fn monitor_changed(window: WindowId, monitor: MonitorId, scale_factor: f32, cause: EventCause) -> Self {
        WindowChanged {
            window,
            state: None,
            position: None,
            monitor: Some((monitor, scale_factor)),
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
    /// The window received a Unicode character.
    ReceivedCharacter(WindowId, char),
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
        /// Device-dependent raw key code.
        scan_code: ScanCode,
        /// If the key was pressed or released.
        state: KeyState,
        /// Device independent key code, if the code was identified.
        key: Option<Key>,
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
    AxisMotion(WindowId, DeviceId, AxisId, f64),
    /// Touch event has been received.
    Touch(WindowId, DeviceId, TouchPhase, DipPoint, Option<TouchForce>, u64),
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
        opaque: bool,
        /// Updated BGRA8 pre-multiplied pixel buffer. This includes all the pixels
        /// decoded so-far.
        partial_bgra8: IpcBytes,
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
        /// Device-dependent raw key code.
        scan_code: ScanCode,
        /// If the key was pressed or released.
        state: KeyState,
        /// Device independent key code, if the code was identified.
        key: Option<Key>,
    },
    /// Device Unicode character input.
    DeviceText(DeviceId, char),
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

    /// Automatically create an image from this rendered frame.
    ///
    /// The [`Event::FrameImageReady`] is sent with the image.
    pub capture_image: bool,

    /// Identifies this frame as the response to the [`WindowChanged`] resized frame request.
    pub wait_id: Option<FrameWaitId>,
}
impl FrameRequest {
    /// Compute webrender analysis info.
    pub fn render_reasons(&self) -> RenderReasons {
        let mut reasons = RenderReasons::SCENE;

        if self.capture_image {
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

    /// Automatically create an image from this rendered frame.
    ///
    /// The [`Event::FrameImageReady`] is send with the image.
    pub capture_image: bool,

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
            capture_image: false,
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
        !self.has_bounds() && self.extensions.is_empty() && self.clear_color.is_none() && !self.capture_image
    }

    /// Compute webrender analysis info.
    pub fn render_reasons(&self) -> RenderReasons {
        let mut reasons = RenderReasons::empty();

        if self.has_bounds() {
            reasons |= RenderReasons::ANIMATED_PROPERTY;
        }

        if self.capture_image {
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
            .field("capture_image", &self.capture_image)
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
        Self {
            size: PxSize::new(Px::MAX, Px::MAX),
            bit_depth: u16::MAX,
            refresh_rate: u32::MAX,
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
            area: DipSize::new(Dip::new(4), Dip::new(4)),
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
    pub opaque: bool,
    /// Reference to the BGRA8 pre-multiplied image pixels.
    pub bgra8: IpcBytes,
}
impl fmt::Debug for ImageLoadedData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImageLoadedData")
            .field("id", &self.id)
            .field("size", &self.size)
            .field("ppi", &self.ppi)
            .field("opaque", &self.opaque)
            .field("bgra8", &format_args!("<{} bytes shared memory>", self.bgra8.len()))
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
    /// The position is relative to the monitor.
    pub position: DipPoint,
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
}
