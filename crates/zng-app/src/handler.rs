//! Handler types and macros.

use std::pin::Pin;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use parking_lot::Mutex;
#[doc(hidden)]
pub use zng_clone_move::*;

use crate::update::UPDATES;
use crate::widget::{UiTaskWidget as _, WIDGET};
use crate::{AppControlFlow, HeadlessApp};
use zng_handle::{Handle, WeakHandle};
use zng_task::{self as task, UiTask};

use crate::INSTANT;

/// Output of [`Handler<A>`].
pub enum HandlerResult {
    /// Handler already finished.
    Done,
    /// Handler is async and the future was pending after first poll. The caller must run the future in the same context the handler was called.
    Continue(Pin<Box<dyn Future<Output = ()> + Send + 'static>>),
}

/// Represents a handler in a widget context.
///
/// There are different flavors of handlers, you can use macros to declare then.
/// See [`hn!`], [`hn_once!`] or [`async_hn!`], [`async_hn_once!`] to start.
///
/// # Type Inference Limitations
///
/// This type is not a full struct because the closure args type inference only works with `Box`, if this was
/// a full `struct` all handler declarations that use the args would have to declare the args type.
/// Methods for this type are implemented in [`HandlerExt`]. Also note that the `A` type must be `Clone + 'static`,
/// unfortunately Rust does not enforce bounds in type alias.
#[allow(type_alias_bounds)] // we need a type alias here
pub type Handler<A: Clone + 'static> = Box<dyn FnMut(&A) -> HandlerResult + Send + 'static>;

/// Extension methods for [`Handler<A>`].
pub trait HandlerExt<A: Clone + 'static> {
    /// Notify the handler in a widget context.
    ///
    /// If the handler is async polls once immediately and returns an [`UiTask`] if the future is pending.
    /// The caller must update the task until completion in the same widget context.
    fn widget_event(&mut self, args: &A) -> Option<UiTask<()>>;

    /// Notify the handler outside of any widget or window context, inside a [`APP_HANDLER`] context.
    ///
    /// If the handler is async polls once and continue execution in [`UPDATES`].
    fn app_event(&mut self, handle: Box<dyn AppWeakHandle>, is_preview: bool, args: &A);

    /// New handler that only calls for arguments approved by `filter`.
    fn filtered(self, filter: impl FnMut(&A) -> bool + Send + 'static) -> Handler<A>;

    /// New handler that calls this one only once.
    fn into_once(self) -> Handler<A>;

    /// Into cloneable handler.
    ///
    /// Note that [`hn_once!`] and [`async_hn_once!`] handlers will still only run once.
    fn into_arc(self) -> ArcHandler<A>;

    /// Wrap the handler into a type that implements the async task management in an widget context.
    fn into_wgt_runner(self) -> WidgetRunner<A>;
}
impl<A: Clone + 'static> HandlerExt<A> for Handler<A> {
    fn widget_event(&mut self, args: &A) -> Option<UiTask<()>> {
        match self(args) {
            HandlerResult::Done => None,
            HandlerResult::Continue(future) => {
                let mut task = UiTask::new_boxed(Some(WIDGET.id()), future);
                if task.update().is_none() { Some(task) } else { None }
            }
        }
    }

    fn app_event(&mut self, handle: Box<dyn AppWeakHandle>, is_preview: bool, args: &A) {
        match APP_HANDLER.with(handle.clone_boxed(), is_preview, || self(args)) {
            HandlerResult::Done => {}
            HandlerResult::Continue(future) => {
                let mut task = UiTask::new_boxed(None, future);
                if APP_HANDLER.with(handle.clone_boxed(), is_preview, || task.update().is_none()) {
                    if is_preview {
                        UPDATES
                            .on_pre_update(hn!(|_| {
                                if APP_HANDLER.with(handle.clone_boxed(), is_preview, || task.update().is_some()) {
                                    APP_HANDLER.unsubscribe();
                                }
                            }))
                            .perm();
                    } else {
                        UPDATES
                            .on_update(hn!(|_| {
                                if APP_HANDLER.with(handle.clone_boxed(), is_preview, || task.update().is_some()) {
                                    APP_HANDLER.unsubscribe();
                                }
                            }))
                            .perm();
                    }
                }
            }
        }
    }

