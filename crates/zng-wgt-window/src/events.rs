//! Window events, [`on_window_open`](fn@on_window_open) and [`on_window_close_requested`](fn@on_window_close_requested).
//!
//! These events are re-exported by [`Window!`](struct@crate::Window) as `on_open` and `on_close_requested`, but you can
//! attach then in any widget inside a window using the property full name.
//!
//! There is no event property for the [`WINDOW_CLOSE_EVENT`] because that event notifies
//! after the window is deinited. You can use [`on_deinit`](fn@zng_wgt::on_deinit) in the
//! [`Window!`](struct@crate::Window) widget to handle a window closing, or create an app level handler for the
//! event using [`EVENTS`](zng_app::event::EVENTS).
//!
//! [`WINDOW_CLOSE_EVENT`]: zng_ext_window::WINDOW_CLOSE_EVENT

use zng_ext_window::*;
use zng_wgt::prelude::*;

event_property! {
    /// On window opened.
    ///
    /// This event notifies once per window, after the window content is inited.
    pub fn window_open {
        event: WINDOW_OPEN_EVENT,
        args: WindowOpenArgs,
        filter: |args| args.window_id == WINDOW.id(),
    }

    /// On window loaded.
    ///
    /// This event notifies once per window, after the first layout and all [`WindowLoadingHandle`]
    /// have expired or dropped.
    ///
    /// [`WindowLoadingHandle`]: zng_ext_window::WindowLoadingHandle
    pub fn window_load {
        event: WINDOW_LOAD_EVENT,
        args: WindowOpenArgs,
        filter: |args| args.window_id == WINDOW.id(),
    }

    /// On window moved, resized or other state changed.
    ///
    /// This event aggregates events moves, resizes and other state changes into a
    /// single event to simplify tracking composite changes, for example, the window changes size and position
    /// when maximized, this can be trivially observed with this event.
    pub fn window_changed {
        event: WINDOW_CHANGED_EVENT,
        args: WindowChangedArgs,
        filter: |args| args.window_id == WINDOW.id(),
    }

    /// On window position changed.
    pub fn window_moved {
        event: WINDOW_CHANGED_EVENT,
        args: WindowChangedArgs,
        filter: |args| args.window_id == WINDOW.id() && args.is_moved(),
    }

    /// On window size changed.
    pub fn window_resized {
        event: WINDOW_CHANGED_EVENT,
        args: WindowChangedArgs,
        filter: |args| args.window_id == WINDOW.id() && args.is_resized(),
    }

    /// On window close requested.
    ///
    /// Calling `propagation().stop()` on this event cancels the window close.
    pub fn window_close_requested {
        event: WINDOW_CLOSE_REQUESTED_EVENT,
        args: WindowCloseRequestedArgs,
        filter: |args| args.windows.contains(&WINDOW.id()),
    }

    /// On window closed.
    ///
    /// See [`WINDOW_CLOSE_EVENT`] for more details of when this event notifies.
    pub fn window_close {
        event: WINDOW_CLOSE_EVENT,
        args: WindowCloseArgs,
        filter: |args| args.windows.contains(&WINDOW.id()),
    }

    /// On window state changed.
    ///
    /// This event notifies every time the user or the app changes the [`WindowVars::state`].
    ///
    /// [`WindowVars::state`]: zng_ext_window::WindowVars
    pub fn window_state_changed {
        event: WINDOW_CHANGED_EVENT,
        args: WindowChangedArgs,
        filter: |args| args.window_id == WINDOW.id() && args.is_state_changed(),
    }

    /// On window state changed to [`WindowState::Maximized`].
    ///
    /// [`WindowState::Maximized`]: zng_ext_window::WindowState::Maximized
    pub fn window_maximized {
        event: WINDOW_CHANGED_EVENT,
        args: WindowChangedArgs,
        filter: |args| args.window_id == WINDOW.id() && args.entered_state(WindowState::Maximized),
    }

    /// On window state changed from [`WindowState::Maximized`].
    ///
    /// [`WindowState::Maximized`]: zng_ext_window::WindowState::Maximized
    pub fn window_unmaximized {
        event: WINDOW_CHANGED_EVENT,
        args: WindowChangedArgs,
        filter: |args| args.window_id == WINDOW.id() && args.exited_state(WindowState::Maximized),
    }

    /// On window state changed to [`WindowState::Minimized`].
    ///
    /// [`WindowState::Minimized`]: zng_ext_window::WindowState::Minimized
    pub fn window_minimized {
        event: WINDOW_CHANGED_EVENT,
        args: WindowChangedArgs,
        filter: |args| args.window_id == WINDOW.id() && args.entered_state(WindowState::Minimized),
    }

    /// On window state changed from [`WindowState::Minimized`].
    ///
    /// [`WindowState::Minimized`]: zng_ext_window::WindowState::Minimized
    pub fn window_unminimized {
        event: WINDOW_CHANGED_EVENT,
        args: WindowChangedArgs,
        filter: |args| args.window_id == WINDOW.id() && args.exited_state(WindowState::Minimized),
    }

    /// On window state changed to [`WindowState::Normal`].
    ///
    /// [`WindowState::Normal`]: zng_ext_window::WindowState::Normal
    pub fn window_restored {
        event: WINDOW_CHANGED_EVENT,
        args: WindowChangedArgs,
        filter: |args| args.window_id == WINDOW.id() && args.entered_state(WindowState::Normal),
    }

    /// On window state changed to [`WindowState::is_fullscreen`].
    ///
    /// [`WindowState::is_fullscreen`]: zng_ext_window::WindowState::is_fullscreen
    pub fn window_fullscreen {
        event: WINDOW_CHANGED_EVENT,
        args: WindowChangedArgs,
        filter: |args| args.window_id == WINDOW.id() && args.entered_fullscreen(),
    }

    /// On window state changed from [`WindowState::is_fullscreen`].
    ///
    /// [`WindowState::is_fullscreen`]: zng_ext_window::WindowState::is_fullscreen
    pub fn window_exited_fullscreen {
        event: WINDOW_CHANGED_EVENT,
        args: WindowChangedArgs,
        filter: |args| args.window_id == WINDOW.id() && args.exited_fullscreen(),
    }

    /// On Input Method Editor event.
    pub fn ime {
        event: IME_EVENT,
        args: ImeArgs,
        filter: |args| args.target.widget_id() == WIDGET.id(),
    }
}

#[cfg(feature = "image")]
event_property! {
    /// On window frame rendered.
    pub fn frame_image_ready {
        event: FRAME_IMAGE_READY_EVENT,
        args: FrameImageReadyArgs,
        filter: |args| args.window_id == WINDOW.id(),
    }
}
