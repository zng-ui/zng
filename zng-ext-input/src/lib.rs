#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/master/examples/res/image/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/master/examples/res/image/zng-logo.png")]
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
//!
//! Input events and focused widget.

#![warn(unused_extern_crates)]
#![warn(missing_docs)]

#[macro_use]
extern crate bitflags;

pub mod focus;
pub mod gesture;
pub mod keyboard;
pub mod mouse;
pub mod pointer_capture;
pub mod touch;
