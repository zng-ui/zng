use std::sync::Arc;

use zng_app::{
    widget::{WidgetId, info::access::AccessEnabled},
    window::{MonitorId, WINDOW, WindowId},
};
use zng_color::LightDark;
use zng_ext_image::Img;
use zng_layout::unit::{
    Dip, DipPoint, DipRect, DipSideOffsets, DipSize, DipToPx, Factor, FactorUnits, Length, LengthUnits, Point, PxPoint, PxSize, Size,
};
use zng_state_map::{StateId, static_id};
use zng_txt::Txt;
use zng_unique_id::IdSet;
use zng_var::{Var, var, var_from, var_merge};
use zng_view_api::{
    config::{ColorScheme, ColorsConfig},
    window::{CursorIcon, FocusIndicator, RenderMode, VideoMode, WindowButton, WindowState},
};

use crate::{AutoSize, CursorSource, FrameCaptureMode, MonitorQuery, WindowIcon};

pub(super) struct WindowVarsData {
    chrome: Var<bool>,
    icon: Var<WindowIcon>,
    pub(super) actual_icon: Var<Option<Img>>,
    cursor: Var<CursorSource>,
    pub(super) actual_cursor_img: Var<Option<(Img, PxPoint)>>,
    title: Var<Txt>,

    state: Var<WindowState>,
    focus_indicator: Var<Option<FocusIndicator>>,

    position: Var<Point>,
    monitor: Var<MonitorQuery>,
    video_mode: Var<VideoMode>,

    size: Var<Size>,
    pub(super) auto_size: Var<AutoSize>,
    auto_size_origin: Var<Point>,
    min_size: Var<Size>,
    max_size: Var<Size>,

    font_size: Var<Length>,

    pub(super) actual_position: Var<DipPoint>,
    pub(super) global_position: Var<PxPoint>,
    pub(super) actual_monitor: Var<Option<MonitorId>>,
    pub(super) actual_size: Var<DipSize>,
    pub(super) safe_padding: Var<DipSideOffsets>,

    pub(super) scale_factor: Var<Factor>,

    pub(super) restore_state: Var<WindowState>,
    pub(super) restore_rect: Var<DipRect>,

    enabled_buttons: Var<WindowButton>,

    resizable: Var<bool>,
    movable: Var<bool>,

    always_on_top: Var<bool>,

    visible: Var<bool>,
    taskbar_visible: Var<bool>,

    parent: Var<Option<WindowId>>,
    pub(super) nest_parent: Var<Option<WidgetId>>,
    modal: Var<bool>,
    pub(super) children: Var<IdSet<WindowId>>,

    color_scheme: Var<Option<ColorScheme>>,
    pub(super) actual_color_scheme: Var<ColorScheme>,
    accent_color: Var<Option<LightDark>>,
    pub(super) actual_accent_color: Var<LightDark>,

    pub(super) is_open: Var<bool>,
    pub(super) is_loaded: Var<bool>,

    frame_capture_mode: Var<FrameCaptureMode>,
    pub(super) render_mode: Var<RenderMode>,

    pub(super) access_enabled: Var<AccessEnabled>,
    system_shutdown_warn: Var<Txt>,
}

