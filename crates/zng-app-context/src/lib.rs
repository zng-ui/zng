#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//! App execution context.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::{any::Any, cell::RefCell, fmt, mem, sync::Arc, thread::LocalKey};

mod util;
pub use util::*;

mod app_local;
pub use app_local::*;

mod context_local;
pub use context_local::*;

use parking_lot::*;
use zng_txt::Txt;
use zng_unique_id::unique_id_32;

#[doc(hidden)]
pub use zng_unique_id::{hot_static, hot_static_ref};

unique_id_32! {
    /// Identifies an app instance.
    pub struct AppId;
}
zng_unique_id::impl_unique_id_name!(AppId);
zng_unique_id::impl_unique_id_fmt!(AppId);
zng_unique_id::impl_unique_id_bytemuck!(AppId);

impl serde::Serialize for AppId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let name = self.name();
        if name.is_empty() {
            use serde::ser::Error;
            return Err(S::Error::custom("cannot serialize unnamed `AppId`"));
        }
        name.serialize(serializer)
    }
}
impl<'de> serde::Deserialize<'de> for AppId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name = Txt::deserialize(deserializer)?;
        Ok(AppId::named(name))
    }
}

#[derive(Clone, Copy)]
enum LocalValueKind {
    Local,
    Var,
    App,
}
impl LocalValueKind {
    /// Include in local captures.
    fn include_local(self) -> bool {
        !matches!(self, Self::Var)
    }

    /// Include in var captures.
    fn include_var(self) -> bool {
        !matches!(self, Self::Local)
    }
}

/// `(value, is_context_var)`
type LocalValue = (Arc<dyn Any + Send + Sync>, LocalValueKind);
// equivalent to rustc_hash::FxHashMap, but can be constructed in `const`.
type LocalData = std::collections::HashMap<AppLocalId, LocalValue, BuildFxHasher>;
#[derive(Clone, Default)]
struct BuildFxHasher;
impl std::hash::BuildHasher for BuildFxHasher {
    type Hasher = rustc_hash::FxHasher;

    fn build_hasher(&self) -> Self::Hasher {
        rustc_hash::FxHasher::default()
    }
}
const fn new_local_data() -> LocalData {
    LocalData::with_hasher(BuildFxHasher)
}

type LocalSet = std::collections::HashSet<AppLocalId, BuildFxHasher>;
const fn new_local_set() -> LocalSet {
    LocalSet::with_hasher(BuildFxHasher)
}

/// Represents an app lifetime, ends the app on drop.
///
/// You can use [`LocalContext::start_app`] to manually create an app scope without actually running an app.
#[must_use = "ends the app scope on drop"]
pub struct AppScope {
    id: AppId,
    _same_thread: std::rc::Rc<()>,
}
impl Drop for AppScope {
    fn drop(&mut self) {
        LocalContext::end_app(self.id);
    }
}

impl AppId {
    fn local_id() -> AppLocalId {
        hot_static! {
            static ID: u8 = 0;
        }
        AppLocalId(hot_static_ref!(ID) as *const u8 as *const () as _)
    }
}
fn cleanup_list_id() -> AppLocalId {
    hot_static! {
        static ID: u8 = 0;
    }
    AppLocalId(hot_static_ref!(ID) as *const u8 as *const () as _)
}

/// Tracks the current execution context.
///
/// The context tracks the current app, all or some [`context_local!`] and [`TracingDispatcherContext`].
#[derive(Clone)]
pub struct LocalContext {
    data: LocalData,
    tracing: Option<tracing::dispatcher::Dispatch>,
}
impl fmt::Debug for LocalContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let app = self
            .data
            .get(&AppId::local_id())
            .map(|(v, _)| v.downcast_ref::<AppId>().unwrap())
            .copied();

        f.debug_struct("LocalContext")
            .field("<app>", &app)
            .field("<entries>", &(self.data.len() - 1))
            .finish()
    }
}
impl Default for LocalContext {
    fn default() -> Self {
        Self::new()
    }
}
impl LocalContext {
    /// New empty context.
    pub const fn new() -> Self {
        Self {
            data: new_local_data(),
            tracing: None,
        }
    }

