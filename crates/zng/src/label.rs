#![cfg(feature = "text_input")]

//! Label widget and properties.
//!
//! The [`Label!`](struct@Label) widget is a text presenter that represents a label.
//!
//! An optional `target` widget can be set, the target is focused when the label is clocked.
//!
//! Labels also integrate with the mnemonic shortcuts and will automatically hide mnemonic markers from text if a parent widget
//! is [`mnemonic`]. Labels can also underline the mnemonic character when active if [`mnemonic_underline`] is set on the widget or context.
//!
//! [`mnemonic`]: fn@zng::gesture::mnemonic
//! [`mnemonic_underline`]: fn@zng::label::mnemonic_underline
//!
//! ```
//! use zng::prelude::*;
//! # fn example() {
//!
//! # let _ =
//! Container! {
//!     child_start = zng::label::Label!("Name", "name-field");
//!     child_spacing = 5;
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

pub use zng_wgt_text_input::label::{DefaultStyle, Label, mnemonic_underline, style_fn};
// TODO(breaking) Label! in prelude
