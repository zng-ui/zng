//! App window manager.

use std::{fmt, mem, rc::Rc, time::Instant};

pub use crate::app::view_process::{CursorIcon, EventCause, MonitorInfo, VideoMode, WindowTheme};
use linear_map::LinearMap;
use webrender_api::{BuiltDisplayList, DynamicProperties, PipelineId};

use crate::{
    app::{
        self,
        raw_events::*,
        view_process::{self, ViewProcess, ViewProcessRespawnedEvent, ViewRenderer, ViewWindow},
        AppEventSender, AppExtended, AppExtension, AppProcessExt, ControlFlow,
    },
    cancelable_event_args,
    color::rgb,
    context::{AppContext, UpdateDisplayRequest, WidgetContext, WindowContext},
    event::{event, EventUpdateArgs},
    event_args, impl_from_and_into_var,
    render::{FrameBuilder, FrameHitInfo, FrameId, FrameInfo, FramePixels, FrameUpdate, WidgetTransformKey},
    service::Service,
    state::OwnedStateMap,
    state_key,
    text::{Text, TextAntiAliasing, ToText},
    units::*,
    var::{response_var, var, IntoValue, RcVar, ReadOnlyRcVar, ResponderVar, ResponseVar, Var},
    BoxedUiNode, UiNode, WidgetId,
};

unique_id! {
    /// Unique identifier of an open window.
    ///
    /// Can be obtained from [`WindowContext::window_id`] or [`WidgetContext::path`].
    #[derive(Debug)]
    pub struct WindowId;
}
impl fmt::Display for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WinId({})", self.get())
    }
}

unique_id! {
    /// Unique identifier of a monitor screen.
    #[derive(Debug)]
    pub struct MonitorId;
}
impl fmt::Display for MonitorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MonitorId({})", self.get())
    }
}

/// Extension trait, adds [`run_window`](AppRunWindowExt::run_window) to [`AppExtended`].
pub trait AppRunWindowExt {
    /// Runs the application event loop and requests a new window.
    ///
    /// The `new_window` argument is the [`WindowContext`] of the new window.
    ///
    /// This method only returns when the app has shutdown.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use zero_ui_core::app::App;
    /// # use zero_ui_core::window::AppRunWindowExt;
    /// # macro_rules! window { ($($tt:tt)*) => { todo!() } }
    /// App::default().run_window(|ctx| {
    ///     println!("starting app with window {:?}", ctx.window_id);
    ///     window! {
    ///         title = "Window 1";
    ///         content = text("Window 1");
    ///     }
    /// })   
    /// ```
    ///
    /// Which is a shortcut for:
    /// ```no_run
    /// # use zero_ui_core::app::App;
    /// # use zero_ui_core::window::WindowsExt;
    /// # macro_rules! window { ($($tt:tt)*) => { todo!() } }
    /// App::default().run(|ctx| {
    ///     ctx.services.windows().open(|ctx| {
    ///         println!("starting app with window {:?}", ctx.window_id);
    ///         window! {
    ///             title = "Window 1";
    ///             content = text("Window 1");
    ///         }
    ///     });
    /// })   
    /// ```
    fn run_window(self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static);
}
impl<E: AppExtension> AppRunWindowExt for AppExtended<E> {
    fn run_window(self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static) {
        self.run(|ctx| {
            ctx.services.windows().open(new_window);
        })
    }
}

/// Extension trait, adds [`open_window`](HeadlessAppWindowExt::open_window) to [`HeadlessApp`](app::HeadlessApp).
pub trait HeadlessAppWindowExt {
    /// Open a new headless window and returns the new window ID.
    ///
    /// The `new_window` argument is the [`WindowContext`] of the new window.
    ///
    /// Returns the [`WindowId`] of the new window.
    fn open_window(&mut self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static) -> WindowId;

    /// Cause the headless window to think it is focused in the screen.
    fn focus_window(&mut self, window_id: WindowId);
    /// Cause the headless window to think focus moved away from it.
    fn blur_window(&mut self, window_id: WindowId);

    /// Copy the current frame pixels of the window.
    fn frame_pixels(&mut self, window_id: WindowId) -> FramePixels;

    /// Sleeps until the next window frame is rendered, then returns the frame pixels.
    fn wait_frame(&mut self, window_id: WindowId) -> FramePixels;

    /// Sends a close request, returns if the window was found and closed.
    fn close_window(&mut self, window_id: WindowId) -> bool;
}
impl HeadlessAppWindowExt for app::HeadlessApp {
    fn open_window(&mut self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static) -> WindowId {
        let response = self.ctx().services.windows().open(new_window);
        let mut window_id = None;
        let cf = self.update_observe(
            |ctx| {
                if let Some(opened) = response.rsp_new(ctx) {
                    window_id = Some(opened.window_id);
                }
            },
            true,
        );

        window_id.unwrap_or_else(|| panic!("window did not open, ControlFlow: {:?}", cf))
    }

    fn focus_window(&mut self, window_id: WindowId) {
        let args = RawWindowFocusArgs::now(window_id, true);
        RawWindowFocusEvent.notify(self.ctx().events, args);
        let _ = self.update(false);
    }

    fn blur_window(&mut self, window_id: WindowId) {
        let args = RawWindowFocusArgs::now(window_id, false);
        RawWindowFocusEvent.notify(self.ctx().events, args);
        let _ = self.update(false);
    }

    fn wait_frame(&mut self, window_id: WindowId) -> FramePixels {
        // the current frame for comparison.
        let frame_id = self.ctx().services.windows().frame_info(window_id).ok().map(|w| w.frame_id());

        loop {
            if let ControlFlow::Exit = self.update(true) {
                return FramePixels::default();
            }

            let windows = self.ctx().services.windows();
            if let Ok(frame) = windows.frame_info(window_id) {
                if Some(frame.frame_id()) != frame_id {
                    return windows.frame_pixels(window_id).unwrap();
                }
            }
        }
    }

    fn frame_pixels(&mut self, window_id: WindowId) -> FramePixels {
        self.ctx().services.windows().frame_pixels(window_id).expect("window not found")
    }

    fn close_window(&mut self, window_id: WindowId) -> bool {
        use app::raw_events::*;

        let args = RawWindowCloseRequestedArgs::now(window_id);
        RawWindowCloseRequestedEvent.notify(self.ctx().events, args);

        let mut requested = false;
        let mut closed = false;

        let _ = self.update_observe_event(
            |_, args| {
                if let Some(args) = WindowCloseRequestedEvent.update(args) {
                    requested |= args.window_id == window_id;
                } else if let Some(args) = WindowCloseEvent.update(args) {
                    closed |= args.window_id == window_id;
                }
            },
            false,
        );

        assert_eq!(requested, closed);

        closed
    }
}

/// Window startup configuration.
///
/// More window configuration is accessible using the [`WindowVars`] type.
pub struct Window {
    state: OwnedStateMap,
    id: WidgetId,
    start_position: StartPosition,
    kiosk: bool,
    headless_monitor: HeadlessMonitor,
    child: BoxedUiNode,
}
impl Window {
    /// New window configuration.
    ///
    /// * `root_id` - Widget ID of `child`.
    /// * `start_position` - Position of the window when it first opens.
    /// * `kiosk` - Only allow full-screen mode. Note this does not configure the operating system, only blocks the app itself
    ///             from accidentally exiting full-screen. Also causes subsequent open windows to be child of this window.
    /// * `mode` - Custom window mode for this window only, set to default to use the app mode.
    /// * `headless_monitor` - "Monitor" configuration used in [headless mode](WindowMode::is_headless).
    /// * `child` - The root widget outermost node, the window sets-up the root widget using this and the `root_id`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        root_id: WidgetId,
        start_position: impl Into<StartPosition>,
        kiosk: bool,
        headless_monitor: impl Into<HeadlessMonitor>,
        child: impl UiNode,
    ) -> Self {
        Window {
            state: OwnedStateMap::default(),
            id: root_id,
            kiosk,
            start_position: start_position.into(),
            headless_monitor: headless_monitor.into(),
            child: child.boxed(),
        }
    }
}

/// "Monitor" configuration used by windows in [headless mode](WindowMode::is_headless).
#[derive(Clone)]
pub struct HeadlessMonitor {
    /// The scale factor used for the headless layout and rendering.
    ///
    /// `1.0` by default.
    pub scale_factor: f32,

    /// Size of the imaginary monitor screen that contains the headless window.
    ///
    /// This is used to calculate relative lengths in the window size definition and is defined in
    /// layout pixels instead of device like in a real monitor info.
    ///
    /// `(1920.0, 1080.0)` by default.
    pub size: LayoutSize,

    /// Pixel-per-inches used for the headless layout and rendering.
    ///
    /// [`Monitors::DEFAULT_PPI`] by default.
    pub ppi: f32,
}
impl fmt::Debug for HeadlessMonitor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() || about_eq(self.ppi, Monitors::DEFAULT_PPI, 0.001) {
            f.debug_struct("HeadlessMonitor")
                .field("scale_factor", &self.scale_factor)
                .field("screen_size", &self.size)
                .field("ppi", &self.ppi)
                .finish()
        } else {
            write!(f, "({}, ({}, {}))", self.scale_factor, self.size.width, self.size.height)
        }
    }
}
impl HeadlessMonitor {
    /// New with custom size at `1.0` scale.
    #[inline]
    pub fn new(size: LayoutSize) -> Self {
        Self::new_scaled(size, 1.0)
    }

    /// New with custom size and scale.
    #[inline]
    pub fn new_scaled(size: LayoutSize, scale_factor: f32) -> Self {
        HeadlessMonitor {
            scale_factor,
            size,
            ppi: Monitors::DEFAULT_PPI,
        }
    }

