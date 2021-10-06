use crate::units::*;
use crate::IpcSharedMemory;
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
/// In the App Process this is an unique id that survives View crashes.
///
/// Zero is never an ID.
pub type WindowId = u32;

/// Device ID in channel.
///
/// In the View Process this is mapped to a system id.
///
/// In the App Process this is mapped to an unique id, but does not survived View crashes.
///
/// Zero is never an ID.
pub type DeviceId = u32;

/// Monitor screen ID in channel.
///
/// In the View Process this is mapped to a system id.
///
/// In the App Process this is mapped to an unique id, but does not survived View crashes.
///
/// Zero is never an ID.
pub type MonitorId = u32;

/// Id of a decoded image in the cache.
pub type ImageId = u32;

/// View-process generation, starts at one and changes every respawn, it is never zero.
pub type ViewProcessGen = u32;

/// Hardware-dependent keyboard scan code.
pub type ScanCode = u32;

/// Identifier for a specific analog axis on some device.
pub type AxisId = u32;

/// Identifier for a specific button on some device.
pub type ButtonId = u32;

/// Pixels-per-inch of each dimension of an image.
///
/// Is `None` when not loaded or not provided by the decoder.
pub type ImagePpi = Option<(f32, f32)>;

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
pub enum Event {
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
    FrameRendered(WindowId, Epoch),

    /// Window maximized/minimized/restored.
    ///
    /// The [`EventCause`] can be used to identify a state change initiated by the app.
    WindowStateChanged(WindowId, WindowState, EventCause),

    /// The size of the window has changed. Contains the client area’s new dimensions and the window state.
    ///
    /// The [`EventCause`] can be used to identify a resize initiated by the app.
    WindowResized(WindowId, DipSize, EventCause),
    /// The position of the window has changed. Contains the window’s new position.
    ///
    /// The [`EventCause`] can be used to identify a move initiated by the app.
    WindowMoved(WindowId, DipPoint, EventCause),
    /// A file has been dropped into the window.
    ///
    /// When the user drops multiple files at once, this event will be emitted for each file separately.
    DroppedFile(WindowId, PathBuf),
    /// A file is being hovered over the window.
    ///
    /// When the user hovers multiple files at once, this event will be emitted for each file separately.
    HoveredFile(WindowId, PathBuf),
    /// A file was hovered, but has exited the window.
    ///
    /// There will be a single event triggered even if multiple files were hovered.
    HoveredFileCancelled(WindowId),
    /// The window received a Unicode character.
    ReceivedCharacter(WindowId, char),
    /// The window gained or lost focus.
    ///
    /// The parameter is true if the window has gained focus, and false if it has lost focus.
    Focused(WindowId, bool),
    /// An event from the keyboard has been received.
    KeyboardInput(WindowId, DeviceId, ScanCode, KeyState, Option<Key>),
    /// The keyboard modifiers have changed.
    ModifiersChanged(WindowId, ModifiersState),
    /// The cursor has moved on the window.
    ///
    /// Contains a hit-test of the point and the frame epoch that was hit.
    CursorMoved(WindowId, DeviceId, DipPoint, HitTestResult, Epoch),

    /// The cursor has entered the window.
    CursorEntered(WindowId, DeviceId),
    /// The cursor has left the window.
    CursorLeft(WindowId, DeviceId),
    /// A mouse wheel movement or touchpad scroll occurred.
    MouseWheel(WindowId, DeviceId, MouseScrollDelta, TouchPhase),
    /// An mouse button press has been received.
    MouseInput(WindowId, DeviceId, ButtonState, MouseButton),
    /// Touchpad pressure event.
    TouchpadPressure(WindowId, DeviceId, f32, i64),
    /// Motion on some analog axis. May report data redundant to other, more specific events.
    AxisMotion(WindowId, DeviceId, AxisId, f64),
    /// Touch event has been received.
    Touch(WindowId, DeviceId, TouchPhase, DipPoint, Option<Force>, u64),
    /// The window’s scale factor has changed.
    ScaleFactorChanged(WindowId, f32),

    /// The available monitors have changed.
    MonitorsChanged(Vec<(WindowId, MonitorInfo)>),

    /// The system window theme has changed.
    WindowThemeChanged(WindowId, WindowTheme),
    /// The window has been requested to close.
    WindowCloseRequested(WindowId),
    /// The window has closed.
    WindowClosed(WindowId),

    /// An image resource already decoded size and PPI.
    ImageMetadataLoaded(ImageId, PxSize, ImagePpi),
    /// An image resource finished decoding.
    ImageLoaded(ImageId, PxSize, ImagePpi, bool, IpcSharedMemory),
    /// An image resource, progressively decoded has decoded more bytes.
    ImagePartiallyLoaded(ImageId, PxSize, ImagePpi, bool, IpcSharedMemory),
    /// An image resource failed to decode, the image ID is not valid.
    ImageLoadError(ImageId, String),
    /// An image finished encoding.
    ImageEncoded(ImageId, String, Vec<u8>),
    /// An image failed to encode.
    ImageEncodeError(ImageId, String, String),

