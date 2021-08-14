//! App window manager.

use std::{fmt, mem, rc::Rc};

use linear_map::LinearMap;
pub use zero_ui_wr::{CursorIcon, Theme as WindowTheme};

use crate::{
    app::{
        self,
        view_process::{ViewRenderer, ViewWindow},
        AppEventSender, AppExtended, AppExtension,
    },
    cancelable_event_args,
    context::{AppContext, UpdateDisplayRequest, WindowContext},
    event, event_args, impl_from_and_into_var,
    render::{FrameHitInfo, FrameInfo, FramePixels, WidgetTransformKey},
    service::Service,
    state::OwnedStateMap,
    state_key,
    text::{Text, ToText},
    units::*,
    var::{response_var, var, IntoValue, RcVar, ResponderVar, ResponseVar},
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
impl HeadlessAppWindowExt for app::HeadlessApp {
    fn open_window(&mut self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static) -> WindowId {
        let response = self.ctx().services.windows().open(new_window);
        let mut window_id = None;
        while window_id.is_none() {
            self.update_observe(
                |ctx| {
                    if let Some(opened) = response.rsp_new(ctx) {
                        window_id = Some(opened.window_id);
                    }
                },
                true,
            );
        }
        let window_id = window_id.unwrap();

        self.focus_window(window_id);

        window_id
    }

    fn focus_window(&mut self, window_id: WindowId) {
        use app::raw_events::*;

        let args = RawWindowFocusArgs::now(window_id, true);
        RawWindowFocusEvent.notify(self.ctx().events, args);
    }

    fn blur_window(&mut self, window_id: WindowId) {
        use app::raw_events::*;

        let args = RawWindowFocusArgs::now(window_id, false);
        RawWindowFocusEvent.notify(self.ctx().events, args);
    }

    fn wait_frame(&mut self, window_id: WindowId) -> FramePixels {
        // the current frame for comparison.
        let frame_id = self.ctx().services.windows().frame_info(window_id).ok().map(|w| w.frame_id());

        loop {
            self.update(true);

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

        self.update_observe_event(
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
    Icon(Rc<zero_ui_wr::Icon>),
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
        ctx.services.register(Screens::new());
        ctx.services.register(Windows::new(ctx.updates.sender()));
    }

    fn update_ui(&mut self, ctx: &mut AppContext) {
        let (open, close) = ctx.services.windows().take_requests();

        // fulfill open requests.
        for r in open {
            let w = AppWindow::new(ctx, r.new, r.force_headless);
            let args = WindowOpenArgs::now(w.id);
            ctx.services.windows().windows.insert(w.id, w);

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

        let wins = ctx.services.windows();
        if wins.send_allow_alt_f4 {
            for app_w in wins.windows.values() {
                if let Some(view_w) = &app_w.headed {
                    view_w.allow_alt_f4(app_w.allow_alt_f4);
                }
            }
        }
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

                        for canceled in e.get().windows.values() {
                            if let Some(c) = canceled {
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

                                // drop all windows, this closes then in the View Process.
                                let windows = ctx.services.windows();
                                for (w, _) in e.windows {
                                    if windows.windows.remove(&w).is_some() {
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
        }
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

    windows: LinearMap<WindowId, AppWindow>,

    open_requests: Vec<OpenWindowRequest>,
    update_sender: AppEventSender,

    close_group_id: CloseGroupId,
    close_requests: LinearMap<WindowId, CloseWindowRequest>,

    send_allow_alt_f4: bool,
}
impl Windows {
    fn new(update_sender: AppEventSender) -> Self {
        Windows {
            shutdown_on_last_close: true,
            windows: LinearMap::with_capacity(1),
            open_requests: Vec::with_capacity(1),
            update_sender,

            close_group_id: 1,
            close_requests: LinearMap::new(),

            send_allow_alt_f4: false,
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
        if self.windows.contains_key(&window_id) {
            let (responder, response) = response_var();

            let group = self.close_group_id.wrapping_add(1);
            self.close_group_id = group;

            self.close_requests.insert(window_id, CloseWindowRequest { responder, group });

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
            if !self.windows.contains_key(&window) {
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

    /// Reference the metadata about the window's latest frame.
    pub fn frame_info(&self, window_id: WindowId) -> Result<&FrameInfo, WindowNotFound> {
        self.windows.get(&window_id).map(|w| &w.frame_info).ok_or(WindowNotFound(window_id))
    }

    /// Copy the pixels of the window's latest frame.
    pub fn frame_pixels(&self, window_id: WindowId) -> Result<FramePixels, WindowNotFound> {
        todo!()
    }

    /// Hit-test the latest window frame.
    pub fn hit_test(&self, window_id: WindowId, point: LayoutPoint) -> Result<FrameHitInfo, WindowNotFound> {
        self.windows
            .get(&window_id)
            .map(|w| w.hit_test(point))
            .ok_or(WindowNotFound(window_id))
    }

    /// Gets if the window is focused in the OS.
    pub fn is_focused(&self, window_id: WindowId) -> Result<bool, WindowNotFound> {
        self.windows.get(&window_id).map(|w| w.is_focused).ok_or(WindowNotFound(window_id))
    }

    /// Iterate over the latest frames of each open window.
    pub fn frames(&self) -> impl Iterator<Item = &FrameInfo> {
        self.windows.values().map(|w| &w.frame_info)
    }

    /// Gets the current window scale factor.
    pub fn scale_factor(&self, window_id: WindowId) -> Result<f32, WindowNotFound> {
        self.windows
            .get(&window_id)
            .map(|w| w.scale_factor())
            .ok_or(WindowNotFound(window_id))
    }

    /// Gets the id of the window that is focused in the OS.
    pub fn focused_window_id(&self) -> Option<WindowId> {
        self.windows.values().find(|w| w.is_focused).map(|w| w.id)
    }

    /// Gets the latest frame for the focused window.
    pub fn focused_frame(&self) -> Option<&FrameInfo> {
        self.windows.values().find(|w| w.is_focused).map(|w| &w.frame_info)
    }

    /// In Windows stops the system from requesting a window close on `ALT+F4` and sends a key
    /// press for F4 instead.
    pub fn allow_alt_f4(&mut self, window_id: WindowId, allow: bool) -> Result<(), WindowNotFound> {
        if let Some(w) = self.windows.get_mut(&window_id) {
            if w.allow_alt_f4 != allow {
                w.allow_alt_f4 = allow;
                self.send_allow_alt_f4 = true;
                self.update_sender.send_update();
            }
            Ok(())
        } else {
            Err(WindowNotFound(window_id))
        }
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

/// An open window.
struct AppWindow {
    // Is some if the window is headed.
    headed: Option<ViewWindow>,
    // Is some if the window is headless.
    headless: Option<HeadlessWindow>,

    // Is some if the window is headed or headless with renderer.
    renderer: Option<ViewRenderer>,

    // Window context.
    context: OwnedWindowContext,

    // copy of some `context` values.
    mode: WindowMode,
    id: WindowId,
    root_id: WidgetId,

    vars: WindowVars,

    // latest frame.
    frame_info: FrameInfo,
    // focus tracked by the raw focus events.
    is_focused: bool,

    allow_alt_f4: bool,
}
impl AppWindow {
    fn new(ctx: &mut AppContext, new_window: Box<dyn FnOnce(&mut WindowContext) -> Window>, force_headless: Option<WindowMode>) -> Self {
        // get mode.
        let mut mode = match (ctx.mode(), force_headless) {
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
                    screen: root.headless_screen.clone(),
                    position: LayoutPoint::zero(),
                    size: LayoutSize::new(800.0, 600.0),
                    state: WindowState::Normal,
                    taskbar_visible: true,
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

        AppWindow {
            headed,
            headless,
            renderer,
            context,
            mode,
            id,
            root_id,
            vars,
            frame_info: FrameInfo::blank(id, root_id),
            is_focused: true, // can we say it opens focused? TODO

            // in Windows is blocked by default, TODO check Unix
            allow_alt_f4: !cfg!(windows),
        }
    }

    fn hit_test(&self, point: LayoutPoint) -> FrameHitInfo {
        todo!()
    }

    fn scale_factor(&self) -> f32 {
        todo!()
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