/// Variables that configure the opening or open window.
///
/// You can get the vars for any window using [`WINDOWS.vars`].
///
/// You can get the vars for the current context window using [`WINDOW.vars`].
///
/// [`WINDOWS.vars`]: crate::WINDOWS::vars
/// [`WINDOW.vars`]: crate::WINDOW_Ext::vars
#[derive(Clone)]
pub struct WindowVars(pub(super) Arc<WindowVarsData>);
impl WindowVars {
    pub(super) fn new(default_render_mode: RenderMode, primary_scale_factor: Factor, system_colors: ColorsConfig) -> Self {
        let vars = Arc::new(WindowVarsData {
            chrome: var(true),
            icon: var(WindowIcon::Default),
            actual_icon: var(None),
            cursor: var_from(CursorIcon::Default),
            actual_cursor_img: var(None),
            title: var(zng_env::about().app.clone()),

            state: var(WindowState::Normal),
            focus_indicator: var(None),

            position: var(Point::default()),
            monitor: var(MonitorQuery::default()),
            video_mode: var(VideoMode::default()),
            size: var(Size::default()),

            font_size: var(11.pt()),

            actual_position: var(DipPoint::zero()),
            global_position: var(PxPoint::zero()),
            actual_monitor: var(None),
            actual_size: var(DipSize::zero()),
            safe_padding: var(DipSideOffsets::zero()),

            scale_factor: var(primary_scale_factor),

            restore_state: var(WindowState::Normal),
            restore_rect: var(DipRect::new(
                DipPoint::new(Dip::new(30), Dip::new(30)),
                DipSize::new(Dip::new(800), Dip::new(600)),
            )),

            enabled_buttons: var(WindowButton::all()),

            min_size: var(Size::new(192, 48)),
            max_size: var(Size::new(100.pct(), 100.pct())),

            auto_size: var(AutoSize::empty()),
            auto_size_origin: var(Point::top_left()),

            resizable: var(true),
            movable: var(true),

            always_on_top: var(false),

            visible: var(true),
            taskbar_visible: var(true),

            parent: var(None),
            nest_parent: var(None),
            modal: var(false),
            children: var(IdSet::default()),

            color_scheme: var(None),
            actual_color_scheme: var(system_colors.scheme),
            accent_color: var(None),
            actual_accent_color: var(system_colors.accent.into()),

            is_open: var(true),
            is_loaded: var(false),

            frame_capture_mode: var(FrameCaptureMode::Sporadic),
            render_mode: var(default_render_mode),

            access_enabled: var(AccessEnabled::empty()),
            system_shutdown_warn: var(Txt::from("")),
        });
        Self(vars)
    }

    /// Require the window vars from the window state.
    ///
    /// # Panics
    ///
    /// Panics if called in a custom window context that did not setup the variables.
    pub(super) fn req() -> Self {
        WINDOW.req_state(*WINDOW_VARS_ID)
    }

    /// Defines if the window chrome is visible.
    ///
    /// The window chrome is the non-client area of the window, usually a border with resize handles and a title bar.
    ///
    /// The default value is `true`.
    ///
    /// Note that if the [`WINDOWS.system_chrome`] reports the windowing system prefers a custom chrome **and** does not
    /// provide one the system chrome is not requested, even if this is `true`. Window widget implementers can use this to
    /// detect when a fallback chrome must be provided.
    ///
    /// [`WINDOWS.system_chrome`]: crate::WINDOWS::system_chrome
    pub fn chrome(&self) -> Var<bool> {
        self.0.chrome.clone()
    }

    /// Window icon.
    ///
    /// See [`WindowIcon`] for details.
    ///
    /// The default value is [`WindowIcon::Default`].
    ///
    /// You can retrieve the custom icon image using [`actual_icon`].
    ///
    /// [`actual_icon`]: Self::actual_icon
    pub fn icon(&self) -> Var<WindowIcon> {
        self.0.icon.clone()
    }

    /// Window icon image.
    ///
    /// This is `None` if [`icon`] is [`WindowIcon::Default`], otherwise it is an [`Img`]
    /// reference clone.
    ///
    /// [`icon`]: Self::icon
    /// [`Img`]: zng_ext_image::Img
    pub fn actual_icon(&self) -> Var<Option<Img>> {
        self.0.actual_icon.read_only()
    }

    /// Window cursor icon and visibility.
    ///
    /// See [`CursorSource`] for details.
    ///
    /// The default is [`CursorIcon::Default`].
    ///
    /// [`CursorIcon`]: zng_view_api::window::CursorIcon
    /// [`CursorIcon::Default`]: zng_view_api::window::CursorIcon::Default
    pub fn cursor(&self) -> Var<CursorSource> {
        self.0.cursor.clone()
    }

