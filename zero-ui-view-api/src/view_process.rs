use std::{
    env, thread,
    time::{Duration, Instant},
};

use crate::{MODE_VAR, SERVER_NAME_VAR, VERSION_VAR};

/// Configuration for starting a view-process.
#[derive(Clone, Debug)]
pub struct ViewConfig {
    /// The [`VERSION`] of the API crate in the app-process.
    ///
    /// [`VERSION`]: crate::VERSION
    pub version: String,

    /// Name of the initial channel used in [`connect_view_process`] to setup the connections to the
    /// client app-process.
    ///
    /// [`connect_view_process`]: crate::connect_view_process
    pub server_name: String,

    /// If the server should consider all window requests, headless window requests.
    pub headless: bool,
}
impl ViewConfig {
    /// Reads config from environment variables set by the [`Controller`] in a view-process instance.
    ///
    /// View API implementers should call this to get the config when it suspects that is running as a view-process.
    /// Returns `Some(_)` if the process was initialized as a view-process.
    ///
    /// [`Controller`]: crate::Controller
    pub fn from_env() -> Option<Self> {
        if let (Ok(version), Ok(server_name)) = (env::var(VERSION_VAR), env::var(SERVER_NAME_VAR)) {
            let headless = env::var(MODE_VAR).map(|m| m == "headless").unwrap_or(false);
            Some(ViewConfig {
                version,
                server_name,
                headless,
            })
        } else {
            None
        }
    }

    /// Returns `true` if the current process is awaiting for the config to start the
    /// view process in the same process.
    pub(crate) fn is_awaiting_same_process() -> bool {
        env::var_os(Self::SAME_PROCESS_VAR).unwrap_or_default() == Self::SG_WAITING
    }

    /// Sets and unblocks the same-process config if there is a request.
    ///
    /// # Panics
    ///
    /// If there is no pending `wait_same_process`.
    pub(crate) fn set_same_process(cfg: ViewConfig) {
        if Self::is_awaiting_same_process() {
            let cfg = format!("{}\n{}\n{}", cfg.version, cfg.server_name, cfg.headless);
            env::set_var(Self::SAME_PROCESS_VAR, cfg);
        } else {
            unreachable!("use `waiting_same_process` to check, then call `set_same_process` only once")
        }
    }

    /// Wait for config from same-process.
    ///
    /// View API implementers should call this to sign that view-process config should be send to the same process
    /// and then start the "app-process" code path in a different thread. This function returns when the app code path sends
    /// the "view-process" configuration.
    pub fn wait_same_process() -> Self {
        let _s = tracing::trace_span!("ViewConfig::wait_same_process").entered();

        if env::var_os(Self::SAME_PROCESS_VAR).is_some() {
            panic!("`wait_same_process` can only be called once");
        }

        env::set_var(Self::SAME_PROCESS_VAR, Self::SG_WAITING);

        let time = Instant::now();
        let timeout = Duration::from_secs(5);

        while Self::is_awaiting_same_process() {
            thread::yield_now();
            if time.elapsed() >= timeout {
                panic!("timeout, `wait_same_process` waited for `{timeout:?}`");
            }
        }

        let config = env::var(Self::SAME_PROCESS_VAR).unwrap();

        env::set_var(Self::SAME_PROCESS_VAR, Self::SG_DONE);

        let config: Vec<_> = config.lines().collect();
        assert_eq!(config.len(), 3, "var `{}` format incorrect, expected 3 lines", Self::SAME_PROCESS_VAR);

        ViewConfig {
            version: config[0].to_owned(),
            server_name: config[1].to_owned(),
            headless: config[2] == "true",
        }
    }

    /// Assert that the [`VERSION`] is the same in the app-process and view-process.
    ///
    /// This method must be called in the view-process implementation, it fails if the versions don't match, panics if
    /// `is_same_process` or writes to *stderr* and exits with code .
    ///
    /// [`VERSION`]: crate::VERSION
    pub fn assert_version(&self, is_same_process: bool) {
        if self.version != crate::VERSION {
            let msg = format!(
                "view API version is not equal, app-process: {}, view-process: {}",
                self.version,
                crate::VERSION
            );
            if is_same_process {
                panic!("{}", msg)
            } else {
                eprintln!("{}", msg);
                std::process::exit(i32::from_le_bytes(*b"vapi"));
            }
        }
    }

    /// Returns `true` if a view-process exited because of [`assert_version`].
    ///
    /// [`assert_version`]: Self::assert_version
    pub fn is_version_err(exit_code: Option<i32>, stderr: Option<&str>) -> bool {
        exit_code.map(|e| e == i32::from_le_bytes(*b"vapi")).unwrap_or(false)
            || stderr.map(|s| s.contains("view API version is not equal")).unwrap_or(false)
    }

    /// Used to communicate the `ViewConfig` in the same process, we don't use
    /// a static variable because prebuild view-process implementations don't
    /// statically link with the same variable.
    const SAME_PROCESS_VAR: &'static str = "zero_ui_view_api::ViewConfig";
    const SG_WAITING: &'static str = "WAITING";
    const SG_DONE: &'static str = "DONE";
}
