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
//!     println!("print in all processes");
//!     zng::env::init!();
//!     println!("print only in the app-process");
//!
//!     // get a path in the app config dir, the config dir is created if needed.
//!     let my_config = zng::env::config("my-config.txt");
//!
//!     // read a config file, or create it
//!     if let Ok(c) = std::fs::read_to_string(&my_config) {
//!         println!("{c}");
//!     } else {
//!         std::fs::write(zng::env::config("my-config.txt"), b"Hello!").unwrap();
//!     }
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

/// Helpers for apps built with `#![windows_subsystem = "windows"]`.
///
/// The Windows operating system does not support hybrid CLI and GUI apps in the same executable,
/// this module contains helpers that help provide a *best effort* compatibility, based on the tricks
/// Microsoft Visual Studio uses.
///
/// The [`attach_console`] function must be called at the start of the hybrid executable when it determinate
/// it is running in CLI mode, the [`build_cli_com_proxy`] must be called in the build script for the hybrid executable.
///
/// # Examples
///
/// The following example declares a hybrid CLI and GUI app that makes full use of this API to work on Windows.
///
/// In the `main.rs` file the GUI mode is the normal run and the CLI mode is a process interception defined using
/// [`on_process_start!`]. When CLI is detected [`attach_console`] is called to enable printing.
///
/// ```no_run
/// // without this in Windows a console window opens with the GUI
/// #![windows_subsystem = "windows"]
///
/// fn main() {
///     zng::env::init!();
///
///     // GUI app running, in Windows this print does not output.
///     println!("GUI");
///     zng::APP.defaults().run(async {});
/// }
///
/// mod cli {
///     zng::env::on_process_start!(|_| {
///         // detect CLI mode
///         if std::env::args().skip(1).any(|a| a.starts_with("-")) {
///             // CLI app running
///
///             // connect to stdio
///             #[cfg(windows)]
///             zng::env::windows_subsystem::attach_console();
///             println!("CLI");
///
///             zng::env::exit(0);
///         }
///     });
/// }
/// ```
///
/// If you deploy just the above setup stdout/err will be already enabled, but there are some heavy limitations. The
/// parent console does not know the app is connected it will return the prompt immediately and any subsequent prints
/// are injected in the console, the user call other commands and both outputs will start mixing. To work around this
/// you can deploy a second executable that is not built for the `"windows"` subsystem, this executable proxies stdio to the main executable.
///
/// In the `Cargo.toml` file a build dependency in `zng-env` is defined.
///
/// ```toml
/// [dependencies]
/// zng = "0.21.8"
///
/// [target.'cfg(windows)'.build-dependencies]
/// zng-env = { version = "0.10.2", features = ["build_cli_com_proxy"] }
/// ```
///
/// And in `build.rs` the [`build_cli_com_proxy`] function is called.
///
/// ```no_run
/// # macro_rules! example { () => {
/// fn main() {
///     #[cfg(windows)]
///     zng_env::windows_subsystem::build_cli_com_proxy("foo.exe", None).unwrap();
/// }
/// # }}
/// ```
///
/// This will build a small executable named `foo.com`, beside the `foo.exe` file. Both must be deployed to the same
/// directory, when users call `foo --args` in a console window the Windows name resolution will give priory to
/// *.com files over *.exe and call `foo.com`, this executable will spawn the `foo.exe` and proxy all stdio to it while
/// blocking the console window because it is not built for the `"windows"` subsystem.
///
/// This is the same trick used by Visual Studio to support both CLI and GUI with the same `devenv` name.
///
/// [`attach_console`]: zng_env::windows_subsystem::attach_console
/// [`build_cli_com_proxy`]: https://zng-ui.github.io/doc/zng_env/windows_subsystem/fn.build_cli_com_proxy.html
///
/// # Full API
///
/// See [`zng_env::windows_subsystem`] for the full API.
pub mod windows_subsystem {
    pub use zng_env::windows_subsystem::attach_console;
}