    fn filtered(mut self, mut filter: impl FnMut(&A) -> bool + Send + 'static) -> Self {
        Box::new(move |a| if filter(a) { self(a) } else { HandlerResult::Done })
    }

    fn into_once(self) -> Self {
        let mut f = Some(self);
        Box::new(move |a| {
            if let Some(mut f) = f.take() {
                APP_HANDLER.unsubscribe();
                f(a)
            } else {
                HandlerResult::Done
            }
        })
    }

    fn into_arc(self) -> ArcHandler<A> {
        ArcHandler(Arc::new(Mutex::new(self)))
    }

    fn into_wgt_runner(self) -> WidgetRunner<A> {
        WidgetRunner::new(self)
    }
}

/// Represents a cloneable handler.
///
/// See [`Handler::into_arc`] for more details.
#[derive(Clone)]
pub struct ArcHandler<A: Clone + 'static>(Arc<Mutex<Handler<A>>>);
impl<A: Clone + 'static> ArcHandler<A> {
    /// Calls [`HandlerExt::widget_event`].
    pub fn widget_event(&self, args: &A) -> Option<UiTask<()>> {
        self.0.lock().widget_event(args)
    }

    /// Calls [`HandlerExt::app_event`].
    pub fn app_event(&self, handle: Box<dyn AppWeakHandle>, is_preview: bool, args: &A) {
        self.0.lock().app_event(handle, is_preview, args)
    }

    /// Calls the handler.
    pub fn call(&self, args: &A) -> HandlerResult {
        self.0.lock()(args)
    }

    /// Make a handler from this arc handler.
    pub fn handler(&self) -> Handler<A> {
        self.clone().into()
    }
}
impl<A: Clone + 'static> From<ArcHandler<A>> for Handler<A> {
    fn from(f: ArcHandler<A>) -> Self {
        Box::new(move |a| f.0.lock()(a))
    }
}

/// Represents an widget [`Handler<A>`] caller that manages the async tasks if needed.
///
/// See [`Handler::into_wgt_runner`] for more details.
pub struct WidgetRunner<A: Clone + 'static> {
    handler: Handler<A>,
    tasks: Vec<UiTask<()>>,
}

impl<A: Clone + 'static> WidgetRunner<A> {
    fn new(handler: Handler<A>) -> Self {
        Self { handler, tasks: vec![] }
    }

    /// Call [`HandlerExt::widget_event`] and start UI task is needed.
    pub fn event(&mut self, args: &A) {
        if let Some(task) = self.handler.widget_event(args) {
            self.tasks.push(task);
        }
    }

    /// Update async tasks.
    ///
    /// UI node implementers must call this on [`UiNodeOp::Update`].
    /// For preview events before delegation to child, for other after delegation.
    ///
    /// [`UiNodeOp::Update`]: crate::widget::node::UiNodeOp::Update
    pub fn update(&mut self) {
        self.tasks.retain_mut(|t| t.update().is_none());
    }

    /// Drop pending tasks.
    ///
    /// Dropped tasks will log a warning.
    ///
    /// UI node implementers must call this on [`UiNodeOp::Deinit`], async tasks must not run across widget reinit.
    ///
    /// [`UiNodeOp::Deinit`]: crate::widget::node::UiNodeOp::Deinit
    pub fn deinit(&mut self) {
        self.tasks.clear();
    }
}

