//! Commands that control toggle.

use crate::core::event::*;

command! {
    /// Represents the **toggle** action.
    ///
    /// The parameter can be a `bool` or `Option<bool>` value to set, otherwise the
    /// command cycle through the options.
    pub static TOGGLE_CMD;
}
