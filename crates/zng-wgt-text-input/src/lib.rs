#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/master/examples/res/image/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/master/examples/res/image/zng-logo.png")]
//!
//! Text input and label widgets.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zng_wgt::enable_widget_macros!();

pub mod label;
pub mod selectable;

mod text_input;
pub use text_input::*;
