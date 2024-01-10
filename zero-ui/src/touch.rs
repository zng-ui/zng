//! Touch service, properties, events and types.
//!
//! # Full API
//!
//! See [`zero_ui_ext_input::touch`] and [`zero_ui_wgt_input::touch`] for the full touch API.

pub use zero_ui_ext_input::touch::{
    TouchConfig, TouchForce, TouchId, TouchInputArgs, TouchLongPressArgs, TouchMove, TouchMoveArgs, TouchPhase, TouchPosition,
    TouchTapArgs, TouchTransformArgs, TouchTransformInfo, TouchTransformMode, TouchUpdate, TouchedArgs, TOUCH, TOUCHED_EVENT,
    TOUCH_INPUT_EVENT, TOUCH_LONG_PRESS_EVENT, TOUCH_MOVE_EVENT, TOUCH_TAP_EVENT, TOUCH_TRANSFORM_EVENT,
};

pub use zero_ui_wgt_input::touch::{
    on_disabled_touch_input, on_disabled_touch_long_press, on_disabled_touch_tap, on_pre_disabled_touch_input,
    on_pre_disabled_touch_long_press, on_pre_disabled_touch_tap, on_pre_touch_cancel, on_pre_touch_end, on_pre_touch_enter,
    on_pre_touch_input, on_pre_touch_leave, on_pre_touch_long_press, on_pre_touch_move, on_pre_touch_start, on_pre_touch_tap,
    on_pre_touch_transform, on_pre_touched, on_touch_cancel, on_touch_end, on_touch_enter, on_touch_input, on_touch_leave,
    on_touch_long_press, on_touch_move, on_touch_start, on_touch_tap, on_touch_transform, on_touched,
};

pub use zero_ui_wgt_input::{is_cap_touched, is_touched, is_touched_from_start, touch_transform};
