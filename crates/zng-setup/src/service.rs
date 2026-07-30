use zng::{l10n::l10n, prelude_wgt::*};

use crate::tasks::SetupTaskError;

/// Setup service.
pub struct SETUP;
impl SETUP {
    /// Gets a localized message for a [`SetupTaskError`].
    pub fn l10n_setup_task_error(&self, e: &SetupTaskError) -> Var<Txt> {
        todo!()
    }
}
