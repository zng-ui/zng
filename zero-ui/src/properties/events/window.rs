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
    /// This event notifies once per window, as soon as the window is created and the content is inited.
    pub fn window_open {
        event: WindowOpenEvent,
        args: WindowOpenArgs,
    }

    /// On window loaded.
    ///
    /// This event notifies once per window, after the window content is inited, updated, layout and the first frame
    /// was send to the renderer. Windows are considered *loaded* after the first layout and all [`WindowLoadingHandle`]
    /// have expired or dropped.
    ///
    /// [`WindowLoadingHandle`]: crate::core::window::WindowLoadingHandle
    pub fn window_load {
        event: WindowLoadEvent,
        args: WindowOpenArgs,
    }

    /// On window moved, resized or state change.
    pub fn window_changed {
        event: WindowChangedEvent,
        args: WindowChangedArgs,
    }

    /// On window position changed.
    pub fn window_moved {
        event: WindowChangedEvent,
        args: WindowChangedArgs,
        filter: |_, args| args.is_moved(),
    }

    /// On window size changed.
    pub fn window_resized {
        event: WindowChangedEvent,
        args: WindowChangedArgs,
        filter: |_, args| args.is_resized(),
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
        event: WindowChangedEvent,
        args: WindowChangedArgs,
        filter: |_, args| args.is_state_changed(),
    }

    /// On window state changed to [`WindowState::Maximized`].
    pub fn window_maximized {
        event: WindowChangedEvent,
        args: WindowChangedArgs,
        filter: |_, args| args.entered_state(WindowState::Maximized),
    }

    /// On window state changed from [`WindowState::Maximized`].
    pub fn window_unmaximized {
        event: WindowChangedEvent,
        args: WindowChangedArgs,
        filter: |_, args| args.exited_state(WindowState::Maximized),
    }

    /// On window state changed to [`WindowState::Minimized`].
    pub fn window_minimized {
        event: WindowChangedEvent,
        args: WindowChangedArgs,
        filter: |_, args| args.entered_state(WindowState::Minimized),
    }

    /// On window state changed from [`WindowState::Minimized`].
    pub fn window_unminimized {
        event: WindowChangedEvent,
        args: WindowChangedArgs,
        filter: |_, args| args.exited_state(WindowState::Minimized),
    }

    /// On window state changed to [`WindowState::Normal`].
    pub fn window_restored {
        event: WindowChangedEvent,
        args: WindowChangedArgs,
        filter: |_, args| args.entered_state(WindowState::Normal),
    }

    /// On window state changed to [`WindowState::Fullscreen`] or [`WindowState::Exclusive`] from a previous not
    /// fullscreen state.
    pub fn window_fullscreen {
        event: WindowChangedEvent,
        args: WindowChangedArgs,
        filter: |_, args| args.entered_fullscreen(),
    }

    /// On window state changed from [`WindowState::Fullscreen`] or [`WindowState::Exclusive`] from a new not
    /// fullscreen state.
    pub fn window_exited_fullscreen {
        event: WindowChangedEvent,
        args: WindowChangedArgs,
        filter: |_, args| args.exited_fullscreen(),
    }

    /// On window frame rendered. The window can also be configured so that the frame pixels are
    /// captured in a *screenshot* that is available in the arguments.
    pub fn frame_image_ready {
        event: FrameImageReadyEvent,
        args: FrameImageReadyArgs,
    }
}