    /// Window custom cursor image.
    ///
    /// This is `None` if [`cursor`] is not set to a custom image, otherwise it is an [`Img`]
    /// reference clone with computed hotspot [`PxPoint`].
    ///
    /// [`cursor`]: Self::cursor
    /// [`Img`]: zng_ext_image::Img
    /// [`PxPoint`]: zng_layout::unit::PxPoint
    pub fn actual_cursor_img(&self) -> Var<Option<(Img, PxPoint)>> {
        self.0.actual_cursor_img.read_only()
    }

    /// Window title text.
    ///
    /// The default value is `""`.
    pub fn title(&self) -> Var<Txt> {
        self.0.title.clone()
    }

    /// Window screen state.
    ///
    /// Minimized, maximized or fullscreen. See [`WindowState`] for details.
    ///
    /// The default value is [`WindowState::Normal`].
    pub fn state(&self) -> Var<WindowState> {
        self.0.state.clone()
    }

    /// Window monitor.
    ///
    /// The query selects the monitor to which the [`position`] and [`size`] is relative to.
    ///
    /// It evaluate once when the window opens and then once every time the variable updates. You can track
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
    ///   [`position`] and [`size`] where set in the same update, in that case these values are used.
    /// * **Minimized/Hidden**: The window restore position and size are defined like **Normal**.
    ///
    /// [`position`]: WindowVars::position
    /// [`actual_monitor`]: WindowVars::actual_monitor
    /// [`size`]: WindowVars::size
    pub fn monitor(&self) -> Var<MonitorQuery> {
        self.0.monitor.clone()
    }

    /// Video mode for exclusive fullscreen.
    pub fn video_mode(&self) -> Var<VideoMode> {
        self.0.video_mode.clone()
    }

    /// Current monitor hosting the window.
    ///
    /// This is `None` only if the window has not opened yet (before first render) or if
    /// no monitors where found in the operating system or if the window is headless without renderer.
    pub fn actual_monitor(&self) -> Var<Option<MonitorId>> {
        self.0.actual_monitor.read_only()
    }

    /// Available video modes in the current monitor.
    pub fn video_modes(&self) -> Var<Vec<VideoMode>> {
        self.0.actual_monitor.flat_map(|&m| {
            m.and_then(|m| super::MONITORS.monitor(m))
                .unwrap_or_else(super::MonitorInfo::fallback)
                .video_modes()
        })
    }

    /// Current scale factor of the current monitor hosting the window.
    pub fn scale_factor(&self) -> Var<Factor> {
        self.0.scale_factor.read_only()
    }

    /// Window actual position on the [monitor].
    ///
    /// This is a read-only variable that tracks the computed position of the window, it updates every
    /// time the window moves.
    ///
    /// The initial value is `(0, 0)`, it starts updating once the window opens. The point
    /// is relative to the origin of the [monitor].
    ///
    /// [monitor]: Self::actual_monitor
    pub fn actual_position(&self) -> Var<DipPoint> {
        self.0.actual_position.read_only()
    }

    /// Window actual position on the virtual screen that encompasses all monitors.
    ///
    /// This is a read-only variable that tracks the computed position of the window, it updates every
    /// time the window moves.
    ///
    /// The initial value is `(0, 0)`, it starts updating once the window opens.
    pub fn global_position(&self) -> Var<PxPoint> {
        self.0.global_position.read_only()
    }

    /// Window restore state.
    ///
    /// The restore state that the window must be set to be restored, if the [current state] is [`Maximized`], [`Fullscreen`] or [`Exclusive`]
    /// the restore state is [`Normal`], if the [current state] is [`Minimized`] the restore state is the previous state.
    ///
    /// When the restore state is [`Normal`] the [`restore_rect`] defines the window position and size.
    ///
    /// [current state]: Self::state
    /// [`Maximized`]: WindowState::Maximized
    /// [`Fullscreen`]: WindowState::Fullscreen
    /// [`Exclusive`]: WindowState::Exclusive
    /// [`Normal`]: WindowState::Normal
    /// [`Minimized`]: WindowState::Minimized
    /// [`restore_rect`]: Self::restore_rect
    pub fn restore_state(&self) -> Var<WindowState> {
        self.0.restore_state.read_only()
    }