///<span data-del-macro-root></span> Declare a mutable *clone-move* event handler.
///
/// The macro input is a closure with optional *clone-move* variables, internally it uses [`clmv!`] so
/// the input is the same syntax.
///
/// # Examples
///
/// The example declares an event handler for the `on_click` property.
///
/// ```
/// # macro_rules! example { () => {
/// on_click = hn!(|_| {
///     println!("Clicked {}!", args.click_count);
/// });
/// # }}
/// ```
///
/// Internally the [`clmv!`] macro is used so you can *clone-move* variables into the handler.
///
/// ```
/// # macro_rules! example { () => {
/// let foo = var(0);
///
/// // ..
///
/// # let
/// on_click = hn!(foo, |args| {
///     foo.set(args.click_count);
/// });
///
/// // can still use after:
/// let bar = foo.map(|c| formatx!("click_count: {c}"));
///
/// # }}
/// ```
///
/// In the example above only a clone of `foo` is moved into the handler. Note that handlers always capture by move, if `foo` was not
/// listed in the *clone-move* section it would not be available after the handler is created. See [`clmv!`] for details.
///
/// # App Scope
///
/// When used in app scopes the [`APP_HANDLER`] contextual service can be used to unsubscribe from inside the handler.
///
/// The example declares an event handler for the `CLICK_EVENT`. Unlike in an widget this handler will run in the app scope, in this case
/// the `APP_HANDLER` is available during handler calls, in the example the subscription handle is marked `perm`, but the event still unsubscribes
/// from the inside.
///
/// ```
/// # zng_app::event::event_args! { pub struct ClickArgs { pub target: zng_txt::Txt, pub click_count: usize, .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) { } } }
/// # zng_app::event::event! { pub static CLICK_EVENT: ClickArgs; }
/// # use zng_app::handler::{hn, APP_HANDLER};
/// # let _scope = zng_app::APP.minimal();
/// # fn assert_type() {
/// CLICK_EVENT
///     .on_event(hn!(|args| {
///         println!("Clicked Somewhere!");
///         if args.target == "something" {
///             APP_HANDLER.unsubscribe();
///         }
///     }))
///     .perm();
/// # }
/// ```
///
/// [`clmv!`]: zng_clone_move::clmv
#[macro_export]
macro_rules! hn {
    ($($clmv:ident,)* |_| $body:expr) => {
        std::boxed::Box::new($crate::handler::clmv!($($clmv,)* |_| {
            #[allow(clippy::redundant_closure_call)] // closure is to support `return;`
            (||{
                $body
            })();
            #[allow(unused)]
            {
                $crate::handler::HandlerResult::Done
            }
        }))
    };
    ($($clmv:ident,)* |$args:ident| $body:expr) => {
        std::boxed::Box::new($crate::handler::clmv!($($clmv,)* |$args| {
            #[allow(clippy::redundant_closure_call)]
            (||{
                $body
            })();
            #[allow(unused)]
            {
                $crate::handler::HandlerResult::Done
            }
        }))
    };
    ($($clmv:ident,)* |$args:ident  : & $Args:ty| $body:expr) => {
        std::boxed::Box::new($crate::handler::clmv!($($clmv,)* |$args: &$Args| {
            #[allow(clippy::redundant_closure_call)]
            (||{
                $body
            })();
            #[allow(unused)]
            {
                $crate::handler::HandlerResult::Done
            }
        }))
    };
}
#[doc(inline)]
pub use crate::hn;

///<span data-del-macro-root></span> Declare a *clone-move* event handler that is only called once.
///
/// The macro input is a closure with optional *clone-move* variables, internally it uses [`clmv!`] so
/// the input is the same syntax.
///
/// # Examples
///
/// The example captures `data` by move and then destroys it in the first call, this cannot be done using [`hn!`] because
/// the `data` needs to be available for all event calls. In this case the closure is only called once, subsequent events
/// are ignored by the handler.
///
/// ```
/// # macro_rules! example { () => {
/// let data = vec![1, 2, 3];
/// # let
/// on_click = hn_once!(|_| {
///     for i in data {
///         print!("{i}, ");
///     }
/// });
/// # }}
/// ```
///
/// [`clmv!`]: zng_clone_move::clmv
#[macro_export]
macro_rules! hn_once {
    ($($clmv:ident,)* |_| $body:expr) => {{
        let mut once: Option<std::boxed::Box<dyn FnOnce() + Send + 'static>> =
            Some(std::boxed::Box::new($crate::handler::clmv!($($clmv,)* || { $body })));
        $crate::handler::hn!(|_| if let Some(f) = once.take() {
            $crate::handler::APP_HANDLER.unsubscribe();
            f();
        })
    }};
    ($($clmv:ident,)* |$args:ident| $body:expr) => {{
        // type inference fails here, error message slightly better them not having this pattern
        let mut once: std::boxed::Box<dyn FnOnce(&_) + Send + 'static> =
            Some(std::boxed::Box::new($crate::handler::clmv!($($clmv,)* |$args: &_| { $body })));
        $crate::handler::hn!(|$args: &_| if let Some(f) = once.take() {
            $crate::handler::APP_HANDLER.unsubscribe();
            f($args);
        })
    }};
    ($($clmv:ident,)* |$args:ident  : & $Args:ty| $body:expr) => {{
        // type inference fails here, error message slightly better them not having this pattern
        let mut once: Option<std::boxed::Box<dyn FnOnce(&$Args) + Send + 'static>> =
            Some(std::boxed::Box::new($crate::handler::clmv!($($clmv,)* |$args: &$Args| { $body })));
        $crate::handler::hn!(|$args: &$Args| if let Some(f) = once.take() {
            $crate::handler::APP_HANDLER.unsubscribe();
            f($args);
        })
    }};
}
#[doc(inline)]
pub use crate::hn_once;

