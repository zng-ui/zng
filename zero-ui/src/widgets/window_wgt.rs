use crate::core::focus::*;
use crate::core::window::{HeadlessMonitor, StartPosition, Window};
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
///         child = text("Window 1");
///     }
/// })
/// ```
/// See [`run_window`](crate::core::window::AppRunWindowExt::run_window) for more details.
#[widget($crate::widgets::window)]
pub mod window {
    use super::*;
    use crate::widgets::mixins::focusable_mixin::vis::*;

    inherit!(container);

    pub use super::{commands, nodes, properties};

    #[doc(inline)]
    pub use nodes::{AnchorMode, AnchorSize, AnchorTransform, LayerIndex, WindowLayers};

    properties! {
        /// Window title.
        pub properties::title;

        /// Window icon.
        ///
        /// See [`WindowIcon`] for details.
        ///
        /// [`WindowIcon`]: crate::core::window::WindowIcon
        pub properties::icon;

        /// Window chrome, the non-client area of the window.
        ///
        /// See [`WindowChrome`] for details.
        ///
        /// [`WindowChrome`]: crate::core::window::WindowChrome
        pub properties::chrome;

        /// Window position when it opens.
        pub start_position(impl IntoValue<StartPosition>);

        /// Window state.
        ///
        /// If set to a writeable variable it is updated back if the user changes the window state.
        ///
        /// See [`WindowState`] for details.
        ///
        /// [`WindowState`]: crate::core::window::WindowState
        pub properties::state;

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
        pub properties::position;

        /// Window position *x*.
        ///
        /// This property value is the same as the [`position.x`](#wp-position) value.
        pub properties::x;

        /// Window position *y*.
        ///
        /// This property value is the same as the [`position.y`](#wp-position) value.
        pub properties::y;

        /// Window size (*width*, *height*).
        ///
        /// Does not include the OS window border.
        ///
        /// You can also set the [`width`](#wp-width) and [`height`](#wp-height) as independent properties.
        pub properties::size;

        /// Window size *width*.
        ///
        /// This property value is the same as the [`size.width`](#wp-size) value.
        pub properties::width;

        /// Window size *height*.
        ///
        /// This property value is the same as the [`size.height`](#wp-size) value.
        pub properties::height;

        /// Window minimum size.
        ///
        /// You can also set the [`min_width`](#wp-min_width) and [`min_height`](#wp-min_height) as independent properties.
        pub properties::min_size;

        /// Window minimum width.
        ///
        /// This property value is the same as the [`min_size.width`](#wp-min_size) value.
        pub properties::min_width;

        /// Window minimum height.
        ///
        /// This property value is the same as the [`min_size.height`](#wp-min_size) value.
        pub properties::min_height;

        /// Window maximum size.
        ///
        /// You can also set the [`max_width`](#wp-max_width) and [`max_height`](#wp-max_height) as independent properties.
        pub properties::max_size;

        /// Window maximum width.
        ///
        /// This property value is the same as the [`max_size.width`](#wp-max_size) value.
        pub properties::max_width;

        /// Window maximum height.
        ///
        /// This property value is the same as the [`max_size.height`](#wp-max_size) value.
        pub properties::max_height;

        /// Window auto-size to content.
        ///
        /// When enabled overwrites [`size`](#wp-size), but is still coerced by [`min_size`](#wp-min_size)
        /// and [`max_size`](#wp-max_size). Auto-size is disabled if the user [manually resizes](#wp-resizable).
        ///
        /// The default value is [`AutoSize::DISABLED`].
        ///
        /// [`AutoSize::DISABLED`]: crate::prelude::AutoSize::DISABLED
        pub properties::auto_size;

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
        pub properties::auto_size_origin;

        /// Parent window ID.
        ///
        /// If a parent is set this behavior applies:
        ///
        /// * If the parent is minimized, this window is also minimized.
        /// * If the parent window is maximized, this window is restored.
        /// * This window is always on-top of the parent window.
        /// * If the parent window is closed, this window is also closed.
        /// * If [`modal`] is set, the parent window cannot be focused while this window is open.
        /// * If a [`color_scheme`] is not set, the [`actual_color_scheme`] fallback is the parent's actual color scheme.
        /// * the window is headless it takes on the [`scale_factor`] of the parent.
        ///
        /// The default value is `None`.
        ///
        /// [`modal`]: #wp-modal
        /// [`color_scheme`]: #wp-color_scheme
        /// [`scale_factor`]: crate::core::window::WindowVars::scale_factor
        /// [`actual_color_scheme`]: crate::core::window::WindowVars::actual_color_scheme
        pub properties::parent;

        /// Window background color.
        pub background_color = color_scheme_map(rgb(0.1, 0.1, 0.1), rgb(0.9, 0.9, 0.9));

        /// Window text color.
        pub text_color = color_scheme_map(rgb(0.92, 0.92, 0.92), rgb(0.08, 0.08, 0.08));

        pub focus_highlight = {
            offsets: FOCUS_HIGHLIGHT_OFFSETS_VAR,
            widths: FOCUS_HIGHLIGHT_WIDTHS_VAR,
            sides: color_scheme_map(
                BorderSides::dashed(rgba(200, 200, 200, 1.0)),
                BorderSides::dashed(colors::BLACK)
            ),
        };

        /// Window clear color.
        ///
        /// Color used to *clear* the previous frame pixels before rendering a new frame.
        /// It is visible if window content does not completely fill the content area, this
        /// can happen if you do not set a background or the background is semi-transparent, also
        /// can happen during very fast resizes.
        pub properties::clear_color = color_scheme_map(rgb(0.1, 0.1, 0.1), rgb(0.9, 0.9, 0.9));

        /// Windows are focus scopes by default.
        focus_scope = true;

        /// Windows cycle TAB navigation by default.
        pub tab_nav = TabNav::Cycle;

        /// Windows cycle arrow navigation by default.
        pub directional_nav = DirectionalNav::Cycle;

        /// Windows remember the last focused widget and return focus when the window is focused.
        pub focus_scope_behavior = FocusScopeOnFocus::LastFocused;

        /// If the user can resize the window.
        ///
        /// Note that the window can still change size, this only disables
        /// the OS window frame controls that change size.
        pub properties::resizable;

        /// If the window is visible.
        ///
        /// When set to `false` the window and its *taskbar* icon are not visible, that is different
        /// from a minimized window where the icon is still visible.
        pub properties::visible;

        /// Whether the window should always stay on top of other windows.
        ///
        /// Note this only applies to other windows that are not also "always-on-top".
        ///
        /// The default value is `false`.
        pub properties::always_on_top;

        /// If the window is visible in the task-bar.
        ///
        /// The default value is `true`.
        pub properties::taskbar_visible;

        /// If the Inspector can be opened for this window.
        ///
        /// The default value is `true`, but only applies if built with the `inspector` feature.
        #[cfg(inspector)]
        pub can_inspect(impl IntoVar<bool>);

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
        /// [`actual_monitor`]: crate::core::window::WindowVars::actual_monitor
        /// [`MonitorQuery::Primary`]: crate::core::window::MonitorQuery::Primary
        pub properties::monitor;

        /// Frame image capture mode.
        ///
        /// This property is specially useful headless windows that are used to render.
        pub properties::frame_capture_mode;

        /// Extra configuration for the window when run in [headless mode](crate::core::window::WindowMode::is_headless).
        ///
        /// When a window runs in headed mode some values are inferred by window context, such as the scale factor that
        /// is taken from the monitor. In headless mode these values can be configured manually.
        pub headless_monitor(impl IntoValue<HeadlessMonitor>);

        /// If the window is forced to be the foreground keyboard focus after opening.
        ///
        /// By default the windows manager decides if the window will receive focus after opening, usually it is focused
        /// only if the process that started the window already has focus. Setting the property to `true` ensures that focus
        /// is moved to the new window, potentially stealing the focus from other apps and disrupting the user.
        pub start_focused(impl IntoValue<bool>);

        /// Lock-in kiosk mode.
        ///
        /// In kiosk mode the only window states allowed are full-screen or full-screen exclusive, and
        /// all subsequent windows opened are child of the kiosk window.
        ///
        /// Note that this does not configure the windows manager,
        /// you still need to setup a kiosk environment, it does not block `ALT+TAB`. This just stops the
        /// app itself from accidentally exiting kiosk mode.
        pub kiosk(impl IntoValue<bool>);

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
        pub allow_transparency(impl IntoValue<bool>);

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
        ///     let selected_mode = WindowVars::req(ctx).render_mode();
        ///     window! {
        ///         title = "Render Mode";
        ///         render_mode = RenderMode::Dedicated;
        ///         child = text(selected_mode.map(|m| formatx!("Preference: Dedicated\nActual: {m:?}")));
        ///     }
        /// }
        /// ```
        ///
        /// The `view-process` will try to match the mode, if it is not available a fallback mode is selected,
        /// see [`RenderMode`] for more details about each mode and fallbacks.
        ///
        /// [`Windows::default_render_mode`]: crate::core::window::Windows::default_render_mode
        pub render_mode(impl IntoValue<Option<RenderMode>>);

        /// Override the preferred color scheme for this window.
        pub properties::color_scheme;

        /// Save and restore the window state.
        pub properties::save_state = properties::SaveState::enabled();

        /// Event just after the window opens.
        ///
        /// This event notifies once per window, after the window content is inited.
        ///
        /// This property is the [`on_pre_window_open`](fn@on_pre_window_open) so window handlers see it first.
        pub on_pre_window_open as on_open;

        /// Event just after the window loads.
        ///
        /// This event notifies once per window, after the window content is inited, updated, layout and the first frame
        /// was send to the renderer. Windows are considered *loaded* after the first layout and all [`WindowLoadingHandle`]
        /// have expired or dropped.
        ///
        /// This property is the [`on_pre_window_load`](fn@on_pre_window_load) so window handlers see it first.
        ///
        /// [`WindowLoadingHandle`]: crate::core::window::WindowLoadingHandle
        pub on_pre_window_load as on_load;

        /// On window close requested.
        ///
        /// This event notifies every time the user or the app tries to close the window, you can stop propagation
        /// to stop the window from being closed.
        pub on_window_close_requested as on_close_requested;

        /// On window deinited.
        ///
        /// This event notifies once after the window content is deinited because it is closing.
        pub crate::properties::events::widget::on_deinit as on_close;

        /// On window position changed.
        ///
        /// This event notifies every time the user or app changes the window position. You can also track the window
        /// position using the [`actual_position`] variable.
        ///
        /// This property is the [`on_pre_window_moved`] so window handlers see it first.
        ///
        /// [`actual_position`]: crate::core::window::WindowVars::actual_position
        /// [`on_pre_window_moved`]: fn@on_pre_window_moved
        pub on_pre_window_moved as on_moved;

        /// On window size changed.
        ///
        /// This event notifies every time the user or app changes the window content area size. You can also track
        /// the window size using the [`actual_size`] variable.
        ///
        /// This property is the [`on_pre_window_resized`] so window handlers see it first.
        ///
        /// [`actual_size`]: crate::core::window::WindowVars::actual_size
        /// [`on_pre_window_resized`]: fn@on_pre_window_resized
        pub on_pre_window_resized as on_resized;

        /// On window state changed.
        ///
        /// This event notifies every time the user or app changes the window state. You can also track the window
        /// state by setting [`state`] to a read-write variable.
        ///
        /// This property is the [`on_pre_window_state_changed`] so window handlers see it first.
        ///
        /// [`state`]: #wp-state
        /// [`on_pre_window_state_changed`]: fn@on_pre_window_state_changed
        pub on_pre_window_state_changed as on_state_changed;

        /// On window maximized.
        ///
        /// This event notifies every time the user or app changes the window state to maximized.
        ///
        /// This property is the [`on_pre_window_maximized`] so window handlers see it first.
        ///
        /// [`on_pre_window_maximized`]: fn@on_pre_window_maximized
        pub on_pre_window_maximized as on_maximized;

        /// On window exited the maximized state.
        ///
        /// This event notifies every time the user or app changes the window state to a different state from maximized.
        ///
        /// This property is the [`on_pre_window_unmaximized`] so window handlers see it first.
        ///
        /// [`on_pre_window_unmaximized`]: fn@on_pre_window_unmaximized
        pub on_pre_window_unmaximized as on_unmaximized;

        /// On window minimized.
        ///
        /// This event notifies every time the user or app changes the window state to maximized.
        ///
        /// This property is the [`on_pre_window_maximized`] so window handlers see it first.
        ///
        /// [`on_pre_window_maximized`]: fn@on_pre_window_maximized
        pub on_pre_window_minimized as on_minimized;

        /// On window exited the minimized state.
        ///
        /// This event notifies every time the user or app changes the window state to a different state from minimized.
        ///
        /// This property is the [`on_pre_window_unminimized`] so window handlers see it first.
        ///
        /// [`on_pre_window_unminimized`]: fn@on_pre_window_unminimized
        pub on_pre_window_unminimized as on_unminimized;

        /// On window state changed to [`Normal`].
        ///
        /// This event notifies every time the user or app changes the window state to [`Normal`].
        ///
        /// This property is the [`on_pre_window_restored`] so window handlers see it first.
        ///
        /// [`Normal`]: crate::core::window::WindowState::Normal
        /// [`on_pre_window_restored`]: fn@on_pre_window_restored
        pub on_pre_window_restored as on_restored;

        /// On window enter one of the fullscreen states.
        ///
        /// This event notifies every time the user or app changes the window state to [`Fullscreen`] or [`Exclusive`].
        ///
        /// This property is the [`on_pre_window_fullscreen`] so window handlers see it first.
        ///
        /// [`Fullscreen`]: crate::core::window::WindowState::Fullscreen
        /// [`Exclusive`]: crate::core::window::WindowState::Exclusive
        /// [`on_pre_window_fullscreen`]: fn@on_pre_window_fullscreen
        pub on_pre_window_fullscreen as on_fullscreen;

        /// On window is no longer fullscreen.
        ///
        /// This event notifies every time the user or app changed the window state to one that is not fullscreen.
        ///
        /// This property is the [`on_pre_window_exited_fullscreen`] so window handlers see it first.
        ///
        /// [`on_pre_window_exited_fullscreen`]: fn@on_pre_window_exited_fullscreen
        pub on_pre_window_exited_fullscreen as on_exited_fullscreen;

        /// On window frame rendered.
        ///
        /// If [`frame_image_capture`](#wp-frame_image_capture) is set
        pub on_pre_frame_image_ready as on_frame_image_ready;
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(|wgt| {
            #[cfg(inspector)]
            {
                let can_inspect = wgt.capture_var_or_else(property_id!(self.can_inspect), || true);
                wgt.push_intrinsic(Priority::Event, "inspect_cmd", |child| commands::inspect_node(child, can_inspect));
            }

            wgt.push_intrinsic(Priority::Event, "layers", nodes::layers);
            wgt.push_intrinsic(Priority::Context, "context", nodes::color_scheme);
        });
    }

    fn build(mut wgt: WidgetBuilder) -> Window {
        Window::new_root(
            wgt.capture_value_or_else(property_id!(self.id), WidgetId::new_unique),
            wgt.capture_value_or_default::<StartPosition>(property_id!(self.start_position)),
            wgt.capture_value_or_default(property_id!(self.kiosk)),
            wgt.capture_value_or_else(property_id!(self.allow_transparency), || true),
            wgt.capture_value_or_default::<Option<RenderMode>>(property_id!(self.render_mode)),
            wgt.capture_value_or_default::<HeadlessMonitor>(property_id!(self.headless_monitor)),
            wgt.capture_value_or_default(property_id!(self.start_focused)),
            wgt.build(),
        )
    }
}
