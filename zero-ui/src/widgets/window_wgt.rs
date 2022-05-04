use crate::core::focus::*;
use crate::core::{
    render::RenderMode,
    window::{HeadlessMonitor, StartPosition, Window},
};
use crate::prelude::new_widget::*;
use crate::properties::events::window::*;

pub mod commands;
pub mod nodes;
pub mod properties;

/// A window container.
///
/// The instance type is [`Window`], that can be given to the [`Windows`](crate::core::window::Windows) service
/// to open a system window that is kept in sync with the window properties set in the widget.
///
/// # Examples
///
/// ```no_run
/// use zero_ui::prelude::*;
///
/// App::default().run_window(|_| {
///     window! {
///         title = "Window 1";
///         content = text("Window 1");
///     }
/// })
/// ```
/// See [`run_window`](crate::core::window::AppRunWindow::run_window) for more details.
#[widget($crate::widgets::window)]
pub mod window {
    use super::*;

    inherit!(container);

    pub use super::{commands, nodes, properties};

    #[doc(inline)]
    pub use nodes::{AnchorMode, AnchorSize, AnchorTransform, LayerIndex, WindowLayers};

    properties! {
        /// Window title.
        properties::title;

        /// Window icon.
        ///
        /// See [`WindowIcon`] for details.
        ///
        /// [`WindowIcon`]: crate::core::window::WindowIcon
        properties::icon;

        /// Window chrome, the non-client area of the window.
        ///
        /// See [`WindowChrome`] for details.
        ///
        /// [`WindowChrome`]: crate::core::window::WindowChrome
        properties::chrome;

        /// Window position when it opens.
        #[allowed_in_when = false]
        start_position(impl IntoValue<StartPosition>) = StartPosition::Default;

        /// Window state.
        ///
        /// If set to a writeable variable it is updated back if the user changes the window state.
        ///
        /// See [`WindowState`] for details.
        ///
        /// [`WindowState`]: crate::core::window::WindowState
        properties::state;

        /// Window position (*x*, *y*).
        ///
        /// The position is computed in relation to the [`monitor`](#wp-monitor) value and is re-applied every
        /// time this property or monitor updates.
        ///
        /// Setting [`Length::Default`] to either *x* or *y* causes the system initial position to be used in both dimensions.
        /// This variable is not updated back if the user moves the window, you can use the [`actual_position`](#wp-actual_position)
        /// to get the computed position.
        ///
        /// You can also set [`x`](#wp-x) and [`y`](#wp-y) as independent properties.
        properties::position;

        /// Window position *x*.
        ///
        /// This property value is the same as the [`position.x`](#wp-position) value.
        properties::x;

        /// Window position *y*.
        ///
        /// This property value is the same as the [`position.y`](#wp-position) value.
        properties::y;

        /// Window size (*width*, *height*).
        ///
        /// Does not include the OS window border.
        ///
        /// You can also set the [`width`](#wp-width) and [`height`](#wp-height) as independent properties.
        properties::size;

        /// Window size *width*.
        ///
        /// This property value is the same as the [`size.width`](#wp-size) value.
        properties::width;

        /// Window size *height*.
        ///
        /// This property value is the same as the [`size.height`](#wp-size) value.
        properties::height;

        /// Window minimum size.
        ///
        /// You can also set the [`min_width`](#wp-min_width) and [`min_height`](#wp-min_height) as independent properties.
        properties::min_size;

        /// Window minimum width.
        ///
        /// This property value is the same as the [`min_size.width`](#wp-min_size) value.
        properties::min_width;

        /// Window minimum height.
        ///
        /// This property value is the same as the [`min_size.height`](#wp-min_size) value.
        properties::min_height;

        /// Window maximum size.
        ///
        /// You can also set the [`max_width`](#wp-max_width) and [`max_height`](#wp-max_height) as independent properties.
        properties::max_size;

        /// Window maximum width.
        ///
        /// This property value is the same as the [`max_size.width`](#wp-max_size) value.
        properties::max_width;

        /// Window maximum height.
        ///
        /// This property value is the same as the [`max_size.height`](#wp-max_size) value.
        properties::max_height;

        /// Window auto-size to content.
        ///
        /// When enabled overwrites [`size`](#wp-size), but is still coerced by [`min_size`](#wp-min_size)
        /// and [`max_size`](#wp-max_size). Auto-size is disabled if the user [manually resizes](#wp-resizable).
        ///
        /// The default value is [`AutoSize::DISABLED`].
        ///
        /// [`AutoSize::DISABLED`]: crate::prelude::AutoSize::DISABLED
        properties::auto_size;

        /// The point in the window content that does not move when the window is resized by [`auto_size`].
        ///
        /// When the window size increases it *grows* to the right-bottom, the top-left corner does not move because
        /// the origin of window position is at the top-left and the position did not change, this variables overwrites this origin
        /// for [`auto_size`] resizes, the window position is adjusted so that it is the *center* of the resize.
        ///
        /// Note this only applies to auto-resizes, the initial auto-size when the window opens is positioned
        /// according to the [`start_position`] value.
        ///
        /// The default value is [`Point::top_left`].
        ///
        /// [`auto_size`]: #wp-auto_size
        /// [`start_position`]: #wp-start_position
        properties::auto_size_origin;

        /// Window background color.
        background_color = rgb(0.1, 0.1, 0.1);

        /// Window clear color.
        ///
        /// Color used to *clear* the previous frame pixels before rendering a new frame.
        /// It is visible if window content does not completely fill the content area, this
        /// can happen if you do not set a background or the background is semi-transparent, also
        /// can happen during very fast resizes.
        properties::clear_color = rgb(0.1, 0.1, 0.1);

        /// Unique identifier of the window root widget.
        #[allowed_in_when = false]
        root_id(impl IntoValue<WidgetId>) = WidgetId::new_unique();

        /// Windows are focus scopes by default.
        focus_scope = true;

        /// Windows cycle TAB navigation by default.
        tab_nav = TabNav::Cycle;

        /// Windows cycle arrow navigation by default.
        directional_nav = DirectionalNav::Cycle;

        /// Windows remember the last focused widget and return focus when the window is focused.
        focus_scope_behavior = FocusScopeOnFocus::LastFocused;

        /// If the user can resize the window.
        ///
        /// Note that the window can still change size, this only disables
        /// the OS window frame controls that change size.
        properties::resizable;

        /// If the window is visible.
        ///
        /// When set to `false` the window and its *taskbar* icon are not visible, that is different
        /// from a minimized window where the icon is still visible.
        properties::visible;

        /// Whether the window should always stay on top of other windows.
        ///
        /// Note this only applies to other windows that are not also "always-on-top".
        ///
        /// The default value is `false`.
        properties::always_on_top;

        /// If the window is visible in the task-bar.
        ///
        /// The default value is `true`.
        properties::taskbar_visible;

        /// If the Inspector can be opened for this window.
        ///
        /// The default value is `true`, but only applies if the built with the `inspector` feature.
        can_inspect(impl IntoVar<bool>) = true;

        /// Monitor used for calculating the [`start_position`], [`position`] and [`size`] of the window.
        ///
        /// When the window is dragged to a different monitor this property does not update, you can use the
        /// [`actual_monitor`] property to get the current monitor.
        ///
        /// You can change this property after the window has opened to move the window to a different monitor,
        /// see [`WindowVars::monitor`] for more details about this function.
        ///
        /// Is the [`MonitorQuery::Primary`] by default.
        ///
        /// [`start_position`]: #wp-start_position
        /// [`position`]: #wp-position
        /// [`size`]: #wp-size
        /// [`WindowVars::monitor`]: crate::core::window::WindowVars::monitor
        /// [`MonitorQuery::Primary`]: crate::core::window::MonitorQuery::Primary
        properties::monitor;

        /// Frame image capture mode.
        ///
        /// This property is specially useful headless windows that are used to render.
        properties::frame_capture_mode;

        /// Extra configuration for the window when run in [headless mode](crate::core::window::WindowMode::is_headless).
        ///
        /// When a window runs in headed mode some values are inferred by window context, such as the scale factor that
        /// is taken from the monitor. In headless mode these values can be configured manually.
        #[allowed_in_when = false]
        headless_monitor(impl IntoValue<HeadlessMonitor>) = HeadlessMonitor::default();

        /// Lock-in kiosk mode.
        ///
        /// In kiosk mode the only window states allowed are full-screen or full-screen exclusive, and
        /// all subsequent windows opened are child of the kiosk window.
        ///
        /// Note that this does not configure the operating system window manager,
        /// you still need to setup a kiosk environment, it does not block `ALT+TAB`. This just stops the
        /// app itself from accidentally exiting kiosk mode.
        #[allowed_in_when = false]
        kiosk(bool) = false;

        /// If semi-transparent content is "see-through", mixin with the OS pixels "behind" the window.
        ///
        /// This is `true` by default, as it avoids the screen flashing black for windows opening in maximized or fullscreen modes
        /// in the Microsoft Windows OS.
        ///
        /// Note that to make use of this feature you must unset the [`clear_color`] and [`background_color`] or set then to
        /// a semi-transparent color. The composition is a simple alpha blend, effects like blur do not apply to
        /// the pixels "behind" the window.
        ///
        /// [`clear_color`]: #wp-clear_color
        /// [`background_color`]: #wp-background_color
        #[allowed_in_when = false]
        allow_transparency(bool) = true;

        /// Render performance mode overwrite for this window, if set to `None` the [`Windows::default_render_mode`] is used.
        ///
        /// # Examples
        ///
        /// Prefer `Dedicated` renderer backend for just this window:
        ///
        /// ```no_run
        /// use zero_ui::prelude::*;
        ///
        /// fn example(ctx: &mut WindowContext) -> Window {
        ///     let selected_mode = ctx.window_state.req(WindowVarsKey).render_mode();
        ///     window! {
        ///         title = "Render Mode";
        ///         render_mode = RenderMode::Dedicated;
        ///         content = text(selected_mode.map(|m| formatx!("Preference: Dedicated\nActual: {m:?}")));
        ///     }
        /// }
        /// ```
        ///
        /// The `view-process` will try to match the mode, if it is not available a fallback mode is selected,
        /// see [`RenderMode`] for more details about each mode and fallbacks.
        ///
        /// [`Windows::default_render_mode`]: crate::core::window::Windows::default_render_mode
        #[allowed_in_when = false]
        render_mode(impl IntoValue<Option<RenderMode>>) = None;

        /// Event just after the window opens.
        ///
        /// This event notifies once per window, after the window content is inited and the first frame was send to the renderer.
        /// Note that the first frame metadata is available in [`Windows::widget_tree`], but it probably has not finished rendering.
        ///
        /// This property is the [`on_pre_window_open`](fn@on_pre_window_open) so window handlers see it first.
        ///
        /// [`Windows::widget_tree`]: crate::core::window::Windows::widget_tree
        on_pre_window_open as on_open;

        /// On window close requested.
        ///
        /// This event notifies every time the user or the app tries to close the window, you can call
        /// [`cancel`](WindowCloseRequestedArgs::cancel) to stop the window from being closed.
        on_window_close_requested as on_close_requested;

        /// On window deinited.
        ///
        /// This event notifies once after the window content is deinited because it is closing.
        crate::properties::events::widget::on_deinit as on_close;

        /// On window position changed.
        ///
        /// This event notifies every time the user or app changes the window position. You can also track the window
        /// position using the [`actual_position`] variable.
        ///
        /// This property is the [`on_pre_window_moved`] so window handlers see it first.
        ///
        /// [`actual_position`]: WindowVars::actual_position
        /// [`on_pre_window_moved`]: fn@on_pre_window_moved
        on_pre_window_moved as on_moved;

        /// On window size changed.
        ///
        /// This event notifies every time the user or app changes the window content area size. You can also track
        /// the window size using the [`actual_size`] variable.
        ///
        /// This property is the [`on_pre_window_resized`] so window handlers see it first.
        ///
        /// [`actual_size`]: WindowVars::actual_size
        /// [`on_pre_window_resized`]: fn@on_pre_window_resized
        on_pre_window_resized as on_resized;

        /// On window state changed.
        ///
        /// This event notifies every time the user or app changes the window state. You can also track the window
        /// state by setting [`state`] to a read-write variable.
        ///
        /// This property is the [`on_pre_window_state_changed`] so window handlers see it first.
        ///
        /// [`state`]: #wp-state
        /// [`on_pre_window_state_changed`]: fn@on_pre_window_state_changed
        on_pre_window_state_changed as on_state_changed;

        /// On window maximized.
        ///
        /// This event notifies every time the user or app changes the window state to maximized.
        ///
        /// This property is the [`on_pre_window_maximized`] so window handlers see it first.
        ///
        /// [`on_pre_window_maximized`]: fn@on_pre_window_maximized
        on_pre_window_maximized as on_maximized;

        /// On window exited the maximized state.
        ///
        /// This event notifies every time the user or app changes the window state to a different state from maximized.
        ///
        /// This property is the [`on_pre_window_unmaximized`] so window handlers see it first.
        ///
        /// [`on_pre_window_unmaximized`]: fn@on_pre_window_unmaximized
        on_pre_window_unmaximized as on_unmaximized;

        /// On window minimized.
        ///
        /// This event notifies every time the user or app changes the window state to maximized.
        ///
        /// This property is the [`on_pre_window_maximized`] so window handlers see it first.
        ///
        /// [`on_pre_window_minimized`]: fn@on_pre_window_minimized
        on_pre_window_minimized as on_minimized;

        /// On window exited the minimized state.
        ///
        /// This event notifies every time the user or app changes the window state to a different state from minimized.
        ///
        /// This property is the [`on_pre_window_unminimized`] so window handlers see it first.
        ///
        /// [`on_pre_window_unminimized`]: fn@on_pre_window_unminimized
        on_pre_window_unminimized as on_unminimized;

        /// On window state changed to [`Normal`].
        ///
        /// This event notifies every time the user or app changes the window state to [`Normal`].
        ///
        /// This property is the [`on_pre_window_restored`] so window handlers see it first.
        ///
        /// [`Normal`]: WindowState::Normal
        /// [`on_pre_window_restored`]: fn@on_pre_window_restored
        on_pre_window_restored as on_restored;

        /// On window enter one of the fullscreen states.
        ///
        /// This event notifies every time the user or app changes the window state to [`Fullscreen`] or [`Exclusive`].
        ///
        /// This property is the [`on_pre_window_fullscreen`] so window handlers see it first.
        ///
        /// [`Fullscreen`]: WindowState::Fullscreen
        /// [`Exclusive`]: WindowState::Exclusive
        /// [`on_pre_window_fullscreen`]: fn@on_pre_window_fullscreen
        on_pre_window_fullscreen as on_fullscreen;

        /// On window is no longer fullscreen.
        ///
        /// This event notifies every time the user or app changed the window state to one that is not fullscreen.
        ///
        /// This property is the [`on_pre_window_exited_fullscreen`] so window handlers see it first.
        ///
        /// [`on_pre_window_exited_fullscreen`]: fn@on_pre_window_exited_fullscreen
        on_pre_window_exited_fullscreen as on_exited_fullscreen;

        /// On window frame rendered.
        ///
        /// If [`frame_image_capture`](#wp-frame_image_capture) is set
        on_pre_frame_image_ready as on_frame_image_ready;

        remove {
            // replaced with `root_id` to more clearly indicate that it is not the window ID.
            id;
            // replaced with `visible` because Visibility::Hidden is not a thing for windows.
            visibility
        }
    }

    fn new_event(child: impl UiNode, #[allow(unused)] can_inspect: impl IntoVar<bool>) -> impl UiNode {
        let child = commands::window_control_node(child);
        #[cfg(inspector)]
        let child = commands::inspect_node(child, can_inspect);

        nodes::layers(child)
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn new(
        child: impl UiNode,
        root_id: impl IntoValue<WidgetId>,
        start_position: impl IntoValue<StartPosition>,
        kiosk: bool,
        allow_transparency: bool,
        render_mode: impl IntoValue<Option<RenderMode>>,
        headless_monitor: impl IntoValue<HeadlessMonitor>,
    ) -> Window {
        Window::new_root(
            root_id,
            start_position,
            kiosk,
            allow_transparency,
            render_mode,
            headless_monitor,
            child,
        )
    }
}