    /// New default size `1920x1080` and custom scale.
    #[inline]
    pub fn new_scale(scale_factor: f32) -> Self {
        HeadlessMonitor {
            scale_factor,
            ..Self::default()
        }
    }
}
impl Default for HeadlessMonitor {
    /// New `1920x1080` at `1.0` scale.
    fn default() -> Self {
        Self::new(LayoutSize::new(1920.0, 1080.0))
    }
}
impl IntoValue<HeadlessMonitor> for (f32, f32) {}
impl From<(f32, f32)> for HeadlessMonitor {
    /// Calls [`HeadlessMonitor::new_scaled`]
    fn from((width, height): (f32, f32)) -> Self {
        Self::new(LayoutSize::new(width, height))
    }
}
impl IntoValue<HeadlessMonitor> for (u32, u32) {}
impl From<(u32, u32)> for HeadlessMonitor {
    /// Calls [`HeadlessMonitor::new`]
    fn from((width, height): (u32, u32)) -> Self {
        Self::new(LayoutSize::new(width as f32, height as f32))
    }
}
impl IntoValue<HeadlessMonitor> for FactorNormal {}
impl From<FactorNormal> for HeadlessMonitor {
    /// Calls [`HeadlessMonitor::new_scale`]
    fn from(f: FactorNormal) -> Self {
        Self::new_scale(f.0)
    }
}
impl IntoValue<HeadlessMonitor> for FactorPercent {}
impl From<FactorPercent> for HeadlessMonitor {
    /// Calls [`HeadlessMonitor::new_scale`]
    fn from(f: FactorPercent) -> Self {
        Self::new_scale(f.0 / 100.0)
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

/// All information about a monitor that [`Monitors`] can provide.
pub struct MonitorFullInfo {
    /// Unique ID.
    pub id: MonitorId,
    /// Metadata from the operating system.
    pub info: MonitorInfo,
    /// PPI config var.
    pub ppi: RcVar<f32>,
}
impl fmt::Debug for MonitorFullInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MonitorFullInfo")
            .field("id", &self.id)
            .field("info", &self.info)
            .finish_non_exhaustive()
    }
}

/// A *selector* that returns a [`MonitorFullInfo`] from [`Monitors`].
#[derive(Clone)]
pub enum MonitorQuery {
    /// The primary monitor, if there is any monitor.
    Primary,
    /// Custom query closure.
    ///
    /// If the closure returns `None` the primary monitor is used, if there is any.
    Query(Rc<dyn Fn(&mut Monitors) -> Option<&MonitorFullInfo>>),
}
impl MonitorQuery {
    /// New query.
    #[inline]
    pub fn new(query: impl Fn(&mut Monitors) -> Option<&MonitorFullInfo> + 'static) -> Self {
        Self::Query(Rc::new(query))
    }

    /// Runs the query.
    #[inline]
    pub fn select<'a, 'b>(&'a self, monitors: &'b mut Monitors) -> Option<&'b MonitorFullInfo> {
        match self {
            MonitorQuery::Primary => None,
            MonitorQuery::Query(q) => q(monitors),
        }
    }
}
impl PartialEq for MonitorQuery {
    /// Returns `true` only if both are [`MonitorQuery::Primary`].
    fn eq(&self, other: &Self) -> bool {
        matches!((self, other), (Self::Primary, Self::Primary))
    }
}
impl Default for MonitorQuery {
    /// Returns [`MonitorQuery::Primary`].
    fn default() -> Self {
        Self::Primary
    }
}
impl fmt::Debug for MonitorQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MonitorQuery(Rc<..>)")
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

/// Window screen state.
#[derive(Clone, Copy, PartialEq)]
pub enum WindowState {
    /// A visible window, at the `position` and `size` configured.
    Normal,
    /// Window not visible, but maybe visible in the taskbar.
    Minimized,
    /// Window fills the screen, but window frame and taskbar are visible.
    Maximized,
    /// Window fully fills the screen, rendered using a frameless top-most window.
    Fullscreen,
    /// Exclusive video access to the monitor, only the window content is visible. TODO video config
    FullscreenExclusive,
}
impl fmt::Debug for WindowState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "WindowState::")?;
        }
        match self {
            WindowState::Normal => write!(f, "Normal"),
            WindowState::Minimized => write!(f, "Minimized"),
            WindowState::Maximized => write!(f, "Maximized"),
            WindowState::Fullscreen => write!(f, "Fullscreen"),
            WindowState::FullscreenExclusive => write!(f, "FullscreenExclusive"),
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
    /// A bitmap icon.
    ///
    /// Use the [`from_rgba`](Self::from_rgba), [`from_bytes`](Self::from_bytes) or [`from_file`](Self::from_file) functions to initialize.
    Icon(Rc<view_process::Icon>),
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
            WindowIcon::Icon(_) => write!(f, "Icon"),
            WindowIcon::Render(_) => write!(f, "Render"),
        }
    }
}
impl PartialEq for WindowIcon {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (WindowIcon::Default, WindowIcon::Default) => true,
            (WindowIcon::Icon(a), WindowIcon::Icon(b)) => Rc::ptr_eq(a, b),
            (WindowIcon::Render(a), WindowIcon::Render(b)) => Rc::ptr_eq(a, b),
            _ => false,
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
    /// New window icon from 32bpp RGBA data.
    ///
    /// The `rgba` must be a sequence of RGBA pixels in top-to-bottom rows.
    ///
    /// # Panics
    ///
    /// If `rgba.len()` is not `width * height * 4`.
    #[inline]
    pub fn from_rgba(rgba: Vec<u8>, width: u32, height: u32) -> Self {
        assert!(rgba.len() == width as usize * height as usize * 4);
        Self::Icon(Rc::new(zero_ui_vp::Icon { rgba, width, height }))
    }

    /// New window icon from the bytes of an encoded image.
    ///
    /// The image format is guessed, PNG is recommended. Loading is done using the [`image::load_from_memory`]
    /// function from the [`image`] crate.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, image::error::ImageError> {
        use image::GenericImageView;

        let image = image::load_from_memory(bytes)?;
        let (width, height) = image.dimensions();
        let icon = Self::from_rgba(image.into_bytes(), width, height);
        Ok(icon)
    }

    /// New window icon from image file.
    ///
    /// The image format is guessed from the path extension. Loading is done using the [`image::open`]
    /// function from the [`image`] crate.
    pub fn from_file<P: AsRef<std::path::Path>>(file: P) -> Result<Self, image::error::ImageError> {
        use image::GenericImageView;

        let image = image::open(file)?;
        let (width, height) = image.dimensions();
        let icon = Self::from_rgba(image.into_bytes(), width, height);
        Ok(icon)
    }

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
    /// [`WindowIcon::from_bytes`] but just logs errors.
    fn from(bytes: &'static [u8]) -> WindowIcon {
        WindowIcon::from_bytes(bytes).unwrap_or_else(|e| {
            log::error!(target: "window", "failed to load icon from encoded bytes: {:?}", e);
            WindowIcon::Default
        })
    }

    /// [`WindowIcon::from_rgba`] but validates the dimensions and length.
    fn from(rgba: (Vec<u8>, u32, u32)) -> WindowIcon {
        if rgba.1 as usize * rgba.2 as usize * 4 == rgba.0.len() {
            WindowIcon::from_rgba(rgba.0, rgba.1, rgba.2)
        } else {
            log::error!("invalid rgba data, length is not width * height * 4");
            WindowIcon::Default
        }
    }

    /// [`WindowIcon::from_file`] but just logs errors.
    fn from(file: &'static str) -> WindowIcon {
        WindowIcon::from_file(file).unwrap_or_else(|e| {
            log::error!(target: "window", "failed to load icon from file: {:?}", e);
            WindowIcon::Default
        })
    }

    /// [`WindowIcon::from_file`] but just logs errors.
    fn from(file: std::path::PathBuf) -> WindowIcon {
        WindowIcon::from_file(file).unwrap_or_else(|e| {
            log::error!(target: "window", "failed to load icon from file: {:?}", e);
            WindowIcon::Default
        })
    }
}
impl<const N: usize> From<&'static [u8; N]> for WindowIcon {
    /// [`WindowIcon::from_file`]
    fn from(bytes: &'static [u8; N]) -> Self {
        Self::from_bytes(bytes).unwrap_or_else(|e| {
            log::error!(target: "window", "failed to load icon from encoded bytes: {:?}", e);
            WindowIcon::Default
        })
    }
}
impl<const N: usize> crate::var::IntoVar<WindowIcon> for &'static [u8; N] {
    type Var = crate::var::OwnedVar<WindowIcon>;

    /// [`WindowIcon::from_file`]
    fn into_var(self) -> Self::Var {
        crate::var::OwnedVar(self.into())
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

    /// [`WindowFocusChangedEvent`], [`WindowFocusEvent`], [`WindowBlurEvent`] args.
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

    /// [`WindowResizeEvent`] args.
    pub struct WindowResizeArgs {
        /// Window ID.
        pub window_id: WindowId,
        /// New window size.
        pub new_size: LayoutSize,
        /// Who resized the window.
        pub cause: EventCause,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// [`WindowMoveEvent`] args.
    pub struct WindowMoveArgs {
        /// Window ID.
        pub window_id: WindowId,
        /// New window position.
        pub new_position: LayoutPoint,
        /// Who moved the window.
        pub cause: EventCause,

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
        pub new_scale_factor: f32,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }

    /// [`MonitorsChangedEvent`] args.
    pub struct MonitorsChangedArgs {
        /// Removed monitors.
        pub removed: Vec<MonitorId>,

        /// Added monitors.
        ///
        /// Use the [`Monitors`] service to get metadata about the added monitors.
        pub added: Vec<MonitorId>,

        ..

        /// Concerns every widget.
        fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
            true
        }
    }
}
cancelable_event_args! {
    /// [`WindowCloseRequestedEvent`] args.
    pub struct WindowCloseRequestedArgs {
        /// Window ID.
        pub window_id: WindowId,

        close_group: CloseGroupId,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }
}

event! {
    /// Window resized event.
    pub WindowResizeEvent: WindowResizeArgs;

    /// Window moved event.
    pub WindowMoveEvent: WindowMoveArgs;

    /// New window event.
    pub WindowOpenEvent: WindowOpenArgs;

    /// Window focus/blur event.
    pub WindowFocusChangedEvent: WindowFocusArgs;

    /// Window got keyboard focus event.
    pub WindowFocusEvent: WindowFocusArgs;

    /// Window lost keyboard event.
    pub WindowBlurEvent: WindowFocusArgs;

    /// Window scale factor changed.
    pub WindowScaleChangedEvent: WindowScaleChangedArgs;

    /// Closing window event.
    pub WindowCloseRequestedEvent: WindowCloseRequestedArgs;

    /// Close window event.
    pub WindowCloseEvent: WindowCloseArgs;

    /// Monitors added or removed event.
    pub MonitorsChangedEvent: MonitorsChangedArgs;
}