    /// An image generated from a rendered frame is ready.
    FrameImageReady(WindowId, Epoch, ImageId, PxRect, ImagePpi, bool, IpcSharedMemory),

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
    DeviceAdded(DeviceId),
    /// Device removed.
    DeviceRemoved(DeviceId),
    /// Mouse pointer motion.
    ///
    /// The values if the delta of movement (x, y), not position.
    DeviceMouseMotion(DeviceId, (f64, f64)),
    /// Mouse scroll wheel turn.
    DeviceMouseWheel(DeviceId, MouseScrollDelta),
    /// Motion on some analog axis.
    ///
    /// This includes the mouse device and any other that fits.
    DeviceMotion(DeviceId, AxisId, f64),
    /// Device button press or release.
    DeviceButton(DeviceId, ButtonId, ButtonState),
    /// Device key press or release.
    DeviceKey(DeviceId, ScanCode, KeyState, Option<Key>),
    /// Device Unicode character input.
    DeviceText(DeviceId, char),
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

/// OS theme.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum WindowTheme {
    /// Dark text on light background.
    Light,

    /// Light text on dark background.
    Dark,
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
pub(crate) type VpResult<T> = std::result::Result<T, Respawned>;

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

    /// Automatically create an image from this rendered frame.
    ///
    /// The [`Event::FrameImageReady`] is send with the image.
    pub screenshot: bool,
    /// Optionally only captures a selection of the screenshot.
    pub screenshot_rect: Option<PxRect>,
}
impl fmt::Debug for FrameRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FrameRequest")
            .field("id", &self.id)
            .field("pipeline_id", &self.pipeline_id)
            .finish_non_exhaustive()
    }
}

/// Configuration of a new window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    /// ID that will identify the new window.
    pub id: WindowId,
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

    /// Video mode used when the window is in exclusive state.
    pub video_mode: VideoMode,
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
    pub icon: Option<ImageId>,
    /// If the window is see-through.
    pub transparent: bool,

    /// Text anti-aliasing.
    pub text_aa: TextAntiAliasing,

    /// If all or most frames will be *screenshotted*.
    ///
    /// If `false` all resources for capturing frame images
    /// are discarded after each screenshot request.
    pub capture_mode: bool,
}

/// Configuration of a headless surface.
///
/// Headless surfaces are always [`capture_mode`] enabled.
///
/// [`capture_mode`]: WindowConfig::capture_mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadlessConfig {
    /// ID that will identify the new headless surface.
    ///
    /// The surface is identified by a [`WindowId`] so that some API methods
    /// can apply to both windows or surfaces, no actual window is created.
    pub id: WindowId,

    /// Scale for the layout units in this config.
    pub scale_factor: f32,

    /// Surface area (viewport size).
    pub size: DipSize,

    /// Text anti-aliasing.
    pub text_aa: TextAntiAliasing,
}

/// BGRA8 pixel data copied from a cached decoded image.
#[derive(Clone, Serialize, Deserialize)]
pub struct ImagePixels {
    /// Selection of the image that is copied here.
    pub area: PxRect,
    /// BGRA8 data, top-to-bottom and pre-multiplied.
    pub bgra: ByteBuf,
    /// "Dots-per-inch" metadata of the image.
    pub ppi: ImagePpi,
    /// If all alpha values in `bgra` are `255`.
    pub opaque: bool,
}
impl Default for ImagePixels {
    fn default() -> Self {
        Self {
            area: PxRect::zero(),
            bgra: ByteBuf::default(),
            ppi: None,
            opaque: true,
        }
    }
}
impl fmt::Debug for ImagePixels {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImagePixels")
            .field("area", &self.area)
            .field("bgra", &format_args!("<{} bytes>", self.bgra.len()))
            .field("ppi", &self.ppi)
            .field("opaque", &self.opaque)
            .finish()
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

/// Exclusive video mode info.
///
/// You can get this values from [`MonitorInfo::video_modes`]. Note that when setting the
/// video mode the actual system mode is selected by approximation, closest `size`, then `bit_depth` then `refresh_rate`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct VideoMode {
    /// Resolution of this video mode.
    pub size: PxSize,
    /// The bit depth of this video mode, as in how many bits you have available per color.
    /// This is generally 24 bits or 32 bits on modern systems, depending on whether the alpha channel is counted or not.
    pub bit_depth: u16,
    /// The refresh rate of this video mode.
    ///
    /// Note: the returned refresh rate is an integer approximation, and you shouldn’t rely on this value to be exact.
    pub refresh_rate: u16,
}
impl Default for VideoMode {
    fn default() -> Self {
        Self {
            size: PxSize::new(Px::MAX, Px::MAX),
            bit_depth: u16::MAX,
            refresh_rate: u16::MAX,
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
        ppi: ImagePpi,
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
    fn hash<H: _core::hash::Hasher>(&self, state: &mut H) {
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

fn ppi_key(ppi: ImagePpi) -> Option<(u16, u16)> {
    ppi.map(|(x, y)| ((x * 3.0) as u16, (y * 3.0) as u16))
}
