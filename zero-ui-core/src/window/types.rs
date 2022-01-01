use std::{
    fmt,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

use crate::{
    app::view_process::EventCause,
    cancelable_event_args,
    context::WindowContext,
    event, event_args,
    image::{Image, ImageDataFormat, ImageSource, ImageVar},
    render::FrameId,
    state::OwnedStateMap,
    task::http::Uri,
    text::Text,
    units::*,
    var::*,
    widget_info::WidgetInfoTree,
    BoxedUiNode, UiNode, WidgetId,
};

use super::HeadlessMonitor;

pub use zero_ui_view_api::{CursorIcon, RenderMode, VideoMode, WindowState};

unique_id_32! {
    /// Unique identifier of an open window.
    ///
    /// Can be obtained from [`WindowContext::window_id`] or [`WidgetContext::path`].
    ///
    /// [`WindowContext::window_id`]: crate::context::WindowContext::window_id
    /// [`WidgetContext::path`]: crate::context::WidgetContext::path
    pub struct WindowId;
}
impl fmt::Debug for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("WindowId")
                .field("id", &self.get())
                .field("sequential", &self.sequential())
                .finish()
        } else {
            write!(f, "WindowId({})", self.sequential())
        }
    }
}
impl fmt::Display for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WinId({})", self.sequential())
    }
}

/// Window startup configuration.
///
/// More window configuration is accessible using the [`WindowVars`] type.
///
/// [`WindowVars`]: crate::window::WindowVars
pub struct Window {
    pub(super) state: OwnedStateMap,
    pub(super) id: WidgetId,
    pub(super) start_position: StartPosition,
    pub(super) kiosk: bool,
    pub(super) transparent: bool,
    pub(super) render_mode: Option<RenderMode>,
    pub(super) headless_monitor: HeadlessMonitor,
    pub(super) child: BoxedUiNode,
}
impl Window {
    /// New window configuration.
    ///
    /// * `root_id` - Widget ID of `child`.
    /// * `start_position` - Position of the window when it first opens.
    /// * `kiosk` - Only allow full-screen mode. Note this does not configure the operating system, only blocks the app itself
    ///             from accidentally exiting full-screen. Also causes subsequent open windows to be child of this window.
    /// * `transparent` - If the window should be created in a compositor mode that renders semi-transparent pixels as "see-through".
    /// * `render_mode` - Render mode preference overwrite for this window, note that the actual render mode selected can be different.
    /// * `headless_monitor` - "Monitor" configuration used in [headless mode](WindowMode::is_headless).
    /// * `child` - The root widget outermost node, the window sets-up the root widget using this and the `root_id`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        root_id: impl IntoValue<WidgetId>,
        start_position: impl IntoValue<StartPosition>,
        kiosk: bool,
        transparent: bool,
        render_mode: impl IntoValue<Option<RenderMode>>,
        headless_monitor: impl IntoValue<HeadlessMonitor>,
        child: impl UiNode,
    ) -> Self {
        Window {
            state: OwnedStateMap::default(),
            id: root_id.into(),
            kiosk,
            transparent,
            render_mode: render_mode.into(),
            start_position: start_position.into(),
            headless_monitor: headless_monitor.into(),
            child: child.boxed(),
        }
    }
}

