use std::{
    borrow::Cow,
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

use linear_map::set::LinearSet;
use parking_lot::Mutex;

use crate::{
    context::WindowContext,
    crate_util::NameIdMap,
    event::{event, event_args},
    image::{Image, ImageDataFormat, ImageSource, ImageVar},
    render::{FrameId, RenderMode},
    text::Text,
    units::*,
    var::*,
    widget_info::WidgetInfoTree,
    BoxedUiNode, IdNameError, UiNode, WidgetId,
};

pub use crate::app::view_process::{CursorIcon, EventCause, FocusIndicator, VideoMode, WindowState};

use super::HeadlessMonitor;

unique_id_32! {
    /// Unique identifier of an open window.
    ///
    /// Can be obtained from [`WindowContext::window_id`] or [`WidgetContext::path`].
    ///
    /// [`WindowContext::window_id`]: crate::context::WindowContext::window_id
    /// [`WidgetContext::path`]: crate::context::WidgetContext::path
    pub struct WindowId;
}
impl WindowId {
    fn name_map() -> parking_lot::MappedMutexGuard<'static, NameIdMap<Self>> {
        static NAME_MAP: Mutex<Option<NameIdMap<WindowId>>> = parking_lot::const_mutex(None);
        parking_lot::MutexGuard::map(NAME_MAP.lock(), |m| m.get_or_insert_with(NameIdMap::new))
    }

    /// Get or generate an id with associated name.
    ///
    /// If the `name` is already associated with an id, returns it.
    /// If the `name` is new, generates a new id and associated it with the name.
    /// If `name` is an empty string just returns a new id.
    pub fn named(name: impl Into<Text>) -> Self {
        Self::name_map().get_id_or_insert(name.into(), Self::new_unique)
    }

    /// Calls [`named`] in a debug build and [`new_unique`] in a release build.
    ///
    /// The [`named`] function causes a hash-map lookup, but if you are only naming a window to find
    /// it in the Inspector you don't need that lookup in a release build, so you can set the [`id`]
    /// to this function call instead.
    ///
    /// [`named`]: WidgetId::named
    /// [`new_unique`]: WidgetId::new_unique
    /// [`id`]: mod@crate::widget_base::implicit_base#wp-id
    pub fn debug_named(name: impl Into<Text>) -> Self {
        #[cfg(debug_assertions)]
        return Self::named(name);

        #[cfg(not(debug_assertions))]
        {
            let _ = name;
            Self::new_unique()
        }
    }

    /// Generate a new id with associated name.
    ///
    /// If the `name` is already associated with an id, returns the [`NameUsed`] error.
    /// If the `name` is an empty string just returns a new id.
    ///
    /// [`NameUsed`]: IdNameError::NameUsed
    pub fn named_new(name: impl Into<Text>) -> Result<Self, IdNameError<Self>> {
        Self::name_map().new_named(name.into(), Self::new_unique)
    }

    /// Returns the name associated with the id or `""`.
    pub fn name(self) -> Text {
        Self::name_map().get_name(self)
    }

    /// Associate a `name` with the id, if it is not named.
    ///
    /// If the `name` is already associated with a different id, returns the [`NameUsed`] error.
    /// If the id is already named, with a name different from `name`, returns the [`AlreadyNamed`] error.
    /// If the `name` is an empty string or already is the name of the id, does nothing.
    ///
    /// [`NameUsed`]: IdNameError::NameUsed
    /// [`AlreadyNamed`]: IdNameError::AlreadyNamed
    pub fn set_name(self, name: impl Into<Text>) -> Result<(), IdNameError<Self>> {
        Self::name_map().set(name.into(), self)
    }
}
impl fmt::Debug for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.name();
        if f.alternate() {
            f.debug_struct("WindowId")
                .field("id", &self.get())
                .field("sequential", &self.sequential())
                .field("name", &name)
                .finish()
        } else if !name.is_empty() {
            write!(f, r#"WindowId("{name}")"#)
        } else {
            write!(f, "WindowId({})", self.sequential())
        }
    }
}
impl fmt::Display for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.name();
        if !name.is_empty() {
            write!(f, "{name}")
        } else {
            write!(f, "WindowId({})", self.sequential())
        }
    }
}
impl_from_and_into_var! {
    /// Calls [`WindowId::named`].
    fn from(name: &'static str) -> WindowId {
        WindowId::named(name)
    }
    /// Calls [`WindowId::named`].
    fn from(name: String) -> WindowId {
        WindowId::named(name)
    }
    /// Calls [`WindowId::named`].
    fn from(name: Cow<'static, str>) -> WindowId {
        WindowId::named(name)
    }
    /// Calls [`WindowId::named`].
    fn from(name: char) -> WindowId {
        WindowId::named(name)
    }
    /// Calls [`WindowId::named`].
    fn from(name: Text) -> WindowId {
        WindowId::named(name)
    }
}
impl fmt::Debug for StaticWindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.get(), f)
    }
}
impl crate::var::IntoValue<WindowId> for &'static StaticWindowId {}

