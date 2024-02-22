//! General event types.

use crate::{
    access::{AccessCmd, AccessNodeId},
    api_extension::{ApiExtensionId, ApiExtensionPayload, ApiExtensions},
    config::{AnimationsConfig, ColorScheme, FontAntiAliasing, KeyRepeatConfig, LocaleConfig, MultiClickConfig, TouchConfig},
    dialog::{DialogId, FileDialogResponse, MsgDialogResponse},
    image::{ImageId, ImageLoadedData, ImagePpi},
    ipc::IpcBytes,
    keyboard::{Key, KeyCode, KeyState},
    mouse::{ButtonId, ButtonState, MouseButton, MouseScrollDelta},
    touch::{TouchPhase, TouchUpdate},
    window::{EventFrameRendered, FrameId, HeadlessOpenData, MonitorId, MonitorInfo, WindowChanged, WindowId, WindowOpenData},
};
use serde::{Deserialize, Serialize};
use std::{fmt, path::PathBuf};
use zero_ui_txt::Txt;
use zero_ui_unit::{DipPoint, PxRect, PxSize, Rgba};

macro_rules! declare_id {
    ($(
        $(#[$docs:meta])+
        pub struct $Id:ident(_);
    )+) => {$(
        $(#[$docs])+
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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

pub(crate) use declare_id;

declare_id! {
    /// Device ID in channel.
    ///
    /// In the View Process this is mapped to a system id.
    ///
    /// In the App Process this is mapped to an unique id, but does not survived View crashes.
    ///
    /// The View Process defines the ID.
    pub struct DeviceId(_);

    /// View-process generation, starts at one and changes every respawn, it is never zero.
    ///
    /// The View Process defines the ID.
    pub struct ViewProcessGen(_);
}

/// Identifier for a specific analog axis on some device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AxisId(pub u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
/// View process is online.
///
/// The [`ViewProcessGen`] is the generation of the new view-process, it must be passed to
/// [`Controller::handle_inited`].
///
/// [`Controller::handle_inited`]: crate::Controller::handle_inited
pub struct Inited {
    /// View-process generation, changes after respawns and is never zero.
    pub generation: ViewProcessGen,
    /// If the view-process is a respawn from a previous crashed process.
    pub is_respawn: bool,

    /// Available monitors.
    pub available_monitors: Vec<(MonitorId, MonitorInfo)>,
    /// System multi-click config.
    pub multi_click_config: MultiClickConfig,
    /// System keyboard pressed key repeat start delay config.
    pub key_repeat_config: KeyRepeatConfig,
    /// System touch config.
    pub touch_config: TouchConfig,
    /// System font anti-aliasing config.
    pub font_aa: FontAntiAliasing,
    /// System animations config.
    pub animations_config: AnimationsConfig,
    /// System locale config.
    pub locale_config: LocaleConfig,
    /// System preferred color scheme.
    pub color_scheme: ColorScheme,
    /// API extensions implemented by the view-process.
    ///
    /// The extension IDs will stay valid for the duration of the view-process.
    pub extensions: ApiExtensions,
}

/// IME preview or insert event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Ime {
    /// Preview an IME insert at the last non-preview caret/selection.
    ///
    /// The associated values are the preview string and caret/selection inside the preview string.
    ///
    /// The preview must visually replace the last non-preview selection or insert at the last non-preview
    /// caret index. If the preview string is empty the preview must be cancelled.
    Preview(Txt, (usize, usize)),

    /// Apply an IME insert at the last non-preview caret/selection. The caret must be moved to
    /// the end of the inserted sub-string.
    Commit(Txt),
}

/// System and User events sent from the View Process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    /// View-process inited.
    Inited(Inited),

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
        error: Txt,
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
    ///
    /// [`EventCause`]: crate::window::EventCause
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
        key: Key,
        /// Semantic key modified by the current active modifiers.
        ///
        /// Pressing `Shift+A` key will produce `Key::Char('A')` in QWERT keyboards, the modifiers are applied.
        key_modified: Key,
        /// Text typed.
        ///
        /// This is only set during [`KeyState::Pressed`] of a key that generates text.
        ///
        /// This is usually the `key_modified` char, but is also `'\r'` for `Key::Enter`. On Windows when a dead key was
        /// pressed earlier but cannot be combined with the character from this key press, the produced text
        /// will consist of two characters: the dead-key-character followed by the character resulting from this key press.
        text: Txt,
    },
    /// IME composition event.
    Ime {
        /// Window that received the IME event.
        window: WindowId,
        /// IME event.
        ime: Ime,
    },

    /// The mouse cursor has moved on the window.
    ///
    /// This event can be coalesced, i.e. multiple cursor moves packed into the same event.
    MouseMoved {
        /// Window that received the cursor move.
        window: WindowId,
        /// Device that generated the cursor move.
        device: DeviceId,

        /// Cursor positions in between the previous event and this one.
        coalesced_pos: Vec<DipPoint>,

        /// Cursor position, relative to the window top-left in device independent pixels.
        position: DipPoint,
    },

    /// The mouse cursor has entered the window.
    MouseEntered {
        /// Window that now is hovered by the cursor.
        window: WindowId,
        /// Device that generated the cursor move event.
        device: DeviceId,
    },
    /// The mouse cursor has left the window.
    MouseLeft {
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
        error: Txt,
    },
    /// An image finished encoding.
    ImageEncoded {
        /// The image that finished encoding.
        image: ImageId,
        /// The format of the encoded data.
        format: Txt,
        /// The encoded image data.
        data: IpcBytes,
    },
    /// An image failed to encode.
    ImageEncodeError {
        /// The image that failed to encode.
        image: ImageId,
        /// The encoded format that was requested.
        format: Txt,
        /// The error message.
        error: Txt,
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

    /* Config events */
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

    /* Raw device events */
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

    /// Accessibility info tree was requested for the first time on a window.
    AccessInit {
        /// Window that received the first accessibility request.
        window: WindowId,
    },

    /// Accessibility command.
    AccessCommand {
        /// Window that had pixels copied.
        window: WindowId,
        /// Target widget.
        target: AccessNodeId,
        /// Command.
        command: AccessCmd,
    },

    /// System low memory warning, some platforms may kill the app if it does not release memory.
    LowMemory,

    /// An internal component panicked, but the view-process manager to recover from it without
    /// needing to respawn.
    RecoveredFromComponentPanic {
        /// Component identifier.
        component: Txt,
        /// How the view-process recovered from the panic.
        recover: Txt,
        /// The panic.
        panic: Txt,
    },

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
                MouseMoved {
                    window,
                    device,
                    coalesced_pos,
                    position,
                },
                MouseMoved {
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
            // IME commit replaces preview.
            (
                Ime {
                    window,
                    ime: ime @ self::Ime::Preview(_, _),
                },
                Ime {
                    window: n_window,
                    ime: n_ime @ self::Ime::Commit(_),
                },
            ) if *window == n_window => {
                *ime = n_ime;
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

/// Offset and color in a gradient.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct GradientStop {
    /// Offset in pixels.
    pub offset: f32,
    /// Color at the offset.
    pub color: Rgba,
}

/// Border side line style and color.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct BorderSide {
    /// Line color.
    pub color: Rgba,
    /// Line Style.
    pub style: BorderStyle,
}

/// Defines if a widget is part of the same 3D space as the parent.
#[derive(Default, Clone, Copy, serde::Deserialize, Eq, Hash, PartialEq, serde::Serialize)]
#[repr(u8)]
pub enum TransformStyle {
    /// Widget is not a part of the 3D space of the parent. If it has
    /// 3D children they will be rendered into a flat plane that is placed in the 3D space of the parent.
    #[default]
    Flat = 0,
    /// Widget is a part of the 3D space of the parent. If it has 3D children
    /// they will be positioned relative to siblings in the same space.
    ///
    /// Note that some properties require a flat image to work on, in particular all pixel filter properties including opacity.
    /// When such a property is set in a widget that is `Preserve3D` and has both a parent and one child also `Preserve3D` the
    /// filters are ignored and a warning is logged. When the widget is `Preserve3D` and the parent is not the filters are applied
    /// *outside* the 3D space, when the widget is `Preserve3D` with all `Flat` children the filters are applied *inside* the 3D space.
    Preserve3D = 1,
}
impl fmt::Debug for TransformStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "TransformStyle::")?;
        }
        match self {
            Self::Flat => write!(f, "Flat"),
            Self::Preserve3D => write!(f, "Preserve3D"),
        }
    }
}

