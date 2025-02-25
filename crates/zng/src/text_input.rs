#![cfg(feature = "text_input")]

//! Text input widget and properties.
//!
//! The [`TextInput!`](struct@TextInput) widget is an text or parsed value editor that is styleable.
//!
//! The example below defines 3 text inputs with the [`FieldStyle!`](struct@FieldStyle).
//!
//! ```
//! use zng::prelude::*;
//! # let _scope = APP.defaults();
//!
//! # let _ =
//! Stack! {
//!     zng::text_input::style_fn = style_fn!(|_| zng::text_input::FieldStyle!());
//!     children = ui_vec![
//!         TextInput! {
//!             txt = var(Txt::from("name"));
//!             max_chars_count = 50;
//!         },
//!         TextInput! {
//!             txt_parse = var(0u32);
//!             zng::text_input::field_help = "help text";
//!             // txt_parse_on_stop = true;
//!         },
//!         TextInput! {
//!             txt = var_from("pass");
//!             obscure_txt = true;
//!         },
//!     ];
//!     direction = StackDirection::top_to_bottom();
//!     spacing = 5;
//! }
//! # ;
//! ```
//!
//! The first input binds directly to a `Txt` read-write variable. The second field binds to an `u32` read-write variable using the
//! [`txt_parse`](struct@TextInput#method.txt_parse) property. The third field obscures the text. The `FieldStyle!` adds data validation
//! adorners to the `TextInput!`, in the first field a char count is shown, in the second field the [`field_help`](fn@field_help)
//! or parse errors are shown.
//!
//! # Full API
//!
//! See [`zng_wgt_text_input`] for the full widget API.

pub use zng_wgt_text_input::{
    DefaultStyle, FieldStyle, SearchStyle, TextInput, data_notes_adorner_fn, field_help, max_chars_count_adorner_fn, style_fn,
};
