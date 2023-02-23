use std::{any::Any, cell::RefCell, fmt, marker::PhantomData, mem, sync::Arc, thread::ThreadId};

use parking_lot::*;

use crate::{
    context::{InfoContext, LayoutContext, MeasureContext, RenderContext, WidgetContext, WidgetUpdates},
    crate_util::{IdNameError, NameIdMap, RunOnDrop},
    event::EventUpdate,
    render::{FrameBuilder, FrameUpdate},
    text::Text,
    ui_node,
    units::{self, TimeUnits},
    widget_info::{WidgetInfoBuilder, WidgetLayout, WidgetMeasure},
    widget_instance::UiNode,
};

unique_id_32! {
    /// Identifies an app instance.
    ///
    /// You can get the current app ID from [`App::current_id`].
    ///
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

struct ThreadOwnerApp {
    id: AppId,
    cleanup: Mutex<Vec<Box<dyn FnOnce(AppId) + Send>>>,
}
impl Drop for ThreadOwnerApp {
    fn drop(&mut self) {
        for c in self.cleanup.get_mut().drain(..) {
            c(self.id);
        }
    }
}

pub(crate) struct AppScope {
    id: AppId,
    _not_send: PhantomData<std::rc::Rc<()>>, // drop must be called in the same thread
}
impl Drop for AppScope {
    fn drop(&mut self) {
        ThreadContext::end_app(self.id)
    }
}

/// Tracks current thread and current task *owner* threads.
pub struct ThreadContext {
    app: Option<Arc<ThreadOwnerApp>>,
    context: Vec<ThreadId>,
}
thread_local! {
    static THREAD_CONTEXT: RefCell<ThreadContext> = RefCell::new(ThreadContext {
        app: None,
        context: vec![],
    });
}
impl ThreadContext {
    fn clone(&self) -> Self {
        Self {
            app: self.app.clone(),
            context: self.context.clone(),
        }
    }

    #[must_use]
    pub(crate) fn start_app(id: AppId) -> AppScope {
        let r = THREAD_CONTEXT.with(|s| {
            let mut s = s.borrow_mut();
            let t_id = std::thread::current().id();
            if let Some(app) = &s.app {
                return Err(format!(
                    "thread `{:?}` already owned by `{:?}`, run `{:?}` in a new thread",
                    t_id, app.id, id
                ));
            }
            if !s.context.is_empty() {
                return Err(format!("thread `{t_id:?}` already contextualized, run `{id:?}` in a new thread"));
            }
            s.app = Some(Arc::new(ThreadOwnerApp {
                id,
                cleanup: Mutex::new(vec![]),
            }));
            s.context.push(t_id);

            Ok(())
        });
        r.unwrap();

        AppScope {
            id,
            _not_send: PhantomData,
        }
    }

    fn end_app(id: AppId) {
        let r = THREAD_CONTEXT.with(|s| {
            let t_id = std::thread::current().id();
            let mut s = s.borrow_mut();
            if let Some(app) = &s.app {
                if app.id != id {
                    return Err(format!(
                        "can only end `{id:?}` in same thread it started, currently in `{:?}`",
                        app.id
                    ));
                }
                if let Some(id) = s.context.first() {
                    if id != &t_id {
                        return Err(format!(
                            "can only end `{id:?}` in same thread it started, currently in `{:?}`",
                            t_id
                        ));
                    }
                }
                if s.context.len() != 1 {
                    return Err(format!("can only end `{id:?}` at the root context"));
                }
                if s.context[0] != t_id {
                    return Err(format!("can only end `{id:?}` at the same root thread `{t_id:?}`"));
                }

                s.context.clear();
                Ok(s.app.take().unwrap())
            } else {
                Err(format!("thread not owned by `{id:?}`"))
            }
        });
        let _app = r.unwrap(); // maybe run cleanup
    }

    fn register_cleanup(cleanup: Box<dyn FnOnce(AppId) + Send>) {
        let r = THREAD_CONTEXT.with(|s| {
            let s = s.borrow();
            if let Some(app) = &s.app {
                app.cleanup.lock().push(cleanup);
                Ok(())
            } else {
                Err(format!("thread `{:?}` not owned by any app", std::thread::current().id()))
            }
        });
        r.unwrap();
    }

    /// The current app.
    pub fn app(&self) -> Option<AppId> {
        self.app.as_ref().map(|a| a.id)
    }

    /// The current thread, followed by the thread that logically *owns* the current executing task, recursive over nested tasks.
    pub fn context(&self) -> &[ThreadId] {
        &self.context
    }

    /// The app that owns the current thread.
    pub fn current_app() -> Option<AppId> {
        THREAD_CONTEXT.with(|s| s.borrow().app.as_ref().map(|a| a.id))
    }

    /// Capture the current context.
    ///
    /// A context must be captured and recorded by tasks that may run in other threads, the context must be loaded
    /// in the other thread using [`with_context`].
    ///
    /// [`with_context`]: ThreadContext::with_context
    pub fn capture() -> ThreadContext {
        THREAD_CONTEXT.with(|s| {
            let mut r = s.borrow().clone();
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
    /// use zero_ui_core::context::ThreadContext;
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
        let prev = THREAD_CONTEXT.with(|s| mem::replace(&mut *s.borrow_mut(), self.clone()));
        let _restore = RunOnDrop::new(move || {
            let _drop = THREAD_CONTEXT.with(|s| mem::replace(&mut *s.borrow_mut(), prev));
        });
        f()
    }
}

struct ContextLocalData<T: Send + Sync + 'static> {
    values: Vec<(ThreadId, T)>,
    default: Option<T>,
}
impl<T: Send + Sync + 'static> ContextLocalData<T> {
    fn new() -> Self {
        Self {
            values: vec![],
            default: None,
        }
    }
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
        self.try_cleanup(id, 0);
    }

    fn try_cleanup(&'static self, id: AppId, tries: u8) {
        if let Some(mut w) = self.value.try_write_for(if tries == 0 { 50.ms() } else { 500.ms() }) {
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
        self.read_impl(self.value.read_recursive())
    }

    /// Try read lock the value associated with the current app.
    ///
    /// Initializes the default value for the app if this is the first read.
    ///
    /// # Panics
    ///
    /// Panics if no app is running, see [`App::is_running`] for more details.
    ///
    /// [`App::is_running`]: crate::app::App::is_running
    pub fn try_read(&'static self) -> Option<MappedRwLockReadGuard<T>> {
        Some(self.read_impl(self.value.try_read_recursive()?))
    }

    fn read_impl(&'static self, read: RwLockReadGuard<'static, Vec<(AppId, T)>>) -> MappedRwLockReadGuard<T> {
        let id = ThreadContext::current_app().expect("no app running, `app_local` can only be accessed inside apps");

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

        ThreadContext::register_cleanup(Box::new(move |id| self.cleanup(id)));

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
        self.write_impl(self.value.write())
    }

    /// Try to write lock the value associated with the current app.
    ///
    /// Initializes the default value for the app if this is the first read.
    ///
    /// # Panics
    ///
    /// Panics if no app is running, see [`App::is_running`] for more details.
    ///
    /// [`App::is_running`]: crate::app::App::is_running
    pub fn try_write(&'static self) -> Option<MappedRwLockWriteGuard<T>> {
        Some(self.write_impl(self.value.try_write()?))
    }

    fn write_impl(&'static self, mut write: RwLockWriteGuard<'static, Vec<(AppId, T)>>) -> MappedRwLockWriteGuard<T> {
        let id = ThreadContext::current_app().expect("no app running, `app_local` can only be accessed inside apps");

        if let Some(i) = write.iter().position(|(s, _)| *s == id) {
            return RwLockWriteGuard::map(write, |v| &mut v[i].1);
        }

        let value = (self.init)();
        let i = write.len();
        write.push((id, value));

        ThreadContext::register_cleanup(Box::new(move |id| self.cleanup(id)));

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

    /// Create a read lock and `map` it to a sub-value.
    pub fn read_map<O>(&'static self, map: impl FnOnce(&T) -> &O) -> MappedRwLockReadGuard<O> {
        MappedRwLockReadGuard::map(self.read(), map)
    }

    /// Try to create a read lock and `map` it to a sub-value.
    pub fn try_wread_map<O>(&'static self, map: impl FnOnce(&T) -> &O) -> Option<MappedRwLockReadGuard<O>> {
        let lock = self.try_read()?;
        Some(MappedRwLockReadGuard::map(lock, map))
    }

    /// Create a write lock and `map` it to a sub-value.
    pub fn write_map<O>(&'static self, map: impl FnOnce(&mut T) -> &mut O) -> MappedRwLockWriteGuard<O> {
        MappedRwLockWriteGuard::map(self.write(), map)
    }

    /// Try to create a write lock and `map` it to a sub-value.
    pub fn try_write_map<O>(&'static self, map: impl FnOnce(&mut T) -> &mut O) -> Option<MappedRwLockWriteGuard<O>> {
        let lock = self.try_write()?;
        Some(MappedRwLockWriteGuard::map(lock, map))
    }
}

///<span data-del-macro-root></span> Declares new app local variable.
///
/// See [`AppLocal<T>`] for more details.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::{app::*, context::*};
/// app_local! {
///     /// A public documented value.
///     pub static FOO: u8 = 10u8;
///
///     // A private value.
///     static BAR: String = "Into!";
/// }
///
/// let app = App::minimal();
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
        $vis static $IDENT: $crate::context::AppLocal<$T> = {
            fn init() -> $T {
                std::convert::Into::into($init)
            }
            $crate::context::AppLocal::new(init)
        };
    )+};
}
#[doc(inline)]
pub use app_local;

