use std::{
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

use zero_ui_app::{
    event::{event, event_args},
    widget::{
        node::{BoxedUiNode, UiNode},
        WidgetId,
    },
    window::{WindowId, WINDOW},
};
use zero_ui_ext_image::{ImageSource, ImageVar, Img};
use zero_ui_layout::unit::{DipPoint, DipSize, Point, PxPoint};
use zero_ui_txt::Txt;
use zero_ui_unique_id::IdSet;
use zero_ui_var::impl_from_and_into_var;
use zero_ui_view_api::{
    image::{ImageDataFormat, ImageMaskMode},
    window::{EventCause, FrameId},
};

pub use zero_ui_view_api::window::{FocusIndicator, RenderMode, VideoMode, WindowState};

use crate::{HeadlessMonitor, WINDOW_Ext as _};

/// Window root node and values.
///
/// The `Window!` widget instantiates this type.
///
/// This struct contains the window config that does not change, other window config is available in [`WINDOW.vars()`].
///
/// [`WINDOW.vars()`]: crate::WINDOW_Ext::vars
pub struct WindowRoot {
    pub(super) id: WidgetId,
    pub(super) start_position: StartPosition,
    pub(super) kiosk: bool,
    pub(super) transparent: bool,
    pub(super) render_mode: Option<RenderMode>,
    pub(super) headless_monitor: HeadlessMonitor,
    pub(super) start_focused: bool,
    pub(super) child: BoxedUiNode,
}
impl WindowRoot {
    /// New window from a `root` node that forms the window root widget.
    ///
    /// * `root_id` - Widget ID of `root`.
    /// * `start_position` - Position of the window when it first opens.
    /// * `kiosk` - Only allow full-screen mode. Note this does not configure the windows manager, only blocks the app itself
    ///             from accidentally exiting full-screen. Also causes subsequent open windows to be child of this window.
    /// * `transparent` - If the window should be created in a compositor mode that renders semi-transparent pixels as "see-through".
    /// * `render_mode` - Render mode preference overwrite for this window, note that the actual render mode selected can be different.
    /// * `headless_monitor` - "Monitor" configuration used in [headless mode](zero_ui_app::window::WindowMode::is_headless).
    /// * `start_focused` - If the window is forced to be the foreground keyboard focus after opening.
    /// * `root` - The root widget's outermost `CONTEXT` node, the window uses this and the `root_id` to form the root widget.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        root_id: WidgetId,
        start_position: StartPosition,
        kiosk: bool,
        transparent: bool,
        render_mode: Option<RenderMode>,
        headless_monitor: HeadlessMonitor,
        start_focused: bool,
        root: impl UiNode,
    ) -> Self {
        WindowRoot {
            id: root_id,
            start_position,
            kiosk,
            transparent,
            render_mode,
            headless_monitor,
            start_focused,
            child: root.boxed(),
        }
    }

    /// New window from a `child` node that becomes the child of the window root widget.
    ///
    /// The `child` parameter is a node that is the window's content, if it is a full widget the `root_id` is the id of
    /// an internal container widget that is the parent of `child`, if it is not a widget it will still be placed in the inner
    /// nest group of the root widget.
    ///
    /// See [`new`] for other parameters.
    ///
    /// [`new`]: Self::new
    #[allow(clippy::too_many_arguments)]
    pub fn new_container(
        root_id: WidgetId,
        start_position: StartPosition,
        kiosk: bool,
        transparent: bool,
        render_mode: Option<RenderMode>,
        headless_monitor: HeadlessMonitor,
        start_focused: bool,
        child: impl UiNode,
    ) -> Self {
        WindowRoot::new(
            root_id,
            start_position,
            kiosk,
            transparent,
            render_mode,
            headless_monitor,
            start_focused,
            zero_ui_app::widget::base::node::widget_inner(child),
        )
    }

    /// New test window.
    #[cfg(any(test, doc, feature = "test_util"))]
    pub fn new_test(child: impl UiNode) -> Self {
        WindowRoot::new_container(
            WidgetId::named("test-window-root"),
            StartPosition::Default,
            false,
            false,
            None,
            HeadlessMonitor::default(),
            false,
            child,
        )
    }
}

