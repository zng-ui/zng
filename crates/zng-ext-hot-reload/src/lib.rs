#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
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
use std::{
    collections::{HashMap, HashSet},
    fmt, io, mem,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

pub use cargo::BuildError;
use node::*;

use zng_app::{
    AppExtension, DInstant, INSTANT,
    event::{event, event_args},
    handler::async_clmv,
    update::UPDATES,
};
use zng_app_context::{LocalContext, app_local};
use zng_ext_fs_watcher::WATCHER;
pub use zng_ext_hot_reload_proc_macros::hot_node;
use zng_task::{SignalOnce, parking_lot::Mutex};
use zng_txt::Txt;
use zng_unique_id::hot_reload::HOT_STATICS;
use zng_unit::TimeUnits as _;
use zng_var::{ResponseVar, Var};

#[doc(inline)]
pub use zng_unique_id::{hot_static, hot_static_ref, lazy_static};

/// Declare hot reload entry.
///
/// Must be called at the root of the crate.
///
/// # Safety
///
/// Must be called only once at the hot-reload crate.
#[macro_export]
macro_rules! zng_hot_entry {
    () => {
        #[doc(hidden)] // used by proc-macro
        pub use $crate::zng_hot_entry;

        #[unsafe(no_mangle)] // SAFETY: docs instruct users to call the macro only once, name is unlikely to have collisions.
        #[doc(hidden)] // used by lib loader
        pub extern "C" fn zng_hot_entry(
            manifest_dir: &&str,
            node_name: &&'static str,
            ctx: &mut $crate::zng_hot_entry::LocalContext,
            exchange: &mut $crate::HotEntryExchange,
        ) {
            $crate::zng_hot_entry::entry(manifest_dir, node_name, ctx, exchange)
        }

        #[unsafe(no_mangle)] // SAFETY: docs instruct users to call the macro only once, name is unlikely to have collisions.
        #[doc(hidden)]
        pub extern "C" fn zng_hot_entry_init(patch: &$crate::StaticPatch) {
            $crate::zng_hot_entry::init(patch)
        }
    };
}

#[doc(hidden)]
pub mod zng_hot_entry {
    pub use crate::node::{HotNode, HotNodeArgs, HotNodeHost};
    use crate::{HotEntryExchange, StaticPatch};
    pub use zng_app_context::LocalContext;

    pub struct HotNodeEntry {
        pub manifest_dir: &'static str,
        pub hot_node_name: &'static str,
        pub hot_node_fn: fn(HotNodeArgs) -> HotNode,
    }

    #[linkme::distributed_slice]
    pub static HOT_NODES: [HotNodeEntry];

    pub fn entry(manifest_dir: &str, node_name: &'static str, ctx: &mut LocalContext, exchange: &mut HotEntryExchange) {
        for entry in HOT_NODES.iter() {
            if node_name == entry.hot_node_name && manifest_dir == entry.manifest_dir {
                let args = match std::mem::replace(exchange, HotEntryExchange::Responding) {
                    HotEntryExchange::Request(args) => args,
                    _ => panic!("bad request"),
                };
                let node = ctx.with_context(|| (entry.hot_node_fn)(args));
                *exchange = HotEntryExchange::Response(Some(node));
                return;
            }
        }
        *exchange = HotEntryExchange::Response(None);
    }

    pub fn init(statics: &StaticPatch) {
        std::panic::set_hook(Box::new(|args| {
            eprintln!("PANIC IN HOT LOADED LIBRARY, ABORTING");
            crate::util::crash_handler(args);
            zng_env::exit(101);
        }));

        // SAFETY: hot reload rebuilds in the same environment, so this is safe if the keys are strong enough.
        unsafe { statics.apply() }
    }
}

type StaticPatchersMap = HashMap<&'static dyn zng_unique_id::hot_reload::PatchKey, unsafe fn(*const ()) -> *const ()>;

#[doc(hidden)]
#[derive(Clone)]
#[repr(C)]
pub struct StaticPatch {
    tracing: tracing_shared::SharedLogger,
    entries: Arc<StaticPatchersMap>,
}
impl StaticPatch {
    /// Called on the static code (host).
    pub fn capture() -> Self {
        let mut entries = StaticPatchersMap::with_capacity(HOT_STATICS.len());
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

        Self {
            entries: Arc::new(entries),
            tracing: tracing_shared::SharedLogger::new(),
        }
    }

    /// Called on the dynamic code (dylib).
    unsafe fn apply(&self) {
        self.tracing.install();

        for (key, patch) in HOT_STATICS.iter() {
            if let Some(val) = self.entries.get(key) {
                // println!("patched `{key:?}`");
                // SAFETY: HOT_STATICS is defined using linkme, so all entries are defined by the hot_static! macro
                unsafe {
                    patch(val(std::ptr::null()));
                }
            } else {
                eprintln!("did not find `{key:?}` to patch, static references may fail");
            }
        }
    }
}

/// Status of a monitored dynamic library crate.
#[derive(Clone, PartialEq, Debug)]
#[non_exhaustive]
pub struct HotStatus {
    /// Dynamic library crate directory.
    ///
    /// Any file changes inside this directory triggers a rebuild.
    pub manifest_dir: Txt,

    /// Build start time if is rebuilding.
    pub building: Option<DInstant>,

    /// Last rebuild and reload result.
    ///
    /// is `Ok(build_duration)` or `Err(build_error)`.
    pub last_build: Result<Duration, BuildError>,

    /// Number of times the dynamically library was rebuilt (successfully or with error).
    pub rebuild_count: usize,
}
impl HotStatus {
    /// Gets the build time if the last build succeeded.
    pub fn ok(&self) -> Option<Duration> {
        self.last_build.as_ref().ok().copied()
    }

    /// If the last build was cancelled.
    pub fn is_cancelled(&self) -> bool {
        matches!(&self.last_build, Err(BuildError::Cancelled))
    }

    /// Gets the last build error if it failed and was not cancelled.
    pub fn err(&self) -> Option<&BuildError> {
        self.last_build.as_ref().err().filter(|e| !matches!(e, BuildError::Cancelled))
    }
}

/// Hot reload app extension.
///
/// # Events
///
/// Events this extension provides.
///
/// * [`HOT_RELOAD_EVENT`]
///
/// # Services
///
/// Services this extension provides.
///
/// * [`HOT_RELOAD`]
#[derive(Default)]
pub struct HotReloadManager {
    libs: HashMap<&'static str, WatchedLib>,
    static_patch: Option<StaticPatch>,
}
impl AppExtension for HotReloadManager {
    fn init(&mut self) {
        // watch all hot libraries.
        let mut status = vec![];
        for entry in crate::zng_hot_entry::HOT_NODES.iter() {
            if let std::collections::hash_map::Entry::Vacant(e) = self.libs.entry(entry.manifest_dir) {
                e.insert(WatchedLib::default());
                WATCHER.watch_dir(entry.manifest_dir, true).perm();

                status.push(HotStatus {
                    manifest_dir: entry.manifest_dir.into(),
                    building: None,
                    last_build: Ok(Duration::MAX),
                    rebuild_count: 0,
                });
            }
        }
        HOT_RELOAD_SV.read().status.set(status);
    }

    fn event_preview(&mut self, update: &mut zng_app::update::EventUpdate) {
        if let Some(args) = zng_ext_fs_watcher::FS_CHANGES_EVENT.on(update) {
            for (manifest_dir, watched) in self.libs.iter_mut() {
                if args.changes_for_path(manifest_dir.as_ref()).next().is_some() {
                    watched.rebuild((*manifest_dir).into(), self.static_patch.get_or_insert_with(StaticPatch::capture));
                }
            }
        }
    }

    fn update_preview(&mut self) {
        for (manifest_dir, watched) in self.libs.iter_mut() {
            if let Some(b) = &watched.building
                && let Some(r) = b.rebuild_load.rsp()
            {
                let build_time = b.start_time.elapsed();
                let mut lib = None;
                let status_r = match r {
                    Ok(l) => {
                        lib = Some(l);
                        Ok(build_time)
                    }
                    Err(e) => {
                        if matches!(&e, BuildError::Cancelled) {
                            tracing::warn!("cancelled rebuild `{manifest_dir}`");
                        } else {
                            tracing::error!("failed rebuild `{manifest_dir}`, {e}");
                        }
                        Err(e)
                    }
                };
                if let Some(lib) = lib {
                    tracing::info!("rebuilt and reloaded `{manifest_dir}` in {build_time:?}");
                    HOT_RELOAD.set(lib.clone());
                    HOT_RELOAD_EVENT.notify(HotReloadArgs::now(lib));
                }

                watched.building = None;

                let manifest_dir = *manifest_dir;
                HOT_RELOAD_SV.read().status.modify(move |s| {
                    let s = s.iter_mut().find(|s| s.manifest_dir == manifest_dir).unwrap();
                    s.building = None;
                    s.last_build = status_r;
                    s.rebuild_count += 1;
                });

                if mem::take(&mut watched.rebuild_again) {
                    HOT_RELOAD_SV.write().rebuild_requests.push(manifest_dir.into());
                }
            }
        }

        let mut sv = HOT_RELOAD_SV.write();
        let requests: HashSet<Txt> = sv.cancel_requests.drain(..).collect();
        for r in requests {
            if let Some(watched) = self.libs.get_mut(r.as_str())
                && let Some(b) = &watched.building
            {
                b.cancel_build.set();
            }
        }

        let requests: HashSet<Txt> = sv.rebuild_requests.drain(..).collect();
        drop(sv);
        for r in requests {
            if let Some(watched) = self.libs.get_mut(r.as_str()) {
                watched.rebuild(r, self.static_patch.get_or_insert_with(StaticPatch::capture));
            } else {
                tracing::error!("cannot rebuild `{r}`, unknown");
            }
        }
    }
}

type RebuildVar = ResponseVar<Result<PathBuf, BuildError>>;

type RebuildLoadVar = ResponseVar<Result<HotLib, BuildError>>;

/// Arguments for custom rebuild runners.
///
/// See [`HOT_RELOAD.rebuilder`] for more details.
///
/// [`HOT_RELOAD.rebuilder`]: HOT_RELOAD::rebuilder
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct BuildArgs {
    /// Crate that changed.
    pub manifest_dir: Txt,
    /// Cancel signal.
    ///
    /// If the build cannot be cancelled or has already finished this signal must be ignored and
    /// the normal result returned.
    pub cancel_build: SignalOnce,
}
impl BuildArgs {
    /// Calls `cargo build [--package {package}] --message-format json` and cancels it as soon as the dylib is rebuilt.
    ///
    /// Always returns `Some(_)`.
    pub fn build(&self, package: Option<&str>) -> Option<RebuildVar> {
        Some(cargo::build(
            &self.manifest_dir,
            "--package",
            package.unwrap_or(""),
            "",
            "",
            self.cancel_build.clone(),
        ))
    }

    /// Calls `cargo build [--package {package}] --example {example} --message-format json` and cancels
    /// it as soon as the dylib is rebuilt.
    ///
    /// Always returns `Some(_)`.
    pub fn build_example(&self, package: Option<&str>, example: &str) -> Option<RebuildVar> {
        Some(cargo::build(
            &self.manifest_dir,
            "--package",
            package.unwrap_or(""),
            "--example",
            example,
            self.cancel_build.clone(),
        ))
    }

    /// Calls `cargo build [--package {package}] --bin {bin}  --message-format json` and cancels it as
    /// soon as the dylib is rebuilt.
    ///
    /// Always returns `Some(_)`.
    pub fn build_bin(&self, package: Option<&str>, bin: &str) -> Option<RebuildVar> {
        Some(cargo::build(
            &self.manifest_dir,
            "--package",
            package.unwrap_or(""),
            "--bin",
            bin,
            self.cancel_build.clone(),
        ))
    }

    /// Calls `cargo build --manifest-path {path} --message-format json` and cancels it as soon as the dylib is rebuilt.
    ///
    /// Always returns `Some(_)`.
    pub fn build_manifest(&self, path: &str) -> Option<RebuildVar> {
        Some(cargo::build(
            &self.manifest_dir,
            "--manifest-path",
            path,
            "",
            "",
            self.cancel_build.clone(),
        ))
    }

    /// Calls a custom command that must write to stdout the same way `cargo build --message-format json` does.
    ///
    /// The command will run until it writes the `"compiler-artifact"` for the `manifest_dir/Cargo.toml` to stdout, it will
    /// then be killed.
    ///
    /// Always returns `Some(_)`.
    pub fn custom(&self, cmd: std::process::Command) -> Option<RebuildVar> {
        Some(cargo::build_custom(&self.manifest_dir, cmd, self.cancel_build.clone()))
    }

    /// Call a custom command defined in an environment var.
    ///
    /// The variable value must be arguments for `cargo`, that is `cargo $VAR`.
    ///
    /// See [`custom`] for other requirements of the command.
    ///
    /// If `var_key` is empty the default key `"ZNG_HOT_RELOAD_REBUILDER"` is used.
    ///
    /// Returns `None` if the var is not found or is set empty.
    ///
    /// [`custom`]: Self::custom
    pub fn custom_env(&self, mut var_key: &str) -> Option<RebuildVar> {
        if var_key.is_empty() {
            var_key = "ZNG_HOT_RELOAD_REBUILDER";
        }

        let custom = std::env::var(var_key).ok()?;
        let mut custom = custom.split(' ');

        let subcommand = custom.next()?;

        let mut cmd = std::process::Command::new("cargo");
        cmd.arg(subcommand);
        cmd.args(custom);

        self.custom(cmd)
    }

    /// The default action.
    ///
    /// Tries `custom_env`, if env is not set, does `build(None)`.
    ///
    /// Always returns `Some(_)`.
    pub fn default_build(&self) -> Option<RebuildVar> {
        self.custom_env("").or_else(|| self.build(None))
    }
}

/// Hot reload service.
#[expect(non_camel_case_types)]
pub struct HOT_RELOAD;
impl HOT_RELOAD {
    /// Hot reload status, libs that are rebuilding, errors.
    pub fn status(&self) -> Var<Vec<HotStatus>> {
        HOT_RELOAD_SV.read().status.read_only()
    }

    /// Register a handler that can override the hot library rebuild.
    ///
    /// The command should rebuild using the same features used to run the program (not just rebuild the dylib).
    /// By default it is just `cargo build`, that works if the program was started using only `cargo run`, but
    /// an example program needs a custom runner.
    ///
    /// If `rebuilder` wants to handle the rebuild it must return a response var that updates when the rebuild is finished with
    /// the path to the rebuilt dylib. The [`BuildArgs`] also provides helper methods to rebuild common workspace setups.
    ///
    /// Note that unlike most services the `rebuilder` is registered immediately, not after an update cycle.
    pub fn rebuilder(&self, rebuilder: impl FnMut(BuildArgs) -> Option<RebuildVar> + Send + 'static) {
        HOT_RELOAD_SV.write().rebuilders.get_mut().push(Box::new(rebuilder));
    }

    /// Request a rebuild, if `manifest_dir` is a hot library.
    ///
    /// Note that changes inside the directory already trigger a rebuild automatically.
    pub fn rebuild(&self, manifest_dir: impl Into<Txt>) {
        HOT_RELOAD_SV.write().rebuild_requests.push(manifest_dir.into());
        UPDATES.update(None);
    }

    /// Request a rebuild cancel for the current building `manifest_dir`.
    pub fn cancel(&self, manifest_dir: impl Into<Txt>) {
        HOT_RELOAD_SV.write().cancel_requests.push(manifest_dir.into());
        UPDATES.update(None);
    }

    pub(crate) fn lib(&self, manifest_dir: &'static str) -> Option<HotLib> {
        HOT_RELOAD_SV
            .read()
            .libs
            .iter()
            .rev()
            .find(|l| l.manifest_dir() == manifest_dir)
            .cloned()
    }

    fn set(&self, lib: HotLib) {
        // we never unload HotLib because hot nodes can pass &'static references (usually inside `Txt`) to the
        // program that will remain being used after.
        HOT_RELOAD_SV.write().libs.push(lib);
    }
}
app_local! {
    static HOT_RELOAD_SV: HotReloadService = {
        HotReloadService {
            libs: vec![],
            rebuilders: Mutex::new(vec![]),
            status: zng_var::var(vec![]),
            rebuild_requests: vec![],
            cancel_requests: vec![],
        }
    };
}
struct HotReloadService {
    libs: Vec<HotLib>,
    // mutex for Sync only
    #[expect(clippy::type_complexity)]
    rebuilders: Mutex<Vec<Box<dyn FnMut(BuildArgs) -> Option<RebuildVar> + Send + 'static>>>,

    status: Var<Vec<HotStatus>>,
    rebuild_requests: Vec<Txt>,
    cancel_requests: Vec<Txt>,
}
impl HotReloadService {
    fn rebuild_reload(&mut self, manifest_dir: Txt, static_patch: &StaticPatch) -> (RebuildLoadVar, SignalOnce) {
        let (rebuild, cancel) = self.rebuild(manifest_dir.clone());
        let rebuild_load = zng_task::respond(async_clmv!(static_patch, {
            let build_path = rebuild.wait_rsp().await?;

            // copy dylib to not block the next rebuild
            let file_name = match build_path.file_name() {
                Some(f) => f.to_string_lossy(),
                None => return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "dylib path does not have a file name").into()),
            };

            // cleanup previous session
            for p in glob::glob(&format!("{}/zng-hot-{file_name}-*", build_path.parent().unwrap().display()))
                .unwrap()
                .flatten()
            {
                let _ = std::fs::remove_file(p);
            }

            let mut unique_path = build_path.clone();
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis();
            unique_path.set_file_name(format!("zng-hot-{file_name}-{ts:x}"));
            std::fs::copy(&build_path, &unique_path)?;

            let dylib = zng_task::wait(move || HotLib::new(&static_patch, manifest_dir, unique_path));
            match zng_task::with_deadline(dylib, 10.secs()).await {
                Ok(r) => r.map_err(Into::into),
                Err(_) => Err(BuildError::Io(Arc::new(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "hot dylib did not init after 10s",
                )))),
            }
        }));
        (rebuild_load, cancel)
    }

    fn rebuild(&mut self, manifest_dir: Txt) -> (RebuildVar, SignalOnce) {
        for r in self.rebuilders.get_mut() {
            let cancel = SignalOnce::new();
            let args = BuildArgs {
                manifest_dir: manifest_dir.clone(),
                cancel_build: cancel.clone(),
            };
            if let Some(r) = r(args.clone()) {
                return (r, cancel);
            }
        }
        let cancel = SignalOnce::new();
        let args = BuildArgs {
            manifest_dir: manifest_dir.clone(),
            cancel_build: cancel.clone(),
        };
        (args.default_build().unwrap(), cancel)
    }
}

