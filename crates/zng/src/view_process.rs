//! View process implementations.
//!
//! This module provides the [`default`] view-process implementation and a [`prebuilt`] version of it.
//!
//! ```no_run
//! use zng::prelude::*;
//! use zng::view_process::default as view_process;
//! // use zng::view_process::prebuilt as view_process;
//!
//! fn main() {
//!     zng::env::init!();
//!     // single_process();
//!     // multi_process();
//! }
//!
//! fn multi_process() {
//!     app();
//! }
//!
//! fn single_process() {
//!     view_process::run_same_process(app);
//! }
//!
//! fn app() {
//!     APP.defaults().run_window(async {
//!         Window! {}
//!     })
//! }
//! ```
//!
//! See the [`app`](crate::app) module documentation for more details about view-processes.
//!
//! See [`zng::env::init!`] for more details about running Android apps.

/// Default view-process implementation.
///
/// # Full API
///
/// See [`zng_view`] for the full API including view API extensions such as enabling ANGLE backend on Windows.
#[cfg(view)]
pub mod default {
    pub use zng_view::run_same_process;

    /// Android init types.
    ///
    /// See [`winit::platform::android`](https://docs.rs/winit/latest/winit/platform/android/) for more details
    /// on how to select a backend "Activity".
    ///
    /// See [`zng::env::init!`] for more details about running Android apps.
    ///
    /// # Full API
    ///
    /// See [`zng_view::platform::android`] for the full API.
    #[cfg(target_os = "android")]
    pub mod android {
        pub use zng_view::platform::android::{activity::AndroidApp, init_android_app};
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
