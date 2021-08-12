use std::path::PathBuf;

use glutin::event::{ElementState, KeyboardInput, ModifiersState, MouseButton, MouseScrollDelta, TouchPhase};
use serde::*;
use webrender::api::{units::LayoutSize, BuiltDisplayListDescriptor, PipelineId};

#[derive(Serialize, Deserialize)]
pub enum Request {
    ProtocolVersion,
    Start(StartRequest),
    OpenWindow(OpenWindowRequest),
    SetWindowTitle(WinId, String),
    SetWindowPosition(WinId, (i32, i32)),
    SetWindowSize(WinId, (u32, u32)),
    SetWindowVisible(WinId, bool),
    CloseWindow(WinId),
    Shutdown,
}

#[derive(Serialize, Deserialize)]
pub struct StartRequest {
    pub device_events: bool,
}

#[derive(Serialize, Deserialize)]
pub struct OpenWindowRequest {
    pub title: String,
    pub pos: (i32, i32),
    pub size: (u32, u32),
    pub visible: bool,
    pub frame: (PipelineId, LayoutSize, (Vec<u8>, BuiltDisplayListDescriptor)),
}

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
    WindowNotFound(WinId),
}

#[derive(Serialize, Deserialize)]
pub enum Ev {
    // Window events
    WindowOpened(WinId),
    WindowResized(WinId, (u32, u32)),
    WindowMoved(WinId, (i32, i32)),
    DroppedFile(WinId, PathBuf),
    HoveredFile(WinId, PathBuf),
    HoveredFileCancelled(WinId),
    ReceivedCharacter(WinId, char),
    Focused(WinId, bool),
    KeyboardInput(WinId, DevId, KeyboardInput),
    ModifiersChanged(WinId, ModifiersState),
    CursorMoved(WinId, DevId, (u32, u32)),
    CursorEntered(WinId, DevId),
    CursorLeft(WinId, DevId),
    MouseWheel(WinId, DevId, MouseScrollDelta, TouchPhase),
    MouseInput(WinId, DevId, ElementState, MouseButton),
    TouchpadPressure(WinId, DevId, f32, i64),
    AxisMotion(WinId, DevId, u32, f64),
    Touch(WinId, DevId, TouchPhase, (u32, u32), Option<Force>, u64),
    ScaleFactorChanged(WinId, f64, (u32, u32)),
    ThemeChanged(WinId, Theme),
    WindowCloseRequested(WinId),
    WindowClosed(WinId),

    // Raw device events
    DeviceAdded(DevId),
    DeviceRemoved(DevId),
    DeviceMouseMotion(DevId, (f64, f64)),
    DeviceMouseWheel(DevId, MouseScrollDelta),
    DeviceMotion(DevId, u32, f64),
    DeviceButton(DevId, u32, ElementState),
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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Theme {
    Light,
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

pub type WinId = u32;
pub type DevId = u32;

pub const MAX_RESPONSE_SIZE: u32 = 1024u32.pow(2) * 20;
pub const MAX_REQUEST_SIZE: u32 = 1024u32.pow(2) * 20;
pub const MAX_EVENT_SIZE: u32 = 1024u32.pow(2) * 20;
