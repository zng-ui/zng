//! Touch events, [`on_touch_move`](fn@on_touch_move), [`on_touch_tap`](fn@on_touch_tap),
//! [`on_touch_start`](fn@on_touch_start) and more.
//!
//! There events are low level and directly tied to touch inputs.
//! Before using them review the [`gesture`](super::gesture) events, in particular the
//! [`on_click`](fn@super::gesture::on_click) event.

use zng_ext_input::touch::{
    TOUCH_INPUT_EVENT, TOUCH_LONG_PRESS_EVENT, TOUCH_MOVE_EVENT, TOUCH_TAP_EVENT, TOUCH_TRANSFORM_EVENT, TOUCHED_EVENT, TouchInputArgs,
    TouchLongPressArgs, TouchMoveArgs, TouchTapArgs, TouchTransformArgs, TouchedArgs,
};
use zng_wgt::prelude::*;

event_property! {
    /// Touch contact moved over the widget and cursor capture allows it.
    pub fn touch_move {
        event: TOUCH_MOVE_EVENT,
        args: TouchMoveArgs,
        filter: |args| args.capture_allows(),
    }

    /// Touch contact started or ended over the widget, it is enabled and cursor capture allows it.
    pub fn touch_input {
        event: TOUCH_INPUT_EVENT,
        args: TouchInputArgs,
        filter: |args| args.target.contains_enabled(WIDGET.id()) && args.capture_allows(),
    }

    /// Touch contact started or ended over the widget, it is disabled and cursor capture allows it.
    pub fn disabled_touch_input {
        event: TOUCH_INPUT_EVENT,
        args: TouchInputArgs,
        filter: |args| args.target.contains_disabled(WIDGET.id()) && args.capture_allows(),
    }

    /// Touch contact started over the widget, it is enabled and cursor capture allows it.
    pub fn touch_start {
        event: TOUCH_INPUT_EVENT,
        args: TouchInputArgs,
        filter: |args| args.is_touch_start() && args.target.contains_enabled(WIDGET.id()) && args.capture_allows(),
    }

    /// Touch contact ended over the widget, it is enabled and cursor capture allows it.
    pub fn touch_end {
        event: TOUCH_INPUT_EVENT,
        args: TouchInputArgs,
        filter: |args| args.is_touch_end() && args.target.contains_enabled(WIDGET.id()) && args.capture_allows(),
    }

    /// Touch contact canceled over the widget, it is enabled and cursor capture allows it.
    pub fn touch_cancel {
        event: TOUCH_INPUT_EVENT,
        args: TouchInputArgs,
        filter: |args| args.is_touch_cancel() && args.target.contains_enabled(WIDGET.id()) && args.capture_allows(),
    }

    /// Touch tap on the widget and it is enabled.
    pub fn touch_tap {
        event: TOUCH_TAP_EVENT,
        args: TouchTapArgs,
        filter: |args| args.target.contains_enabled(WIDGET.id()),
    }

    /// Touch tap on the widget and it is disabled.
    pub fn disabled_touch_tap {
        event: TOUCH_TAP_EVENT,
        args: TouchTapArgs,
        filter: |args| args.target.contains_disabled(WIDGET.id()),
    }

    /// Touch contact is now over the widget or a descendant and it is enabled.
    pub fn touch_enter {
        event: TOUCHED_EVENT,
        args: TouchedArgs,
        filter: |args| args.is_touch_enter_enabled(),
    }

    /// Touch contact is no longer over the widget or any descendant and it is enabled.
    pub fn touch_leave {
        event: TOUCHED_EVENT,
        args: TouchedArgs,
        filter: |args| args.is_touch_leave_enabled(),
    }

    /// Touch contact entered or left the widget and descendants area and it is enabled.
    ///
    /// You can use the [`is_touch_enter`] and [`is_touch_leave`] methods to determinate the state change.
    ///
    /// [`is_touch_enter`]: TouchedArgs::is_touch_enter
    /// [`is_touch_leave`]: TouchedArgs::is_touch_leave
    pub fn touched {
        event: TOUCHED_EVENT,
        args: TouchedArgs,
        filter: |args| args.is_enabled(WIDGET.id()),
    }

    /// Touch gesture to translate, scale or rotate happened over this widget.
    pub fn touch_transform {
        event: TOUCH_TRANSFORM_EVENT,
        args: TouchTransformArgs,
        filter: |args| args.target.contains_enabled(WIDGET.id()),
    }

    /// Single touch contact was made and held in place for a duration of time (default 500ms) on
    /// the widget and the widget is enabled.
    pub fn touch_long_press {
        event: TOUCH_LONG_PRESS_EVENT,
        args: TouchLongPressArgs,
        filter: |args| args.target.contains_enabled(WIDGET.id()),
    }

    /// Single touch contact was made and held in place for a duration of time (default 500ms) on
    /// the widget and the widget is disabled.
    pub fn disabled_touch_long_press {
        event: TOUCH_LONG_PRESS_EVENT,
        args: TouchLongPressArgs,
        filter: |args| args.target.contains_disabled(WIDGET.id()),
    }
}
