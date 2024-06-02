use std::mem;

use parking_lot::Mutex;

/// Register a `FnOnce(&ProcessStartArgs)` closure to be called on [`init!`].
///
/// Components that spawn special process instances implemented on the same executable
/// can use this macro to inject their own "main" without needing to ask the user to plug an init
/// function on the executable main. The component can spawn an instance of the current executable
/// with marker environment variables that identify the component's process.
///
/// [`init!`]: crate::init!
///
/// # Examples
///
/// The example below declares a "main" for a foo component and a function that spawns it.
///
/// ```
/// zng_env::on_process_start!(|_| {
///     if std::env::var("FOO_MARKER").is_ok() {
///         println!("Spawned as foo!");
///         zng_env::exit(0);
///     }
/// });
///
/// fn main() {
///     zng_env::init!(); // foo_main OR
///     // normal main
/// }
///
/// pub fn spawn_foo() -> std::io::Result<()> {
///     std::process::Command::new(std::env::current_exe()?).env("FOO_MARKER", "").spawn()?;
///     Ok(())
/// }
/// ```
///
/// Note the use of [`exit`], it is important to call it to collaborate with [`on_process_exit`] handlers.
#[macro_export]
macro_rules! on_process_start {
    ($closure:expr) => {
        // expanded from:
        // #[linkme::distributed_slice(ZNG_ENV_ON_PROCESS_START)]
        // static _ON_PROCESS_START: fn...;
        // so that users don't need to depend on linkme just to call this macro.
        #[used]
        #[cfg_attr(
            any(
                target_os = "none",
                target_os = "linux",
                target_os = "android",
                target_os = "fuchsia",
                target_os = "psp"
            ),
            link_section = "linkme_ZNG_ENV_ON_PROCESS_START"
        )]
        #[cfg_attr(
            any(target_os = "macos", target_os = "ios", target_os = "tvos"),
            link_section = "__DATA,__linkme7nCnSSdn,regular,no_dead_strip"
        )]
        #[cfg_attr(target_os = "windows", link_section = ".linkme_ZNG_ENV_ON_PROCESS_START$b")]
        #[cfg_attr(target_os = "illumos", link_section = "set_linkme_ZNG_ENV_ON_PROCESS_START")]
        #[cfg_attr(target_os = "freebsd", link_section = "linkme_ZNG_ENV_ON_PROCESS_START")]
        #[doc(hidden)]
        static _ON_PROCESS_START: fn(&$crate::ProcessStartArgs) = _on_process_start;
        fn _on_process_start(args: &$crate::ProcessStartArgs) {
            fn on_process_start(args: &$crate::ProcessStartArgs, handler: impl FnOnce(&$crate::ProcessStartArgs)) {
                handler(args)
            }
            on_process_start(args, $closure)
        }
    };
}

#[doc(hidden)]
#[linkme::distributed_slice]
pub static ZNG_ENV_ON_PROCESS_START: [fn(&ProcessStartArgs)];

pub(crate) fn process_init() -> impl Drop {
    let process_state = std::mem::replace(
        &mut *zng_unique_id::hot_static_ref!(PROCESS_LIFETIME_STATE).lock(),
        ProcessLifetimeState::Inited,
    );
    assert_eq!(process_state, ProcessLifetimeState::BeforeInit, "init!() already called");

    let args = ProcessStartArgs { _private: () };
    for h in ZNG_ENV_ON_PROCESS_START {
        h(&args);
    }
    MainExitHandler
}

/// Arguments for [`on_process_start`] handlers.
///
/// Empty in this release
pub struct ProcessStartArgs {
    _private: (),
}

struct MainExitHandler;
impl Drop for MainExitHandler {
    fn drop(&mut self) {
        run_exit_handlers(if std::thread::panicking() { 101 } else { 0 })
    }
}

type ExitHandler = Box<dyn FnOnce(&ProcessExitArgs) + Send + 'static>;
use super::*;
zng_unique_id::hot_static! {
    static ON_PROCESS_EXIT: Mutex<Vec<ExitHandler>> = Mutex::new(vec![]);
}

/// Terminates the current process with the specified exit code.
///
/// This function must be used instead of `std::process::exit` as it runs the [`on_process_exit`].
pub fn exit(code: i32) -> ! {
    run_exit_handlers(code);
    std::process::exit(code)
}

fn run_exit_handlers(code: i32) {
    *zng_unique_id::hot_static_ref!(PROCESS_LIFETIME_STATE).lock() = ProcessLifetimeState::Exiting;

    let on_exit = mem::take(&mut *zng_unique_id::hot_static_ref!(ON_PROCESS_EXIT).lock());
    let args = ProcessExitArgs { code };
    for h in on_exit {
        h(&args);
    }
}

/// Arguments for [`on_process_exit`] handlers.
pub struct ProcessExitArgs {
    /// Exit code that will be used.
    pub code: i32,
}

/// Register a `handler` to run once when the current process exits.
///
/// Note that the handler is only called if the process is terminated by [`exit`], or by the executable main
/// function returning if [`init!`] is called on it.
///
/// [`init!`]: crate::init!
pub fn on_process_exit(handler: impl FnOnce(&ProcessExitArgs) + Send + 'static) {
    zng_unique_id::hot_static_ref!(ON_PROCESS_EXIT).lock().push(Box::new(handler))
}

/// Defines the state of the current process instance.
///
/// Use [`process_lifetime_state()`] to get.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessLifetimeState {
    /// Init not called yet.
    BeforeInit,
    /// Init called and the function where it is called has not returned yet.
    Inited,
    /// Init called and the function where it is called is returning.
    Exiting,
}

zng_unique_id::hot_static! {
    static PROCESS_LIFETIME_STATE: Mutex<ProcessLifetimeState> = Mutex::new(ProcessLifetimeState::BeforeInit);
}

/// Get the state of the current process instance.
pub fn process_lifetime_state() -> ProcessLifetimeState {
    *zng_unique_id::hot_static_ref!(PROCESS_LIFETIME_STATE).lock()
}

/// Panics with an standard message if `zng::env::init!()` was not called or was not called correctly.
pub fn assert_inited() {
    match process_lifetime_state() {
        ProcessLifetimeState::BeforeInit => panic!("env not inited, please call `zng::env::init!()` in main"),
        ProcessLifetimeState::Inited => {}
        ProcessLifetimeState::Exiting => {
            panic!("env not inited correctly, please call `zng::env::init!()` at the beginning of the actual main function")
        }
    }
}