    /// Window restore position and size when restoring to [`Normal`].
    ///
    /// The restore rectangle is the window position and size when its state is [`Normal`], when the state is not [`Normal`]
    /// this variable tracks the last normal position and size, it will be the window [`actual_position`] and [`actual_size`] again
    /// when the state is set back to [`Normal`].
    ///
    /// This is a read-only variable, to programmatically set it assign the [`position`] and [`size`] variables. The initial
    /// value is `(30, 30).at(800, 600)`, it starts updating when the window opens.
    ///
    /// Note that to restore the window you only need to set [`state`] to [`restore_state`], if the restore state is [`Normal`]
    /// this position and size will be applied automatically.
    ///
    /// [`Normal`]: WindowState::Normal
    /// [`actual_position`]: Self::actual_position
    /// [`actual_size`]: Self::actual_size
    /// [`position`]: Self::position
    /// [`size`]: Self::size
    /// [`monitor`]: Self::monitor
    /// [`actual_monitor`]: Self::actual_monitor
    /// [`state`]: Self::state
    /// [`restore_state`]: Self::restore_state
    pub fn restore_rect(&self) -> Var<DipRect> {
        self.0.restore_rect.read_only()
    }

    /// Window top-left offset on the [`monitor`] when the window is [`Normal`].
    ///
    /// Relative values are computed in relation to the [`monitor`] size, updating every time the
    /// position or monitor variable updates.
    ///
    /// When the user moves the window this value is considered stale, when it updates it overwrites the window position again,
    /// note that the window is only moved if it is in the [`Normal`] state, otherwise only the [`restore_rect`] updates.
    ///
    /// When the window is moved by the user this variable does **not** update back, to track the current position of the window
    /// use [`actual_position`], to track the restore position use [`restore_rect`].
    ///
    /// The [`Length::Default`] value causes the OS to select a value.
    ///
    /// [`restore_rect`]: WindowVars::restore_rect
    /// [`actual_position`]: WindowVars::actual_position
    /// [`monitor`]: WindowVars::monitor
    /// [`Normal`]: WindowState::Normal
    /// [`Length::Default`]: zng_layout::unit::Length::Default
    pub fn position(&self) -> Var<Point> {
        self.0.position.clone()
    }

    /// Window actual size on the screen.
    ///
    /// This is a read-only variable that tracks the computed size of the window, it updates every time
    /// the window resizes.
    ///
    /// The initial value is `(0, 0)`, it starts updating when the window opens.
    pub fn actual_size(&self) -> Var<DipSize> {
        self.0.actual_size.read_only()
    }

    /// Window [`actual_size`], converted to pixels given the [`scale_factor`].
    ///
    /// [`actual_size`]: Self::actual_size
    /// [`scale_factor`]: Self::scale_factor
    pub fn actual_size_px(&self) -> Var<PxSize> {
        var_merge!(self.0.actual_size.clone(), self.0.scale_factor.clone(), |size, factor| {
            PxSize::new(size.width.to_px(*factor), size.height.to_px(*factor))
        })
    }

    /// Padding that must be applied to the window content so that it stays clear of screen obstructions
    /// such as a camera notch cutout.
    ///
    /// Note that the *unsafe* area must still be rendered as it may be partially visible, just don't place nay
    /// interactive or important content outside of this padding.
    pub fn safe_padding(&self) -> Var<DipSideOffsets> {
        self.0.safe_padding.read_only()
    }

    /// Window width and height on the screen when the window is [`Normal`].
    ///
    /// Relative values are computed in relation to the [`monitor`] size, updating every time the
    /// size or monitor variable updates.
    ///
    /// When the user resizes the window this value is considered stale, when it updates it overwrites the window size again,
    /// note that the window is only resized if it is in the [`Normal`] state, otherwise only the [`restore_rect`] updates.
    ///
    /// When the window is resized this variable is **not** updated back, to track the current window size use [`actual_size`],
    /// to track the restore size use [`restore_rect`].
    ///
    /// The default value is `(800, 600)`.
    ///
    /// [`actual_size`]: WindowVars::actual_size
    /// [`monitor`]: WindowVars::monitor
    /// [`restore_rect`]: WindowVars::restore_rect
    /// [`Normal`]: WindowState::Normal
    pub fn size(&self) -> Var<Size> {
        self.0.size.clone()
    }

