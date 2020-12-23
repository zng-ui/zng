//! Mouse events, [`on_mouse_move`], [`on_mouse_enter`] and more.
//!
//! There events are low level and directly tied to a mouse device.
//! Before using then review the [`gesture`](super::gesture) events, in particular the [`on_click`](super::gesture::on_click) event.

use super::event_property;
use crate::core::context::WidgetContext;
use crate::core::event::EventArgs;
use crate::core::mouse::*;

event_property! {
    pub fn mouse_move {
        event: MouseMoveEvent,
        args: MouseMoveArgs,
    }

    pub fn mouse_input {
        event: MouseInputEvent,
        args: MouseInputArgs,
    }

    pub fn mouse_down {
        event: MouseDownEvent,
        args: MouseInputArgs,
    }

    pub fn mouse_up {
        event: MouseUpEvent,
        args: MouseInputArgs,
    }

    pub fn mouse_any_click {
        event: MouseClickEvent,
        args: MouseClickArgs,
    }

    pub fn mouse_any_single_click {
        event: MouseSingleClickEvent,
        args: MouseClickArgs,
    }

    pub fn mouse_any_double_click {
        event: MouseDoubleClickEvent,
        args: MouseClickArgs,
    }

    pub fn mouse_any_triple_click {
        event: MouseTripleClickEvent,
        args: MouseClickArgs,
    }

    pub fn mouse_click {
        event: MouseClickEvent,
        args: MouseClickArgs,
        filter: mouse_primary_filter,
    }

    pub fn mouse_single_click {
        event: MouseSingleClickEvent,
        args: MouseClickArgs,
        filter: mouse_primary_filter,
    }

    pub fn mouse_double_click {
        event: MouseDoubleClickEvent,
        args: MouseClickArgs,
        filter: mouse_primary_filter,
    }

    pub fn mouse_triple_click {
        event: MouseTripleClickEvent,
        args: MouseClickArgs,
        filter: mouse_primary_filter,
    }

    pub fn mouse_enter {
        event: MouseEnterEvent,
        args: MouseHoverArgs,
    }

    pub fn mouse_leave {
        event: MouseLeaveEvent,
        args: MouseHoverArgs,
    }

    pub fn got_mouse_capture {
        event: MouseCaptureEvent,
        args: MouseCaptureArgs,
        filter: |ctx, args| args.is_got(ctx.path.widget_id()),
    }

    pub fn lost_mouse_capture {
        event: MouseCaptureEvent,
        args: MouseCaptureArgs,
        filter: |ctx, args| args.is_lost(ctx.path.widget_id()),
    }

    pub fn mouse_capture_changed {
        event: MouseCaptureEvent,
        args: MouseCaptureArgs,
    }
}
// filter used in mouse_click, mouse_single_click, mouse_double_click and mouse_triple_click.
fn mouse_primary_filter(ctx: &mut WidgetContext, args: &MouseClickArgs) -> bool {
    args.concerns_widget(ctx) && args.is_primary()
}