/// Application extension that manages windows.
///
/// # Events
///
/// Events this extension provides:
///
/// * [WindowOpenEvent]
/// * [WindowFocusChangedEvent]
/// * [WindowFocusEvent]
/// * [WindowBlurEvent]
/// * [WindowResizeEvent]
/// * [WindowMoveEvent]
/// * [WindowScaleChangedEvent]
/// * [WindowCloseRequestedEvent]
/// * [WindowCloseEvent]
/// * [MonitorsChangedEvent]
///
/// # Services
///
/// Services this extension provides:
///
/// * [Windows]
/// * [Monitors]
pub struct WindowManager {
    pending_closes: LinearMap<CloseGroupId, PendingClose>,
}
struct PendingClose {
    windows: LinearMap<WindowId, Option<bool>>,
    responder: ResponderVar<CloseWindowResult>,
}
impl Default for WindowManager {
    fn default() -> Self {
        Self {
            pending_closes: LinearMap::new(),
        }
    }
}
impl AppExtension for WindowManager {
    fn init(&mut self, ctx: &mut AppContext) {
        let monitors = Monitors::new(ctx.services.get::<ViewProcess>());
        ctx.services.register(monitors);
        ctx.services.register(Windows::new(ctx.updates.sender()));
    }

    fn event_preview<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        if let Some(args) = RawWindowFocusEvent.update(args) {
            let wns = ctx.services.windows();
            if let Some(window) = wns.windows_info.get_mut(&args.window_id) {
                if window.is_focused == args.focused {
                    return;
                }

                window.is_focused = args.focused;

                let args = WindowFocusArgs::new(args.timestamp, args.window_id, window.is_focused, false);

                WindowFocusChangedEvent.notify(ctx.events, args.clone());
                if args.focused {
                    WindowFocusEvent.notify(ctx.events, args);
                } else {
                    WindowBlurEvent.notify(ctx.events, args);
                }
            }
        } else if let Some(args) = RawWindowResizedEvent.update(args) {
            if let Some(mut window) = ctx.services.windows().windows.remove(&args.window_id) {
                if window.vars.0.actual_size.set_ne(ctx.vars, args.size) {
                    window.size = args.size;
                    // raise window_resize
                    WindowResizeEvent.notify(
                        ctx.events,
                        WindowResizeArgs::new(args.timestamp, args.window_id, args.size, args.cause),
                    );

                    if matches!(args.cause, EventCause::System) {
                        // the view process is waiting a new frame or update, this will send one.
                        window.on_resize_event(ctx, args.size);
                    }
                } else if matches!(args.cause, EventCause::System) {
                    log::warn!("received `RawWindowResizedEvent` with the same size, caused by system");
                    // view process is waiting a frame.
                    window.render_empty_update();
                }
                ctx.services.windows().windows.insert(args.window_id, window);
            }
        } else if let Some(args) = RawWindowMovedEvent.update(args) {
            if let Some(window) = ctx.services.windows().windows.get_mut(&args.window_id) {
                if window.vars.0.actual_position.set_ne(ctx.vars, args.position) {
                    WindowMoveEvent.notify(
                        ctx.events,
                        WindowMoveArgs::new(args.timestamp, args.window_id, args.position, args.cause),
                    );
                } else if matches!(args.cause, EventCause::System) {
                    log::warn!("received `RawWindowMovedEvent` with the same position, caused by system");
                }
            }
        } else if let Some(args) = RawWindowCloseRequestedEvent.update(args) {
            let _ = ctx.services.windows().close(args.window_id);
        } else if let Some(args) = RawWindowScaleFactorChangedEvent.update(args) {
            if ctx.services.windows().windows.contains_key(&args.window_id) {
                let args = WindowScaleChangedArgs::new(args.timestamp, args.window_id, args.scale_factor);
                WindowScaleChangedEvent.notify(ctx.events, args);
            }
        } else if let Some(args) = RawWindowCloseEvent.update(args) {
            if ctx.services.windows().windows.contains_key(&args.window_id) {
                log::error!("view-process closed window without request");
                let args = WindowCloseArgs::new(args.timestamp, args.window_id);
                WindowCloseEvent.notify(ctx, args);
            }
        } else if ViewProcessRespawnedEvent.update(args).is_some() {
            // `respawn` will force a `render` only and the `RenderContext` does not
            // give access to `services` so this is fine.
            let mut windows = mem::take(&mut ctx.services.windows().windows);

            for (_, w) in windows.iter_mut() {
                w.respawn(ctx);
            }

            ctx.services.windows().windows = windows;
        } else if let Some(args) = RawMonitorsChangedEvent.update(args) {
            let monitors = ctx.services.monitors();
            let ms: LinearMap<_, _> = args.available_monitors.iter().cloned().collect();
            let removed: Vec<_> = monitors.monitors.keys().filter(|k| !ms.contains_key(k)).copied().collect();
            let added: Vec<_> = ms.keys().filter(|k| !monitors.monitors.contains_key(k)).copied().collect();

            for key in &removed {
                monitors.monitors.remove(key);
            }
            for key in &added {
                monitors.monitors.insert(
                    *key,
                    MonitorFullInfo {
                        id: *key,
                        info: ms.get(key).cloned().unwrap(),
                        ppi: var(Monitors::DEFAULT_PPI),
                    },
                );
            }

            if !removed.is_empty() || !added.is_empty() {
                let args = MonitorsChangedArgs::new(args.timestamp, removed, added);
                MonitorsChangedEvent.notify(ctx, args);
            }
        }
    }

    fn event_ui<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        with_detached_windows(ctx, |ctx, windows| {
            for (_, w) in windows.iter_mut() {
                w.event(ctx, args);
            }
        })
    }

    fn event<EV: event::EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        if let Some(args) = WindowCloseRequestedEvent.update(args) {
            // If we caused this event, fulfill the close request.
            match self.pending_closes.entry(args.close_group) {
                linear_map::Entry::Occupied(mut e) => {
                    let caused_by_us = if let Some(canceled) = e.get_mut().windows.get_mut(&args.window_id) {
                        // caused by us, update the status for the window.
                        *canceled = Some(args.cancel_requested());
                        true
                    } else {
                        // not us, window not in group
                        false
                    };

                    if caused_by_us {
                        // check if this is the last window in the group
                        let mut all_some = true;
                        // and if any cancelled we cancel all, otherwise close all.
                        let mut cancel = false;

                        for cancel_flag in e.get().windows.values() {
                            if let Some(c) = cancel_flag {
                                cancel |= c;
                            } else {
                                all_some = false;
                                break;
                            }
                        }

                        if all_some {
                            // if the last window in the group, no longer pending
                            let e = e.remove();

                            if cancel {
                                // respond to all windows in the group.
                                e.responder.respond(ctx, CloseWindowResult::Cancel);
                            } else {
                                e.responder.respond(ctx, CloseWindowResult::Closed);

                                // notify close, but does not remove then yet, this
                                // lets the window content handle the close event,
                                // we deinit the window when we handle our own close event.
                                let windows = ctx.services.windows();
                                for (w, _) in e.windows {
                                    if windows.windows.contains_key(&w) {
                                        WindowCloseEvent.notify(ctx.events, WindowCloseArgs::now(w));
                                    }
                                }
                            }
                        }
                    }
                }
                linear_map::Entry::Vacant(_) => {
                    // Not us, no pending entry.
                }
            }
        } else if let Some(args) = WindowCloseEvent.update(args) {
            // finish close, this notifies  `UiNode::deinit` and drops the window
            // causing the ViewWindow to drop and close.

            if let Some(w) = ctx.services.windows().windows.remove(&args.window_id) {
                w.deinit(ctx);
                let wns = ctx.services.windows();
                let info = wns.windows_info.remove(&args.window_id).unwrap();

                info.vars.0.is_open.set(ctx.vars, false);

                if wns.shutdown_on_last_close && wns.windows.is_empty() && wns.open_requests.is_empty() {
                    // fulfill `shutdown_on_last_close`
                    ctx.services.app_process().shutdown();
                }

                if info.is_focused {
                    let args = WindowFocusArgs::now(info.id, false, true);
                    WindowFocusChangedEvent.notify(ctx.events, args)
                }
            }
        }
    }

    fn update_ui(&mut self, ctx: &mut AppContext) {
        let (open, close) = ctx.services.windows().take_requests();

        // fulfill open requests.
        for r in open {
            let (w, info) = AppWindow::new(ctx, r.new, r.force_headless);
            let args = WindowOpenArgs::now(w.id);
            {
                let wns = ctx.services.windows();
                wns.windows.insert(w.id, w);
                wns.windows_info.insert(info.id, info);
            }

            r.responder.respond(ctx, args.clone());
            WindowOpenEvent.notify(ctx, args);
        }

        // notify close requests, the request is fulfilled or canceled
        // in the `event` handler.
        for (w_id, r) in close {
            let args = WindowCloseRequestedArgs::now(w_id, r.group);
            WindowCloseRequestedEvent.notify(ctx.events, args);

            self.pending_closes
                .entry(r.group)
                .or_insert_with(|| PendingClose {
                    responder: r.responder,
                    windows: LinearMap::with_capacity(1),
                })
                .windows
                .insert(w_id, None);
        }

        // notify content
        with_detached_windows(ctx, |ctx, windows| {
            for (_, w) in windows.iter_mut() {
                w.update(ctx);
            }
        });
    }

    fn update_display(&mut self, ctx: &mut AppContext, r: UpdateDisplayRequest) {
        with_detached_windows(ctx, |ctx, windows| {
            for (_, w) in windows.iter_mut() {
                w.on_layout(ctx, r);
                w.on_render(ctx, r);
                w.on_render_update(ctx);
            }
        });
    }
}