/// Represents an [`AppLocal<T>`] value that can be temporarily overridden in a context.
///
/// The *context* works across threads, as long as the threads are instrumented using [`ThreadContext`].
///
/// Use the [`context_local!`] macro to declare a static variable in the same style as [`thread_local!`].
pub struct ContextLocal<T: Send + Sync + 'static> {
    data: AppLocal<ContextLocalData<T>>,
    init: fn() -> T,
}
impl<T: Send + Sync + 'static> ContextLocal<T> {
    #[doc(hidden)]
    pub const fn new(init: fn() -> T) -> Self {
        Self {
            data: AppLocal::new(ContextLocalData::new),
            init,
        }
    }
    /// Calls `f` with the `value` loaded in context.
    ///
    /// The `value` is moved in context, `f` is called, then restores the `value`.
    ///
    /// # Panics
    ///
    /// If `value` is `None`. Note that if `T: Option<I>` you can use [`with_context_opt`].
    ///
    /// If no app is running, see [`App::is_running`] for more details.
    ///
    /// [`with_context_opt`]: Self::with_context_opt
    /// [`App::is_running`]: crate::app::App::is_running
    pub fn with_context<R>(&'static self, value: &mut Option<T>, f: impl FnOnce() -> R) -> R {
        let new_value = value.take().expect("no override provided");
        let thread_id = std::thread::current().id();

        let i;
        let prev_value;

        let mut write = self.data.write();
        if let Some(idx) = write.values.iter_mut().position(|(id, _)| *id == thread_id) {
            // already contextualized in this thread

            i = idx;
            prev_value = mem::replace(&mut write.values[i].1, new_value);

            drop(write);

            let _restore = RunOnDrop::new(move || {
                let mut write = self.data.write();
                *value = Some(mem::replace(&mut write.values[i].1, prev_value));
            });

            f()
        } else {
            // first contextualization in this thread
            write.values.push((thread_id, new_value));

            drop(write);

            let _restore = RunOnDrop::new(move || {
                let mut write = self.data.write();
                let i = write.values.iter_mut().position(|(id, _)| *id == thread_id).unwrap();
                *value = Some(write.values.swap_remove(i).1);
            });

            f()
        }
    }

    /// Calls  `f` with the `value` loaded in context, even if it is `None`.
    ///
    /// This behave similar to [`with_context`], but where `T: Option<I>`.
    ///
    /// [`with_context`]: Self::with_context
    pub fn with_context_opt<R, I: Send + Sync + 'static>(&'static self, value: &mut Option<I>, f: impl FnOnce() -> R) -> R
    where
        T: option::Option<I>,
    {
        let new_value: T = option::Option::cast(value.take());
        let thread_id = std::thread::current().id();

        let i;
        let prev_value;

        let mut write = self.data.write();
        if let Some(idx) = write.values.iter_mut().position(|(id, _)| *id == thread_id) {
            // already contextualized in this thread

            i = idx;
            prev_value = mem::replace(&mut write.values[i].1, new_value);

            drop(write);

            let _restore = RunOnDrop::new(move || {
                let mut write = self.data.write();
                *value = mem::replace(&mut write.values[i].1, prev_value).get_mut().take();
            });

            f()
        } else {
            // first contextualization in this thread
            write.values.push((thread_id, new_value));

            drop(write);

            let _restore = RunOnDrop::new(move || {
                let mut write = self.data.write();
                let i = write.values.iter_mut().position(|(id, _)| *id == thread_id).unwrap();
                *value = write.values.swap_remove(i).1.get_mut().take();
            });

            f()
        }
    }

    /// Lock the context local for read.
    ///
    /// The value can be read locked more than once at the same time, including on the same thread. While the
    /// read guard is alive calls to [`with_context`] and [`write`] are blocked.
    ///
    /// [`with_context`]: Self::with_context
    /// [`write`]: Self::write
    pub fn read(&'static self) -> MappedRwLockReadGuard<T> {
        let read = self.data.read();
        for thread_id in ThreadContext::capture().context() {
            if let Some(i) = read.values.iter().position(|(id, _)| id == thread_id) {
                // contextualized in thread or task parent thread.
                return MappedRwLockReadGuard::map(read, move |v| &v.values[i].1);
            }
        }

        if read.default.is_some() {
            return MappedRwLockReadGuard::map(read, move |v| v.default.as_ref().unwrap());
        }
        drop(read);

        let mut write = self.data.write();
        write.default = Some((self.init)());

        drop(write);
        let read = self.data.read();
        MappedRwLockReadGuard::map(read, move |v| v.default.as_ref().unwrap())
    }

    /// Exclusive lock the context local for write.
    ///
    /// The value can only be locked once at the same time, deadlocks if called twice in the same thread, blocks calls
    /// to [`with_context`] and [`write`].
    ///
    /// [`with_context`]: Self::with_context
    /// [`write`]: Self::write
    pub fn write(&'static self) -> MappedRwLockWriteGuard<T> {
        let mut write = self.data.write();
        for thread_id in ThreadContext::capture().context() {
            if let Some(i) = write.values.iter().position(|(id, _)| id == thread_id) {
                // contextualized in thread or task parent thread.
                return MappedRwLockWriteGuard::map(write, move |v| &mut v.values[i].1);
            }
        }

        if write.default.is_some() {
            return MappedRwLockWriteGuard::map(write, |v| v.default.as_mut().unwrap());
        }

        write.default = Some((self.init)());

        MappedRwLockWriteGuard::map(write, |v| v.default.as_mut().unwrap())
    }

    /// Get a clone of the current contextual value.
    pub fn get(&'static self) -> T
    where
        T: Clone,
    {
        self.read().clone()
    }

    /// Set the current contextual value.
    ///
    /// This changes the current contextual value or the **default value**.
    pub fn set(&'static self, value: T) {
        *self.write() = value;
    }
}

