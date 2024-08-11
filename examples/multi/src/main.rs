//! Demonstrates a web, mobile and desktop app setup.
//!
//! Note that web support is very limited, only a small subset of services are supported and
//! only headless (without renderer) apps can run.
//!
//! Use `cargo do run multi` to run on the desktop.
//!
//! Use `cargo do run-wasm multi` to run on the browser.
//!
//! Use `cargo do build-apk multi` to build a package and Android Studio "Profile or Debug APK" to run on a device.

mod app;

fn main() {
    zng::env::init!();
    app::run();
}
