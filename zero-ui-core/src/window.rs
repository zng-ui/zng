use std::{fmt, rc::Rc};

use linear_map::LinearMap;
pub use zero_ui_wr::{CursorIcon, Theme as WindowTheme};

use crate::{
    app::{
        view_process::{ViewRenderer, ViewWindow},
        AppEventSender, AppExtended, AppExtension,
    },
    cancelable_event_args,
    context::{AppContext, UpdateDisplayRequest, WindowContext},
    event, event_args, impl_from_and_into_var,
    render::{FramePixels, WidgetTransformKey},
    service::Service,
    state::OwnedStateMap,
    state_key,
    text::Text,
    units::*,
    var::{var, IntoValue, RcVar, ResponderVar},
    BoxedUiNode, UiNode, WidgetId,
};

unique_id! {
    /// Unique identifier of a [`OpenWindow`].
    ///
    /// Can be obtained from [`OpenWindow::id`] or [`WindowContext::window_id`] or [`WidgetContext::path`].
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
    pub struct ScreenId;
}

/// Extension trait, adds [`run_window`](AppRunWindowExt::run_window) to [`AppExtended`].
pub trait AppRunWindowExt {
    /// Runs the application event loop and requests a new window.
    ///
    /// The `new_window` argument is the [`WindowContext`] of the new window.
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
    fn run_window(self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static) -> !;
}
impl<E: AppExtension> AppRunWindowExt for AppExtended<E> {
    fn run_window(self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static) -> ! {
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

/// Window startup configuration.
///
/// More window configuration is accessible using the [`WindowVars`] type.
pub struct Window {
    state: OwnedStateMap,
    id: WidgetId,
    start_position: StartPosition,
    kiosk: bool,
    headless_screen: HeadlessScreen,
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
    /// * `headless_screen` - "Screen" configuration used in [headless mode](WindowMode::is_headless).
    /// * `child` - The root widget outermost node, the window sets-up the root widget using this and the `root_id`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        root_id: WidgetId,
        start_position: impl Into<StartPosition>,
        kiosk: bool,
        headless_screen: impl Into<HeadlessScreen>,
        child: impl UiNode,
    ) -> Self {
        Window {
            state: OwnedStateMap::default(),
            id: root_id,
            kiosk,
            start_position: start_position.into(),
            headless_screen: headless_screen.into(),
            child: child.boxed(),
        }
    }
}

/// "Screen" configuration used by windows in [headless mode](WindowMode::is_headless).
#[derive(Clone)]
pub struct HeadlessScreen {
    /// The scale factor used for the headless layout and rendering.
    ///
    /// `1.0` by default.
    pub scale_factor: f32,

    /// Size of the imaginary monitor screen that contains the headless window.
    ///
    /// This is used to calculate relative lengths in the window size definition.
    ///
    /// `(1920.0, 1080.0)` by default.
    pub screen_size: LayoutSize,

    /// Pixel-per-inches used for the headless layout and rendering.
    ///
    /// `96.0` by default.
    pub ppi: f32,
}
impl fmt::Debug for HeadlessScreen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() || about_eq(self.ppi, 96.0, 0.001) {
            f.debug_struct("HeadlessScreen")
                .field("scale_factor", &self.scale_factor)
                .field("screen_size", &self.screen_size)
                .field("ppi", &self.ppi)
                .finish()
        } else {
            write!(
                f,
                "({}, ({}, {}))",
                self.scale_factor, self.screen_size.width, self.screen_size.height
            )
        }
    }
}
impl HeadlessScreen {
    /// New with custom size at `1.0` scale.
    #[inline]
    pub fn new(screen_size: LayoutSize) -> Self {
        Self::new_scaled(screen_size, 1.0)
    }

    /// New with custom size and scale.
    #[inline]
    pub fn new_scaled(screen_size: LayoutSize, scale_factor: f32) -> Self {
        HeadlessScreen {
            scale_factor,
            screen_size,
            ppi: 96.0,
        }
    }

