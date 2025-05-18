use std::{env, mem, time::Duration};

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use parking_lot::Mutex;
use zng_txt::Txt;

use crate::{VIEW_MODE, VIEW_SERVER, VIEW_VERSION};

/// Configuration for starting a view-process.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ViewConfig {
    /// The [`VERSION`] of the API crate in the app-process.
    ///
    /// [`VERSION`]: crate::VERSION
    pub version: Txt,

    /// Name of the initial channel used in [`connect_view_process`] to setup the connections to the
    /// client app-process.
    ///
    /// [`connect_view_process`]: crate::ipc::connect_view_process
    pub server_name: Txt,

    /// If the server should consider all window requests, headless window requests.
    pub headless: bool,
}
impl ViewConfig {
    /// New config.
    pub fn new(version: impl Into<Txt>, server_name: impl Into<Txt>, headless: bool) -> Self {
        Self {
            version: version.into(),
            server_name: server_name.into(),
            headless,
        }
    }

    /// Reads config from environment variables set by the [`Controller`] in a view-process instance.
    ///
    /// View API implementers should call this to get the config when it suspects that is running as a view-process.
    /// Returns `Some(_)` if the process was initialized as a view-process.
    ///
    /// [`Controller`]: crate::Controller
    pub fn from_env() -> Option<Self> {
        if let (Ok(version), Ok(server_name)) = (env::var(VIEW_VERSION), env::var(VIEW_SERVER)) {
            let headless = env::var(VIEW_MODE).map(|m| m == "headless").unwrap_or(false);
            Some(ViewConfig {
                version: Txt::from_str(&version),
                server_name: Txt::from_str(&server_name),
                headless,
            })
        } else {
            None
        }
    }

    /// Returns `true` if the current process is awaiting for the config to start the
    /// view process in the same process.
    pub(crate) fn is_awaiting_same_process() -> bool {
        matches!(*same_process().lock(), SameProcess::Awaiting)
    }

    /// Sets and unblocks the same-process config if there is a request.
    ///
    /// # Panics
    ///
    /// If there is no pending `wait_same_process`.
    pub(crate) fn set_same_process(cfg: ViewConfig) {
        if Self::is_awaiting_same_process() {
            *same_process().lock() = SameProcess::Ready(cfg);
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

        if !matches!(*same_process().lock(), SameProcess::Not) {
            panic!("`wait_same_process` can only be called once");
        }

        *same_process().lock() = SameProcess::Awaiting;

        let time = Instant::now();
        let timeout = Duration::from_secs(5);
        let sleep = Duration::from_millis(10);

        while Self::is_awaiting_same_process() {
            std::thread::sleep(sleep);
            if time.elapsed() >= timeout {
                panic!("timeout, `wait_same_process` waited for `{timeout:?}`");
            }
        }

        match mem::replace(&mut *same_process().lock(), SameProcess::Done) {
            SameProcess::Ready(cfg) => cfg,
            _ => unreachable!(),
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
                eprintln!("{msg}");
                zng_env::exit(i32::from_le_bytes(*b"vapi"));
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
}

enum SameProcess {
    Not,
    Awaiting,
    Ready(ViewConfig),
    Done,
}

// because some view libs are dynamically loaded this variable needs to be patchable.
//
// This follows the same idea as the "hot-reload" patches, just manually implemented.
static mut SAME_PROCESS: &Mutex<SameProcess> = &SAME_PROCESS_COLD;
static SAME_PROCESS_COLD: Mutex<SameProcess> = Mutex::new(SameProcess::Not);

fn same_process() -> &'static Mutex<SameProcess> {
    // SAFETY: this is safe because SAME_PROCESS is only mutated on dynamic lib init, before any other code.
    unsafe { *std::ptr::addr_of!(SAME_PROCESS) }
}

/// Dynamic view-process "same process" implementations must patch the static variables used by
/// the view-api. This patch also propagates the tracing and log contexts.
pub struct StaticPatch {
    same_process: *const Mutex<SameProcess>,
    tracing: tracing_shared::SharedLogger,
}
impl StaticPatch {
    /// Called in the main executable.
    pub fn capture() -> Self {
        Self {
            same_process: same_process(),
            tracing: tracing_shared::SharedLogger::new(),
        }
    }

    /// Called in the dynamic library.
    ///
    /// # Safety
    ///
    /// Only safe if it is the first view-process code to run in the dynamic library.
    pub unsafe fn install(&self) {
        // SAFETY: safety handled by the caller
        unsafe {
            *std::ptr::addr_of_mut!(SAME_PROCESS) = &*self.same_process;
        }
        self.tracing.install();
    }
}
