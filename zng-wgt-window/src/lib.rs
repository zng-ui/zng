#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
//!
//! Window widget, properties, properties and nodes.

#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use zng_ext_input::focus::{DirectionalNav, FocusScopeOnFocus, TabNav};
use zng_ext_window::{
    FrameImageReadyArgs, HeadlessMonitor, RenderMode, StartPosition, WindowChangedArgs, WindowCloseRequestedArgs, WindowOpenArgs,
    WindowRoot,
};
use zng_wgt::prelude::*;
use zng_wgt_fill::background_color;
use zng_wgt_input::focus::{
    directional_nav, focus_highlight, focus_scope, focus_scope_behavior, tab_nav, FOCUS_HIGHLIGHT_OFFSETS_VAR, FOCUS_HIGHLIGHT_WIDTHS_VAR,
};
use zng_wgt_text::{font_color, lang, FONT_SIZE_VAR};

pub mod events;
mod window_properties;

#[allow(clippy::useless_attribute)] // not useless
#[allow(ambiguous_glob_reexports)] // we override `font_size`.
pub use self::window_properties::*;

/// A window container.
///
/// The instance type is [`WindowRoot`], it can be given to the [`WINDOWS`](zng_ext_window::WINDOWS) service
/// to open a system window that is kept in sync with the window properties set in the widget.
///
/// See [`run_window`](zng_ext_window::AppRunWindowExt::run_window) for more details.
#[widget($crate::Window)]
pub struct Window(zng_wgt_container::Container);
impl Window {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            // set the root font size
            font_size = FONT_SIZE_VAR;

            // optimization, actualize mapping context-vars early, see `context_var!` docs.
            zng_wgt_text::font_palette = zng_wgt_text::FONT_PALETTE_VAR;

            // set layout direction.
            lang = zng_ext_l10n::LANG_VAR;

