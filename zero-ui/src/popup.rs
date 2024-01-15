//! Popup widget and properties.
//!
//! # Full API
//!
//! See [`zero_ui_wgt_layer::popup`] for the full widget API.

pub use zero_ui_wgt_layer::popup::{
    anchor_mode, close_delay, close_on_focus_leave, context_capture, is_close_delaying, on_popup_close_requested,
    on_pre_popup_close_requested, style_fn, ContextCapture, DefaultStyle, Popup, PopupCloseMode, PopupCloseRequestedArgs, PopupState,
    POPUP, POPUP_CLOSE_CMD, POPUP_CLOSE_REQUESTED_EVENT,
};
