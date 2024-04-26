#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/master/examples/res/image/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/master/examples/res/image/zng-logo.png")]
//!
//! Inspector, debug crash handler and debug properties.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zng_wgt::enable_widget_macros!();

use zng_wgt::prelude::*;

pub mod crash_handler;
pub mod debug;

#[cfg(feature = "live")]
mod live;

command! {
    /// Represent the window **inspect** action.
    pub static INSPECT_CMD = {
        name: "Debug Inspector",
        info: "Inspect the window.",
        shortcut: [shortcut!(CTRL|SHIFT+'I'), shortcut!(F12)],
    };
}

/// Setup the inspector for the window.
#[property(WIDGET)]
pub fn inspector(child: impl UiNode, mut inspector: impl UiNode) -> impl UiNode {
    match_node(child, move |c, op| match op {
        UiNodeOp::Measure { wm, desired_size } => {
            *desired_size = c.measure(wm);
            LAYOUT.with_constraints(PxConstraints2d::new_exact_size(*desired_size), || {
                let _ = inspector.measure(wm);
            });
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = c.layout(wl);
            LAYOUT.with_constraints(PxConstraints2d::new_exact_size(*final_size), || {
                let _ = inspector.layout(wl);
            });
        }
        mut op => {
            c.op(op.reborrow());
            inspector.op(op);
        }
    })
}

#[cfg(feature = "live")]
/// Live interactive inspector.
pub fn live_inspector(can_inspect: impl IntoVar<bool>) -> impl UiNode {
    live::inspect_node(can_inspect)
}
