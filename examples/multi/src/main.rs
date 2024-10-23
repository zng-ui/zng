//! Demonstrates a web, mobile and desktop app setup.
//!
//! Use `cargo do run multi` to run on the desktop.
//!
//! Use `cargo do build-apk multi` to build a package and Android Studio "Profile or Debug APK" to run on a device.
//!
//! Use `cargo do run-wasm multi` to run on the browser.
//!
//! Use `cargo do build-ios multi` to build library for XCode iOS project.
//!
//! Note that WASM support is very limited, only a small subset of services are supported and
//! only headless (without renderer) apps can run. Also note that iOS does not build yet, support is planed after
//! Glutin implements it (or some other ANGLE based crate).

mod app;

fn main() {
    // usually resources are packed to a default dir using `cargo zng res --pack`
    zng::env::init_res(concat!(env!("CARGO_MANIFEST_DIR"), "/res"));

    zng::env::init!();
    app::run();
}
