//! View process implementations.
//!
//! This module provides the [`default`] view-process implementation and a [`prebuilt`] version of it.
//!
//! ```
//! use zero_ui::prelude::*;
//! use zero_ui::view_process::default as view_process;
//! // use zero_ui::view_process::prebuilt as view_process;
//!
//! fn main() {
//!     // single_process();
//!     // multi_process();
//! }
//!
//! fn multi_process() {
//!     view_process::init();
//!     app();
//! }
//!
//! fn single_process() {
//!     view_process::run_same_process(app);
//! }
//!
//! fn app() {
//!     APP.defaults().run_window(async {
//!         Window! {
//!         }
//!     })
//! }
//! ```
//!
//! See the [`app`](crate::app) module documentation for more details about view-processes.

/// Default view-process implementation.
///
/// # Full API
///
/// See [`zero_ui_view`] for the full API.
#[cfg(feature = "view")]
pub mod default {
    pub use zero_ui_view::{init, run_same_process};
}

/// Default view-process implementation as an embedded precompiled binary.
///
/// # Full API
///
/// See [`zero_ui_view_prebuilt`] and [`zero_ui_view`] for the full API.
#[cfg(feature = "view_prebuilt")]
pub mod prebuilt {
    pub use zero_ui_view_prebuilt::{init, run_same_process};
}