            font_color = color_scheme_map(rgb(0.92, 0.92, 0.92), rgb(0.08, 0.08, 0.08));
            background_color = color_scheme_map(rgb(0.1, 0.1, 0.1), rgb(0.9, 0.9, 0.9));
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
            tab_nav = TabNav::Cycle;
            directional_nav = DirectionalNav::Cycle;
            focus_scope_behavior = FocusScopeOnFocus::LastFocused;
            save_state = SaveState::enabled();
        }

        self.widget_builder().push_build_action(|wgt| {
            wgt.push_intrinsic(NestGroup::EVENT, "layers", zng_wgt_layer::layers_node);
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

/// Defines how the window is positioned when it first opens.
#[property(LAYOUT, capture, widget_impl(Window))]
pub fn start_position(position: impl IntoValue<StartPosition>) {}

/// If the window is steals keyboard focus on open.
///
/// By default the operating system decides if the window will receive focus after opening, usually it is focused
/// only if the process that started the window already has focus. Enabling this ensures that focus
/// is moved to the new window, potentially stealing the focus from other apps and disrupting the user.
#[property(CONTEXT, capture, widget_impl(Window))]
pub fn start_focused(enabled: impl IntoValue<bool>) {}

/// Lock-in kiosk mode.
///
/// In kiosk mode the only window states allowed are fullscreen or fullscreen exclusive, and
/// all subsequent windows opened are child of the kiosk window.
///
/// Note that this does not configure the operating system,
/// you still need to setup a kiosk environment. This just stops the
/// app itself from accidentally exiting fullscreen.
#[property(CONTEXT, capture, widget_impl(Window))]
pub fn kiosk(kiosk: impl IntoValue<bool>) {}

/// If semi-transparent content is see-through, mixing with the operating system pixels behind the window.
///
/// Note that to actually see behind the window you must set the [`clear_color`] and [`background_color`] to a transparent color.
/// The composition is a simple alpha blend, effects like blur do not apply to the pixels behind the window.
///
/// [`clear_color`]: fn@clear_color
/// [`background_color`]: fn@background_color
#[property(CONTEXT, capture, widget_impl(Window))]
pub fn allow_transparency(allow: impl IntoValue<bool>) {}

/// Render performance mode overwrite for this window, if set to `None` the [`WINDOWS.default_render_mode`] is used.
///
/// The `view-process` will try to match the mode, if it is not available a fallback mode is selected,
/// see [`RenderMode`] for more details about each mode and fallbacks.
///
/// [`WINDOWS.default_render_mode`]: zng_ext_window::WINDOWS::default_render_mode
#[property(CONTEXT, capture, widget_impl(Window))]
pub fn render_mode(mode: impl IntoValue<Option<RenderMode>>) {}

/// Event just after the window opens.
///
/// This event notifies once per window, after the window content is inited.
///
/// This property is the same as [`on_pre_window_open`].
///
/// [`on_pre_window_open`]: fn@events::on_pre_window_open
#[property(EVENT, widget_impl(Window))]
pub fn on_open(child: impl UiNode, handler: impl WidgetHandler<WindowOpenArgs>) -> impl UiNode {
    events::on_pre_window_open(child, handler)
}

/// Event just after the window loads.
///
/// This event notifies once per window, after the first layout and all [`WindowLoadingHandle`]
/// have expired or dropped.
///
/// This property is the same as [`on_pre_window_load`].
///
/// [`WindowLoadingHandle`]: zng_ext_window::WindowLoadingHandle
/// [`on_pre_window_load`]: fn@events::on_pre_window_load
#[property(EVENT, widget_impl(Window))]
pub fn on_load(child: impl UiNode, handler: impl WidgetHandler<WindowOpenArgs>) -> impl UiNode {
    events::on_pre_window_load(child, handler)
}

/// On window close requested.
///
/// This event notifies every time an attempt to close the window is made. Close can be cancelled by stopping propagation
/// on the event args, the window only closes after all handlers receive this event and propagation is not stopped.
///
/// This property is the same as [`on_window_close_requested`].
///
/// [`on_window_close_requested`]: fn@events::on_window_close_requested
#[property(EVENT, widget_impl(Window))]
pub fn on_close_requested(child: impl UiNode, handler: impl WidgetHandler<WindowCloseRequestedArgs>) -> impl UiNode {
    events::on_window_close_requested(child, handler)
}

/// On window deinit.
///
/// This event notifies once on deinit.
///
/// This property is the same as [`on_deinit`].
///
/// [`on_deinit`]: fn@zng_wgt::on_deinit
#[property(EVENT, widget_impl(Window))]
pub fn on_close(child: impl UiNode, handler: impl WidgetHandler<zng_wgt::OnNodeOpArgs>) -> impl UiNode {
    zng_wgt::on_deinit(child, handler)
}

/// On window position changed.
///
/// This event notifies every time the window position changes. You can also track the window
/// position using the [`actual_position`] variable.
///
/// This property is the same as [`on_pre_window_moved`].
///
/// [`actual_position`]: zng_ext_window::WindowVars::actual_position
/// [`on_pre_window_moved`]: fn@events::on_pre_window_moved
#[property(EVENT, widget_impl(Window))]
pub fn on_moved(child: impl UiNode, handler: impl WidgetHandler<WindowChangedArgs>) -> impl UiNode {
    events::on_pre_window_moved(child, handler)
}

/// On window size changed.
///
/// This event notifies every time the window content area size changes. You can also track
/// the window size using the [`actual_size`] variable.
///
/// This property is the same as [`on_pre_window_resized`].
///
/// [`actual_size`]: zng_ext_window::WindowVars::actual_size
/// [`on_pre_window_resized`]: fn@events::on_pre_window_resized
#[property(EVENT, widget_impl(Window))]
pub fn on_resized(child: impl UiNode, handler: impl WidgetHandler<WindowChangedArgs>) -> impl UiNode {
    events::on_pre_window_resized(child, handler)
}

/// On window state changed.
///
/// This event notifies every time the window state changes.
///
/// Note that you can also track the window
/// state by setting [`state`] to a read-write variable.
///
/// This property is the same as [`on_pre_window_state_changed`].
///
/// [`state`]: fn@state
/// [`on_pre_window_state_changed`]: fn@events::on_pre_window_state_changed
#[property(EVENT, widget_impl(Window))]
pub fn on_state_changed(child: impl UiNode, handler: impl WidgetHandler<WindowChangedArgs>) -> impl UiNode {
    events::on_pre_window_state_changed(child, handler)
}

/// On window maximized.
///
/// This event notifies every time the window state changes to maximized.
///
/// This property is the same as [`on_pre_window_maximized`].
///
/// [`on_pre_window_maximized`]: fn@events::on_pre_window_maximized
#[property(EVENT, widget_impl(Window))]
pub fn on_maximized(child: impl UiNode, handler: impl WidgetHandler<WindowChangedArgs>) -> impl UiNode {
    events::on_pre_window_maximized(child, handler)
}

/// On window exited the maximized state.
///
/// This event notifies every time the window state changes to a different state from maximized.
///
/// This property is the same as [`on_pre_window_unmaximized`].
///
/// [`on_pre_window_unmaximized`]: fn@events::on_pre_window_unmaximized
#[property(EVENT, widget_impl(Window))]
pub fn on_unmaximized(child: impl UiNode, handler: impl WidgetHandler<WindowChangedArgs>) -> impl UiNode {
    events::on_pre_window_unmaximized(child, handler)
}

/// On window minimized.
///
/// This event notifies every time the window state changes to minimized.
///
/// This property is the same as [`on_pre_window_maximized`].
///
/// [`on_pre_window_maximized`]: fn@events::on_pre_window_maximized
#[property(EVENT, widget_impl(Window))]
pub fn on_minimized(child: impl UiNode, handler: impl WidgetHandler<WindowChangedArgs>) -> impl UiNode {
    events::on_pre_window_minimized(child, handler)
}

/// On window exited the minimized state.
///
/// This event notifies every time the window state changes to a different state from minimized.
///
/// This property is the same as [`on_pre_window_unminimized`].
///
/// [`on_pre_window_unminimized`]: fn@events::on_pre_window_unminimized
#[property(EVENT, widget_impl(Window))]
pub fn on_unminimized(child: impl UiNode, handler: impl WidgetHandler<WindowChangedArgs>) -> impl UiNode {
    events::on_pre_window_unminimized(child, handler)
}

/// On window state changed to [`Normal`].
///
/// This event notifies every time the window state changes to [`Normal`].
///
/// This property is the same as [`on_pre_window_restored`].
///
/// [`Normal`]: zng_ext_window::WindowState::Normal
/// [`on_pre_window_restored`]: fn@events::on_pre_window_restored
#[property(EVENT, widget_impl(Window))]
pub fn on_restored(child: impl UiNode, handler: impl WidgetHandler<WindowChangedArgs>) -> impl UiNode {
    events::on_pre_window_restored(child, handler)
}

/// On window enter one of the fullscreen states.
///
/// This event notifies every time the window state changes to [`Fullscreen`] or [`Exclusive`].
///
/// This property is the same as [`on_pre_window_fullscreen`].
///
/// [`Fullscreen`]: zng_ext_window::WindowState::Fullscreen
/// [`Exclusive`]: zng_ext_window::WindowState::Exclusive
/// [`on_pre_window_fullscreen`]: fn@events::on_pre_window_fullscreen
#[property(EVENT, widget_impl(Window))]
pub fn on_fullscreen(child: impl UiNode, handler: impl WidgetHandler<WindowChangedArgs>) -> impl UiNode {
    events::on_pre_window_fullscreen(child, handler)
}

/// On window is no longer fullscreen.
///
/// This event notifies every time the window state changes to one that is not fullscreen.
///
/// This property is the same as [`on_pre_window_exited_fullscreen`].
///
/// [`on_pre_window_exited_fullscreen`]: fn@events::on_pre_window_exited_fullscreen
#[property(EVENT, widget_impl(Window))]
pub fn on_exited_fullscreen(child: impl UiNode, handler: impl WidgetHandler<WindowChangedArgs>) -> impl UiNode {
    events::on_pre_window_exited_fullscreen(child, handler)
}

/// On window frame rendered.
///
/// If [`frame_capture_mode`](fn@frame_capture_mode) is set the image will be available in the event args.
#[property(EVENT, widget_impl(Window))]
pub fn on_frame_image_ready(child: impl UiNode, handler: impl WidgetHandler<FrameImageReadyArgs>) -> impl UiNode {
    events::on_pre_frame_image_ready(child, handler)
}

/// Imaginary monitor used by the window when it runs in [headless mode](zng_app::window::WindowMode::is_headless).
#[property(LAYOUT, capture, widget_impl(Window))]
pub fn headless_monitor(monitor: impl IntoValue<HeadlessMonitor>) {}