/// Takes ownership of [`Windows::windows`] for the duration of the call to `f`.
///
/// The windows map is empty for the duration of `f` and should not be used, this is for
/// mutating the window content while still allowing it to query the [`Windows::windows_info`].
fn with_detached_windows(ctx: &mut AppContext, f: impl FnOnce(&mut AppContext, &mut LinearMap<WindowId, AppWindow>)) {
    let mut windows = mem::take(&mut ctx.services.windows().windows);
    f(ctx, &mut windows);
    let mut wns = ctx.services.windows();
    debug_assert!(wns.windows.is_empty());
    wns.windows = windows;
}

/// Monitors service.
///
/// List monitor screens and configure the PPI of a given monitor.
///
/// # Uses
///
/// Uses of this service:
///
/// ## Start Position
///
/// Windows are positioned on a *virtual screen* that overlaps all monitors, but for the window start
/// position the user may want to select a specific monitor, this service is used to provide information
/// to implement this feature.
///
/// See [The Virtual Screen] for more information about this in the Windows OS.
///
/// ## Full-Screen
///
/// To set a window to full-screen a monitor must be selected, by default it can be the one the window is at but
/// the users may want to select a monitor. To enter full-screen exclusive the video mode must be decided also, all video
/// modes supported by the monitor are available in the [`MonitorInfo`] value.
///
/// ## Real-Size Preview
///
/// Some apps, like image editors, may implement a feature where the user can preview the *real* dimensions of
/// the content they are editing, to accurately implement this you must known the real dimensions of the monitor screen,
/// unfortunately this information is not provided by monitor devices. You can ask the user to measure their screen and
/// set the **pixel-per-inch** ratio for the screen using [`ppi`] variable, this value is then available in the [`LayoutMetrics`]
/// for the next layout. If not set, the default is `96.0ppi`.
///
/// # Provider
///
/// This service is provided by the [`WindowManager`].
///
/// [`ppi`]: MonitorFullInfo::ppi
/// [`LayoutMetrics`]: crate::context::LayoutMetrics
/// [The Virtual Screen]: https://docs.microsoft.com/en-us/windows/win32/gdi/the-virtual-screen
#[derive(Service)]
pub struct Monitors {
    monitors: LinearMap<MonitorId, MonitorFullInfo>,
}
impl Monitors {
    /// Initial PPI of monitors, `96.0`.
    pub const DEFAULT_PPI: f32 = 96.0;

    fn new(view: Option<&mut ViewProcess>) -> Self {
        Monitors {
            monitors: view
                .and_then(|v| v.available_monitors().ok())
                .map(|ms| {
                    ms.into_iter()
                        .map(|(id, info)| {
                            (
                                id,
                                MonitorFullInfo {
                                    id,
                                    info,
                                    ppi: var(Self::DEFAULT_PPI),
                                },
                            )
                        })
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    /// Reference the primary monitor.
    ///
    /// Returns `None` if no monitor was identified as the primary.
    pub fn primary_monitor(&mut self) -> Option<&MonitorFullInfo> {
        self.monitors.values().find(|m| m.info.is_primary)
    }

    /// Reference the monitor info.
    ///
    /// Returns `None` if the monitor was not found or the app is running in headless mode without renderer.
    pub fn monitor(&mut self, monitor_id: MonitorId) -> Option<&MonitorFullInfo> {
        self.monitors.get(&monitor_id)
    }

    /// Iterate over all available monitors.
    ///
    /// Each item is `(ID, info, PPI)`.
    ///
    /// Is empty if no monitor was found or the app is running in headless mode without renderer.
    pub fn available_monitors(&mut self) -> impl Iterator<Item = &MonitorFullInfo> {
        self.monitors.values()
    }
}

/// Windows service.
///
/// # Provider
///
/// This service is provided by the [`WindowManager`].
#[derive(Service)]
pub struct Windows {
    /// If shutdown is requested when a window closes and there are no more windows open, `true` by default.
    pub shutdown_on_last_close: bool,

    windows: LinearMap<WindowId, AppWindow>,
    windows_info: LinearMap<WindowId, AppWindowInfo>,

    open_requests: Vec<OpenWindowRequest>,
    update_sender: AppEventSender,

    close_group_id: CloseGroupId,
    close_requests: LinearMap<WindowId, CloseWindowRequest>,
}
impl Windows {
    fn new(update_sender: AppEventSender) -> Self {
        Windows {
            shutdown_on_last_close: true,
            windows: LinearMap::with_capacity(1),
            windows_info: LinearMap::with_capacity(1),
            open_requests: Vec::with_capacity(1),
            update_sender,

            close_group_id: 1,
            close_requests: LinearMap::new(),
        }
    }

    // Requests a new window.
    ///
    /// The `new_window` argument is the [`WindowContext`] of the new window.
    ///
    /// Returns a listener that will update once when the window is opened, note that while the `window_id` is
    /// available in the `new_window` argument already, the window is only available in this service after
    /// the returned listener updates.
    pub fn open(&mut self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static) -> ResponseVar<WindowOpenArgs> {
        self.open_impl(new_window, None)
    }

    /// Requests a new headless window.
    ///
    /// Headless windows don't show on screen, but if `with_renderer` is `true` they will still render frames.
    ///
    /// Note that in a headless app the [`open`] method also creates headless windows, this method
    /// creates headless windows even in a headed app.
    ///
    /// [`open`]: Windows::open
    pub fn open_headless(
        &mut self,
        new_window: impl FnOnce(&mut WindowContext) -> Window + 'static,
        with_renderer: bool,
    ) -> ResponseVar<WindowOpenArgs> {
        self.open_impl(
            new_window,
            Some(if with_renderer {
                WindowMode::HeadlessWithRenderer
            } else {
                WindowMode::Headless
            }),
        )
    }

    fn open_impl(
        &mut self,
        new_window: impl FnOnce(&mut WindowContext) -> Window + 'static,
        force_headless: Option<WindowMode>,
    ) -> ResponseVar<WindowOpenArgs> {
        let (responder, response) = response_var();
        let request = OpenWindowRequest {
            new: Box::new(new_window),
            force_headless,
            responder,
        };
        self.open_requests.push(request);
        let _ = self.update_sender.send_update();

        response
    }

    /// Starts closing a window, the operation can be canceled by listeners of
    /// [`WindowCloseRequestedEvent`].
    ///
    /// Returns a response var that will update once with the result of the operation.
    pub fn close(&mut self, window_id: WindowId) -> Result<ResponseVar<CloseWindowResult>, WindowNotFound> {
        if self.windows_info.contains_key(&window_id) {
            let (responder, response) = response_var();

            let group = self.close_group_id.wrapping_add(1);
            self.close_group_id = group;

            self.close_requests.insert(window_id, CloseWindowRequest { responder, group });
            let _ = self.update_sender.send_update();

            Ok(response)
        } else {
            Err(WindowNotFound(window_id))
        }
    }

    /// Requests closing multiple windows together, the operation can be canceled by listeners of the
    /// [`WindowCloseRequestedEvent`]. If canceled none of the windows are closed.
    ///
    /// Returns a response var that will update once with the result of the operation. Returns
    /// [`Cancel`] if `windows` is empty or contains a window that already
    /// requested close during this update.
    ///
    /// [`Cancel`]: CloseWindowResult::Cancel
    pub fn close_together(
        &mut self,
        windows: impl IntoIterator<Item = WindowId>,
    ) -> Result<ResponseVar<CloseWindowResult>, WindowNotFound> {
        let windows = windows.into_iter();
        let mut requests = LinearMap::with_capacity(windows.size_hint().0);

        let group = self.close_group_id.wrapping_add(1);
        self.close_group_id = group;

        let (responder, response) = response_var();

        for window in windows {
            if !self.windows_info.contains_key(&window) {
                return Err(WindowNotFound(window));
            }

            requests.insert(
                window,
                CloseWindowRequest {
                    responder: responder.clone(),
                    group,
                },
            );
        }

        self.close_requests.extend(requests);
        let _ = self.update_sender.send_update();

        Ok(response)
    }

    /// Get the window [mode].
    ///
    /// This value indicates if the window is headless or not.
    ///
    /// [mode]: WindowMode
    pub fn mode(&self, window_id: WindowId) -> Result<WindowMode, WindowNotFound> {
        self.windows_info.get(&window_id).map(|w| w.mode).ok_or(WindowNotFound(window_id))
    }

    /// Reference the metadata about the window's latest frame.
    pub fn frame_info(&self, window_id: WindowId) -> Result<&FrameInfo, WindowNotFound> {
        self.windows_info
            .get(&window_id)
            .map(|w| &w.frame_info)
            .ok_or(WindowNotFound(window_id))
    }

    /// Copy the pixels of the window's latest frame.
    ///
    /// Returns an empty zero-by-zero frame if the window is headless without renderer.
    pub fn frame_pixels(&self, window_id: WindowId) -> Result<FramePixels, WindowNotFound> {
        self.windows_info
            .get(&window_id)
            .ok_or(WindowNotFound(window_id))? // not found here
            .renderer
            .as_ref()
            .map(|r| r.read_pixels().map(Into::into))
            .unwrap_or_else(|| Ok(FramePixels::default())) // no renderer
            .map_err(|_| WindowNotFound(window_id)) // not found in view
    }

    /// Copy a rectangle of pixels of the window's latest frame.
    ///
    /// The `rect` is converted to pixels coordinates using the current window's scale factor.
    pub fn frame_pixels_rect(&self, window_id: WindowId, rect: impl Into<LayoutRect>) -> Result<FramePixels, WindowNotFound> {
        self.windows_info
            .get(&window_id)
            .ok_or(WindowNotFound(window_id))? // not found here
            .renderer
            .as_ref()
            .map(|r| r.read_pixels_rect(rect.into()).map(Into::into))
            .unwrap_or_else(|| Ok(FramePixels::default())) // no renderer
            .map_err(|_| WindowNotFound(window_id)) // not found in view
    }

    /// Reference the [`WindowVars`] for the window.
    pub fn vars(&self, window_id: WindowId) -> Result<&WindowVars, WindowNotFound> {
        self.windows_info.get(&window_id).map(|w| &w.vars).ok_or(WindowNotFound(window_id))
    }

    /// Hit-test the latest window frame.
    pub fn hit_test(&self, window_id: WindowId, point: LayoutPoint) -> Result<FrameHitInfo, WindowNotFound> {
        self.windows_info
            .get(&window_id)
            .map(|w| w.hit_test(point))
            .ok_or(WindowNotFound(window_id))
    }

    /// Gets if the window is focused in the OS.
    pub fn is_focused(&self, window_id: WindowId) -> Result<bool, WindowNotFound> {
        self.windows_info
            .get(&window_id)
            .map(|w| w.is_focused)
            .ok_or(WindowNotFound(window_id))
    }

    /// Iterate over the latest frames of each open window.
    pub fn frames(&self) -> impl Iterator<Item = &FrameInfo> {
        self.windows_info.values().map(|w| &w.frame_info)
    }

    /// Gets the current window scale factor.
    pub fn scale_factor(&self, window_id: WindowId) -> Result<f32, WindowNotFound> {
        self.windows_info
            .get(&window_id)
            .map(|w| w.scale_factor)
            .ok_or(WindowNotFound(window_id))
    }

    /// Gets the id of the window that is focused in the OS.
    pub fn focused_window_id(&self) -> Option<WindowId> {
        self.windows_info.values().find(|w| w.is_focused).map(|w| w.id)
    }

    /// Gets the latest frame for the focused window.
    pub fn focused_frame(&self) -> Option<&FrameInfo> {
        self.windows_info.values().find(|w| w.is_focused).map(|w| &w.frame_info)
    }

    fn take_requests(&mut self) -> (Vec<OpenWindowRequest>, LinearMap<WindowId, CloseWindowRequest>) {
        (mem::take(&mut self.open_requests), mem::take(&mut self.close_requests))
    }
}
struct OpenWindowRequest {
    new: Box<dyn FnOnce(&mut WindowContext) -> Window>,
    force_headless: Option<WindowMode>,
    responder: ResponderVar<WindowOpenArgs>,
}

struct CloseWindowRequest {
    responder: ResponderVar<CloseWindowResult>,
    group: CloseGroupId,
}

type CloseGroupId = u32;

/// Response message of [`close`](Windows::close) and [`close_together`](Windows::close_together).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloseWindowResult {
    /// Operation completed, all requested windows closed.
    Closed,

    /// Operation canceled, no window closed.
    Cancel,
}

/// Error when a [`WindowId`] is not opened by the [`Windows`] service.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct WindowNotFound(pub WindowId);
impl fmt::Display for WindowNotFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "window `{}` not found", self.0)
    }
}
impl std::error::Error for WindowNotFound {}

