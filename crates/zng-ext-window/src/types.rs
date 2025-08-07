use std::{
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

use zng_app::{
    AppEventSender, Deadline,
    event::{event, event_args},
    update::UpdateOp,
    widget::{WidgetId, node::UiNode},
    window::{WINDOW, WindowId},
};
use zng_ext_image::{ImageSource, ImageVar, Img};
use zng_layout::unit::{DipPoint, DipSize, Point, PxPoint};
use zng_txt::Txt;
use zng_unique_id::IdSet;
use zng_var::impl_from_and_into_var;
use zng_view_api::{
    image::{ImageDataFormat, ImageMaskMode},
    ipc::ViewChannelError,
    window::{CursorIcon, EventCause, FrameId},
};

pub use zng_view_api::window::{FocusIndicator, RenderMode, VideoMode, WindowButton, WindowState};
use zng_wgt::prelude::IntoUiNode;

use crate::{HeadlessMonitor, WINDOW_Ext as _, WINDOWS};

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
    pub(super) child: UiNode,
}
impl WindowRoot {
    /// New window from a `root` node that forms the window root widget.
    ///
    /// * `root_id` - Widget ID of `root`.
    /// * `start_position` - Position of the window when it first opens.
    /// * `kiosk` - Only allow fullscreen mode. Note this does not configure the windows manager, only blocks the app itself
    ///   from accidentally exiting fullscreen. Also causes subsequent open windows to be child of this window.
    /// * `transparent` - If the window should be created in a compositor mode that renders semi-transparent pixels as "see-through".
    /// * `render_mode` - Render mode preference overwrite for this window, note that the actual render mode selected can be different.
    /// * `headless_monitor` - "Monitor" configuration used in [headless mode](zng_app::window::WindowMode::is_headless).
    /// * `start_focused` - If the window is forced to be the foreground keyboard focus after opening.
    /// * `root` - The root widget's outermost `CONTEXT` node, the window uses this and the `root_id` to form the root widget.
    #[expect(clippy::too_many_arguments)]
    pub fn new(
        root_id: WidgetId,
        start_position: StartPosition,
        kiosk: bool,
        transparent: bool,
        render_mode: Option<RenderMode>,
        headless_monitor: HeadlessMonitor,
        start_focused: bool,
        root: impl IntoUiNode,
    ) -> Self {
        WindowRoot {
            id: root_id,
            start_position,
            kiosk,
            transparent,
            render_mode,
            headless_monitor,
            start_focused,
            child: root.into_node(),
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
    #[expect(clippy::too_many_arguments)]
    pub fn new_container(
        root_id: WidgetId,
        start_position: StartPosition,
        kiosk: bool,
        transparent: bool,
        render_mode: Option<RenderMode>,
        headless_monitor: HeadlessMonitor,
        start_focused: bool,
        child: impl IntoUiNode,
    ) -> Self {
        WindowRoot::new(
            root_id,
            start_position,
            kiosk,
            transparent,
            render_mode,
            headless_monitor,
            start_focused,
            zng_app::widget::base::node::widget_inner(child),
        )
    }

    /// New test window.
    #[cfg(any(test, doc, feature = "test_util"))]
    pub fn new_test(child: impl IntoUiNode) -> Self {
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
///
///  [`UiNode::layout`]: zng_app::widget::node::UiNode::layout
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
        /// Enable fullscreen, but only windowed not exclusive video.
        const FULLSCREEN_WN_ONLY = 0b0100;
        /// Allow fullscreen windowed or exclusive video.
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
    /// [`IMAGES`]: zng_ext_image::IMAGES
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
    /// [`image::render_retain`]: fn@zng_ext_image::render_retain
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use zng_ext_window::WindowIcon;
    /// # macro_rules! Container { ($($tt:tt)*) => { zng_app::widget::node::FillUiNode } }
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
    ///
    /// [`UiNode`]: zng_app::widget::node::UiNode
    pub fn render(new_icon: impl Fn() -> UiNode + Send + Sync + 'static) -> Self {
        Self::Image(ImageSource::render_node(RenderMode::Software, move |args| {
            let node = new_icon();
            WINDOW.vars().parent().set(args.parent);
            node
        }))
    }
}
#[cfg(feature = "http")]
impl_from_and_into_var! {
    fn from(uri: zng_task::http::Uri) -> WindowIcon {
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

    /// Icon to use if the image cannot be displayed.
    pub fallback: CursorIcon,
}
impl_from_and_into_var! {
    fn from(img: CursorImg) -> Option<CursorImg>;
}

/// Window cursor source.
#[derive(Debug, Clone, PartialEq)]
pub enum CursorSource {
    /// Platform dependent named cursor icon.
    Icon(CursorIcon),
    /// Custom cursor image, with fallback.
    Img(CursorImg),
    /// Don't show cursor.
    Hidden,
}
impl CursorSource {
    /// Get the icon, image fallback or `None` if is hidden.
    pub fn icon(&self) -> Option<CursorIcon> {
        match self {
            CursorSource::Icon(ico) => Some(*ico),
            CursorSource::Img(img) => Some(img.fallback),
            CursorSource::Hidden => None,
        }
    }

    /// Custom icon image source.
    pub fn img(&self) -> Option<&ImageSource> {
        match self {
            CursorSource::Img(img) => Some(&img.source),
            _ => None,
        }
    }

    /// Custom icon image click point, when the image data does not contain a hotspot.
    pub fn hotspot(&self) -> Option<&Point> {
        match self {
            CursorSource::Img(img) => Some(&img.hotspot),
            _ => None,
        }
    }
}
impl_from_and_into_var! {
    fn from(icon: CursorIcon) -> CursorSource {
        CursorSource::Icon(icon)
    }
    fn from(img: CursorImg) -> CursorSource {
        CursorSource::Img(img)
    }
    /// Converts `true` to `CursorIcon::Default` and `false` to `CursorSource::Hidden`.
    fn from(default_icon_or_hidden: bool) -> CursorSource {
        if default_icon_or_hidden {
            CursorIcon::Default.into()
        } else {
            CursorSource::Hidden
        }
    }
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

        /// The frame pixels if it was requested when the frame request was sent to the view-process.
        ///
        /// See [`WindowVars::frame_capture_mode`] for more details.
        ///
        /// [`WindowVars::frame_capture_mode`]: crate::WindowVars::frame_capture_mode
        pub frame_image: Option<Img>,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }

    /// [`WINDOW_CLOSE_REQUESTED_EVENT`] args.
    ///
    /// Requesting `propagation().stop()` on this event cancels the window close.
    pub struct WindowCloseRequestedArgs {
        /// Windows closing, headed and headless.
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
        if self.closed { self.prev_focus } else { None }
    }
}
impl WindowCloseRequestedArgs {
    /// Gets only headed windows that will close.
    pub fn headed(&self) -> impl Iterator<Item = WindowId> + '_ {
        self.windows
            .iter()
            .copied()
            .filter(|&id| WINDOWS.mode(id).map(|m| m.is_headed()).unwrap_or(false))
    }

    /// Gets only headless windows that will close.
    pub fn headless(&self) -> impl Iterator<Item = WindowId> + '_ {
        self.windows
            .iter()
            .copied()
            .filter(|&id| WINDOWS.mode(id).map(|m| m.is_headless()).unwrap_or(false))
    }
}

event! {
    /// Window moved, resized or other state changed.
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
    /// Calling `propagation().stop()` on this event cancels the window close.
    pub static WINDOW_CLOSE_REQUESTED_EVENT: WindowCloseRequestedArgs;

    /// Window closed event.
    ///
    /// The closed windows deinit after this event notifies, so the window content can subscribe to it.
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
/// [`WindowId`]: crate::WindowId
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct WindowNotFoundError(WindowId);
impl WindowNotFoundError {
    /// New from id.
    pub fn new(id: impl Into<WindowId>) -> Self {
        Self(id.into())
    }

    /// Gets the ID that was not found.
    pub fn id(&self) -> WindowId {
        self.0
    }
}
impl fmt::Display for WindowNotFoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "window `{}` not found", self.0)
    }
}
impl std::error::Error for WindowNotFoundError {}

