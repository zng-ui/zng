//! Demonstrates a hybrid web and desktop app setup.
//!
//! Note that only a small subset of services are supported and only headless (without renderer) apps can run.
//!
//! Use `cargo do run-wasm web` to run on the browser and `cargo do run web` to run standalone.

mod app;

fn main() {
    app::run();
}
