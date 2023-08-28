//! Mouse and touch capture events.

use crate::core::{context::WIDGET, pointer_capture::*};

use super::event_property;

event_property! {
    /// Widget acquired mouse and touch capture.
    pub fn got_pointer_capture {
        event: POINTER_CAPTURE_EVENT,
        args: PointerCaptureArgs,
        filter: |args| args.is_got(WIDGET.id()),
    }

    /// Widget lost mouse and touch capture.
    pub fn lost_pointer_capture {
        event: POINTER_CAPTURE_EVENT,
        args: PointerCaptureArgs,
        filter: |args| args.is_lost(WIDGET.id()),
    }

    /// Widget acquired or lost mouse and touch capture.
    pub fn pointer_capture_changed {
        event: POINTER_CAPTURE_EVENT,
        args: PointerCaptureArgs,
    }
}
