use zng_ext_setup::{SETUP, SetupStatus};
use zng_wgt::prelude::*;
use zng_wgt_wizard::Page;

/// Setup operation status page.
#[non_exhaustive]
pub struct StatusPage {
    /// The status displayed by the page.
    ///
    /// Is [`SETUP::status`] by default.
    pub status: Var<SetupStatus>,
}
impl Default for StatusPage {
    fn default() -> Self {
        Self { status: SETUP.status() }
    }
}
impl StatusPage {
    /// New default page.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build page.
    pub fn build(self) -> Page {
        todo!()
    }
}