///<span data-del-macro-root></span> Declare an async *clone-move* event handler.
///
/// The macro input is a closure with optional *clone-move* variables, internally it uses [`clmv!`] so
/// the input is the same syntax, for each call is also uses [`async_clmv!`] to clone the args and other cloning captures.
///
/// # Examples
///
/// The example declares an async event handler for the `on_click` property.
///
/// ```
/// # macro_rules! example { () => {
/// on_click = async_hn!(|args| {
///     println!("Clicked {} {} times!", WIDGET.id(), args.click_count);
///
///     task::run(async {
///         println!("In other thread!");
///     })
///     .await;
///
///     println!("Back in UI thread, in a widget update.");
/// });
/// # }}
/// ```
///
/// Internally the [`clmv!`] macro is used so you can *clone-move* variables into the handler.
///
/// ```
/// # zng_app::event::event_args! { pub struct ClickArgs { pub target: zng_txt::Txt, pub click_count: usize, .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) { } } }
/// # use zng_app::handler::async_hn;
/// # use zng_var::{var, Var};
/// # use zng_task as task;
/// # use zng_txt::formatx;
/// # let _scope = zng_app::APP.minimal();
/// # fn assert_type() -> zng_app::handler::Handler<ClickArgs> {
/// let enabled = var(true);
///
/// // ..
///
/// # let
/// on_click = async_hn!(enabled, |args: &ClickArgs| {
///     enabled.set(false);
///
///     task::run(async move {
///         println!("do something {}", args.click_count);
///     })
///     .await;
///
///     enabled.set(true);
/// });
///
/// // can still use after:
/// # let
/// text = enabled.map(|&e| if e { "Click Me!" } else { "Busy.." });
/// enabled;
///
/// # on_click }
/// ```
///
/// In the example above only a clone of `enabled` is moved into the handler. Note that handlers always capture by move, if `enabled` was not
/// listed in the *clone-move* section it would not be available after the handler is created. See [`async_clmv_fn!`] for details.
///
/// The example also demonstrates a common pattern with async handlers, most events are only raised when the widget is enabled, so you can
/// disable the widget while the async task is running. This way you don't block the UI running a task but the user cannot spawn a second
/// task while the first is still running.
///
/// ## Futures and Clone-Move
///
/// You want to always *clone-move* captures for async handlers, because they then automatically get cloned again for each event. This
/// needs to happen because you can have more then one *handler task* running at the same type, and both want access to the captured variables.
///
/// This second cloning can be avoided by using the [`async_hn_once!`] macro instead, but only if you expect a single event.
///
/// Note that this means you are declaring a normal closure that returns a `'static` future, not an async closure, see [`async_clmv_fn!`].
///
/// [`async_clmv_fn!`]: zng_clone_move::async_clmv_fn
#[macro_export]
macro_rules! async_hn {
    ($($clmv:ident,)* |_| $body:expr) => {
        std::boxed::Box::new($crate::handler::clmv!($($clmv,)* |_| {
            $crate::handler::HandlerResult::Continue(std::boxed::Box::pin($crate::handler::async_clmv!($($clmv,)* {$body})))
        }))
    };
    ($($clmv:ident,)* |$args:ident| $body:expr) => {
        std::boxed::Box::new($crate::handler::clmv!($($clmv,)* |$args| {
            $crate::handler::HandlerResult::Continue(std::boxed::Box::pin($crate::handler::async_clmv!($args, $($clmv,)* {$body})))
        }))
    };
    ($($clmv:ident,)* |$args:ident  : & $Args:ty| $body:expr) => {
        std::boxed::Box::new($crate::handler::clmv!($($clmv,)* |$args: &$Args| {
            $crate::handler::HandlerResult::Continue(std::boxed::Box::pin($crate::handler::async_clmv!($args, $($clmv,)* {$body})))
        }))
    };
}
#[doc(inline)]
pub use crate::async_hn;

