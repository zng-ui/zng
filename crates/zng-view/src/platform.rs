//! Platform specific types.

/// Android backend.
///
/// See [`winit::platform::android`](https://docs.rs/winit/latest/winit/platform/android/) for more details
/// on how to select a backend "Activity".
#[cfg(target_os = "android")]
pub mod android {
    pub use winit::platform::android::activity;

    #[cfg(target_os = "android")]
    static ANDROID_APP: parking_lot::RwLock<Option<platform::android::activity::AndroidApp>> = parking_lot::RwLock::new(None);

    /// Sets the [`android_app`] instance for this process and the Android config paths.
    ///
    /// This must be called just after `zng::env::init!` and before `run_same_process*`.
    #[cfg(target_os = "android")]
    pub fn init_android_app(app: platform::android::activity::AndroidApp) {
        if let Some(p) = app.internal_data_path() {
            zng_env::init_config(p);
        }
        *ANDROID_APP.write() = Some(app);
    }

    /// Gets the `AndroidApp` instance for this process.
    ///
    /// Panics if called before [`init_android_app`].
    #[cfg(target_os = "android")]
    pub fn android_app() -> platform::android::activity::AndroidApp {
        ANDROID_APP
            .read()
            .clone()
            .expect("android_app is only available after `init_android_app` call")
    }
}