bitflags! {
    /// Window auto-size config.
    pub struct AutoSize: u8 {
        /// Does not automatically adjust size.
        const DISABLED = 0;
        /// Uses the content desired width.
        const CONTENT_WIDTH = 0b01;
        /// Uses the content desired height.
        const CONTENT_HEIGHT = 0b10;
        /// Uses the content desired width and height.
        const CONTENT = Self::CONTENT_WIDTH.bits | Self::CONTENT_HEIGHT.bits;
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
/// is open just after the first [`UiNode::render`] call.
#[derive(Clone)]
pub enum StartPosition {
    /// Resolves the `position` property.
    Default,

    /// Centralizes the window in the monitor screen, defined by the `monitor` property.
    ///
    /// Uses the `headless_monitor` in headless windows and falls-back to not centering if no
    /// monitor was found in headed windows.
    CenterMonitor,
    /// Centralizes the window in the parent window, defined by the `parent` property.
    ///
    /// Falls-back to center on the monitor if the window has no parent.
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

/// Mode of an open window.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WindowMode {
    /// Normal mode, shows a system window with content rendered.
    Headed,

    /// Headless mode, no system window and no renderer. The window does layout and calls [`UiNode::render`] but
    /// it does not actually generates frame textures.
    Headless,
    /// Headless mode, no visible system window but with a renderer. The window does everything a [`Headed`](WindowMode::Headed)
    /// window does, except presenting frame textures in a system window.
    HeadlessWithRenderer,
}
impl fmt::Debug for WindowMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "WindowMode::")?;
        }
        match self {
            WindowMode::Headed => write!(f, "Headed"),
            WindowMode::Headless => write!(f, "Headless"),
            WindowMode::HeadlessWithRenderer => write!(f, "HeadlessWithRenderer"),
        }
    }
}
impl WindowMode {
    /// If is the [`Headed`](WindowMode::Headed) mode.
    #[inline]
    pub fn is_headed(self) -> bool {
        match self {
            WindowMode::Headed => true,
            WindowMode::Headless | WindowMode::HeadlessWithRenderer => false,
        }
    }

    /// If is the [`Headless`](WindowMode::Headed) or [`HeadlessWithRenderer`](WindowMode::Headed) modes.
    #[inline]
    pub fn is_headless(self) -> bool {
        match self {
            WindowMode::Headless | WindowMode::HeadlessWithRenderer => true,
            WindowMode::Headed => false,
        }
    }

    /// If is the [`Headed`](WindowMode::Headed) or [`HeadlessWithRenderer`](WindowMode::HeadlessWithRenderer) modes.
    #[inline]
    pub fn has_renderer(self) -> bool {
        match self {
            WindowMode::Headed | WindowMode::HeadlessWithRenderer => true,
            WindowMode::Headless => false,
        }
    }
}

/// Window chrome, the non-client area of the window.
#[derive(Clone, PartialEq)]
pub enum WindowChrome {
    /// Operating system chrome.
    Default,
    /// Chromeless.
    None,
    /// An [`UiNode`] that provides the window chrome.
    Custom,
}
impl fmt::Debug for WindowChrome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "WindowChrome::")?;
        }
        match self {
            WindowChrome::Default => write!(f, "Default"),
            WindowChrome::None => write!(f, "None"),
            WindowChrome::Custom => write!(f, "Custom"),
        }
    }
}
impl WindowChrome {
    /// Is operating system chrome.
    #[inline]
    pub fn is_default(&self) -> bool {
        matches!(self, WindowChrome::Default)
    }

    /// Is chromeless.
    #[inline]
    pub fn is_none(&self) -> bool {
        matches!(self, WindowChrome::None)
    }
}
impl Default for WindowChrome {
    /// [`WindowChrome::Default`]
    fn default() -> Self {
        Self::Default
    }
}
impl_from_and_into_var! {
    /// | Input  | Output                  |
    /// |--------|-------------------------|
    /// |`true`  | `WindowChrome::Default` |
    /// |`false` | `WindowChrome::None`    |
    fn from(default_: bool) -> WindowChrome {
        if default_ {
            WindowChrome::Default
        } else {
            WindowChrome::None
        }
    }
}

