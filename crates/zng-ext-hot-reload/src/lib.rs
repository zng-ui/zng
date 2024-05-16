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
mod util;
use std::{collections::HashMap, fmt, sync::Arc};

use cargo::BuildError;
use node::*;

use zng_app::{
    event::{event, event_args},
    AppExtension, DInstant, INSTANT,
};
use zng_app_context::{app_local, LocalContext};
use zng_ext_fs_watcher::WATCHER;
pub use zng_ext_hot_reload_proc_macros::hot_node;
use zng_hot_entry::HotRequest;
use zng_unique_id::hot_reload::HOT_STATICS;
use zng_var::ResponseVar;

#[doc(inline)]
pub use zng_unique_id::{hot_static, hot_static_ref};

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
        pub extern "C" fn zng_hot_entry(request: $crate::zng_hot_entry::HotRequest) -> Option<$crate::zng_hot_entry::HotNode> {
            $crate::zng_hot_entry::entry(request)
        }

        #[no_mangle]
        #[doc(hidden)]
        pub extern "C" fn zng_hot_entry_init(patch: &$crate::StaticPatch) {
            $crate::zng_hot_entry::init(patch)
        }
    };
}

#[doc(hidden)]
pub mod zng_hot_entry {
    pub use crate::node::{HotNode, HotNodeArgs, HotNodeHost};
    use crate::StaticPatch;
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
        pub ctx: LocalContext,
        pub args: HotNodeArgs,
    }

    pub fn entry(mut request: HotRequest) -> Option<crate::HotNode> {
        for entry in HOT_NODES.iter() {
            if request.hot_node_name == entry.hot_node_name && request.manifest_dir == entry.manifest_dir {
                return request.ctx.with_context(|| Some((entry.hot_node_fn)(request.args)));
            }
        }
        None
    }

    pub fn init(patch: &StaticPatch) {
        std::panic::set_hook(Box::new(|args| {
            eprintln!("PANIC IN HOT LOADED LIBRARY, ABORTING");
            crate::util::crash_handler(args);
            std::process::exit(101);
        }));

        // SAFETY: hot reload rebuilds in the same environment, so this is safe if the keys are strong enough.
        unsafe { patch.apply() }
    }
}

#[doc(hidden)]
#[derive(Default, Clone)]
pub struct StaticPatch {
    entries: HashMap<zng_unique_id::hot_reload::PatchKey, unsafe fn(*const ()) -> *const ()>,
}
impl StaticPatch {
    /// Called on the static code (host).
    fn capture() -> Self {
        let mut entries = HashMap::with_capacity(HOT_STATICS.len());
        for (key, val) in HOT_STATICS.iter() {
            match entries.entry(*key) {
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(*val);
                }
                std::collections::hash_map::Entry::Occupied(_) => {
                    panic!("repeated hot static key `{key:?}`");
                }
            }
        }
        Self { entries }
    }

    /// Called on the dynamic code (dylib).
    unsafe fn apply(&self) {
        for (key, patch) in HOT_STATICS.iter() {
            if let Some(val) = self.entries.get(key) {
                patch(val(std::ptr::null()));
            } else {
                eprintln!("did not find `{key:?}` to patch, static reference may fail");
            }
        }
    }
}

/// Hot reload app extension.
#[derive(Default)]
pub struct HotReloadManager {
    libs: HashMap<&'static str, WatchedLib>,
    static_patch: StaticPatch,
}
impl AppExtension for HotReloadManager {
    fn init(&mut self) {
        for entry in crate::zng_hot_entry::HOT_NODES.iter() {
            if let std::collections::hash_map::Entry::Vacant(e) = self.libs.entry(entry.manifest_dir) {
                e.insert(WatchedLib::default());
                WATCHER.watch_dir(entry.manifest_dir, true).perm();
            }
        }

        // !!: TODO, test
        self.static_patch = StaticPatch::capture();
        for (manifest_dir, _) in self.libs.iter() {
            match HotLib::new(
                &self.static_patch,
                manifest_dir,
                "C:/code/zng/target/debug/deps/examples_hot_reload.dll",
            ) {
                Ok(lib) => {
                    HOT.set(lib.clone());
                    HOT_RELOAD_EVENT.notify(HotReloadArgs::now(lib));
                }
                Err(e) => tracing::error!("failed to load rebuilt dyn library, {e}"),
            }
        }
    }

