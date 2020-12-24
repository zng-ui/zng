//! Keyboard events, [`on_key_down`], [`on_key_up`], [`on_char_input`] and more.
//!
//! These events are low level and directly tied to a keyboard device.
//! Before using them review the [`gesture`](super::gesture) events, in particular
//! the [`on_shortcut`](super::gesture::on_shortcut) event.

use super::event_property;
use crate::core::keyboard::*;

event_property! {
    /// Event fired when a keyboard key is pressed or released.
    ///
    /// # Route
    ///
    /// The event is raised in the [keyboard focused](crate::properties::is_focused)
    /// widget and then each parent up to the root. If [`stop_propagation`](EventArgs::stop_propagation)
    /// is requested the event is not notified further. If the widget is [disabled](IsEnabled) the event is not notified.
    ///
    /// This route is also called *bubbling*.
    ///
    /// # Keys
    ///
    /// Any key press/release generates a key input event, including keys that don't map
    /// to any virtual key, see [`KeyInputArgs`] for more details. To take text input use [`on_char_input`] instead.
    /// For key combinations consider using [`on_shortcut`].
    ///
    /// # Underlying Event
    ///
    /// This event property uses the [`KeyInputEvent`] that is included in the default app.
    pub fn key_input {
        event: KeyInputEvent,
        args: KeyInputArgs,
    }

    /// Event fired when a keyboard key is pressed.
    ///
    /// # Route
    ///
    /// The event is raised in the [keyboard focused](crate::properties::is_focused)
    /// widget and then each parent up to the root. If [`stop_propagation`](EventArgs::stop_propagation)
    /// is requested the event is not notified further. If the widget is [disabled](IsEnabled) the event is not notified.
    ///
    /// This route is also called *bubbling*.
    ///
    /// # Keys
    ///
    /// Any key press generates a key down event, including keys that don't map to any virtual key, see [`KeyInputArgs`]
    /// for more details. To take text input use [`on_char_input`] instead.
    /// For key combinations consider using [`on_shortcut`].
    ///
    /// # Underlying Event
    ///
    /// This event property uses the [`KeyDownEvent`] that is included in the default app.
    pub fn key_down {
        event: KeyDownEvent,
        args: KeyInputArgs,
    }

    /// Event fired when a keyboard key is released.
    ///
    /// # Route
    ///
    /// The event is raised in the [keyboard focused](crate::properties::is_focused)
    /// widget and then each parent up to the root. If [`stop_propagation`](EventArgs::stop_propagation)
    /// is requested the event is not notified further. If the widget is [disabled](IsEnabled) the event is not notified.
    ///
    /// This route is also called *bubbling*.
    ///
    /// # Keys
    ///
    /// Any key release generates a key up event, including keys that don't map to any virtual key, see [`KeyInputArgs`]
    /// for more details. To take text input use [`on_char_input`] instead.
    /// For key combinations consider using [`on_shortcut`].
    ///
    /// # Underlying Event
    ///
    /// This event property uses the [`KeyUpEvent`] that is included in the default app.
    pub fn key_up {
        event: KeyUpEvent,
        args: KeyInputArgs,
    }

    pub fn char_input {
        event: CharInputEvent,
        args: CharInputArgs,
    }
}
