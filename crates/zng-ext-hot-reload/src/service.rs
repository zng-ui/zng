use std::sync::Arc;

use libloading::Library;
use zng_app::AppExtension;
use zng_app_context::{app_local, LocalContext};
use zng_ext_fs_watcher::WATCHER;

use crate::{HotNode, HotNodeArgs};

/// Hot reload app extension.
#[derive(Default)]
pub struct HotReloadManager {}
impl AppExtension for HotReloadManager {
    fn init(&mut self) {
        // !!: TODO, load blocking here? need to cargo build
        // !!: TODO, watch all files in crate.
        // !!: TODO, watch all files in crate.

        WATCHER.watch_dir("dir", true).perm();
    }
}

#[allow(non_camel_case_types)]
pub(crate) struct HOT_LIB;

impl HOT_LIB {
    pub(crate) fn instantiate(&self, name: &str, args: HotNodeArgs) -> HotNode {
        let sv = HOT_LIB_SV.read();
        match &sv.lib {
            Some(lib) => match lib.hot_entry(name, LocalContext::capture(), args) {
                Some(n) => n,
                None => {
                    tracing::error!("cannot instantiate `{name:?}`, not found in dyn library");
                    HotNode::nil()
                }
            },
            None => {
                tracing::debug!("cannot instantiate `{name:?}` yet, dyn library not loaded");
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
    hot_entry: unsafe fn(&str, LocalContext, HotNodeArgs) -> Option<HotNode>,
}
impl HotLib {
    pub fn new(lib: impl AsRef<std::ffi::OsStr>) -> Result<Self, libloading::Error> {
        unsafe {
            let lib = Library::new(lib)?;
            Ok(Self {
                hot_entry: *lib.get(b"hot_entry")?,
                lib: Arc::new(lib),
            })
        }
    }

    pub fn hot_entry(&self, name: &str, ctx: LocalContext, args: HotNodeArgs) -> Option<HotNode> {
        // SAFETY: lib is still loaded and will remain until all HotNodes are dropped.
        let mut r = unsafe { (self.hot_entry)(name, ctx, args) };
        if let Some(n) = &mut r {
            n._lib = Some(self.lib.clone());
        }
        r
    }
}