    /// Defines if and how the window size is controlled by the content layout.
    ///
    /// When enabled overwrites [`size`](Self::size), but is still coerced by [`min_size`](Self::min_size)
    /// and [`max_size`](Self::max_size). Auto-size is disabled if the user [manually resizes](Self::resizable).
    ///
    /// The default value is [`AutoSize::DISABLED`].
    pub fn auto_size(&self) -> Var<AutoSize> {
        self.0.auto_size.clone()
    }

    /// The point in the window content that does not move when the window is resized by [`auto_size`].
    ///
    /// When the window size increases it *grows* to the right-bottom, the top-left corner does not move because
    /// the origin of windows it at the top-left and the position did not change, this variables overwrites this origin
    /// for [`auto_size`] size changes, the window position is adjusted so that it is the center of the resize.
    ///
    /// Note this only applies to resizes, the initial auto-size when the window opens is positioned according to the [`StartPosition`] value.
    ///
    /// The default value is [`Point::top_left`].
    ///
    /// [`auto_size`]: Self::auto_size
    /// [`monitor`]: WindowVars::monitor
    /// [`StartPosition`]: crate::StartPosition
    /// [`Point::top_left`]: zng_layout::unit::Point::top_left
    pub fn auto_size_origin(&self) -> Var<Point> {
        self.0.auto_size_origin.clone()
    }

    /// Minimal window width and height constraint on the [`size`].
    ///
    /// Relative values are computed in relation to the [`monitor`] size, updating every time the
    /// size or monitor variable updates.
    ///
    /// Note that the OS can also define a minimum size that supersedes this variable.
    ///
    /// The default value is `(192, 48)`.
    ///
    /// [`monitor`]: WindowVars::monitor
    /// [`size`]: Self::size
    pub fn min_size(&self) -> Var<Size> {
        self.0.min_size.clone()
    }

    /// Maximal window width and height constraint on the [`size`].
    ///
    /// Relative values are computed in relation to the [`monitor`] size, updating every time the
    /// size or monitor variable updates.
    ///
    /// Note that the OS can also define a maximum size that supersedes this variable.
    ///
    /// The default value is `(100.pct(), 100.pct())`
    ///
    /// [`monitor`]: WindowVars::monitor
    /// [`size`]: Self::size
    pub fn max_size(&self) -> Var<Size> {
        self.0.max_size.clone()
    }

    /// Root font size.
    ///
    /// This is the font size in all widget branches  that do not override the font size. The [`rem`] unit is relative to this value.
    ///
    /// [`rem`]: LengthUnits::rem
    pub fn font_size(&self) -> Var<Length> {
        self.0.font_size.clone()
    }

    /// Defines if the user can resize the window using the window frame.
    ///
    /// Note that even if disabled the window can still be resized from other sources.
    ///
    /// The default value is `true`.
    pub fn resizable(&self) -> Var<bool> {
        self.0.resizable.clone()
    }

    /// Defines if the user can move the window using the window frame.
    ///
    /// Note that even if disabled the window can still be moved from other sources.
    ///
    /// The default value is `true`.
    pub fn movable(&self) -> Var<bool> {
        self.0.movable.clone()
    }

    /// Defines the enabled state of the window chrome buttons.
    pub fn enabled_buttons(&self) -> Var<WindowButton> {
        self.0.enabled_buttons.clone()
    }

    /// Defines if the window should always stay on top of other windows.
    ///
    /// Note this only applies to other windows that are not also "always-on-top".
    ///
    /// The default value is `false`.
    pub fn always_on_top(&self) -> Var<bool> {
        self.0.always_on_top.clone()
    }

    /// Defines if the window is visible on the screen and in the task-bar.
    ///
    /// This variable is observed only after the first frame render, before that the window
    /// is always not visible.
    ///
    /// The default value is `true`.
    pub fn visible(&self) -> Var<bool> {
        self.0.visible.clone()
    }

