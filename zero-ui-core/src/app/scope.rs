use std::{cell::RefCell, fmt, sync::Arc};

use crate::{
    crate_util::{IdNameError, NameIdMap},
    text::Text,
};

use parking_lot::{MappedRwLockReadGuard, MappedRwLockWriteGuard, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};

unique_id_32! {
    /// Identifies an [`App`] instance.
    ///
    /// You can get the current app ID from [`App::current_id`].
    ///
    /// [`App`]: crate::app::App
    /// [`App::current_id`]: crate::app::App::current_id
    pub struct AppId;
}
impl AppId {
    fn name_map() -> parking_lot::MappedMutexGuard<'static, NameIdMap<Self>> {
        static NAME_MAP: Mutex<Option<NameIdMap<AppId>>> = parking_lot::const_mutex(None);
        parking_lot::MutexGuard::map(NAME_MAP.lock(), |m| m.get_or_insert_with(NameIdMap::new))
    }

    /// Returns the name associated with the id or `""`.
    pub fn name(self) -> Text {
        Self::name_map().get_name(self)
    }

    /// Associate a `name` with the id, if it is not named.
    ///
    /// If the `name` is already associated with a different id, returns the [`NameUsed`] error.
    /// If the id is already named, with a name different from `name`, returns the [`AlreadyNamed`] error.
    /// If the `name` is an empty string or already is the name of the id, does nothing.
    ///
    /// [`NameUsed`]: IdNameError::NameUsed
    /// [`AlreadyNamed`]: IdNameError::AlreadyNamed
    pub fn set_name(self, name: impl Into<Text>) -> Result<(), IdNameError<Self>> {
        Self::name_map().set(name.into(), self)
    }
}
impl fmt::Debug for AppId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.name();
        if f.alternate() {
            f.debug_struct("AppId")
                .field("id", &self.get())
                .field("sequential", &self.sequential())
                .field("name", &name)
                .finish()
        } else if !name.is_empty() {
            write!(f, r#"AppId("{name}")"#)
        } else {
            write!(f, "AppId({})", self.sequential())
        }
    }
}

struct AppScopeData {
    id: AppId,
    cleanup: Mutex<Vec<Box<dyn FnOnce(AppId) + Send + Sync>>>,
}

pub(crate) struct AppScope(Arc<AppScopeData>);
impl AppScope {
    pub(crate) fn new_loaded() -> Self {
        let me = Self(Arc::new(AppScopeData {
            id: AppId::new_unique(),
            cleanup: Mutex::new(vec![]),
        }));
        me.load_in_thread();
        me
    }

    pub(crate) fn load_in_thread(&self) {
        CURRENT_SCOPE.with(|s| {
            if let Some(other) = s.borrow_mut().replace(AppScope(self.0.clone())) {
                if other.0.id != self.0.id {
                    tracing::error!("displaced app `{:?}` in thread {:?}", other.0.id, std::thread::current())
                }
            }
        })
    }

    pub(crate) fn unload_in_thread(&self) {
        CURRENT_SCOPE.with(|s| {
            let mut s = s.borrow_mut();
            if let Some(other) = s.take() {
                if other.0.id != self.0.id {
                    tracing::error!(
                        "tried to unload wrong scope in thread {:?}, expected scope {:?}, but was {:?}",
                        std::thread::current(),
                        self.0.id,
                        other.0.id
                    );
                    *s = Some(other);
                }
                drop(s);
            }
        })
    }

    pub(super) fn current_id() -> Option<AppId> {
        CURRENT_SCOPE.with(|s| s.borrow().as_ref().map(|s| s.0.id))
    }

    pub(super) fn register_cleanup(cleanup: Box<dyn FnOnce(AppId) + Send + Sync>) -> bool {
        CURRENT_SCOPE.with(|s| {
            if let Some(s) = &*s.borrow() {
                s.0.cleanup.lock().push(cleanup);
                true
            } else {
                false
            }
        })
    }
}
impl Drop for AppScope {
    fn drop(&mut self) {
        self.unload_in_thread();

        let id = self.0.id;
        for c in self.0.cleanup.lock().drain(..) {
            c(id);
        }
    }
}