///<span data-del-macro-root></span> Declare an async *clone-move* event handler that is only called once.
///
/// The macro input is a closure with optional *clone-move* variables, internally it uses [`clmv!`] so
/// the input is the same syntax.
///
/// # Examples
///
/// The example captures `data` by move and then moves it again to another thread. This is not something you can do using [`async_hn!`]
/// because that handler expects to be called many times. We expect `on_open` to only be called once, so we can don't need to capture by
/// *clone-move* here just to use `data`.
///
/// ```
/// # macro_rules! example { () => {
/// let data = vec![1, 2, 3];
/// # let
/// on_open = async_hn_once!(|_| {
///     task::run(async move {
///         for i in data {
///             print!("{i}, ");
///         }
///     })
///     .await;
///
///     println!("Done!");
/// });
/// # }}
/// ```
///
/// You can still *clone-move* to have access to the variable after creating the handler, in this case the `data` will be cloned into the handler
/// but will just be moved to the other thread, avoiding a needless clone.
///
/// ```
/// # macro_rules! example { () => {
/// let data = vec![1, 2, 3];
/// # let
/// on_open = async_hn_once!(data, |_| {
///     task::run(async move {
///         for i in data {
///             print!("{i}, ");
///         }
///     })
///     .await;
///
///     println!("Done!");
/// });
/// println!("{data:?}");
/// # }}
/// ```
///
/// [`async_clmv_fn_once!`]: zng_clone_move::async_clmv_fn_once
#[macro_export]
macro_rules! async_hn_once {
    ($($clmv:ident,)* |_| $body:expr) => {
        {
            let mut once: Option<std::boxed::Box<dyn FnOnce() -> std::pin::Pin<std::boxed::Box<dyn Future<Output = ()> + Send + 'static>> + Send + 'static>>
                = Some(std::boxed::Box::new($crate::handler::clmv!($($clmv,)* || {
                    $crate::handler::APP_HANDLER.unsubscribe();
                    std::boxed::Box::pin($crate::handler::async_clmv!($($clmv,)* { $body }))
                })));

            std::boxed::Box::new(move |_| if let Some(f) = once.take() {
                $crate::handler::HandlerResult::Continue(f())
            } else {
                $crate::handler::HandlerResult::Done
            })
        }
    };
    ($($clmv:ident,)* |$args:ident| $body:expr) => {
        {
            let mut once: Option<std::boxed::Box<dyn FnOnce(&_) -> std::pin::Pin<std::boxed::Box<dyn Future<Output = ()> + Send + 'static>> + Send + 'static>>
                = Some(std::boxed::Box::new($crate::handler::clmv!($($clmv,)* |$args: &_| {
                    $crate::handler::APP_HANDLER.unsubscribe();
                    std::boxed::Box::pin($crate::handler::async_clmv!($args, $($clmv,)* { $body }))
                })));

            std::boxed::Box::new(move |$args: &_| if let Some(f) = once.take() {
                $crate::handler::HandlerResult::Continue(f($args))
            } else {
                $crate::handler::HandlerResult::Done
            })
        }
    };
    ($($clmv:ident,)* |$args:ident  : & $Args:ty| $body:expr) => {
        {
            let mut once: Option<std::boxed::Box<dyn FnOnce(&$Args) -> std::pin::Pin<std::boxed::Box<dyn Future<Output = ()> + Send + 'static>> + Send + 'static>>
                = Some(std::boxed::Box::new($crate::handler::clmv!($($clmv,)* |$args: &$Args| {
                    $crate::handler::APP_HANDLER.unsubscribe();
                    std::boxed::Box::pin($crate::handler::async_clmv!($args, $($clmv,)* { $body }))
                })));

            std::boxed::Box::new(move |$args: &$Args| if let Some(f) = once.take() {
                $crate::handler::HandlerResult::Continue(f($args))
            } else {
                $crate::handler::HandlerResult::Done
            })
        }
    };
}
#[doc(inline)]
pub use crate::async_hn_once;

