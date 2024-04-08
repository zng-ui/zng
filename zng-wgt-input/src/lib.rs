#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/master/examples/res/image/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/master/examples/res/image/zng-logo.png")]
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
//!
//! Input events and focus properties.

#![warn(unused_extern_crates)]
#![warn(missing_docs)]

pub mod cmd;
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
