//! Mouse service, properties, events and other types.
//!
//! The example below defines a window that shows the pressed mouse buttons and prints the button state changes. The
//! pressed buttons text follows the cursor position.
//!
//! ```
//! use zng::prelude::*;
//! # fn example() {
//!
//! # let _ =
//! Window! {
//!     child_align = layout::Align::TOP_LEFT;
//!     child = Text! {
//!         txt = mouse::MOUSE.buttons().map_debug(false);
//!         layout::offset = mouse::MOUSE.position().map(|p| match p {
//!             Some(p) => layout::Vector::from(p.position.to_vector()) - layout::Vector::new(0, 100.pct()),
//!             None => layout::Vector::zero(),
//!         });
//!     };
//!     mouse::on_mouse_input = hn!(|args: &mouse::MouseInputArgs| {
//!         println!("button {:?} {:?}", args.button, args.state);
//!     });
//! }
//! # ; }
//! ```
//!
//! Mouse events are send to the top widget under the cursor. This module also provides mouse exclusive gestures like mouse clicks
//! and mouse hovered, these gestures are composed with others in [`gesture`] to provide the final pointer gestures. You should
//! prefer using [`gesture::on_click`] over [`on_mouse_click`], unless you really want to exclusively handle mouse clicks.
//!
//! [`gesture`]: crate::gesture
//! [`gesture::on_click`]: fn@crate::gesture::on_click
//! [`on_mouse_click`]: fn@on_mouse_click
//!
//! # Full API
//!
//! See [`zng_ext_input::mouse`] and [`zng_wgt_input::mouse`] for the full mouse API.

pub use zng_ext_input::mouse::{
    ButtonRepeatConfig, ButtonState, ClickMode, ClickTrigger, MOUSE, MOUSE_CLICK_EVENT, MOUSE_HOVERED_EVENT, MOUSE_INPUT_EVENT,
    MOUSE_MOVE_EVENT, MOUSE_WHEEL_EVENT, MouseButton, MouseClickArgs, MouseHoverArgs, MouseInputArgs, MouseMoveArgs, MousePosition,
    MouseScrollDelta, MouseWheelArgs, MultiClickConfig, WidgetInfoBuilderMouseExt, WidgetInfoMouseExt,
};

pub use zng_wgt_input::mouse::{
    ctrl_scroll, on_disabled_mouse_any_click, on_disabled_mouse_click, on_disabled_mouse_hovered, on_disabled_mouse_input,
    on_disabled_mouse_wheel, on_mouse_any_click, on_mouse_any_double_click, on_mouse_any_single_click, on_mouse_any_triple_click,
    on_mouse_click, on_mouse_double_click, on_mouse_down, on_mouse_enter, on_mouse_hovered, on_mouse_input, on_mouse_leave, on_mouse_move,
    on_mouse_scroll, on_mouse_single_click, on_mouse_triple_click, on_mouse_up, on_mouse_wheel, on_mouse_zoom,
    on_pre_disabled_mouse_any_click, on_pre_disabled_mouse_click, on_pre_disabled_mouse_hovered, on_pre_disabled_mouse_input,
    on_pre_disabled_mouse_wheel, on_pre_mouse_any_click, on_pre_mouse_any_double_click, on_pre_mouse_any_single_click,
    on_pre_mouse_any_triple_click, on_pre_mouse_click, on_pre_mouse_double_click, on_pre_mouse_down, on_pre_mouse_enter,
    on_pre_mouse_hovered, on_pre_mouse_input, on_pre_mouse_leave, on_pre_mouse_move, on_pre_mouse_scroll, on_pre_mouse_single_click,
    on_pre_mouse_triple_click, on_pre_mouse_up, on_pre_mouse_wheel, on_pre_mouse_zoom,
};

pub use zng_wgt_input::{CursorIcon, CursorSource, click_mode, cursor, is_cap_mouse_pressed, is_mouse_pressed};

#[cfg(feature = "image")]
pub use zng_wgt_input::CursorImg;

/// Raw mouse hardware events, received independent of what window is under the pointer.
///
/// You must enable device events in the app to receive this events.
pub mod raw_device_events {
    pub use zng_app::view_process::raw_device_events::{
        BUTTON_EVENT, ButtonArgs, POINTER_MOTION_EVENT, PointerMotionArgs, SCROLL_MOTION_EVENT, ScrollMotionArgs,
    };
}
