//! Platform specific types.

/// Android backend.
///
/// See [`winit::platform::android`](https://docs.rs/winit/latest/winit/platform/android/) for more details
/// on how to select a backend "Activity".
#[cfg(target_os = "android")]
pub mod android {
    pub use winit::platform::android::activity;
}