    /// Start an app scope in the current thread.
    pub fn start_app(id: AppId) -> AppScope {
        let valid = LOCAL.with_borrow_mut_dyn(|c| match c.entry(AppId::local_id()) {
            std::collections::hash_map::Entry::Occupied(_) => false,
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert((Arc::new(id), LocalValueKind::App));
                true
            }
        });
        assert!(valid, "cannot start app, another app is already in the thread context");

        AppScope {
            id,
            _same_thread: std::rc::Rc::new(()),
        }
    }
    fn end_app(id: AppId) {
        let valid = LOCAL.with_borrow_mut_dyn(|c| {
            if c.get(&AppId::local_id())
                .map(|(v, _)| v.downcast_ref::<AppId>() == Some(&id))
                .unwrap_or(false)
            {
                Some(mem::take(&mut *c))
            } else {
                None
            }
        });

        if let Some(data) = valid {
            // SAFETY: app resources may leak, but we terminate the process
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                drop(data); // deinit
            }));
            if let Err(p) = r {
                tracing::error!("panic on app drop. {}", panic_str(&p));
                eprintln!("panic on app drop. {}", panic_str(&p));
                zng_env::exit(i32::from_le_bytes(*b"appa"));
            }
        } else {
            tracing::error!("can only drop app in one of its threads");
            eprintln!("can only drop app in one of its threads");
            zng_env::exit(i32::from_le_bytes(*b"appa"));
        }
    }

    /// Get the ID of the app that owns the current context.
    pub fn current_app() -> Option<AppId> {
        LOCAL.with_borrow_dyn(|c| c.get(&AppId::local_id()).map(|(v, _)| v.downcast_ref::<AppId>().unwrap()).copied())
    }

    /// Register to run when the app deinits and all clones of the app context are dropped.
    pub fn register_cleanup(cleanup: impl FnOnce(AppId) + Send + 'static) {
        let id = Self::current_app().expect("no app in context");
        Self::register_cleanup_dyn(Box::new(move || cleanup(id)));
    }
    fn register_cleanup_dyn(cleanup: Box<dyn FnOnce() + Send>) {
        let cleanup = RunOnDrop::new(cleanup);

        type CleanupList = Vec<RunOnDrop<Box<dyn FnOnce() + Send>>>;
        LOCAL.with_borrow_mut_dyn(|c| {
            let c = c
                .entry(cleanup_list_id())
                .or_insert_with(|| (Arc::new(Mutex::new(CleanupList::new())), LocalValueKind::App));
            c.0.downcast_ref::<Mutex<CleanupList>>().unwrap().lock().push(cleanup);
        });
    }

    /// Capture a snapshot of the current context that can be restored in another thread to recreate
    /// the current context.
    ///
    /// Context locals modified after this capture are not included in the capture.
    ///
    /// This is equivalent to [``CaptureFilter::All`].
    pub fn capture() -> Self {
        Self {
            data: LOCAL.with_borrow_dyn(|c| c.clone()),
            tracing: Some(tracing::dispatcher::get_default(|d| d.clone())),
        }
    }

    /// Capture a snapshot of the current context that only includes `filter`.
    pub fn capture_filtered(filter: CaptureFilter) -> Self {
        match filter {
            CaptureFilter::None => Self::new(),
            CaptureFilter::All => Self::capture(),
            CaptureFilter::ContextVars { exclude } => {
                let mut data = new_local_data();
                LOCAL.with_borrow_dyn(|c| {
                    for (k, (v, kind)) in c.iter() {
                        if kind.include_var() && !exclude.0.contains(k) {
                            data.insert(*k, (v.clone(), *kind));
                        }
                    }
                });
                Self { data, tracing: None }
            }
            CaptureFilter::ContextLocals { exclude } => {
                let mut data = new_local_data();
                LOCAL.with_borrow_dyn(|c| {
                    for (k, (v, kind)) in c.iter() {
                        if kind.include_local() && !exclude.0.contains(k) {
                            data.insert(*k, (v.clone(), *kind));
                        }
                    }
                });
                Self {
                    data,
                    tracing: Some(tracing::dispatcher::get_default(|d| d.clone())),
                }
            }
            CaptureFilter::Include(set) => {
                let mut data = new_local_data();
                LOCAL.with_borrow_dyn(|c| {
                    for (k, v) in c.iter() {
                        if set.0.contains(k) {
                            data.insert(*k, v.clone());
                        }
                    }
                });
                Self {
                    data,
                    tracing: if set.contains(&TracingDispatcherContext) {
                        Some(tracing::dispatcher::get_default(|d| d.clone()))
                    } else {
                        None
                    },
                }
            }
            CaptureFilter::Exclude(set) => {
                let mut data = new_local_data();
                LOCAL.with_borrow_dyn(|c| {
                    for (k, v) in c.iter() {
                        if !set.0.contains(k) {
                            data.insert(*k, v.clone());
                        }
                    }
                });
                Self {
                    data,
                    tracing: if !set.contains(&TracingDispatcherContext) {
                        Some(tracing::dispatcher::get_default(|d| d.clone()))
                    } else {
                        None
                    },
                }
            }
        }
    }

    /// Collects a set of all the values in the context.
    pub fn value_set(&self) -> ContextValueSet {
        let mut set = ContextValueSet::new();
        LOCAL.with_borrow_dyn(|c| {
            for k in c.keys() {
                set.0.insert(*k);
            }
        });
        set
    }

    /// Calls `f` in the captured context.
    ///
    /// Note that this fully replaces the parent context for the duration of the `f` call, see [`with_context_blend`]
    /// for a blending alternative.
    ///
    /// [`with_context_blend`]: Self::with_context_blend
    #[inline(always)]
    pub fn with_context<R>(&mut self, f: impl FnOnce() -> R) -> R {
        struct Restore<'a> {
            prev_data: LocalData,
            _tracing_restore: Option<tracing::dispatcher::DefaultGuard>,
            ctx: &'a mut LocalContext,
        }
        impl<'a> Restore<'a> {
            fn new(ctx: &'a mut LocalContext) -> Self {
                let data = mem::take(&mut ctx.data);
                Self {
                    prev_data: LOCAL.with_borrow_mut_dyn(|c| mem::replace(c, data)),
                    _tracing_restore: ctx.tracing.as_ref().map(tracing::dispatcher::set_default),
                    ctx,
                }
            }
        }
        impl<'a> Drop for Restore<'a> {
            fn drop(&mut self) {
                self.ctx.data = LOCAL.with_borrow_mut_dyn(|c| mem::replace(c, mem::take(&mut self.prev_data)));
            }
        }
        let _restore = Restore::new(self);

        f()
    }

    /// Calls `f` while all contextual values of `self` are set on the parent context.
    ///
    /// Unlike [`with_context`] this does not remove values that are only set in the parent context, the
    /// downside is that this call is more expensive.
    ///
    /// If `over` is `true` all the values of `self` are set over the parent values, if `false` only
    /// the values not already set in the parent are set.
    ///
    /// [`with_context`]: Self::with_context
    #[inline(always)]
    pub fn with_context_blend<R>(&mut self, over: bool, f: impl FnOnce() -> R) -> R {
        if self.data.is_empty() {
            f()
        } else {
            struct Restore {
                prev_data: LocalData,
                _tracing_restore: Option<tracing::dispatcher::DefaultGuard>,
            }
            impl Restore {
                fn new(ctx: &mut LocalContext, over: bool) -> Self {
                    let prev_data = LOCAL.with_borrow_mut_dyn(|c| {
                        let mut new_data = c.clone();
                        if over {
                            for (k, v) in &ctx.data {
                                new_data.insert(*k, v.clone());
                            }
                        } else {
                            for (k, v) in &ctx.data {
                                new_data.entry(*k).or_insert_with(|| v.clone());
                            }
                        }

                        mem::replace(c, new_data)
                    });

                    let mut _tracing_restore = None;
                    if let Some(d) = &ctx.tracing
                        && over
                    {
                        _tracing_restore = Some(tracing::dispatcher::set_default(d));
                    }

                    Self {
                        prev_data,
                        _tracing_restore,
                    }
                }
            }
            impl Drop for Restore {
                fn drop(&mut self) {
                    LOCAL.with_borrow_mut_dyn(|c| {
                        *c = mem::take(&mut self.prev_data);
                    });
                }
            }
            let _restore = Restore::new(self, over);

            f()
        }
    }

    /// Blend `ctx` over `self`.
    pub fn extend(&mut self, ctx: Self) {
        self.data.extend(ctx.data);
    }

    fn contains(key: AppLocalId) -> bool {
        LOCAL.with_borrow_dyn(|c| c.contains_key(&key))
    }

    fn get(key: AppLocalId) -> Option<LocalValue> {
        LOCAL.with_borrow_dyn(|c| c.get(&key).cloned())
    }

    fn set(key: AppLocalId, value: LocalValue) -> Option<LocalValue> {
        LOCAL.with_borrow_mut_dyn(|c| c.insert(key, value))
    }
    fn remove(key: AppLocalId) -> Option<LocalValue> {
        LOCAL.with_borrow_mut_dyn(|c| c.remove(&key))
    }

    #[inline(always)]
    fn with_value_ctx<T: Send + Sync + 'static>(
        key: &'static ContextLocal<T>,
        kind: LocalValueKind,
        value: &mut Option<Arc<T>>,
        f: impl FnOnce(),
    ) {
        struct Restore<'a, T: Send + Sync + 'static> {
            key: AppLocalId,
            prev: Option<LocalValue>,
            value: &'a mut Option<Arc<T>>,
        }
        impl<'a, T: Send + Sync + 'static> Restore<'a, T> {
            fn new(key: &'static ContextLocal<T>, kind: LocalValueKind, value: &'a mut Option<Arc<T>>) -> Self {
                Self {
                    key: key.id(),
                    prev: LocalContext::set(key.id(), (value.take().expect("no `value` to set"), kind)),
                    value,
                }
            }
        }
        impl<'a, T: Send + Sync + 'static> Drop for Restore<'a, T> {
            fn drop(&mut self) {
                let back = if let Some(prev) = self.prev.take() {
                    LocalContext::set(self.key, prev)
                } else {
                    LocalContext::remove(self.key)
                }
                .unwrap();
                *self.value = Some(Arc::downcast(back.0).unwrap());
            }
        }
        let _restore = Restore::new(key, kind, value);

        f()
    }

    #[inline(always)]
    fn with_default_ctx<T: Send + Sync + 'static>(key: &'static ContextLocal<T>, f: impl FnOnce()) {
        struct Restore {
            key: AppLocalId,
            prev: Option<LocalValue>,
        }
        impl Drop for Restore {
            fn drop(&mut self) {
                if let Some(prev) = self.prev.take() {
                    LocalContext::set(self.key, prev);
                }
            }
        }
        let _restore = Restore {
            key: key.id(),
            prev: Self::remove(key.id()),
        };

        f()
    }
}
thread_local! {
    static LOCAL: RefCell<LocalData> = const { RefCell::new(new_local_data()) };
}

