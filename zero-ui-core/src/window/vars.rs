use std::{rc::Rc, sync::Arc};

use linear_map::set::LinearSet;

use super::{types::*, MonitorId, MonitorQuery};
use crate::{
    color::ColorScheme,
    context::{state_map, BorrowStateMap, StaticStateId},
    image::Image,
    render::RenderMode,
    text::{Text, ToText},
    units::*,
    var::*,
};

pub(super) struct WindowVarsData {
    chrome: RcVar<WindowChrome>,
    icon: RcVar<WindowIcon>,
    pub(super) actual_icon: RcVar<Option<Image>>,
    cursor: RcVar<Option<CursorIcon>>,
    title: RcVar<Text>,

    state: RcVar<WindowState>,
    focus_indicator: RcVar<Option<FocusIndicator>>,

    position: RcVar<Point>,
    monitor: RcVar<MonitorQuery>,
    video_mode: RcVar<VideoMode>,

    size: RcVar<Size>,
    pub(super) auto_size: RcVar<AutoSize>,
    auto_size_origin: RcVar<Point>,
    min_size: RcVar<Size>,
    max_size: RcVar<Size>,

    pub(super) actual_position: RcVar<DipPoint>,
    pub(super) actual_monitor: RcVar<Option<MonitorId>>,
    pub(super) actual_size: RcVar<DipSize>,

    pub(super) scale_factor: RcVar<Factor>,

    pub(super) restore_state: RcVar<WindowState>,
    pub(super) restore_rect: RcVar<DipRect>,

    resizable: RcVar<bool>,
    movable: RcVar<bool>,

    always_on_top: RcVar<bool>,

    visible: RcVar<bool>,
    taskbar_visible: RcVar<bool>,

    parent: RcVar<Option<WindowId>>,
    modal: RcVar<bool>,
    pub(super) children: RcVar<LinearSet<WindowId>>,

    color_scheme: RcVar<Option<ColorScheme>>,
    pub(super) actual_color_scheme: RcVar<ColorScheme>,

    pub(super) is_open: RcVar<bool>,
    pub(super) is_loaded: RcVar<bool>,

    frame_capture_mode: RcVar<FrameCaptureMode>,
    pub(super) render_mode: RcVar<RenderMode>,
}

/// Controls properties of an open window using variables.
///
/// You can get the controller for any window using [`Windows::vars`].
///
/// You can get the controller for the current context window using [`req`] or [`get`] and the `window_state`
/// in [`WindowContext`] and [`WidgetContext`].
///
/// [`WindowContext`]: crate::context::WindowContext::window_state
/// [`WidgetContext`]: crate::context::WidgetContext::window_state
/// [`Windows::vars`]: crate::window::Windows::vars
/// [`req`]: WindowVars::req
/// [`get`]: WindowVars::get
pub struct WindowVars(pub(super) Arc<WindowVarsData>);
impl WindowVars {
    pub(super) fn new(default_render_mode: RenderMode, primary_scale_factor: Factor) -> Self {
        let vars = Rc::new(WindowVarsData {
            chrome: var(WindowChrome::Default),
            icon: var(WindowIcon::Default),
            actual_icon: var(None),
            cursor: var(Some(CursorIcon::Default)),
            title: var("".to_text()),

            state: var(WindowState::Normal),
            focus_indicator: var(None),

            position: var(Point::default()),
            monitor: var(MonitorQuery::Primary),
            video_mode: var(VideoMode::default()),
            size: var(Size::new(800, 600)),

            actual_position: var(DipPoint::zero()),
            actual_monitor: var(None),
            actual_size: var(DipSize::zero()),

            scale_factor: var(primary_scale_factor),

            restore_state: var(WindowState::Normal),
            restore_rect: var(DipRect::new(
                DipPoint::new(Dip::new(30), Dip::new(30)),
                DipSize::new(Dip::new(800), Dip::new(600)),
            )),

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
            modal: var(false),
            children: var(LinearSet::new()),

            color_scheme: var(None),
            actual_color_scheme: var(ColorScheme::default()),

            is_open: var(true),
            is_loaded: var(false),

            frame_capture_mode: var(FrameCaptureMode::Sporadic),
            render_mode: var(default_render_mode),
        });
        Self(vars)
    }

