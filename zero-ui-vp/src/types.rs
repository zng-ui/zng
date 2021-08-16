pub use glutin::event::{
    AxisId, ButtonId, ElementState, KeyboardInput, ModifiersState, MouseButton, MouseScrollDelta, ScanCode, TouchPhase, VirtualKeyCode,
};
pub use glutin::window::CursorIcon;
use serde::{Deserialize, Serialize};
use std::{fmt, path::PathBuf};
use webrender::api::units::{LayoutPoint, LayoutSize};
use webrender::api::{BuiltDisplayListDescriptor, ColorF, Epoch, PipelineId};

/// Window ID in channel.
///
/// In the View Process this is mapped to a system id.
///
/// In the App Process this is mapped to a unique id that survives View crashes.
pub type WinId = u32;

/// Device ID in channel.
///
/// In the View Process this is mapped to a system id.
///
/// In the App Process this is mapped to a unique id, but does not survived View crashes.
pub type DevId = u32;

/// Monitor screen ID in channel.
///
/// In the View Process this is mapped to a system id.
///
/// In the App Process this is mapped to a unique id, but does not survived View crashes.
///
/// The `0` value is always the primary screen.
pub type ScreenId = u32;

/// System/User events sent from the View Process.
#[derive(Debug, Serialize, Deserialize)]
pub enum Ev {
    /// The View Process crashed and respawned, all resources must be rebuild.
    Respawned,

    // Window events
    WindowResized(WinId, LayoutSize),
    WindowMoved(WinId, LayoutPoint),
    DroppedFile(WinId, PathBuf),
    HoveredFile(WinId, PathBuf),
    HoveredFileCancelled(WinId),
    ReceivedCharacter(WinId, char),
    Focused(WinId, bool),
    KeyboardInput(WinId, DevId, KeyboardInput),
    ModifiersChanged(WinId, ModifiersState),
    CursorMoved(WinId, DevId, LayoutPoint),
    CursorEntered(WinId, DevId),
    CursorLeft(WinId, DevId),
    MouseWheel(WinId, DevId, MouseScrollDelta, TouchPhase),
    MouseInput(WinId, DevId, ElementState, MouseButton),
    TouchpadPressure(WinId, DevId, f32, i64),
    AxisMotion(WinId, DevId, AxisId, f64),
    Touch(WinId, DevId, TouchPhase, LayoutPoint, Option<Force>, u64),
    ScaleFactorChanged(WinId, f32),
    ThemeChanged(WinId, Theme),
    WindowCloseRequested(WinId),
    WindowClosed(WinId),

    // Config events
    FontsChanged,
    TextAaChanged(TextAntiAliasing),

    // Raw device events
    DeviceAdded(DevId),
    DeviceRemoved(DevId),
    DeviceMouseMotion(DevId, (f64, f64)),
    DeviceMouseWheel(DevId, MouseScrollDelta),
    DeviceMotion(DevId, AxisId, f64),
    DeviceButton(DevId, ButtonId, ElementState),
    DeviceKey(DevId, KeyboardInput),
    DeviceText(DevId, char),
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
pub enum Theme {
    /// Dark text on light background.
    Light,

    /// Light text on dark background.
    Dark,
}
impl From<glutin::window::Theme> for Theme {
    fn from(t: glutin::window::Theme) -> Self {
        match t {
            glutin::window::Theme::Light => Theme::Light,
            glutin::window::Theme::Dark => Theme::Dark,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Icon {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
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

/// View Process IPC error.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum Error {
    /// Tried to operate on an unknown window.
    WindowNotFound(WinId),
    /// The View Process crashed and respawned, all resources must be recreated.
    Respawn,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::WindowNotFound(id) => write!(f, "unknown window `{}`", id),
            Error::Respawn => write!(f, "view-process crashed and respawned, all resources must be rebuild"),
        }
    }
}
impl std::error::Error for Error {}

/// View Process IPC result.
pub type Result<T> = std::result::Result<T, Error>;

/// Data for rendering a new frame.
#[derive(Clone, Serialize, Deserialize)]
pub struct FrameRequest {
    /// Frame Tag.
    pub id: Epoch,
    /// Pipeline Tag.
    pub pipeline_id: PipelineId,

    /// Window inner size.
    ///
    /// This is both the viewport_size and document_size for webrender
    /// as we don't do root level scrolling.
    pub size: LayoutSize,

    /// Display list, split in serializable parts.
    pub display_list: (Vec<u8>, BuiltDisplayListDescriptor),
}

/// Configuration of a window.
#[derive(Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    /// Title text.
    pub title: String,
    /// Top-left offset, including the chrome (outer-position).
    pub pos: LayoutPoint,
    /// Content size (inner-size).
    pub size: LayoutSize,
    /// Window visibility.
    pub visible: bool,
    /// Window taskbar icon visibility.
    pub taskbar_visible: bool,
    /// Window chrome visibility (decoration-visibility).
    pub chrome_visible: bool,
    /// In Windows, if `Alt+F4` does **not** causes a close request and instead causes a key-press event.
    pub allow_alt_f4: bool,
    /// OpenGL clear color.
    pub clear_color: Option<ColorF>,
    /// Text anti-aliasing.
    pub text_aa: TextAntiAliasing,
    /// Frame.
    pub frame: FrameRequest,
}

/// BGRA8 pixel data copied from a rendered frame.
#[derive(Clone, Serialize, Deserialize)]
pub struct FramePixels {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,

    /// BGRA8 data, bottom-to-top.
    pub bgra: Vec<u8>,

    /// Scale factor when the frame was rendered.
    pub scale_factor: f32,

    /// If all alpha values are `255`.
    pub opaque: bool,
}