/// Represents a handle that stops the window from loading while the handle is alive.
///
/// A handle can be retrieved using [`WINDOWS.loading_handle`] or [`WINDOW.loading_handle`], the window does not
/// open until all handles expire or are dropped.
///
/// [`WINDOWS.loading_handle`]: WINDOWS::loading_handle
/// [`WINDOW.loading_handle`]: WINDOW::loading_handle
#[derive(Clone)]
#[must_use = "the window does not await loading if the handle is dropped"]
pub struct WindowLoadingHandle(pub(crate) Arc<WindowLoadingHandleData>);
impl WindowLoadingHandle {
    /// Handle expiration deadline.
    pub fn deadline(&self) -> Deadline {
        self.0.deadline
    }
}
pub(crate) struct WindowLoadingHandleData {
    pub(crate) update: AppEventSender,
    pub(crate) deadline: Deadline,
}
impl Drop for WindowLoadingHandleData {
    fn drop(&mut self) {
        let _ = self.update.send_update(UpdateOp::Update, None);
    }
}
impl PartialEq for WindowLoadingHandle {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for WindowLoadingHandle {}
impl std::hash::Hash for WindowLoadingHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (Arc::as_ptr(&self.0) as usize).hash(state);
    }
}
impl fmt::Debug for WindowLoadingHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WindowLoadingHandle(_)")
    }
}

/// Error calling a view-process API extension associated with a window or renderer.
#[derive(Debug)]
#[non_exhaustive]
pub enum ViewExtensionError {
    /// Window is not open in the `WINDOWS` service.
    WindowNotFound(WindowNotFoundError),
    /// Window must be headed to call window extensions.
    WindowNotHeaded(WindowId),
    /// Window is not open in the view-process.
    ///
    /// If the window is headless without renderer it will never open in view-process, if the window is headed
    /// headless with renderer the window opens in the view-process after the first layout.
    NotOpenInViewProcess(WindowId),
    /// View-process is not running.
    Disconnected,
    /// Api Error.
    Api(zng_view_api::api_extension::ApiExtensionRecvError),
}
impl fmt::Display for ViewExtensionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WindowNotFound(e) => fmt::Display::fmt(e, f),
            Self::WindowNotHeaded(id) => write!(f, "window `{id}` is not headed"),
            Self::NotOpenInViewProcess(id) => write!(f, "window/renderer `{id}` not open in the view-process"),
            Self::Disconnected => fmt::Display::fmt(&ViewChannelError::Disconnected, f),
            Self::Api(e) => fmt::Display::fmt(e, f),
        }
    }
}
impl std::error::Error for ViewExtensionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::WindowNotFound(e) => Some(e),
            Self::WindowNotHeaded(_) => None,
            Self::NotOpenInViewProcess(_) => None,
            Self::Disconnected => Some(&ViewChannelError::Disconnected),
            Self::Api(e) => Some(e),
        }
    }
}