bitflags! {
    /// Window auto-size config.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
    #[serde(transparent)]
    pub struct AutoSize: u8 {
        /// Does not automatically adjust size.
        const DISABLED = 0;
        /// Uses the content desired width.
        const CONTENT_WIDTH = 0b01;
        /// Uses the content desired height.
        const CONTENT_HEIGHT = 0b10;
        /// Uses the content desired width and height.
        const CONTENT = Self::CONTENT_WIDTH.bits() | Self::CONTENT_HEIGHT.bits();
    }
}
impl_from_and_into_var! {
    /// Returns [`AutoSize::CONTENT`] if `content` is `true`, otherwise
    // returns [`AutoSize::DISABLED`].
    fn from(content: bool) -> AutoSize {
        if content {
            AutoSize::CONTENT
        } else {
            AutoSize::DISABLED
        }
    }
}

/// Window startup position.
///
/// The startup position affects the window once, at the moment the window
/// is open just after the first [`UiNode::layout`] call.
#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StartPosition {
    /// Resolves to [`position`](crate::WindowVars::position).
    Default,

    /// Centralizes the window in the monitor screen, defined by the [`monitor`](crate::WindowVars::monitor).
    ///
    /// Uses the `headless_monitor` in headless windows and falls back to not centering if no
    /// monitor was found in headed windows.
    CenterMonitor,
    /// Centralizes the window in the parent window, defined by the [`parent`](crate::WindowVars::parent).
    ///
    /// Falls back to center on the monitor if the window has no parent.
    CenterParent,
}
impl Default for StartPosition {
    fn default() -> Self {
        Self::Default
    }
}
impl fmt::Debug for StartPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "StartPosition::")?;
        }
        match self {
            StartPosition::Default => write!(f, "Default"),
            StartPosition::CenterMonitor => write!(f, "CenterMonitor"),
            StartPosition::CenterParent => write!(f, "CenterParent"),
        }
    }
}

bitflags! {
    /// Mask of allowed [`WindowState`] states of a window.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
    #[serde(transparent)]
    pub struct WindowStateAllowed: u8 {
        /// Enable minimize.
        const MINIMIZE = 0b0001;
        /// Enable maximize.
        const MAXIMIZE = 0b0010;
        /// Enable full-screen, but only windowed not exclusive video.
        const FULLSCREEN_WN_ONLY = 0b0100;
        /// Allow full-screen windowed or exclusive video.
        const FULLSCREEN = 0b1100;
    }
}

