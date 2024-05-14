#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/master/examples/res/image/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/master/examples/res/image/zng-logo.png")]
//!
//! Hot reload service.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

mod cargo;
mod node;
mod service;
use std::collections::HashMap;

use node::*;
use service::*;

use zng_app::AppExtension;
use zng_ext_fs_watcher::WATCHER;
pub use zng_ext_hot_reload_proc_macros::hot_node;

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
        pub fn zng_hot_entry(request: $crate::zng_hot_entry::HotRequest) -> Option<$crate::zng_hot_entry::HotNode> {
            $crate::zng_hot_entry::entry(request)
        }
    };
}

#[doc(hidden)]
pub mod zng_hot_entry {
    pub use crate::node::{HotNode, HotNodeArgs, HotNodeHost};
    use zng_app_context::LocalContext;

    pub struct HotNodeEntry {
        pub manifest_dir: &'static str,
        pub hot_node_name: &'static str,
        pub hot_node_fn: fn(HotNodeArgs) -> HotNode,
    }

    #[linkme::distributed_slice]
    pub static HOT_NODES: [HotNodeEntry];

    pub struct HotRequest {
        pub manifest_dir: &'static str,
        pub hot_node_name: &'static str,
        ctx: LocalContext,
        args: HotNodeArgs,
    }

    pub fn entry(mut request: HotRequest) -> Option<crate::HotNode> {
        for entry in HOT_NODES.iter() {
            if request.hot_node_name == entry.hot_node_name && request.manifest_dir == entry.manifest_dir {
                return request.ctx.with_context(|| Some((entry.hot_node_fn)(request.args)));
            }
        }
        None
    }
}

/// Hot reload app extension.
#[derive(Default)]
pub struct HotReloadManager {
    libs: HashMap<&'static str, ()>,
}
impl AppExtension for HotReloadManager {
    fn init(&mut self) {
        for entry in crate::zng_hot_entry::HOT_NODES.iter() {
            if let std::collections::hash_map::Entry::Vacant(e) = self.libs.entry(entry.manifest_dir) {
                e.insert(());
                tracing::info!("watching `{}`", entry.manifest_dir);
                WATCHER.watch_dir(entry.manifest_dir, true).perm();
            }
        }
    }
}
