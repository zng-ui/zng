//! Window widget, properties, properties and nodes..

use crate::core::focus::*;
use crate::core::window::{
    FrameImageReadyArgs, HeadlessMonitor, StartPosition, WindowChangedArgs, WindowCloseRequestedArgs, WindowOpenArgs, WindowRoot,
};
use crate::prelude::new_widget::*;

pub mod commands;
pub mod layers;
pub mod nodes;
mod window_properties;

#[allow(clippy::useless_attribute)] // not useless
#[allow(ambiguous_glob_reexports)] // we override `font_size`.
pub use self::window_properties::*;

/// A window container.
///
/// The instance type is [`WindowRoot`], that can be given to the [`WINDOWS`](crate::core::window::WINDOWS) service
/// to open a system window that is kept in sync with the window properties set in the widget.
///
/// # Examples
///
/// ```no_run
/// use zero_ui::prelude::*;
///
/// App::default().run_window(async {
///     Window! {
///         title = "Window 1";
///         child = Text!("Window 1");
///     }
/// })
/// ```
/// See [`run_window`](crate::core::window::AppRunWindowExt::run_window) for more details.
#[widget($crate::widgets::Window)]
pub struct Window(Container);
impl Window {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            background_color = color_scheme_map(rgb(0.1, 0.1, 0.1), rgb(0.9, 0.9, 0.9));
            font_color = color_scheme_map(rgb(0.92, 0.92, 0.92), rgb(0.08, 0.08, 0.08));
            crate::widgets::focusable_mix::focus_highlight = {
                offsets: crate::widgets::focusable_mix::FOCUS_HIGHLIGHT_OFFSETS_VAR,
                widths: crate::widgets::focusable_mix::FOCUS_HIGHLIGHT_WIDTHS_VAR,
                sides: color_scheme_map(
                    BorderSides::dashed(rgba(200, 200, 200, 1.0)),
                    BorderSides::dashed(colors::BLACK)
                ),
            };
            clear_color = color_scheme_map(rgb(0.1, 0.1, 0.1), rgb(0.9, 0.9, 0.9));
            focus_scope = true;
            tab_nav = TabNav::Cycle;
            directional_nav = DirectionalNav::Cycle;
            focus_scope_behavior = FocusScopeOnFocus::LastFocused;
            save_state = SaveState::enabled();
            // Use the `FONT_SIZE_VAR` default as the root font size.
            font_size = crate::widgets::text::FONT_SIZE_VAR;

            // optimization, actualize `FONT_PALETTE_VAR` early, see `Variables.md`.
            crate::widgets::text::font_palette = crate::widgets::text::FONT_PALETTE_VAR;
        }

        self.widget_builder().push_build_action(|wgt| {
            #[cfg(inspector)]
            {
                let can_inspect = wgt.capture_var_or_else(property_id!(can_inspect), || true);
                wgt.push_intrinsic(NestGroup::EVENT, "inspect_cmd", |child| commands::inspect_node(child, can_inspect));
            }

            wgt.push_intrinsic(NestGroup::EVENT, "layers", nodes::layers);
        });
    }

    /// Build a [`WindowRoot`].
    pub fn widget_build(&mut self) -> WindowRoot {
        let mut wgt = self.widget_take();
        WindowRoot::new(
            wgt.capture_value_or_else(property_id!(Self::id), WidgetId::new_unique),
            wgt.capture_value_or_default::<StartPosition>(property_id!(Self::start_position)),
            wgt.capture_value_or_default(property_id!(Self::kiosk)),
            wgt.capture_value_or_else(property_id!(Self::allow_transparency), || true),
            wgt.capture_value_or_default::<Option<RenderMode>>(property_id!(Self::render_mode)),
            wgt.capture_value_or_default::<HeadlessMonitor>(property_id!(Self::headless_monitor)),
            wgt.capture_value_or_default(property_id!(Self::start_focused)),
            wgt.build(),
        )
    }
}