/// [`AppWindow`] data, detached so we can make the window visible in [`Windows`]
/// from inside the window content.
struct AppWindowInfo {
    id: WindowId,
    mode: WindowMode,
    renderer: Option<ViewRenderer>,
    vars: WindowVars,
    scale_factor: f32,

    // latest frame.
    frame_info: FrameInfo,
    // focus tracked by the raw focus events.
    is_focused: bool,
}
impl AppWindowInfo {
    fn hit_test(&self, point: LayoutPoint) -> FrameHitInfo {
        if let Some(r) = &self.renderer {
            match r.hit_test(point) {
                Ok(h) => {
                    return FrameHitInfo::new(self.id, self.frame_info.frame_id(), point, h);
                }
                Err(e) => log::error!("failed `hit_test`, will return `no_hits`, {:?}", e),
            }
        }

        FrameHitInfo::no_hits(self.id)
    }
}

/// An open window.
struct AppWindow {
    // Is some if the window is headed.
    headed: Option<ViewWindow>,
    // Is some if the window is headless, a fake screen for size calculations.
    headless_monitor: Option<HeadlessMonitor>,

    // Is some if the window is headed or headless with renderer.
    renderer: Option<ViewRenderer>,

    // Window context.
    context: OwnedWindowContext,

    // copy of some `context` values.
    mode: WindowMode,
    id: WindowId,
    root_id: WidgetId,
    kiosk: bool,

    vars: WindowVars,

    first_update: bool,
    first_render: bool,

    // latest frame.
    frame_id: FrameId,

    position: LayoutPoint,
    size: LayoutSize,
    min_size: LayoutSize,
    max_size: LayoutSize,

    deinited: bool,
}
impl AppWindow {
    fn new(
        ctx: &mut AppContext,
        new_window: Box<dyn FnOnce(&mut WindowContext) -> Window>,
        force_headless: Option<WindowMode>,
    ) -> (Self, AppWindowInfo) {
        // get mode.
        let mode = match (ctx.mode(), force_headless) {
            (WindowMode::Headed | WindowMode::HeadlessWithRenderer, Some(mode)) => {
                debug_assert!(!matches!(mode, WindowMode::Headed));
                mode
            }
            (mode, _) => mode,
        };

        // init vars.
        let vars = WindowVars::new();
        let mut wn_state = OwnedStateMap::default();
        wn_state.set(WindowVarsKey, vars.clone());

        // init root.
        let id = WindowId::new_unique();
        let root = ctx.window_context(id, mode, &mut wn_state, &None, new_window).0;
        let root_id = root.id;

        let headless_monitor = if matches!(mode, WindowMode::Headless) {
            Some(root.headless_monitor.clone())
        } else {
            None
        };

        let kiosk = root.kiosk;

        // init context.
        let context = OwnedWindowContext {
            window_id: id,
            mode,
            root_transform_key: WidgetTransformKey::new_unique(),
            state: wn_state,
            root,
            update: UpdateDisplayRequest::Layout,
        };

        // we want the window content to init, update, layout & render to get
        // all the values needed to actually spawn a real window, this is so we
        // have a frame ready to show when the window is visible.
        ctx.updates.update();
        ctx.updates.layout();

        let frame_info = FrameInfo::blank(id, root_id);

        let win = AppWindow {
            headed: None, // headed & renderer will initialize on first render.
            renderer: None,
            headless_monitor,
            context,
            mode,
            id,
            root_id,
            kiosk,
            vars: vars.clone(),

            first_update: true,
            first_render: true,

            frame_id: frame_info.frame_id(),
            position: LayoutPoint::new(f32::NAN, f32::NAN), // we use NAN to mean OS start position.
            size: LayoutSize::zero(),
            min_size: LayoutSize::zero(),
            max_size: LayoutSize::zero(),

            deinited: false,
        };
        let info = AppWindowInfo {
            id,
            mode,
            renderer: None, // will be set on the first render
            vars,
            scale_factor: 1.0, // will be set on the first layout
            frame_info,        // TODO
            is_focused: false, // will be set by listening to RawWindowFocusEvent, usually in first render
        };

        (win, info)
    }

