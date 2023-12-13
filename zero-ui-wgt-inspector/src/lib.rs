//! Debug properties and inspector implementation.

#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zero_ui_wgt::enable_widget_macros!();

use zero_ui_wgt::prelude::*;

pub mod debug;

#[cfg(feature = "live")]
mod live;

command! {
    /// Represent the window **inspect** action.
    pub static INSPECT_CMD = {
        name: "Debug Inspector",
        info: "Inspect the current window.",
        shortcut: [shortcut!(CTRL|SHIFT+'I'), shortcut!(F12)],
    };
}

/// Setup the inspector for the window.
#[property(WIDGET)]
pub fn inspector(child: impl UiNode, mut inspector: impl UiNode) -> impl UiNode {
    match_node(child, move |c, mut op| {
        c.op(op.reborrow());
        inspector.op(op);
    })
}

#[cfg(feature = "live")]
/// Live interactive inspector.
pub fn live_inspector(can_inspect: impl IntoVar<bool>) -> impl UiNode {
    live::inspect_node(can_inspect)
}