/// Window position when it opens.
#[property(LAYOUT, capture, widget_impl(Window))]
pub fn start_position(position: impl IntoValue<StartPosition>) {}

/// If the Inspector can be opened for this window.
///
/// The default value is `true`, but only applies if built with the `inspector` feature.
#[cfg(inspector)]
#[property(LAYOUT, capture, widget_impl(Window))]
pub fn can_inspect(enabled: impl IntoVar<bool>) {}

/// Extra configuration for the window when run in [headless mode](crate::core::window::WindowMode::is_headless).
///
/// When a window runs in headed mode some values are inferred by window context, such as the scale factor that
/// is taken from the monitor. In headless mode these values can be configured manually.
#[property(LAYOUT, capture, widget_impl(Window))]
pub fn headless_monitor(monitor: impl IntoValue<HeadlessMonitor>) {}

/// If the window is forced to be the foreground keyboard focus after opening.
///
/// By default the windows manager decides if the window will receive focus after opening, usually it is focused
/// only if the process that started the window already has focus. Setting the property to `true` ensures that focus
/// is moved to the new window, potentially stealing the focus from other apps and disrupting the user.
#[property(CONTEXT, capture, widget_impl(Window))]
pub fn start_focused(enabled: impl IntoValue<bool>) {}

/// Lock-in kiosk mode.
///
/// In kiosk mode the only window states allowed are full-screen or full-screen exclusive, and
/// all subsequent windows opened are child of the kiosk window.
///
/// Note that this does not configure the windows manager,
/// you still need to setup a kiosk environment, it does not block `ALT+TAB`. This just stops the
/// app itself from accidentally exiting kiosk mode.
#[property(CONTEXT, capture, widget_impl(Window))]
pub fn kiosk(kiosk: impl IntoValue<bool>) {}

/// If semi-transparent content is "see-through", mixin with the OS pixels "behind" the window.
///
/// This is `true` by default, as it avoids the screen flashing black for windows opening in maximized or fullscreen modes
/// in the Microsoft Windows OS.
///
/// Note that to make use of this feature you must unset the [`clear_color`] and [`background_color`] or set then to
/// a semi-transparent color. The composition is a simple alpha blend, effects like blur do not apply to
/// the pixels "behind" the window.
///
/// [`clear_color`]: fn@clear_color
/// [`background_color`]: fn@background_color
#[property(CONTEXT, capture, widget_impl(Window))]
pub fn allow_transparency(allow: impl IntoValue<bool>) {}

/// Render performance mode overwrite for this window, if set to `None` the [`WINDOWS.default_render_mode`] is used.
///
/// # Examples
///
/// Prefer `Dedicated` renderer backend for just this window:
///
/// ```no_run
/// use zero_ui::prelude::*;
///
/// fn example() -> WindowRoot {
///     let selected_mode = WINDOW.vars().render_mode();
///     Window! {
///         title = "Render Mode";
///         render_mode = RenderMode::Dedicated;
///         child = Text!(selected_mode.map(|m| formatx!("Preference: Dedicated\nActual: {m:?}")));
///     }
/// }
/// ```
///
/// The `view-process` will try to match the mode, if it is not available a fallback mode is selected,
/// see [`RenderMode`] for more details about each mode and fallbacks.
///
/// [`WINDOWS.default_render_mode`]: crate::core::window::WINDOWS::default_render_mode
#[property(CONTEXT, capture, widget_impl(Window))]
pub fn render_mode(mode: impl IntoValue<Option<RenderMode>>) {}

/// Event just after the window opens.
///
/// This event notifies once per window, after the window content is inited.
///
/// This property handles the same event as [`on_pre_window_open`] so window handlers see it first.
///
/// [`on_pre_window_open`]: fn@events::window::on_pre_window_open
#[property(EVENT, widget_impl(Window))]
pub fn on_open(child: impl UiNode, handler: impl WidgetHandler<WindowOpenArgs>) -> impl UiNode {
    events::window::on_pre_window_open(child, handler)
}