/// Represents a weak handle to a [`Handler`] subscription in the app context.
///
/// Inside the handler use [`APP_HANDLER`] to access this handle.
pub trait AppWeakHandle: Send + Sync + 'static {
    /// Dynamic clone.
    fn clone_boxed(&self) -> Box<dyn AppWeakHandle>;

    /// Unsubscribes the [`Handler`].
    ///
    /// This stops the handler from being called again and causes it to be dropped in a future app update.
    fn unsubscribe(&self);
}
impl<D: Send + Sync + 'static> AppWeakHandle for WeakHandle<D> {
    fn clone_boxed(&self) -> Box<dyn AppWeakHandle> {
        Box::new(self.clone())
    }

    fn unsubscribe(&self) {
        if let Some(handle) = self.upgrade() {
            handle.force_drop();
        }
    }
}

/// Service available in app scoped [`Handler<A>`] calls.
#[allow(non_camel_case_types)]
pub struct APP_HANDLER;

impl APP_HANDLER {
    /// Acquire a weak reference to the event subscription handle if the handler is being called in the app scope.
    pub fn weak_handle(&self) -> Option<Box<dyn AppWeakHandle>> {
        if let Some(ctx) = &*APP_HANDLER_CTX.get() {
            Some(ctx.handle.clone_boxed())
        } else {
            None
        }
    }

    /// Unsubscribe, if the handler is being called in the app scope.
    pub fn unsubscribe(&self) {
        if let Some(h) = self.weak_handle() {
            h.unsubscribe();
        }
    }

    /// If the handler is being called in the *preview* track.
    pub fn is_preview(&self) -> bool {
        if let Some(ctx) = &*APP_HANDLER_CTX.get() {
            ctx.is_preview
        } else {
            false
        }
    }

    /// Calls `f` with the `handle` and `is_preview` values in context.
    pub fn with<R>(&self, handle: Box<dyn AppWeakHandle>, is_preview: bool, f: impl FnOnce() -> R) -> R {
        APP_HANDLER_CTX.with_context(&mut Some(Arc::new(Some(AppHandlerCtx { handle, is_preview }))), f)
    }
}
zng_app_context::context_local! {
    static APP_HANDLER_CTX: Option<AppHandlerCtx> = None;
}

struct AppHandlerCtx {
    handle: Box<dyn AppWeakHandle>,
    is_preview: bool,
}

impl HeadlessApp {
    /// Calls a [`Handler<A>`] once and blocks until the update tasks started during the call complete.
    ///
    /// This function *spins* until all update tasks are completed. Timers or send events can
    /// be received during execution but the loop does not sleep, it just spins requesting an update
    /// for each pass.
    pub fn block_on<A>(&mut self, handler: &mut Handler<A>, args: &A, timeout: Duration) -> Result<(), String>
    where
        A: Clone + 'static,
    {
        self.block_on_multi(vec![handler], args, timeout)
    }

    /// Calls multiple [`Handler<A>`] once each and blocks until all update tasks are complete.
    ///
    /// This function *spins* until all update tasks are completed. Timers or send events can
    /// be received during execution but the loop does not sleep, it just spins requesting an update
    /// for each pass.
    pub fn block_on_multi<A>(&mut self, handlers: Vec<&mut Handler<A>>, args: &A, timeout: Duration) -> Result<(), String>
    where
        A: Clone + 'static,
    {
        let (pre_len, pos_len) = UPDATES.handler_lens();

        let handle = Handle::dummy(()).downgrade();
        for handler in handlers {
            handler.app_event(handle.clone_boxed(), false, args);
        }

        let mut pending = UPDATES.new_update_handlers(pre_len, pos_len);

        if !pending.is_empty() {
            let start_time = INSTANT.now();
            while {
                pending.retain(|h| h());
                !pending.is_empty()
            } {
                UPDATES.update(None);
                let flow = self.update(false);
                if INSTANT.now().duration_since(start_time) >= timeout {
                    return Err(format!(
                        "block_on reached timeout of {timeout:?} before the handler task could finish",
                    ));
                }

                match flow {
                    AppControlFlow::Poll => continue,
                    AppControlFlow::Wait => {
                        thread::yield_now();
                        continue;
                    }
                    AppControlFlow::Exit => return Ok(()),
                }
            }
        }

        Ok(())
    }

