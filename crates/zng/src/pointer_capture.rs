//! Pointer capture service, properties, events and other types.
//!
//! Pointer events target to the topmost widget under the pointer by default, the [`POINTER_CAPTURE`] service
//! can be used to *capture* the pointer for a widget so that it remains the target for pointer events. The
//! [`capture_pointer`](fn@capture_pointer) property can be used to automatically capture the pointer when pressed.
//!
//! ```
//! use zng::prelude::*;
//! # fn example() {
//!
//! # let _ =
//! Wgt! {
//!     zng::pointer_capture::capture_pointer = true;
//!
//!     zng::pointer_capture::on_got_pointer_capture = hn!(|_| {
//!         println!("got capture");
//!     });
//!     zng::pointer_capture::on_lost_pointer_capture = hn!(|_| {
//!         println!("lost capture");
//!     });
//!
//!     when *#gesture::is_cap_hovered {
//!         widget::background_color = colors::GREEN;
//!     }
//!     widget::background_color = colors::RED;
//!     layout::size = 80;
//! }
//! # ; }
//! ```
//!
//! The example above declares a widget is green when hovered or is holding the pointer capture, the widget also logs
//! when it gets and loses the capture. Note that the [`gesture::is_cap_hovered`] state is not the same as [`gesture::is_hovered`],
//! if changed to the second the example will not be green when not hovering, even though the widget still holds the capture, pointer
//! capture changes the target of pointer events, but it does not mask the fact that the pointer is not actually over the widget.
//!
//! [`gesture::is_cap_hovered`]: fn@crate::gesture::is_cap_hovered
//! [`gesture::is_hovered`]: fn@crate::gesture::is_hovered
//!
//! # Full API
//!
//! See [`zng_ext_input::pointer_capture`] and [`zng_wgt_input::pointer_capture`] for the full pointer capture API.

pub use zng_ext_input::pointer_capture::{CaptureInfo, CaptureMode, POINTER_CAPTURE, POINTER_CAPTURE_EVENT, PointerCaptureArgs};

pub use zng_wgt_input::pointer_capture::{
    capture_pointer, capture_pointer_on_init, on_got_pointer_capture, on_lost_pointer_capture, on_pointer_capture_changed,
    on_pre_got_pointer_capture, on_pre_lost_pointer_capture, on_pre_pointer_capture_changed,
};
