//! Commands that control toggle.

use parking_lot::Mutex;
use std::{fmt, sync::Arc};
use zng_var::BoxedVarValueAny;

use zng_wgt::prelude::*;

use super::SELECTOR;

command! {
    /// Represents the **toggle** action.
    ///
    /// # Handlers
    ///
    /// * [`checked`]: The property toggles for no param and or sets to the `bool` param or to
    /// the `Option<bool>` param coercing `None` to `false`.
    ///
    /// * [`checked_opt`]: The property cycles or toggles depending on [`tristate`] for no params, otherwise
    /// it sets the `bool` or `Option<bool>` param.
    ///
    /// * [`value`]: The property toggles select/unselect the value for no params, otherwise it selects the value
    /// for param `true` or `Some(true)` and deselects the value for param `false` or `None::<bool>`. Note that you
    /// can also use the [`SELECT_CMD`] for value.
    ///
    /// [`checked`]: fn@super::checked
    /// [`checked_opt`]: fn@super::checked_opt
    /// [`tristate`]: fn@super::tristate
    /// [`value`]: fn@super::value
    pub static TOGGLE_CMD;

    /// Represents the **select** action.
    ///
    /// # Handlers
    ///
    /// * [`value`]: The property selects the value if the command has no param.
    ///
    /// * [`selector`]: The property applies the [`SelectOp`] param.
    ///
    /// [`value`]: fn@super::value
    /// [`selector`]: fn@super::selector
    pub static SELECT_CMD;
}

/// Represents a select operation that can be send to [`selector`] using [`SELECT_CMD`].
///
/// [`selector`]: fn@super::selector
#[derive(Clone)]
pub struct SelectOp {
    op: Arc<Mutex<dyn FnMut() + Send>>,
}
impl fmt::Debug for SelectOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SelectOp").finish_non_exhaustive()
    }
}
impl SelectOp {
    /// New (de)select operation.
    ///
    /// The [`selector`] property handles [`SELECT_CMD`] by calling `op` during event handling.
    /// You can use [`SELECTOR`] to get and set the selection.
    ///
    /// [`selector`]: fn@super::selector
    pub fn new(op: impl FnMut() + Send + 'static) -> Self {
        Self {
            op: Arc::new(Mutex::new(op)),
        }
    }

    /// Select the `value`.
    pub fn select(value: BoxedVarValueAny) -> Self {
        let mut value = Some(value);
        Self::new(move || {
            if let Some(value) = value.take() {
                if let Err(e) = SELECTOR.get().select(value) {
                    tracing::error!("select error: {e}");
                }
            }
        })
    }

    /// Deselect the `value`.
    pub fn deselect(value: BoxedVarValueAny) -> Self {
        Self::new(move || {
            if let Err(e) = SELECTOR.get().deselect(&*value) {
                tracing::error!("deselect error: {e}");
            }
        })
    }

    /// Run the operation.
    pub fn call(&self) {
        (self.op.lock())()
    }
}
