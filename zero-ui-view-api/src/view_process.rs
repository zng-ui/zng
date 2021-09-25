use std::env;

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

    /// Reads config from same-process thread-local.
    pub fn from_thread() -> Option<Self> {
        todo!()
    }
}
