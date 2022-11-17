use std::{cell::RefCell, fmt, sync::Arc};

use crate::{
    crate_util::{IdNameError, NameIdMap},
    text::Text,
};

use parking_lot::{MappedRwLockReadGuard, MappedRwLockWriteGuard, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};

unique_id_32! {
    /// Identifies an [`App`] instance.
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

pub(super) struct AppScope(Arc<AppScopeData>);
impl AppScope {
    pub(super) fn new_unique() -> Self {
        Self(Arc::new(AppScopeData {
            id: AppId::new_unique(),
            cleanup: Mutex::new(vec![]),
        }))
    }

    pub(super) fn load_in_thread(&self) {
        CURRENT_SCOPE.with(|s| {
            if let Some(other) = s.borrow_mut().replace(AppScope(self.0.clone())) {
                tracing::error!("displaced app `{:?}` in thread {:?}", other.0.id, std::thread::current())
            }
        })
    }

    pub(super) fn unload_in_thread(&self) {
        CURRENT_SCOPE.with(|s| {
            if s.borrow_mut().take().is_none() {
                tracing::error!("no app loaded in thread {:?}", std::thread::current());
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
        let id = self.0.id;
        for c in self.0.cleanup.lock().drain(..) {
            c(id);
        }
    }
}

thread_local! {
    static CURRENT_SCOPE: RefCell<Option<AppScope>> = RefCell::new(None);
}

static NO_APP_ID: StaticAppId = StaticAppId::new_unique();

/// An app local storage.
///
/// This is similar to [`std::thread::LocalKey`], but works across all UI threads of the [`App::current_id`].
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
    pub fn read(&'static self) -> MappedRwLockReadGuard<T> {
        let id = AppScope::current_id().unwrap_or_else(|| NO_APP_ID.get());

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
    pub fn write(&'static self) -> MappedRwLockWriteGuard<T> {
        let id = AppScope::current_id().unwrap_or_else(|| NO_APP_ID.get());

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
}

///<span data-del-macro-root></span> Declares new app local variable.
///
/// See [`AppLocal<T>`] for more details.
#[macro_export]
macro_rules! app_local {
    (
        $(#[$meta:meta])*
        $vis:vis static $IDENT:ident : $T:ty = $init:expr;
    ) => {
        $(#[$meta])*
        $vis static $IDENT: $crate::app::AppLocal<$T> = {
            fn init() -> $T {
                $init
            }
            $crate::app::AppLocal::new(init)
        };
    };
}
#[doc(inline)]
pub use app_local;
