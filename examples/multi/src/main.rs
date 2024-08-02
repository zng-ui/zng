//! Demonstrates a web, mobile and desktop app setup.
//!
//! Note that web support is very limited, only a small subset of services are supported and
//! only headless (without renderer) apps can run.
//!
//! Use `cargo do run-wasm multi` to run on the browser, `cargo do run multi` to run on the desktop
//! and `cargo do run-apk multi` to run on Android (!!: TODO).

mod app;

fn main() {
    app::run();
}