/// Identifies a reference frame.
///
/// This ID is defined by the app process.
#[derive(Default, Debug, Clone, Copy, serde::Deserialize, Eq, Hash, PartialEq, serde::Serialize)]
pub struct ReferenceFrameId(pub u64, pub u64);

/// Nine-patch border repeat mode.
///
/// Defines how the edges and middle region of a nine-patch border is filled.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum RepeatMode {
    /// The source image's edge regions are stretched to fill the gap between each border.
    Stretch,
    /// The source image's edge regions are tiled (repeated) to fill the gap between each
    /// border. Tiles may be clipped to achieve the proper fit.
    Repeat,
    /// The source image's edge regions are tiled (repeated) to fill the gap between each
    /// border. Tiles may be stretched to achieve the proper fit.
    Round,
    /// The source image's edge regions are tiled (repeated) to fill the gap between each
    /// border. Extra space will be distributed in between tiles to achieve the proper fit.
    Space,
}

/// Color mix blend mode.
#[allow(missing_docs)]
#[repr(u8)]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum MixBlendMode {
    Normal = 0,
    Multiply = 1,
    Screen = 2,
    Overlay = 3,
    Darken = 4,
    Lighten = 5,
    ColorDodge = 6,
    ColorBurn = 7,
    HardLight = 8,
    SoftLight = 9,
    Difference = 10,
    Exclusion = 11,
    Hue = 12,
    Saturation = 13,
    Color = 14,
    Luminosity = 15,
    PlusLighter = 16,
}

