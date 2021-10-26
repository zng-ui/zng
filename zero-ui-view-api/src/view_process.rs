use std::{env, sync::Arc, time::Duration};

use parking_lot::{Condvar, Mutex};

use crate::{MODE_VAR, SERVER_NAME_VAR};

/// Configuration for starting a view-process.
#[derive(Clone, Debug)]
pub struct ViewConfig {
    /// Name of the initial channel used in [`connect_view_process`] to setup the connections to the
    /// client app-process.
    ///
    /// [`connect_view_process`]: crate::connect_view_process
    pub server_name: String,

    /// If the server should consider all window requests, headless window requests.
    pub headless: bool,
}
impl ViewConfig {
    /// Reads config from environment variables set by the [`Controller`].
    ///
    /// [`Controller`]: crate::Controller
    pub fn from_env() -> Option<Self> {
        if let Ok(server_name) = env::var(SERVER_NAME_VAR) {
            let headless = env::var(MODE_VAR).map(|m| m == "headless").unwrap_or(false);
            Some(ViewConfig { server_name, headless })
        } else {
            None
        }
    }

    /// Returns `true` if the current process is awaiting for the config to start the
    /// view process in the same process.
    pub(crate) fn waiting_same_process() -> bool {
        println!("[2]SAME_PROCESS_CONFIG@0x{:x}", (&SAME_PROCESS_CONFIG) as *const _ as usize);
        SAME_PROCESS_CONFIG.lock().is_some()
    }

    /// Sets and unblocks the same-process config if there is a request.
    ///
    /// # Panics
    ///
    /// If there is no pending `wait_same_process`.
    pub(crate) fn set_same_process(cfg: ViewConfig) {
        if let Some(c) = &mut *SAME_PROCESS_CONFIG.lock() {
            c.cfg = cfg;
            c.waiter.notify_one();
        } else {
            unreachable!("use `waiting_same_process` to check")
        }
    }

    /// Wait for config from same-process.
    pub fn wait_same_process() -> ViewConfig {
        println!("[1]SAME_PROCESS_CONFIG@0x{:x}", (&SAME_PROCESS_CONFIG) as *const _ as usize);
        let mut config = SAME_PROCESS_CONFIG.lock();
        let waiter = Arc::new(Condvar::new());
        *config = Some(SameProcessConfig {
            waiter: waiter.clone(),
            // temporary
            cfg: ViewConfig {
                server_name: String::new(),
                headless: false,
            },
        });

        if cfg!(debug_assertions) {
            waiter.wait(&mut config);
        } else {
            let r = waiter.wait_for(&mut config, Duration::from_secs(10)).timed_out();
            if r {
                panic!("Controller::start was not called in 10 seconds");
            }
        };

        config.take().unwrap().cfg
    }
}

pub(crate) struct SameProcessConfig {
    pub waiter: Arc<Condvar>,
    pub cfg: ViewConfig,
}
pub(crate) static SAME_PROCESS_CONFIG: Mutex<Option<SameProcessConfig>> = parking_lot::const_mutex(None);
