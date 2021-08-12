//! Windowing and renderer.
//!
//! Zero-Ui isolates all OpenGL related code to a different process to be able to recover from driver errors.
//! This crate contains the `glutin` and `webrender` code that interacts with the actual system. Communication
//! with the app process is done using `ipmpsc`.

mod controller;
mod message;
mod view;

use std::{env, path::PathBuf};

const CHANNEL_VAR: &str = "ZERO_UI_WR_CHANNELS";

/// Version 0.1
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Call this method before anything else in the app `main` function.
///
/// A second instance of the app executable will be started to run as the windowing and renderer process,
/// in that instance this function highjacks the process and never returns.
///
/// # Examples
///
/// ```
/// # mod zero_ui { pub mod core { pub fn init() } }
/// fn main() {
///     zero_ui::core:::init();
///
///     // .. init app normally.
/// }
/// ```
pub fn init() {
    if let Ok(channel_dir) = env::var(CHANNEL_VAR) {
        view::run(PathBuf::from(channel_dir));
    }
}

pub use controller::App;
pub use message::{OpenWindowRequest, StartRequest};
