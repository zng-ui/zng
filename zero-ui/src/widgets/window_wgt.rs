use crate::core::focus::*;
use crate::core::window::{HeadlessMonitor, StartPosition, Window};
use crate::prelude::new_widget::*;

pub mod commands;
pub mod nodes;
pub mod window_properties;

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

    use crate::properties::{events, focus};

    inherit!(container);

    pub use super::{commands, nodes};

    #[doc(inline)]
    pub use nodes::{AnchorMode, AnchorSize, AnchorTransform, LayerIndex, WindowLayers};

    #[doc(inline)]
    pub use crate::widgets::window_wgt::window_properties::{
        always_on_top, auto_size, auto_size_origin, chrome, color_scheme, font_size, frame_capture_mode, height, icon, max_height,
        max_size, max_width, min_height, min_size, min_width, modal, monitor, movable, parent, position, resizable, size, state,
        taskbar_visible, title, visible, x, y, *,
    };

    properties! {
        /// Window position when it opens.
        pub start_position(impl IntoValue<StartPosition>);

        background_color = color_scheme_map(rgb(0.1, 0.1, 0.1), rgb(0.9, 0.9, 0.9));
        txt_color = color_scheme_map(rgb(0.92, 0.92, 0.92), rgb(0.08, 0.08, 0.08));
        focus_highlight = {
            offsets: FOCUS_HIGHLIGHT_OFFSETS_VAR,
            widths: FOCUS_HIGHLIGHT_WIDTHS_VAR,
            sides: color_scheme_map(
                BorderSides::dashed(rgba(200, 200, 200, 1.0)),
                BorderSides::dashed(colors::BLACK)
            ),
        };
        clear_color = color_scheme_map(rgb(0.1, 0.1, 0.1), rgb(0.9, 0.9, 0.9));
        focus_scope = true;

        /// Windows cycle TAB navigation by default.
        pub focus::tab_nav = TabNav::Cycle;

        /// Windows cycle arrow navigation by default.
        pub focus::directional_nav = DirectionalNav::Cycle;

        /// Windows remember the last focused widget and return focus when the window is focused.
        pub focus::focus_scope_behavior = FocusScopeOnFocus::LastFocused;

        /// If the Inspector can be opened for this window.
        ///
        /// The default value is `true`, but only applies if built with the `inspector` feature.
        #[cfg(inspector)]
        pub can_inspect(impl IntoVar<bool>);


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


        /// Save and restore the window state.
        save_state = SaveState::enabled();

        /// Event just after the window opens.
        ///
        /// This event notifies once per window, after the window content is inited.
        ///
        /// This property is the [`on_pre_window_open`] so window handlers see it first.
        ///
        /// [`on_pre_window_open`]: fn@events::window::on_pre_window_open
        pub events::window::on_pre_window_open as on_open;

        /// Event just after the window loads.
        ///
        /// This event notifies once per window, after the window content is inited, updated, layout and the first frame
        /// was send to the renderer. Windows are considered *loaded* after the first layout and all [`WindowLoadingHandle`]
        /// have expired or dropped.
        ///
        /// This property is the [`on_pre_window_load`] so window handlers see it first.
        ///
        /// [`WindowLoadingHandle`]: crate::core::window::WindowLoadingHandle
        /// [`on_pre_window_load`]: fn@events::window::on_pre_window_load
        pub events::window::on_pre_window_load as on_load;

        /// On window close requested.
        ///
        /// This event notifies every time the user or the app tries to close the window, you can stop propagation
        /// to stop the window from being closed.
        pub events::window::on_window_close_requested as on_close_requested;

        /// On window deinited.
        ///
        /// This event notifies once after the window content is deinited because it is closing.
        pub events::widget::on_deinit as on_close;

        /// On window position changed.
        ///
        /// This event notifies every time the user or app changes the window position. You can also track the window
        /// position using the [`actual_position`] variable.
        ///
        /// This property is the [`on_pre_window_moved`] so window handlers see it first.
        ///
        /// [`actual_position`]: crate::core::window::WindowVars::actual_position
        /// [`on_pre_window_moved`]: fn@events::window::on_pre_window_moved
        pub events::window::on_pre_window_moved as on_moved;

        /// On window size changed.
        ///
        /// This event notifies every time the user or app changes the window content area size. You can also track
        /// the window size using the [`actual_size`] variable.
        ///
        /// This property is the [`on_pre_window_resized`] so window handlers see it first.
        ///
        /// [`actual_size`]: crate::core::window::WindowVars::actual_size
        /// [`on_pre_window_resized`]: fn@events::window::on_pre_window_resized
        pub events::window::on_pre_window_resized as on_resized;

        /// On window state changed.
        ///
        /// This event notifies every time the user or app changes the window state. You can also track the window
        /// state by setting [`state`] to a read-write variable.
        ///
        /// This property is the [`on_pre_window_state_changed`] so window handlers see it first.
        ///
        /// [`state`]: #wp-state
        /// [`on_pre_window_state_changed`]: fn@events::window::on_pre_window_state_changed
        pub events::window::on_pre_window_state_changed as on_state_changed;

        /// On window maximized.
        ///
        /// This event notifies every time the user or app changes the window state to maximized.
        ///
        /// This property is the [`on_pre_window_maximized`] so window handlers see it first.
        ///
        /// [`on_pre_window_maximized`]: fn@events::window::on_pre_window_maximized
        pub events::window::on_pre_window_maximized as on_maximized;

        /// On window exited the maximized state.
        ///
        /// This event notifies every time the user or app changes the window state to a different state from maximized.
        ///
        /// This property is the [`on_pre_window_unmaximized`] so window handlers see it first.
        ///
        /// [`on_pre_window_unmaximized`]: fn@events::window::on_pre_window_unmaximized
        pub events::window::on_pre_window_unmaximized as on_unmaximized;

        /// On window minimized.
        ///
        /// This event notifies every time the user or app changes the window state to maximized.
        ///
        /// This property is the [`on_pre_window_maximized`] so window handlers see it first.
        ///
        /// [`on_pre_window_maximized`]: fn@events::window::on_pre_window_maximized
        pub events::window::on_pre_window_minimized as on_minimized;

        /// On window exited the minimized state.
        ///
        /// This event notifies every time the user or app changes the window state to a different state from minimized.
        ///
        /// This property is the [`on_pre_window_unminimized`] so window handlers see it first.
        ///
        /// [`on_pre_window_unminimized`]: fn@events::window::on_pre_window_unminimized
        pub events::window::on_pre_window_unminimized as on_unminimized;

        /// On window state changed to [`Normal`].
        ///
        /// This event notifies every time the user or app changes the window state to [`Normal`].
        ///
        /// This property is the [`on_pre_window_restored`] so window handlers see it first.
        ///
        /// [`Normal`]: crate::core::window::WindowState::Normal
        /// [`on_pre_window_restored`]: fn@events::window::on_pre_window_restored
        pub events::window::on_pre_window_restored as on_restored;

        /// On window enter one of the fullscreen states.
        ///
        /// This event notifies every time the user or app changes the window state to [`Fullscreen`] or [`Exclusive`].
        ///
        /// This property is the [`on_pre_window_fullscreen`] so window handlers see it first.
        ///
        /// [`Fullscreen`]: crate::core::window::WindowState::Fullscreen
        /// [`Exclusive`]: crate::core::window::WindowState::Exclusive
        /// [`on_pre_window_fullscreen`]: fn@events::window::on_pre_window_fullscreen
        pub events::window::on_pre_window_fullscreen as on_fullscreen;

        /// On window is no longer fullscreen.
        ///
        /// This event notifies every time the user or app changed the window state to one that is not fullscreen.
        ///
        /// This property is the [`on_pre_window_exited_fullscreen`] so window handlers see it first.
        ///
        /// [`on_pre_window_exited_fullscreen`]: fn@events::window::on_pre_window_exited_fullscreen
        pub events::window::on_pre_window_exited_fullscreen as on_exited_fullscreen;

        /// On window frame rendered.
        ///
        /// If [`frame_image_capture`](#wp-frame_image_capture) is set
        pub events::window::on_pre_frame_image_ready as on_frame_image_ready;

        /// Use the `FONT_SIZE_VAR` default as the root font size.
        font_size = crate::widgets::text::FONT_SIZE_VAR;
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(|wgt| {
            #[cfg(inspector)]
            {
                let can_inspect = wgt.capture_var_or_else(property_id!(self::can_inspect), || true);
                wgt.push_intrinsic(NestGroup::EVENT, "inspect_cmd", |child| commands::inspect_node(child, can_inspect));
            }

            wgt.push_intrinsic(NestGroup::EVENT, "layers", nodes::layers);
            wgt.push_intrinsic(NestGroup::CONTEXT, "context", nodes::color_scheme);
        });
    }

    fn build(mut wgt: WidgetBuilder) -> Window {
        Window::new_root(
            wgt.capture_value_or_else(property_id!(self::id), WidgetId::new_unique),
            wgt.capture_value_or_default::<StartPosition>(property_id!(self::start_position)),
            wgt.capture_value_or_default(property_id!(self::kiosk)),
            wgt.capture_value_or_else(property_id!(self::allow_transparency), || true),
            wgt.capture_value_or_default::<Option<RenderMode>>(property_id!(self::render_mode)),
            wgt.capture_value_or_default::<HeadlessMonitor>(property_id!(self::headless_monitor)),
            wgt.capture_value_or_default(property_id!(self::start_focused)),
            wgt.build(),
        )
    }
}
