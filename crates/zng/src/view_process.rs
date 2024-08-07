//! View process implementations.
//!
//! This module provides the [`default`] view-process implementation and a [`prebuilt`] version of it.
//!
//! ```
//! use zng::prelude::*;
//! use zng::view_process::default as view_process;
//! // use zng::view_process::prebuilt as view_process;
//!
//! fn main() {
//!     // single_process();
//!     // multi_process();
//! }
//!
//! fn multi_process() {
//!     zng::env::init!();
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
/// See [`zng_view`] for the full API.
#[cfg(view)]
pub mod default {
    pub use zng_view::run_same_process;

    /// Android backend required types.
    ///
    /// See [`winit::platform::android`](https://docs.rs/winit/latest/winit/platform/android/) for more details
    /// on how to select a backend "Activity".
    ///
    /// # Full API
    ///
    /// See [`zng_view::platform::android`] for the full API.
    #[cfg(target_os = "android")]
    pub mod android {
        pub use zng_view::platform::android::activity::AndroidApp;
    }
}

/// Default view-process implementation as an embedded precompiled binary.
///
/// # Full API
///
/// See [`zng_view_prebuilt`] and [`zng_view`] for the full API.
#[cfg(view_prebuilt)]
pub mod prebuilt {
    pub use zng_view_prebuilt::run_same_process;
}