/// Window icon.
#[derive(Clone, PartialEq)]
pub enum WindowIcon {
    /// The operating system's default icon.
    Default,
    /// Image is requested from [`IMAGES`].
    ///
    /// [`IMAGES`]: zero_ui_ext_image::IMAGES
    Image(ImageSource),
}
impl fmt::Debug for WindowIcon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "WindowIcon::")?;
        }
        match self {
            WindowIcon::Default => write!(f, "Default"),
            WindowIcon::Image(r) => write!(f, "Image({r:?})"),
        }
    }
}
impl Default for WindowIcon {
    /// [`WindowIcon::Default`]
    fn default() -> Self {
        Self::Default
    }
}
impl WindowIcon {
    /// New window icon from a closure that generates a new icon [`UiNode`] for the window.
    ///
    /// The closure is called once on init and every time the window icon property changes,
    /// the closure runs in a headless window context, it must return a node to be rendered as an icon.
    ///
    /// The icon node is deinited and dropped after the first render, you can enable [`image::render_retain`] on it
    /// to cause the icon to continue rendering on updates.
    ///
    /// [`image::render_retain`]: fn@zero_ui_ext_image::render_retain
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use zero_ui_ext_window::WindowIcon;
    /// # macro_rules! Container { ($($tt:tt)*) => { zero_ui_app::widget::node::FillUiNode } }
    /// # let _ =
    /// WindowIcon::render(
    ///     || Container! {
    ///         // image::render_retain = true;
    ///         size = (36, 36);
    ///         background_gradient = Line::to_bottom_right(), stops![colors::MIDNIGHT_BLUE, 70.pct(), colors::CRIMSON];
    ///         corner_radius = 6;
    ///         font_size = 28;
    ///         font_weight = FontWeight::BOLD;
    ///         child = Text!("A");
    ///     }
    /// )
    /// # ;
    /// ```
    pub fn render<I, F>(new_icon: F) -> Self
    where
        I: UiNode,
        F: Fn() -> I + Send + Sync + 'static,
    {
        Self::Image(ImageSource::render_node(RenderMode::Software, move |args| {
            let node = new_icon();
            WINDOW.vars().parent().set(args.parent);
            node
        }))
    }
}
#[cfg(http)]
impl_from_and_into_var! {
    fn from(uri: crate::task::http::Uri) -> WindowIcon {
        ImageSource::from(uri).into()
    }
}
impl_from_and_into_var! {
    fn from(source: ImageSource) -> WindowIcon {
        WindowIcon::Image(source)
    }
    fn from(image: ImageVar) -> WindowIcon {
        ImageSource::Image(image).into()
    }
    fn from(path: PathBuf) -> WindowIcon {
        ImageSource::from(path).into()
    }
    fn from(path: &Path) -> WindowIcon {
        ImageSource::from(path).into()
    }
    /// See [`ImageSource`] conversion from `&str`
    fn from(s: &str) -> WindowIcon {
        ImageSource::from(s).into()
    }
    /// Same as conversion from `&str`.
    fn from(s: String) -> WindowIcon {
        ImageSource::from(s).into()
    }
    /// Same as conversion from `&str`.
    fn from(s: Txt) -> WindowIcon {
        ImageSource::from(s).into()
    }
    /// From encoded data of [`Unknown`] format.
    ///
    /// [`Unknown`]: ImageDataFormat::Unknown
    fn from(data: &'static [u8]) -> WindowIcon {
        ImageSource::from(data).into()
    }
    /// From encoded data of [`Unknown`] format.
    ///
    /// [`Unknown`]: ImageDataFormat::Unknown
    fn from<const N: usize>(data: &'static [u8; N]) -> WindowIcon {
        ImageSource::from(data).into()
    }
    /// From encoded data of [`Unknown`] format.
    ///
    /// [`Unknown`]: ImageDataFormat::Unknown
    fn from(data: Arc<Vec<u8>>) -> WindowIcon {
        ImageSource::from(data).into()
    }
    /// From encoded data of [`Unknown`] format.
    ///
    /// [`Unknown`]: ImageDataFormat::Unknown
    fn from(data: Vec<u8>) -> WindowIcon {
        ImageSource::from(data).into()
    }
    /// From encoded data of known format.
    fn from<F: Into<ImageDataFormat>>((data, format): (&'static [u8], F)) -> WindowIcon {
        ImageSource::from((data, format)).into()
    }
    /// From encoded data of known format.
    fn from<F: Into<ImageDataFormat>, const N: usize>((data, format): (&'static [u8; N], F)) -> WindowIcon {
        ImageSource::from((data, format)).into()
    }
    /// From encoded data of known format.
    fn from<F: Into<ImageDataFormat>>((data, format): (Vec<u8>, F)) -> WindowIcon {
        ImageSource::from((data, format)).into()
    }
    /// From encoded data of known format.
    fn from<F: Into<ImageDataFormat>>((data, format): (Arc<Vec<u8>>, F)) -> WindowIcon {
        ImageSource::from((data, format)).into()
    }
}

/// Window custom cursor.
#[derive(Debug, Clone, PartialEq)]
pub struct CursorImg {
    /// Cursor image source.
    ///
    /// For better compatibility use a square image between 32 and 128 pixels.
    pub source: ImageSource,
    /// Pixel in the source image that is the exact mouse position.
    ///
    /// This value is ignored if the image source format already has hotspot information.
    pub hotspot: Point,
}
impl_from_and_into_var! {
    fn from(img: CursorImg) -> Option<CursorImg>;
}

/// Frame image capture mode in a window.
///
/// You can set the capture mode using [`WindowVars::frame_capture_mode`].
///
/// [`WindowVars::frame_capture_mode`]: crate::WindowVars::frame_capture_mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FrameCaptureMode {
    /// Frames are not automatically captured, but you can
    /// use [`WINDOWS.frame_image`] to capture frames.
    ///
    /// [`WINDOWS.frame_image`]: crate::WINDOWS::frame_image
    Sporadic,
    /// The next rendered frame will be captured and available in [`FrameImageReadyArgs::frame_image`]
    /// as a full BGRA8 image.
    ///
    /// After the frame is captured the mode changes to `Sporadic`.
    Next,
    /// The next rendered frame will be captured and available in [`FrameImageReadyArgs::frame_image`]
    /// as an A8 mask image.
    ///
    /// After the frame is captured the mode changes to `Sporadic`.
    NextMask(ImageMaskMode),
    /// All subsequent frames rendered will be captured and available in [`FrameImageReadyArgs::frame_image`]
    /// as full BGRA8 images.
    All,
    /// All subsequent frames rendered will be captured and available in [`FrameImageReadyArgs::frame_image`]
    /// as A8 mask images.
    AllMask(ImageMaskMode),
}
impl Default for FrameCaptureMode {
    /// [`Sporadic`]: FrameCaptureMode::Sporadic
    fn default() -> Self {
        Self::Sporadic
    }
}

event_args! {
    /// [`WINDOW_OPEN_EVENT`] args.
    pub struct WindowOpenArgs {
        /// Id of window that has opened.
        pub window_id: WindowId,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// [`WINDOW_CLOSE_EVENT`] args.
    pub struct WindowCloseArgs {
        /// IDs of windows that have closed.
        ///
        /// This is at least one window, is multiple if the close operation was requested as group.
        pub windows: IdSet<WindowId>,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// [`WINDOW_CHANGED_EVENT`] args.
    pub struct WindowChangedArgs {
        /// Window that has moved, resized or has a state change.
        pub window_id: WindowId,

        /// Window state change, if it has changed the values are `(prev, new)` states.
        pub state: Option<(WindowState, WindowState)>,

        /// New window position if it was moved.
        ///
        /// The values are `(global_position, actual_position)` where the global position is
        /// in the virtual space that encompasses all monitors and actual position is in the monitor space.
        pub position: Option<(PxPoint, DipPoint)>,

        /// New window size if it was resized.
        pub size: Option<DipSize>,

        /// If the app or operating system caused the change.
        pub cause: EventCause,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// [`WINDOW_FOCUS_CHANGED_EVENT`] args.
    pub struct WindowFocusChangedArgs {
        /// Previously focused window.
        pub prev_focus: Option<WindowId>,

        /// Newly focused window.
        pub new_focus: Option<WindowId>,

        /// If the focus changed because the previously focused window closed.
        pub closed: bool,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// [`FRAME_IMAGE_READY_EVENT`] args.
    pub struct FrameImageReadyArgs {
        /// Window ID.
        pub window_id: WindowId,

        /// Frame that finished rendering.
        ///
        /// This is *probably* the ID of frame pixels if they are requested immediately.
        pub frame_id: FrameId,

        /// The frame pixels if it was requested when the frame request was sent to the view process.
        pub frame_image: Option<Img>,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// [`WINDOW_CLOSE_REQUESTED_EVENT`] args.
    ///
    /// Requesting [`propagation().stop()`] on this event cancels the window close.
    ///
    /// [`propagation().stop()`]: crate::event::EventPropagationHandle::stop
    pub struct WindowCloseRequestedArgs {
        /// Windows closing.
        ///
        /// This is at least one window, is multiple if the close operation was requested as group, cancelling the request
        /// cancels close for all windows.
        pub windows: IdSet<WindowId>,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }
}
impl WindowChangedArgs {
    /// Returns `true` if this event represents a window state change.
    pub fn is_state_changed(&self) -> bool {
        self.state.is_some()
    }

    /// Returns the previous window state if it has changed.
    pub fn prev_state(&self) -> Option<WindowState> {
        self.state.map(|(p, _)| p)
    }

    /// Returns the new window state if it has changed.
    pub fn new_state(&self) -> Option<WindowState> {
        self.state.map(|(_, n)| n)
    }

    /// Returns `true` if [`new_state`] is `state` and [`prev_state`] is not.
    ///
    /// [`new_state`]: Self::new_state
    /// [`prev_state`]: Self::prev_state
    pub fn entered_state(&self, state: WindowState) -> bool {
        if let Some((prev, new)) = self.state {
            prev != state && new == state
        } else {
            false
        }
    }

    /// Returns `true` if [`prev_state`] is `state` and [`new_state`] is not.
    ///
    /// [`new_state`]: Self::new_state
    /// [`prev_state`]: Self::prev_state
    pub fn exited_state(&self, state: WindowState) -> bool {
        if let Some((prev, new)) = self.state {
            prev == state && new != state
        } else {
            false
        }
    }

    /// Returns `true` if [`new_state`] is one of the fullscreen states and [`prev_state`] is not.
    ///
    /// [`new_state`]: Self::new_state
    /// [`prev_state`]: Self::prev_state
    pub fn entered_fullscreen(&self) -> bool {
        if let Some((prev, new)) = self.state {
            !prev.is_fullscreen() && new.is_fullscreen()
        } else {
            false
        }
    }

    /// Returns `true` if [`prev_state`] is one of the fullscreen states and [`new_state`] is not.
    ///
    /// [`new_state`]: Self::new_state
    /// [`prev_state`]: Self::prev_state
    pub fn exited_fullscreen(&self) -> bool {
        if let Some((prev, new)) = self.state {
            prev.is_fullscreen() && !new.is_fullscreen()
        } else {
            false
        }
    }

    /// Returns `true` if this event represents a window move.
    pub fn is_moved(&self) -> bool {
        self.position.is_some()
    }

    /// Returns `true` if this event represents a window resize.
    pub fn is_resized(&self) -> bool {
        self.size.is_some()
    }
}
impl WindowFocusChangedArgs {
    /// If `window_id` got focus.
    pub fn is_focus(&self, window_id: WindowId) -> bool {
        self.new_focus == Some(window_id)
    }

    /// If `window_id` lost focus.
    pub fn is_blur(&self, window_id: WindowId) -> bool {
        self.prev_focus == Some(window_id)
    }

    /// If `window_id` lost focus because it was closed.
    pub fn is_close(&self, window_id: WindowId) -> bool {
        self.closed && self.is_blur(window_id)
    }

    /// Gets the previous focused window if it was closed.
    pub fn closed(&self) -> Option<WindowId> {
        if self.closed {
            self.prev_focus
        } else {
            None
        }
    }
}

event! {
    /// Window moved, resized or has a state change.
    ///
    /// This event aggregates events moves, resizes and other state changes into a
    /// single event to simplify tracking composite changes, for example, the window changes size and position
    /// when maximized, this can be trivially observed with this event.
    pub static WINDOW_CHANGED_EVENT: WindowChangedArgs;

    /// New window has inited.
    pub static WINDOW_OPEN_EVENT: WindowOpenArgs;

    /// Window finished loading and has opened in the view-process.
    pub static WINDOW_LOAD_EVENT: WindowOpenArgs;

    /// Window focus/blur event.
    pub static WINDOW_FOCUS_CHANGED_EVENT: WindowFocusChangedArgs;

    /// Window close requested event.
    ///
    /// Requesting [`propagation().stop()`] on this event cancels the window close.
    ///
    /// [`propagation().stop()`]: crate::event::EventPropagationHandle::stop
    pub static WINDOW_CLOSE_REQUESTED_EVENT: WindowCloseRequestedArgs;

    /// Window closed event.
    pub static WINDOW_CLOSE_EVENT: WindowCloseArgs;

    /// A window frame has finished rendering.
    ///
    /// You can request a copy of the pixels using [`WINDOWS.frame_image`] or by setting the [`WindowVars::frame_capture_mode`].
    ///
    /// [`WINDOWS.frame_image`]: crate::WINDOWS::frame_image
    /// [`WindowVars::frame_capture_mode`]: crate::WindowVars::frame_capture_mode
    pub static FRAME_IMAGE_READY_EVENT: FrameImageReadyArgs;
}

/// Response message of [`close`] and [`close_together`].
///
/// [`close`]: crate::WINDOWS::close
/// [`close_together`]: crate::WINDOWS::close_together
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloseWindowResult {
    /// Operation completed, all requested windows closed.
    Closed,

    /// Operation canceled, no window closed.
    Cancel,
}

/// Error when a [`WindowId`] is not opened by the [`WINDOWS`] service.
///
/// [`WINDOWS`]: crate::WINDOWS
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct WindowNotFound(pub WindowId);
impl fmt::Display for WindowNotFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "window `{}` not found", self.0)
    }
}
impl std::error::Error for WindowNotFound {}
