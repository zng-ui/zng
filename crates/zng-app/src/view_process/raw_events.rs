//! Events directly from the view-process targeting the app windows.
//!
//! These events get processed by [app extensions] to generate the events used in widgets, for example
//! the `KeyboardManager` uses the [`RAW_KEY_INPUT_EVENT`] into focus targeted events.
//!
//! # Synthetic Input
//!
//! You can [`notify`] these events to fake hardware input, please be careful that you mimic the exact sequence a real
//! hardware would generate, [app extensions] can assume that the raw events are correct. The [`InputDeviceId`] for fake
//! input must be unique but constant for each distinctive *synthetic event source*.
//!
//! [app extensions]: crate::AppExtension
//! [`RAW_KEY_INPUT_EVENT`]: crate::view_process::raw_events::RAW_KEY_INPUT_EVENT
//! [`notify`]: crate::event::Event::notify
//! [`InputDeviceId`]: crate::view_process::raw_device_events::InputDeviceId

use zng_layout::unit::{DipPoint, DipSideOffsets, DipSize, Factor, PxPoint, PxRect};
use zng_txt::Txt;
use zng_view_api::{
    AxisId, DragDropId, Ime,
    api_extension::{ApiExtensionId, ApiExtensionPayload},
    config::{
        AnimationsConfig, ChromeConfig, ColorsConfig, FontAntiAliasing, KeyRepeatConfig, LocaleConfig, MultiClickConfig, TouchConfig,
    },
    drag_drop::{DragDropData, DragDropEffect},
    keyboard::{Key, KeyCode, KeyLocation, KeyState},
    mouse::{ButtonState, MouseButton, MouseScrollDelta},
    touch::{TouchPhase, TouchUpdate},
    window::{EventCause, FrameId, FrameWaitId, HeadlessOpenData, MonitorInfo, WindowStateAll},
};

use crate::{
    event::{event, event_args},
    window::{MonitorId, WindowId},
};

use super::{ViewHeadless, ViewImage, ViewWindow, WindowOpenData, raw_device_events::InputDeviceId};

