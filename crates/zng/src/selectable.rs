#![cfg(feature = "text_input")]

//! Selectable text widget and properties.
//!
//! The [`SelectableText!`](struct@SelectableText) is a read-only styleable text with text selection enabled.
//! Any `Text!` widget can enable selection using the `txt_selectable` property, this widget complements that
//! by adding a context menu and touch selection toolbar. The widget should be used for every text the user might wish to copy.
//!
//! The example below uses the widget to display an error message, it looks just like a simple `Text!` rendered, but
//! the cursor is different, the `SELECT_ALL_CMD` and `COPY_CMD` commands are available in the context menu and
//! the selection toolbar (if selected by touch).
//!
//! ```
//! use zng::prelude::*;
//! fn show_error(msg: impl Into<Txt>) {
//!     LAYERS.insert(
//!         LayerIndex::TOP_MOST,
//!         Container! {
//!             id = "error-dlg";
//!             widget::modal = true;
//!             child_align = layout::Align::CENTER;
//!             child = Container! {
//!                 padding = 10;
//!                 widget::background_color = colors::RED.desaturate(80.pct());
//!                 child_top = text::Strong!("Error"), 5;
//!                 child = SelectableText!(msg.into());
//!                 child_bottom =
//!                     Button! {
//!                         child = Text!("Ok");
//!                         layout::align = layout::Align::END;
//!                         on_click = hn!(|_| {
//!                             LAYERS.remove("error-dlg");
//!                         });
//!                     },
//!                     10,
//!                 ;
//!             };
//!         },
//!     );
//! }
//! ```
//!
//! # Full API
//!
//! See [`zng_wgt_text_input::selectable`] for the full widget API.

pub use zng_wgt_text_input::selectable::{DefaultStyle, SelectableText, style_fn};