/// Window startup configuration.
///
/// More window configuration is accessible using the [`WindowVars`] type.
///
/// [`WindowVars`]: crate::window::WindowVars
pub struct Window {
    pub(super) id: WidgetId,
    pub(super) start_position: StartPosition,
    pub(super) kiosk: bool,
    pub(super) transparent: bool,
    pub(super) render_mode: Option<RenderMode>,
    pub(super) headless_monitor: HeadlessMonitor,
    pub(super) start_focused: bool,
    pub(super) child: BoxedUiNode,
}
impl Window {
    /// New window from a `root` node that forms the window root widget.
    ///
    /// * `root_id` - Widget ID of `root`.
    /// * `start_position` - Position of the window when it first opens.
    /// * `kiosk` - Only allow full-screen mode. Note this does not configure the windows manager, only blocks the app itself
    ///             from accidentally exiting full-screen. Also causes subsequent open windows to be child of this window.
    /// * `transparent` - If the window should be created in a compositor mode that renders semi-transparent pixels as "see-through".
    /// * `render_mode` - Render mode preference overwrite for this window, note that the actual render mode selected can be different.
    /// * `headless_monitor` - "Monitor" configuration used in [headless mode](WindowMode::is_headless).
    /// * `start_focused` - If the window is forced to be the foreground keyboard focus after opening.
    /// * `root` - The root widget's context priority node, the window uses this and the `root_id` to form the root widget.
    #[allow(clippy::too_many_arguments)]
    pub fn new_root(
        root_id: impl IntoValue<WidgetId>,
        start_position: impl IntoValue<StartPosition>,
        kiosk: bool,
        transparent: bool,
        render_mode: impl IntoValue<Option<RenderMode>>,
        headless_monitor: impl IntoValue<HeadlessMonitor>,
        start_focused: bool,
        root: impl UiNode,
    ) -> Self {
        Window {
            id: root_id.into(),
            start_position: start_position.into(),
            kiosk,
            transparent,
            render_mode: render_mode.into(),
            headless_monitor: headless_monitor.into(),
            start_focused,
            child: root.boxed(),
        }
    }

    /// New window from a `child` node that becomes the child of the window root widget.
    ///
    /// The `child` parameter is a node that is the window's content, if it is an [`Widget`] the `root_id` is the id of
    /// an internal container widget that is the parent of `child`, if it is not a widget it will still be placed in the inner
    /// priority of the root widget.
    ///
    /// See [`new_root`] for other parameters.
    ///
    /// [`new_root`]: Self::new_root
    /// [`Widget`]: crate::Widget
    #[allow(clippy::too_many_arguments)]
    pub fn new_container(
        root_id: impl IntoValue<WidgetId>,
        start_position: impl IntoValue<StartPosition>,
        kiosk: bool,
        transparent: bool,
        render_mode: impl IntoValue<Option<RenderMode>>,
        headless_monitor: impl IntoValue<HeadlessMonitor>,
        start_focused: bool,
        child: impl UiNode,
    ) -> Self {
        Window::new_root(
            root_id,
            start_position,
            kiosk,
            transparent,
            render_mode,
            headless_monitor,
            start_focused,
            crate::widget_base::implicit_base::new_border(child),
        )
    }

