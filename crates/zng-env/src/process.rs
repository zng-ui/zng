use std::{
    mem,
    sync::atomic::{AtomicU8, Ordering},
};

use parking_lot::Mutex;

#[doc(hidden)]
#[cfg(not(target_arch = "wasm32"))]
pub use linkme as __linkme;

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
/// zng_env::on_process_start!(|args| {
///     if args.yield_count == 0 {
///         return args.yield_once();
///     }
///
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
/// Note that the handler yields once, this gives a chance for all handlers to run first before the handler is called again
/// and takes over the process. It is good practice to yield at least once to ensure handlers that are supposed to affect all
/// processes actually init, as an example, the trace recorder may never start for the process if it does not yield.
///
/// Also note the use of custom [`exit`], it is important to call it to collaborate with [`on_process_exit`] handlers.
///
/// # App Context
///
/// This event happens on the executable process context, before any `APP` context starts, you can use
/// `zng::APP::on_init` here to register a handler to be called in the app context, if and when it starts.
///
/// # Web Assembly
///
/// Crates that declare `on_process_start` must have the [`wasm_bindgen`] dependency to compile for the `wasm32` target.
///
/// In `Cargo.toml` add this dependency:
///
/// ```toml
/// [target.'cfg(target_arch = "wasm32")'.dependencies]
/// wasm-bindgen = "*"
/// ```
///
/// Try to match the version used by `zng-env`.
///
/// # Linker Optimizer Issues
///
/// The macOS system linker can "optimize" away crates that are only referenced via this macro, that is, a crate dependency
/// that is not otherwise directly addressed by code. To workaround this issue you can add a bogus reference to the crate code, something
/// that is not trivial to optimize away. Unfortunately this code must be added on the dependent crate, or on an intermediary dependency,
/// if your crate is at risk of being used this way please document this issue.
///
/// See [`zng#437`] for an example of how to fix this issue.
///
/// [`wasm_bindgen`]: https://crates.io/crates/wasm-bindgen
/// [`zng#437`]: https://github.com/zng-ui/zng/pull/437
#[macro_export]
macro_rules! on_process_start {
    ($closure:expr) => {
        $crate::__on_process_start! {$closure}
    };
}

#[cfg(not(target_arch = "wasm32"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __on_process_start {
    ($closure:expr) => {
        const _: () = {
            #[$crate::__linkme::distributed_slice($crate::ZNG_ENV_ON_PROCESS_START)]
            #[linkme(crate = $crate::__linkme)]
            #[doc(hidden)]
            static _ON_PROCESS_START: fn(&$crate::ProcessStartArgs) = _on_process_start;
            #[doc(hidden)]
            fn _on_process_start(args: &$crate::ProcessStartArgs) {
                fn on_process_start(args: &$crate::ProcessStartArgs, handler: impl FnOnce(&$crate::ProcessStartArgs)) {
                    handler(args)
                }
                on_process_start(args, $closure)
            }
        };
    };
}

#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
#[macro_export]
macro_rules! __on_process_start {
    ($closure:expr) => {
        $crate::wasm_process_start! {$crate,$closure}
    };
}

#[doc(hidden)]
#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen::prelude::wasm_bindgen;

#[doc(hidden)]
#[cfg(target_arch = "wasm32")]
pub use zng_env_proc_macros::wasm_process_start;
use zng_txt::Txt;

#[cfg(target_arch = "wasm32")]
std::thread_local! {
    #[doc(hidden)]
    pub static WASM_INIT: std::cell::RefCell<Vec<fn(&ProcessStartArgs)>> = const { std::cell::RefCell::new(vec![]) };
}

#[cfg(not(target_arch = "wasm32"))]
#[doc(hidden)]
#[linkme::distributed_slice]
pub static ZNG_ENV_ON_PROCESS_START: [fn(&ProcessStartArgs)];

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn process_init() -> impl Drop {
    process_init_impl(&ZNG_ENV_ON_PROCESS_START)
}