    /// Defines if the window is visible in the task-bar.
    ///
    /// The default value is `true`.
    pub fn taskbar_visible(&self) -> Var<bool> {
        self.0.taskbar_visible.clone()
    }

    /// Defines the parent window.
    ///
    /// If a parent is set this behavior applies:
    ///
    /// * If the parent is minimized, this window is also minimized.
    /// * If the parent window is maximized, this window is restored.
    /// * This window is always on-top of the parent window.
    /// * If the parent window is closed, this window is also closed.
    /// * If [`modal`] is set, the parent window cannot be focused while this window is open.
    /// * If a [`color_scheme`] is not set, the fallback is the parent's actual scheme.
    /// * If an [`accent_color`] is not set, the fallback is the parent's actual accent.
    ///
    /// The default value is `None`.
    ///
    /// # Validation
    ///
    /// The parent window cannot have a parent, if it has, that parent ID is used instead.
    /// The parent window must exist. This window (child) cannot have children, it also can't set itself as the parent.
    ///
    /// If any of these conditions are not met, an error is logged and the parent var is restored to the previous value.
    ///
    /// [`modal`]: Self::modal
    /// [`color_scheme`]: Self::color_scheme
    /// [`accent_color`]: Self::accent_color
    pub fn parent(&self) -> Var<Option<WindowId>> {
        self.0.parent.clone()
    }

    /// Gets the widget in [`parent`] that hosts the window, if it is nesting.
    ///
    /// Nesting windows are presented as an widget, similar to an "iframe".
    ///
    /// [`parent`]: Self::parent
    pub fn nest_parent(&self) -> Var<Option<WidgetId>> {
        self.0.nest_parent.read_only()
    }

    /// Defines the [`parent`](Self::parent) connection.
    ///
    /// Value is ignored if `parent` is not set. When this is `true` the parent window cannot be focused while this window is open.
    ///
    /// The default value is `false`.
    pub fn modal(&self) -> Var<bool> {
        self.0.modal.clone()
    }

    /// Window children.
    ///
    /// This is a set of other windows that have this window as a [`parent`].
    ///
    /// [`parent`]: Self::parent
    pub fn children(&self) -> Var<IdSet<WindowId>> {
        self.0.children.read_only()
    }

    /// Override the preferred color scheme.
    ///
    /// If set to `None` the system preference is used, see [`actual_color_scheme`].
    ///
    /// [`actual_color_scheme`]: Self::actual_color_scheme
    pub fn color_scheme(&self) -> Var<Option<ColorScheme>> {
        self.0.color_scheme.clone()
    }

    /// Actual color scheme to use.
    ///
    /// This is the system preference, or [`color_scheme`] if it is set.
    ///
    /// [`color_scheme`]: Self::color_scheme
    pub fn actual_color_scheme(&self) -> Var<ColorScheme> {
        self.0.actual_color_scheme.read_only()
    }

    /// Override the preferred accent color.
    ///
    /// If set to `None` the system preference is used, see [`actual_accent_color`].
    ///
    /// [`actual_accent_color`]: Self::actual_accent_color
    pub fn accent_color(&self) -> Var<Option<LightDark>> {
        self.0.accent_color.clone()
    }

    /// Actual accent color to use.
    ///
    /// This is the system preference, or [`color_scheme`] if it is set.
    ///
    /// The window widget also sets [`ACCENT_COLOR_VAR`] to this variable.
    ///
    /// [`color_scheme`]: Self::color_scheme
    /// [`ACCENT_COLOR_VAR`]: zng_color::colors::ACCENT_COLOR_VAR
    pub fn actual_accent_color(&self) -> Var<LightDark> {
        self.0.actual_accent_color.read_only()
    }

    /// If the window is open.
    ///
    /// This is a read-only variable, it starts set to `true` and will update only once,
    /// when the window finishes opening.
    ///
    /// Note that a window is only actually opened in the view-process after it [`is_loaded`].
    ///
    /// [`is_loaded`]: Self::is_loaded
    pub fn is_open(&self) -> Var<bool> {
        self.0.is_open.read_only()
    }