    fn event<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        self.context.event(ctx, args);
    }

    fn update(&mut self, ctx: &mut AppContext) {
        if self.first_update {
            self.context.init(ctx);
            self.first_update = false;
        } else {
            self.context.update(ctx);

            if self.kiosk {
                todo!()
            }

            if self.vars.size().is_new(ctx)
                || self.vars.auto_size().is_new(ctx)
                || self.vars.min_size().is_new(ctx)
                || self.vars.max_size().is_new(ctx)
            {
                self.on_size_vars_update(ctx);
            }

            if self.vars.position().is_new(ctx) && !self.first_render {
                self.position = self.layout_position(ctx);

                if let Some(w) = &self.headed {
                    w.set_position(self.position).unwrap();
                } else {
                    RawWindowMovedEvent.notify(ctx.events, RawWindowMovedArgs::now(self.id, self.position, EventCause::App));
                }
            }

            if let Some(w) = &self.headed {
                if let Some(monitor) = self.vars.monitor().get_new(ctx.vars) {
                    let monitor_info = monitor.select(ctx.services.monitors());

                    if let Some(pos) = self.vars.position().get_new(ctx.vars) {
                        todo!("use pos, else center {:?}", pos)
                    }

                    if let Some(size) = self.vars.size().get_new(ctx.vars) {
                        todo!("use new size, else actual_size {:?}", size)
                    }

                    todo!("refresh monitor {:?}", monitor_info);
                }

                if let Some(title) = self.vars.title().get_new(ctx) {
                    w.set_title(title.to_string()).unwrap();
                }
                if let Some(chrome) = self.vars.chrome().get_new(ctx) {
                    match chrome {
                        WindowChrome::Default => w.set_chrome_visible(true).unwrap(),
                        WindowChrome::None => w.set_chrome_visible(false).unwrap(),
                        WindowChrome::Custom => {
                            w.set_chrome_visible(false).unwrap();
                            todo!();
                        }
                    }
                }
                if let Some(ico) = self.vars.icon().get_new(ctx) {
                    match ico {
                        WindowIcon::Default => w.set_icon(None).unwrap(),
                        WindowIcon::Icon(ico) => w.set_icon(Some(view_process::Icon::clone(ico))).unwrap(),
                        WindowIcon::Render(_) => todo!(),
                    }
                }
                if let Some(state) = self.vars.state().get_new(ctx) {
                    todo!("apply {:?}", state);
                }
                if let Some(resizable) = self.vars.resizable().copy_new(ctx) {
                    w.set_resizable(resizable).unwrap();
                }
                if let Some(movable) = self.vars.movable().copy_new(ctx) {
                    w.set_movable(movable).unwrap();
                }
                if let Some(always_on_top) = self.vars.always_on_top().copy_new(ctx) {
                    w.set_always_on_top(always_on_top).unwrap();
                }
                if let Some(visible) = self.vars.visible().copy_new(ctx) {
                    w.set_visible(visible).unwrap();
                }
                if let Some(visible) = self.vars.taskbar_visible().copy_new(ctx) {
                    w.set_taskbar_visible(visible).unwrap();
                }
                if self.vars.parent().is_new(ctx) || self.vars.modal().is_new(ctx) {
                    w.set_parent(self.vars.parent().copy(ctx), self.vars.modal().copy(ctx)).unwrap();
                }
                if let Some(transparent) = self.vars.transparent().copy_new(ctx) {
                    w.set_transparent(transparent).unwrap();
                }
                if let Some(allow) = self.vars.allow_alt_f4().copy_new(ctx) {
                    w.set_allow_alt_f4(allow).unwrap();
                }
            }

            if let Some(r) = &self.renderer {
                if let Some(text_aa) = self.vars.text_aa().copy_new(ctx) {
                    r.set_text_aa(text_aa).unwrap();
                }
            }
        }
    }

    /// (monitor_size, scale_factor, ppi)
    fn monitor_metrics(&mut self, ctx: &mut AppContext) -> (LayoutSize, f32, f32) {
        if let WindowMode::Headed = self.mode {
            // TODO only query monitors in the first layout and after `monitor` updates only.

            // try `actual_monitor`
            if let Some(id) = self.vars.actual_monitor().copy(ctx) {
                if let Some(m) = ctx.services.monitors().monitor(id) {
                    return (m.info.layout_size(), m.info.scale_factor, m.ppi.copy(ctx.vars));
                }
            }

            // try `monitor`, TODO set `actual_monitor` here?
            {
                let query = self.vars.monitor().get(ctx.vars);
                if let Some(m) = query.select(ctx.services.monitors()) {
                    return (m.info.layout_size(), m.info.scale_factor, m.ppi.copy(ctx.vars));
                }
            }

            // fallback to primary monitor.
            if let Some(p) = ctx.services.monitors().primary_monitor() {
                return (p.info.layout_size(), p.info.scale_factor, p.ppi.copy(ctx.vars));
            }

            // fallback to headless defaults.
            let h = self.headless_monitor.clone().unwrap_or_default();
            (h.size, h.scale_factor, h.ppi)
        } else {
            let scr = self.headless_monitor.as_ref().unwrap();
            (scr.size, scr.scale_factor, scr.ppi)
        }
    }

    /// On resize we need to re-layout, render and send a frame render quick, because
    /// the view process blocks for up to one second waiting the new frame, to reduce the
    /// chances of the user seeing the clear_color when resizing.
    fn on_resize_event(&mut self, ctx: &mut AppContext, actual_size: LayoutSize) {
        let (_scr_size, scr_factor, scr_ppi) = self.monitor_metrics(ctx);
        self.context.layout(ctx, 16.0, scr_factor, scr_ppi, actual_size, |_| actual_size);
        // the frame is send using the normal request
        self.on_render(ctx, UpdateDisplayRequest::ForceRender);
    }

    /// On any of the variables involved in sizing updated.
    ///
    /// Do measure/arrange, and if sizes actually changed send resizes.
    fn on_size_vars_update(&mut self, ctx: &mut AppContext) {
        if self.first_render {
            // values will be used in first-layout.
            return;
        }

        // `size` var is only used on init or once after update AND if auto_size did not override it.
        let use_system_size = self.vars.size().is_new(ctx.vars);
        let (size, min_size, max_size) = self.layout_size(ctx, use_system_size);

        if self.size != size {
            let frame = self.render_frame(ctx);

            // resize view
            self.size = size;
            if let Some(w) = &self.headed {
                let _ = w.set_size(size, frame);
            } else if let Some(_r) = &self.renderer {
                // TODO resize headless renderer.
                todo!()
            } else {
                // headless "resize"
                RawWindowResizedEvent.notify(ctx.events, RawWindowResizedArgs::now(self.id, self.size, EventCause::App));
            }
            // the `actual_size` is set from the resize event only.
        }

        // after potential resize, so we don't cause a resize from system
        // because winit coerces sizes.
        if self.min_size != min_size {
            self.min_size = min_size;
            if let Some(w) = &self.headed {
                let _ = w.set_min_size(min_size);
            }
        }
        if self.max_size != max_size {
            self.max_size = max_size;
            if let Some(w) = &self.headed {
                let _ = w.set_max_size(max_size);
            }
        }
    }

    /// On layout request can go two ways, if auto-size is enabled we will end-up resizing the window (probably)
    /// in this case we also render to send the frame together with the resize request, otherwise we just do layout
    /// and then wait for the normal render request.
    fn on_layout(&mut self, ctx: &mut AppContext, request: UpdateDisplayRequest) {
        if !request.in_window(self.context.update).is_layout() {
            return;
        }

        if self.first_render {
            self.on_init_layout(ctx);
            return;
        }

        // layout using the "system" size, it can still be overwritten by auto_size.
        let (size, _, _) = self.layout_size(ctx, true);

        if self.size != size {
            let frame = self.render_frame(ctx);

            self.size = size;
            if let Some(w) = &self.headed {
                let _ = w.set_size(size, frame);
            } else if let Some(_r) = &self.renderer {
                // TODO resize headless renderer.
                todo!()
            } else {
                // headless "resize"
                RawWindowResizedEvent.notify(ctx.events, RawWindowResizedArgs::now(self.id, self.size, EventCause::App));
            }
            // the `actual_size` is set from the resize event only.
        }
    }

    /// `on_layout` requested before the first frame render.
    fn on_init_layout(&mut self, ctx: &mut AppContext) {
        debug_assert!(self.first_render);

        let (final_size, min_size, max_size) = self.layout_size(ctx, false);

        self.size = final_size;
        self.min_size = min_size;
        self.max_size = max_size;

        // compute start position.
        match self.context.root.start_position {
            StartPosition::Default => {
                // `layout_position` can return `NAN` or a computed position.
                // We use NAN to signal the view-process to let the OS define the start position.
                self.position = self.layout_position(ctx);
            }
            StartPosition::CenterMonitor => {
                let (scr_size, _, _) = self.monitor_metrics(ctx);
                self.position.x = (scr_size.width - self.size.width) / 2.0;
                self.position.y = (scr_size.height - self.size.height) / 2.0;
            }
            StartPosition::CenterParent => todo!(),
        }

        // `on_render` will complete first_render.
        self.context.update = UpdateDisplayRequest::Render;
    }

    /// Calculate the position var in the current monitor.
    fn layout_position(&mut self, ctx: &mut AppContext) -> LayoutPoint {
        let (scr_size, scr_factor, scr_ppi) = self.monitor_metrics(ctx);

        ctx.outer_layout_context(scr_size, scr_factor, scr_ppi, self.id, self.root_id, |ctx| {
            self.vars.position().get(ctx.vars).to_layout(scr_size, ctx)
        })
    }

    /// Measure and arrange the content, returns the final, min and max sizes.
    ///
    /// If `use_system_size` is `true` the `size` variable is ignored.
    fn layout_size(&mut self, ctx: &mut AppContext, use_system_size: bool) -> (LayoutSize, LayoutSize, LayoutSize) {
        let (scr_size, scr_factor, scr_ppi) = self.monitor_metrics(ctx);

        let (available_size, min_size, max_size, auto_size) =
            ctx.outer_layout_context(scr_size, scr_factor, scr_ppi, self.id, self.root_id, |ctx| {
                let mut size = if use_system_size {
                    self.size
                } else {
                    self.vars.size().get(ctx.vars).to_layout(scr_size, ctx)
                };
                let min_size = self.vars.min_size().get(ctx.vars).to_layout(scr_size, ctx);
                let max_size = self.vars.max_size().get(ctx.vars).to_layout(scr_size, ctx);

                let auto_size = self.vars.auto_size().copy(ctx);
                if auto_size.contains(AutoSize::CONTENT_WIDTH) {
                    size.width = max_size.width;
                } else {
                    size.width = size.width.max(min_size.width).min(max_size.width);
                }
                if auto_size.contains(AutoSize::CONTENT_HEIGHT) {
                    size.height = max_size.height;
                } else {
                    size.height = size.height.max(min_size.height).min(max_size.height);
                }

                (size, min_size, max_size, auto_size)
            });

        let final_size = self.context.layout(ctx, 16.0, scr_factor, scr_ppi, scr_size, |desired_size| {
            let mut final_size = available_size;
            if auto_size.contains(AutoSize::CONTENT_WIDTH) {
                final_size.width = desired_size.width.max(min_size.width).min(available_size.width);
            }
            if auto_size.contains(AutoSize::CONTENT_HEIGHT) {
                final_size.height = desired_size.height.max(min_size.height).min(available_size.height);
            }
            final_size
        });

        (final_size, min_size, max_size)
    }

    /// Render frame for sending.
    ///
    /// The FrameId and FrameInfo are up-dated, but the FrameRequest is not send.
    fn render_frame(&mut self, ctx: &mut AppContext) -> view_process::FrameRequest {
        let scale_factor = self.monitor_metrics(ctx).1;
        let next_frame_id = self.frame_id.0.wrapping_add(1);
        let next_frame_id = if next_frame_id == u32::MAX {
            webrender_api::Epoch(0)
        } else {
            webrender_api::Epoch(next_frame_id)
        };

        // `UiNode::render`
        let ((pipeline_id, size, display_list), frame_info) =
            self.context.render(ctx, next_frame_id, self.size, scale_factor, &self.renderer);

        // update frame info.
        self.frame_id = frame_info.frame_id();
        let w_info = ctx.services.windows().windows_info.get_mut(&self.id).unwrap();

        //let fps = 1.secs().as_nanos() / (frame_info.timestamp() - w_info.frame_info.timestamp()).as_nanos();
        //println!("fps: {}", fps);

        w_info.frame_info = frame_info;

        // already notify, extensions are interested only in the frame metadata.
        ctx.updates.new_frame_rendered(self.id, self.frame_id);

        view_process::FrameRequest {
            id: self.frame_id,
            pipeline_id,
            size,
            display_list: display_list.into_data(),
        }
    }

    /// On render request.
    ///
    /// If there is a pending request we generate the frame and send.
    fn on_render(&mut self, ctx: &mut AppContext, request: UpdateDisplayRequest) {
        if !request.in_window(self.context.update).is_render() {
            return;
        }

        if self.first_render {
            // in first frame we can open the window, it will stay hidden until it receives the first frame
            // but the renderer will exist for resources to start loading.

            self.first_render = false;

            let vp = ctx.services.get::<ViewProcess>();

            match self.mode {
                WindowMode::Headed => {
                    // send window request to the view-process, in the view-process the window will start but
                    // still not visible, when the renderer has a frame ready to draw then the window becomes
                    // visible. All layout values are ready here too.
                    let config = view_process::WindowConfig {
                        title: self.vars.title().get(ctx.vars).to_string(),
                        pos: self.position,
                        size: self.size,
                        min_size: self.min_size,
                        max_size: self.max_size,
                        visible: self.vars.visible().copy(ctx.vars),
                        taskbar_visible: self.vars.taskbar_visible().copy(ctx.vars),
                        chrome_visible: self.vars.chrome().get(ctx.vars).is_default(),
                        allow_alt_f4: self.vars.allow_alt_f4().copy(ctx.vars),
                        clear_color: Some(rgb(255, 0, 0).into()),
                        text_aa: self.vars.text_aa().copy(ctx.vars),
                        always_on_top: self.vars.always_on_top().copy(ctx.vars),
                        movable: self.vars.movable().copy(ctx.vars),
                        resizable: self.vars.resizable().copy(ctx.vars),
                        icon: match self.vars.icon().get(ctx.vars) {
                            WindowIcon::Default => None,
                            WindowIcon::Icon(ico) => Some(view_process::Icon::clone(ico)),
                            WindowIcon::Render(_) => todo!(),
                        },
                        transparent: self.vars.transparent().copy(ctx.vars),
                    };

                    // keep the ViewWindow connection and already create the weak-ref ViewRenderer too.
                    let headed = vp.unwrap().open_window(self.id, config).expect("TODO, deal with respawn here?");

                    self.renderer = Some(headed.renderer());
                    self.headed = Some(headed);
                    ctx.services.windows().windows_info.get_mut(&self.id).unwrap().renderer = self.renderer.clone();
                }
                WindowMode::HeadlessWithRenderer => todo!(),
                WindowMode::Headless => {
                    // headless without renderer only provides the `FrameInfo` (notified in `render_frame`),
                    // but if we are in a full headless app we can simulate the behavior of headed windows that
                    // become visible and focused when they present the first frame and "resized" and "moved" with
                    // initial values.

                    let timestamp = Instant::now();
                    if vp.is_none() {
                        // if we are in a headless app too, we simulate focus.
                        drop(vp);
                        if let Some((prev_focus_id, _)) = ctx.services.windows().windows_info.iter_mut().find(|(_, w)| w.is_focused) {
                            let args = RawWindowFocusArgs::new(timestamp, *prev_focus_id, false);
                            RawWindowFocusEvent.notify(ctx.events, args)
                        }
                        let args = RawWindowFocusArgs::new(timestamp, self.id, true);
                        RawWindowFocusEvent.notify(ctx.events, args)
                    }

                    RawWindowMovedEvent.notify(
                        ctx.events,
                        RawWindowMovedArgs::new(timestamp, self.id, self.position, EventCause::App),
                    );
                    RawWindowResizedEvent.notify(
                        ctx.events,
                        RawWindowResizedArgs::new(timestamp, self.id, self.size, EventCause::App),
                    );
                }
            }
        }

        let frame = self.render_frame(ctx);

        if let Some(renderer) = &mut self.renderer {
            renderer.render(frame).expect("TODO, deal with respawn here?");
        }
    }

    /// On render update request.
    ///
    /// If there is a pending request we collect updates and send.
    fn on_render_update(&mut self, ctx: &mut AppContext) {
        if !self.context.update.is_render_update() {
            return;
        }

        debug_assert!(!self.first_render);

        let updates = self.context.render_update(ctx, self.frame_id);
        if updates.transforms.is_empty() && updates.floats.is_empty() {
            return;
        }

        if let Some(renderer) = &self.renderer {
            // send update if we have a renderer.
            renderer.render_update(updates).expect("TODO, deal with respawn here?");
        }

        // TODO notify, after we implement metadata modification in render_update.
    }

    /// Send an empty frame update.
    ///
    /// this is used when the view-process demands a frame but we don't need to generate one
    /// (like a resize to same size).
    fn render_empty_update(&mut self) {
        if let Some(renderer) = &self.renderer {
            // send update if we have a renderer.
            renderer
                .render_update(DynamicProperties {
                    transforms: vec![],
                    floats: vec![],
                })
                .expect("TODO, deal with respawn here?");
        }
    }

    fn respawn(&mut self, _ctx: &mut AppContext) {
        todo!("window respawn")
    }

    fn deinit(mut self, ctx: &mut AppContext) {
        assert!(!self.deinited);
        self.deinited = true;
        self.context.deinit(ctx);
    }
}
impl Drop for AppWindow {
    fn drop(&mut self) {
        if !self.deinited {
            log::error!("`AppWindow` dropped without calling `deinit`, no memory is leaked but shared state may be incorrect now");
        }
    }
}

