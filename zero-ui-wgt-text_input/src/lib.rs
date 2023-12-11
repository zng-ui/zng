//! Text input and label widgets.

#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zero_ui_wgt::enable_widget_macros!();

pub mod label;

mod text_input;
pub use text_input::*;