    /// If the window has finished loading.
    ///
    /// This is a read-only variable, it starts set to `false` and will update only once, after
    /// the first window layout and when all loading handles to the window are released.
    ///
    /// A window is only opened in the view-process once it is loaded, see [`WINDOWS.loading_handle`] for more details.
    ///
    /// [`WINDOWS.loading_handle`]: crate::WINDOWS::loading_handle
    pub fn is_loaded(&self) -> Var<bool> {
        self.0.is_loaded.read_only()
    }

    /// Defines the active user attention required indicator.
    ///
    /// This is usually a visual indication on the taskbar icon that prompts the user to focus on the window, it is automatically
    /// changed to `None` once the window receives focus or you can set it to `None` to cancel the indicator.
    ///
    /// Prefer using the `FOCUS` service and advanced `FocusRequest` configs instead of setting this variable directly.
    pub fn focus_indicator(&self) -> Var<Option<FocusIndicator>> {
        self.0.focus_indicator.clone()
    }

    /// Defines if and how the frame pixels are captured for the next rendered frames.
    ///
    /// If set to [`Next`] the value will change to [`Sporadic`] after the next frame is rendered.
    ///
    /// Note that setting this to [`Next`] does not cause a frame request. Use [`WIDGET.render_update`] for that.
    ///
    /// [`Next`]: FrameCaptureMode::Next
    /// [`Sporadic`]: FrameCaptureMode::Sporadic
    /// [`WIDGET.render_update`]: zng_app::widget::WIDGET::render_update
    pub fn frame_capture_mode(&self) -> Var<FrameCaptureMode> {
        self.0.frame_capture_mode.clone()
    }

    /// Window actual render mode.
    ///
    /// The initial value is the [`default_render_mode`], it can update after the window is created, when the view-process
    /// actually creates the backend window and after a view-process respawn.
    ///
    /// [`default_render_mode`]: crate::WINDOWS::default_render_mode
    pub fn render_mode(&self) -> Var<RenderMode> {
        self.0.render_mode.read_only()
    }

    /// If an accessibility service has requested info from this window.
    ///
    /// You can enable this in the app-process using [`enable_access`], the
    /// view-process can also enable it on the first request for accessibility info by an external tool.
    ///
    /// This variable does not update to fully disabled after first enable, but the VIEW bit can disable and re-enable.
    ///
    /// [`enable_access`]: crate::WINDOW_Ext::enable_access
    pub fn access_enabled(&self) -> Var<AccessEnabled> {
        self.0.access_enabled.read_only()
    }

    /// Attempt to set a system wide shutdown warning associated with the window.
    ///
    /// Operating systems that support this show the text in a warning for the user, it must be a short text
    /// that identifies the critical operation that cannot be cancelled.
    ///
    /// Set to an empty text to remove the warning.
    ///
    /// Note that this does not stop the window from closing or the app from exiting normally, you must
    /// handle window close requests and show some feedback to the user, the view-process will send a window close
    /// request when a system shutdown attempt is detected.
    ///
    /// Note that there is no guarantee that the view-process or operating system will actually set a block, there
    /// is no error result because operating systems can silently ignore block requests at any moment, even after
    /// an initial successful block.
    ///
    /// ## Current Limitations
    ///
    /// The current `zng::view_process` or `zng-view` only implements this feature on Windows and it will only work properly
    /// under these conditions:
    ///
    /// * It must be running in `run_same_process` mode. Windows kills all other processes, so in a run with `init` the app-process
    ///   will be lost. Note that this also mean that the crash handler and worker processes are also killed.
    /// * Must be built with `#![windows_subsystem = "windows"]` and must be running from the Windows Explorer (desktop).
    pub fn system_shutdown_warn(&self) -> Var<Txt> {
        self.0.system_shutdown_warn.clone()
    }
}
impl PartialEq for WindowVars {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for WindowVars {}

static_id! {
    pub(super) static ref WINDOW_VARS_ID: StateId<WindowVars>;
}