struct OwnedWindowContext {
    window_id: WindowId,
    mode: WindowMode,
    root_transform_key: WidgetTransformKey,
    state: OwnedStateMap,
    root: Window,
    update: UpdateDisplayRequest,
}
impl OwnedWindowContext {
    fn init(&mut self, ctx: &mut AppContext) {
        self.widget_ctx(ctx, |ctx, child| child.init(ctx))
    }

    fn event<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        self.widget_ctx(ctx, |ctx, root| root.event(ctx, args));
    }

    fn update(&mut self, ctx: &mut AppContext) {
        self.widget_ctx(ctx, |ctx, child| child.update(ctx))
    }

    fn deinit(&mut self, ctx: &mut AppContext) {
        self.widget_ctx(ctx, |ctx, child| child.deinit(ctx))
    }

    fn widget_ctx(&mut self, ctx: &mut AppContext, f: impl FnOnce(&mut WidgetContext, &mut BoxedUiNode)) {
        let root = &mut self.root;
        let ((), update) = ctx.window_context(self.window_id, self.mode, &mut self.state, &None, |ctx| {
            let child = &mut root.child;
            ctx.widget_context(root.id, &mut root.state, |ctx| f(ctx, child))
        });
        self.update |= update;
    }

    fn layout(
        &mut self,
        ctx: &mut AppContext,
        font_size: f32,
        scale_factor: f32,
        screen_ppi: f32,
        available_size: LayoutSize,
        calc_final_size: impl FnOnce(LayoutSize) -> LayoutSize,
    ) -> LayoutSize {
        let root = &mut self.root;
        let (final_size, _) = ctx.window_context(self.window_id, self.mode, &mut self.state, &None, |ctx| {
            let child = &mut root.child;
            ctx.layout_context(
                font_size,
                PixelGrid::new(scale_factor),
                screen_ppi,
                available_size,
                root.id,
                &mut root.state,
                |ctx| {
                    let desired_size = child.measure(ctx, available_size);
                    let final_size = calc_final_size(desired_size);
                    child.arrange(ctx, final_size);
                    final_size
                },
            )
        });
        final_size
    }

    fn render(
        &mut self,
        ctx: &mut AppContext,
        frame_id: FrameId,
        root_size: LayoutSize,
        scale_factor: f32,
        renderer: &Option<ViewRenderer>,
    ) -> ((PipelineId, LayoutSize, BuiltDisplayList), FrameInfo) {
        self.update = UpdateDisplayRequest::None;

        let root = &mut self.root;
        let root_transform_key = self.root_transform_key;

        let (frame, _) = ctx.window_context(self.window_id, self.mode, &mut self.state, renderer, |ctx| {
            let child = &mut root.child;
            let mut builder = FrameBuilder::new(
                frame_id,
                *ctx.window_id,
                ctx.renderer.clone(),
                root.id,
                root_transform_key,
                root_size,
                scale_factor,
            );
            ctx.render_context(root.id, &root.state, |ctx| {
                child.render(ctx, &mut builder);
            });

            builder
        });
        frame.finalize()
    }

    fn render_update(&mut self, ctx: &mut AppContext, frame_id: FrameId) -> DynamicProperties {
        debug_assert!(self.update.is_render_update());
        self.update = UpdateDisplayRequest::None;

        let root = &self.root;
        let root_transform_key = self.root_transform_key;

        let (updates, _) = ctx.window_context(self.window_id, self.mode, &mut self.state, &None, |ctx| {
            let window_id = *ctx.window_id;
            ctx.render_context(root.id, &root.state, |ctx| {
                let mut update = FrameUpdate::new(window_id, root.id, root_transform_key, frame_id);
                root.child.render_update(ctx, &mut update);
                update.finalize()
            })
        });

        updates
    }
}

struct WindowVarsData {
    chrome: RcVar<WindowChrome>,
    icon: RcVar<WindowIcon>,
    title: RcVar<Text>,

    state: RcVar<WindowState>,

    position: RcVar<Point>,
    monitor: RcVar<MonitorQuery>,

    size: RcVar<Size>,
    auto_size: RcVar<AutoSize>,
    min_size: RcVar<Size>,
    max_size: RcVar<Size>,

    actual_position: RcVar<LayoutPoint>,
    actual_monitor: RcVar<Option<MonitorId>>,
    actual_size: RcVar<LayoutSize>,

    resizable: RcVar<bool>,
    movable: RcVar<bool>,

    always_on_top: RcVar<bool>,

    visible: RcVar<bool>,
    taskbar_visible: RcVar<bool>,

    parent: RcVar<Option<WindowId>>,
    modal: RcVar<bool>,

    transparent: RcVar<bool>,

    text_aa: RcVar<TextAntiAliasing>,

    allow_alt_f4: RcVar<bool>,

    is_open: RcVar<bool>,
}