/// Event just after the window loads.
///
/// This event notifies once per window, after the window content is inited, updated, layout and the first frame
/// was send to the renderer. Windows are considered *loaded* after the first layout and all [`WindowLoadingHandle`]
/// have expired or dropped.
///
/// This property handles the same event as [`on_pre_window_load`] so window handlers see it first.
///
/// [`WindowLoadingHandle`]: crate::core::window::WindowLoadingHandle
/// [`on_pre_window_load`]: fn@events::window::on_pre_window_load
#[property(EVENT, widget_impl(Window))]
pub fn on_load(child: impl UiNode, handler: impl WidgetHandler<WindowOpenArgs>) -> impl UiNode {
    events::window::on_pre_window_load(child, handler)
}

/// On window close requested.
///
/// This event notifies every time the user or the app tries to close the window, you can stop propagation
/// to stop the window from being closed.
#[property(EVENT, widget_impl(Window))]
pub fn on_close_requested(child: impl UiNode, handler: impl WidgetHandler<WindowCloseRequestedArgs>) -> impl UiNode {
    events::window::on_window_close_requested(child, handler)
}

/// On window deinited.
///
/// This event notifies once after the window content is deinited because it is closing.
#[property(EVENT, widget_impl(Window))]
pub fn on_close(child: impl UiNode, handler: impl WidgetHandler<events::widget::OnNodeOpArgs>) -> impl UiNode {
    events::widget::on_deinit(child, handler)
}

/// On window position changed.
///
/// This event notifies every time the user or app changes the window position. You can also track the window
/// position using the [`actual_position`] variable.
///
/// This property handles the same event as [`on_pre_window_moved`] so window handlers see it first.
///
/// [`actual_position`]: crate::core::window::WindowVars::actual_position
/// [`on_pre_window_moved`]: fn@events::window::on_pre_window_moved
#[property(EVENT, widget_impl(Window))]
pub fn on_moved(child: impl UiNode, handler: impl WidgetHandler<WindowChangedArgs>) -> impl UiNode {
    events::window::on_pre_window_moved(child, handler)
}

/// On window size changed.
///
/// This event notifies every time the user or app changes the window content area size. You can also track
/// the window size using the [`actual_size`] variable.
///
/// This property handles the same event as [`on_pre_window_resized`] so window handlers see it first.
///
/// [`actual_size`]: crate::core::window::WindowVars::actual_size
/// [`on_pre_window_resized`]: fn@events::window::on_pre_window_resized
#[property(EVENT, widget_impl(Window))]
pub fn on_resized(child: impl UiNode, handler: impl WidgetHandler<WindowChangedArgs>) -> impl UiNode {
    events::window::on_pre_window_resized(child, handler)
}

/// On window state changed.
///
/// This event notifies every time the user or app changes the window state. You can also track the window
/// state by setting [`state`] to a read-write variable.
///
/// This property handles the same event as [`on_pre_window_state_changed`] so window handlers see it first.
///
/// [`state`]: fn@state
/// [`on_pre_window_state_changed`]: fn@events::window::on_pre_window_state_changed
#[property(EVENT, widget_impl(Window))]
pub fn on_state_changed(child: impl UiNode, handler: impl WidgetHandler<WindowChangedArgs>) -> impl UiNode {
    events::window::on_pre_window_state_changed(child, handler)
}

/// On window maximized.
///
/// This event notifies every time the user or app changes the window state to maximized.
///
/// This property handles the same event as [`on_pre_window_maximized`] so window handlers see it first.
///
/// [`on_pre_window_maximized`]: fn@events::window::on_pre_window_maximized
#[property(EVENT, widget_impl(Window))]
pub fn on_maximized(child: impl UiNode, handler: impl WidgetHandler<WindowChangedArgs>) -> impl UiNode {
    events::window::on_pre_window_maximized(child, handler)
}

