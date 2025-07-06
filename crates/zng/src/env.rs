//! Process events, external directories and metadata.
//!
//! This module contains functions and macros that operate on the executable level, not the app level. Zng apps
//! can have multiple process instances, most common a view-process and app-process pair, plus the crash handler.
//!
//! The app-process is the normal execution, the other processes use [`on_process_start!`] to takeover the
//! process if specific environment variables are set. The process start handlers are called on [`init!`],
//! if a process takeover it exits without returning, so only the normal app-process code executes after `init!()`.
//!
//! ```no_run
//! fn main() {
//!    println!("print in all processes");
//!    zng::env::init!();
//!    println!("print only in the app-process");
//!
//!    // get a path in the app config dir, the config dir is created if needed.    
//!    let my_config = zng::env::config("my-config.txt");
//!
//!    // read a config file, or create it
//!    if let Ok(c) = std::fs::read_to_string(&my_config) {
//!       println!("{c}");
//!    } else {
//!       std::fs::write(zng::env::config("my-config.txt"), b"Hello!").unwrap();
//!    }
//! }
//! ```
//!
//! Note that init **must be called in main**, it must be called in main to define the lifetime of the processes,
//! this is needed to properly call [`on_process_exit`] handlers.
//!
//! Also see [`init!`] docs for details init in `"wasm32"` and `"android"` target builds.
//!
//! # Full API
//!
//! See [`zng_env`] for the full API.

pub use zng_env::{
    About, ProcessExitArgs, ProcessStartArgs, about, bin, cache, clear_cache, config, exit, init, init_cache, init_config, init_res,
    migrate_cache, migrate_config, on_process_exit, on_process_start, process_name, res,
};

#[cfg(target_os = "android")]
pub use zng_env::{android_external, android_install_res};

#[cfg(any(debug_assertions, feature = "built_res"))]
pub use zng_env::init_built_res;