    fn event_preview(&mut self, update: &mut zng_app::update::EventUpdate) {
        if let Some(args) = zng_ext_fs_watcher::FS_CHANGES_EVENT.on(update) {
            for (manifest_dir, watched) in self.libs.iter_mut() {
                if args.changes_for_path(manifest_dir.as_ref()).next().is_some() {
                    if watched.building.is_none() {
                        tracing::info!("rebuilding `{manifest_dir}`");

                        watched.building = Some(BuildingLib {
                            start_time: INSTANT.now(),
                            process: cargo::build(manifest_dir),
                        });
                    } else {
                        // !!: TODO, cancel?
                    }
                }
            }
        }
    }

    fn update_preview(&mut self) {
        for (manifest_dir, watched) in self.libs.iter_mut() {
            if let Some(b) = &watched.building {
                if let Some(r) = b.process.rsp() {
                    match r {
                        Ok(()) => tracing::info!("rebuilt `{manifest_dir}` in {:?}", b.start_time.elapsed()),
                        Err(e) => tracing::error!("failed rebuild `{manifest_dir}`, {e}"),
                    }
                    watched.building = None;
                }
            }
        }
    }
}

#[allow(clippy::upper_case_acronyms)]
pub(crate) struct HOT;
impl HOT {
    pub fn lib(&self, manifest_dir: &'static str) -> Option<HotLib> {
        HOT_SV.read().libs.iter().find(|l| l.manifest_dir() == manifest_dir).cloned()
    }

    fn set(&self, lib: HotLib) {
        let mut sv = HOT_SV.write();
        if let Some(i) = sv.libs.iter().position(|l| l.manifest_dir() == lib.manifest_dir()) {
            sv.libs[i] = lib;
        } else {
            sv.libs.push(lib);
        }
    }
}
app_local! {
    static HOT_SV: HotService = const {HotService { libs: vec![] }};
}
struct HotService {
    libs: Vec<HotLib>,
}

event_args! {
    /// Args for [`HOT_RELOAD_EVENT`].
    pub(crate) struct HotReloadArgs {
        /// Reloaded library.
        pub lib: HotLib,

        ..

        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_widgets();
        }
    }
}

event! {
    /// Event notifies when a new version of a hot reload dynamic library has finished build and is loaded.
    pub static HOT_RELOAD_EVENT: HotReloadArgs;
}

#[derive(Default)]
struct WatchedLib {
    building: Option<BuildingLib>,
}

struct BuildingLib {
    start_time: DInstant,
    process: ResponseVar<Result<(), BuildError>>,
}

/// Dynamically loaded library.
#[derive(Clone)]
pub(crate) struct HotLib {
    manifest_dir: &'static str,
    lib: Arc<libloading::Library>,
    hot_entry: unsafe fn(HotRequest) -> Option<HotNode>,
}
impl fmt::Debug for HotLib {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HotLib")
            .field("manifest_dir", &self.manifest_dir)
            .finish_non_exhaustive()
    }
}
impl HotLib {
    pub fn new(
        static_patch: &StaticPatch,
        manifest_dir: &'static str,
        lib: impl AsRef<std::ffi::OsStr>,
    ) -> Result<Self, libloading::Error> {
        unsafe {
            // SAFETY: assuming the the hot lib was setup as the documented, this works,
            // even the `linkme` stuff does not require any special care.
            //
            // If the hot lib developer add some "ctor/dtor" stuff and that fails they will probably
            // know why, hot reloading should only run in dev machines.
            let lib = libloading::Library::new(lib)?;

            // SAFETY: thats the signature.
            let init: unsafe fn(&StaticPatch) = *lib.get(b"zng_hot_entry_init")?;
            init(static_patch);

            Ok(Self {
                manifest_dir,
                hot_entry: *lib.get(b"zng_hot_entry")?,
                lib: Arc::new(lib),
            })
        }
    }

    /// Lib identifier.
    pub fn manifest_dir(&self) -> &'static str {
        self.manifest_dir
    }

    pub fn instantiate(&self, hot_node_name: &'static str, ctx: LocalContext, args: HotNodeArgs) -> Option<HotNode> {
        let request = HotRequest {
            manifest_dir: self.manifest_dir,
            hot_node_name,
            ctx,
            args,
        };
        // SAFETY: lib is still loaded and will remain until all HotNodes are dropped.
        let mut r = unsafe { (self.hot_entry)(request) };
        if let Some(n) = &mut r {
            n._lib = Some(self.lib.clone());
        }
        r
    }
}
