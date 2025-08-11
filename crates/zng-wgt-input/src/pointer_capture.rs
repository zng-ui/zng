//! Mouse and touch capture properties.

use zng_ext_input::{
    mouse::MOUSE_INPUT_EVENT,
    pointer_capture::{POINTER_CAPTURE, POINTER_CAPTURE_EVENT, PointerCaptureArgs},
    touch::TOUCH_INPUT_EVENT,
};
use zng_wgt::prelude::*;

pub use zng_ext_input::pointer_capture::CaptureMode;

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

/// Capture mouse and touch for the widget on press.
///
/// The capture happens only if any mouse button or touch is pressed on the window and the `mode` is [`Widget`] or [`Subtree`].
///
/// Captures are released when all mouse buttons and touch contacts stop being pressed on the window.
/// The capture is also released back to window if the `mode` changes to [`Window`].
///
/// [`Widget`]: CaptureMode::Widget
/// [`Subtree`]: CaptureMode::Subtree
/// [`Window`]: CaptureMode::Window
#[property(CONTEXT, default(false))]
pub fn capture_pointer(child: impl IntoUiNode, mode: impl IntoVar<CaptureMode>) -> UiNode {
    let mode = mode.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_event(&MOUSE_INPUT_EVENT).sub_event(&TOUCH_INPUT_EVENT).sub_var(&mode);
        }
        UiNodeOp::Event { update } => {
            let mut capture = false;
            if let Some(args) = MOUSE_INPUT_EVENT.on(update) {
                capture = args.is_mouse_down();
            } else if let Some(args) = TOUCH_INPUT_EVENT.on(update) {
                capture = args.is_touch_start();
            }

            if capture {
                let widget_id = WIDGET.id();

                match mode.get() {
                    CaptureMode::Widget => {
                        POINTER_CAPTURE.capture_widget(widget_id);
                    }
                    CaptureMode::Subtree => {
                        POINTER_CAPTURE.capture_subtree(widget_id);
                    }
                    CaptureMode::Window => (),
                }
            }
        }
        UiNodeOp::Update { .. } => {
            if let Some(new_mode) = mode.get_new() {
                let tree = WINDOW.info();
                let widget_id = WIDGET.id();
                if tree.get(widget_id).map(|w| w.interactivity().is_enabled()).unwrap_or(false)
                    && let Some(current) = POINTER_CAPTURE.current_capture().get()
                    && current.target.widget_id() == widget_id
                {
                    // If mode updated and we are capturing the mouse:
                    match new_mode {
                        CaptureMode::Widget => POINTER_CAPTURE.capture_widget(widget_id),
                        CaptureMode::Subtree => POINTER_CAPTURE.capture_subtree(widget_id),
                        CaptureMode::Window => POINTER_CAPTURE.release_capture(),
                    }
                }
            }
        }
        _ => {}
    })
}

/// Capture mouse and touch for the widget on init.
///
/// The capture happens only if any mouse button or touch is pressed on the window and the `mode` is [`Widget`] or [`Subtree`].
///
/// Pointer captures are released when all mouse buttons stop being pressed on the window.
/// The capture is also released back to window if the `mode` changes to [`Window`] when the mouse is captured for the widget.
///
/// [`Widget`]: CaptureMode::Widget
/// [`Subtree`]: CaptureMode::Subtree
/// [`Window`]: CaptureMode::Window
#[property(CONTEXT, default(false))]
pub fn capture_pointer_on_init(child: impl IntoUiNode, mode: impl IntoVar<CaptureMode>) -> UiNode {
    let mode = mode.into_var();
    let mut capture = true;

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&mode);
            capture = true; // wait for info
        }
        UiNodeOp::Info { .. } => {
            if std::mem::take(&mut capture) {
                let widget_id = WIDGET.id();

                match mode.get() {
                    CaptureMode::Widget => {
                        POINTER_CAPTURE.capture_widget(widget_id);
                    }
                    CaptureMode::Subtree => {
                        POINTER_CAPTURE.capture_subtree(widget_id);
                    }
                    CaptureMode::Window => (),
                }
            }
        }
        UiNodeOp::Update { .. } => {
            if let Some(new_mode) = mode.get_new() {
                let tree = WINDOW.info();
                let widget_id = WIDGET.id();
                if tree.get(widget_id).map(|w| w.interactivity().is_enabled()).unwrap_or(false)
                    && let Some(current) = POINTER_CAPTURE.current_capture().get()
                    && current.target.widget_id() == widget_id
                {
                    // If mode updated and we are capturing the mouse:
                    match new_mode {
                        CaptureMode::Widget => POINTER_CAPTURE.capture_widget(widget_id),
                        CaptureMode::Subtree => POINTER_CAPTURE.capture_subtree(widget_id),
                        CaptureMode::Window => POINTER_CAPTURE.release_capture(),
                    }
                }
            }
        }
        _ => {}
    })
}
