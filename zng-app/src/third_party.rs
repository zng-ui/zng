//! Third party licenses management.

use zng_app_context::app_local;
pub use zng_tp_licenses::{License, LicenseUser};
use zng_var::{var, ArcVar, Var as _};

use crate::view_process::VIEW_PROCESS;

/// Third party licenses.
pub struct LICENSES;

impl LICENSES {
    /// Aggregates all registered third party licenses.
    ///
    /// Exact licenses and users deduplicated.
    pub fn licenses(&self) -> Vec<License> {
        let mut r = vec![];

        let sv = LICENSES_SV.read();

        for l in sv.sources.iter() {
            let l = l();
            zng_tp_licenses::merge_licenses(&mut r, l);
        }

        if sv.include_view_process.get() {
            let l = self.view_process_licenses();
            zng_tp_licenses::merge_licenses(&mut r, l);
        }

        r
    }

    /// Third party licenses provided by the view-process.
    ///
    /// Returns an empty vec if there is no view-process running or the view-process does not provide any license.
    pub fn view_process_licenses(&self) -> Vec<License> {
        VIEW_PROCESS.third_party_licenses().unwrap_or_default()
    }

    /// If view-process provided third party licenses are included in [`licenses`].
    ///
    /// Note that prebuilt view-process licenses may not be found by license scraper tools.
    ///
    /// This is `true` by default.
    ///
    /// [`licenses`]: Self::licenses
    pub fn include_view_process(&self) -> ArcVar<bool> {
        LICENSES_SV.read().include_view_process.clone()
    }

    /// Register a function that loads some third party licenses used by this app.
    pub fn register_source(&self, source: fn() -> Vec<License>) {
        LICENSES_SV.write().sources.push(source);
    }
}

app_local! {
    static LICENSES_SV: Licenses = Licenses { sources: vec![], include_view_process: var(true), };
}

struct Licenses {
    sources: Vec<fn() -> Vec<License>>,
    include_view_process: ArcVar<bool>,
}
