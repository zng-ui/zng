//! Zero-Ui View Process.
//!
//! Zero-Ui isolates all OpenGL and windowing related code to a different process to be able to recover from driver errors.
//! This crate contains the `glutin` and `webrender` code that interacts with the actual system. Communication
//! with the app process is done using `ipmpsc`.

mod controller;
mod message;
mod view;

use std::{env, path::PathBuf};

const CHANNEL_VAR: &str = "ZERO_UI_WR_CHANNELS";
const MODE_VAR: &str = "ZERO_UI_WR_MODE";

/// Version 0.1.
///
/// The *App Process* and *View Process* must be build using the same exact version of `zero-ui-vp` and this is
/// validated during run-time, causing a panic if the versions don't match. Usually the same executable is used
/// for both processes so this is not a problem.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Call this function before anything else in the app `main` function.
///
/// If the process is started with the right environment configuration this function
/// high-jacks the process and turns it into a *View Process*, never returning.
///
/// This function does nothing if the *View Process* environment is not set, you can safely call it more then once.
/// The `App::default()` and `App::blank()` methods also call this function, so if the first line of the `main` is
/// `App::default` you don't need to explicitly call the function.
///
/// # Examples
///
/// ```
/// # mod zero_ui { pub mod prelude { pub fn init_view_process() } }
/// fn main() {
///     zero_ui::prelude::init_view_process();
///
///     println!("Only Prints if is not View Process");
///     // .. init app normally.
/// }
/// ```
pub fn init_view_process() {
    if let Ok(channel_dir) = env::var(CHANNEL_VAR) {
        view::run(PathBuf::from(channel_dir));
    }
}

pub use controller::{App, WindowNotFound};
pub use message::{
    AxisId, ButtonId, CursorIcon, DevId, ElementState, Ev, Icon, ModifiersState, MouseButton, MouseScrollDelta, OpenWindowRequest,
    ScanCode, StartRequest, TextAntiAliasing, Theme, VirtualKeyCode, WinId,
};

pub use glutin::event_loop::ControlFlow;
