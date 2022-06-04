//! Mouse events, [`on_mouse_move`](fn@on_mouse_move), [`on_mouse_enter`](fn@on_mouse_enter),
//! [`on_mouse_down`](fn@on_mouse_down) and more.
//!
//! There events are low level and directly tied to a mouse device.
//! Before using them review the [`gesture`](super::gesture) events, in particular the
//! [`on_click`](fn@super::gesture::on_click) event.

use super::event_property;
use crate::core::mouse::*;

event_property! {
    /// Mouse cursor moved over the widget.
    pub fn mouse_move {
        event: MouseMoveEvent,
        args: MouseMoveArgs,
    }

    /// Mouse button pressed or released while the cursor is over the widget and the widget is enabled.
    pub fn mouse_input {
        event: MouseInputEvent,
        args: MouseInputArgs,
        filter: |ctx, args| args.is_enabled(ctx.path),
    }

    /// Mouse button pressed or release while the cursor is over the widget and the widget is disabled.
    pub fn disabled_mouse_input {
        event: MouseInputEvent,
        args: MouseInputArgs,
        filter: |ctx, args| args.is_disabled(ctx.path),
    }

    /// Mouse button pressed while the cursor is over the widget and the widget is enabled.
    pub fn mouse_down {
        event: MouseInputEvent,
        args: MouseInputArgs,
        filter: |ctx, args| args.is_mouse_down() && args.is_enabled(ctx.path),
    }

    /// Mouse button released while the cursor if over the widget and the widget is enabled.
    pub fn mouse_up {
        event: MouseInputEvent,
        args: MouseInputArgs,
        filter: |ctx, args| args.is_mouse_up() && args.is_enabled(ctx.path),
    }

    /// Mouse clicked on the widget with any button and including double+ clicks and the widget is enabled.
    pub fn mouse_any_click {
        event: MouseClickEvent,
        args: MouseClickArgs,
        filter: |ctx, args| args.is_enabled(ctx.path),
    }

    /// Mouse clicked on the widget with any button and including double+ clicks and the widget is disabled.
    pub fn disabled_mouse_any_click {
        event: MouseClickEvent,
        args: MouseClickArgs,
        filter: |ctx, args| args.is_disabled(ctx.path),
    }

    /// Mouse clicked on the widget with any button but excluding double+ clicks and the widget is enabled.
    pub fn mouse_any_single_click {
        event: MouseClickEvent,
        args: MouseClickArgs,
        filter: |ctx, args| args.is_single() && args.is_enabled(ctx.path),
    }

    /// Mouse double clicked on the widget with any button and the widget is enabled.
    pub fn mouse_any_double_click {
        event: MouseClickEvent,
        args: MouseClickArgs,
        filter: |ctx, args| args.is_double() && args.is_enabled(ctx.path),
    }

    /// Mouse triple clicked on the widget with any button and the widget is enabled.
    pub fn mouse_any_triple_click {
        event: MouseClickEvent,
        args: MouseClickArgs,
        filter: |ctx, args| args.is_triple() && args.is_enabled(ctx.path),
    }

    /// Mouse clicked on the widget with the primary button including double+ clicks and the widget is enabled.
    pub fn mouse_click {
        event: MouseClickEvent,
        args: MouseClickArgs,
        filter: |ctx, args| args.is_primary() && args.is_enabled(ctx.path),
    }

    /// Mouse clicked on the widget with the primary button including double+ clicks and the widget is disabled.
    pub fn disabled_mouse_click {
        event: MouseClickEvent,
        args: MouseClickArgs,
        filter: |ctx, args| args.is_primary() && args.is_disabled(ctx.path),
    }

    /// Mouse clicked on the widget with the primary button excluding double+ clicks and the widget is enabled.
    pub fn mouse_single_click {
        event: MouseClickEvent,
        args: MouseClickArgs,
        filter: |ctx, args| args.is_primary() && args.is_single() && args.is_enabled(ctx.path),
    }

    /// Mouse double clicked on the widget with the primary button and the widget is enabled.
    pub fn mouse_double_click {
        event: MouseClickEvent,
        args: MouseClickArgs,
        filter: |ctx, args| args.is_primary() && args.is_double() && args.is_enabled(ctx.path),
    }

    /// Mouse triple clicked on the widget with the primary button and the widget is enabled.
    pub fn mouse_triple_click {
        event: MouseClickEvent,
        args: MouseClickArgs,
        filter: |ctx, args| args.is_primary() && args.is_triple() && args.is_enabled(ctx.path),
    }

    /// Mouse is now over the widget or a descendant widget and the widget is enabled.
    pub fn mouse_enter {
        event: MouseHoveredEvent,
        args: MouseHoverArgs,
        filter: |ctx, args| args.is_mouse_enter_enabled(ctx.path),
    }

    /// Mouse is no longer over the widget or any descendant widget and the widget is enabled.
    pub fn mouse_leave {
        event: MouseHoveredEvent,
        args: MouseHoverArgs,
        filter: |ctx, args| args.is_mouse_leave_enabled(ctx.path),
    }

    /// Mouse entered or left the widget and descendant widgets area and the widget is enabled.
    ///
    /// You can use the [`is_mouse_enter`] and [`is_mouse_leave`] methods to determinate the state change.
    ///
    /// [`is_mouse_enter`]: MouseHoverArgs::is_mouse_enter
    /// [`is_mouse_leave`]: MouseHoverArgs::is_mouse_leave
    pub fn mouse_hovered {
        event: MouseHoveredEvent,
        args: MouseHoverArgs,
        filter: |ctx, args| args.is_enabled(ctx.path),
    }

    /// Mouse entered or left the widget and descendant widgets area and the widget is disabled.
    pub fn disabled_mouse_hovered {
        event: MouseHoveredEvent,
        args: MouseHoverArgs,
        filter: |ctx, args| args.is_disabled(ctx.path),
    }

    /// Widget acquired mouse capture.
    pub fn got_mouse_capture {
        event: MouseCaptureEvent,
        args: MouseCaptureArgs,
        filter: |ctx, args| args.is_got(ctx.path.widget_id()),
    }

    /// Widget lost mouse capture.
    pub fn lost_mouse_capture {
        event: MouseCaptureEvent,
        args: MouseCaptureArgs,
        filter: |ctx, args| args.is_lost(ctx.path.widget_id()),
    }

    /// Widget acquired or lost mouse capture.
    pub fn mouse_capture_changed {
        event: MouseCaptureEvent,
        args: MouseCaptureArgs,
    }

    /// Mouse wheel scrolled while pointer is hovering widget and the widget is enabled.
    pub fn mouse_wheel {
        event: MouseWheelEvent,
        args: MouseWheelArgs,
        filter: |ctx, args| args.is_enabled(ctx.path),
    }

    /// Mouse wheel scrolled while pointer is hovering widget and the widget is disabled.
    pub fn disabled_mouse_wheel {
        event: MouseWheelEvent,
        args: MouseWheelArgs,
        filter: |ctx, args| args.is_enabled(ctx.path),
    }

    /// Mouse wheel scrolled while pointer is hovering the widget and the pressed keyboard modifiers allow a scroll operation and
    /// the widget is enabled.
    pub fn mouse_scroll {
        event: MouseWheelEvent,
        args: MouseWheelArgs,
        filter: |ctx, args| args.is_scroll() && args.is_enabled(ctx.path),
    }

    /// Mouse wheel scrolled while pointer is hovering the widget and the pressed keyboard modifiers allow a zoom operation and
    /// the widget is enabled.
    pub fn mouse_zoom {
        event: MouseWheelEvent,
        args: MouseWheelArgs,
        filter: |ctx, args| args.is_zoom() && args.is_enabled(ctx.path),
    }
}
