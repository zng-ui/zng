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

    /// Mouse button pressed or released while the cursor is over the widget.
    pub fn mouse_input {
        event: MouseInputEvent,
        args: MouseInputArgs,
    }

    /// Mouse button pressed while the cursor is over the widget.
    pub fn mouse_down {
        event: MouseInputEvent,
        args: MouseInputArgs,
        filter: |ctx, args|  args.concerns_widget(ctx) && args.is_mouse_down(),
    }

    /// Mouse button released while the cursor if over the widget.
    pub fn mouse_up {
        event: MouseInputEvent,
        args: MouseInputArgs,
        filter: |ctx, args|  args.concerns_widget(ctx) && args.is_mouse_up(),
    }

    /// Mouse clicked on the widget with any button and including double+ clicks.
    pub fn mouse_any_click {
        event: MouseClickEvent,
        args: MouseClickArgs,
    }

    /// Mouse clicked on the widget with any button but excluding double+ clicks.
    pub fn mouse_any_single_click {
        event: MouseClickEvent,
        args: MouseClickArgs,
        filter: |ctx, args|  args.concerns_widget(ctx) && args.is_single(),
    }

    /// Mouse double clicked on the widget with any button.
    pub fn mouse_any_double_click {
        event: MouseClickEvent,
        args: MouseClickArgs,
        filter: |ctx, args|  args.concerns_widget(ctx) && args.is_double(),
    }

    /// Mouse triple clicked on the widget with any button.
    pub fn mouse_any_triple_click {
        event: MouseClickEvent,
        args: MouseClickArgs,
        filter: |ctx, args|  args.concerns_widget(ctx) && args.is_triple(),
    }

    /// Mouse clicked on the widget with the primary button including double+ clicks.
    pub fn mouse_click {
        event: MouseClickEvent,
        args: MouseClickArgs,
        filter: |ctx, args|args.concerns_widget(ctx) && args.is_primary(),
    }

    /// Mouse clicked on the widget with the primary button excluding double+ clicks.
    pub fn mouse_single_click {
        event: MouseClickEvent,
        args: MouseClickArgs,
        filter: |ctx, args|args.concerns_widget(ctx) && args.is_primary() && args.is_single(),
    }

    /// Mouse double clicked on the widget with the primary button.
    pub fn mouse_double_click {
        event: MouseClickEvent,
        args: MouseClickArgs,
        filter: |ctx, args|args.concerns_widget(ctx) && args.is_primary() && args.is_double(),
    }

    /// Mouse triple clicked on the widget with the primary button.
    pub fn mouse_triple_click {
        event: MouseClickEvent,
        args: MouseClickArgs,
        filter: |ctx, args|args.concerns_widget(ctx) && args.is_primary() && args.is_triple(),
    }

    /// Mouse is now over the widget or a descendant widget.
    pub fn mouse_enter {
        event: MouseHoveredEvent,
        args: MouseHoverArgs,
        filter: |ctx, args|args.is_mouse_enter(ctx.path),
    }

    /// Mouse is no longer over the widget or any descendant widget.
    pub fn mouse_leave {
        event: MouseHoveredEvent,
        args: MouseHoverArgs,
        filter: |ctx, args|args.is_mouse_leave(ctx.path),
    }

    /// Mouse entered or left the widget and descendant widgets area.
    ///
    /// You can use the [`is_mouse_enter`] and [`is_mouse_leave`] methods to determinate the state change.
    ///
    /// [`is_mouse_enter`]: MouseHoverArgs::is_mouse_enter
    /// [`is_mouse_leave`]: MouseHoverArgs::is_mouse_leave
    pub fn mouse_hovered {
        event: MouseHoveredEvent,
        args: MouseHoverArgs,
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

    /// Mouse wheel scrolled while pointer hovering widget.
    pub fn mouse_wheel {
        event: MouseWheelEvent,
        args: MouseWheelArgs,
    }

    /// Mouse wheel scrolled while pointer is hovering the widget and the pressed keyboard modifiers allow a scroll operation.
    pub fn mouse_scroll {
        event: MouseWheelEvent,
        args: MouseWheelArgs,
        filter: |ctx, args| args.is_scroll() && args.concerns_widget(ctx)
    }

    /// Mouse wheel scrolled while pointer is hovering the widget and the pressed keyboard modifiers allow a zoom operation.
    pub fn mouse_zoom {
        event: MouseWheelEvent,
        args: MouseWheelArgs,
        filter: |ctx, args| args.is_zoom() && args.concerns_widget(ctx)
    }
}