fn process_init_impl(handlers: &[fn(&ProcessStartArgs)]) -> MainExitHandler {
    let process_state = std::mem::replace(
        &mut *zng_unique_id::hot_static_ref!(PROCESS_LIFETIME_STATE).lock(),
        ProcessLifetimeState::Inited,
    );
    assert_eq!(process_state, ProcessLifetimeState::BeforeInit, "init!() already called");

    let mut yielded = vec![];
    let mut yield_until_app = vec![];
    let mut next_handlers_count = handlers.len();
    for h in handlers {
        next_handlers_count -= 1;
        let args = ProcessStartArgs {
            next_handlers_count,
            yield_count: 0,
            yield_requested: AtomicU8::new(0),
        };
        h(&args);
        match args.yield_requested.load(Ordering::Relaxed) {
            ProcessStartArgs::YIELD_ONCE => {
                yielded.push(h);
                next_handlers_count += 1;
            }
            ProcessStartArgs::YIELD_UNTIL_APP => {
                yield_until_app.push(h);
            }
            _ => {}
        }
    }

    let mut yield_count = 0;
    while !yielded.is_empty() {
        yield_count += 1;
        if yield_count > ProcessStartArgs::MAX_YIELD_COUNT {
            eprintln!("start handlers requested `yield_start` more them 32 times");
            break;
        }

        next_handlers_count = yielded.len();
        for h in mem::take(&mut yielded) {
            next_handlers_count -= 1;
            let args = ProcessStartArgs {
                next_handlers_count,
                yield_count,
                yield_requested: AtomicU8::new(0),
            };
            h(&args);
            match args.yield_requested.load(Ordering::Relaxed) {
                ProcessStartArgs::YIELD_ONCE => {
                    yielded.push(h);
                    next_handlers_count += 1;
                }
                ProcessStartArgs::YIELD_UNTIL_APP => {
                    yield_until_app.push(h);
                }
                _ => {}
            }
        }
    }

    for h in yield_until_app {
        let args = ProcessStartArgs {
            next_handlers_count: 0,
            yield_count: ProcessStartArgs::MAX_YIELD_COUNT,
            yield_requested: AtomicU8::new(0),
        };
        h(&args);
        if args.yield_requested.load(Ordering::Relaxed) != 0 {
            eprintln!("handler requested `yield_until_app` and then yielded again")
        }
    }
    MainExitHandler
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn process_init() -> impl Drop {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let window = web_sys::window().expect("cannot 'init!', no window object");
    let module = js_sys::Reflect::get(&window, &"__zng_env_init_module".into())
        .expect("cannot 'init!', missing module in 'window.__zng_env_init_module'");

    if module == wasm_bindgen::JsValue::undefined() || module == wasm_bindgen::JsValue::null() {
        panic!("cannot 'init!', missing module in 'window.__zng_env_init_module'");
    }

    let module: js_sys::Object = module.into();

    for entry in js_sys::Object::entries(&module) {
        let entry: js_sys::Array = entry.into();
        let ident = entry.get(0).as_string().expect("expected ident at entry[0]");

        if ident.starts_with("__zng_env_start_") {
            let func: js_sys::Function = entry.get(1).into();
            if let Err(e) = func.call0(&wasm_bindgen::JsValue::NULL) {
                panic!("'init!' function error, {e:?}");
            }
        }
    }

    process_init_impl(&WASM_INIT.with_borrow_mut(std::mem::take))
}

/// Arguments for [`on_process_start`] handlers.
///
/// Empty in this release.
pub struct ProcessStartArgs {
    /// Number of start handlers yet to run.
    pub next_handlers_count: usize,

    /// Number of times this handler has yielded.
    ///
    /// If this exceeds 32 times the handler is ignored.
    pub yield_count: u16,

    yield_requested: AtomicU8,
}
impl ProcessStartArgs {
    /// Yield requests after this are ignored.
    pub const MAX_YIELD_COUNT: u16 = 32;

    const YIELD_ONCE: u8 = 1;
    const YIELD_UNTIL_APP: u8 = 2;

    /// Let other process start handlers run first.
    ///
    /// The handler must call this if it takes over the process and it cannot determinate if it should from the environment.
    ///
    /// ```
    /// # macro_rules! on_process_start { ($($tt:tt)*) => { } }
    /// fn run_foo_process() {}
    /// on_process_start!(|args| {
    ///     if args.yield_count == 0 {
    ///         return args.yield_once();
    ///     }
    ///
    ///     // yielded once, handlers that affect all processes (loggers, tracers) are inited now
    ///     if std::env::var("IS_FOO").is_ok() {
    ///         // take over as "foo" process
    ///         run_foo_process();
    ///         zng_env::exit(0);
    ///     }
    /// });
    /// ```
    pub fn yield_once(&self) {
        self.yield_requested.store(Self::YIELD_ONCE, Ordering::Relaxed);
    }

    /// Yields until is running the the app-process.
    ///
    /// Returns `true` if should skip the handler.
    ///
    /// ```rust
    /// # macro_rules! on_process_start { ($($tt:tt)*) => { } }
    /// on_process_start!(|args| {
    ///     if args.yield_until_app() {
    ///         return;
    ///     }
    ///
    ///     println!("Is running in the app-process");
    /// });
    /// ```
    ///
    /// Note that the handler is still called before the `APP` context starts, you can register a `APP.on_init` handler
    /// to run in the new app context.
    pub fn yield_until_app(&self) -> bool {
        if self.next_handlers_count > 0 && self.yield_count < Self::MAX_YIELD_COUNT {
            // yield until we are the last handler, this ensures we are running in the app-process
            self.yield_requested.store(Self::YIELD_UNTIL_APP, Ordering::Relaxed);
            return true;
        }

        // init name early, since there is interest
        crate::init_process_name("app-process");

        false
    }
}

struct MainExitHandler;
impl Drop for MainExitHandler {
    fn drop(&mut self) {
        run_exit_handlers(if std::thread::panicking() { 101 } else { 0 })
    }
}

type ExitHandler = Box<dyn FnOnce(&ProcessExitArgs) + Send + 'static>;

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
#[non_exhaustive]
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
zng_unique_id::hot_static! {
    static PROCESS_NAME: Mutex<Txt> = Mutex::new(Txt::from_static(""));
}

/// Get the state of the current process instance.
pub fn process_lifetime_state() -> ProcessLifetimeState {
    *zng_unique_id::hot_static_ref!(PROCESS_LIFETIME_STATE).lock()
}

/// Gets a process runtime name.
///
/// The primary use of this name is to identify the process in logs, see [`set_process_name`] for details about the logged name.
/// On set or init the name is logged as an info message "pid: {pid}, name: {name}".
///
/// # Common Names
///
/// All Zng provided process handlers name the process.
///
/// * `"app-process"` - Set by `APP` if no other name was set before the app starts building.
/// * `"view-process"` - Set by the view-process implementer when running in multi process mode.
/// * `"crash-handler-process"` - Set by the crash-handler when running with crash handling.
/// * `"crash-dialog-process"` - Set by the crash-handler on the crash dialog process.
/// * `"worker-process ({worker_name}, {pid})"` - Set by task worker processes if no name was set before the task runner server starts.
pub fn process_name() -> Txt {
    zng_unique_id::hot_static_ref!(PROCESS_NAME).lock().clone()
}

/// Changes the process runtime name.
///
/// This sets [`process_name`] and traces an info message "pid: {pid}, name: {name}". If the same PID is named multiple times
/// the last name should be used when presenting the process in trace viewers.
///
/// The process name ideally should be set only by the [`on_process_start!`] "process takeover" handlers. You can use [`init_process_name`]
/// to only set the name if it has not been set yet.
pub fn set_process_name(name: impl Into<Txt>) {
    set_process_name_impl(name.into(), true);
}

/// Set the process runtime name if it has not been named yet.
///
/// See [`set_process_name`] for more details.
///
/// Returns `true` if the name was set.
pub fn init_process_name(name: impl Into<Txt>) -> bool {
    set_process_name_impl(name.into(), false)
}

fn set_process_name_impl(new_name: Txt, replace: bool) -> bool {
    let mut name = zng_unique_id::hot_static_ref!(PROCESS_NAME).lock();
    if replace || name.is_empty() {
        *name = new_name;
        drop(name);
        tracing::info!("pid: {}, name: {}", std::process::id(), process_name());
        true
    } else {
        false
    }
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
