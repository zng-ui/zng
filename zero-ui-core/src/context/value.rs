use std::{any::Any, cell::RefCell, mem, thread::ThreadId};

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
        let prev = THREAD_CONTEXT.with(|s| mem::replace(&mut *s.borrow_mut(), self.context.clone()));
        let _restore = RunOnDrop::new(move || THREAD_CONTEXT.with(|s| *s.borrow_mut() = prev));
        f()
    }
}

/// Represents an [`AppLocal<T>`] value that can be temporarily overridden in a context.
///
/// The *context* works across threads, as long as the threads are instrumented using [`ThreadContext`].
///
/// Use the [`context_local!`] macro to declare a static variable in the same style as [`thread_local!`].
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
    /// If `value` is `None`. Note that if `T: Option<I>` you can use [`with_context_opt`].
    ///
    /// [`with_context_opt`]: Self::with_context_opt
    pub fn with_context<R>(&'static self, value: &mut Option<T>, f: impl FnOnce() -> R) -> R {
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

            drop(write);

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

            drop(write);

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
    /// read guard is alive calls to [`with_context`] and [`write`] are blocked.
    ///
    /// [`with_context`]: Self::with_context
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
    /// to [`with_context`] and [`write`].
    ///
    /// [`with_context`]: Self::with_context
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
/// # use zero_ui_core::context::context_local;
/// context_local! { static FOO: String = "default"; }
///
/// fn print_value() {
///     println!("value is {}!", FOO.read());
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
    WithContextLocalNode {
        child,
        context,
        value: RefCell::new(Some(value.into())),
    }
}
