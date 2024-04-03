#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
//!
//! Text input and label widgets.

#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zng_wgt::enable_widget_macros!();

pub mod label;
pub mod selectable;

mod text_input;
pub use text_input::*;