    pub(super) fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }

    /// Require the window vars from the window state.
    ///
    /// # Panics
    ///
    /// Panics if called in a custom window context that did not setup the variables.
    pub fn req(window_state: &impl BorrowStateMap<state_map::Window>) -> &Self {
        window_state.borrow().req(&WINDOW_VARS_ID)
    }

    /// Tries to get the window vars from the window state.
    pub fn get(window_state: &impl BorrowStateMap<state_map::Window>) -> Option<&Self> {
        window_state.borrow().get(&WINDOW_VARS_ID)
    }

    /// Window chrome, the non-client area of the window.
    ///
    /// See [`WindowChrome`] for details.
    ///
    /// The default value is [`WindowChrome::Default`].
    pub fn chrome(&self) -> &RcVar<WindowChrome> {
        &self.0.chrome
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
    pub fn icon(&self) -> &RcVar<WindowIcon> {
        &self.0.icon
    }

    /// Window icon image.
    ///
    /// This is `None` if [`icon`] is [`WindowIcon::Default`], otherwise it is an [`Image`]
    /// reference clone.
    ///
    /// [`icon`]: Self::icon
    pub fn actual_icon(&self) -> ReadOnlyRcVar<Option<Image>> {
        self.0.actual_icon.read_only()
    }

    /// Window cursor icon and visibility.
    ///
    /// See [`CursorIcon`] for details.
    ///
    /// The default is [`CursorIcon::Default`], if set to `None` no cursor icon is shown.
    pub fn cursor(&self) -> &RcVar<Option<CursorIcon>> {
        &self.0.cursor
    }

    /// Window title text.
    ///
    /// The default value is `""`.
    pub fn title(&self) -> &RcVar<Text> {
        &self.0.title
    }

    /// Window screen state.
    ///
    /// Minimized, maximized or full-screen. See [`WindowState`] for details.
    ///
    /// The default value is [`WindowState::Normal`]
    pub fn state(&self) -> &RcVar<WindowState> {
        &self.0.state
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
    pub fn monitor(&self) -> &RcVar<MonitorQuery> {
        &self.0.monitor
    }

    /// Video mode for exclusive fullscreen.
    pub fn video_mode(&self) -> &RcVar<VideoMode> {
        &self.0.video_mode
    }

    /// Current monitor hosting the window.
    ///
    /// This is `None` only if the window has not opened yet (before first render) or if
    /// no monitors where found in the operating system or if the window if headless without renderer.
    pub fn actual_monitor(&self) -> ReadOnlyRcVar<Option<MonitorId>> {
        self.0.actual_monitor.read_only()
    }

    /// Current scale factor of the current monitor hosting the window.
    pub fn scale_factor(&self) -> ReadOnlyRcVar<Factor> {
        self.0.scale_factor.read_only()
    }

    /// Window actual position on the [monitor].
    ///
    /// This is a read-only variable that tracks the computed position of the window, it updates every
    /// time the window moves.
    ///
    /// The initial value is `(0, 0)` but this is updated quickly to an actual value. The point
    /// is relative to the origin of the [monitor].
    ///
    /// [monitor]: Self::actual_monitor
    pub fn actual_position(&self) -> ReadOnlyRcVar<DipPoint> {
        self.0.actual_position.read_only()
    }

    /// Window *restore* state.
    ///
    /// The *restore* state that the window must be set to be restored, if the [current state] is [`Maximized`], [`Fullscreen`] or [`Exclusive`]
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
    pub fn restore_state(&self) -> ReadOnlyRcVar<WindowState> {
        self.0.restore_state.read_only()
    }

    /// Window *restore* position and size when restoring to [`Normal`].
    ///
    /// The *restore* rectangle is the window position and size when its state is [`Normal`], when the state is not [`Normal`]
    /// this variable tracks the last normal position and size, it will be the window [`actual_position`] and [`actual_size`] again
    /// when the state is set back to [`Normal`].
    ///
    /// This is a read-only variable, to programmatically set it assign the [`position`] and [`size`] variables, note that
    /// unlike this variable the [`position`] is relative to the [`monitor`] top-left.
    ///
    /// The initial value is `(30, 30).at(800, 600)` but this is updated quickly to an actual position. The point
    /// is relative to the origin of the [`actual_monitor`].
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
    pub fn restore_rect(&self) -> ReadOnlyRcVar<DipRect> {
        self.0.restore_rect.read_only()
    }

    /// Window top-left offset on the [`monitor`] when the window is [`Normal`].
    ///
    /// When a dimension is not a finite value it is computed from other variables.
    /// Relative values are computed in relation to the [`monitor`] size, updating every time the
    /// position or monitor variable updates, not every layout.
    ///
    /// When the user moves the window this value is considered stale, when it updates it overwrites the window position again,
    /// note that the window is only moved if it is in the [`Normal`] state, otherwise only the [`restore_rect`] updates.
    ///
    /// When the the window is moved by the user this variable does **not** update back, to track the current position of the window
    /// use [`actual_position`], to track the *restore* position use [`restore_rect`].
    ///
    /// The default value causes the window or OS to select a value.
    ///
    /// [`restore_rect`]: WindowVars::restore_rect
    /// [`actual_position`]: WindowVars::actual_position
    /// [`monitor`]: WindowVars::monitor
    /// [`Normal`]: WindowState::Normal
    pub fn position(&self) -> &RcVar<Point> {
        &self.0.position
    }

    /// Window actual size on the screen.
    ///
    /// This is a read-only variable that tracks the computed size of the window, it updates every time
    /// the window resizes.
    ///
    /// The initial value is `(0, 0)` but this is updated quickly to an actual value.
    pub fn actual_size(&self) -> ReadOnlyRcVar<DipSize> {
        self.0.actual_size.read_only()
    }

    /// Window width and height on the screen when the window is [`Normal`].
    ///
    /// When a dimension is not a finite value it is computed from other variables.
    /// Relative values are computed in relation to the full-screen size.
    ///
    /// When the user resizes the window this value is considered stale, when it updates it overwrites the window size again,
    /// note that the window is only resized if it is in the [`Normal`] state, otherwise only the [`restore_rect`] updates.
    ///
    /// When the window is resized this variable does **not** updated back, to track the current window size use [`actual_size`],
    /// to track the *restore* size use [`restore_rect`].
    ///
    /// The default value is `(800, 600)`.
    ///
    /// [`actual_size`]: WindowVars::actual_size
    /// [`restore_rect`]: WindowVars::restore_rect
    /// [`Normal`]: WindowState::Normal
    pub fn size(&self) -> &RcVar<Size> {
        &self.0.size
    }

    /// Configure window size-to-content.
    ///
    /// When enabled overwrites [`size`](Self::size), but is still coerced by [`min_size`](Self::min_size)
    /// and [`max_size`](Self::max_size). Auto-size is disabled if the user [manually resizes](Self::resizable).
    ///
    /// The default value is [`AutoSize::DISABLED`].
    pub fn auto_size(&self) -> &RcVar<AutoSize> {
        &self.0.auto_size
    }

    /// The point in the window content that does not move when the window is resized by [`auto_size`].
    ///
    /// When the window size increases it *grows* to the right-bottom, the top-left corner does not move because
    /// the origin of windows it at the top-left and the position did not change, this variables overwrites this origin
    /// for [`auto_size`] resized, the window position is adjusted so that it is the *center* of the resize.
    ///
    /// Note this only applies to auto-resizes, the initial auto-size when the window opens is positioned according to the [`StartPosition`] value.
    ///
    /// The default value is [`Point::top_left`].
    ///
    /// [`auto_size`]: Self::auto_size
    pub fn auto_size_origin(&self) -> &RcVar<Point> {
        &self.0.auto_size_origin
    }

    /// Minimal window width and height constrain on the [`size`].
    ///
    /// When a dimension is not a finite value it fallback to the previous valid value.
    /// Relative values are computed in relation to the full-screen size.
    ///
    /// Note that the operation systems can have their own minimal size that supersedes this variable.
    ///
    /// The default value is `(192, 48)`.
    ///
    /// [`size`]: Self::size
    pub fn min_size(&self) -> &RcVar<Size> {
        &self.0.min_size
    }

    /// Maximal window width and height constrain on the [`size`].
    ///
    /// When a dimension is not a finite value it fallback to the previous valid value.
    /// Relative values are computed in relation to the full-screen size.
    ///
    /// Note that the operation systems can have their own maximal size that supersedes this variable.
    ///
    /// The default value is `(100.pct(), 100.pct())`
    ///
    /// [`size`]: Self::size
    pub fn max_size(&self) -> &RcVar<Size> {
        &self.0.max_size
    }

    /// If the user can resize the window using the window frame.
    ///
    /// Note that even if disabled the window can still be resized from other sources.
    ///
    /// The default value is `true`.
    pub fn resizable(&self) -> &RcVar<bool> {
        &self.0.resizable
    }

    /// If the user can move the window using the window frame.
    ///
    /// Note that even if disabled the window can still be moved from other sources.
    ///
    /// The default value is `true`.
    pub fn movable(&self) -> &RcVar<bool> {
        &self.0.movable
    }

    /// Whether the window should always stay on top of other windows.
    ///
    /// Note this only applies to other windows that are not also "always-on-top".
    ///
    /// The default value is `false`.
    pub fn always_on_top(&self) -> &RcVar<bool> {
        &self.0.always_on_top
    }

    /// If the window is visible on the screen and in the task-bar.
    ///
    /// This variable is observed only after the first frame render, before that the window
    /// is always not visible.
    ///
    /// The default value is `true`.
    pub fn visible(&self) -> &RcVar<bool> {
        &self.0.visible
    }

    /// If the window is visible in the task-bar.
    ///
    /// The default value is `true`.
    pub fn taskbar_visible(&self) -> &RcVar<bool> {
        &self.0.taskbar_visible
    }

    /// Window parent.
    ///
    /// If a parent is set this behavior applies:
    ///
    /// * If the parent is minimized, this window is also minimized.
    /// * If the parent window is maximized, this window is restored.
    /// * This window is always on-top of the parent window.
    /// * If the parent window is closed, this window is also closed.
    /// * If [`modal`] is set, the parent window cannot be focused while this window is open.
    /// * If a [`color_scheme`] is not set, the [`color_scheme`] fallback it the parent's actual scheme.
    ///
    /// The default value is `None`.
    ///
    /// # Validation
    ///
    /// The parent window must exist, it cannot have a parent and the child window cannot have children, it also can't set itself as the parent.
    /// If these condition are not met an error is logged and the parent is set to `None`.
    ///
    /// [`modal`]: Self::modal
    /// [`color_scheme`]: Self::color_scheme
    /// [`actual_color_scheme`]: Self::color_scheme
    pub fn parent(&self) -> &RcVar<Option<WindowId>> {
        &self.0.parent
    }

    /// Configure the [`parent`](Self::parent) connection.
    ///
    /// Value is ignored is `parent` is not set.
    ///
    /// The default value is `false`.
    pub fn modal(&self) -> &RcVar<bool> {
        &self.0.modal
    }

    /// Window children.
    ///
    /// This is a set of other windows that have this window as a [`parent`].
    ///
    /// [`parent`]: Self::parent
    pub fn children(&self) -> ReadOnlyRcVar<LinearSet<WindowId>> {
        self.0.children.read_only()
    }

    /// Override the preferred color scheme.
    ///
    /// If set to `None` the system preference is used, see [`actual_color_scheme`].
    ///
    /// [`actual_color_scheme`]: Self::actual_color_scheme
    pub fn color_scheme(&self) -> &RcVar<Option<ColorScheme>> {
        &self.0.color_scheme
    }

    /// Actual color scheme to use.
    ///
    /// This is the system preference, or [`color_scheme`] if it is set.
    ///
    /// [`color_scheme`]: Self::color_scheme
    pub fn actual_color_scheme(&self) -> ReadOnlyRcVar<ColorScheme> {
        self.0.actual_color_scheme.read_only()
    }

    /// If the window is open.
    ///
    /// This is a read-only variable, it starts set to `true` and will update only once,
    /// when the window finishes closing.
    ///
    /// Note that a window is only actually opened in the operating system after it [`is_loaded`].
    ///
    /// [`is_loaded`]: Self::is_loaded
    pub fn is_open(&self) -> ReadOnlyRcVar<bool> {
        self.0.is_open.read_only()
    }

    /// If the window has finished loading.
    ///
    /// This is a read-only variable, it starts set to `false` and will update only once, after
    /// the first window layout and when all loading handles to the window are released.
    ///
    /// A window is only opened in the view-process once it is loaded, see [`Windows::loading_handle`] for more details.
    ///
    /// [`Windows::loading_handle`]: crate::window::Windows::loading_handle
    pub fn is_loaded(&self) -> ReadOnlyRcVar<bool> {
        self.0.is_loaded.read_only()
    }

    /// The active user attention required indicator.
    ///
    /// This is usually a visual indication on the taskbar icon that prompts the user to focus on the window, it is automatically
    /// changed to `None` once the window receives focus or you can set it to `None` to cancel the indicator.
    ///
    /// Prefer using the [`Focus`] service and advanced [`FocusRequest`] configs instead of setting this variable directly.
    ///
    /// [`Focus`]: crate::focus::Focus
    /// [`FocusRequest`]: crate::focus::FocusRequest
    pub fn focus_indicator(&self) -> &RcVar<Option<FocusIndicator>> {
        &self.0.focus_indicator
    }

    /// The window [`FrameCaptureMode`].
    ///
    /// If set to [`Next`] the value will change to [`Sporadic`] after the frame is rendered.
    ///
    /// Note that setting this to [`Next`] does not cause a frame request. Use [`Updates::render_update`] for that.
    ///
    /// [`Next`]: FrameCaptureMode::Next
    /// [`Sporadic`]: FrameCaptureMode::Sporadic
    /// [`Updates::render_update`]: crate::context::Updates::render_update
    pub fn frame_capture_mode(&self) -> &RcVar<FrameCaptureMode> {
        &self.0.frame_capture_mode
    }

    /// Window actual render mode.
    ///
    /// The initial value is the [`default_render_mode`], it can update after the window is created, when the view-process
    /// actually creates the backend window and after a view-process respawn.
    ///
    /// [`default_render_mode`]: crate::window::Windows::default_render_mode
    pub fn render_mode(&self) -> ReadOnlyRcVar<RenderMode> {
        self.0.render_mode.read_only()
    }
}

pub(super) static WINDOW_VARS_ID: StaticStateId<WindowVars> = StaticStateId::new_unique();