    /// Polls a `future` and updates the app repeatedly until it completes or the `timeout` is reached.
    pub fn block_on_fut<F: Future>(&mut self, future: F, timeout: Duration) -> Result<F::Output, String> {
        let future = task::with_deadline(future, timeout);
        let mut future = std::pin::pin!(future);

        let waker = UPDATES.waker(None);
        let mut cx = std::task::Context::from_waker(&waker);

        loop {
            let mut fut_poll = future.as_mut().poll(&mut cx);
            let flow = self.update_observe(
                || {
                    if fut_poll.is_pending() {
                        fut_poll = future.as_mut().poll(&mut cx);
                    }
                },
                true,
            );

            match fut_poll {
                std::task::Poll::Ready(r) => match r {
                    Ok(r) => return Ok(r),
                    Err(e) => return Err(e.to_string()),
                },
                std::task::Poll::Pending => {}
            }

            match flow {
                AppControlFlow::Poll => continue,
                AppControlFlow::Wait => {
                    thread::yield_now();
                    continue;
                }
                AppControlFlow::Exit => return Err("app exited".to_owned()),
            }
        }
    }

    /// Calls the `handler` once and [`block_on`] it with a 60 seconds timeout using the minimal headless app.
    ///
    /// [`block_on`]: Self::block_on
    #[track_caller]
    #[cfg(any(test, doc, feature = "test_util"))]
    pub fn doc_test<A, H>(args: A, mut handler: Handler<A>)
    where
        A: Clone + 'static,
    {
        let mut app = crate::APP.minimal().run_headless(false);
        app.block_on(&mut handler, &args, DOC_TEST_BLOCK_ON_TIMEOUT).unwrap();
    }

    /// Calls the `handlers` once each and [`block_on_multi`] with a 60 seconds timeout.
    ///
    /// [`block_on_multi`]: Self::block_on_multi
    #[track_caller]
    #[cfg(any(test, doc, feature = "test_util"))]
    pub fn doc_test_multi<A>(args: A, mut handlers: Vec<Handler<A>>)
    where
        A: Clone + 'static,
    {
        let mut app = crate::APP.minimal().run_headless(false);
        app.block_on_multi(handlers.iter_mut().collect(), &args, DOC_TEST_BLOCK_ON_TIMEOUT)
            .unwrap()
    }
}

#[cfg(any(test, doc, feature = "test_util"))]
const DOC_TEST_BLOCK_ON_TIMEOUT: Duration = Duration::from_secs(60);

#[cfg(test)]
mod tests {
    use crate::handler::{Handler, async_hn, async_hn_once, hn, hn_once};

    #[test]
    fn hn_return() {
        t(hn!(|args| {
            if args.field {
                return;
            }
            println!("else");
        }))
    }

    #[test]
    fn hn_once_return() {
        t(hn_once!(|args: &TestArgs| {
            if args.field {
                return;
            }
            println!("else");
        }))
    }

    #[test]
    fn async_hn_return() {
        t(async_hn!(|args| {
            if args.field {
                return;
            }
            args.task().await;
        }))
    }

    #[test]
    fn async_hn_once_return() {
        t(async_hn_once!(|args: &TestArgs| {
            if args.field {
                return;
            }
            args.task().await;
        }))
    }

    fn t(_: Handler<TestArgs>) {}

    #[derive(Clone, Default)]
    struct TestArgs {
        pub field: bool,
    }

    impl TestArgs {
        async fn task(&self) {}
    }
}