event_args! {
    /// Arguments for the [`RAW_KEY_INPUT_EVENT`].
    pub struct RawKeyInputArgs {
        /// Window that received the event.
        pub window_id: WindowId,

        /// Keyboard device that generated the event.
        pub device_id: InputDeviceId,

        /// Physical key.
        pub key_code: KeyCode,

        /// The location of the key on the keyboard.
        pub key_location: KeyLocation,

        /// If the key was pressed or released.
        pub state: KeyState,

        /// Semantic key.
        ///
        /// Pressing `Shift+A` key will produce `Key::Char('a')` in QWERTY keyboards, the modifiers are not applied.
        pub key: Key,
        /// Semantic key modified by the current active modifiers.
        ///
        /// Pressing `Shift+A` key will produce `Key::Char('A')` in QWERTY keyboards, the modifiers are applied.
        pub key_modified: Key,

        /// Text typed.
        ///
        /// This is only set for `KeyState::Pressed` of a key that generates text.
        ///
        /// This is usually the `key_modified` char, but is also `'\r'` for `Key::Enter`. On Windows when a dead key was
        /// pressed earlier but cannot be combined with the character from this key press, the produced text
        /// will consist of two characters: the dead-key-character followed by the character resulting from this key press.
        pub text: Txt,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all();
        }
    }

    /// Arguments for the [`RAW_IME_EVENT`].
    pub struct RawImeArgs {
        /// Window that received the event.
        pub window_id: WindowId,

        /// The IME event.
        pub ime: Ime,

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
        /// Window that has moved, resized or has a state change.
        pub window_id: WindowId,

        /// New state if any part of it has changed.
        pub state: Option<WindowStateAll>,

        /// New window position if it was moved.
        ///
        /// The values are `(global_position, position_in_monitor)`.
        pub position: Option<(PxPoint, DipPoint)>,

        /// New window monitor.
        ///
        /// The window's monitor change when it is moved enough so that most of the
        /// client area is in the new monitor screen.
        pub monitor: Option<MonitorId>,

        /// New window size if it was resized.
        pub size: Option<DipSize>,

        /// New window safe padding.
        pub safe_padding: Option<DipSideOffsets>,

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
        /// [`render`]: crate::view_process::ViewRenderer::render
        /// [`render_update`]: crate::view_process::ViewRenderer::render_update
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
        pub window: ViewWindow,

        /// Extra data send by the view-process.
        pub data: WindowOpenData,

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
        pub surface: ViewHeadless,

        /// Extra data send by the view-process.
        pub data: HeadlessOpenData,

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
        pub error: Txt,

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
        /// Window that has closed.
        pub window_id: WindowId,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all();
        }
    }

    /// Arguments for the [`RAW_DRAG_HOVERED_EVENT`].
    pub struct RawDragHoveredArgs {
        /// Window where it was dragged over.
        pub window_id: WindowId,

        /// Data payload.
        pub data: Vec<DragDropData>,
        /// Allowed effects.
        pub allowed: DragDropEffect,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for the [`RAW_DRAG_MOVED_EVENT`].
    pub struct RawDragMovedArgs {
        /// Window that is hovered by drag&drop.
        pub window_id: WindowId,

        /// Cursor positions in between the previous event and this one.
        ///
        /// Drag move events can be coalesced, i.e. multiple moves packed into a single event.
        pub coalesced_pos: Vec<DipPoint>,

        /// Position of the cursor over the window, (0, 0) is the top-left.
        pub position: DipPoint,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for the [`RAW_DRAG_DROPPED_EVENT`].
    pub struct RawDragDroppedArgs {
        /// Window where it was dropped.
        pub window_id: WindowId,

        /// Data payload.
        pub data: Vec<DragDropData>,
        /// Allowed effects.
        pub allowed: DragDropEffect,
        /// ID of this drop operation.
        ///
        /// Handlers must call `drag_dropped` with this ID and what effect was applied to the data.
        pub drop_id: DragDropId,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for the [`RAW_DRAG_CANCELLED_EVENT`].
    pub struct RawDragCancelledArgs {
        /// Window where the file was previously dragged over.
        pub window_id: WindowId,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for the [`RAW_APP_DRAG_ENDED_EVENT`].
    pub struct RawAppDragEndedArgs {
        /// Window that started the drag operation.
        pub window_id: WindowId,

        /// ID of the drag & drop operation.
        pub id: DragDropId,

        /// Effect applied to the data by the drop target.
        ///
        /// Is a single flag if the data was dropped in a valid drop target, or is empty if was canceled.
        pub applied: DragDropEffect,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for the [`RAW_MOUSE_MOVED_EVENT`].
    pub struct RawMouseMovedArgs {
        /// Window the mouse was moved over.
        pub window_id: WindowId,

        /// Device that generated this event.
        pub device_id: InputDeviceId,

        /// Positions of the mouse in between the previous event and this one.
        ///
        /// Mouse move events can be coalesced, i.e. multiple mouse moves packed into a single event.
        pub coalesced_pos: Vec<DipPoint>,

        /// Position of the mouse over the window, (0, 0) is the top-left.
        pub position: DipPoint,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for the [`RAW_MOUSE_ENTERED_EVENT`] and [`RAW_MOUSE_LEFT_EVENT`].
    pub struct RawMouseArgs {
        /// Window the mouse entered or left.
        pub window_id: WindowId,

        /// Device that generated this event.
        pub device_id: InputDeviceId,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for the [`RAW_MOUSE_WHEEL_EVENT`].
    pub struct RawMouseWheelArgs {
        /// Window that is hovered by the mouse.
        pub window_id: WindowId,

        /// Device that generated this event.
        pub device_id: InputDeviceId,

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
        /// Window that is hovered by the mouse.
        pub window_id: WindowId,

        /// Device that generated this event.
        pub device_id: InputDeviceId,

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
        pub device_id: InputDeviceId,

        /// Pressure level between 0 and 1.
        pub pressure: Factor,

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
        pub device_id: InputDeviceId,

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
        pub device_id: InputDeviceId,

        /// Coalesced touch updates.
        pub touches: Vec<TouchUpdate>,

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

    /// Arguments for the [`RAW_TOUCH_CONFIG_CHANGED_EVENT`].
    pub struct RawTouchConfigChangedArgs {
        /// New config.
        pub config: TouchConfig,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for the [`RAW_LOCALE_CONFIG_CHANGED_EVENT`].
    pub struct RawLocaleChangedArgs {
        /// New config.
        pub config: LocaleConfig,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for the [`RAW_COLORS_CONFIG_CHANGED_EVENT`].
    pub struct RawColorsConfigChangedArgs {
        /// New config.
        pub config: ColorsConfig,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all();
        }
    }

    /// Arguments for the [`RAW_CHROME_CONFIG_CHANGED_EVENT`].
    pub struct RawChromeConfigChangedArgs {
        /// New config.
        pub config: ChromeConfig,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all();
        }
    }

    /// Arguments for the [`RAW_EXTENSION_EVENT`].
    pub struct RawExtensionEventArgs {
        /// Id of the sender extension.
        pub extension_id: ApiExtensionId,
        /// Event payload.
        pub payload: ApiExtensionPayload,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// Arguments for [`LOW_MEMORY_EVENT`].
    pub struct LowMemoryArgs {

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
    /// by `KeyboardManager` to generate the `KEY_INPUT_EVENT` that actually targets the focused widget.
    ///
    /// *See also the [module level documentation](self) for details of how you can fake this event*
    pub static RAW_KEY_INPUT_EVENT: RawKeyInputArgs;

    /// An IME event was received by a window.
    pub static RAW_IME_EVENT: RawImeArgs;

    /// A window received or lost focus.
    pub static RAW_WINDOW_FOCUS_EVENT: RawWindowFocusArgs;

    /// A window was moved, resized or has a state change.
    ///
    /// This event aggregates events moves, resizes and other state changes into a
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

    /// Data was dragged over a window.
    pub static RAW_DRAG_HOVERED_EVENT: RawDragHoveredArgs;

    /// Data dragging over the window has moved.
    pub static RAW_DRAG_MOVED_EVENT: RawDragMovedArgs;

    /// Data was drag-dropped on a window.
    pub static RAW_DRAG_DROPPED_EVENT: RawDragDroppedArgs;

    /// Data was dragged away from the window or the operation was cancelled.
    pub static RAW_DRAG_CANCELLED_EVENT: RawDragCancelledArgs;

    /// Drag & drop operation started by the app has dropped or was cancelled.
    pub static RAW_APP_DRAG_ENDED_EVENT: RawAppDragEndedArgs;

    /// Mouse pointer moved over a window.
    pub static RAW_MOUSE_MOVED_EVENT: RawMouseMovedArgs;

    /// Mouse pointer started hovering a window.
    pub static RAW_MOUSE_ENTERED_EVENT: RawMouseArgs;

    /// Mouse pointer stopped hovering a window.
    pub static RAW_MOUSE_LEFT_EVENT: RawMouseArgs;

    /// Mouse wheel scrolled when the mouse was over a window.
    pub static RAW_MOUSE_WHEEL_EVENT: RawMouseWheelArgs;

    /// Mouse button was pressed or released when the mouse was over a window.
    pub static RAW_MOUSE_INPUT_EVENT: RawMouseInputArgs;

    /// Touchpad touched when the mouse was over a window.
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

    /// Monitors added, removed or modified.
    pub static RAW_MONITORS_CHANGED_EVENT: RawMonitorsChangedArgs;

    /// Color scheme or accent color preference changed for a window.
    pub static RAW_COLORS_CONFIG_CHANGED_EVENT: RawColorsConfigChangedArgs;

    /// System window chrome config changed.
    pub static RAW_CHROME_CONFIG_CHANGED_EVENT: RawChromeConfigChangedArgs;

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

    /// Change in system touch config.
    pub static RAW_TOUCH_CONFIG_CHANGED_EVENT: RawTouchConfigChangedArgs;

    /// Change in system locale config.
    pub static RAW_LOCALE_CONFIG_CHANGED_EVENT: RawLocaleChangedArgs;

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

    /// System low memory warning, some platforms may kill the app if it does not release memory.
    pub static LOW_MEMORY_EVENT: LowMemoryArgs;

    /// Custom view-process extension event.
    pub static RAW_EXTENSION_EVENT: RawExtensionEventArgs;
}
