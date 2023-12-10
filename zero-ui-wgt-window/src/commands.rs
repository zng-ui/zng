//! Commands that control the scoped window.

use zero_ui_wgt::prelude::*;

command! {
    /// Represent the window **inspect** action.
    pub static INSPECT_CMD = {
        name: "Debug Inspector",
        info: "Inspect the current window.",
        shortcut: [shortcut!(CTRL|SHIFT+'I'), shortcut!(F12)],
    };
}

#[cfg(inspector)]
pub(super) fn inspect_node(
    child: impl zero_ui_app::widget::instance::UiNode,
    can_inspect: impl crate::core::var::IntoVar<bool>,
) -> impl zero_ui_app::widget::instance::UiNode {
    // !!: TODO, use zero-ui-wgt-inspector
    live_inspector::inspect_node(child, can_inspect)
    // prompt_inspector::inspect_node(child, can_inspect)
}
