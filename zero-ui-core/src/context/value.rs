use std::{
    any::Any,
    cell::RefCell,
    mem,
    thread::{LocalKey, ThreadId},
};

use parking_lot::*;

use crate::{
    app::AppLocal,
    context::{InfoContext, LayoutContext, MeasureContext, RenderContext, WidgetContext, WidgetUpdates},
    crate_util::RunOnDrop,
    event::EventUpdate,
    render::{FrameBuilder, FrameUpdate},
    ui_node, units,
    widget_info::{WidgetInfoBuilder, WidgetLayout},
    widget_instance::UiNode,
};

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
    ///
    /// The `value` is moved in context, `f` is called, then restores the `value`.
    ///
    /// # Panics
    ///
    /// If `value` is `None`.
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

    /// Calls  `f` with the `value` loaded in context, even if it is `None`.
    ///
    /// This behave similar to [`with_override`], but where `T: Option<I>`.
    pub fn with_override_opt<R, I: Send + Sync + 'static>(&'static self, value: &mut Option<I>, f: impl FnOnce() -> R) -> R
    where
        T: option::Option<I>,
    {
        let new_value: T = option::Option::cast(value.take());
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
                *value = mem::replace(&mut write[i].1, prev_value).get_mut().take();
            });

            f()
        } else {
            // first contextualization in this thread
            write.push((thread_id, new_value));

            let _restore = RunOnDrop::new(move || {
                let mut write = self.data.write();
                let i = write.iter_mut().position(|(id, _)| *id == thread_id).unwrap();
                *value = write.swap_remove(i).1.get_mut().take();
            });

            f()
        }
    }

    /// Lock the context local for read.
    ///
    /// The value can be read locked more than once at the same time, including on the same thread. While the
    /// read guard is alive calls to [`with_override`] and [`write`] are blocked.
    ///
    /// [`with_override`]: Self::with_override
    /// [`write`]: Self::write
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

    /// Exclusive lock the context local for write.
    ///
    /// The value can only be locked once at the same time, deadlocks if called twice in the same thread, blocks calls
    /// to [`with_override`] and [`write`].
    ///
    /// [`with_override`]: Self::with_override
    /// [`write`]: Self::write
    pub fn write(&'static self) -> MappedRwLockWriteGuard<T> {
        let write = self.data.write();
        for thread_id in ThreadContext::capture().context() {
            if let Some(i) = write.iter().position(|(id, _)| id == thread_id) {
                // contextualized in thread or task parent thread.
                return MappedRwLockWriteGuard::map(write, move |v| &mut v[i].1);
            }
        }
        drop(write);

        let mut write = self.default.write();
        if write.is_some() {
            return RwLockWriteGuard::map(write, |v| v.as_mut().unwrap());
        }

        *write = Some((self.init)());

        RwLockWriteGuard::map(write, |v| v.as_mut().unwrap())
    }
}

