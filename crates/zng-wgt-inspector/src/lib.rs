#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Inspector, debug crash handler and debug properties.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zng_wgt::enable_widget_macros!();

use zng_wgt::{ICONS, prelude::*};

pub mod crash_handler;
pub mod debug;

mod live;

#[cfg(feature = "live")]
pub use crate::live::data_model::{INSPECTOR, InspectedInfo, InspectedTree, InspectedWidget, InspectorWatcherBuilder, WeakInspectedTree};

command! {
    /// Represent the window **inspect** action.
    pub static INSPECT_CMD = {
        l10n!: "inspector",
        name: "Debug Inspector",
        info: "Inspect the window",
        shortcut: [shortcut!(CTRL | SHIFT + 'I'), shortcut!(F12)],
        icon: wgt_fn!(|_| ICONS.get(["inspector", "screen-search-desktop"])),
    };
}

/// Setup the inspector for the window.
#[property(WIDGET)]
pub fn inspector(child: impl IntoUiNode, inspector: impl IntoUiNode) -> UiNode {
    match_node(ui_vec![child, inspector], move |c, op| match op {
        UiNodeOp::Measure { wm, desired_size } => {
            c.delegated();
            let children = c.node_impl::<UiVec>();
            *desired_size = children[0].measure(wm);
            LAYOUT.with_constraints(PxConstraints2d::new_exact_size(*desired_size), || {
                let _ = children[1].measure(wm);
            });
        }
        UiNodeOp::Layout { wl, final_size } => {
            c.delegated();
            let children = c.node_impl::<UiVec>();
            *final_size = children[0].layout(wl);
            LAYOUT.with_constraints(PxConstraints2d::new_exact_size(*final_size), || {
                let _ = children[1].layout(wl);
            });
        }
        _ => {}
    })
}

#[cfg(feature = "live")]
/// Live interactive inspector.
///
/// Can be set on a window using the [`inspector`](fn@inspector) property.
/// Note that the main `APP.defaults()` already sets this for all windows when
/// the `"inspector"` feature is enabled.
pub fn live_inspector(can_inspect: impl IntoVar<bool>) -> UiNode {
    live::inspect_node(can_inspect)
}
