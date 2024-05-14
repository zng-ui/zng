#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/master/examples/res/image/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/master/examples/res/image/zng-logo.png")]
//!
//! Hot reloading service.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

mod node;
use node::*;
mod service;
use service::*;

/// Declare hot reload entry.
///
/// Must be called at the root of the crate.
#[macro_export]
macro_rules! zng_hot_entry {
    () => {
        #[doc(hidden)] // used by proc-macro
        pub use $crate::zng_hot_entry;

        #[no_mangle]
        #[doc(hidden)] // used by lib loader
        pub fn zng_hot_entry(request: $crate::zng_hot_entry::HotRequest) -> Option<$crate::zng_hot_entry::HotNodeInstance> {
            $crate::zng_hot_entry::entry(request)
        }
    };
}

#[doc(hidden)]
pub mod zng_hot_entry {
    pub use crate::node::{HotNode, HotNodeHost, HotNodeArgs};
    pub use linkme::distributed_slice;
    use zng_app_context::LocalContext;

    pub type HotNodeEntry = (&'static str, fn(HotNodeArgs) -> HotNode);

    #[distributed_slice]
    pub static HOT_NODES: [HotNodeEntry];

    pub struct HotRequest {
        pub name: &'static str,
        ctx: LocalContext,
        args: HotNodeArgs,
    }

    pub fn entry(mut request: HotRequest) -> Option<crate::HotNode> {
        for (name, hot_node_fn) in HOT_NODES.iter() {
            if &request.name == name {
                return request.ctx.with_context(|| Some(hot_node_fn(request.args)));
            }
        }
        None
    }
}

macro_rules! __api_design {
    () => {
        #[hot_node("unique-name (optional)")]
        pub fn my_node(child: impl UiNode, property_input_types: impl IntoVar<bool>, any_cloneable: Arc<AtomicBool>) -> impl UiNode {
            match_node(child, |_, op| {
                // ..
            })
        }

        // expands to:

        #[allow(unexpected_cfgs)]
        #[cfg(zng_hot_build)]
        #[crate::zng_hot_entry::distributed_slice(crate::zng_hot_entry::HOT_NODES)]
        static __HOT_my_node__: crate::zng_hot_entry::HotNodeEntry = (
            "unique-name",
            __hot_my_node__,
        );
        #[allow(unexpected_cfgs)]
        #[cfg(zng_hot_build)]
        fn __hot_my_node__(args: crate::zng_hot_entry::HotNodeArgs) -> crate::zng_hot_entry::HotNode {
           my_node(args.arg_ui_node(0), args.arg_var(1), args.arg_clone(2))
        }

        #[allow(unexpected_cfgs)]
        #[cfg(zng_hot_build)]
        pub fn my_node(child: impl UiNode, property_input_types: impl IntoVar<bool>, any_cloneable: Arc<AtomicBool>) -> impl UiNode {
            match_node(child, |_, op| {
                // ..
            })
        }

        #[allow(unexpected_cfgs)]
        #[cfg(zng_hot_build)] // same function
        pub fn my_node(child: impl UiNode, property_input_types: impl IntoVar<bool>, any_cloneable: Arc<AtomicBool>) -> impl UiNode {
            match_node(child, |_, op| {
                // ..
            })
        }

        #[allow(unexpected_cfgs)]
        #[cfg(not(zng_hot_build))] // proxy function
        pub fn my_node(child: impl UiNode, property_input_types: impl IntoVar<bool>, any_cloneable: Arc<AtomicBool>) -> impl UiNode {
            let mut args__ = crate::zng_hot_entry::HotNodeArgs::with_capacity(3);
            args__.push_ui_node(child);
            args__.push_var(property_input_types);
            args__.push_clone(any_cloneable);

            crate::zng_hot_entry::HotNodeHost::new("unique-name", args__)
        }

    };
}