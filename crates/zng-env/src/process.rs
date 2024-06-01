/// Register a `fn() -> !` pointer to be called on [`init!`].
///
/// Components that spawn special process instances implemented on the same executable
/// can use this macro to inject their own "main" without needing to ask the user to plug an init
/// function on the executable main. The component can set the [`PROCESS_MAIN`] env var to spawn an
/// instance of the executable that run as the component's process.
/// 
/// [`init!`]: crate::init!
///
/// # Examples
///
/// The example below declares a "main" for a foo component and a function that spawns it.
///
/// ```
/// zng_env::process_main!("my-crate/foo-process" => foo_main);
/// fn foo_main() -> ! {
///     println!("Spawned as foo!");
///     std::process::exit(0)
/// }
///
/// fn main() {
///     zng_env::init!(); // foo_main OR
///     // normal main
/// }
///
/// pub fn spawn_foo() -> std::io::Result<()> {
///     std::process::Command::new(std::env::current_exe()?).env(zng_env::PROCESS_MAIN, "my-crate/foo-process").spawn()?;
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! process_main {
    ($name:tt => $init_fn:path) => {
        // expanded from:
        // #[linkme::distributed_slice(ZNG_ENV_RUN_PROCESS)]
        // static _PROCESS_MAIN = $crate::RunAsProcessHandler = $crate::RunAsProcessHandler::new($name, $init_fn);
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
            link_section = "linkme_ZNG_ENV_RUN_PROCESS"
        )]
        #[cfg_attr(
            any(target_os = "macos", target_os = "ios", target_os = "tvos"),
            link_section = "__DATA,__linkme7ZNvpcpD,regular,no_dead_strip"
        )]
        #[cfg_attr(target_os = "windows", link_section = ".linkme_ZNG_ENV_RUN_PROCESS$b")]
        #[cfg_attr(target_os = "illumos", link_section = "set_linkme_ZNG_ENV_RUN_PROCESS")]
        #[cfg_attr(target_os = "freebsd", link_section = "linkme_ZNG_ENV_RUN_PROCESS")]
        #[doc(hidden)]
        static _PROCESS_MAIN: $crate::RunAsProcessHandler = $crate::RunAsProcessHandler::new($name, $init_fn);
    };
}

/// Environment variable name that must be set to the [`process_main!`] name to run that process.
pub const PROCESS_MAIN: &str = "ZNG_ENV_PROCESS_MAIN";

#[doc(hidden)]
#[linkme::distributed_slice]
pub static ZNG_ENV_RUN_PROCESS: [RunAsProcessHandler];

#[doc(hidden)]
pub struct RunAsProcessHandler {
    name: &'static str,
    handler: fn() -> !,
}
impl RunAsProcessHandler {
    pub const fn new(name: &'static str, handler: fn() -> !) -> Self {
        Self { name, handler }
    }
}

pub(crate) fn process_init() {
    let name = std::env::var(PROCESS_MAIN).unwrap_or_default();
    for h in ZNG_ENV_RUN_PROCESS {
        if h.name == name {
            (h.handler)() // -> !
        }
    }
    if !name.is_empty() {
        panic!("{PROCESS_MAIN}={name:?} is not registered with process_main!");
    }
}
