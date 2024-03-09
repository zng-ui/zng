//! Window events, [`on_window_open`](fn@on_window_open) and [`on_window_close_requested`](fn@on_window_close_requested).
//!
//! These events are re-exported by [`Window!`](struct@crate::Window) as `on_open` and `on_close_requested`, but you can
//! attach then in any widget inside a window using the property full name.
//!
//! There is no event property for the [`WINDOW_CLOSE_EVENT`] because that event notifies
//! after the window is deinited. You can use [`on_deinit`](fn@zero_ui_wgt::on_deinit) in the
//! [`Window!`](struct@crate::Window) widget to handle a window closing, or create an app level handler for the
//! event using [`EVENTS`](zero_ui_app::event::EVENTS).

use zero_ui_ext_window::*;
use zero_ui_wgt::prelude::*;

event_property! {
    /// On window opened.
    ///
    /// This event notifies once per window, as soon as the window is created and the content is inited.
    pub fn window_open {
        event: WINDOW_OPEN_EVENT,
        args: WindowOpenArgs,
        filter: |args| args.window_id == WINDOW.id(),
    }

    /// On window loaded.
    ///
    /// This event notifies once per window, after the window content is inited, updated, layout and the first frame
    /// was send to the renderer. Windows are considered *loaded* after the first layout and all [`WindowLoadingHandle`]
    /// have expired or dropped.
    ///
    /// [`WindowLoadingHandle`]: zero_ui_app::window::WindowLoadingHandle
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
    /// Calling [`propagation().stop()`] on this event cancels the window close.
    ///
    /// [`propagation().stop()`]: crate::event::EventPropagationHandle::stop
    pub fn window_close_requested {
        event: WINDOW_CLOSE_REQUESTED_EVENT,
        args: WindowCloseRequestedArgs,
        filter: |args| args.windows.contains(&WINDOW.id()),
    }

    /// On window state changed.
    ///
    /// This event notifies every time the user or the app changes the [`WindowState`].
    pub fn window_state_changed {
        event: WINDOW_CHANGED_EVENT,
        args: WindowChangedArgs,
        filter: |args| args.window_id == WINDOW.id() && args.is_state_changed(),
    }

    /// On window state changed to [`WindowState::Maximized`].
    pub fn window_maximized {
        event: WINDOW_CHANGED_EVENT,
        args: WindowChangedArgs,
        filter: |args| args.window_id == WINDOW.id() && args.entered_state(WindowState::Maximized),
    }

    /// On window state changed from [`WindowState::Maximized`].
    pub fn window_unmaximized {
        event: WINDOW_CHANGED_EVENT,
        args: WindowChangedArgs,
        filter: |args| args.window_id == WINDOW.id() && args.exited_state(WindowState::Maximized),
    }

    /// On window state changed to [`WindowState::Minimized`].
    pub fn window_minimized {
        event: WINDOW_CHANGED_EVENT,
        args: WindowChangedArgs,
        filter: |args| args.window_id == WINDOW.id() && args.entered_state(WindowState::Minimized),
    }

    /// On window state changed from [`WindowState::Minimized`].
    pub fn window_unminimized {
        event: WINDOW_CHANGED_EVENT,
        args: WindowChangedArgs,
        filter: |args| args.window_id == WINDOW.id() && args.exited_state(WindowState::Minimized),
    }

    /// On window state changed to [`WindowState::Normal`].
    pub fn window_restored {
        event: WINDOW_CHANGED_EVENT,
        args: WindowChangedArgs,
        filter: |args| args.window_id == WINDOW.id() && args.entered_state(WindowState::Normal),
    }

    /// On window state changed to [`WindowState::Fullscreen`] or [`WindowState::Exclusive`] from a previous not
    /// fullscreen state.
    pub fn window_fullscreen {
        event: WINDOW_CHANGED_EVENT,
        args: WindowChangedArgs,
        filter: |args| args.window_id == WINDOW.id() && args.entered_fullscreen(),
    }

    /// On window state changed from [`WindowState::Fullscreen`] or [`WindowState::Exclusive`] from a new not
    /// fullscreen state.
    pub fn window_exited_fullscreen {
        event: WINDOW_CHANGED_EVENT,
        args: WindowChangedArgs,
        filter: |args| args.window_id == WINDOW.id() && args.exited_fullscreen(),
    }

    /// On window frame rendered.
    pub fn frame_image_ready {
        event: FRAME_IMAGE_READY_EVENT,
        args: FrameImageReadyArgs,
        filter: |args| args.window_id == WINDOW.id(),
    }

    /// On Input Method Editor event.
    pub fn ime {
        event: IME_EVENT,
        args: ImeArgs,
        filter: |args| args.target.widget_id() == WIDGET.id(),
    }
}
