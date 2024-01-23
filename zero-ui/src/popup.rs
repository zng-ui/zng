//! Popup widget and properties.
//!
//! A popup is a temporary *flyover* inserted as a top-most layer using [`POPUP`] service. The service works
//! as an extension of the [`LAYERS`](crate::layer::LAYERS) service that implements the concept of *open* and *close* and
//! close requests. The [`Popup!`](struct@Popup) widget is a styleable container that is a good popup root widget.
//!
//! ```
//! use zero_ui::prelude::*;
//! # let _scope = APP.defaults();
//!
//!
//! let mut popup = None;
//! let is_closed = var(true);
//! # let _ =
//! Button! {
//!     layout::align = layout::Align::CENTER;
//!     child = Text!(is_closed.map(|&b| if b { "Open Popup" } else { "Close Popup" }.into()));
//!     on_click = hn!(|_| {
//!         if is_closed.get() {
//!             let p = POPUP.open(zero_ui::popup::Popup! {
//!                 child = Text!("Popup content!");
//!             });
//!             p.bind_map(&is_closed, |s| matches!(s, zero_ui::popup::PopupState::Closed)).perm();
//!             popup = Some(p);
//!         } else if let Some(p) = popup.take() {
//!             POPUP.close(&p);
//!         }
//!     })
//! }
//! # ;
//! ```
//!
//! The example above declares a button that opens and closes a popup
//!
//! Note that the toggle widget provides a [combo](crate::toggle#Combo) style and the [`checked_popup`](struct@crate::toggle::Toggle#checked_popup)
//! property that implements a similar behavior.
//!
//! # Full API
//!
//! See [`zero_ui_wgt_layer::popup`] for the full widget API.

pub use zero_ui_wgt_layer::popup::{
    anchor_mode, close_delay, close_on_focus_leave, context_capture, is_close_delaying, on_popup_close_requested,
    on_pre_popup_close_requested, style_fn, ContextCapture, DefaultStyle, Popup, PopupCloseMode, PopupCloseRequestedArgs, PopupState,
    POPUP, POPUP_CLOSE_CMD, POPUP_CLOSE_REQUESTED_EVENT,
};