event_args! {
    /// Args for [`HOT_RELOAD_EVENT`].
    pub struct HotReloadArgs {
        /// Reloaded library.
        pub(crate) lib: HotLib,

        ..

        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all();
        }
    }
}
impl HotReloadArgs {
    /// Crate directory that changed and caused the rebuild.
    pub fn manifest_dir(&self) -> &Txt {
        self.lib.manifest_dir()
    }
}

event! {
    /// Event notifies when a new version of a hot reload dynamic library has finished rebuild and has loaded.
    ///
    /// This event is used internally by hot nodes to reinit.
    pub static HOT_RELOAD_EVENT: HotReloadArgs;
}

#[derive(Default)]
struct WatchedLib {
    building: Option<BuildingLib>,
    rebuild_again: bool,
}
impl WatchedLib {
    fn rebuild(&mut self, manifest_dir: Txt, static_path: &StaticPatch) {
        if let Some(b) = &self.building {
            if b.start_time.elapsed() > WATCHER.debounce().get() + 34.ms() {
                // WATCHER debounce notifies immediately, then debounces. Some
                // IDEs (VsCode) touch the saving file multiple times within
                // the debounce interval, this causes two rebuild requests.
                //
                // So we only cancel rebuild if the second event (current) is not
                // within debounce + a generous 34ms for the notification delay.
                b.cancel_build.set();
                self.rebuild_again = true;
            }
        } else {
            let start_time = INSTANT.now();
            tracing::info!("rebuilding `{manifest_dir}`");

            let mut sv = HOT_RELOAD_SV.write();

            let (rebuild_load, cancel_build) = sv.rebuild_reload(manifest_dir.clone(), static_path);
            self.building = Some(BuildingLib {
                start_time,
                rebuild_load,
                cancel_build,
            });

            sv.status.modify(move |s| {
                s.iter_mut().find(|s| s.manifest_dir == manifest_dir).unwrap().building = Some(start_time);
            });
        }
    }
}

