//! Mouse service, properties, events and types.
//!
//! # Full API
//!
//! See [`zero_ui_ext_input::mouse`] and [`zero_ui_wgt_input::mouse`] for the full mouse API.

pub use zero_ui_ext_input::mouse::{
    ButtonRepeatConfig, ButtonState, ClickMode, ClickTrigger, MouseButton, MouseClickArgs, MouseHoverArgs, MouseInputArgs, MouseMoveArgs,
    MousePosition, MouseScrollDelta, MouseWheelArgs, MultiClickConfig, WidgetInfoBuilderMouseExt, WidgetInfoMouseExt, MOUSE,
    MOUSE_CLICK_EVENT, MOUSE_HOVERED_EVENT, MOUSE_INPUT_EVENT, MOUSE_MOVE_EVENT, MOUSE_WHEEL_EVENT,
};

pub use zero_ui_wgt_input::mouse::{
    on_disabled_mouse_any_click, on_disabled_mouse_click, on_disabled_mouse_hovered, on_disabled_mouse_input, on_disabled_mouse_wheel,
    on_mouse_any_click, on_mouse_any_double_click, on_mouse_any_single_click, on_mouse_any_triple_click, on_mouse_click,
    on_mouse_double_click, on_mouse_down, on_mouse_enter, on_mouse_hovered, on_mouse_input, on_mouse_leave, on_mouse_move, on_mouse_scroll,
    on_mouse_single_click, on_mouse_triple_click, on_mouse_up, on_mouse_wheel, on_mouse_zoom, on_pre_disabled_mouse_any_click,
    on_pre_disabled_mouse_click, on_pre_disabled_mouse_hovered, on_pre_disabled_mouse_input, on_pre_disabled_mouse_wheel,
    on_pre_mouse_any_click, on_pre_mouse_any_double_click, on_pre_mouse_any_single_click, on_pre_mouse_any_triple_click,
    on_pre_mouse_click, on_pre_mouse_double_click, on_pre_mouse_down, on_pre_mouse_enter, on_pre_mouse_hovered, on_pre_mouse_input,
    on_pre_mouse_leave, on_pre_mouse_move, on_pre_mouse_scroll, on_pre_mouse_single_click, on_pre_mouse_triple_click, on_pre_mouse_up,
    on_pre_mouse_wheel, on_pre_mouse_zoom,
};

pub use zero_ui_wgt_input::{click_mode, cursor, cursor_img, is_cap_mouse_pressed, is_mouse_pressed, CursorIcon, CursorImg};