trait LocalKeyDyn {
    fn with_borrow_dyn<R>(&'static self, f: impl FnOnce(&LocalData) -> R) -> R;
    fn with_borrow_mut_dyn<R>(&'static self, f: impl FnOnce(&mut LocalData) -> R) -> R;
}
impl LocalKeyDyn for LocalKey<RefCell<LocalData>> {
    fn with_borrow_dyn<R>(&'static self, f: impl FnOnce(&LocalData) -> R) -> R {
        let mut r = None;
        let f = |l: &LocalData| r = Some(f(l));

        self.with_borrow(f);

        r.unwrap()
    }

    fn with_borrow_mut_dyn<R>(&'static self, f: impl FnOnce(&mut LocalData) -> R) -> R {
        let mut r = None;
        let f = |l: &mut LocalData| r = Some(f(l));

        self.with_borrow_mut(f);

        r.unwrap()
    }
}

/// Defines a [`LocalContext::capture_filtered`] filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureFilter {
    /// Don't capture anything, equivalent of [`LocalContext::new`].
    None,

    /// Capture all [`context_local!`] values and [`TracingDispatcherContext`].
    All,
    /// Capture all variables not excluded, no [`context_local!`] nor [`TracingDispatcherContext`].
    ContextVars {
        /// Vars to not include.
        exclude: ContextValueSet,
    },
    /// Capture all [`context_local!`] and [`TracingDispatcherContext`] not excluded, no context variables.
    ContextLocals {
        /// Locals to not include.
        exclude: ContextValueSet,
    },

    /// Capture only this set.
    Include(ContextValueSet),

    /// Capture all except this set.
    Exclude(ContextValueSet),
}
impl CaptureFilter {
    /// Capture all variables, no [`context_local!`] nor [`TracingDispatcherContext`].
    pub const fn context_vars() -> Self {
        Self::ContextVars {
            exclude: ContextValueSet::new(),
        }
    }

