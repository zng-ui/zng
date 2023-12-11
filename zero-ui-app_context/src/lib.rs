//!: App execution context statics.

#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::{
    any::{Any, TypeId},
    cell::RefCell,
    fmt, mem, ops,
    sync::Arc,
    time::Duration,
};

use parking_lot::*;
use zero_ui_txt::Txt;
use zero_ui_unique_id::{unique_id_32, IdMap, IdSet};

unique_id_32! {
    /// Identifies an app instance.
    pub struct AppId;
}
zero_ui_unique_id::impl_unique_id_name!(AppId);
zero_ui_unique_id::impl_unique_id_fmt!(AppId);

impl serde::Serialize for AppId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let name = self.name();
        if name.is_empty() {
            use serde::ser::Error;
            return Err(S::Error::custom("cannot serialize unammed `AppId`"));
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
type LocalData = IdMap<TypeId, LocalValue>;

/// Represents an app lifetime, ends the app on drop.
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

/// Tracks the current execution context.
#[derive(Clone)]
pub struct LocalContext {
    data: LocalData,
}
impl fmt::Debug for LocalContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let app = self
            .data
            .get(&TypeId::of::<AppId>())
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
        Self { data: LocalData::new() }
    }

    /// Start an app scope in the current thread.
    pub fn start_app(id: AppId) -> AppScope {
        let valid = LOCAL.with_borrow_mut(|c| match c.entry(TypeId::of::<AppId>()) {
            hashbrown::hash_map::Entry::Occupied(_) => false,
            hashbrown::hash_map::Entry::Vacant(e) => {
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
        let valid = LOCAL.with_borrow_mut(|c| {
            if c.get(&TypeId::of::<AppId>())
                .map(|(v, _)| v.downcast_ref::<AppId>() == Some(&id))
                .unwrap_or(false)
            {
                Some(mem::take(&mut *c))
            } else {
                None
            }
        });

        if let Some(data) = valid {
            drop(data); // deinit
        } else {
            panic!("cannot end app from outside");
        }
    }

    /// Get the ID of the app that owns the current context.
    pub fn current_app() -> Option<AppId> {
        LOCAL.with_borrow(|c| {
            c.get(&TypeId::of::<AppId>())
                .map(|(v, _)| v.downcast_ref::<AppId>().unwrap())
                .copied()
        })
    }

    /// Register to run when the app deinits and all clones of the app context are dropped.
    pub fn register_cleanup(cleanup: impl FnOnce(AppId) + Send + 'static) {
        let id = Self::current_app().expect("no app in context");
        Self::register_cleanup_dyn(Box::new(move || cleanup(id)));
    }
    fn register_cleanup_dyn(cleanup: Box<dyn FnOnce() + Send>) {
        let cleanup = RunOnDrop::new(cleanup);

        type CleanupList = Vec<RunOnDrop<Box<dyn FnOnce() + Send>>>;
        LOCAL.with_borrow_mut(|c| {
            let c = c
                .entry(TypeId::of::<CleanupList>())
                .or_insert_with(|| (Arc::new(Mutex::new(CleanupList::new())), LocalValueKind::App));
            c.0.downcast_ref::<Mutex<CleanupList>>().unwrap().lock().push(cleanup);
        });
    }

    /// Capture a snapshot of the current context that can be restored in another thread to recreate
    /// the current context.
    ///
    /// Context locals modified after this capture are not included in the capture.
    pub fn capture() -> Self {
        Self {
            data: LOCAL.with_borrow(|c| c.clone()),
        }
    }

    /// Capture a snapshot of the current context that only includes `filter`.
    pub fn capture_filtered(filter: CaptureFilter) -> Self {
        match filter {
            CaptureFilter::None => Self::new(),
            CaptureFilter::All => Self::capture(),
            CaptureFilter::ContextVars { exclude } => {
                let mut data = LocalData::new();
                LOCAL.with_borrow(|c| {
                    for (k, (v, kind)) in c.iter() {
                        if kind.include_var() && !exclude.0.contains(k) {
                            data.insert(*k, (v.clone(), *kind));
                        }
                    }
                });
                Self { data }
            }
            CaptureFilter::ContextLocals { exclude } => {
                let mut data = LocalData::new();
                LOCAL.with_borrow(|c| {
                    for (k, (v, kind)) in c.iter() {
                        if kind.include_local() && !exclude.0.contains(k) {
                            data.insert(*k, (v.clone(), *kind));
                        }
                    }
                });
                Self { data }
            }
            CaptureFilter::Include(set) => {
                let mut data = LocalData::new();
                LOCAL.with_borrow(|c| {
                    for (k, v) in c.iter() {
                        if set.0.contains(k) {
                            data.insert(*k, v.clone());
                        }
                    }
                });
                Self { data }
            }
            CaptureFilter::Exclude(set) => {
                let mut data = LocalData::new();
                LOCAL.with_borrow(|c| {
                    for (k, v) in c.iter() {
                        if !set.0.contains(k) {
                            data.insert(*k, v.clone());
                        }
                    }
                });
                Self { data }
            }
        }
    }

    /// Collects a set of all the values in the context.
    pub fn value_set(&self) -> ContextValueSet {
        let mut set = ContextValueSet::new();
        LOCAL.with_borrow(|c| {
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
        let prev = LOCAL.with_borrow_mut(|c| mem::replace(c, data));
        let _restore = RunOnDrop::new(|| {
            self.data = LOCAL.with_borrow_mut(|c| mem::replace(c, prev));
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
            let prev = LOCAL.with_borrow_mut(|c| {
                let (mut base, over) = if over { (c.clone(), &self.data) } else { (self.data.clone(), &*c) };
                for (k, v) in over {
                    base.insert(*k, v.clone());
                }

                mem::replace(c, base)
            });
            let _restore = RunOnDrop::new(|| {
                LOCAL.with(|c| {
                    *c.borrow_mut() = prev;
                });
            });
            f()
        }
    }

    fn contains(key: TypeId) -> bool {
        LOCAL.with_borrow(|c| c.contains_key(&key))
    }

    fn get(key: TypeId) -> Option<LocalValue> {
        LOCAL.with_borrow(|c| c.get(&key).cloned())
    }

    fn set(key: TypeId, value: LocalValue) -> Option<LocalValue> {
        LOCAL.with_borrow_mut(|c| c.insert(key, value))
    }
    fn remove(key: TypeId) -> Option<LocalValue> {
        LOCAL.with_borrow_mut(|c| c.remove(&key))
    }

    fn with_value_ctx<T: Send + Sync + 'static, R>(
        key: &'static ContextLocal<T>,
        kind: LocalValueKind,
        value: &mut Option<Arc<T>>,
        f: impl FnOnce() -> R,
    ) -> R {
        let key = key.key();
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

        f()
    }

    fn with_default_ctx<T: Send + Sync + 'static, R>(key: &'static ContextLocal<T>, f: impl FnOnce() -> R) -> R {
        let key = key.key();
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
    static LOCAL: RefCell<LocalData> = const {
        RefCell::new(LocalData::new())
    };
}

/// A local context that can be loaded cheaply.
pub struct FullLocalContext(LocalContext);
impl FullLocalContext {}
impl ops::Deref for FullLocalContext {
    type Target = LocalContext;

    fn deref(&self) -> &Self::Target {
        &self.0
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

    fn read_impl(&'static self, read: RwLockReadGuard<'static, Option<T>>) -> MappedRwLockReadGuard<T> {
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

    fn write_impl(&'static self, mut write: RwLockWriteGuard<'static, Option<T>>) -> MappedRwLockWriteGuard<T> {
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

    fn read_impl(&'static self, read: RwLockReadGuard<'static, Vec<(AppId, T)>>) -> MappedRwLockReadGuard<T> {
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

    fn write_impl(&'static self, mut write: RwLockWriteGuard<'static, Vec<(AppId, T)>>) -> MappedRwLockWriteGuard<T> {
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
    fn read(&'static self) -> MappedRwLockReadGuard<T>;
    fn try_read(&'static self) -> Option<MappedRwLockReadGuard<T>>;
    fn write(&'static self) -> MappedRwLockWriteGuard<T>;
    fn try_write(&'static self) -> Option<MappedRwLockWriteGuard<T>>;
}

impl<T: Send + Sync + 'static> AppLocalImpl<T> for AppLocalVec<T> {
    fn read(&'static self) -> MappedRwLockReadGuard<T> {
        self.read_impl(self.value.read_recursive())
    }

    fn try_read(&'static self) -> Option<MappedRwLockReadGuard<T>> {
        Some(self.read_impl(self.value.try_read_recursive()?))
    }

    fn write(&'static self) -> MappedRwLockWriteGuard<T> {
        self.write_impl(self.value.write())
    }

    fn try_write(&'static self) -> Option<MappedRwLockWriteGuard<T>> {
        Some(self.write_impl(self.value.try_write()?))
    }
}
impl<T: Send + Sync + 'static> AppLocalImpl<T> for AppLocalOption<T> {
    fn read(&'static self) -> MappedRwLockReadGuard<T> {
        self.read_impl(self.value.read_recursive())
    }

    fn try_read(&'static self) -> Option<MappedRwLockReadGuard<T>> {
        Some(self.read_impl(self.value.try_read_recursive()?))
    }

    fn write(&'static self) -> MappedRwLockWriteGuard<T> {
        self.write_impl(self.value.write())
    }

    fn try_write(&'static self) -> Option<MappedRwLockWriteGuard<T>> {
        Some(self.write_impl(self.value.try_write()?))
    }
}
impl<T: Send + Sync + 'static> AppLocalImpl<T> for AppLocalConst<T> {
    fn read(&'static self) -> MappedRwLockReadGuard<T> {
        RwLockReadGuard::map(self.value.read(), |l| l)
    }

    fn try_read(&'static self) -> Option<MappedRwLockReadGuard<T>> {
        Some(RwLockReadGuard::map(self.value.try_read()?, |l| l))
    }

    fn write(&'static self) -> MappedRwLockWriteGuard<T> {
        RwLockWriteGuard::map(self.value.write(), |l| l)
    }

    fn try_write(&'static self) -> Option<MappedRwLockWriteGuard<T>> {
        Some(RwLockWriteGuard::map(self.value.try_write()?, |l| l))
    }
}

/// An app local storage.
///
/// This is similar to [`std::thread::LocalKey`], but works across all threads of the app.
///
/// Use the [`app_local!`] macro to declare a static variable in the same style as [`thread_local!`].
///
/// Note that an app local can only be used if an app is running in the thread, if no app is running read and write **will panic**.
pub struct AppLocal<T: Send + Sync + 'static> {
    inner: &'static dyn AppLocalImpl<T>,
}
impl<T: Send + Sync + 'static> AppLocal<T> {
    #[doc(hidden)]
    pub const fn new(inner: &'static dyn AppLocalImpl<T>) -> Self {
        AppLocal { inner }
    }

    /// Read lock the value associated with the current app.
    ///
    /// Initializes the default value for the app if this is the first read.
    ///
    /// # Panics
    ///
    /// Panics if no app is running.
    #[inline]
    pub fn read(&'static self) -> MappedRwLockReadGuard<T> {
        self.inner.read()
    }

    /// Try read lock the value associated with the current app.
    ///
    /// Initializes the default value for the app if this is the first read.
    ///
    /// # Panics
    ///
    /// Panics if no app is running.
    #[inline]
    pub fn try_read(&'static self) -> Option<MappedRwLockReadGuard<T>> {
        self.inner.try_read()
    }

    /// Write lock the value associated with the current app.
    ///
    /// Initializes the default value for the app if this is the first read.
    ///
    /// # Panics
    ///
    /// Panics if no app is running.
    #[inline]
    pub fn write(&'static self) -> MappedRwLockWriteGuard<T> {
        self.inner.write()
    }

    /// Try to write lock the value associated with the current app.
    ///
    /// Initializes the default value for the app if this is the first read.
    ///
    /// # Panics
    ///
    /// Panics if no app is running.
    pub fn try_write(&'static self) -> Option<MappedRwLockWriteGuard<T>> {
        self.inner.try_write()
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
    pub fn read_map<O>(&'static self, map: impl FnOnce(&T) -> &O) -> MappedRwLockReadGuard<O> {
        MappedRwLockReadGuard::map(self.read(), map)
    }

    /// Try to create a read lock and `map` it to a sub-value.
    #[inline]
    pub fn try_wread_map<O>(&'static self, map: impl FnOnce(&T) -> &O) -> Option<MappedRwLockReadGuard<O>> {
        let lock = self.try_read()?;
        Some(MappedRwLockReadGuard::map(lock, map))
    }

    /// Create a write lock and `map` it to a sub-value.
    #[inline]
    pub fn write_map<O>(&'static self, map: impl FnOnce(&mut T) -> &mut O) -> MappedRwLockWriteGuard<O> {
        MappedRwLockWriteGuard::map(self.write(), map)
    }

    /// Try to create a write lock and `map` it to a sub-value.
    #[inline]
    pub fn try_write_map<O>(&'static self, map: impl FnOnce(&mut T) -> &mut O) -> Option<MappedRwLockWriteGuard<O>> {
        let lock = self.try_write()?;
        Some(MappedRwLockWriteGuard::map(lock, map))
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
/// If the crate is compiled with the `multi_app` feature a different internal implementation is used that supports multiple
/// apps, either running in parallel in different threads or one after the other. This backing implementation has some small overhead,
/// but usually you only want multiple app instances per-process when running tests.
///
/// The lifetime of `multi_app` locals is also more limited, trying to use an app-local before starting to build an app will panic,
/// the app-local value will be dropped when the app is dropped. Without the `multi_app` feature the app-locals can be used at
/// any point before or after the app lifetime, values are not explicitly dropped, just unloaded with the process.
///
/// # Const
///
/// The initialization expression can be wrapped in a `const { .. }` block, if the `multi_app` feature is **not** enabled
/// a faster implementation is used that is equivalent to a direct `static LOCAL: RwLock<T>` in terms of performance.
///
/// Note that this syntax is available even if the `multi_app` feature is enabled, the expression must be const either way,
/// but with the feature the same dynamic implementation is used.
///
/// Note that `const` initialization does not automatically convert the value into the static type.
///
/// # Examples
///
/// ```
/// # use zero_ui_app_context::*;
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
/// Note that app locals can only be used when an app exists in the thread, as soon as an app starts building a new app scope is created,
/// the app scope is the last thing that is "dropped" after the app exits or the app builder is dropped.
#[macro_export]
macro_rules! app_local {
    ($(
        $(#[$meta:meta])*
        $vis:vis static $IDENT:ident : $T:ty = $(const { $init_const:expr })? $($init:expr)?;
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
            static IMPL: $crate::AppLocalConst<$T> = $crate::AppLocalConst::new($init);
            $crate::AppLocal::new(&IMPL)
        };
    };
    (
        $(#[$meta:meta])*
        $vis:vis static $IDENT:ident : $T:ty = $init:expr;
    ) => {
        $(#[$meta])*
        $vis static $IDENT: $crate::AppLocal<$T> = {
            fn init() -> $T {
                std::convert::Into::into($init)
            }
            static IMPL: $crate::AppLocalOption<$T> = $crate::AppLocalOption::new(init);
            $crate::AppLocal::new(&IMPL)
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
            const fn init() -> $T {
                $init
            }
            static IMPL: $crate::AppLocalVec<$T> = $crate::AppLocalVec::new(init);
            $crate::AppLocal::new(&IMPL)
        };
    };
    (
        $(#[$meta:meta])*
        $vis:vis static $IDENT:ident : $T:ty = $init:expr;
    ) => {
        $(#[$meta])*
        $vis static $IDENT: $crate::AppLocal<$T> = {
            fn init() -> $T {
                std::convert::Into::into($init)
            }
            static IMPL: $crate::AppLocalVec<$T> = $crate::AppLocalVec::new(init);
            $crate::AppLocal::new(&IMPL)
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
    key: fn() -> TypeId,
    default_init: fn() -> T,
    default_value: Option<Arc<T>>,
}
impl<T: Send + Sync + 'static> ContextLocalData<T> {
    #[doc(hidden)]
    pub const fn new(key: fn() -> TypeId, default_init: fn() -> T) -> Self {
        Self {
            key,
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
    pub const fn new(storage: &'static dyn AppLocalImpl<ContextLocalData<T>>) -> Self {
        Self {
            data: AppLocal::new(storage),
        }
    }

    fn key(&'static self) -> TypeId {
        (self.data.read().key)()
    }

    /// Calls `f` with the `value` loaded in context.
    ///
    /// The `value` is moved in context, `f` is called, then restores the `value`.
    ///
    /// # Panics
    ///
    /// Panics if `value` is `None`.
    pub fn with_context<R>(&'static self, value: &mut Option<Arc<T>>, f: impl FnOnce() -> R) -> R {
        #[cfg(dyn_closure)]
        let f: Box<dyn FnOnce() -> R> = Box::new(f);
        LocalContext::with_value_ctx(self, LocalValueKind::Local, value, f)
    }

    /// Same as [`with_context`], but `value` represents a variable.
    ///
    /// [`with_context`]: Self::with_context
    pub fn with_context_var<R>(&'static self, value: &mut Option<Arc<T>>, f: impl FnOnce() -> R) -> R {
        #[cfg(dyn_closure)]
        let f: Box<dyn FnOnce() -> R> = Box::new(f);
        LocalContext::with_value_ctx(self, LocalValueKind::Var, value, f)
    }

    /// Calls `f` with the `value` loaded in context.
    pub fn with_context_value<R>(&'static self, value: T, f: impl FnOnce() -> R) -> R {
        self.with_context(&mut Some(Arc::new(value)), f)
    }

    /// Calls `f` with the `value` loaded in context.
    ///
    /// The `value` is moved in context, `f` is called, then restores the `value`. A clone is restored if
    /// the value is still shared when `f` returns.
    ///
    /// # Panics
    ///
    /// Panics if `value` is `None`.
    pub fn with_context_opt<R>(&'static self, value: &mut Option<T>, f: impl FnOnce() -> R) -> R
    where
        T: Clone,
    {
        let mut val = value.take().map(Arc::new);
        let r = self.with_context(&mut val, f);
        match Arc::try_unwrap(val.unwrap()) {
            Ok(val) => *value = Some(val),
            Err(arc) => *value = Some(arc.as_ref().clone()),
        }
        r
    }

    /// Calls `f` with no value loaded in context.
    pub fn with_default<R>(&'static self, f: impl FnOnce() -> R) -> R {
        #[cfg(dyn_closure)]
        let f: Box<dyn FnOnce() -> R> = Box::new(f);
        LocalContext::with_default_ctx(self, f)
    }

    /// Gets if no value is set in the context.
    pub fn is_default(&'static self) -> bool {
        let cl = self.data.read();
        !LocalContext::contains((cl.key)())
    }

    /// Clone a reference to the current value in the context or the default value.
    pub fn get(&'static self) -> Arc<T> {
        let cl = self.data.read();
        match LocalContext::get((cl.key)()) {
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
        match LocalContext::get((cl.key)()) {
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

///<span data-del-macro-root></span> Declares new app and context local variable.
///
/// # Examples
///
/// ```
/// # use zero_ui_app_context::*;
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
/// After you declare the contextual value you can use it by loading a context, calling a closure and inside it *visiting* the value.
///
/// ```
/// # use zero_ui_app_context::*;
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
            fn init() -> $T {
                std::convert::Into::into($init)
            }
            fn key() -> std::any::TypeId {
                struct Key { }
                std::any::TypeId::of::<Key>()
            }
            static IMPL: $crate::AppLocalConst<$crate::ContextLocalData<$T>> =
                $crate::AppLocalConst::new(
                    $crate::ContextLocalData::new(key, init)
                );
            $crate::ContextLocal::new(&IMPL)
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
            fn init() -> $T {
                std::convert::Into::into($init)
            }
            fn key() -> std::any::TypeId {
                struct Key { }
                std::any::TypeId::of::<Key>()
            }
            static IMPL: $crate::AppLocalVec<$crate::ContextLocalData<$T>> =
            $crate::AppLocalVec::new(
                || $crate::ContextLocalData::new(key, init)
            );
            $crate::ContextLocal::new(&IMPL)
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
    /// Don't capture any.
    None,

    /// Capture all.
    All,
    /// Capture all variables not excluded, and no [`context_local!`].
    ContextVars {
        /// Vars to not include.
        exclude: ContextValueSet,
    },
    /// Capture all [`context_local!`] not excluded, no context variables.
    ContextLocals {
        /// Locals to not include.
        exclude: ContextValueSet,
    },

    /// Capture only this set.
    Include(ContextValueSet),

    /// Capture all except this set.
    Exclude(ContextValueSet),
}

/// Provides an identifying key for a context local value.
///
/// Implemented by all [`ContextLocal<T>`] already, only implement this for context local thin wrappers.
pub trait ContextLocalKeyProvider {
    /// Gets the key.
    fn context_local_key(&'static self) -> TypeId;
}
impl<T: Send + Sync + 'static> ContextLocalKeyProvider for ContextLocal<T> {
    fn context_local_key(&'static self) -> TypeId {
        self.key()
    }
}

/// Identifies a selection of [`LocalContext`] values.
#[derive(Default, Clone, PartialEq, Eq)]
pub struct ContextValueSet(IdSet<TypeId>);
impl ContextValueSet {
    /// New empty.
    pub const fn new() -> Self {
        Self(crate::IdSet::new())
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