thread_local! {
    static CURRENT_SCOPE: RefCell<Option<AppScope>> = RefCell::new(None);
}

/// An app local storage.
///
/// This is similar to [`std::thread::LocalKey`], but works across all threads of the app.
///
/// Use the [`app_local!`] macro to declare a static variable in the same style as [`thread_local!`].
///
/// Note that an app local can only be used if [`App::is_running`] in the thread, if no app is running read and write **will panic**.
///
/// [`App::is_running`]: crate::app::App::is_running
pub struct AppLocal<T: Send + Sync + 'static> {
    value: RwLock<Vec<(AppId, T)>>,
    init: fn() -> T,
}
impl<T: Send + Sync + 'static> AppLocal<T> {
    #[doc(hidden)]
    pub const fn new(init: fn() -> T) -> Self {
        AppLocal {
            value: RwLock::new(vec![]),
            init,
        }
    }

    fn cleanup(&'static self, id: AppId) {
        let mut write = self.value.write();
        if let Some(i) = write.iter().position(|(s, _)| *s == id) {
            write.swap_remove(i);
        }
    }

    /// Read lock the value associated with the current app.
    ///
    /// Initializes the default value for the app if this is the first read.
    ///
    /// # Panics
    ///
    /// Panics if no app is running, see [`App::is_running`] for more details.
    ///
    /// [`App::is_running`]: crate::app::App::is_running
    pub fn read(&'static self) -> MappedRwLockReadGuard<T> {
        let id = AppScope::current_id().expect("no app running, `app_local` can only be accessed inside apps");

        {
            let read = self.value.read_recursive();
            if let Some(i) = read.iter().position(|(s, _)| *s == id) {
                return RwLockReadGuard::map(read, |v| &v[i].1);
            }
        }

        let mut write = self.value.write();
        if write.iter().any(|(s, _)| *s == id) {
            drop(write);
            return self.read();
        }

        let value = (self.init)();
        let i = write.len();
        write.push((id, value));

        AppScope::register_cleanup(Box::new(move |id| self.cleanup(id)));

        let read = RwLockWriteGuard::downgrade(write);

        RwLockReadGuard::map(read, |v| &v[i].1)
    }

    /// Write lock the value associated with the current app.
    ///
    /// Initializes the default value for the app if this is the first read.
    ///
    /// # Panics
    ///
    /// Panics if no app is running, see [`App::is_running`] for more details.
    ///
    /// [`App::is_running`]: crate::app::App::is_running
    pub fn write(&'static self) -> MappedRwLockWriteGuard<T> {
        let id = AppScope::current_id().expect("no app running, `app_local` can only be accessed inside apps");

        let mut write = self.value.write();

        if let Some(i) = write.iter().position(|(s, _)| *s == id) {
            return RwLockWriteGuard::map(write, |v| &mut v[i].1);
        }

        let value = (self.init)();
        let i = write.len();
        write.push((id, value));

        AppScope::register_cleanup(Box::new(move |id| self.cleanup(id)));

        RwLockWriteGuard::map(write, |v| &mut v[i].1)
    }

    /// Get a clone of the value.
    pub fn get(&'static self) -> T
    where
        T: Clone,
    {
        self.read().clone()
    }

    /// Set the value.
    pub fn set(&'static self, value: T) {
        *self.write() = value;
    }
}

///<span data-del-macro-root></span> Declares new app local variable.
///
/// See [`AppLocal<T>`] for more details.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::app::*;
/// app_local! {
///     /// A public documented value.
///     pub static FOO: u8 = 10u8;
///
///     // A private value.
///     static BAR: String = "Into!";
/// }
///
/// let app = App::blank();
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
        $vis:vis static $IDENT:ident : $T:ty = $init:expr;
    )+) => {$(
        $(#[$meta])*
        $vis static $IDENT: $crate::app::AppLocal<$T> = {
            fn init() -> $T {
                std::convert::Into::into($init)
            }
            $crate::app::AppLocal::new(init)
        };
    )+};
}
#[doc(inline)]
pub use app_local;
