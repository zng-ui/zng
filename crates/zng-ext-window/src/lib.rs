#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! App window and monitors manager.
//!
//! # Events
//!
//! Events this extension provides:
//!
//! * [`WINDOW_OPEN_EVENT`]
//! * [`WINDOW_CHANGED_EVENT`]
//! * [`WINDOW_FOCUS_CHANGED_EVENT`]
//! * [`WINDOW_CLOSE_REQUESTED_EVENT`]
//! * [`WINDOW_CLOSE_EVENT`]
//! * [`MONITORS_CHANGED_EVENT`]
//!
//! # Services
//!
//! Services this extension provides:
//!
//! * [`WINDOWS`]
//! * [`MONITORS`]
//!
//! The [`WINDOWS`] service is also setup as the implementer for [`IMAGES`] rendering.
//!
//! [`IMAGES`]: zng_ext_image::IMAGES
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]
// suppress nag about very simple boxed closure signatures.
#![expect(clippy::type_complexity)]

#[macro_use]
extern crate bitflags;

mod ime;
pub use ime::*;

mod types;
pub use types::*;

mod monitor;
pub use monitor::*;

mod vars;
pub use vars::*;

mod app;
pub use app::*;

mod window;
pub use window::*;

mod windows;
pub use windows::*;

pub mod cmd;