///<span data-del-macro-root></span> Declares new app and context local variable.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::context::context_local;
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
/// # use zero_ui_core::{context::context_local, app::App};
/// context_local! { static FOO: String = "default"; }
///
/// fn print_value() {
///     println!("value is {}!", FOO.read());
/// }
///
/// let _scope = App::minimal();
///
/// let mut value = Some(String::from("other"));
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
        $(#[$meta])*
        $vis static $IDENT: $crate::context::ContextLocal<$T> = {
            fn init() -> $T {
                std::convert::Into::into($init)
            }
            $crate::context::ContextLocal::new(init)
        };
    )+};
}
#[doc(inline)]
pub use context_local;

#[doc(hidden)]
pub mod option {
    pub trait Option<T> {
        fn get(&self) -> &std::option::Option<T>;
        fn get_mut(&mut self) -> &mut std::option::Option<T>;
        fn cast(value: std::option::Option<T>) -> Self;
    }

    impl<T> Option<T> for std::option::Option<T> {
        fn get(&self) -> &std::option::Option<T> {
            self
        }
        fn get_mut(&mut self) -> &mut std::option::Option<T> {
            self
        }
        fn cast(value: std::option::Option<T>) -> Self {
            value
        }
    }
}

/// Helper for declaring nodes that sets a [`ContextLocal`].
pub fn with_context_local<T: Any + Send + Sync + 'static>(
    child: impl UiNode,
    context: &'static ContextLocal<T>,
    value: impl Into<T>,
) -> impl UiNode {
    #[ui_node(struct WithContextLocalNode<T: Any + Send + Sync + 'static> {
        child: impl UiNode,
        context: &'static ContextLocal<T>,
        value: RefCell<Option<T>>,
    })]
    impl WithContextLocalNode {
        fn with<R>(&self, mtd: impl FnOnce(&T_child) -> R) -> R {
            let mut value = self.value.borrow_mut();
            self.context.with_context(&mut value, move || mtd(&self.child))
        }

        fn with_mut<R>(&mut self, mtd: impl FnOnce(&mut T_child) -> R) -> R {
            let value = self.value.get_mut();
            self.context.with_context(value, || mtd(&mut self.child))
        }

        #[UiNode]
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.with_mut(|c| c.init(ctx))
        }

        #[UiNode]
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.with_mut(|c| c.deinit(ctx))
        }

        #[UiNode]
        fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
            self.with(|c| c.info(ctx, info))
        }

        #[UiNode]
        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            self.with_mut(|c| c.event(ctx, update))
        }

        #[UiNode]
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.with_mut(|c| c.update(ctx, updates))
        }

        #[UiNode]
        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> units::PxSize {
            self.with(|c| c.measure(ctx, wm))
        }

        #[UiNode]
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> units::PxSize {
            self.with_mut(|c| c.layout(ctx, wl))
        }

        #[UiNode]
        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.with(|c| c.render(ctx, frame))
        }

        #[UiNode]
        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            self.with(|c| c.render_update(ctx, update))
        }
    }
    WithContextLocalNode {
        child,
        context,
        value: RefCell::new(Some(value.into())),
    }
}