    /// Capture all [`context_local!`] and [`TracingDispatcherContext`], no context variables.
    pub const fn context_locals() -> Self {
        Self::ContextLocals {
            exclude: ContextValueSet::new(),
        }
    }

    /// Only capture the [`app_local!`] and [`TracingDispatcherContext`].
    pub fn app_only() -> Self {
        let mut set = ContextValueSet::new();
        set.insert_app();
        Self::Include(set)
    }
}

/// Provides an identifying key for a context local value.
///
/// Implemented by all [`ContextLocal<T>`] already, only implement this for context local thin wrappers.
pub trait ContextLocalKeyProvider {
    /// Gets the key.
    fn context_local_key(&'static self) -> AppLocalId;
}
impl<T: Send + Sync + 'static> ContextLocalKeyProvider for ContextLocal<T> {
    fn context_local_key(&'static self) -> AppLocalId {
        self.id()
    }
}

/// Represents the [`tracing::dispatcher::get_default`] dispatcher in a context value set.
///
/// [`tracing::dispatcher::get_default`]: https://docs.rs/tracing/latest/tracing/dispatcher/fn.get_global_default.html
#[allow(clippy::exhaustive_structs)]
pub struct TracingDispatcherContext;

impl ContextLocalKeyProvider for TracingDispatcherContext {
    fn context_local_key(&'static self) -> AppLocalId {
        static ID: bool = true;
        AppLocalId(&ID as *const bool as *const () as usize)
    }
}

/// Identifies a selection of [`LocalContext`] values.
#[derive(Default, Clone, PartialEq, Eq)]
pub struct ContextValueSet(LocalSet);
impl ContextValueSet {
    /// New empty.
    pub const fn new() -> Self {
        Self(new_local_set())
    }

