use zng_app::widget::base::{PARALLEL_VAR, Parallel};

use crate::prelude::*;

/// Defines what node list methods can run in parallel in the widget and descendants.
///
/// This property sets the [`PARALLEL_VAR`] that is used by list node implementers to toggle parallel processing.
///
/// Note that this property is set at the `WIDGET` nest group, so it will affect all parallelization in the widget, even in
/// list at the root of the widget.
///
/// See also `WINDOWS.parallel` to define parallelization between multiple windows.
///
/// [`PARALLEL_VAR`]: zng_app::widget::base::PARALLEL_VAR
#[property(WIDGET, default(PARALLEL_VAR))]
pub fn parallel(child: impl IntoUiNode, enabled: impl IntoVar<Parallel>) -> UiNode {
    with_context_var(child, PARALLEL_VAR, enabled)
}