    /// New test window.
    #[cfg(any(test, doc, feature = "test_util"))]
    pub fn new_test(child: impl UiNode) -> Self {
        Window::new_container(
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
#[derive(Clone, Copy)]
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
    pub fn is_headed(self) -> bool {
        match self {
            WindowMode::Headed => true,
            WindowMode::Headless | WindowMode::HeadlessWithRenderer => false,
        }
    }

    /// If is the [`Headless`](WindowMode::Headed) or [`HeadlessWithRenderer`](WindowMode::Headed) modes.
    pub fn is_headless(self) -> bool {
        match self {
            WindowMode::Headless | WindowMode::HeadlessWithRenderer => true,
            WindowMode::Headed => false,
        }
    }

    /// If is the [`Headed`](WindowMode::Headed) or [`HeadlessWithRenderer`](WindowMode::HeadlessWithRenderer) modes.
    pub fn has_renderer(self) -> bool {
        match self {
            WindowMode::Headed | WindowMode::HeadlessWithRenderer => true,
            WindowMode::Headless => false,
        }
    }
}

/// Window chrome, the non-client area of the window.
#[derive(Clone, PartialEq, Eq)]
pub enum WindowChrome {
    /// System chrome.
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
    /// Is system chrome.
    pub fn is_default(&self) -> bool {
        matches!(self, WindowChrome::Default)
    }

    /// Is chromeless.
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

/// Window icon.
#[derive(Clone, PartialEq)]
pub enum WindowIcon {
    /// Operating system default icon.
    ///
    /// In Windows this is the icon associated with the executable.
    Default,
    /// Image is requested from [`Images`].
    ///
    /// [`Images`]: crate::image::Images
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
    /// New window icon from a function that generates a new icon [`UiNode`] for the window.
    ///
    /// The function is called once on init and every time the window icon property changes,
    /// the input is a headless window context, the result is a node that is rendered to create an icon.
    ///
    /// The icon node is updated like any other node and it can request a new render. Note that just
    /// because you can update the icon does not mean that animating it is a good idea.
    ///
    /// # Examples
    ///
    /// ```
    /// # use zero_ui_core::{window::WindowIcon, render::RenderMode};
    /// # macro_rules! container { ($($tt:tt)*) => { zero_ui_core::NilUiNode } }
    /// # let _ =
    /// WindowIcon::render(
    ///     |_ctx| container! {
    ///         size = (36, 36);
    ///         background_gradient = Line::to_bottom_right(), stops![colors::MIDNIGHT_BLUE, 70.pct(), colors::CRIMSON];
    ///         corner_radius = 6;
    ///         font_size = 28;
    ///         font_weight = FontWeight::BOLD;
    ///         content = text("A");
    ///     }
    /// )
    /// # ;
    /// ```
    pub fn render<I, F>(new_icon: F) -> Self
    where
        I: UiNode,
        F: Fn(&mut WindowContext) -> I + 'static,
    {
        Self::Image(ImageSource::render_node(RenderMode::Software, move |ctx, args| {
            let node = new_icon(ctx);
            super::WindowVars::req(&ctx.window_state).parent().set_ne(ctx.vars, args.parent);
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
    /// [`WINDOW_OPEN_EVENT`] args.
    pub struct WindowOpenArgs {
        /// Id of window that was opened or closed.
        pub window_id: WindowId,

        ..

        /// Broadcast to all widgets in the window.
        fn delivery_list(&self) -> EventDeliveryList {
            EventDeliveryList::window(self.window_id)
        }
    }

    /// [`WINDOW_CLOSE_EVENT`] args.
    pub struct WindowCloseArgs {
        /// Id of windows that were closed.
        ///
        ///  This is at least one window, is multiple if the close operation was requested as group.
        pub windows: LinearSet<WindowId>,

        ..

        /// Broadcast to all widgets in the window.
        fn delivery_list(&self) -> EventDeliveryList {
            let mut list = EventDeliveryList::none();
            for w in self.windows.iter() {
                list = list.with_window(*w);
            }
            list
        }
    }

    /// [`WINDOW_CHANGED_EVENT`] args.
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

        /// Broadcast to all widgets in the window.
        fn delivery_list(&self) -> EventDeliveryList {
            EventDeliveryList::window(self.window_id)
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

        /// Broadcast to all widgets in the previous and new focused window.
        fn delivery_list(&self) -> EventDeliveryList {
            EventDeliveryList::window_opt(self.prev_focus).with_window_opt(self.new_focus)
        }
    }

    /// [`WIDGET_INFO_CHANGED_EVENT`] args.
    pub struct WidgetInfoChangedArgs {
        /// Window ID.
        pub window_id: WindowId,

        /// Previous widget tree.
        ///
        /// This is an empty tree before the first tree build.
        pub prev_tree: WidgetInfoTree,

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

        /// Broadcast to all widgets in the window.
        fn delivery_list(&self) -> EventDeliveryList {
            EventDeliveryList::window(self.window_id)
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

        /// Broadcast to all widgets in the window.
        fn delivery_list(&self) -> EventDeliveryList {
            EventDeliveryList::window(self.window_id)
        }
    }

    /// [`WindowCloseRequestedEvent`] args.
    ///
    /// Requesting [`propagation().stop()`] on this event cancels the window close.
    ///
    /// [`propagation().stop()`]: crate::event::EventPropagationHandle::stop
    pub struct WindowCloseRequestedArgs {
        /// Windows closing.
        ///
        /// This is at least one window, is multiple if the close operation was requested as group, cancelling the request
        /// cancels close for all windows .
        pub windows: LinearSet<WindowId>,

        ..

        /// Broadcast to all widgets in the windows.
        fn delivery_list(&self) -> EventDeliveryList {
            let mut list = EventDeliveryList::none();
            for w in self.windows.iter() {
                list = list.with_window(*w);
            }
            list
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
    /// This event coalesces events usually named `WINDOW_MOVED`, `WINDOW_RESIZED` and `WINDOW_STATE_CHANGED` into a
    /// single event to simplify tracking composite changes, for example, the window changes size and position
    /// when maximized, this can be trivially observed with this event.
    pub static WINDOW_CHANGED_EVENT: WindowChangedArgs;

    /// New window event.
    pub static WINDOW_OPEN_EVENT: WindowOpenArgs;

    /// Window finished loading and has opened in the view-process.
    pub static WINDOW_LOAD_EVENT: WindowOpenArgs;

    /// Window focus/blur event.
    pub static WINDOW_FOCUS_CHANGED_EVENT: WindowFocusChangedArgs;

    /// Closing window event.
    ///
    /// Requesting [`propagation().stop()`] on this event cancels the window close.
    ///
    /// [`propagation().stop()`]: crate::event::EventPropagationHandle::stop
    pub static WINDOW_CLOSE_REQUESTED_EVENT: WindowCloseRequestedArgs;

    /// Close window event.
    pub static WINDOW_CLOSE_EVENT: WindowCloseArgs;

    /// A window widget tree was rebuild.
    ///
    /// You can request the widget info tree using [`Windows::widget_tree`].
    ///
    /// [`Windows::widget_tree`]: crate::window::Windows::widget_tree
    pub static WIDGET_INFO_CHANGED_EVENT: WidgetInfoChangedArgs;

    /// A window frame has finished rendering.
    ///
    /// You can request a copy of the pixels using [`Windows::frame_image`] or by setting the [`WindowVars::frame_capture_mode`].
    ///
    /// [`Windows::frame_image`]: crate::window::Windows::frame_image
    /// [`WindowVars::frame_capture_mode`]: crate::window::WindowVars::frame_capture_mode
    pub static FRAME_IMAGE_READY_EVENT: FrameImageReadyArgs;
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
    type Var = crate::var::LocalVar<Option<CursorIcon>>;

    fn into_var(self) -> Self::Var {
        crate::var::LocalVar(Some(self))
    }
}
impl crate::var::IntoValue<Option<CursorIcon>> for CursorIcon {}

impl crate::var::IntoVar<Option<RenderMode>> for RenderMode {
    type Var = crate::var::LocalVar<Option<RenderMode>>;

    fn into_var(self) -> Self::Var {
        crate::var::LocalVar(Some(self))
    }
}
impl crate::var::IntoValue<Option<RenderMode>> for RenderMode {}

impl crate::var::IntoVar<Option<WindowId>> for WindowId {
    type Var = crate::var::LocalVar<Option<WindowId>>;

    fn into_var(self) -> Self::Var {
        crate::var::LocalVar(Some(self))
    }
}
impl crate::var::IntoValue<Option<WindowId>> for WindowId {}
