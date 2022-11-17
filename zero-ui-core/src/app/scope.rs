use std::{cell::RefCell, fmt, mem, sync::Arc, thread::ThreadId};

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

use crate::{
    crate_util::{IdNameError, NameIdMap, RunOnDrop},
    text::Text,
};

/// Represents an [`AppLocal<T>`] value that can be temporarily overridden in a context.
///
/// The *context* works across threads, as long as the threads are instrumented using [`ThreadContext`].
pub struct ContextLocal<T: Send + Sync + 'static> {
    data: AppLocal<Vec<(ThreadId, T)>>,
    default: RwLock<Option<T>>,
    init: fn() -> T,
}
impl<T: Send + Sync + 'static> ContextLocal<T> {
    #[doc(hidden)]
    pub const fn new(init: fn() -> T) -> Self {
        Self {
            data: AppLocal::new(Vec::new),
            default: RwLock::new(None),
            init,
        }
    }

    /// Calls `f` with the `value` loaded in context.
    pub fn with_override<R>(&'static self, value: &mut Option<T>, f: impl FnOnce() -> R) -> R {
        let new_value = value.take().expect("no override provided");
        let thread_id = std::thread::current().id();

        let i;
        let prev_value;

        let mut write = self.data.write();
        if let Some(idx) = write.iter_mut().position(|(id, _)| *id == thread_id) {
            // already contextualized in this thread

            i = idx;
            prev_value = mem::replace(&mut write[i].1, new_value);

            drop(write);

            let _restore = RunOnDrop::new(move || {
                let mut write = self.data.write();
                *value = Some(mem::replace(&mut write[i].1, prev_value));
            });

            f()
        } else {
            // first contextualization in this thread
            write.push((thread_id, new_value));

            let _restore = RunOnDrop::new(move || {
                let mut write = self.data.write();
                let i = write.iter_mut().position(|(id, _)| *id == thread_id).unwrap();
                *value = Some(write.swap_remove(i).1);
            });

            f()
        }
    }

    /// Read the contextual value.
    pub fn read(&'static self) -> MappedRwLockReadGuard<T> {
        let read = self.data.read();
        for thread_id in ThreadContext::capture().context() {
            if let Some(i) = read.iter().position(|(id, _)| id == thread_id) {
                // contextualized in thread or task parent thread.
                return MappedRwLockReadGuard::map(read, move |v| &v[i].1);
            }
        }
        drop(read);

        let read = self.default.read_recursive();
        if read.is_some() {
            return RwLockReadGuard::map(read, move |v| v.as_ref().unwrap());
        }
        drop(read);

        let mut write = self.default.write();
        *write = Some((self.init)());
        let read = RwLockWriteGuard::downgrade(write);
        RwLockReadGuard::map(read, move |v| v.as_ref().unwrap())
    }
}

/// Tracks current thread and current task *owner* threads.
pub struct ThreadContext {
    context: Vec<ThreadId>,
}
thread_local! {
    static THREAD_CONTEXT: RefCell<Vec<ThreadId>> = RefCell::new(vec![]);
}
impl ThreadContext {
    /// The current thread, followed by the thread that logically *owns* the current executing task, recursive over nested tasks.
    pub fn context(&self) -> &[ThreadId] {
        &self.context
    }

    /// Capture the current context.
    ///
    /// A context must be captured and recorded by tasks that may run in other threads, the context must be loaded
    /// in the other thread using [`with_context`].
    ///
    /// [`with_context`]: ThreadContext::with_context
    pub fn capture() -> ThreadContext {
        THREAD_CONTEXT.with(|s| {
            let mut r = ThreadContext {
                context: s.borrow().clone(),
            };
            let current = std::thread::current().id();
            if r.context.last() != Some(&current) {
                r.context.push(current);
            }
            r
        })
    }

    /// Runs `f` within the context.
    ///
    /// This method must be used every time there is the possibility that the caller is running in a different thread.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::thread;
    /// use zero_ui_core::app::ThreadContext;
    ///
    /// let outer_id = thread::current().id();
    /// let ctx = ThreadContext::capture();
    ///
    /// assert_eq!(&[outer_id], ctx.context());
    ///
    /// thread::spawn(move || ctx.with_context(move || {
    ///     let inner_id = thread::current().id();
    ///     let ctx = ThreadContext::capture();
    ///
    ///     assert_eq!(&[inner_id, outer_id], ctx.context());
    /// })).join();
    /// ```
    pub fn with_context<R>(&self, f: impl FnOnce() -> R) -> R {
        let prev = THREAD_CONTEXT.with(|s| mem::replace(&mut *s.borrow_mut(), self.context.clone()));
        let _restore = RunOnDrop::new(move || THREAD_CONTEXT.with(|s| *s.borrow_mut() = prev));
        f()
    }
}