///<span data-del-macro-root></span> Declares new thread local static that facilitates sharing *contextual* values.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::context_value;
/// context_value! {
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
/// The default value is instantiated once per thread, the expression can be any static value that converts [`Into<T>`].
///
/// # Usage
///
/// After you declare the contextual value you can use it by loading a context, calling a closure and inside it *visiting* the value.
///
/// ```
/// # use zero_ui_core::context_value;
/// context_value! { static FOO: String = "default"; }
///
/// fn print_value() {
///     FOO.with(|val| println!("value is {val}!"));
/// }
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
#[macro_export]
macro_rules! context_value {
($(
    $(#[$attr:meta])*
    $vis:vis static $NAME:ident: $Type:ty = $default:expr;
)+) => {$(
    $crate::paste! {
        std::thread_local! {
            #[doc(hidden)]
            static [<$NAME _LOCAL>]: $crate::context::ContextValueData<$Type> = $crate::context::ContextValueData::init($default);
        }
    }

    $(#[$attr])*
    $vis static $NAME: $crate::context::ContextValue<$Type> = paste::paste! { $crate::context::ContextValue::new(&[<$NAME _LOCAL>]) };
)+}
}
#[doc(inline)]
pub use context_value;

#[doc(hidden)]
pub struct ContextValueData<T: Any> {
    value: RefCell<T>,
}
impl<T: Any> ContextValueData<T> {
    pub fn init(default: impl Into<T>) -> Self {
        Self {
            value: RefCell::new(default.into()),
        }
    }
}

/// Represents value that can only be read in a context.
///
/// See [`context_value!`] for more details.
pub struct ContextValue<T: Any> {
    local: &'static LocalKey<ContextValueData<T>>,
}

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

impl<T: Any> ContextValue<T> {
    #[doc(hidden)]
    pub const fn new(local: &'static LocalKey<ContextValueData<T>>) -> Self {
        ContextValue { local }
    }

    /// Calls `f` with read-only access to the contextual value.
    ///
    /// Returns the result of `f`.
    pub fn with<R>(self, f: impl FnOnce(&T) -> R) -> R {
        self.local.with(|l| f(&*l.value.borrow()))
    }

    /// Like [`with`] but only calls `f` if the value is `Some(I)`.
    ///
    /// [`with`]: Self::with
    pub fn with_opt<I: Any, R>(self, f: impl FnOnce(&I) -> R) -> Option<R>
    where
        T: option::Option<I>,
    {
        self.with(|opt| opt.get().as_ref().map(f))
    }

    /// Calls `f` with exclusive access to the contextual value.
    ///
    /// Returns the result of `f`.
    pub fn with_mut<R>(self, f: impl FnOnce(&mut T) -> R) -> R {
        self.local.with(|l| f(&mut *l.value.borrow_mut()))
    }

    /// Like [`with_mut`] but only calls `f` if the value is `Some(I)`.
    ///
    /// [`with_mut`]: Self::with_mut
    pub fn with_mut_opt<I: Any, R>(self, f: impl FnOnce(&mut I) -> R) -> Option<R>
    where
        T: option::Option<I>,
    {
        self.with_mut(|opt| opt.get_mut().as_mut().map(f))
    }

    /// Runs `action` while the `value` is moved into context, restores the `value` if `action` does not panic.
    ///
    /// Returns the result of `action`, panics if `value` is `None`.
    pub fn with_context<R>(self, value: &mut Option<T>, action: impl FnOnce() -> R) -> R {
        let prev = self.set_context(value.take().expect("no contextual value to load"));
        let r = action();
        *value = Some(self.set_context(prev));
        r
    }

    /// Like [`with_context`], but for context values that are `T: Option<I>`.
    ///
    /// [`with_context`]: Self::with_context
    pub fn with_context_opt<I: Any, R>(self, value: &mut Option<I>, action: impl FnOnce() -> R) -> R
    where
        T: option::Option<I>,
    {
        let prev = self.set_context_opt(value.take());
        let r = action();
        *value = self.set_context_opt(prev);
        r
    }

    /// Get a clone of the current contextual value.
    pub fn get(self) -> T
    where
        T: Clone,
    {
        self.with(Clone::clone)
    }

    fn set_context(&self, val: T) -> T {
        self.local.with(|l| mem::replace(&mut *l.value.borrow_mut(), val))
    }

    fn set_context_opt<I>(&self, val: Option<I>) -> Option<I>
    where
        T: option::Option<I>,
    {
        self.local.with(|l| mem::replace(l.value.borrow_mut().get_mut(), val))
    }
}

impl<T: Any> Clone for ContextValue<T> {
    fn clone(&self) -> Self {
        Self { local: self.local }
    }
}
impl<T: Any> Copy for ContextValue<T> {}

/// Helper for declaring nodes that sets a context value.
pub fn with_context_value<T: Any + Send>(child: impl UiNode, context: ContextValue<T>, value: impl Into<T>) -> impl UiNode {
    #[ui_node(struct WithContextValueNode<T: Any + Send> {
        child: impl UiNode,
        context: ContextValue<T>,
        value: RefCell<Option<T>>,
    })]
    impl WithContextValueNode {
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
        fn measure(&self, ctx: &mut MeasureContext) -> units::PxSize {
            self.with(|c| c.measure(ctx))
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
    WithContextValueNode {
        child,
        context,
        value: RefCell::new(Some(value.into())),
    }
}
