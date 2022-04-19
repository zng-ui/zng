//! Events directly from `winit` targeting the app windows.
//!
//! These events get processed by [app extensions] to generate the events used in widgets, for example
//! the [`KeyboardManager`] uses the [`RawKeyInputEvent`] into focus targeted events.
//!
//! # Synthetic Input
//!
//! You can [`notify`] these events to fake hardware input, please be careful that you mimic the exact sequence a real
//! hardware would generate, [app extensions] can assume that the raw events are correct. The [`DeviceId`] for fake
//! input must be unique but constant for each distinctive *synthetic event source*.
//!
//! [app extensions]: crate::app::AppExtension
//! [`KeyboardManager`]: crate::keyboard::KeyboardManager
//! [`RawKeyInputEvent`]: crate::app::raw_events::RawKeyInputEvent
//! [`notify`]: crate::event::Event::notify
//! [`DeviceId`]: crate::app::DeviceId

use std::{path::PathBuf, time::Duration};

use zero_ui_view_api::FrameWaitId;

use super::{
    raw_device_events::AxisId,
    view_process::{MonitorInfo, ViewImage, WindowStateAll},
    DeviceId,
};
use crate::{
    event::*,
    keyboard::{Key, KeyState, ModifiersState, ScanCode},
    mouse::{ButtonState, MouseButton, MouseScrollDelta, MultiClickConfig, TouchPhase},
    render::FrameId,
    text::TextAntiAliasing,
    units::{DipPoint, DipSize, Factor, PxPoint, PxRect},
    window::{EventCause, MonitorId, WindowId, WindowTheme},
};