    /// Insert a context local.
    pub fn insert(&mut self, value: &'static impl ContextLocalKeyProvider) -> bool {
        self.0.insert(value.context_local_key())
    }

    /// Remove a context local.
    pub fn remove(&mut self, value: &'static impl ContextLocalKeyProvider) -> bool {
        self.0.remove(&value.context_local_key())
    }

    /// Checks if the context local is in the set.
    pub fn contains(&self, value: &'static impl ContextLocalKeyProvider) -> bool {
        self.0.contains(&value.context_local_key())
    }

    /// Number of unique values in the set.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// If the set has any values.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Extend this set with all `other` contexts.
    pub fn insert_all(&mut self, other: &Self) {
        self.0.extend(other.0.iter().copied());
    }

    /// Removes all `other` contexts from this set.
    pub fn remove_all(&mut self, other: &Self) {
        for o in other.0.iter() {
            self.0.remove(o);
        }
    }

    /// Insert the [`app_local!`] ID and [`TracingDispatcherContext`].
    pub fn insert_app(&mut self) -> bool {
        let inserted_app = self.0.insert(AppId::local_id());
        static TRACING: TracingDispatcherContext = TracingDispatcherContext;
        self.insert(&TRACING) || inserted_app
    }
}
impl fmt::Debug for ContextValueSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContextValueSet").field("len()", &self.len()).finish()
    }
}
