use std::sync::Arc;

use libloading::Library;
use zng_app_context::{app_local, LocalContext};

use crate::{HotNode, HotNodeArgs};

#[allow(non_camel_case_types)]
pub(crate) struct HOT_LIB;

impl HOT_LIB {
    pub(crate) fn instantiate(&self, manifest_dir: &str, hot_node_name: &str, args: HotNodeArgs) -> Option<HotNode> {
        let sv = HOT_LIB_SV.read();
        match &sv.lib {
            Some(lib) => match lib.hot_entry(manifest_dir, hot_node_name, LocalContext::capture(), args) {
                Some(n) => Some(n),
                None => {
                    tracing::error!("cannot instantiate `{hot_node_name:?}`, not found in dyn library");
                    None
                }
            },
            None => {
                tracing::debug!("cannot instantiate `{hot_node_name:?}` yet, dyn library not loaded");
                None
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