/// On window exited the maximized state.
///
/// This event notifies every time the user or app changes the window state to a different state from maximized.
///
/// This property handles the same event as [`on_pre_window_unmaximized`] so window handlers see it first.
///
/// [`on_pre_window_unmaximized`]: fn@events::window::on_pre_window_unmaximized
#[property(EVENT, widget_impl(Window))]
pub fn on_unmaximized(child: impl UiNode, handler: impl WidgetHandler<WindowChangedArgs>) -> impl UiNode {
    events::window::on_pre_window_unmaximized(child, handler)
}

/// On window minimized.
///
/// This event notifies every time the user or app changes the window state to maximized.
///
/// This property handles the same event as [`on_pre_window_maximized`] so window handlers see it first.
///
/// [`on_pre_window_maximized`]: fn@events::window::on_pre_window_maximized
#[property(EVENT, widget_impl(Window))]
pub fn on_minimized(child: impl UiNode, handler: impl WidgetHandler<WindowChangedArgs>) -> impl UiNode {
    events::window::on_pre_window_minimized(child, handler)
}

/// On window exited the minimized state.
///
/// This event notifies every time the user or app changes the window state to a different state from minimized.
///
/// This property handles the same event as [`on_pre_window_unminimized`] so window handlers see it first.
///
/// [`on_pre_window_unminimized`]: fn@events::window::on_pre_window_unminimized
#[property(EVENT, widget_impl(Window))]
pub fn on_unminimized(child: impl UiNode, handler: impl WidgetHandler<WindowChangedArgs>) -> impl UiNode {
    events::window::on_pre_window_unminimized(child, handler)
}

/// On window state changed to [`Normal`].
///
/// This event notifies every time the user or app changes the window state to [`Normal`].
///
/// This property handles the same event as [`on_pre_window_restored`] so window handlers see it first.
///
/// [`Normal`]: crate::core::window::WindowState::Normal
/// [`on_pre_window_restored`]: fn@events::window::on_pre_window_restored
#[property(EVENT, widget_impl(Window))]
pub fn on_restored(child: impl UiNode, handler: impl WidgetHandler<WindowChangedArgs>) -> impl UiNode {
    events::window::on_pre_window_restored(child, handler)
}

/// On window enter one of the fullscreen states.
///
/// This event notifies every time the user or app changes the window state to [`Fullscreen`] or [`Exclusive`].
///
/// This property handles the same event as [`on_pre_window_fullscreen`] so window handlers see it first.
///
/// [`Fullscreen`]: crate::core::window::WindowState::Fullscreen
/// [`Exclusive`]: crate::core::window::WindowState::Exclusive
/// [`on_pre_window_fullscreen`]: fn@events::window::on_pre_window_fullscreen
#[property(EVENT, widget_impl(Window))]
pub fn on_fullscreen(child: impl UiNode, handler: impl WidgetHandler<WindowChangedArgs>) -> impl UiNode {
    events::window::on_pre_window_fullscreen(child, handler)
}

/// On window is no longer fullscreen.
///
/// This event notifies every time the user or app changed the window state to one that is not fullscreen.
///
/// This property handles the same event as [`on_pre_window_exited_fullscreen`] so window handlers see it first.
///
/// [`on_pre_window_exited_fullscreen`]: fn@events::window::on_pre_window_exited_fullscreen
#[property(EVENT, widget_impl(Window))]
pub fn on_exited_fullscreen(child: impl UiNode, handler: impl WidgetHandler<WindowChangedArgs>) -> impl UiNode {
    events::window::on_pre_window_exited_fullscreen(child, handler)
}

/// On window frame rendered.
///
/// If [`frame_capture_mode`](fn@frame_capture_mode) is set the image will be available in the event args.
#[property(EVENT, widget_impl(Window))]
pub fn on_frame_image_ready(child: impl UiNode, handler: impl WidgetHandler<FrameImageReadyArgs>) -> impl UiNode {
    events::window::on_pre_frame_image_ready(child, handler)
}
