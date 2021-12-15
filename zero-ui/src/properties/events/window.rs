//! Window events, [`on_window_open`](fn@on_window_open) and [`on_window_close_requested`](fn@on_window_close_requested).
//!
//! These events are re-exported by [`window!`](mod@crate::widgets::window) as `on_open` and `on_close_requested`, but you can
//! attach then in any widget inside a window using the property full name.
//!
//! There is no event property for the [`WindowCloseEvent`](crate::core::window::WindowCloseEvent) because that event notifies
//! after the window is deinited. You can use [`on_deinit`](fn@crate::properties::events::widget::on_deinit) in the
//! [`window!`](mod@crate::widgets::window) widget to handle a window closing, or create an app level handler for the
//! event using [`Events`](crate::core::event::Events).

use super::event_property;
use crate::core::window::*;

event_property! {
    /// On window opened.
    ///
    /// This event notifies once per window, after the window content is inited and the first frame was send to the renderer.
    ///
    /// Note, the frame metadata is available using [`Windows::frame_info`] but the frame pixels are probably not ready yet,
    /// use [`on_frame_pixels_ready`] if you want to copy the pixels of the first frame.
    pub fn window_open {
        event: WindowOpenEvent,
        args: WindowOpenArgs,
    }

    /// On window position changed.
    pub fn window_moved {
        event: WindowMoveEvent,
        args: WindowMoveArgs,
    }

    /// On window size changed.
    pub fn window_resized {
        event: WindowResizeEvent,
        args: WindowResizeArgs,
    }

    /// On window close requested.
    ///
    /// This event notifies every time the user or the app tries to close the window, you can call
    /// [`cancel`](WindowCloseRequestedArgs::cancel) to stop the window from being closed.
    pub fn window_close_requested {
        event: WindowCloseRequestedEvent,
        args: WindowCloseRequestedArgs
    }

    /// On window state changed.
    ///
    /// This event notifies every time the user or the app changes the [`WindowState`].
    pub fn window_state_changed {
        event: WindowStateChangedEvent,
        args: WindowStateChangedArgs,
    }

    /// On window state changed to [`WindowState::Maximized`].
    pub fn window_maximized {
        event: WindowStateChangedEvent,
        args: WindowStateChangedArgs,
        filter: |ctx, args|  args.concerns_widget(ctx) && args.entered_state(WindowState::Maximized),
    }

    /// On window state changed from [`WindowState::Maximized`].
    pub fn window_unmaximized {
        event: WindowStateChangedEvent,
        args: WindowStateChangedArgs,
        filter: |ctx, args|  args.concerns_widget(ctx) && args.exited_state(WindowState::Maximized),
    }

    /// On window state changed to [`WindowState::Minimized`].
    pub fn window_minimized {
        event: WindowStateChangedEvent,
        args: WindowStateChangedArgs,
        filter: |ctx, args|  args.concerns_widget(ctx) && args.entered_state(WindowState::Minimized),
    }

    /// On window state changed from [`WindowState::Minimized`].
    pub fn window_unminimized {
        event: WindowStateChangedEvent,
        args: WindowStateChangedArgs,
        filter: |ctx, args|  args.concerns_widget(ctx) && args.exited_state(WindowState::Minimized),
    }

    /// On window state changed to [`WindowState::Normal`].
    pub fn window_restored {
        event: WindowStateChangedEvent,
        args: WindowStateChangedArgs,
        filter: |ctx, args|  args.concerns_widget(ctx) && args.entered_state(WindowState::Normal),
    }

    /// On window state changed to [`WindowState::Fullscreen`] or [`WindowState::Exclusive`] from a previous not
    /// fullscreen state.
    pub fn window_fullscreen {
        event: WindowStateChangedEvent,
        args: WindowStateChangedArgs,
        filter: |ctx, args|  args.concerns_widget(ctx) && args.entered_fullscreen(),
    }

    /// On window state changed from [`WindowState::Fullscreen`] or [`WindowState::Exclusive`] from a new not
    /// fullscreen state.
    pub fn window_exited_fullscreen {
        event: WindowStateChangedEvent,
        args: WindowStateChangedArgs,
        filter: |ctx, args|  args.concerns_widget(ctx) && args.exited_fullscreen(),
    }

    /// On window frame rendered. The window can also be configured so that the frame pixels are
    /// captured in a *screenshot* that is available in the arguments.
    pub fn frame_image_ready {
        event: FrameImageReadyEvent,
        args: FrameImageReadyArgs,
    }
}
