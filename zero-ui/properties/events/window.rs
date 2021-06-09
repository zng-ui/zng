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
use crate::core::window::{WindowCloseRequestedArgs, WindowCloseRequestedEvent, WindowEventArgs, WindowOpenEvent};

event_property! {
    /// On window opened.
    ///
    /// This event notifies once per window, after the window content is inited and the first frame is rendered.
    pub fn window_open {
        event: WindowOpenEvent,
        args: WindowEventArgs,
    }

    /// On window close requested.
    ///
    /// This event notifies every time the user or the app tries to close the window, you can call
    /// [`cancel`](WindowCloseRequestedArgs::cancel) to stop the window from being closed.
    pub fn window_close_requested {
        event: WindowCloseRequestedEvent,
        args: WindowCloseRequestedArgs
    }
}
