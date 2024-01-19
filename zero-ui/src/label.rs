//! Label widget and properties.
//!
//! The [`Label!`](struct@Label) widget is a text presenter that is focusable, when it receives
//! focus it can transfer it to another target widget.
//! 
//! ```
//! use zero_ui::prelude::*;
//! # let _scope = APP.defaults();
//! 
//! # let _ =
//! Container! {
//!     child_start = zero_ui::label::Label!("Name", "name-field"), 5;
//!     child = TextInput! {
//!         id = "name-field";
//!         txt = var_from("");
//!     };
//! }
//! # ;
//! ```
//! 
//! # Full API
//!
//! See [`zero_ui_wgt_text_input::label`] for the full widget API.

pub use zero_ui_wgt_text_input::label::{style_fn, DefaultStyle, Label};
