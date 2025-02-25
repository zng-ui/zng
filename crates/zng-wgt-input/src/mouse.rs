//! Mouse events, [`on_mouse_move`](fn@on_mouse_move), [`on_mouse_enter`](fn@on_mouse_enter),
//! [`on_mouse_down`](fn@on_mouse_down) and more.
//!
//! There events are low level and directly tied to a mouse device.
//! Before using them review the [`gesture`](super::gesture) events, in particular the
//! [`on_click`](fn@super::gesture::on_click) event.

use zng_ext_input::mouse::{
    MOUSE_CLICK_EVENT, MOUSE_HOVERED_EVENT, MOUSE_INPUT_EVENT, MOUSE_MOVE_EVENT, MOUSE_WHEEL_EVENT, MouseClickArgs, MouseHoverArgs,
    MouseInputArgs, MouseMoveArgs, MouseWheelArgs,
};
use zng_wgt::prelude::*;

event_property! {
    /// Mouse cursor moved over the widget and cursor capture allows it.
    pub fn mouse_move {
        event: MOUSE_MOVE_EVENT,
        args: MouseMoveArgs,
        filter: |args| args.capture_allows(),
    }

    /// Mouse button pressed or released while the cursor is over the widget, the widget is enabled and no cursor
    /// capture blocks it.
    pub fn mouse_input {
        event: MOUSE_INPUT_EVENT,
        args: MouseInputArgs,
        filter: |args| args.is_enabled(WIDGET.id()) && args.capture_allows(),
    }

    /// Mouse button pressed or release while the cursor is over the widget, the widget is disabled and no cursor
    /// capture blocks it.
    pub fn disabled_mouse_input {
        event: MOUSE_INPUT_EVENT,
        args: MouseInputArgs,
        filter: |args| args.is_disabled(WIDGET.id()) && args.capture_allows(),
    }

    /// Mouse button pressed while the cursor is over the widget, the widget is enabled and cursor capture allows it.
    pub fn mouse_down {
        event: MOUSE_INPUT_EVENT,
        args: MouseInputArgs,
        filter: |args| args.is_mouse_down() && args.is_enabled(WIDGET.id()) && args.capture_allows(),
    }

    /// Mouse button released while the cursor if over the widget, the widget is enabled and cursor capture allows it.
    pub fn mouse_up {
        event: MOUSE_INPUT_EVENT,
        args: MouseInputArgs,
        filter: |args| args.is_mouse_up() && args.is_enabled(WIDGET.id()) && args.capture_allows(),
    }

    /// Mouse clicked on the widget with any button and including repeat clicks and it is enabled.
    pub fn mouse_any_click {
        event: MOUSE_CLICK_EVENT,
        args: MouseClickArgs,
        filter: |args| args.is_enabled(WIDGET.id()),
    }

    /// Mouse clicked on the disabled widget with any button, including repeat clicks.
    pub fn disabled_mouse_any_click {
        event: MOUSE_CLICK_EVENT,
        args: MouseClickArgs,
        filter: |args| args.is_disabled(WIDGET.id()),
    }

    /// Mouse clicked on the widget with any button but excluding repeat clicks and it is enabled.
    pub fn mouse_any_single_click {
        event: MOUSE_CLICK_EVENT,
        args: MouseClickArgs,
        filter: |args| args.is_single() && args.is_enabled(WIDGET.id()),
    }

    /// Mouse double clicked on the widget with any button and it is enabled.
    pub fn mouse_any_double_click {
        event: MOUSE_CLICK_EVENT,
        args: MouseClickArgs,
        filter: |args| args.is_double() && args.is_enabled(WIDGET.id()),
    }

    /// Mouse triple clicked on the widget with any button and it is enabled.
    pub fn mouse_any_triple_click {
        event: MOUSE_CLICK_EVENT,
        args: MouseClickArgs,
        filter: |args| args.is_triple() && args.is_enabled(WIDGET.id()),
    }

    /// Mouse clicked on the widget with the primary button including repeat clicks and it is enabled.
    pub fn mouse_click {
        event: MOUSE_CLICK_EVENT,
        args: MouseClickArgs,
        filter: |args| args.is_primary() && args.is_enabled(WIDGET.id()),
    }

    /// Mouse clicked on the disabled widget with the primary button, including repeat clicks.
    pub fn disabled_mouse_click {
        event: MOUSE_CLICK_EVENT,
        args: MouseClickArgs,
        filter: |args| args.is_primary() && args.is_disabled(WIDGET.id()),
    }

    /// Mouse clicked on the widget with the primary button excluding repeat clicks and it is enabled.
    pub fn mouse_single_click {
        event: MOUSE_CLICK_EVENT,
        args: MouseClickArgs,
        filter: |args| args.is_primary() && args.is_single() && args.is_enabled(WIDGET.id()),
    }

    /// Mouse double clicked on the widget with the primary button and it is enabled.
    pub fn mouse_double_click {
        event: MOUSE_CLICK_EVENT,
        args: MouseClickArgs,
        filter: |args| args.is_primary() && args.is_double() && args.is_enabled(WIDGET.id()),
    }

    /// Mouse triple clicked on the widget with the primary button and it is enabled.
    pub fn mouse_triple_click {
        event: MOUSE_CLICK_EVENT,
        args: MouseClickArgs,
        filter: |args| args.is_primary() && args.is_triple() && args.is_enabled(WIDGET.id()),
    }

    /// Mouse is now over the widget or a descendant widget, the widget is enabled and cursor capture allows it.
    pub fn mouse_enter {
        event: MOUSE_HOVERED_EVENT,
        args: MouseHoverArgs,
        filter: |args| args.is_mouse_enter_enabled(),
    }

    /// Mouse is no longer over the widget or any descendant widget, the widget is enabled and cursor capture allows it.
    pub fn mouse_leave {
        event: MOUSE_HOVERED_EVENT,
        args: MouseHoverArgs,
        filter: |args| args.is_mouse_leave_enabled(),
    }

    /// Mouse entered or left the widget and descendant widgets area, the widget is enabled and cursor capture allows it.
    ///
    /// You can use the [`is_mouse_enter`] and [`is_mouse_leave`] methods to determinate the state change.
    ///
    /// [`is_mouse_enter`]: MouseHoverArgs::is_mouse_enter
    /// [`is_mouse_leave`]: MouseHoverArgs::is_mouse_leave
    pub fn mouse_hovered {
        event: MOUSE_HOVERED_EVENT,
        args: MouseHoverArgs,
        filter: |args| args.is_enabled(WIDGET.id()) && args.capture_allows(),
    }

    /// Mouse entered or left the widget and descendant widgets area, the widget is disabled and cursor capture allows it.
    pub fn disabled_mouse_hovered {
        event: MOUSE_HOVERED_EVENT,
        args: MouseHoverArgs,
        filter: |args| args.is_disabled(WIDGET.id()) && args.capture_allows(),
    }

    /// Mouse wheel scrolled while pointer is hovering widget and it is enabled.
    pub fn mouse_wheel {
        event: MOUSE_WHEEL_EVENT,
        args: MouseWheelArgs,
        filter: |args| args.is_enabled(WIDGET.id()),
    }

    /// Mouse wheel scrolled while pointer is hovering widget and it is disabled.
    pub fn disabled_mouse_wheel {
        event: MOUSE_WHEEL_EVENT,
        args: MouseWheelArgs,
        filter: |args| args.is_enabled(WIDGET.id()),
    }

    /// Mouse wheel scrolled while pointer is hovering the widget and the pressed keyboard modifiers allow a scroll operation and
    /// the widget is enabled.
    pub fn mouse_scroll {
        event: MOUSE_WHEEL_EVENT,
        args: MouseWheelArgs,
        filter: |args| args.is_scroll() && args.is_enabled(WIDGET.id()),
    }

    /// Mouse wheel scrolled while pointer is hovering the widget and the pressed keyboard modifiers allow a zoom operation and
    /// the widget is enabled.
    pub fn mouse_zoom {
        event: MOUSE_WHEEL_EVENT,
        args: MouseWheelArgs,
        filter: |args| args.is_zoom() && args.is_enabled(WIDGET.id()),
    }
}