event_args! {
    /// Arguments for the [`RawKeyInputEvent`].
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

        /// Returns `true` for all widgets in the [window](Self::window_id).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// Arguments for the [`RawModifiersChangedEvent`].
    pub struct RawModifiersChangedArgs {
        /// Window that received the event.
        pub window_id: WindowId,

        /// New modifiers state.
        pub modifiers: ModifiersState,

        ..

        /// Returns `true` for all widgets in the [window](Self::window_id).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// Arguments for the [`RawCharInputEvent`].
    pub struct RawCharInputArgs {
        /// Window that received the event.
        pub window_id: WindowId,

        /// Unicode character.
        pub character: char,

        ..

        /// Returns `true` for all widgets in the [window](Self::window_id).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// Arguments for the [`RawWindowFocusEvent`].
    pub struct RawWindowFocusArgs {
        /// Window that was focuses/blurred.
        pub window_id: WindowId,

        /// If the window received focus.
        pub focused: bool,

        ..

        /// Returns `true` for all widgets in the [window](Self::window_id).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// Arguments for the [`RawFrameRenderedEvent`].
    pub struct RawFrameRenderedArgs {
        /// Window that presents the rendered frame.
        pub window_id: WindowId,

        /// Frame tag.
        pub frame_id: FrameId,

        /// The frame pixels if it was requested when the frame request was sent to the view process.
        pub frame_image: Option<ViewImage>,

        /// Hit-test at the cursor position.
        pub cursor_hits: (PxPoint, crate::render::webrender_api::HitTestResult),

        ..

        /// Returns `true` for all widgets in the [window](Self::window_id).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// Arguments for the [`RawWindowChangedEvent`].
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
        /// generates an [`RawScaleFactorChangedEvent`] only.
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

        /// Returns `true` for all widgets in the [window](Self::window_id).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// Arguments for the [`RawWindowOpenEvent`].
    pub struct RawWindowOpenArgs {
        /// Window that finished opening.
        pub window_id: WindowId,

        /// Live connection to the window in the view-process.
        pub window: super::view_process::ViewWindow,

        /// Extra data send by the view-process.
        pub data: super::view_process::WindowOpenData,

        ..

        /// Returns `true` for all widgets in the [window](Self::window_id).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// Arguments for the [`RawHeadlessOpenEvent`].
    pub struct RawHeadlessOpenArgs {
        /// Window id that represents the headless surface that finished opening.
        pub window_id: WindowId,

        /// Live connection to the headless surface in the view-process.
        pub surface: super::view_process::ViewHeadless,

        /// Extra data send by the view-process.
        pub data: super::view_process::HeadlessOpenData,

        ..

        /// Returns `true` for all widgets in the [window](Self::window_id).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// Arguments for the [`RawWindowOrHeadlessOpenErrorEvent`].
    pub struct RawWindowOrHeadlessOpenErrorArgs {
        /// Window id that failed to open.
        pub window_id: WindowId,
        /// Error message from the view-process.
        pub error: String,
        ..
        /// Returns `true` for all widgets in the [window](Self::window_id).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// Arguments for the [`RawWindowCloseRequestedEvent`].
    pub struct RawWindowCloseRequestedArgs {
        /// Window that was requested to close.
        pub window_id: WindowId,

        ..

        /// Returns `true` for all widgets in the [window](Self::window_id).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// Arguments for the [`RawWindowCloseEvent`].
    pub struct RawWindowCloseArgs {
        /// Window that was destroyed.
        pub window_id: WindowId,

        ..

        /// Returns `true` for all widgets.
        fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
            true
        }
    }

    /// Arguments for the [`RawDroppedFileEvent`].
    pub struct RawDroppedFileArgs {
        /// Window where it was dropped.
        pub window_id: WindowId,

        /// Path to file that was dropped.
        pub file: PathBuf,

        ..

        /// Returns `true` for all widgets in the [window](Self::window_id).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// Arguments for the [`RawHoveredFileEvent`].
    pub struct RawHoveredFileArgs {
        /// Window where it was dragged over.
        pub window_id: WindowId,

        /// Path to file that was dragged over the window.
        pub file: PathBuf,

        ..

        /// Returns `true` for all widgets in the [window](Self::window_id).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// Arguments for the [`RawHoveredFileCancelledEvent`].
    ///
    /// The file is the one that was last [hovered] into the window.
    ///
    /// [hovered]: RawHoveredFileEvent
    pub struct RawHoveredFileCancelledArgs {
        /// Window where the file was previously dragged over.
        pub window_id: WindowId,

        ..

        /// Returns `true` for all widgets in the [window](Self::window_id).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// Arguments for the [`RawCursorMovedEvent`].
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

        /// Hit-test results for `position`.
        pub hits: (FrameId, PxPoint, crate::render::webrender_api::HitTestResult),

        ..

        /// Returns `true` for all widgets in the [window](Self::window_id).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// Arguments for the [`RawCursorEnteredEvent`] and [`RawCursorLeftEvent`].
    pub struct RawCursorArgs {
        /// Window the cursor entered or left.
        pub window_id: WindowId,

        /// Device that generated this event.
        pub device_id: DeviceId,

        ..

        /// Returns `true` for all widgets in the [window](Self::window_id).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// Arguments for the [`RawMouseWheelEvent`].
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

        /// Returns `true` for all widgets in the [window](Self::window_id).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// Arguments for the [`RawMouseInputEvent`].
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

        /// Returns `true` for all widgets in the [window](Self::window_id).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// Arguments for the [`RawTouchpadPressureEvent`].
    pub struct RawTouchpadPressureArgs {
        /// Window that is touched.
        pub window_id: WindowId,

        /// Device that generated this event.
        pub device_id: DeviceId,

        // TODO

        ..

        /// Returns `true` for all widgets in the [window](Self::window_id).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// Arguments for the [`RawAxisMotionEvent`].
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

        /// Returns `true` for all widgets in the [window](Self::window_id).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// Arguments for the [`RawTouchEvent`].
    pub struct RawTouchArgs {
        /// Window that was touched.
        pub window_id: WindowId,

        /// Device that generated this event.
        pub device_id: DeviceId,

        // TODO

        ..

        /// Returns `true` for all widgets in the [window](Self::window_id).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// Arguments for the [`RawScaleFactorChangedEvent`].
    pub struct RawScaleFactorChangedArgs {
        /// Monitor that has changed.
        pub monitor_id: MonitorId,

        /// Window in the monitor that has changed.
        pub windows: Vec<WindowId>,

        /// New pixel scale factor.
        pub scale_factor: Factor,

        ..

        /// Returns `true` for all widgets in any of the [`windows`](Self::windows).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            let window_id = ctx.path.window_id();
            self.windows.iter().any(|&w| w == window_id)
        }
    }

    /// Arguments for the [`RawMonitorsChangedEvent`].
    pub struct RawMonitorsChangedArgs {
        /// Up-to-date monitors list.
        pub available_monitors: Vec<(MonitorId, MonitorInfo)>,

        ..

        /// Concerns all widgets.
        fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
            true
        }
    }

    /// Arguments for the [`RawWindowThemeChangedEvent`].
    pub struct RawWindowThemeChangedArgs {
        /// Window for which the theme was changed.
        pub window_id: WindowId,

        /// New theme.
        pub theme: WindowTheme,

        ..

        /// Returns `true` for all widgets in the [window](Self::window_id).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// Arguments for the image events.
    pub struct RawImageArgs {
        /// Image that changed.
        pub image: ViewImage,

        ..

        /// Concerns all widgets.
        fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
            true
        }
    }

    /// Arguments for the [`RawFrameImageReadyEvent`].
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

        /// Concerns all widgets.
        fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
            true
        }
    }

    /// [`RawFontChangedEvent`] arguments.
    pub struct RawFontChangedArgs {
        ..

        /// Concerns all widgets.
        fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
            true
        }
    }

    /// Arguments for the [`RawTextAaChangedEvent`].
    pub struct RawTextAaChangedArgs {
        /// The new anti-aliasing config.
        pub aa: TextAntiAliasing,

        ..

        /// Concerns all widgets.
        fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
            true
        }
    }

    /// Arguments for the [`RawMultiClickConfigChangedEvent`].
    pub struct RawMultiClickConfigChangedArgs {
        /// New config.
        pub config: MultiClickConfig,

        ..

        /// Concerns all widgets.
        fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
            true
        }
    }

    /// Arguments for the [`RawAnimationsEnabledChangedEvent`].
    pub struct RawAnimationsEnabledChangedArgs {
        /// If animation is enabled in the operating system.
        pub enabled: bool,

        ..

        /// Concerns all widgets.
        fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
            true
        }
    }

    /// Arguments for the [`RawKeyRepeatDelayChangedEvent`].
    pub struct RawKeyRepeatDelayChangedArgs {
        /// New delay.
        ///
        /// When the user holds a key pressed the system will generate a new key-press event
        /// every time this delay elapses. The real delay time depends on the hardware but it
        /// roughly matches this value.
        pub delay: Duration,

        ..

        /// Concerns all widgets.
        fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
            true
        }
    }
}

event! {
    /// A key press or release targeting a window.
    ///
    /// This event represents a key input directly from the operating system. It is processed
    /// by [`KeyboardManager`] to generate the [`KeyInputEvent`] that actually targets the focused widget.
    ///
    /// *See also the [module level documentation](self) for details of how you can fake this event*
    ///
    /// [`KeyboardManager`]: crate::keyboard::KeyboardManager
    /// [`KeyInputEvent`]: crate::keyboard::KeyInputEvent
    pub RawKeyInputEvent: RawKeyInputArgs;

    /// A modifier key press or release updated the state of the modifier keys.
    ///
    /// This event represents a key input directly from the operating system. It is processed
    /// by [`KeyboardManager`] to generate the keyboard events that are used in general.
    ///
    /// *See also the [module level documentation](self) for details of how you can fake this event*
    ///
    /// [`KeyboardManager`]: crate::keyboard::KeyboardManager
    pub RawModifiersChangedEvent: RawModifiersChangedArgs;

    /// A window received an Unicode character.
    pub RawCharInputEvent: RawCharInputArgs;

    /// A window received or lost focus.
    pub RawWindowFocusEvent: RawWindowFocusArgs;

    /// A window was moved, resized or has a state change.
    ///
    /// This event coalesces events usually named `WindowMoved`, `WindowResized` and `WindowStateChanged` into a
    /// single event to simplify tracking composite changes, for example, the window changes size and position
    /// when maximized, this can be trivially observed with this event.
    pub RawWindowChangedEvent: RawWindowChangedArgs;

    /// A frame finished rendering and was presented in a window.
    pub RawFrameRenderedEvent: RawFrameRenderedArgs;

    /// A window has finished initializing in the view-process.
    pub RawWindowOpenEvent: RawWindowOpenArgs;

    /// A headless surface has finished initializing in the view-process.
    pub RawHeadlessOpenEvent: RawHeadlessOpenArgs;

    /// A window or headless surface initialization failed in the view-process.
    pub RawWindowOrHeadlessOpenErrorEvent: RawWindowOrHeadlessOpenErrorArgs;

    /// A window was requested to close.
    pub RawWindowCloseRequestedEvent: RawWindowCloseRequestedArgs;

    /// A window was destroyed.
    pub RawWindowCloseEvent: RawWindowCloseArgs;

    /// A file was drag-dropped on a window.
    pub RawDroppedFileEvent: RawDroppedFileArgs;

    /// A file was dragged over a window.
    ///
    /// If the file is dropped [`RawDroppedFileEvent`] will raise.
    pub RawHoveredFileEvent: RawHoveredFileArgs;

    /// A dragging file was moved away from the window or the operation was cancelled.
    ///
    /// The file is the last one that emitted a [`RawHoveredFileEvent`].
    pub RawHoveredFileCancelledEvent: RawHoveredFileCancelledArgs;

    /// Cursor pointer moved over a window.
    pub RawCursorMovedEvent: RawCursorMovedArgs;

    /// Cursor pointer started hovering a window.
    pub RawCursorEnteredEvent: RawCursorArgs;

    /// Cursor pointer stopped hovering a window.
    pub RawCursorLeftEvent: RawCursorArgs;

    /// Mouse wheel scrolled when the cursor was over a window.
    pub RawMouseWheelEvent: RawMouseWheelArgs;

    /// Mouse button was pressed or released when the cursor was over a window.
    pub RawMouseInputEvent: RawMouseInputArgs;

    /// Touchpad touched when the cursor was over a window.
    pub RawTouchpadPressureEvent: RawTouchpadPressureArgs;

    /// Motion on some analog axis send to a window.
    pub RawAxisMotionEvent: RawAxisMotionArgs;

    /// A window was touched.
    pub RawTouchEvent: RawTouchArgs;

    /// Pixel scale factor for a monitor screen and its windows has changed.
    ///
    /// This can happen if the user change the screen settings. Note that a
    /// window's scale factor can also change if it is moved to a different monitor,
    /// this change can be monitored using [`RawWindowChangedEvent`].
    pub RawScaleFactorChangedEvent: RawScaleFactorChangedArgs;

    /// Monitors added or removed.
    pub RawMonitorsChangedEvent: RawMonitorsChangedArgs;

    /// System theme changed for a window.
    pub RawWindowThemeChangedEvent: RawWindowThemeChangedArgs;

    /// Change in system text anti-aliasing config.
    pub RawTextAaChangedEvent: RawTextAaChangedArgs;

    /// Change in system text fonts, install or uninstall.
    pub RawFontChangedEvent: RawFontChangedArgs;

    /// Change in system "double-click" config.
    pub RawMultiClickConfigChangedEvent: RawMultiClickConfigChangedArgs;

    /// Change in system animation enabled config.
    pub RawAnimationsEnabledChangedEvent: RawAnimationsEnabledChangedArgs;

    /// Change in system key repeat interval config.
    pub RawKeyRepeatDelayChangedEvent: RawKeyRepeatDelayChangedArgs;

    /// Image metadata loaded without errors.
    pub RawImageMetadataLoadedEvent: RawImageArgs;

    /// Progressively decoded image has decoded more pixels.
    pub RawImagePartiallyLoadedEvent: RawImageArgs;

    /// Image loaded without errors.
    pub RawImageLoadedEvent: RawImageArgs;

    /// Image failed to load.
    pub RawImageLoadErrorEvent: RawImageArgs;

    /// Image generated from a frame is ready for reading.
    pub RawFrameImageReadyEvent: RawFrameImageReadyArgs;
}
