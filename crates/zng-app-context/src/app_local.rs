use parking_lot::{MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::{AppId, LocalContext};

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
///
/// [`app_local!`]: crate::app_local!
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
pub struct AppLocalId(pub(crate) usize);
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

use std::{fmt, time::Duration};

#[cfg(feature = "multi_app")]
#[doc(hidden)]
pub use app_local_impl_multi as app_local_impl;
#[cfg(not(feature = "multi_app"))]
#[doc(hidden)]
pub use app_local_impl_single as app_local_impl;