    /// New default size `1920x1080` and custom scale.
    #[inline]
    pub fn new_scale(scale_factor: f32) -> Self {
        HeadlessScreen {
            scale_factor,
            ..Self::default()
        }
    }
}
impl Default for HeadlessScreen {
    /// New `1920x1080` at `1.0` scale.
    fn default() -> Self {
        Self::new(LayoutSize::new(1920.0, 1080.0))
    }
}
impl IntoValue<HeadlessScreen> for (f32, f32) {}
impl From<(f32, f32)> for HeadlessScreen {
    /// Calls [`HeadlessScreen::new_scaled`]
    fn from((width, height): (f32, f32)) -> Self {
        Self::new(LayoutSize::new(width, height))
    }
}
impl IntoValue<HeadlessScreen> for (u32, u32) {}
impl From<(u32, u32)> for HeadlessScreen {
    /// Calls [`HeadlessScreen::new`]
    fn from((width, height): (u32, u32)) -> Self {
        Self::new(LayoutSize::new(width as f32, height as f32))
    }
}
impl IntoValue<HeadlessScreen> for FactorNormal {}
impl From<FactorNormal> for HeadlessScreen {
    /// Calls [`HeadlessScreen::new_scale`]
    fn from(f: FactorNormal) -> Self {
        Self::new_scale(f.0)
    }
}
impl IntoValue<HeadlessScreen> for FactorPercent {}
impl From<FactorPercent> for HeadlessScreen {
    /// Calls [`HeadlessScreen::new_scale`]
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

/// Window startup position.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StartPosition {
    /// Uses the value of the `position` property.
    Default,
    /// Centralizes the window in the monitor screen.
    CenterScreen,
    /// Centralizes the window the parent window.
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
            StartPosition::CenterScreen => write!(f, "CenterScreen"),
            StartPosition::CenterParent => write!(f, "CenterParent"),
        }
    }
}

/// Mode of an [`OpenWindow`].
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

