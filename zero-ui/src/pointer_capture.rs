//! Pointer capture service, properties, events and types.
//!
//! # Full API
//!
//! See [`zero_ui_ext_input::pointer_capture`] and [`zero_ui_wgt_input::pointer_capture`] for the full pointer capture API.

pub use zero_ui_ext_input::pointer_capture::{CaptureInfo, CaptureMode, PointerCaptureArgs, POINTER_CAPTURE, POINTER_CAPTURE_EVENT};

pub use zero_ui_wgt_input::pointer_capture::{
    capture_pointer, capture_pointer_on_init, on_got_pointer_capture, on_lost_pointer_capture, on_pointer_capture_changed,
    on_pre_got_pointer_capture, on_pre_lost_pointer_capture, on_pre_pointer_capture_changed,
};
