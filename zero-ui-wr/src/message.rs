use std::fmt;
use std::path::PathBuf;

pub use glutin::event::{
    AxisId, ButtonId, ElementState, KeyboardInput, ModifiersState, MouseButton, MouseScrollDelta, ScanCode, TouchPhase, VirtualKeyCode,
};
pub use glutin::window::CursorIcon;
use serde::*;
use webrender::api::units::LayoutPoint;
use webrender::api::{units::LayoutSize, BuiltDisplayListDescriptor, PipelineId};
use webrender::api::{ColorF, HitTestResult};

/// Requests sent from the App Process.
#[derive(Serialize, Deserialize)]
pub enum Request {
    ProtocolVersion,
    Start(StartRequest),
    OpenWindow(OpenWindowRequest),
    SetWindowTitle(WinId, String),
    SetWindowPosition(WinId, (i32, i32)),
    SetWindowSize(WinId, (u32, u32)),
    SetWindowVisible(WinId, bool),
    HitTest(WinId, LayoutPoint),
    ReadPixels(WinId, [u32; 4]),
    CloseWindow(WinId),
    Shutdown,
}

/// Response for each [`Request`], sent from the View Process.
#[derive(Serialize, Deserialize)]
pub enum Response {
    ProtocolVersion(String),
    Started,
    WindowOpened(WinId),
    WindowResized(WinId, (u32, u32)),
    WindowMoved(WinId, (i32, i32)),
    WindowTitleChanged(WinId),
    WindowVisibilityChanged(WinId, bool),
    WindowClosed(WinId),
    HitTestResult(WinId, HitTestResult),
    FramePixels(WinId, Vec<u8>),
    WindowNotFound(WinId),
}

/// View Process startup config.
#[derive(Serialize, Deserialize)]
pub struct StartRequest {
    /// If raw device events are sent to the App Process.
    pub device_events: bool,
    /// If only the renderer is used and no windows should be visible.
    pub headless: bool,
}

/// View Window start config.
#[derive(Serialize, Deserialize)]
pub struct OpenWindowRequest {
    /// Initial title.
    pub title: String,
    /// Initial position, in device pixels.
    pub pos: (i32, i32),
    /// Initial size, in device pixels.
    pub size: (u32, u32),
    /// Visibility after the first frame is rendered.
    pub visible: bool,

    /// Color used to clear the frame buffer for a new rendering.
    pub clear_color: Option<ColorF>,

    /// Text anti-aliasing.
    pub text_aa: TextAntiAliasing,

    /// The first frame.
    pub frame: (PipelineId, LayoutSize, (Vec<u8>, BuiltDisplayListDescriptor)),
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

/// System/User events sent from the View Process.
#[derive(Debug, Serialize, Deserialize)]
pub enum Ev {
    // Window events
    WindowResized(WinId, (u32, u32)),
    WindowMoved(WinId, (i32, i32)),
    DroppedFile(WinId, PathBuf),
    HoveredFile(WinId, PathBuf),
    HoveredFileCancelled(WinId),
    ReceivedCharacter(WinId, char),
    Focused(WinId, bool),
    KeyboardInput(WinId, DevId, KeyboardInput),
    ModifiersChanged(WinId, ModifiersState),
    CursorMoved(WinId, DevId, (i32, i32)),
    CursorEntered(WinId, DevId),
    CursorLeft(WinId, DevId),
    MouseWheel(WinId, DevId, MouseScrollDelta, TouchPhase),
    MouseInput(WinId, DevId, ElementState, MouseButton),
    TouchpadPressure(WinId, DevId, f32, i64),
    AxisMotion(WinId, DevId, AxisId, f64),
    Touch(WinId, DevId, TouchPhase, (u32, u32), Option<Force>, u64),
    ScaleFactorChanged(WinId, f32, (u32, u32)),
    ThemeChanged(WinId, Theme),
    WindowCloseRequested(WinId),
    WindowClosed(WinId),

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

pub const MAX_RESPONSE_SIZE: u32 = 1024u32.pow(2) * 20;
pub const MAX_REQUEST_SIZE: u32 = 1024u32.pow(2) * 20;
pub const MAX_EVENT_SIZE: u32 = 1024u32.pow(2) * 20;

#[derive(Clone, Serialize, Deserialize)]
pub struct Icon {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}
