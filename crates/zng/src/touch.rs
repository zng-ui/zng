//! Touch service, properties, events and other types.
//!
//! The example below defines a window that shows the active touches and prints the touch state changes. The
//! touch's text follows the first touch position.
//!
//! ```
//! use zng::prelude::*;
//! # fn example() {
//!
//! # let _ =
//! Window! {
//!     child_align = layout::Align::TOP_LEFT;
//!     child = Text! {
//!         txt = touch::TOUCH.positions().map(|p| {
//!             let mut t = Txt::from("[\n");
//!             for p in p {
//!                 use std::fmt::Write as _;
//!                 writeln!(&mut t, "   ({:?}, {:?})", p.touch, p.position).unwrap();
//!             }
//!             t.push(']');
//!             t.end_mut();
//!             t
//!         });
//!         font_size = 1.4.em();
//!         layout::offset = touch::TOUCH.positions().map(|p| match p.first() {
//!             Some(p) => layout::Vector::from(p.position.to_vector()) - layout::Vector::new(0, 100.pct()),
//!             None => layout::Vector::zero(),
//!         });
//!     };
//!     touch::on_touch_input = hn!(|args: &touch::TouchInputArgs| {
//!         println!("touch {:?} {:?}", args.touch, args.phase);
//!     });
//! }
//! # ; }
//! ```
//!
//! Touch events are send to the top widget under the touch point. This module also provides touch exclusive gestures like
//! tap, touch enter/leave and [`on_touch_transform`]. Note some touch gestures are composed with others in [`gesture`] to provide the
//! final pointer gestures. You should prefer using [`gesture::on_click`] over [`on_touch_tap`], unless you really want to exclusively
//! touch clicks.
//!
//! [`on_touch_tap`]: fn@on_touch_tap
//! [`on_touch_transform`]: fn@on_touch_transform
//! [`gesture`]: crate::gesture
//! [`gesture::on_click`]: fn@crate::gesture::on_click
//!
//! # Full API
//!
//! See [`zng_ext_input::touch`] and [`zng_wgt_input::touch`] for the full touch API.

pub use zng_ext_input::touch::{
    TOUCH, TOUCH_INPUT_EVENT, TOUCH_LONG_PRESS_EVENT, TOUCH_MOVE_EVENT, TOUCH_TAP_EVENT, TOUCH_TRANSFORM_EVENT, TOUCHED_EVENT, TouchConfig,
    TouchForce, TouchId, TouchInputArgs, TouchLongPressArgs, TouchMove, TouchMoveArgs, TouchPhase, TouchPosition, TouchTapArgs,
    TouchTransformArgs, TouchTransformInfo, TouchTransformMode, TouchUpdate, TouchedArgs,
};

pub use zng_wgt_input::touch::{
    on_disabled_touch_input, on_disabled_touch_long_press, on_disabled_touch_tap, on_pre_disabled_touch_input,
    on_pre_disabled_touch_long_press, on_pre_disabled_touch_tap, on_pre_touch_cancel, on_pre_touch_end, on_pre_touch_enter,
    on_pre_touch_input, on_pre_touch_leave, on_pre_touch_long_press, on_pre_touch_move, on_pre_touch_start, on_pre_touch_tap,
    on_pre_touch_transform, on_pre_touched, on_touch_cancel, on_touch_end, on_touch_enter, on_touch_input, on_touch_leave,
    on_touch_long_press, on_touch_move, on_touch_start, on_touch_tap, on_touch_transform, on_touched,
};

pub use zng_wgt_input::{is_cap_touched, is_touch_active, is_touched, is_touched_from_start, touch_active_config, touch_transform};
