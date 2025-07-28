//! Third party licenses service and types.

use zng_app_context::app_local;
pub use zng_tp_licenses::{License, LicenseUsed, User, UserLicense};
use zng_var::{Var, var};

use crate::{
    event::{CommandNameExt as _, command},
    view_process::VIEW_PROCESS,
};

/// Third party licenses.
pub struct LICENSES;

impl LICENSES {
    /// Aggregates all registered third party licenses, grouped by license, sorted by name.
    ///
    /// Exact licenses and users are deduplicated.
    pub fn licenses(&self) -> Vec<LicenseUsed> {
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

        zng_tp_licenses::sort_licenses(&mut r);

        r
    }

    /// Third party licenses provided by the view-process, grouped by license, sorted by name.
    ///
    /// Returns an empty vec if there is no view-process running or the view-process does not provide any license.
    pub fn view_process_licenses(&self) -> Vec<LicenseUsed> {
        let mut r = VIEW_PROCESS.third_party_licenses().unwrap_or_default();
        zng_tp_licenses::sort_licenses(&mut r);
        r
    }

    /// Aggregates all registered third party licenses, by user, sorted by name.
    ///
    /// Exact licenses and users are deduplicated.
    pub fn user_licenses(&self) -> Vec<UserLicense> {
        zng_tp_licenses::user_licenses(&self.licenses())
    }

    /// Third party licenses provided by the view-process, by user, sorted by name.
    ///
    /// Returns an empty vec if there is no view-process running or the view-process does not provide any license.
    pub fn view_process_user_licenses(&self) -> Vec<UserLicense> {
        zng_tp_licenses::user_licenses(&self.view_process_licenses())
    }

    /// If view-process provided third party licenses are included in [`licenses`].
    ///
    /// Note that prebuilt view-process licenses may not be found by license scraper tools.
    ///
    /// This is `true` by default.
    ///
    /// [`licenses`]: Self::licenses
    pub fn include_view_process(&self) -> Var<bool> {
        LICENSES_SV.read().include_view_process.clone()
    }

    /// Register a function that loads some third party licenses used by this app.
    pub fn register(&self, source: fn() -> Vec<LicenseUsed>) {
        LICENSES_SV.write().sources.push(source);
    }
}

app_local! {
    static LICENSES_SV: Licenses = Licenses {
        sources: vec![],
        include_view_process: var(true),
    };
}

struct Licenses {
    sources: Vec<fn() -> Vec<LicenseUsed>>,
    include_view_process: Var<bool>,
}

command! {
    /// Open or focus the third party licenses screen.
    ///
    /// Note that the `zng` crate provides a default implementation for this command, you can override this
    /// default by handling the command in an [`on_pre_event`] handle.
    ///
    /// [`on_pre_event`]: crate::event::Command::on_pre_event
    pub static OPEN_LICENSES_CMD = {
        l10n!: true,
        name: "Third Party Licenses"
    };
}