#[derive(Clone, Copy)]
enum WindowInitState {
    /// Window not visible, awaiting first call to `OpenWindow::preload_update`.
    New,
    /// Content `UiNode::init` called, next calls to `OpenWindow::preload_update` will do updates
    /// until the first layout and render.
    ContentInited,
    /// First frame rendered and presented, window `visible`synched with var, the window
    /// is fully launched.
    Inited,
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
    fn is_default(&self) -> bool {
        matches!(self, WindowChrome::Default)
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
    Icon(zero_ui_wr::Icon),
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
    #[inline]
    pub fn from_rgba(rgba: Vec<u8>, width: u32, height: u32) -> Result<Self, ()> {
        // TODO validate
        Ok(Self::Icon(Rc::new(zero_ui_wr::Icon { rgba, width, height })))
    }

    /// New window icon from the bytes of an encoded image.
    ///
    /// The image format is guessed, PNG is recommended. Loading is done using the [`image::load_from_memory`]
    /// function from the [`image`] crate.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, image::error::ImageError> {
        use image::GenericImageView;

        let image = image::load_from_memory(bytes)?;
        let (width, height) = image.dimensions();
        let icon = Self::from_rgba(image.into_bytes(), width, height).expect("image loaded a BadIcon from memory");
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
        let icon = Self::from_rgba(image.into_bytes(), width, height).expect("image loaded a BadIcon from file");
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
    /// [`WindowIcon::from_bytes`]
    fn from(bytes: &'static [u8]) -> WindowIcon {
        WindowIcon::from_bytes(bytes).unwrap_or_else(|e| {
            log::error!(target: "window", "failed to load icon from encoded bytes: {:?}", e);
            WindowIcon::Default
        })
    }

    /// [`WindowIcon::from_rgba`]
    fn from(rgba: (Vec<u8>, u32, u32)) -> WindowIcon {
        WindowIcon::from_rgba(rgba.0, rgba.1, rgba.2).unwrap_or_else(|e| {
            log::error!(target: "window", "failed to load icon from RGBA data: {:?}", e);
            WindowIcon::Default
        })
    }

    /// [`WindowIcon::from_file`]
    fn from(file: &'static str) -> WindowIcon {
        WindowIcon::from_file(file).unwrap_or_else(|e| {
            log::error!(target: "window", "failed to load icon from file: {:?}", e);
            WindowIcon::Default
        })
    }

    /// [`WindowIcon::from_file`]
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
    pub struct WindowIsFocusedArgs {
        /// Id of window that got or lost keyboard focus.
        pub window_id: WindowId,

        /// `true` if the window got focus, `false` if the window lost focus (blur).
        pub focused: bool,

        /// If the window was lost focus because it closed.
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
        /// New window size, given by the OS.
        pub new_size: LayoutSize,

        ..

        /// If the widget is in the same window.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            ctx.path.window_id() == self.window_id
        }
    }
}
cancelable_event_args! {
    /// [`WindowCloseRequestedEvent`] args.
    pub struct WindowCloseRequestedArgs {
        /// Window ID.
        pub window_id: WindowId,

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
    pub WindowFocusChangedEvent: WindowIsFocusedArgs;

    /// Window got keyboard focus event.
    pub WindowFocusEvent: WindowIsFocusedArgs;

    /// Window lost keyboard event.
    pub WindowBlurEvent: WindowIsFocusedArgs;

    /// Window scale factor changed.
    pub WindowScaleChangedEvent: WindowScaleChangedArgs;

    /// Closing window event.
    pub WindowCloseRequestedEvent: WindowCloseRequestedArgs;

    /// Close window event.
    pub WindowCloseEvent: WindowCloseArgs;
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
///
/// # Services
///
/// Services this extension provides:
///
/// * [Windows]
/// * [Screens]
pub struct WindowManager {}
impl Default for WindowManager {
    fn default() -> Self {
        Self {}
    }
}
impl AppExtension for WindowManager {
    fn init(&mut self, ctx: &mut AppContext) {
        ctx.services.register(Screens::new());
        ctx.services.register(Windows::new(ctx.updates.sender()));
    }
}

/// Monitor screens service.
///
/// # Provider
///
/// This service is provided by the [`WindowManager`].
#[derive(Service)]
pub struct Screens {
    ppi: LinearMap<ScreenId, f32>,
}
impl Screens {
    fn new() -> Self {
        Screens { ppi: LinearMap::default() }
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

    windows: Vec<OpenWindow>,

    open_requests: Vec<OpenWindowRequest>,
    opening_windows: Vec<OpenWindow>,
    update_sender: AppEventSender,
}
impl Windows {
    fn new(update_sender: AppEventSender) -> Self {
        Windows {
            shutdown_on_last_close: true,
            windows: Vec::with_capacity(1),
            open_requests: Vec::with_capacity(1),
            opening_windows: Vec::with_capacity(1),
            update_sender,
        }
    }
}
struct OpenWindowRequest {
    new: Box<dyn FnOnce(&mut WindowContext) -> Window>,
    force_headless: Option<WindowMode>,
    responder: ResponderVar<WindowOpenArgs>,
}

/// An open window.
pub struct OpenWindow {
    // Is some if the window is headed.
    headed: Option<ViewWindow>,
    // Is some if the window is headless.
    headless: Option<HeadlessWindow>,

    // Is some if the window is headed or headless with renderer.
    renderer: Option<ViewRenderer>,

    // Is some unless is "borrowed" for a window update.
    context: Option<OwnedWindowContext>,

    // copy of some `context` values.
    mode: WindowMode,
    id: WindowId,
    root_id: WidgetId,

    vars: WindowVars,
}
impl OpenWindow {
    fn new(ctx: &mut AppContext, new_window: Box<dyn FnOnce(&mut WindowContext) -> Window>, force_headless: Option<WindowMode>) -> Self {
        // get mode.
        let mut mode = match (ctx.mode(), force_headless) {
            (WindowMode::Headed | WindowMode::HeadlessWithRenderer, Some(mode)) => {
                debug_assert!(!matches!(mode, WindowMode::Headed));
                mode
            }
            mode => mode,
        };

        // init vars.
        let vars = WindowVars::new();
        let mut wn_state = OwnedStateMap::default();
        wn_state.set(WindowVarsKey, vars.clone());

        // init root.
        let id = WindowId::new_unique();
        let root = ctx.window_context(id, mode, &mut wn_state, new_window).0;
        let root_id = root.id;

        let app_sender = ctx.updates.sender();

        // init mode.
        let mut headed = None;
        let mut headless = None;
        let mut renderer = None;

        match mode {
            WindowMode::Headed => todo!(),
            WindowMode::Headless => {
                headless = Some(HeadlessWindow {
                    screen: root.headless_screen,
                    position: (),
                    size: (),
                    state: (),
                    taskbar_visible: (),
                })
            }
            WindowMode::HeadlessWithRenderer => todo!(),
        }

        // init context.
        let context = OwnedWindowContext {
            window_id: id,
            mode,
            root_transform_key: WidgetTransformKey::new_unique(),
            state: wn_state,
            root,
            update: UpdateDisplayRequest::None,
        };

        OpenWindow {
            headed,
            headless,
            renderer,
            context,
            mode,
            id,
            root_id,
            vars,
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

// OpenWindow headless info.
struct HeadlessWindow {
    screen: HeadlessScreen,
    position: LayoutPoint,
    size: LayoutSize,
    state: WindowState,
    taskbar_visible: bool,
}

struct WindowVarsData {
    chrome: RcVar<WindowChrome>,
    icon: RcVar<WindowIcon>,
    title: RcVar<Text>,

    state: RcVar<WindowState>,

    position: RcVar<Point>,

    size: RcVar<Size>,
    auto_size: RcVar<AutoSize>,
    min_size: RcVar<Size>,
    max_size: RcVar<Size>,

    resizable: RcVar<bool>,
    movable: RcVar<bool>,

    always_on_top: RcVar<bool>,

    visible: RcVar<bool>,
    taskbar_visible: RcVar<bool>,

    parent: RcVar<Option<WindowId>>,
    modal: RcVar<bool>,

    transparent: RcVar<bool>,
}

/// Controls properties of an open window using variables.
///
/// You can get the controller for any window using [`OpenWindow::vars`].
///
/// You can get the controller for the current context window by getting [`WindowVarsKey`] from the `window_state`
/// in [`WindowContext`] and [`WidgetContext`].
///
/// [`WindowContext`]: WindowContext::window_state
/// [`WidgetContext`]: WidgetContext::window_state
pub struct WindowVars {
    vars: Rc<WindowVarsData>,
}
impl WindowVars {
    fn new() -> Self {
        let vars = Rc::new(WindowVarsData {
            chrome: var(WindowChrome::Default),
            icon: var(WindowIcon::Default),
            title: var("".to_text()),

            state: var(WindowState::Normal),

            position: var(Point::new(f32::NAN, f32::NAN)),
            size: var(Size::new(f32::NAN, f32::NAN)),

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
        });
        Self { vars }
    }

    /// Update all variables with the same value.
    fn refresh_all(&self, vars: &crate::var::Vars) {
        self.chrome().touch(vars);
        self.icon().touch(vars);
        self.title().touch(vars);
        self.state().touch(vars);
        self.position().touch(vars);
        self.size().touch(vars);
        self.min_size().touch(vars);
        self.max_size().touch(vars);
        self.auto_size().touch(vars);
        self.resizable().touch(vars);
        self.movable().touch(vars);
        self.always_on_top().touch(vars);
        self.visible().touch(vars);
        self.taskbar_visible().touch(vars);
        self.parent().touch(vars);
        self.modal().touch(vars);
        self.transparent().touch(vars);
    }

    fn clone(&self) -> Self {
        Self {
            vars: Rc::clone(&self.vars),
        }
    }

    /// Window chrome, the non-client area of the window.
    ///
    /// See [`WindowChrome`] for details.
    ///
    /// The default value is [`WindowChrome::Default`].
    #[inline]
    pub fn chrome(&self) -> &RcVar<WindowChrome> {
        &self.vars.chrome
    }

    /// If the window is see-through.
    ///
    /// The default value is `false`.
    #[inline]
    pub fn transparent(&self) -> &RcVar<bool> {
        &self.vars.transparent
    }

    /// Window icon.
    ///
    /// See [`WindowIcon`] for details.
    ///
    /// The default value is [`WindowIcon::Default`].
    #[inline]
    pub fn icon(&self) -> &RcVar<WindowIcon> {
        &self.vars.icon
    }

    /// Window title text.
    ///
    /// The default value is `""`.
    #[inline]
    pub fn title(&self) -> &RcVar<Text> {
        &self.vars.title
    }

    /// Window screen state.
    ///
    /// Minimized, maximized or full-screen. See [`WindowState`] for details.
    ///
    /// The default value is [`WindowState::Normal`]
    #[inline]
    pub fn state(&self) -> &RcVar<WindowState> {
        &self.vars.state
    }

    /// Window top-left offset on the screen.
    ///
    /// When a dimension is not a finite value it is computed from other variables.
    /// Relative values are computed in relation to the full-screen size.
    ///
    /// When the the window is moved this variable is updated back.
    ///
    /// The default value is `(f32::NAN, f32::NAN)`.
    #[inline]
    pub fn position(&self) -> &RcVar<Point> {
        &self.vars.position
    }

    /// Window width and height on the screen.
    ///
    /// When a dimension is not a finite value it is computed from other variables.
    /// Relative values are computed in relation to the full-screen size.
    ///
    /// When the window is resized this variable is updated back.
    ///
    /// The default value is `(f32::NAN, f32::NAN)`.
    #[inline]
    pub fn size(&self) -> &RcVar<Size> {
        &self.vars.size
    }

    /// Configure window size-to-content.
    ///
    /// When enabled overwrites [`size`](Self::size), but is still coerced by [`min_size`](Self::min_size)
    /// and [`max_size`](Self::max_size). Auto-size is disabled if the user [manually resizes](Self::resizable).
    ///
    /// The default value is [`AutoSize::DISABLED`].
    #[inline]
    pub fn auto_size(&self) -> &RcVar<AutoSize> {
        &self.vars.auto_size
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
        &self.vars.min_size
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
        &self.vars.max_size
    }

    /// If the user can resize the window using the window frame.
    ///
    /// Note that even if disabled the window can still be resized from other sources.
    ///
    /// The default value is `true`.
    #[inline]
    pub fn resizable(&self) -> &RcVar<bool> {
        &self.vars.resizable
    }

    /// If the user can move the window using the window frame.
    ///
    /// Note that even if disabled the window can still be moved from other sources.
    ///
    /// The default value is `true`.
    #[inline]
    pub fn movable(&self) -> &RcVar<bool> {
        &self.vars.movable
    }

    /// Whether the window should always stay on top of other windows.
    ///
    /// Note this only applies to other windows that are not also "always-on-top".
    ///
    /// The default value is `false`.
    #[inline]
    pub fn always_on_top(&self) -> &RcVar<bool> {
        &self.vars.always_on_top
    }

    /// If the window is visible on the screen and in the task-bar.
    ///
    /// This variable is observed only after the first frame render, before that the window
    /// is always not visible.
    ///
    /// The default value is `true`.
    #[inline]
    pub fn visible(&self) -> &RcVar<bool> {
        &self.vars.visible
    }

    /// If the window is visible in the task-bar.
    ///
    /// The default value is `true`.
    #[inline]
    pub fn taskbar_visible(&self) -> &RcVar<bool> {
        &self.vars.taskbar_visible
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
        &self.vars.parent
    }

    /// Configure the [`parent`](Self::parent) connection.
    ///
    /// Value is ignored is `parent` is not set.
    ///
    /// The default value is `false`.
    #[inline]
    pub fn modal(&self) -> &RcVar<bool> {
        &self.vars.modal
    }
}
state_key! {
    /// Key for the instance of [`WindowVars`] in the window state.
    pub struct WindowVarsKey: WindowVars;
}
