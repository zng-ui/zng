#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! Input events and focus properties.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

pub mod cmd;
pub mod drag_drop;
pub mod focus;
pub mod gesture;
pub mod keyboard;
pub mod mouse;
pub mod pointer_capture;
pub mod touch;

mod misc;
pub use misc::*;

mod state;
pub use state::*;

mod touch_props;
pub use touch_props::*;
