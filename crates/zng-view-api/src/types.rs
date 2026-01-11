//! General event types.

use crate::{
    access::{AccessCmd, AccessNodeId},
    api_extension::{ApiExtensionId, ApiExtensionPayload, ApiExtensions},
    audio::{AudioDecoded, AudioDeviceId, AudioDeviceInfo, AudioId, AudioMetadata, AudioOutputId, AudioOutputOpenData, AudioPlayId},
    config::{
        AnimationsConfig, ChromeConfig, ColorsConfig, FontAntiAliasing, KeyRepeatConfig, LocaleConfig, MultiClickConfig, TouchConfig,
    },
    dialog::{DialogId, FileDialogResponse, MsgDialogResponse, NotificationResponse},
    drag_drop::{DragDropData, DragDropEffect},
    image::{ImageDecoded, ImageEncodeId, ImageId, ImageMetadata},
    keyboard::{Key, KeyCode, KeyLocation, KeyState},
    mouse::{ButtonState, MouseButton, MouseScrollDelta},
    raw_input::{InputDeviceCapability, InputDeviceEvent, InputDeviceId, InputDeviceInfo},
    touch::{TouchPhase, TouchUpdate},
    window::{EventFrameRendered, HeadlessOpenData, MonitorId, MonitorInfo, WindowChanged, WindowId, WindowOpenData},
};

use serde::{Deserialize, Serialize};
use std::fmt;
use zng_task::channel::{ChannelError, IpcBytes};
use zng_txt::Txt;
use zng_unit::{DipPoint, Rgba};

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

            /// Returns self and replace self with [`next`].
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
    /// View-process generation, starts at one and changes every respawn, it is never zero.
    ///
    /// The View Process defines the ID.
    pub struct ViewProcessGen(_);
}

/// Identifier for a specific analog axis on some device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AxisId(pub u32);

/// Identifier for a drag drop operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DragDropId(pub u32);

/// View-process implementation info.
///
/// The view-process implementation may not cover the full API, depending on operating system, build, headless mode.
/// When the view-process does not implement something it just logs an error and ignores the request, this struct contains
/// detailed info about what operations are available in the view-process instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ViewProcessInfo {
    /// View-process generation, changes after respawns and is never zero.
    pub generation: ViewProcessGen,
    /// If the view-process is a respawn from a previous crashed process.
    pub is_respawn: bool,

    /// Input device events implemented by the view-process.
    pub input_device: InputDeviceCapability,

    /// Window operations implemented by the view-process.
    pub window: crate::window::WindowCapability,

    /// Dialog operations implemented by the view-process.
    pub dialog: crate::dialog::DialogCapability,

    /// System menu capabilities.
    pub menu: crate::menu::MenuCapability,

    /// Clipboard data types and operations implemented by the view-process.
    pub clipboard: crate::clipboard::ClipboardTypes,

    /// Image decode and encode capabilities implemented by the view-process.
    pub image: Vec<crate::image::ImageFormat>,

    /// Audio decode and encode capabilities implemented by the view-process.
    pub audio: Vec<crate::audio::AudioFormat>,

    /// API extensions implemented by the view-process.
    ///
    /// The extension IDs will stay valid for the duration of the view-process.
    pub extensions: ApiExtensions,
}
impl ViewProcessInfo {
    /// New response.
    pub fn new(generation: ViewProcessGen, is_respawn: bool) -> Self {
        Self {
            generation,
            is_respawn,
            input_device: InputDeviceCapability::empty(),
            window: crate::window::WindowCapability::empty(),
            dialog: crate::dialog::DialogCapability::empty(),
            menu: crate::menu::MenuCapability::empty(),
            clipboard: crate::clipboard::ClipboardTypes::new(vec![], vec![], false),
            image: vec![],
            audio: vec![],
            extensions: ApiExtensions::new(),
        }
    }
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
#[non_exhaustive]
pub enum Event {
    /// View-process inited.
    Inited(ViewProcessInfo),
    /// View-process suspended.
    Suspended,

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
    FrameRendered(EventFrameRendered),

