use zero_ui_app::widget::base::{Parallel, PARALLEL_VAR};

use crate::prelude::*;

/// Defines what node list methods can run in parallel in the widget and descendants.
///
/// This property sets the [`PARALLEL_VAR`] that is used by [`UiNodeList`] implementers to toggle parallel processing.
///
/// See also `WINDOWS.parallel` to define parallelization in multi-window apps.
///
/// [`UiNode`]: zero_ui_app::widget::instance::UiNodeList
#[property(CONTEXT, default(PARALLEL_VAR))]
pub fn parallel(child: impl UiNode, enabled: impl IntoVar<Parallel>) -> impl UiNode {
    with_context_var(child, PARALLEL_VAR, enabled)
}