bitflags! {
    /// Mask of allowed [`WindowState`] states of a window.
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

// We don't use Rc<dyn ..> because of this issue: https://github.com/rust-lang/rust/issues/69757
type RenderIcon = Rc<Box<dyn Fn(&mut WindowContext) -> BoxedUiNode>>;

/// Window icon.
#[derive(Clone)]
pub enum WindowIcon {
    /// Operating system default icon.
    ///
    /// In Windows this is the icon associated with the executable.
    Default,
    /// Image is requested from [`Images`].
    ///
    /// [`Images`]: crate::image::Images
    Image(ImageSource),
    /// An [`UiNode`] that draws the icon.
    ///
    /// Use the [`render`](Self::render) function to initialize.
    Render(RenderIcon),
}
impl fmt::Debug for WindowIcon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "WindowIcon::")?;
        }
        match self {
            WindowIcon::Default => write!(f, "Default"),
            WindowIcon::Image(r) => write!(f, "Image({:?})", r),
            WindowIcon::Render(_) => write!(f, "Render(_)"),
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
    /// New window icon from a function that generates a new icon [`UiNode`] for the window.
    ///
    /// The function is called once on init and every time the window icon property changes,
    /// the input is the window context, the result is a node that is rendered to create an icon.
    ///
    /// The icon node is updated like any other node and it can request a new render, you should
    /// limit the interval for new frames,
    pub fn render<I: UiNode, F: Fn(&mut WindowContext) -> I + 'static>(new_icon: F) -> Self {
        Self::Render(Rc::new(Box::new(move |ctx| new_icon(ctx).boxed())))
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
    fn from(uri: Uri) -> WindowIcon {
        ImageSource::from(uri).into()
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
    fn from(s: Text) -> WindowIcon {
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
    fn from<F: Into<ImageDataFormat> + Clone>((data, format): (&'static [u8], F)) -> WindowIcon {
        ImageSource::from((data, format)).into()
    }
    /// From encoded data of known format.
    fn from<F: Into<ImageDataFormat> + Clone, const N: usize>((data, format): (&'static [u8; N], F)) -> WindowIcon {
        ImageSource::from((data, format)).into()
    }
    /// From encoded data of known format.
    fn from<F: Into<ImageDataFormat> + Clone>((data, format): (Vec<u8>, F)) -> WindowIcon {
        ImageSource::from((data, format)).into()
    }
    /// From encoded data of known format.
    fn from<F: Into<ImageDataFormat> + Clone>((data, format): (Arc<Vec<u8>>, F)) -> WindowIcon {
        ImageSource::from((data, format)).into()
    }
}
impl PartialEq for WindowIcon {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Image(l0), Self::Image(r0)) => l0 == r0,
            (Self::Render(l0), Self::Render(r0)) => Rc::ptr_eq(l0, r0),
            (Self::Default, Self::Default) => true,
            _ => false,
        }
    }
}

/// Frame image capture mode in a window.
///
/// You can set the capture mode using [`WindowVars::frame_capture_mode`].
///
/// [`WindowVars::frame_capture_mode`]: crate::window::WindowVars::frame_capture_mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameCaptureMode {
    /// Frames are not automatically captured, but you can
    /// use [`Windows::frame_image`] to capture frames.
    ///
    /// [`Windows::frame_image`]: crate::window::Windows::frame_image
    Sporadic,
    /// The next rendered frame will be captured and available in [`FrameImageReadyArgs::frame_image`].
    ///
    /// After the frame is captured the mode changes to `Sporadic`.
    Next,
    /// All subsequent frames rendered will be captured and available in [`FrameImageReadyArgs::frame_image`].
    All,
}
impl Default for FrameCaptureMode {
    /// [`Sporadic`]: FrameCaptureMode::Sporadic
    fn default() -> Self {
        Self::Sporadic
    }
}