/// Image scaling algorithm in the renderer.
///
/// If an image is not rendered at the same size as their source it must be up-scaled or
/// down-scaled. The algorithms used for this scaling can be selected using this `enum`.
///
/// Note that the algorithms used in the renderer value performance over quality and do a good
/// enough job for small or temporary changes in scale only, such as a small size correction or a scaling animation.
/// If and image is constantly rendered at a different scale you should considered scaling it on the CPU using a
/// slower but more complex algorithm or pre-scaling it before including in the app.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ImageRendering {
    /// Let the renderer select the algorithm, currently this is the same as [`CrispEdges`].
    ///
    /// [`CrispEdges`]: ImageRendering::CrispEdges
    Auto = 0,
    /// The image is scaled with an algorithm that preserves contrast and edges in the image,
    /// and which does not smooth colors or introduce blur to the image in the process.
    ///
    /// Currently the [Bilinear] interpolation algorithm is used.
    ///
    /// [Bilinear]: https://en.wikipedia.org/wiki/Bilinear_interpolation
    CrispEdges = 1,
    /// When scaling the image up, the image appears to be composed of large pixels.
    ///
    /// Currently the [Nearest-neighbor] interpolation algorithm is used.
    ///
    /// [Nearest-neighbor]: https://en.wikipedia.org/wiki/Nearest-neighbor_interpolation
    Pixelated = 2,
}

/// Pixel color alpha type.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum AlphaType {
    /// Components are not pre-multiplied by alpha.
    Alpha = 0,
    /// Components are pre-multiplied by alpha.
    PremultipliedAlpha = 1,
}

/// Gradient extend mode.
#[allow(missing_docs)]
#[repr(u8)]
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Serialize, Deserialize, Ord, PartialOrd)]
pub enum ExtendMode {
    Clamp,
    Repeat,
}

/// Orientation of a straight line.
#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LineOrientation {
    /// Top-bottom line.
    Vertical,
    /// Left-right line.
    Horizontal,
}
impl fmt::Debug for LineOrientation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "LineOrientation::")?;
        }
        match self {
            LineOrientation::Vertical => {
                write!(f, "Vertical")
            }
            LineOrientation::Horizontal => {
                write!(f, "Horizontal")
            }
        }
    }
}

/// Represents a line style.
#[allow(missing_docs)]
#[repr(u8)]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum LineStyle {
    Solid,
    Dotted,
    Dashed,

    /// A wavy line, like an error underline.
    ///
    /// The wave magnitude is defined by the overall line thickness, the associated value
    /// here defines the thickness of the wavy line.
    Wavy(f32),
}

/// The line style for the sides of a widget's border.
#[repr(u8)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Hash, Eq, serde::Serialize, serde::Deserialize)]
pub enum BorderStyle {
    /// No border line.
    #[default]
    None = 0,

    /// A single straight solid line.
    Solid = 1,
    /// Two straight solid lines that add up to the pixel size defined by the side width.
    Double = 2,

    /// Displays a series of rounded dots.
    Dotted = 3,
    /// Displays a series of short square-ended dashes or line segments.
    Dashed = 4,

    /// Fully transparent line.
    Hidden = 5,

    /// Displays a border with a carved appearance.
    Groove = 6,
    /// Displays a border with an extruded appearance.
    Ridge = 7,

    /// Displays a border that makes the widget appear embedded.
    Inset = 8,
    /// Displays a border that makes the widget appear embossed.
    Outset = 9,
}

#[cfg(test)]
mod tests {
    use super::*;

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
