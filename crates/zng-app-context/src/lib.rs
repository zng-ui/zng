#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//! App execution context.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::{any::Any, cell::RefCell, fmt, mem, ops, sync::Arc, thread::LocalKey, time::Duration};

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
    pub fn with_context<R>(&mut self, f: impl FnOnce() -> R) -> R {
        let data = mem::take(&mut self.data);
        let prev = LOCAL.with_borrow_mut_dyn(|c| mem::replace(c, data));
        let _tracing_restore = self.tracing.as_ref().map(tracing::dispatcher::set_default);
        let _restore = RunOnDrop::new(|| {
            self.data = LOCAL.with_borrow_mut_dyn(|c| mem::replace(c, prev));
        });
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
    pub fn with_context_blend<R>(&mut self, over: bool, f: impl FnOnce() -> R) -> R {
        if self.data.is_empty() {
            f()
        } else {
            let prev = LOCAL.with_borrow_mut_dyn(|c| {
                let (mut base, over) = if over { (c.clone(), &self.data) } else { (self.data.clone(), &*c) };
                for (k, v) in over {
                    base.insert(*k, v.clone());
                }

                mem::replace(c, base)
            });
            let _restore = RunOnDrop::new(|| {
                LOCAL.with_borrow_mut_dyn(|c| {
                    *c = prev;
                });
            });
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

    fn with_value_ctx<T: Send + Sync + 'static>(
        key: &'static ContextLocal<T>,
        kind: LocalValueKind,
        value: &mut Option<Arc<T>>,
        f: impl FnOnce(),
    ) {
        let key = key.id();
        let prev = Self::set(key, (value.take().expect("no `value` to set"), kind));
        let _restore = RunOnDrop::new(move || {
            let back = if let Some(prev) = prev {
                Self::set(key, prev)
            } else {
                Self::remove(key)
            }
            .unwrap();
            *value = Some(Arc::downcast(back.0).unwrap());
        });

        f();
    }

    fn with_default_ctx<T: Send + Sync + 'static>(key: &'static ContextLocal<T>, f: impl FnOnce()) {
        let key = key.id();
        let prev = Self::remove(key);
        let _restore = RunOnDrop::new(move || {
            if let Some(prev) = prev {
                Self::set(key, prev);
            }
        });

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

        #[cfg(feature = "dyn_closure")]
        let f: Box<dyn FnOnce(&LocalData)> = Box::new(f);

        self.with_borrow(f);

        r.unwrap()
    }

    fn with_borrow_mut_dyn<R>(&'static self, f: impl FnOnce(&mut LocalData) -> R) -> R {
        let mut r = None;
        let f = |l: &mut LocalData| r = Some(f(l));

        #[cfg(feature = "dyn_closure")]
        let f: Box<dyn FnOnce(&mut LocalData)> = Box::new(f);

        self.with_borrow_mut(f);

        r.unwrap()
    }
}

/*
    app_local!
*/

#[doc(hidden)]
pub struct AppLocalConst<T: Send + Sync + 'static> {
    value: RwLock<T>,
}
impl<T: Send + Sync + 'static> AppLocalConst<T> {
    pub const fn new(init: T) -> Self {
        Self { value: RwLock::new(init) }
    }
}
#[doc(hidden)]
pub struct AppLocalOption<T: Send + Sync + 'static> {
    value: RwLock<Option<T>>,
    init: fn() -> T,
}
impl<T: Send + Sync + 'static> AppLocalOption<T> {
    pub const fn new(init: fn() -> T) -> Self {
        Self {
            value: RwLock::new(None),
            init,
        }
    }

    fn read_impl(&'static self, read: RwLockReadGuard<'static, Option<T>>) -> MappedRwLockReadGuard<'static, T> {
        if read.is_some() {
            return RwLockReadGuard::map(read, |v| v.as_ref().unwrap());
        }
        drop(read);

        let mut write = self.value.write();
        if write.is_some() {
            drop(write);
            return self.read();
        }

        let value = (self.init)();
        *write = Some(value);

        let read = RwLockWriteGuard::downgrade(write);

        RwLockReadGuard::map(read, |v| v.as_ref().unwrap())
    }

    fn write_impl(&'static self, mut write: RwLockWriteGuard<'static, Option<T>>) -> MappedRwLockWriteGuard<'static, T> {
        if write.is_some() {
            return RwLockWriteGuard::map(write, |v| v.as_mut().unwrap());
        }

        let value = (self.init)();
        *write = Some(value);

        RwLockWriteGuard::map(write, |v| v.as_mut().unwrap())
    }
}