event_args! {
    /// [`WindowOpenEvent`] args.
    pub struct WindowOpenArgs {
        /// Id of window that was opened or closed.
        pub window_id: WindowId,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// [`WindowCloseEvent`] args.
    pub struct WindowCloseArgs {
        /// Id of window that was opened or closed.
        pub window_id: WindowId,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// [`WindowChangedEvent`] args.
    pub struct WindowChangedArgs {
        /// Window that was moved, resized or has a state change.
        pub window_id: WindowId,

        /// Window state change, if it has changed the values are `(prev, new)` states.
        pub state: Option<(WindowState, WindowState)>,

        /// New window position if it was moved.
        pub position: Option<DipPoint>,

        /// New window size if it was resized.
        pub size: Option<DipSize>,

        /// If the app or operating system caused the change.
        pub cause: EventCause,

        ..

        /// Returns `true` for all widgets in the [window](Self::window_id).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// [`WindowFocusChangedEvent`] args.
    pub struct WindowFocusArgs {
        /// Id of window that got or lost keyboard focus.
        pub window_id: WindowId,

        /// `true` if the window got focus, `false` if the window lost focus (blur).
        pub focused: bool,

        /// If the focused window was closed.
        pub closed: bool,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// [`WindowScaleChangedEvent`] args.
    pub struct WindowScaleChangedArgs {
        /// Window ID.
        pub window_id: WindowId,
        /// New scale factor.
        pub new_scale_factor: Factor,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// [`WidgetInfoChangedEvent`] args.
    pub struct WidgetInfoChangedArgs {
        /// Window ID.
        pub window_id: WindowId,

        /// New widget tree.
        pub tree: WidgetInfoTree,

        /// If a layout was requested with this change.
        ///
        /// If `true` the widget bounds and visibility may be out-of-date until after the next layout.
        pub pending_layout: bool,

        /// If a frame rebuild was requested with this change.
        ///
        /// If `true` the widget visibility may be out-of-date until after the next render.
        pub pending_render: bool,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// [`FrameImageReadyEvent`] args.
    pub struct FrameImageReadyArgs {
        /// Window ID.
        pub window_id: WindowId,

        /// Frame that finished rendering.
        ///
        /// This is *probably* the ID of frame pixels if they are requested immediately.
        pub frame_id: FrameId,

        /// The frame pixels if it was requested when the frame request was sent to the view process.
        pub frame_image: Option<Image>,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
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
cancelable_event_args! {
    /// [`WindowCloseRequestedEvent`] args.
    pub struct WindowCloseRequestedArgs {
        /// Window ID.
        pub window_id: WindowId,

        pub(super) close_group: CloseGroupId,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }
}

pub(super) type CloseGroupId = u32;

event! {
    /// Window moved, resized or has a state change.
    ///
    /// This event coalesces events usually named `WindowMoved`, `WindowResized` and `WindowStateChanged` into a
    /// single event to simplify tracking composite changes, for example, the window changes size and position
    /// when maximized, this can be trivially observed with this event.
    pub WindowChangedEvent: WindowChangedArgs;

    /// New window event.
    pub WindowOpenEvent: WindowOpenArgs;

    /// Window focus/blur event.
    pub WindowFocusChangedEvent: WindowFocusArgs;

    /// Window scale factor changed.
    pub WindowScaleChangedEvent: WindowScaleChangedArgs;

    /// Closing window event.
    pub WindowCloseRequestedEvent: WindowCloseRequestedArgs;

    /// Close window event.
    pub WindowCloseEvent: WindowCloseArgs;

    /// A window widget tree was rebuild.
    ///
    /// You can request the widget info tree using [`Windows::widget_tree`].
    ///
    /// [`Windows::widget_tree`]: crate::window::Windows::widget_tree
    pub WidgetInfoChangedEvent: WidgetInfoChangedArgs;

    /// A window frame has finished rendering.
    ///
    /// You can request a copy of the pixels using [`Windows::frame_image`] or by setting the [`WindowVars::frame_capture_mode`].
    ///
    /// [`Windows::frame_image`]: crate::window::Windows::frame_image
    /// [`WindowVars::frame_capture_mode`]: crate::window::WindowVars::frame_capture_mode
    pub FrameImageReadyEvent: FrameImageReadyArgs;
}

/// Response message of [`close`] and [`close_together`].
///
/// [`close`]: crate::window::Windows::close
/// [`close_together`]: crate::window::Windows::close_together
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloseWindowResult {
    /// Operation completed, all requested windows closed.
    Closed,

    /// Operation canceled, no window closed.
    Cancel,
}

/// Error when a [`WindowId`] is not opened by the [`Windows`] service.
///
/// [`Windows`]: crate::window::Windows
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct WindowNotFound(pub WindowId);
impl fmt::Display for WindowNotFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "window `{}` not found", self.0)
    }
}
impl std::error::Error for WindowNotFound {}

impl crate::var::IntoVar<Option<CursorIcon>> for CursorIcon {
    type Var = crate::var::OwnedVar<Option<CursorIcon>>;

    fn into_var(self) -> Self::Var {
        crate::var::OwnedVar(Some(self))
    }
}
impl crate::var::IntoValue<Option<CursorIcon>> for CursorIcon {}

impl crate::var::IntoVar<Option<RenderMode>> for RenderMode {
    type Var = crate::var::OwnedVar<Option<RenderMode>>;

    fn into_var(self) -> Self::Var {
        crate::var::OwnedVar(Some(self))
    }
}
impl crate::var::IntoValue<Option<RenderMode>> for RenderMode {}
