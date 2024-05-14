use std::{collections::HashMap, sync::Arc};

use libloading::Library;
use zng_app::AppExtension;
use zng_app_context::{app_local, LocalContext};
use zng_ext_fs_watcher::WATCHER;

use crate::{HotNode, HotNodeArgs};

/// Hot reload app extension.
#[derive(Default)]
pub struct HotReloadManager {
    libs: HashMap<&'static str, ()>,
}
impl AppExtension for HotReloadManager {
    fn init(&mut self) {
        for (manifest_dir, _, _) in crate::zng_hot_entry::HOT_NODES.iter() {
            if let std::collections::hash_map::Entry::Vacant(e) = self.libs.entry(manifest_dir) {
                e.insert(());
                tracing::info!("watching `{manifest_dir}`");
                WATCHER.watch_dir(manifest_dir, true).perm();
            }
        }
    }
}

#[allow(non_camel_case_types)]
pub(crate) struct HOT_LIB;

impl HOT_LIB {
    pub(crate) fn instantiate(&self, manifest_dir: &str, hot_node_name: &str, args: HotNodeArgs) -> HotNode {
        let sv = HOT_LIB_SV.read();
        match &sv.lib {
            Some(lib) => match lib.hot_entry(manifest_dir, hot_node_name, LocalContext::capture(), args) {
                Some(n) => n,
                None => {
                    tracing::error!("cannot instantiate `{hot_node_name:?}`, not found in dyn library");
                    HotNode::nil()
                }
            },
            None => {
                tracing::debug!("cannot instantiate `{hot_node_name:?}` yet, dyn library not loaded");
                HotNode::nil()
            }
        }
    }
}

app_local! {
    static HOT_LIB_SV: HotLibService = const { HotLibService {
        lib: None
    }};
}

struct HotLibService {
    lib: Option<HotLib>,
}

/// Dynamically loaded library.
struct HotLib {
    lib: Arc<Library>,
    hot_entry: unsafe fn(&str, &str, LocalContext, HotNodeArgs) -> Option<HotNode>,
}
impl HotLib {
    pub fn new(lib: impl AsRef<std::ffi::OsStr>) -> Result<Self, libloading::Error> {
        unsafe {
            let lib = Library::new(lib)?;
            Ok(Self {
                hot_entry: *lib.get(b"zng_hot_entry")?,
                lib: Arc::new(lib),
            })
        }
    }

    pub fn hot_entry(&self, manifest_dir: &str, hot_node_name: &str, ctx: LocalContext, args: HotNodeArgs) -> Option<HotNode> {
        // SAFETY: lib is still loaded and will remain until all HotNodes are dropped.
        let mut r = unsafe { (self.hot_entry)(manifest_dir, hot_node_name, ctx, args) };
        if let Some(n) = &mut r {
            n._lib = Some(self.lib.clone());
        }
        r
    }
}
