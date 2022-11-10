//! Events directly from `winit` targeting the app windows.
//!
//! These events get processed by [app extensions] to generate the events used in widgets, for example
//! the [`KeyboardManager`] uses the [`RAW_KEY_INPUT_EVENT`] into focus targeted events.
//!
//! # Synthetic Input
//!
//! You can [`notify`] these events to fake hardware input, please be careful that you mimic the exact sequence a real
//! hardware would generate, [app extensions] can assume that the raw events are correct. The [`DeviceId`] for fake
//! input must be unique but constant for each distinctive *synthetic event source*.
//!
//! [app extensions]: crate::app::AppExtension
//! [`KeyboardManager`]: crate::keyboard::KeyboardManager
//! [`RAW_KEY_INPUT_EVENT`]: crate::app::raw_events::RAW_KEY_INPUT_EVENT
//! [`notify`]: crate::event::Event::notify
//! [`DeviceId`]: crate::app::DeviceId

use std::path::PathBuf;

use zero_ui_view_api::FrameWaitId;

use super::{
    raw_device_events::AxisId,
    view_process::{AnimationsConfig, MonitorInfo, ViewImage, WindowStateAll},
    DeviceId,
};
use crate::{
    color::ColorScheme,
    event::*,
    keyboard::{Key, KeyRepeatConfig, KeyState, ScanCode},
    mouse::{ButtonState, MouseButton, MouseScrollDelta, MultiClickConfig, TouchForce, TouchPhase},
    render::FrameId,
    text::FontAntiAliasing,
    units::{DipPoint, DipSize, Factor, PxRect},
    window::{EventCause, MonitorId, WindowId},
};