    /// Window moved, resized, or minimized/maximized etc.
    ///
    /// This event aggregates events moves, resizes and other state changes into a
    /// single event to simplify tracking composite changes, for example, the window changes size and position
    /// when maximized, this can be trivially observed with this event.
    ///
    /// The [`EventCause`] can be used to identify a state change initiated by the app.
    ///
    /// [`EventCause`]: crate::window::EventCause
    WindowChanged(WindowChanged),

    /// A drag&drop gesture started dragging over the window.
    DragHovered {
        /// Window that is hovered.
        window: WindowId,
        /// Data payload.
        data: Vec<DragDropData>,
        /// Allowed effects.
        allowed: DragDropEffect,
    },
    /// A drag&drop gesture moved over the window.
    DragMoved {
        /// Window that is hovered.
        window: WindowId,
        /// Cursor positions in between the previous event and this one.
        coalesced_pos: Vec<DipPoint>,
        /// Cursor position, relative to the window top-left in device independent pixels.
        position: DipPoint,
    },
    /// A drag&drop gesture finished over the window.
    DragDropped {
        /// Window that received the file drop.
        window: WindowId,
        /// Data payload.
        data: Vec<DragDropData>,
        /// Allowed effects.
        allowed: DragDropEffect,
        /// ID of this drop operation.
        ///
        /// Handlers must call `drag_dropped` with this ID and what effect was applied to the data.
        drop_id: DragDropId,
    },
    /// A drag&drop gesture stopped hovering the window without dropping.
    DragCancelled {
        /// Window that was previous hovered.
        window: WindowId,
    },
    /// A drag started by the app was dropped or canceled.
    AppDragEnded {
        /// Window that started the drag.
        window: WindowId,
        /// Drag ID.
        drag: DragDropId,
        /// Effect applied to the data by the drop target.
        ///
        /// Is a single flag if the data was dropped in a valid drop target, or is empty if was canceled.
        applied: DragDropEffect,
    },

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
        device: InputDeviceId,
        /// Physical key.
        key_code: KeyCode,
        /// If the key was pressed or released.
        state: KeyState,
        /// The location of the key on the keyboard.
        key_location: KeyLocation,