struct BuildingLib {
    start_time: DInstant,
    rebuild_load: RebuildLoadVar,
    cancel_build: SignalOnce,
}

#[doc(hidden)]
pub enum HotEntryExchange {
    Request(HotNodeArgs),
    Responding,
    Response(Option<HotNode>),
}

/// Dynamically loaded library.
#[derive(Clone)]
pub(crate) struct HotLib {
    manifest_dir: Txt,
    lib: Arc<libloading::Library>,
    hot_entry: unsafe extern "C" fn(&&str, &&'static str, &mut LocalContext, &mut HotEntryExchange),
}
impl PartialEq for HotLib {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.lib, &other.lib)
    }
}
impl fmt::Debug for HotLib {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HotLib")
            .field("manifest_dir", &self.manifest_dir)
            .finish_non_exhaustive()
    }
}
impl HotLib {
    pub fn new(patch: &StaticPatch, manifest_dir: Txt, lib: impl AsRef<std::ffi::OsStr>) -> Result<Self, libloading::Error> {
        unsafe {
            // SAFETY: assuming the hot lib was setup as the docs instruct, this works,
            // even the `linkme` stuff does not require any special care.
            //
            // If the hot lib developer add some "ctor/dtor" stuff and that fails they will probably
            // know why, hot reloading should only run in dev machines.
            let lib = libloading::Library::new(lib)?;

            // SAFETY: thats the signature.
            let init: unsafe extern "C" fn(&StaticPatch) = *lib.get(b"zng_hot_entry_init")?;
            init(patch);

            Ok(Self {
                manifest_dir,
                hot_entry: *lib.get(b"zng_hot_entry")?,
                lib: Arc::new(lib),
            })
        }
    }

    /// Lib identifier.
    pub fn manifest_dir(&self) -> &Txt {
        &self.manifest_dir
    }

    pub fn instantiate(&self, hot_node_name: &'static str, ctx: &mut LocalContext, args: HotNodeArgs) -> Option<HotNode> {
        let mut exchange = HotEntryExchange::Request(args);
        // SAFETY: lib is still loaded and will remain until all HotNodes are dropped.
        unsafe { (self.hot_entry)(&self.manifest_dir.as_str(), &hot_node_name, ctx, &mut exchange) };
        let mut node = match exchange {
            HotEntryExchange::Response(n) => n,
            _ => None,
        };
        if let Some(n) = &mut node {
            n._lib = Some(self.lib.clone());
        }
        node
    }
}