#[doc(hidden)]
pub struct AppLocalVec<T: Send + Sync + 'static> {
    value: RwLock<Vec<(AppId, T)>>,
    init: fn() -> T,
}
impl<T: Send + Sync + 'static> AppLocalVec<T> {
    pub const fn new(init: fn() -> T) -> Self {
        Self {
            value: RwLock::new(vec![]),
            init,
        }
    }

    fn cleanup(&'static self, id: AppId) {
        self.try_cleanup(id, 0);
    }
    fn try_cleanup(&'static self, id: AppId, tries: u8) {
        if let Some(mut w) = self.value.try_write_for(if tries == 0 {
            Duration::from_millis(50)
        } else {
            Duration::from_millis(500)
        }) {
            if let Some(i) = w.iter().position(|(s, _)| *s == id) {
                w.swap_remove(i);
            }
        } else if tries > 5 {
            tracing::error!("failed to cleanup `app_local` for {id:?}, was locked after app drop");
        } else {
            std::thread::spawn(move || {
                self.try_cleanup(id, tries + 1);
            });
        }
    }

    fn read_impl(&'static self, read: RwLockReadGuard<'static, Vec<(AppId, T)>>) -> MappedRwLockReadGuard<'static, T> {
        let id = LocalContext::current_app().expect("no app running, `app_local` can only be accessed inside apps");

        if let Some(i) = read.iter().position(|(s, _)| *s == id) {
            return RwLockReadGuard::map(read, |v| &v[i].1);
        }
        drop(read);

        let mut write = self.value.write();
        if write.iter().any(|(s, _)| *s == id) {
            drop(write);
            return self.read();
        }

        let value = (self.init)();
        let i = write.len();
        write.push((id, value));

        LocalContext::register_cleanup(Box::new(move |id| self.cleanup(id)));

        let read = RwLockWriteGuard::downgrade(write);

        RwLockReadGuard::map(read, |v| &v[i].1)
    }

    fn write_impl(&'static self, mut write: RwLockWriteGuard<'static, Vec<(AppId, T)>>) -> MappedRwLockWriteGuard<'static, T> {
        let id = LocalContext::current_app().expect("no app running, `app_local` can only be accessed inside apps");

        if let Some(i) = write.iter().position(|(s, _)| *s == id) {
            return RwLockWriteGuard::map(write, |v| &mut v[i].1);
        }

        let value = (self.init)();
        let i = write.len();
        write.push((id, value));

        LocalContext::register_cleanup(move |id| self.cleanup(id));

        RwLockWriteGuard::map(write, |v| &mut v[i].1)
    }
}
#[doc(hidden)]
pub trait AppLocalImpl<T: Send + Sync + 'static>: Send + Sync + 'static {
    fn read(&'static self) -> MappedRwLockReadGuard<'static, T>;
    fn try_read(&'static self) -> Option<MappedRwLockReadGuard<'static, T>>;
    fn write(&'static self) -> MappedRwLockWriteGuard<'static, T>;
    fn try_write(&'static self) -> Option<MappedRwLockWriteGuard<'static, T>>;
}

impl<T: Send + Sync + 'static> AppLocalImpl<T> for AppLocalVec<T> {
    fn read(&'static self) -> MappedRwLockReadGuard<'static, T> {
        self.read_impl(self.value.read_recursive())
    }

    fn try_read(&'static self) -> Option<MappedRwLockReadGuard<'static, T>> {
        Some(self.read_impl(self.value.try_read_recursive()?))
    }

    fn write(&'static self) -> MappedRwLockWriteGuard<'static, T> {
        self.write_impl(self.value.write())
    }

    fn try_write(&'static self) -> Option<MappedRwLockWriteGuard<'static, T>> {
        Some(self.write_impl(self.value.try_write()?))
    }
}
impl<T: Send + Sync + 'static> AppLocalImpl<T> for AppLocalOption<T> {
    fn read(&'static self) -> MappedRwLockReadGuard<'static, T> {
        self.read_impl(self.value.read_recursive())
    }

    fn try_read(&'static self) -> Option<MappedRwLockReadGuard<'static, T>> {
        Some(self.read_impl(self.value.try_read_recursive()?))
    }

    fn write(&'static self) -> MappedRwLockWriteGuard<'static, T> {
        self.write_impl(self.value.write())
    }

    fn try_write(&'static self) -> Option<MappedRwLockWriteGuard<'static, T>> {
        Some(self.write_impl(self.value.try_write()?))
    }
}
impl<T: Send + Sync + 'static> AppLocalImpl<T> for AppLocalConst<T> {
    fn read(&'static self) -> MappedRwLockReadGuard<'static, T> {
        RwLockReadGuard::map(self.value.read(), |l| l)
    }

    fn try_read(&'static self) -> Option<MappedRwLockReadGuard<'static, T>> {
        Some(RwLockReadGuard::map(self.value.try_read()?, |l| l))
    }

    fn write(&'static self) -> MappedRwLockWriteGuard<'static, T> {
        RwLockWriteGuard::map(self.value.write(), |l| l)
    }

    fn try_write(&'static self) -> Option<MappedRwLockWriteGuard<'static, T>> {
        Some(RwLockWriteGuard::map(self.value.try_write()?, |l| l))
    }
}

/// An app local storage.
///
/// This is similar to [`std::thread::LocalKey`], but works across all threads of the app.
///
/// Use the [`app_local!`] macro to declare a static variable in the same style as [`thread_local!`].
///
/// Note that in `"multi_app"` builds the app local can only be used if an app is running in the thread,
/// if no app is running read and write **will panic**.
pub struct AppLocal<T: Send + Sync + 'static> {
    inner: fn() -> &'static dyn AppLocalImpl<T>,
}
impl<T: Send + Sync + 'static> AppLocal<T> {
    #[doc(hidden)]
    pub const fn new(inner: fn() -> &'static dyn AppLocalImpl<T>) -> Self {
        AppLocal { inner }
    }

    /// Read lock the value associated with the current app.
    ///
    /// Initializes the default value for the app if this is the first value access.
    ///
    /// # Panics
    ///
    /// Panics if no app is running in `"multi_app"` builds.
    #[inline]
    pub fn read(&'static self) -> MappedRwLockReadGuard<'static, T> {
        (self.inner)().read()
    }

    /// Try read lock the value associated with the current app.
    ///
    /// Initializes the default value for the app if this is the first value access.
    ///
    /// Returns `None` if can’t acquire a read lock.
    ///
    /// # Panics
    ///
    /// Panics if no app is running in `"multi_app"` builds.
    #[inline]
    pub fn try_read(&'static self) -> Option<MappedRwLockReadGuard<'static, T>> {
        (self.inner)().try_read()
    }

    /// Write lock the value associated with the current app.
    ///
    /// Initializes the default value for the app if this is the first value access.
    ///
    /// # Panics
    ///
    /// Panics if no app is running in `"multi_app"` builds.
    #[inline]
    pub fn write(&'static self) -> MappedRwLockWriteGuard<'static, T> {
        (self.inner)().write()
    }

    /// Try to write lock the value associated with the current app.
    ///
    /// Initializes the default value for the app if this is the first value access.
    ///
    /// Returns `None` if can’t acquire a write lock.
    ///
    /// # Panics
    ///
    /// Panics if no app is running in `"multi_app"` builds.
    pub fn try_write(&'static self) -> Option<MappedRwLockWriteGuard<'static, T>> {
        (self.inner)().try_write()
    }

    /// Get a clone of the value.
    #[inline]
    pub fn get(&'static self) -> T
    where
        T: Clone,
    {
        self.read().clone()
    }

    /// Set the value.
    #[inline]
    pub fn set(&'static self, value: T) {
        *self.write() = value;
    }

    /// Try to get a clone of the value.
    ///
    /// Returns `None` if can't acquire a read lock.
    #[inline]
    pub fn try_get(&'static self) -> Option<T>
    where
        T: Clone,
    {
        self.try_read().map(|l| l.clone())
    }

    /// Try to set the value.
    ///
    /// Returns `Err(value)` if can't acquire a write lock.
    #[inline]
    pub fn try_set(&'static self, value: T) -> Result<(), T> {
        match self.try_write() {
            Some(mut l) => {
                *l = value;
                Ok(())
            }
            None => Err(value),
        }
    }

    /// Create a read lock and `map` it to a sub-value.
    #[inline]
    pub fn read_map<O>(&'static self, map: impl FnOnce(&T) -> &O) -> MappedRwLockReadGuard<'static, O> {
        MappedRwLockReadGuard::map(self.read(), map)
    }

    /// Try to create a read lock and `map` it to a sub-value.
    #[inline]
    pub fn try_read_map<O>(&'static self, map: impl FnOnce(&T) -> &O) -> Option<MappedRwLockReadGuard<'static, O>> {
        let lock = self.try_read()?;
        Some(MappedRwLockReadGuard::map(lock, map))
    }

    /// Create a write lock and `map` it to a sub-value.
    #[inline]
    pub fn write_map<O>(&'static self, map: impl FnOnce(&mut T) -> &mut O) -> MappedRwLockWriteGuard<'static, O> {
        MappedRwLockWriteGuard::map(self.write(), map)
    }

    /// Try to create a write lock and `map` it to a sub-value.
    #[inline]
    pub fn try_write_map<O>(&'static self, map: impl FnOnce(&mut T) -> &mut O) -> Option<MappedRwLockWriteGuard<'static, O>> {
        let lock = self.try_write()?;
        Some(MappedRwLockWriteGuard::map(lock, map))
    }

    /// Gets an ID for this local instance that is valid for the lifetime of the process.
    ///
    /// Note that comparing two `&'static LOCAL` pointers is incorrect, because in `"hot_reload"` builds the statics
    /// can be different and still represent the same app local. This ID identifies the actual inner pointer.
    pub fn id(&'static self) -> AppLocalId {
        AppLocalId((self.inner)() as *const dyn AppLocalImpl<T> as *const () as _)
    }
}
impl<T: Send + Sync + 'static> PartialEq for AppLocal<T> {
    fn eq(&self, other: &Self) -> bool {
        let a = AppLocalId((self.inner)() as *const dyn AppLocalImpl<T> as *const () as _);
        let b = AppLocalId((other.inner)() as *const dyn AppLocalImpl<T> as *const () as _);
        a == b
    }
}
impl<T: Send + Sync + 'static> Eq for AppLocal<T> {}
impl<T: Send + Sync + 'static> std::hash::Hash for AppLocal<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let a = AppLocalId((self.inner)() as *const dyn AppLocalImpl<T> as *const () as _);
        std::hash::Hash::hash(&a, state)
    }
}

/// Identifies an [`AppLocal<T>`] instance.
///
/// Note that comparing two `&'static LOCAL` pointers is incorrect, because in `"hot_reload"` builds the statics
/// can be different and still represent the same app local. This ID identifies the actual inner pointer, it is
/// valid for the lifetime of the process.
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct AppLocalId(usize);
impl AppLocalId {
    /// Get the underlying value.
    pub fn get(self) -> usize {
        // VarPtr depends on this being an actual pointer (must be unique against an `Arc<T>` raw pointer).
        self.0 as _
    }
}
impl fmt::Debug for AppLocalId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AppLocalId({:#x})", self.0)
    }
}

///<span data-del-macro-root></span> Declares new app local variable.
///
/// An app local is a static variable that is declared using the same syntax as [`thread_local!`], but can be
/// accessed by any thread in the app. In apps that only run once per process it compiles down to the equivalent
/// of a `static LOCAL: RwLock<T> = const;` or `static LOCAL: RwLock<Option<T>>` that initializes on first usage. In test
/// builds with multiple parallel apps it compiles to a switching storage that provides a different value depending on
/// what app is running in the current thread.
///
/// See [`AppLocal<T>`] for more details.
///
/// # Multi App
///
/// If the crate is compiled with the `"multi_app"` feature a different internal implementation is used that supports multiple
/// apps, either running in parallel in different threads or one after the other. This backing implementation has some small overhead,
/// but usually you only want multiple app instances per-process when running tests.
///
/// The lifetime of `"multi_app"` locals is also more limited, trying to use an app-local before starting to build an app will panic,
/// the app-local value will be dropped when the app is dropped. Without the `"multi_app"` feature the app-locals can be used at
/// any point before or after the app lifetime, values are not explicitly dropped, just unloaded with the process.
///
/// # Const
///
/// The initialization expression can be wrapped in a `const { .. }` block, if the `"multi_app"` feature is **not** enabled
/// a faster implementation is used that is equivalent to a direct `static LOCAL: RwLock<T>` in terms of performance.
///
/// Note that this syntax is available even if the `"multi_app"` feature is enabled, the expression must be const either way,
/// but with the feature the same dynamic implementation is used.
///
/// Note that `const` initialization does not automatically convert the value into the static type.
///
/// # Examples
///
/// The example below declares two app locals, note that `BAR` init value automatically converts into the app local type.
///
/// ```
/// # use zng_app_context::*;
/// app_local! {
///     /// A public documented value.
///     pub static FOO: u8 = const { 10u8 };
///
///     // A private value.
///     static BAR: String = "Into!";
/// }
///
/// let app = LocalContext::start_app(AppId::new_unique());
///
/// assert_eq!(10, FOO.get());
/// ```
///
/// Also note that an app context is started before the first use, in `multi_app` builds trying to use an app local in
/// a thread not owned by an app panics.
#[macro_export]
macro_rules! app_local {
    ($(
        $(#[$meta:meta])*
        $vis:vis static $IDENT:ident : $T:ty = $(const { $init_const:expr })? $($init:expr_2021)?;
    )+) => {$(
        $crate::app_local_impl! {
            $(#[$meta])*
            $vis static $IDENT: $T = $(const { $init_const })? $($init)?;
        }
    )+};
}

#[doc(hidden)]
#[macro_export]
macro_rules! app_local_impl_single {
    (
        $(#[$meta:meta])*
        $vis:vis static $IDENT:ident : $T:ty = const { $init:expr };
    ) => {
        $(#[$meta])*
        $vis static $IDENT: $crate::AppLocal<$T> = {
            fn s() -> &'static dyn $crate::AppLocalImpl<$T> {
                $crate::hot_static! {
                    static IMPL: $crate::AppLocalConst<$T> = $crate::AppLocalConst::new($init);
                }
                $crate::hot_static_ref!(IMPL)
            }
            $crate::AppLocal::new(s)
        };
    };
    (
        $(#[$meta:meta])*
        $vis:vis static $IDENT:ident : $T:ty = $init:expr_2021;
    ) => {
        $(#[$meta])*
        $vis static $IDENT: $crate::AppLocal<$T> = {
            fn s() -> &'static dyn $crate::AppLocalImpl<$T> {
                fn init() -> $T {
                    std::convert::Into::into($init)
                }
                $crate::hot_static! {
                    static IMPL: $crate::AppLocalOption<$T> = $crate::AppLocalOption::new(init);
                }
                $crate::hot_static_ref!(IMPL)
            }
            $crate::AppLocal::new(s)
        };
    };
    (
        $(#[$meta:meta])*
        $vis:vis static $IDENT:ident : $T:ty = ($tt:tt)*
    ) => {
        std::compile_error!("expected `const { $expr };` or `$expr;`")
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! app_local_impl_multi {
    (
        $(#[$meta:meta])*
        $vis:vis static $IDENT:ident : $T:ty = const { $init:expr };
    ) => {
        $(#[$meta])*
        $vis static $IDENT: $crate::AppLocal<$T> = {
            fn s() -> &'static dyn $crate::AppLocalImpl<$T> {
                const fn init() -> $T {
                    $init
                }
                $crate::hot_static! {
                    static IMPL: $crate::AppLocalVec<$T> = $crate::AppLocalVec::new(init);
                }
                $crate::hot_static_ref!(IMPL)
            }
            $crate::AppLocal::new(s)
        };
    };
    (
        $(#[$meta:meta])*
        $vis:vis static $IDENT:ident : $T:ty = $init:expr_2021;
    ) => {
        $(#[$meta])*
        $vis static $IDENT: $crate::AppLocal<$T> = {
            fn s() -> &'static dyn $crate::AppLocalImpl<$T> {
                fn init() -> $T {
                    std::convert::Into::into($init)
                }
                $crate::hot_static! {
                    static IMPL: $crate::AppLocalVec<$T> = $crate::AppLocalVec::new(init);
                }
                $crate::hot_static_ref!(IMPL)
            }
            $crate::AppLocal::new(s)
        };
    };
    (
        $(#[$meta:meta])*
        $vis:vis static $IDENT:ident : $T:ty = ($tt:tt)*
    ) => {
        std::compile_error!("expected `const { $expr };` or `$expr;`")
    };
}

#[cfg(feature = "multi_app")]
#[doc(hidden)]
pub use app_local_impl_multi as app_local_impl;
#[cfg(not(feature = "multi_app"))]
#[doc(hidden)]
pub use app_local_impl_single as app_local_impl;

/*
    context_local!
*/

#[doc(hidden)]
pub struct ContextLocalData<T: Send + Sync + 'static> {
    default_init: fn() -> T,
    default_value: Option<Arc<T>>,
}
impl<T: Send + Sync + 'static> ContextLocalData<T> {
    #[doc(hidden)]
    pub const fn new(default_init: fn() -> T) -> Self {
        Self {
            default_init,
            default_value: None,
        }
    }
}

/// Represents an [`AppLocal<T>`] value that can be temporarily overridden in a context.
///
/// The *context* works across threads, as long as the threads are instrumented using [`LocalContext`].
///
/// Use the [`context_local!`] macro to declare a static variable in the same style as [`thread_local!`].
pub struct ContextLocal<T: Send + Sync + 'static> {
    data: AppLocal<ContextLocalData<T>>,
}
impl<T: Send + Sync + 'static> ContextLocal<T> {
    #[doc(hidden)]
    pub const fn new(storage: fn() -> &'static dyn AppLocalImpl<ContextLocalData<T>>) -> Self {
        Self {
            data: AppLocal::new(storage),
        }
    }

    /// Gets an ID for this context local instance that is valid for the lifetime of the process.
    ///
    /// Note that comparing two `&'static CTX_LOCAL` pointers is incorrect, because in `"hot_reload"` builds the statics
    /// can be different and still represent the same app local. This ID identifies the actual inner pointer.
    pub fn id(&'static self) -> AppLocalId {
        self.data.id()
    }

    /// Calls `f` with the `value` loaded in context.
    ///
    /// The `value` is moved into context, `f` is called, then the value is moved back to `value`.
    ///
    /// # Panics
    ///
    /// Panics if `value` is `None`.
    pub fn with_context<R>(&'static self, value: &mut Option<Arc<T>>, f: impl FnOnce() -> R) -> R {
        let mut r = None;
        let f = || r = Some(f());
        #[cfg(feature = "dyn_closure")]
        let f: Box<dyn FnOnce()> = Box::new(f);

        LocalContext::with_value_ctx(self, LocalValueKind::Local, value, f);

        r.unwrap()
    }

    /// Same as [`with_context`], but `value` represents a variable.
    ///
    /// Values loaded with this method are captured by [`CaptureFilter::ContextVars`].
    ///
    /// [`with_context`]: Self::with_context
    pub fn with_context_var<R>(&'static self, value: &mut Option<Arc<T>>, f: impl FnOnce() -> R) -> R {
        let mut r = None;
        let f = || r = Some(f());

        #[cfg(feature = "dyn_closure")]
        let f: Box<dyn FnOnce()> = Box::new(f);

        LocalContext::with_value_ctx(self, LocalValueKind::Var, value, f);

        r.unwrap()
    }

    /// Calls `f` with no value loaded in context.
    pub fn with_default<R>(&'static self, f: impl FnOnce() -> R) -> R {
        let mut r = None;
        let f = || r = Some(f());

        #[cfg(feature = "dyn_closure")]
        let f: Box<dyn FnOnce()> = Box::new(f);
        LocalContext::with_default_ctx(self, f);

        r.unwrap()
    }

    /// Gets if no value is set in the context.
    pub fn is_default(&'static self) -> bool {
        !LocalContext::contains(self.id())
    }

    /// Clone a reference to the current value in the context or the default value.
    pub fn get(&'static self) -> Arc<T> {
        let cl = self.data.read();
        match LocalContext::get(self.id()) {
            Some(c) => Arc::downcast(c.0).unwrap(),
            None => match &cl.default_value {
                Some(d) => d.clone(),
                None => {
                    drop(cl);
                    let mut cl = self.data.write();
                    match &cl.default_value {
                        None => {
                            let d = Arc::new((cl.default_init)());
                            cl.default_value = Some(d.clone());
                            d
                        }
                        Some(d) => d.clone(),
                    }
                }
            },
        }
    }

    /// Clone the current value in the context or the default value.
    pub fn get_clone(&'static self) -> T
    where
        T: Clone,
    {
        let cl = self.data.read();
        match LocalContext::get(self.id()) {
            Some(c) => c.0.downcast_ref::<T>().unwrap().clone(),
            None => match &cl.default_value {
                Some(d) => d.as_ref().clone(),
                None => {
                    drop(cl);
                    let mut cl = self.data.write();
                    match &cl.default_value {
                        None => {
                            let val = (cl.default_init)();
                            let r = val.clone();
                            cl.default_value = Some(Arc::new(val));
                            r
                        }
                        Some(d) => d.as_ref().clone(),
                    }
                }
            },
        }
    }
}

impl<T: Send + Sync + 'static> ContextLocal<RwLock<T>> {
    /// Gets a read-only shared reference to the current context value.
    pub fn read_only(&'static self) -> ReadOnlyRwLock<T> {
        ReadOnlyRwLock::new(self.get())
    }

    /// Locks this `RwLock` with shared read access, blocking the current thread until it can be acquired.
    ///
    /// See `parking_lot::RwLock::read` for more details.
    pub fn read(&'static self) -> RwLockReadGuardOwned<T> {
        RwLockReadGuardOwned::lock(self.get())
    }

    /// Locks this `RwLock` with shared read access, blocking the current thread until it can be acquired.
    ///
    /// Unlike `read`, this method is guaranteed to succeed without blocking if
    /// another read lock is held at the time of the call.
    ///
    /// See `parking_lot::RwLock::read` for more details.
    pub fn read_recursive(&'static self) -> RwLockReadGuardOwned<T> {
        RwLockReadGuardOwned::lock_recursive(self.get())
    }

    /// Locks this `RwLock` with exclusive write access, blocking the current
    /// thread until it can be acquired.
    ///
    /// See `parking_lot::RwLock::write` for more details.
    pub fn write(&'static self) -> RwLockWriteGuardOwned<T> {
        RwLockWriteGuardOwned::lock(self.get())
    }

    /// Try lock this `RwLock` with shared read access, blocking the current thread until it can be acquired.
    ///
    /// See `parking_lot::RwLock::try_read` for more details.
    pub fn try_read(&'static self) -> Option<RwLockReadGuardOwned<T>> {
        RwLockReadGuardOwned::try_lock(self.get())
    }

    /// Locks this `RwLock` with shared read access, blocking the current thread until it can be acquired.
    ///
    /// See `parking_lot::RwLock::try_read_recursive` for more details.
    pub fn try_read_recursive(&'static self) -> Option<RwLockReadGuardOwned<T>> {
        RwLockReadGuardOwned::try_lock_recursive(self.get())
    }

    /// Locks this `RwLock` with exclusive write access, blocking the current
    /// thread until it can be acquired.
    ///
    /// See `parking_lot::RwLock::try_write` for more details.
    pub fn try_write(&'static self) -> Option<RwLockWriteGuardOwned<T>> {
        RwLockWriteGuardOwned::try_lock(self.get())
    }
}

/// Represents a read guard for an `Arc<RwLock<T>>` that owns a reference to it.
pub struct RwLockReadGuardOwned<T: 'static> {
    lock: parking_lot::RwLockReadGuard<'static, T>,
    _owned: Arc<RwLock<T>>,
}
impl<T> RwLockReadGuardOwned<T> {
    /// Lock owned.    
    ///
    /// See `parking_lot::RwLock::read` for more details.
    pub fn lock(own: Arc<RwLock<T>>) -> Self {
        Self {
            // SAFETY: we cast to 'static only for storage, `lock` is dropped before `_owned`.
            lock: unsafe { mem::transmute::<parking_lot::RwLockReadGuard<'_, T>, parking_lot::RwLockReadGuard<'static, T>>(own.read()) },
            _owned: own,
        }
    }

    /// Locks this `RwLock` with shared read access, blocking the current thread until it can be acquired.
    ///
    /// See `parking_lot::RwLock::read_recursive` for more details.
    pub fn lock_recursive(own: Arc<RwLock<T>>) -> Self {
        Self {
            // SAFETY: we cast to 'static only for storage, `lock` is dropped before `_owned`.
            lock: unsafe {
                mem::transmute::<parking_lot::RwLockReadGuard<'_, T>, parking_lot::RwLockReadGuard<'static, T>>(own.read_recursive())
            },
            _owned: own,
        }
    }

    /// Try lock owned.
    ///
    /// See `parking_lot::RwLock::try_read` for more details.
    pub fn try_lock(own: Arc<RwLock<T>>) -> Option<Self> {
        let lock = own.try_read()?;
        Some(Self {
            // SAFETY: we cast to 'static only for storage, `lock` is dropped before `_owned`.
            lock: unsafe { mem::transmute::<parking_lot::RwLockReadGuard<'_, T>, parking_lot::RwLockReadGuard<'static, T>>(lock) },
            _owned: own,
        })
    }

    /// Try lock owned.
    ///
    /// See `parking_lot::RwLock::try_read` for more details.
    pub fn try_lock_recursive(own: Arc<RwLock<T>>) -> Option<Self> {
        let lock = own.try_read_recursive()?;
        Some(Self {
            // SAFETY: we cast to 'static only for storage, `lock` is dropped before `_owned`.
            lock: unsafe { mem::transmute::<parking_lot::RwLockReadGuard<'_, T>, parking_lot::RwLockReadGuard<'static, T>>(lock) },
            _owned: own,
        })
    }

    /// Make a new `MappedRwLockReadGuardOwned` for a component of the locked data.
    ///
    /// This is an associated function that needs to be
    /// used as `RwLockReadGuardOwned::map(...)`. A method would interfere with methods of
    /// the same name on the contents of the locked data.
    pub fn map<O>(guard: Self, map: impl FnOnce(&T) -> &O) -> MappedRwLockReadGuardOwned<T, O> {
        MappedRwLockReadGuardOwned {
            lock: parking_lot::RwLockReadGuard::map(guard.lock, map),
            _owned: guard._owned,
        }
    }
}
impl<T> ops::Deref for RwLockReadGuardOwned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.lock.deref()
    }
}

/// Represents a read guard for an `Arc<RwLock<T>>` that owns a reference to it, mapped from another read guard.
pub struct MappedRwLockReadGuardOwned<T: 'static, O: 'static> {
    lock: parking_lot::MappedRwLockReadGuard<'static, O>,
    _owned: Arc<RwLock<T>>,
}
impl<T, O> MappedRwLockReadGuardOwned<T, O> {
    /// Make a new `MappedRwLockReadGuardOwned` for a component of the locked data.
    ///
    /// This is an associated function that needs to be
    /// used as `MappedRwLockReadGuardOwned::map(...)`. A method would interfere with methods of
    /// the same name on the contents of the locked data.
    pub fn map<O2>(guard: Self, map: impl FnOnce(&O) -> &O2) -> MappedRwLockReadGuardOwned<T, O2> {
        MappedRwLockReadGuardOwned {
            lock: parking_lot::MappedRwLockReadGuard::map(guard.lock, map),
            _owned: guard._owned,
        }
    }
}
impl<T, O> ops::Deref for MappedRwLockReadGuardOwned<T, O> {
    type Target = O;

    fn deref(&self) -> &Self::Target {
        self.lock.deref()
    }
}

/// Represents a read guard for an `Arc<RwLock<T>>` that owns a reference to it.
pub struct RwLockWriteGuardOwned<T: 'static> {
    lock: parking_lot::RwLockWriteGuard<'static, T>,
    _owned: Arc<RwLock<T>>,
}
impl<T> RwLockWriteGuardOwned<T> {
    /// Lock owned.
    ///
    /// See `parking_lot::RwLock::write` for more details.
    pub fn lock(own: Arc<RwLock<T>>) -> Self {
        Self {
            // SAFETY: we cast to 'static only for storage, `lock` is dropped before `_owned`.
            lock: unsafe { mem::transmute::<parking_lot::RwLockWriteGuard<'_, T>, parking_lot::RwLockWriteGuard<'static, T>>(own.write()) },
            _owned: own,
        }
    }

    /// Lock owned.
    ///
    /// See `parking_lot::RwLock::try_write` for more details.
    pub fn try_lock(own: Arc<RwLock<T>>) -> Option<Self> {
        let lock = own.try_write()?;
        Some(Self {
            // SAFETY: we cast to 'static only for storage, `lock` is dropped before `_owned`.
            lock: unsafe { mem::transmute::<parking_lot::RwLockWriteGuard<'_, T>, parking_lot::RwLockWriteGuard<'static, T>>(lock) },
            _owned: own,
        })
    }

    /// Make a new `MappedRwLockReadGuardOwned` for a component of the locked data.
    ///
    /// This is an associated function that needs to be
    /// used as `MappedRwLockReadGuardOwned::map(...)`. A method would interfere with methods of
    /// the same name on the contents of the locked data.
    pub fn map<O>(guard: Self, map: impl FnOnce(&mut T) -> &mut O) -> MappedRwLockWriteGuardOwned<T, O> {
        MappedRwLockWriteGuardOwned {
            lock: parking_lot::RwLockWriteGuard::map(guard.lock, map),
            _owned: guard._owned,
        }
    }
}
impl<T> ops::Deref for RwLockWriteGuardOwned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.lock.deref()
    }
}
impl<T> ops::DerefMut for RwLockWriteGuardOwned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.lock.deref_mut()
    }
}

/// Represents a write guard for an `Arc<RwLock<T>>` that owns a reference to it, mapped from another read guard.
pub struct MappedRwLockWriteGuardOwned<T: 'static, O: 'static> {
    lock: parking_lot::MappedRwLockWriteGuard<'static, O>,
    _owned: Arc<RwLock<T>>,
}
impl<T, O> MappedRwLockWriteGuardOwned<T, O> {
    /// Make a new `MappedRwLockWriteGuardOwned` for a component of the locked data.
    ///
    /// This is an associated function that needs to be
    /// used as `MappedRwLockWriteGuardOwned::map(...)`. A method would interfere with methods of
    /// the same name on the contents of the locked data.
    pub fn map<O2>(guard: Self, map: impl FnOnce(&mut O) -> &mut O2) -> MappedRwLockWriteGuardOwned<T, O2> {
        MappedRwLockWriteGuardOwned {
            lock: parking_lot::MappedRwLockWriteGuard::map(guard.lock, map),
            _owned: guard._owned,
        }
    }
}
impl<T, O> ops::Deref for MappedRwLockWriteGuardOwned<T, O> {
    type Target = O;

    fn deref(&self) -> &Self::Target {
        self.lock.deref()
    }
}
impl<T, O> ops::DerefMut for MappedRwLockWriteGuardOwned<T, O> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.lock.deref_mut()
    }
}

/// Read-only wrapper on an `Arc<RwLock<T>>` contextual value.
pub struct ReadOnlyRwLock<T>(Arc<RwLock<T>>);
impl<T> Clone for ReadOnlyRwLock<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T> ReadOnlyRwLock<T> {
    /// New.
    pub fn new(l: Arc<RwLock<T>>) -> Self {
        Self(l)
    }

    /// Locks this `RwLock` with shared read access, blocking the current thread until it can be acquired.
    ///
    /// See `parking_lot::RwLock::read` for more details.
    pub fn read(&self) -> parking_lot::RwLockReadGuard<'_, T> {
        self.0.read()
    }

    /// Locks this `RwLock` with shared read access, blocking the current thread until it can be acquired.
    ///
    /// Unlike `read`, this method is guaranteed to succeed without blocking if
    /// another read lock is held at the time of the call.
    ///
    /// See `parking_lot::RwLock::read_recursive` for more details.
    pub fn read_recursive(&self) -> parking_lot::RwLockReadGuard<'_, T> {
        self.0.read_recursive()
    }

    /// Attempts to acquire this `RwLock` with shared read access.
    ///
    /// See `parking_lot::RwLock::try_read` for more details.
    pub fn try_read(&self) -> Option<parking_lot::RwLockReadGuard<'_, T>> {
        self.0.try_read()
    }

    /// Attempts to acquire this `RwLock` with shared read access.
    ///
    /// See `parking_lot::RwLock::try_read_recursive` for more details.
    pub fn try_read_recursive(&self) -> Option<parking_lot::RwLockReadGuard<'_, T>> {
        self.0.try_read_recursive()
    }

    /// Gets if the read-only shared reference is to the same lock as `other`.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

///<span data-del-macro-root></span> Declares new app and context local variable.
///
/// # Examples
///
/// ```
/// # use zng_app_context::*;
/// context_local! {
///     /// A public documented value.
///     pub static FOO: u8 = 10u8;
///
///     // A private value.
///     static BAR: String = "Into!";
/// }
/// ```
///
/// # Default Value
///
/// All contextual values must have a fallback value that is used when no context is loaded.
///
/// The default value is instantiated once per app, the expression can be any static value that converts [`Into<T>`].
///
/// # Usage
///
/// After you declare the context local you can use it by loading a contextual value for the duration of a closure call.
///
/// ```
/// # use zng_app_context::*;
/// # use std::sync::Arc;
/// context_local! { static FOO: String = "default"; }
///
/// fn print_value() {
///     println!("value is {}!", FOO.get());
/// }
///
/// let _scope = LocalContext::start_app(AppId::new_unique());
///
/// let mut value = Some(Arc::new(String::from("other")));
/// FOO.with_context(&mut value, || {
///     print!("in context, ");
///     print_value();
/// });
///
/// print!("out of context, ");
/// print_value();
/// ```
///
/// The example above prints:
///
/// ```text
/// in context, value is other!
/// out of context, value is default!
/// ```
///
/// See [`ContextLocal<T>`] for more details.
#[macro_export]
macro_rules! context_local {
    ($(
        $(#[$meta:meta])*
        $vis:vis static $IDENT:ident : $T:ty = $init:expr;
    )+) => {$(
        $crate::context_local_impl! {
            $(#[$meta])*
            $vis static $IDENT: $T = $init;
        }
    )+};
}

#[doc(hidden)]
#[macro_export]
macro_rules! context_local_impl_single {
    ($(
        $(#[$meta:meta])*
        $vis:vis static $IDENT:ident : $T:ty = $init:expr;
    )+) => {$(
        $(#[$meta])*
        $vis static $IDENT: $crate::ContextLocal<$T> = {
            fn s() -> &'static dyn $crate::AppLocalImpl<$crate::ContextLocalData<$T>> {
                fn init() -> $T {
                    std::convert::Into::into($init)
                }
                $crate::hot_static! {
                    static IMPL: $crate::AppLocalConst<$crate::ContextLocalData<$T>> =
                    $crate::AppLocalConst::new(
                        $crate::ContextLocalData::new(init)
                    );
                }
                $crate::hot_static_ref!(IMPL)
            }
            $crate::ContextLocal::new(s)
        };
    )+};
}

#[doc(hidden)]
#[macro_export]
macro_rules! context_local_impl_multi {
    ($(
        $(#[$meta:meta])*
        $vis:vis static $IDENT:ident : $T:ty = $init:expr;
    )+) => {$(
        $(#[$meta])*
        $vis static $IDENT: $crate::ContextLocal<$T> = {
            fn s() -> &'static dyn $crate::AppLocalImpl<$crate::ContextLocalData<$T>> {
                fn init() -> $T {
                    std::convert::Into::into($init)
                }
                $crate::hot_static! {
                    static IMPL: $crate::AppLocalVec<$crate::ContextLocalData<$T>> =
                    $crate::AppLocalVec::new(
                        || $crate::ContextLocalData::new(init)
                    );
                }
                $crate::hot_static_ref!(IMPL)
            }
            $crate::ContextLocal::new(s)
        };
    )+};
}

#[cfg(feature = "multi_app")]
#[doc(hidden)]
pub use context_local_impl_multi as context_local_impl;

#[cfg(not(feature = "multi_app"))]
#[doc(hidden)]
pub use context_local_impl_single as context_local_impl;

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

/// Helper, runs a cleanup action once on drop.
pub struct RunOnDrop<F: FnOnce()>(Option<F>);
impl<F: FnOnce()> RunOnDrop<F> {
    /// New with closure that will run once on drop.
    pub fn new(clean: F) -> Self {
        RunOnDrop(Some(clean))
    }
}
impl<F: FnOnce()> Drop for RunOnDrop<F> {
    fn drop(&mut self) {
        if let Some(clean) = self.0.take() {
            clean();
        }
    }
}

fn panic_str<'s>(payload: &'s Box<dyn std::any::Any + Send + 'static>) -> &'s str {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s
    } else {
        "<unknown-panic-message-type>"
    }
}