event_args! {
    /// Arguments for the [`RAW_KEY_INPUT_EVENT`].
    pub struct RawKeyInputArgs {
        /// Window that received the event.
        pub window_id: WindowId,

        /// Keyboard device that generated the event.
        pub device_id: DeviceId,

        /// Raw code of key.
        pub scan_code: ScanCode,

        /// If the key was pressed or released.
        pub state: KeyState,

        /// Symbolic name of [`scan_code`](Self::scan_code).
        pub key: Option<Key>,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all();
        }
    }

    /// Arguments for the [`RAW_CHAR_INPUT_EVENT`].
    pub struct RawCharInputArgs {
        /// Window that received the event.
        pub window_id: WindowId,

        /// Unicode character.
        pub character: char,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all();
        }
    }

    /// Arguments for the [`RAW_WINDOW_FOCUS_EVENT`].
    pub struct RawWindowFocusArgs {
        /// Window that load focus.
        pub prev_focus: Option<WindowId>,

        /// Window that got focus.
        pub new_focus: Option<WindowId>,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all();
        }
    }

    /// Arguments for the [`RAW_FRAME_RENDERED_EVENT`].
    pub struct RawFrameRenderedArgs {
        /// Window that presents the rendered frame.
        pub window_id: WindowId,

        /// Frame tag.
        pub frame_id: FrameId,

        /// The frame pixels if it was requested when the frame request was sent to the view process.
        pub frame_image: Option<ViewImage>,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all();
        }
    }

    /// Arguments for the [`RAW_WINDOW_CHANGED_EVENT`].
    pub struct RawWindowChangedArgs {
        /// Window that was moved, resized or has a state change.
        pub window_id: WindowId,

        /// New [`WindowStateAll`] if any part of it has changed.
        pub state: Option<WindowStateAll>,

        /// New window position if it was moved.
        pub position: Option<DipPoint>,

        /// New window monitor and its scale factor.
        ///
        /// The window's monitor change when it is moved enough so that most of the
        /// client area is in the new monitor screen.
        ///
        /// Note that the window's scale factor can also change by system settings, that change
        /// generates an [`RAW_SCALE_FACTOR_CHANGED_EVENT`] only.
        pub monitor: Option<(MonitorId, Factor)>,

        /// New window size if it was resized.
        pub size: Option<DipSize>,

        /// If the app or operating system caused the change.
        pub cause: EventCause,

        /// If the view-process is blocking the event loop for a time waiting for a frame for the new `size` this
        /// ID must be send with the frame to signal that it is the frame for the new size.
        ///
        /// Event loop implementations can use this to resize without visible artifacts
        /// like the clear color flashing on the window corners, there is a timeout to this delay but it
        /// can be a noticeable stutter, a [`render`] or [`render_update`] request for the window unblocks the loop early
        /// to continue the resize operation.
        ///
        /// [`render`]: crate::app::view_process::ViewRenderer::render
        /// [`render_update`]: crate::app::view_process::ViewRenderer::render_update
        pub frame_wait_id: Option<FrameWaitId>,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all();
        }
    }

    /// Arguments for the [`RAW_WINDOW_OPEN_EVENT`].
    pub struct RawWindowOpenArgs {
        /// Window that finished opening.
        pub window_id: WindowId,

        /// Live connection to the window in the view-process.
        pub window: super::view_process::ViewWindow,

        /// Extra data send by the view-process.
        pub data: super::view_process::WindowOpenData,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all();
        }
    }

    /// Arguments for the [`RAW_HEADLESS_OPEN_EVENT`].
    pub struct RawHeadlessOpenArgs {
        /// Window id that represents the headless surface that finished opening.
        pub window_id: WindowId,

        /// Live connection to the headless surface in the view-process.
        pub surface: super::view_process::ViewHeadless,

        /// Extra data send by the view-process.
        pub data: super::view_process::HeadlessOpenData,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all();
        }
    }

    /// Arguments for the [`RAW_WINDOW_OR_HEADLESS_OPEN_ERROR_EVENT`].
    pub struct RawWindowOrHeadlessOpenErrorArgs {
        /// Window id that failed to open.
        pub window_id: WindowId,
        /// Error message from the view-process.
        pub error: String,
        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all();
        }
    }

    /// Arguments for the [`RAW_WINDOW_CLOSE_REQUESTED_EVENT`].
    pub struct RawWindowCloseRequestedArgs {
        /// Window that was requested to close.
        pub window_id: WindowId,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all();
        }
    }

    /// Arguments for the [`RAW_WINDOW_CLOSE_EVENT`].
    pub struct RawWindowCloseArgs {
        /// Window that was destroyed.
        pub window_id: WindowId,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all();
        }
    }

    /// Arguments for the [`RAW_DROPPED_FILE_EVENT`].
    pub struct RawDroppedFileArgs {
        /// Window where it was dropped.
        pub window_id: WindowId,

        /// Path to file that was dropped.
        pub file: PathBuf,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for the [`RAW_HOVERED_FILE_EVENT`].
    pub struct RawHoveredFileArgs {
        /// Window where it was dragged over.
        pub window_id: WindowId,

        /// Path to file that was dragged over the window.
        pub file: PathBuf,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for the [`RAW_HOVERED_FILE_CANCELLED_EVENT`].
    ///
    /// The file is the one that was last [hovered] into the window.
    ///
    /// [hovered]: RAW_HOVERED_FILE_EVENT
    pub struct RawHoveredFileCancelledArgs {
        /// Window where the file was previously dragged over.
        pub window_id: WindowId,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for the [`RAW_CURSOR_MOVED_EVENT`].
    pub struct RawCursorMovedArgs {
        /// Window the cursor was moved over.
        pub window_id: WindowId,

        /// Device that generated this event.
        pub device_id: DeviceId,

        /// Positions of the cursor in between the previous event and this one.
        ///
        /// Cursor move events can be coalesced, i.e. multiple cursor moves packed into a single event.
        pub coalesced_pos: Vec<DipPoint>,

        /// Position of the cursor over the window, (0, 0) is the top-left.
        pub position: DipPoint,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for the [`RAW_CURSOR_ENTERED_EVENT`] and [`RAW_CURSOR_LEFT_EVENT`].
    pub struct RawCursorArgs {
        /// Window the cursor entered or left.
        pub window_id: WindowId,

        /// Device that generated this event.
        pub device_id: DeviceId,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for the [`RAW_MOUSE_WHEEL_EVENT`].
    pub struct RawMouseWheelArgs {
        /// Window that is hovered by the cursor.
        pub window_id: WindowId,

        /// Device that generated this event.
        pub device_id: DeviceId,

        /// Wheel motion delta, value is in pixels if the *wheel* is a touchpad.
        pub delta: MouseScrollDelta,

        /// Touch state if the device that generated the event is a touchpad.
        pub phase: TouchPhase,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for the [`RAW_MOUSE_INPUT_EVENT`].
    pub struct RawMouseInputArgs {
        /// Window that is hovered by the cursor.
        pub window_id: WindowId,

        /// Device that generated this event.
        pub device_id: DeviceId,

        /// If the button was pressed or released.
        pub state: ButtonState,

        /// What button was pressed or released.
        pub button: MouseButton,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all();
        }
    }

    /// Arguments for the [`RAW_TOUCHPAD_PRESSURE_EVENT`].
    pub struct RawTouchpadPressureArgs {
        /// Window that is touched.
        pub window_id: WindowId,

        /// Device that generated this event.
        pub device_id: DeviceId,

        /// Pressure level between 0 and 1.
        pub pressue: Factor,

        /// Click level.
        pub stage: i64,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
           list.search_all();
        }
    }

    /// Arguments for the [`RAW_AXIS_MOTION_EVENT`].
    pub struct RawAxisMotionArgs {
        /// Window that received the event.
        pub window_id: WindowId,

        /// Device that generated the event.
        pub device_id: DeviceId,

        /// Analog axis.
        pub axis: AxisId,

        /// Motion amount.
        pub value: f64,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all();
        }
    }

    /// Arguments for the [`RAW_TOUCH_EVENT`].
    pub struct RawTouchArgs {
        /// Window that was touched.
        pub window_id: WindowId,

        /// Device that generated this event.
        pub device_id: DeviceId,

        /// Touch phase.
        pub phase: TouchPhase,

        /// Touch center point.
        pub position: DipPoint,

        /// Touch force.
        pub force: Option<TouchForce>,

        /// Raw finger ID.
        pub finger_id: u64,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all();
        }
    }

    /// Arguments for the [`RAW_SCALE_FACTOR_CHANGED_EVENT`].
    pub struct RawScaleFactorChangedArgs {
        /// Monitor that has changed.
        pub monitor_id: MonitorId,

        /// Window in the monitor that has changed.
        pub windows: Vec<WindowId>,

        /// New pixel scale factor.
        pub scale_factor: Factor,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all();
        }
    }

    /// Arguments for the [`RAW_MONITORS_CHANGED_EVENT`].
    pub struct RawMonitorsChangedArgs {
        /// Up-to-date monitors list.
        pub available_monitors: Vec<(MonitorId, MonitorInfo)>,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all();
        }
    }

    /// Arguments for the [`RAW_COLOR_SCHEME_CHANGED_EVENT`].
    pub struct RawColorSchemeChangedArgs {
        /// Window for which the preference was changed.
        pub window_id: WindowId,

        /// New preference.
        pub color_scheme: ColorScheme,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
           list.search_all();
        }
    }

    /// Arguments for the image events.
    pub struct RawImageArgs {
        /// Image that changed.
        pub image: ViewImage,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for the [`RAW_FRAME_IMAGE_READY_EVENT`].
    pub struct RawFrameImageReadyArgs {
        /// Frame image that is ready.
        pub image: ViewImage,

        /// Window that was captured.
        pub window_id: WindowId,

        /// Frame that was captured.
        pub frame_id: FrameId,

        /// Area of the frame that was captured.
        pub area: PxRect,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// [`RAW_FONT_CHANGED_EVENT`] arguments.
    pub struct RawFontChangedArgs {
        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for the [`RAW_FONT_AA_CHANGED_EVENT`].
    pub struct RawFontAaChangedArgs {
        /// The new anti-aliasing config.
        pub aa: FontAntiAliasing,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for the [`RAW_MULTI_CLICK_CONFIG_CHANGED_EVENT`].
    pub struct RawMultiClickConfigChangedArgs {
        /// New config.
        pub config: MultiClickConfig,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for the [`RAW_ANIMATIONS_CONFIG_CHANGED_EVENT`].
    pub struct RawAnimationsConfigChangedArgs {
        /// New config.
        pub config: AnimationsConfig,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for the [`RAW_KEY_REPEAT_CONFIG_CHANGED_EVENT`].
    pub struct RawKeyRepeatConfigChangedArgs {
        /// New config.
        pub config: KeyRepeatConfig,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }
}

event! {
    /// A key press or release targeting a window.
    ///
    /// This event represents a key input directly from the operating system. It is processed
    /// by [`KeyboardManager`] to generate the [`KEY_INPUT_EVENT`] that actually targets the focused widget.
    ///
    /// *See also the [module level documentation](self) for details of how you can fake this event*
    ///
    /// [`KeyboardManager`]: crate::keyboard::KeyboardManager
    /// [`KEY_INPUT_EVENT`]: crate::keyboard::KEY_INPUT_EVENT
    pub static RAW_KEY_INPUT_EVENT: RawKeyInputArgs;

    /// A window received an Unicode character.
    pub static RAW_CHAR_INPUT_EVENT: RawCharInputArgs;

    /// A window received or lost focus.
    pub static RAW_WINDOW_FOCUS_EVENT: RawWindowFocusArgs;

    /// A window was moved, resized or has a state change.
    ///
    /// This event coalesces events usually named `WINDOW_MOVED`, `WINDOW_RESIZED` and `WINDOW_STATE_CHANGED` into a
    /// single event to simplify tracking composite changes, for example, the window changes size and position
    /// when maximized, this can be trivially observed with this event.
    pub static RAW_WINDOW_CHANGED_EVENT: RawWindowChangedArgs;

    /// A frame finished rendering and was presented in a window.
    pub static RAW_FRAME_RENDERED_EVENT: RawFrameRenderedArgs;

    /// A window has finished initializing in the view-process.
    pub static RAW_WINDOW_OPEN_EVENT: RawWindowOpenArgs;

    /// A headless surface has finished initializing in the view-process.
    pub static RAW_HEADLESS_OPEN_EVENT: RawHeadlessOpenArgs;

    /// A window or headless surface initialization failed in the view-process.
    pub static RAW_WINDOW_OR_HEADLESS_OPEN_ERROR_EVENT: RawWindowOrHeadlessOpenErrorArgs;

    /// A window was requested to close.
    pub static RAW_WINDOW_CLOSE_REQUESTED_EVENT: RawWindowCloseRequestedArgs;

    /// A window was destroyed.
    pub static RAW_WINDOW_CLOSE_EVENT: RawWindowCloseArgs;

    /// A file was drag-dropped on a window.
    pub static RAW_DROPPED_FILE_EVENT: RawDroppedFileArgs;

    /// A file was dragged over a window.
    ///
    /// If the file is dropped [`RAW_DROPPED_FILE_EVENT`] will raise.
    pub static RAW_HOVERED_FILE_EVENT: RawHoveredFileArgs;

    /// A dragging file was moved away from the window or the operation was cancelled.
    ///
    /// The file is the last one that emitted a [`RAW_HOVERED_FILE_EVENT`].
    pub static RAW_HOVERED_FILE_CANCELLED_EVENT: RawHoveredFileCancelledArgs;

    /// Cursor pointer moved over a window.
    pub static RAW_CURSOR_MOVED_EVENT: RawCursorMovedArgs;

    /// Cursor pointer started hovering a window.
    pub static RAW_CURSOR_ENTERED_EVENT: RawCursorArgs;

    /// Cursor pointer stopped hovering a window.
    pub static RAW_CURSOR_LEFT_EVENT: RawCursorArgs;

    /// Mouse wheel scrolled when the cursor was over a window.
    pub static RAW_MOUSE_WHEEL_EVENT: RawMouseWheelArgs;

    /// Mouse button was pressed or released when the cursor was over a window.
    pub static RAW_MOUSE_INPUT_EVENT: RawMouseInputArgs;

    /// Touchpad touched when the cursor was over a window.
    pub static RAW_TOUCHPAD_PRESSURE_EVENT: RawTouchpadPressureArgs;

    /// Motion on some analog axis send to a window.
    pub static RAW_AXIS_MOTION_EVENT: RawAxisMotionArgs;

    /// A window was touched.
    pub static RAW_TOUCH_EVENT: RawTouchArgs;

    /// Pixel scale factor for a monitor screen and its windows has changed.
    ///
    /// This can happen if the user change the screen settings. Note that a
    /// window's scale factor can also change if it is moved to a different monitor,
    /// this change can be monitored using [`RAW_WINDOW_CHANGED_EVENT`].
    pub static RAW_SCALE_FACTOR_CHANGED_EVENT: RawScaleFactorChangedArgs;

    /// Monitors added or removed.
    pub static RAW_MONITORS_CHANGED_EVENT: RawMonitorsChangedArgs;

    /// Color scheme preference changed for a window.
    pub static RAW_COLOR_SCHEME_CHANGED_EVENT: RawColorSchemeChangedArgs;

    /// Change in system font anti-aliasing config.
    pub static RAW_FONT_AA_CHANGED_EVENT: RawFontAaChangedArgs;

    /// Change in system text fonts, install or uninstall.
    pub static RAW_FONT_CHANGED_EVENT: RawFontChangedArgs;

    /// Change in system "double-click" config.
    pub static RAW_MULTI_CLICK_CONFIG_CHANGED_EVENT: RawMultiClickConfigChangedArgs;

    /// Change in system animation enabled config.
    pub static RAW_ANIMATIONS_CONFIG_CHANGED_EVENT: RawAnimationsConfigChangedArgs;

    /// Change in system key repeat interval config.
    pub static RAW_KEY_REPEAT_CONFIG_CHANGED_EVENT: RawKeyRepeatConfigChangedArgs;

    /// Image metadata loaded without errors.
    pub static RAW_IMAGE_METADATA_LOADED_EVENT: RawImageArgs;

    /// Progressively decoded image has decoded more pixels.
    pub static RAW_IMAGE_PARTIALLY_LOADED_EVENT: RawImageArgs;

    /// Image loaded without errors.
    pub static RAW_IMAGE_LOADED_EVENT: RawImageArgs;

    /// Image failed to load.
    pub static RAW_IMAGE_LOAD_ERROR_EVENT: RawImageArgs;

    /// Image generated from a frame is ready for reading.
    pub static RAW_FRAME_IMAGE_READY_EVENT: RawFrameImageReadyArgs;
}
