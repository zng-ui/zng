#![cfg(feature = "text_input")]

//! Label widget and properties.
//!
//! The [`Label!`](struct@Label) widget is a text presenter that is focusable, when it receives
//! focus it can transfer it to another target widget.
//!
//! ```
//! use zng::prelude::*;
//! # fn example() {
//!
//! # let _ =
//! Container! {
//!     child_start = zng::label::Label!("Name", "name-field"), 5;
//!     child = TextInput! {
//!         id = "name-field";
//!         txt = var_from("");
//!     };
//! }
//! # ; }
//! ```
//!
//! # Full API
//!
//! See [`zng_wgt_text_input::label`] for the full widget API.

pub use zng_wgt_text_input::label::{DefaultStyle, Label, style_fn};