/// Controls properties of an open window using variables.
///
/// You can get the controller for any window using [`Windows::vars`].
///
/// You can get the controller for the current context window by getting [`WindowVarsKey`] from the `window_state`
/// in [`WindowContext`] and [`WidgetContext`].
///
/// [`WindowContext`]: crate::context::WindowContext::window_state
/// [`WidgetContext`]: crate::context::WidgetContext::window_state
pub struct WindowVars(Rc<WindowVarsData>);
impl WindowVars {
    fn new() -> Self {
        let vars = Rc::new(WindowVarsData {
            chrome: var(WindowChrome::Default),
            icon: var(WindowIcon::Default),
            title: var("".to_text()),

            state: var(WindowState::Normal),

            position: var(Point::new(f32::NAN, f32::NAN)),
            monitor: var(MonitorQuery::Primary),
            size: var(Size::new(800.0, 600.0)),

            actual_position: var(LayoutPoint::new(f32::NAN, f32::NAN)),
            actual_monitor: var(None),
            actual_size: var(LayoutSize::new(f32::NAN, f32::NAN)),

            min_size: var(Size::new(192.0, 48.0)),
            max_size: var(Size::new(100.pct(), 100.pct())),
            auto_size: var(AutoSize::empty()),

            resizable: var(true),
            movable: var(true),

            always_on_top: var(false),

            visible: var(true),
            taskbar_visible: var(true),

            parent: var(None),
            modal: var(false),

            transparent: var(false),

            text_aa: var(TextAntiAliasing::Default),

            allow_alt_f4: var(!cfg!(windows)),

            is_open: var(true),
        });
        Self(vars)
    }

    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }

    /// Window chrome, the non-client area of the window.
    ///
    /// See [`WindowChrome`] for details.
    ///
    /// The default value is [`WindowChrome::Default`].
    #[inline]
    pub fn chrome(&self) -> &RcVar<WindowChrome> {
        &self.0.chrome
    }

    /// If the window is see-through.
    ///
    /// The default value is `false`.
    #[inline]
    pub fn transparent(&self) -> &RcVar<bool> {
        &self.0.transparent
    }

    /// Window icon.
    ///
    /// See [`WindowIcon`] for details.
    ///
    /// The default value is [`WindowIcon::Default`].
    #[inline]
    pub fn icon(&self) -> &RcVar<WindowIcon> {
        &self.0.icon
    }

    /// Window title text.
    ///
    /// The default value is `""`.
    #[inline]
    pub fn title(&self) -> &RcVar<Text> {
        &self.0.title
    }

    /// Window screen state.
    ///
    /// Minimized, maximized or full-screen. See [`WindowState`] for details.
    ///
    /// The default value is [`WindowState::Normal`]
    #[inline]
    pub fn state(&self) -> &RcVar<WindowState> {
        &self.0.state
    }

    /// Window top-left offset on the [`monitor`].
    ///
    /// When a dimension is not a finite value it is computed from other variables.
    /// Relative values are computed in relation to the [`monitor`], updating every time the position or
    /// monitor variable updates, not every layout.
    ///
    /// When the the window is moved this variable does **not** update back, to track the current position of the window
    /// use [`actual_position`].
    ///
    /// The default value is `(f32::NAN, f32::NAN)` that causes the window or OS to select a value.
    ///
    /// [`actual_position`]: WindowVars::actual_position
    /// [`monitor`]: WindowVars::monitor
    #[inline]
    pub fn position(&self) -> &RcVar<Point> {
        &self.0.position
    }

    /// Window monitor.
    ///
    /// The query selects the monitor to which the [`position`] and [`size`] is relative to.
    ///
    /// It evaluate once on startup and then once every time the variable updates. You can track
    /// what the current monitor is by using [`actual_monitor`].
    ///
    /// # Behavior After Open
    ///
    /// If this variable is changed after the window has opened, and the new query produces a different
    /// monitor from the [`actual_monitor`] and the window is visible; then the window is moved to
    /// the new monitor:
    ///
    /// * **Maximized**: The window is maximized in the new monitor.
    /// * **Normal**: The window is centered in the new monitor, keeping the same size, unless the
    /// [`position`] and [`size`] where set in the same update, in that case these values are used.
    /// * **Minimized/Hidden**: The window restore position and size are defined like **Normal**.
    ///
    /// [`position`]: WindowVars::position
    /// [`actual_monitor`]: WindowVars::actual_monitor
    /// [`size`]: WindowVars::size
    #[inline]
    pub fn monitor(&self) -> &RcVar<MonitorQuery> {
        &self.0.monitor
    }

    /// Current monitor hosting the window.
    ///
    /// This is `None` only if the window has not opened yet (before first render) or if
    /// no monitors where found in the operating system or if the window if headless without renderer.
    #[inline]
    pub fn actual_monitor(&self) -> ReadOnlyRcVar<Option<MonitorId>> {
        self.0.actual_monitor.clone().into_read_only()
    }

    /// Window actual position on the screen.
    ///
    /// This is a read-only variable that tracks the computed position of the window, it updates every
    /// time the window moves.
    ///
    /// The initial value is `(f32::NAN, f32::NAN)` but this is updated quickly to an actual value. The point
    /// is relative to the origin of the virtual screen that envelops all monitors.
    #[inline]
    pub fn actual_position(&self) -> ReadOnlyRcVar<LayoutPoint> {
        self.0.actual_position.clone().into_read_only()
    }

    /// Window width and height on the screen.
    ///
    /// When a dimension is not a finite value it is computed from other variables.
    /// Relative values are computed in relation to the full-screen size.
    ///
    /// When the window is resized this variable does **not** updated back, to track the current window size use [`actual_size`].
    ///
    /// The default value is `(f32::NAN, f32::NAN)`.
    ///
    /// [`actual_size`]: WindowVars::actual_size
    #[inline]
    pub fn size(&self) -> &RcVar<Size> {
        &self.0.size
    }

    /// Window actual size on the screen.
    ///
    /// This is a read-only variable that tracks the computed size of the window, it updates every time
    /// the window resizes.
    ///
    /// The initial value is `(f32::NAN, f32::NAN)` but this is updated quickly to an actual value.
    #[inline]
    pub fn actual_size(&self) -> ReadOnlyRcVar<LayoutSize> {
        self.0.actual_size.clone().into_read_only()
    }

    /// Configure window size-to-content.
    ///
    /// When enabled overwrites [`size`](Self::size), but is still coerced by [`min_size`](Self::min_size)
    /// and [`max_size`](Self::max_size). Auto-size is disabled if the user [manually resizes](Self::resizable).
    ///
    /// The default value is [`AutoSize::DISABLED`].
    #[inline]
    pub fn auto_size(&self) -> &RcVar<AutoSize> {
        &self.0.auto_size
    }

    /// Minimal window width and height.
    ///
    /// When a dimension is not a finite value it fallback to the previous valid value.
    /// Relative values are computed in relation to the full-screen size.
    ///
    /// Note that the operation systems can have their own minimal size that supersedes this variable.
    ///
    /// The default value is `(192, 48)`.
    #[inline]
    pub fn min_size(&self) -> &RcVar<Size> {
        &self.0.min_size
    }

    /// Maximal window width and height.
    ///
    /// When a dimension is not a finite value it fallback to the previous valid value.
    /// Relative values are computed in relation to the full-screen size.
    ///
    /// Note that the operation systems can have their own maximal size that supersedes this variable.
    ///
    /// The default value is `(100.pct(), 100.pct())`
    #[inline]
    pub fn max_size(&self) -> &RcVar<Size> {
        &self.0.max_size
    }

    /// If the user can resize the window using the window frame.
    ///
    /// Note that even if disabled the window can still be resized from other sources.
    ///
    /// The default value is `true`.
    #[inline]
    pub fn resizable(&self) -> &RcVar<bool> {
        &self.0.resizable
    }

    /// If the user can move the window using the window frame.
    ///
    /// Note that even if disabled the window can still be moved from other sources.
    ///
    /// The default value is `true`.
    #[inline]
    pub fn movable(&self) -> &RcVar<bool> {
        &self.0.movable
    }

    /// Whether the window should always stay on top of other windows.
    ///
    /// Note this only applies to other windows that are not also "always-on-top".
    ///
    /// The default value is `false`.
    #[inline]
    pub fn always_on_top(&self) -> &RcVar<bool> {
        &self.0.always_on_top
    }

    /// If the window is visible on the screen and in the task-bar.
    ///
    /// This variable is observed only after the first frame render, before that the window
    /// is always not visible.
    ///
    /// The default value is `true`.
    #[inline]
    pub fn visible(&self) -> &RcVar<bool> {
        &self.0.visible
    }

    /// If the window is visible in the task-bar.
    ///
    /// The default value is `true`.
    #[inline]
    pub fn taskbar_visible(&self) -> &RcVar<bool> {
        &self.0.taskbar_visible
    }

    /// The window parent.
    ///
    /// If a parent is set this behavior applies:
    ///
    /// * If the parent is minimized, this window is also minimized.
    /// * If the parent window is maximized, this window is restored.
    /// * This window is always-on-top of the parent window.
    /// * If the parent window is closed, this window is also closed.
    /// * If [`modal`](Self::modal) is set, the parent window cannot be focused while this window is open.
    ///
    /// The default value is `None`.
    #[inline]
    pub fn parent(&self) -> &RcVar<Option<WindowId>> {
        &self.0.parent
    }

    /// Configure the [`parent`](Self::parent) connection.
    ///
    /// Value is ignored is `parent` is not set.
    ///
    /// The default value is `false`.
    #[inline]
    pub fn modal(&self) -> &RcVar<bool> {
        &self.0.modal
    }

    /// Text anti-aliasing config in the window.
    ///
    /// The default value is [`TextAntiAliasing::Default`] that is the OS default.
    #[inline]
    pub fn text_aa(&self) -> &RcVar<TextAntiAliasing> {
        &self.0.text_aa
    }

    /// In Windows the `Alt+F4` shortcut is intercepted by the system and causes a window close request,
    /// if this variable is set to `true` this default behavior is disabled and a key-press event is generated
    /// instead.
    #[inline]
    pub fn allow_alt_f4(&self) -> &RcVar<bool> {
        &self.0.allow_alt_f4
    }

    /// If the window is open.
    ///
    /// This is a read-only variable, it starts set to `true` and will update only once,
    /// when the window finishes closing.
    #[inline]
    pub fn is_open(&self) -> ReadOnlyRcVar<bool> {
        self.0.is_open.clone().into_read_only()
    }
}
state_key! {
    /// Key for the instance of [`WindowVars`] in the window state.
    pub struct WindowVarsKey: WindowVars;
}
