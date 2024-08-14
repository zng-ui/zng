//! Platform specific types.

/// Android backend.
///
/// See [`winit::platform::android`](https://docs.rs/winit/latest/winit/platform/android/) for more details
/// on how to select a backend "Activity".
#[cfg(target_os = "android")]
pub mod android {
    pub use winit::platform::android::activity;

    #[cfg(target_os = "android")]
    static ANDROID_APP: parking_lot::RwLock<Option<activity::AndroidApp>> = parking_lot::RwLock::new(None);

    /// Sets the [`android_app`] instance for this process and the Android paths.
    ///
    /// This must be called just after `zng::env::init!` and before `run_same_process*`.
    #[cfg(target_os = "android")]
    pub fn init_android_app(app: activity::AndroidApp) {
        let internal = app.internal_data_path().unwrap_or_default();
        let external = app.external_data_path().unwrap_or_default();
        zng_env::init_android_paths(internal, external);
        *ANDROID_APP.write() = Some(app);
    }

    /// Gets the `AndroidApp` instance for this process.
    ///
    /// Panics if called before [`init_android_app`].
    #[cfg(target_os = "android")]
    pub fn android_app() -> activity::AndroidApp {
        ANDROID_APP
            .read()
            .clone()
            .expect("android_app is only available after `init_android_app` call")
    }
}
