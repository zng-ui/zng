//! Keyboard events, [`on_key_down`](fn@on_key_down), [`on_key_up`](fn@on_key_up), [`on_char_input`](fn@on_char_input) and more.
//!
//! These events are low level and directly tied to a keyboard device.
//! Before using them review the [`gesture`](super::gesture) properties, in particular
//! the [`click_shortcut`](fn@super::gesture::click_shortcut) property.

use super::event_property;
use crate::core::keyboard::*;

event_property! {
    /// Event fired when a keyboard key is pressed or released and the widget is enabled.
    ///
    /// # Route
    ///
    /// The event is raised in the [keyboard focused](crate::properties::is_focused)
    /// widget and then each parent up to the root. If [`propagation`](EventArgs::propagation) stop
    /// is requested the event is not notified further. If the widget is disabled or blocked the event is not notified.
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
        filter: |ctx, args| args.is_enabled(ctx.path),
    }

    /// Event fired when a keyboard key is pressed or released and the widget is disabled.
    ///
    /// # Route
    ///
    /// The event is raised in the [keyboard focused](crate::properties::is_focused)
    /// widget and then each parent up to the root. If [`propagation`](EventArgs::propagation) stop
    /// is requested the event is not notified further. If the widget is enabled or blocked the event is not notified.
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
    pub fn disabled_key_input {
        event: KeyInputEvent,
        args: KeyInputArgs,
        filter: |ctx, args| args.is_disabled(ctx.path),
    }

    /// Event fired when a keyboard key is pressed and the widget is enabled.
    ///
    /// # Route
    ///
    /// The event is raised in the [keyboard focused](crate::properties::is_focused)
    /// widget and then each parent up to the root. If [`propagation`](EventArgs::propagation) stop
    /// is requested the event is not notified further. If the widget is disabled or blocked the event is not notified.
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
    /// This event property uses the [`KeyInputEvent`] that is included in the default app.
    pub fn key_down {
        event: KeyInputEvent,
        args: KeyInputArgs,
        filter: |ctx, args| args.state == KeyState::Pressed && args.is_enabled(ctx.path),
    }

    /// Event fired when a keyboard key is released and the widget is enabled.
    ///
    /// # Route
    ///
    /// The event is raised in the [keyboard focused](crate::properties::is_focused)
    /// widget and then each parent up to the root. If [`propagation`](EventArgs::propagation) stop
    /// is requested the event is not notified further. If the widget is disabled or blocked the event is not notified.
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
    /// This event property uses the [`KeyInputEvent`] that is included in the default app.
    pub fn key_up {
        event: KeyInputEvent,
        args: KeyInputArgs,
        filter: |ctx, args| args.state == KeyState::Released && args.is_enabled(ctx.path),
    }

    /// Event fired when a text character is typed and the widget is enabled.
    ///
    /// # Route
    ///
    /// The event is raised in the [keyboard focused](crate::properties::is_focused)
    /// widget and then each parent up to the root. If [`propagation`](EventArgs::propagation) stop
    /// is requested the event is not notified further. If the widget is disabled or blocked the event is not notified.
    ///
    /// This route is also called *bubbling*.
    ///
    /// # Underlying Event
    ///
    /// This event property uses the [`CharInputEvent`] that is included in the default app.
    pub fn char_input {
        event: CharInputEvent,
        args: CharInputArgs,
        filter: |ctx, args| args.is_enabled(ctx.path)
    }

    /// Event fired when a text character is typed and the widget is disabled.
    ///
    /// # Route
    ///
    /// The event is raised in the [keyboard focused](crate::properties::is_focused)
    /// widget and then each parent up to the root. If [`propagation`](EventArgs::propagation) stop
    /// is requested the event is not notified further. If the widget is enabled or blocked the event is not notified.
    ///
    /// This route is also called *bubbling*.
    ///
    /// # Underlying Event
    ///
    /// This event property uses the [`CharInputEvent`] that is included in the default app.
    pub fn disabled_char_input {
        event: CharInputEvent,
        args: CharInputArgs,
        filter: |ctx, args| args.is_disabled(ctx.path)
    }
}