        /// Semantic key unmodified.
        ///
        /// Pressing `Shift+A` key will produce `Key::Char('a')` in QWERTY keyboards, the modifiers are not applied. Note that
        /// the numpad keys do not represents the numbers unmodified
        key: Key,
        /// Semantic key modified by the current active modifiers.
        ///
        /// Pressing `Shift+A` key will produce `Key::Char('A')` in QWERTY keyboards, the modifiers are applied.
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
        device: InputDeviceId,

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
        device: InputDeviceId,
    },
    /// The mouse cursor has left the window.
    MouseLeft {
        /// Window that is no longer hovered by the cursor.
        window: WindowId,
        /// Device that generated the cursor move event.
        device: InputDeviceId,
    },
    /// A mouse wheel movement or touchpad scroll occurred.
    MouseWheel {
        /// Window that was hovered by the cursor when the mouse wheel was used.
        window: WindowId,
        /// Device that generated the mouse wheel event.
        device: InputDeviceId,
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
        device: InputDeviceId,
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
        device: InputDeviceId,
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
        device: InputDeviceId,
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
        device: InputDeviceId,

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
    /// The available audio input and output devices have changed.
    AudioDevicesChanged(Vec<(AudioDeviceId, AudioDeviceInfo)>),
    /// The available raw input devices have changed.
    InputDevicesChanged(Vec<(InputDeviceId, InputDeviceInfo)>),

    /// The window has been requested to close.
    WindowCloseRequested(WindowId),
    /// The window has closed.
    WindowClosed(WindowId),

    /// An image resource already decoded header metadata.
    ImageMetadataDecoded(ImageMetadata),
    /// An image resource has partially or fully decoded.
    ImageDecoded(ImageDecoded),
    /// An image resource failed to decode, the image ID is not valid.
    ImageDecodeError {
        /// The image that failed to decode.
        image: ImageId,
        /// The error message.
        error: Txt,
    },
    /// An image finished encoding.
    ImageEncoded {
        /// Id of the encode task.
        task: ImageEncodeId,
        /// The encoded image data.
        data: IpcBytes,
    },
    /// An image failed to encode.
    ImageEncodeError {
        /// Id of the encode task.
        task: ImageEncodeId,
        /// The error message.
        error: Txt,
    },

    /// An audio resource decoded header metadata.
    AudioMetadataDecoded(AudioMetadata),
    /// An audio resource decoded chunk or finished decoding.
    AudioDecoded(AudioDecoded),
    /// An audio resource failed to decode, the audio ID is not valid.
    AudioDecodeError {
        /// The audio that failed to decode.
        audio: AudioId,
        /// The error message.
        error: Txt,
    },

    /// Audio output is connected with device and ready to receive commands.
    AudioOutputOpened(AudioOutputId, AudioOutputOpenData),
    /// Audio playback stream failed to connect.
    AudioOutputOpenError {
        /// The output ID.
        id: AudioOutputId,
        /// The error message.
        error: Txt,
    },
    /// Audio playback failed.
    AudioPlayError {
        /// The request ID.
        play: AudioPlayId,
        /// The error message.
        error: Txt,
    },

    /* Config events */
    /// System fonts have changed.
    FontsChanged,
    /// System text anti-aliasing configuration has changed.
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
    /// System color scheme or colors changed.
    ColorsConfigChanged(ColorsConfig),
    /// System window chrome (decorations) preference changed.
    ChromeConfigChanged(ChromeConfig),

    /// Raw input device event.
    InputDeviceEvent {
        /// Device that generated the event.
        device: InputDeviceId,
        /// Event.
        event: InputDeviceEvent,
    },

    /// User responded to a native message dialog.
    MsgDialogResponse(DialogId, MsgDialogResponse),
    /// User responded to a native file dialog.
    FileDialogResponse(DialogId, FileDialogResponse),
    /// User dismissed a notification dialog.
    NotificationResponse(DialogId, NotificationResponse),

    /// A system menu command was requested.
    ///
    /// The menu item can be from the application menu or tray icon.
    MenuCommand {
        /// Menu command ID.
        id: Txt,
    },

    /// Accessibility info tree is now required for the window.
    AccessInit {
        /// Window that must now build access info.
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
    /// Accessibility info tree is no longer needed for the window.
    ///
    /// Note that accessibility may be enabled again after this. It is not an error
    /// to send access updates after this, but they will be ignored.
    AccessDeinit {
        /// Window that can release access info.
        window: WindowId,
    },

    /// System low memory warning, some platforms may kill the app if it does not release memory.
    LowMemory,

    /// An internal component panicked, but the view-process managed to recover from it without
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

    /// Signal the view-process is alive.
    ///
    /// The associated value must be the count requested by [`Api::ping`](crate::Api::ping).
    Pong(u16),
}
impl Event {
    /// Change `self` to incorporate `other` or returns `other` if both events cannot be coalesced.
    #[expect(clippy::result_large_err)]
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
            (
                DragMoved {
                    window,
                    coalesced_pos,
                    position,
                },
                DragMoved {
                    window: n_window,
                    coalesced_pos: n_coal_pos,
                    position: n_pos,
                },
            ) if *window == n_window => {
                coalesced_pos.push(*position);
                coalesced_pos.extend(n_coal_pos);
                *position = n_pos;
            }

            (
                InputDeviceEvent { device, event },
                InputDeviceEvent {
                    device: n_device,
                    event: n_event,
                },
            ) if *device == n_device => {
                if let Err(e) = event.coalesce(n_event) {
                    return Err(InputDeviceEvent {
                        device: n_device,
                        event: e,
                    });
                }
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

                if n_change.safe_padding.is_some() {
                    change.safe_padding = n_change.safe_padding;
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
            // drag hovered
            (
                DragHovered {
                    window,
                    data,
                    allowed: effects,
                },
                DragHovered {
                    window: n_window,
                    data: mut n_data,
                    allowed: n_effects,
                },
            ) if *window == n_window && effects.contains(n_effects) => {
                data.append(&mut n_data);
            }
            // drag dropped
            (
                DragDropped {
                    window,
                    data,
                    allowed,
                    drop_id,
                },
                DragDropped {
                    window: n_window,
                    data: mut n_data,
                    allowed: n_allowed,
                    drop_id: n_drop_id,
                },
            ) if *window == n_window && allowed.contains(n_allowed) && *drop_id == n_drop_id => {
                data.append(&mut n_data);
            }
            // drag cancelled
            (DragCancelled { window }, DragCancelled { window: n_window }) if *window == n_window => {}
            // input devices changed
            (InputDevicesChanged(devices), InputDevicesChanged(n_devices)) => {
                *devices = n_devices;
            }
            // audio devices changed
            (AudioDevicesChanged(devices), AudioDevicesChanged(n_devices)) => {
                *devices = n_devices;
            }
            (_, e) => return Err(e),
        }
        Ok(())
    }
}

/// View Process IPC result.
pub(crate) type VpResult<T> = std::result::Result<T, ChannelError>;

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
/// This ID is mostly defined by the app process. IDs that set the most significant
/// bit of the second part (`id.1 & (1 << 63) != 0`) are reserved for the view process.
#[derive(Default, Debug, Clone, Copy, serde::Deserialize, Eq, Hash, PartialEq, serde::Serialize)]
pub struct ReferenceFrameId(pub u64, pub u64);
impl ReferenceFrameId {
    /// If ID does not set the bit that indicates it is generated by the view process.
    pub fn is_app_generated(&self) -> bool {
        self.1 & (1 << 63) == 0
    }
}

/// Nine-patch border repeat mode.
///
/// Defines how the edges and middle region of a nine-patch border is filled.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Serialize, serde::Deserialize, Default)]
pub enum RepeatMode {
    /// The source image's edge regions are stretched to fill the gap between each border.
    #[default]
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
#[cfg(feature = "var")]
zng_var::impl_from_and_into_var! {
    /// Converts `true` to `Repeat` and `false` to the default `Stretch`.
    fn from(value: bool) -> RepeatMode {
        match value {
            true => RepeatMode::Repeat,
            false => RepeatMode::Stretch,
        }
    }
}

/// Color mix blend mode.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize, Default)]
#[non_exhaustive]
pub enum MixBlendMode {
    /// The final color is the top color, regardless of what the bottom color is.
    /// The effect is like two opaque pieces of paper overlapping.
    #[default]
    Normal = 0,
    /// The final color is the result of multiplying the top and bottom colors.
    /// A black layer leads to a black final layer, and a white layer leads to no change.
    /// The effect is like two images printed on transparent film overlapping.
    Multiply = 1,
    /// The final color is the result of inverting the colors, multiplying them, and inverting that value.
    /// A black layer leads to no change, and a white layer leads to a white final layer.
    /// The effect is like two images shining onto a projection screen.
    Screen = 2,
    /// The final color is the result of [`Multiply`] if the bottom color is darker, or [`Screen`] if the bottom color is lighter.
    /// This blend mode is equivalent to [`HardLight`] but with the layers swapped.
    ///
    /// [`Multiply`]: MixBlendMode::Multiply
    /// [`Screen`]: MixBlendMode::Screen
    /// [`HardLight`]: MixBlendMode::HardLight
    Overlay = 3,
    /// The final color is composed of the darkest values of each color channel.
    Darken = 4,
    /// The final color is composed of the lightest values of each color channel.
    Lighten = 5,
    /// The final color is the result of dividing the bottom color by the inverse of the top color.
    /// A black foreground leads to no change.
    /// A foreground with the inverse color of the backdrop leads to a fully lit color.
    /// This blend mode is similar to [`Screen`], but the foreground only needs to be as light as the inverse
    /// of the backdrop to create a fully lit color.
    ///
    /// [`Screen`]: MixBlendMode::Screen
    ColorDodge = 6,
    /// The final color is the result of inverting the bottom color, dividing the value by the top color, and inverting that value.
    /// A white foreground leads to no change. A foreground with the inverse color of the backdrop leads to a black final image.
    /// This blend mode is similar to [`Multiply`], but the foreground only needs to be as dark as the inverse of the backdrop
    /// to make the final image black.
    ///
    /// [`Multiply`]: MixBlendMode::Multiply
    ColorBurn = 7,
    /// The final color is the result of [`Multiply`] if the top color is darker, or [`Screen`] if the top color is lighter.
    /// This blend mode is equivalent to [`Overlay`] but with the layers swapped.
    /// The effect is similar to shining a harsh spotlight on the backdrop.
    ///
    /// The shorthand unit `HardLight!` converts into this.
    ///
    /// [`Multiply`]: MixBlendMode::Multiply
    /// [`Screen`]: MixBlendMode::Screen
    /// [`Overlay`]: MixBlendMode::Overlay
    HardLight = 8,
    /// The final color is similar to [`HardLight`], but softer. This blend mode behaves similar to [`HardLight`].
    /// The effect is similar to shining a diffused spotlight on the backdrop.
    ///
    /// [`HardLight`]: MixBlendMode::HardLight
    SoftLight = 9,
    /// The final color is the result of subtracting the darker of the two colors from the lighter one.
    /// A black layer has no effect, while a white layer inverts the other layer's color.
    Difference = 10,
    /// The final color is similar to [`Difference`], but with less contrast.
    /// As with [`Difference`], a black layer has no effect, while a white layer inverts the other layer's color.
    ///
    /// [`Difference`]: MixBlendMode::Difference
    Exclusion = 11,
    /// The final color has the *hue* of the top color, while using the *saturation* and *luminosity* of the bottom color.
    Hue = 12,
    /// The final color has the *saturation* of the top color, while using the *hue* and *luminosity* of the bottom color.
    /// A pure gray backdrop, having no saturation, will have no effect.
    Saturation = 13,
    /// The final color has the *hue* and *saturation* of the top color, while using the *luminosity* of the bottom color.
    /// The effect preserves gray levels and can be used to colorize the foreground.
    Color = 14,
    /// The final color has the *luminosity* of the top color, while using the *hue* and *saturation* of the bottom color.
    /// This blend mode is equivalent to [`Color`], but with the layers swapped.
    ///
    /// [`Color`]: MixBlendMode::Color
    Luminosity = 15,
    /// The final color adds the top color multiplied by alpha to the bottom color multiplied by alpha.
    /// This blend mode is particularly useful in cross fades where the opacity of both layers transition in reverse.
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
#[non_exhaustive]
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
    /// Top-to-bottom line.
    Vertical,
    /// Left-to-right line.
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
#[non_exhaustive]
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
#[non_exhaustive]
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

/// Result of a focus request.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum FocusResult {
    /// Focus was requested, an [`Event::FocusChanged`] will be send if the operating system gives focus to the window.
    Requested,
    /// Window is already focused.
    AlreadyFocused,
}

/// Defines what raw device events the view-process instance should monitor and notify.
///
/// Raw device events are global and can be received even when the app has no visible window.
///
/// These events are disabled by default as they can impact performance or may require special security clearance,
/// depending on the view-process implementation and operating system.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct DeviceEventsFilter {
    /// What raw input events should be watched/send.
    ///
    /// Note that although the view-process will filter input device events using these flags setting
    /// just one of them may cause a general native listener to init.
    pub input: InputDeviceCapability,
}
impl DeviceEventsFilter {
    /// Default value, no device events are needed.
    pub fn empty() -> Self {
        Self {
            input: InputDeviceCapability::empty(),
        }
    }

    /// If the filter does not include any event.
    pub fn is_empty(&self) -> bool {
        self.input.is_empty()
    }

    /// New with input device events needed.
    pub fn new(input: InputDeviceCapability) -> Self {
        Self { input }
    }
}
impl Default for DeviceEventsFilter {
    fn default() -> Self {
        Self::empty()
    }
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
            assert_eq!(k.name(), &format!("{k:?}"));
        }
    }
}
