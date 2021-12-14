//! Handler types and macros.
//!
//! A handler is a closure that takes a *context* and *arguments*, the context can be [`WidgetContext`] or [`AppContext`]
//! with handler types implementing [`WidgetHandler`] or [`AppHandler`] respectively. These traits allow a single caller
//! to support multiple different flavors of handlers, both synchronous and asynchronous, and both `FnMut` and `FnOnce` all
//! by implementing a single entry point.
//!
//! Macros are provided for declaring the various flavors of handlers, [`hn!`], [`hn_once!`], [`async_hn!`], [`async_hn_once!`]
//! for widget contexts and [`app_hn!`], [`app_hn_once!`], [`async_app_hn!`], [`async_app_hn_once!`] for the app context. These
//! macros also build on top of the primitive macros [`clone_move!`], [`async_clone_move_fn!`] and [`async_clone_move_fn_once!`] to
//! provide a very easy way to *clone-move* captured variables into the handler.

use std::future::Future;
use std::time::{Duration, Instant};
use std::{mem, thread};

use retain_mut::RetainMut;

use crate::app::HeadlessApp;
use crate::context::{AppContext, AppContextMut, WidgetContext, WidgetContextMut};
use crate::crate_util::{Handle, WeakHandle};
use crate::task::ui::{AppTask, WidgetTask};
use crate::widget_info::{UpdateSlot, WidgetSubscriptions};

/// Marker traits that can be used to constrain what [`AppHandler`] or [ `WidgetHandler`] are accepted.
///
/// For example a function that wants to receive only synchronous [`WidgetHandler`] handlers can use the
/// [`NotAsyncHn`](marker::NotAsyncHn) to constrain its parameter:
///
/// ```
/// # use zero_ui_core::handler::*;
/// fn foo<H>(handler: H) where H: WidgetHandler<()> + marker::NotAsyncHn { }
/// ```
pub mod marker {
    #[allow(unused_imports)] // use in doc links.
    use super::*;

    /// Represents a handler that is **not** async. Only the [`hn!`], [`hn_once!`], [`app_hn!`] and [`app_hn_once!`]
    /// handlers implement this trait.
    ///
    /// When this constrain is used the [`WidgetHandler`] should not expect to be [updated](WidgetHandler::update).
    ///
    /// See the [module level](self) documentation for details.
    pub trait NotAsyncHn {}

    /// Represents a handler that is async. Only the [`async_hn!`], [`async_hn_once!`], [`async_app_hn!`] and [`async_hn_once!`]
    /// handlers implement this trait.
    ///
    /// See the [module level](self) documentation for details.
    pub trait AsyncHn {}

    /// Represents a handler that will consume it self in the first event call. Only the [`hn_once!`], [`async_hn_once!`],
    /// [`app_hn_once!`] and [`async_hn_once!`] handlers implement this trait.
    ///
    /// See the [module level](self) documentation for details.
    pub trait OnceHn {}
}

/// Represents a handler in a widget context.
///
/// There are different flavors of handlers, you can use macros to declare then.
/// See [`hn!`], [`hn_once!`] or [`async_hn!`], [`async_hn_once!`] to start.
pub trait WidgetHandler<A: Clone + 'static>: 'static {
    /// Called every time the widget [`info`] is rebuild, register async handler waker update slot.
    ///
    /// [`info`]: crate::UiNode::info
    fn subscribe(&self, widget_subscriptions: &mut WidgetSubscriptions) {
        let _ = widget_subscriptions;
    }

    /// Called every time the event happens in the widget context.
    ///
    /// Returns `true` when the event handler is async and it has not finished handing the event, if this
    /// is the case the handler also requests a widget [`info`] rebuild.
    ///
    /// [`update`]: WidgetHandler::update
    /// [`info`]: crate::UiNode::info
    /// [`subscribe`]: WidgetHandler::subscribe
    fn event(&mut self, ctx: &mut WidgetContext, args: &A) -> bool;

    /// Called every widget update.
    ///
    /// Returns `false` when all pending async tasks are completed. Note that event properties
    /// will call this method every update even if it is returning `false`. The return value is
    /// used to implement the test [`block_on`] only.
    ///
    /// [`update`]: WidgetHandler::update
    /// [`block_on`]: crate::context::TestWidgetContext::block_on
    fn update(&mut self, ctx: &mut WidgetContext) -> bool {
        let _ = ctx;
        false
    }
}
#[doc(hidden)]
pub struct FnMutWidgetHandler<H> {
    handler: H,
}
impl<A, H> WidgetHandler<A> for FnMutWidgetHandler<H>
where
    A: Clone + 'static,
    H: FnMut(&mut WidgetContext, &A) + 'static,
{
    fn event(&mut self, ctx: &mut WidgetContext, args: &A) -> bool {
        (self.handler)(ctx, args);
        false
    }
}
impl<H> marker::NotAsyncHn for FnMutWidgetHandler<H> {}
#[doc(hidden)]
pub fn hn<A, H>(handler: H) -> FnMutWidgetHandler<H>
where
    A: Clone + 'static,
    H: FnMut(&mut WidgetContext, &A) + 'static,
{
    FnMutWidgetHandler { handler }
}

///<span data-inline></span> Declare a mutable *clone-move* event handler.
///
/// The macro input is a closure with optional *clone-move* variables, internally it uses [`clone_move!`] so
/// the input is the same syntax.
///
/// # Examples
///
/// The example declares an event handler for the `on_click` property.
///
/// ```
/// # use zero_ui_core::gesture::ClickArgs;
/// # use zero_ui_core::handler::hn;
/// # fn assert_type() -> impl zero_ui_core::handler::WidgetHandler<ClickArgs> {
/// # let
/// on_click = hn!(|_, _| {
///     println!("Clicked!");
/// });
/// # on_click }
/// ```
///
/// The closure input is `&mut WidgetContext` for all handlers and `&ClickArgs` for this property. Note that
/// if you want to use the event args you must annotate the input type, the context type is inferred.
///
/// ```
/// # use zero_ui_core::gesture::ClickArgs;
/// # use zero_ui_core::handler::hn;
/// # fn assert_type() -> impl zero_ui_core::handler::WidgetHandler<ClickArgs> {
/// # let
/// on_click = hn!(|ctx, args: &ClickArgs| {
///     println!("Clicked {}!", args.click_count);
///     let _ = ctx.services;
/// });
/// # on_click }
/// ```
///
/// Internally the [`clone_move!`] macro is used so you can *clone-move* variables into the handler.
///
/// ```
/// # use zero_ui_core::gesture::ClickArgs;
/// # use zero_ui_core::text::formatx;
/// # use zero_ui_core::var::{var, Var};
/// # use zero_ui_core::handler::hn;
/// # fn assert_type() -> impl zero_ui_core::handler::WidgetHandler<ClickArgs> {
/// let foo = var(0);
///
/// // ..
///
/// # let
/// on_click = hn!(foo, |ctx, args: &ClickArgs| {
///     foo.set(ctx, args.click_count);
/// });
///
/// // can still use after:
/// let bar = foo.map(|c| formatx!("click_count: {}", c));
///
/// # on_click }
/// ```
///
/// In the example above only a clone of `foo` is moved into the handler. Note that handlers always capture by move, if `foo` was not
/// listed in the *clone-move* section it would not be available after the handler is created. See [`clone_move!`] for details.
#[macro_export]
macro_rules! hn {
    ($($tt:tt)+) => {
        $crate::handler::hn($crate::clone_move!{ $($tt)+ })
    }
}
#[doc(inline)]
pub use crate::hn;

#[doc(hidden)]
pub struct FnOnceWidgetHandler<H> {
    handler: Option<H>,
}
impl<A, H> WidgetHandler<A> for FnOnceWidgetHandler<H>
where
    A: Clone + 'static,
    H: FnOnce(&mut WidgetContext, &A) + 'static,
{
    fn event(&mut self, ctx: &mut WidgetContext, args: &A) -> bool {
        if let Some(handler) = self.handler.take() {
            handler(ctx, args);
        }
        false
    }
}
impl<H> marker::OnceHn for FnOnceWidgetHandler<H> {}
impl<H> marker::NotAsyncHn for FnOnceWidgetHandler<H> {}
#[doc(hidden)]
pub fn hn_once<A, H>(handler: H) -> FnOnceWidgetHandler<H>
where
    A: Clone + 'static,
    H: FnOnce(&mut WidgetContext, &A) + 'static,
{
    FnOnceWidgetHandler { handler: Some(handler) }
}

///<span data-inline></span> Declare a *clone-move* event handler that is only called once.
///
/// The macro input is a closure with optional *clone-move* variables, internally it uses [`clone_move!`] so
/// the input is the same syntax.
///
/// # Examples
///
/// The example captures `data` by move and then destroys it in the first call, this cannot be done using [`hn!`] because
/// the `data` needs to be available for all event calls. In this case the closure is only called once, subsequent events
/// are ignored by the handler.
///
/// ```
/// # use zero_ui_core::gesture::ClickArgs;
/// # use zero_ui_core::handler::hn_once;
/// # fn assert_type() -> impl zero_ui_core::handler::WidgetHandler<ClickArgs> {
/// let data = vec![1, 2, 3];
/// # let
/// on_click = hn_once!(|_, _| {
///     for i in data {
///         print!("{}, ", i);
///     }
/// });
/// # on_click }
/// ```
///
/// Other then declaring a `FnOnce` this macro behaves like [`hn!`], so the same considerations apply. You can *clone-move* variables,
/// the type of the first closure input is `&mut WidgetContext` and is inferred automatically, the type if the second input is the event
/// arguments and must be annotated.
///
/// ```
/// # use zero_ui_core::gesture::ClickArgs;
/// # use zero_ui_core::handler::hn_once;
/// # fn assert_type() -> impl zero_ui_core::handler::WidgetHandler<ClickArgs> {
/// let data = vec![1, 2, 3];
/// # let
/// on_click = hn_once!(data, |ctx, args: &ClickArgs| {
///     drop(data);
/// });
///
///  println!("{:?}", data);
/// # on_click }
/// ```
#[macro_export]
macro_rules! hn_once {
    ($($tt:tt)+) => {
        $crate::handler::hn_once($crate::clone_move! { $($tt)+ })
    }
}
#[doc(inline)]
pub use crate::hn_once;

#[doc(hidden)]
pub struct AsyncFnMutWidgetHandler<H> {
    handler: H,
    tasks: Vec<WidgetTask<()>>,
}
impl<A, F, H> WidgetHandler<A> for AsyncFnMutWidgetHandler<H>
where
    A: Clone + 'static,
    F: Future<Output = ()> + 'static,
    H: FnMut(WidgetContextMut, A) -> F + 'static,
{
    fn subscribe(&self, widget_subscriptions: &mut WidgetSubscriptions) {
        for t in &self.tasks {
            t.subscribe(widget_subscriptions);
        }
    }

    fn event(&mut self, ctx: &mut WidgetContext, args: &A) -> bool {
        let handler = &mut self.handler;
        let mut task = WidgetTask::new(ctx, |ctx| handler(ctx, args.clone()));
        let need_update = task.update(ctx).is_none();
        if need_update {
            self.tasks.push(task);
            ctx.updates.subscriptions();
        }
        need_update
    }

    fn update(&mut self, ctx: &mut WidgetContext) -> bool {
        self.tasks.retain_mut(|t| t.update(ctx).is_none());
        !self.tasks.is_empty()
    }
}
impl<H> marker::AsyncHn for AsyncFnMutWidgetHandler<H> {}
#[doc(hidden)]
pub fn async_hn<A, F, H>(handler: H) -> AsyncFnMutWidgetHandler<H>
where
    A: Clone + 'static,
    F: Future<Output = ()> + 'static,
    H: FnMut(WidgetContextMut, A) -> F + 'static,
{
    AsyncFnMutWidgetHandler { handler, tasks: vec![] }
}

///<span data-inline></span> Declare an async *clone-move* event handler.
///
/// The macro input is a closure with optional *clone-move* variables, internally it uses [`async_clone_move_fn!`] so
/// the input is the same syntax.
///
/// # Examples
///
/// The example declares an async event handler for the `on_click` property.
///
/// ```
/// # use zero_ui_core::gesture::ClickArgs;
/// # use zero_ui_core::handler::async_hn;
/// # use zero_ui_core::task;
/// # fn assert_type() -> impl zero_ui_core::handler::WidgetHandler<ClickArgs> {
/// # let
/// on_click = async_hn!(|_, _| {
///     println!("Clicked!");
///
///     task::run(async {
///         println!("In other thread!");
///     }).await;
///
///     println!("Back in UI thread, in a widget update.");
/// });
/// # on_click }
/// ```
///
/// The closure input is `WidgetContextMut` for all handlers and `ClickArgs` for this property. Note that
/// if you want to use the event args you must annotate the input type, the context type is inferred.
///
/// ```
/// # use zero_ui_core::gesture::ClickArgs;
/// # use zero_ui_core::handler::async_hn;
/// # fn assert_type() -> impl zero_ui_core::handler::WidgetHandler<ClickArgs> {
/// # let
/// on_click = async_hn!(|ctx, args: ClickArgs| {
///     println!("Clicked {}!", args.click_count);
///     ctx.with(|c| {  });
/// });
/// # on_click }
/// ```
///
/// Internally the [`async_clone_move_fn!`] macro is used so you can *clone-move* variables into the handler.
///
/// ```
/// # use zero_ui_core::gesture::ClickArgs;
/// # use zero_ui_core::handler::async_hn;
/// # use zero_ui_core::var::{var, Var};
/// # use zero_ui_core::task;
/// # use zero_ui_core::text::formatx;
/// # fn assert_type() -> impl zero_ui_core::handler::WidgetHandler<ClickArgs> {
/// let enabled = var(true);
///
/// // ..
///
/// # let
/// on_click = async_hn!(enabled, |ctx, args: ClickArgs| {
///     enabled.set(&ctx, false);
///
///     task::run(async move {
///         println!("do something {}", args.click_count);
///     }).await;
///
///     enabled.set(&ctx, true);
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
/// listed in the *clone-move* section it would not be available after the handler is created. See [`async_clone_move_fn!`] for details.
///
/// The example also demonstrates a common pattern with async handlers, most events are only raised when the widget is enabled, so you can
/// disable the widget while the async task is running. This way you don't block the UI running a task but the user cannot spawn a second
/// task while the first is still running.
///
/// ## Futures and Clone-Move
///
/// You may want to always *clone-move* captures for async handlers, because they then automatically get cloned again for each event. This
/// needs to happen because you can have more then one *handler task* running at the same type, and both want access to the captured variables.
///
/// This second cloning can be avoided by using the [`async_hn_once!`] macro instead, but only if you expect a single event.
#[macro_export]
macro_rules! async_hn {
    ($($tt:tt)+) => {
        $crate::handler::async_hn($crate::async_clone_move_fn! { $($tt)+ })
    }
}
#[doc(inline)]
pub use crate::async_hn;

enum AsyncFnOnceWhState<H> {
    NotCalled(H),
    Pending(WidgetTask<()>),
    Done,
}
#[doc(hidden)]
pub struct AsyncFnOnceWidgetHandler<H> {
    state: AsyncFnOnceWhState<H>,
}
impl<A, F, H> WidgetHandler<A> for AsyncFnOnceWidgetHandler<H>
where
    A: Clone + 'static,
    F: Future<Output = ()> + 'static,
    H: FnOnce(WidgetContextMut, A) -> F + 'static,
{
    fn subscribe(&self, widget_subscriptions: &mut WidgetSubscriptions) {
        if let AsyncFnOnceWhState::Pending(t) = &self.state {
            t.subscribe(widget_subscriptions);
        }
    }

    fn event(&mut self, ctx: &mut WidgetContext, args: &A) -> bool {
        if let AsyncFnOnceWhState::NotCalled(handler) = mem::replace(&mut self.state, AsyncFnOnceWhState::Done) {
            let mut task = WidgetTask::new(ctx, |ctx| handler(ctx, args.clone()));
            let is_pending = task.update(ctx).is_none();
            if is_pending {
                self.state = AsyncFnOnceWhState::Pending(task);
                ctx.updates.subscriptions();
            }
            is_pending
        } else {
            false
        }
    }

    fn update(&mut self, ctx: &mut WidgetContext) -> bool {
        let mut is_pending = false;
        if let AsyncFnOnceWhState::Pending(t) = &mut self.state {
            is_pending = t.update(ctx).is_none();
            if !is_pending {
                self.state = AsyncFnOnceWhState::Done;
            }
        }
        is_pending
    }
}
impl<H> marker::AsyncHn for AsyncFnOnceWidgetHandler<H> {}
impl<H> marker::OnceHn for AsyncFnOnceWidgetHandler<H> {}
#[doc(hidden)]
pub fn async_hn_once<A, F, H>(handler: H) -> AsyncFnOnceWidgetHandler<H>
where
    A: Clone + 'static,
    F: Future<Output = ()> + 'static,
    H: FnOnce(WidgetContextMut, A) -> F + 'static,
{
    AsyncFnOnceWidgetHandler {
        state: AsyncFnOnceWhState::NotCalled(handler),
    }
}

///<span data-inline></span> Declare an async *clone-move* event handler that is only called once.
///
/// The macro input is a closure with optional *clone-move* variables, internally it uses [`async_clone_move_fn_once!`] so
/// the input is the same syntax.
///
/// # Example
///
/// The example captures `data` by move and then moves it again to another thread. This is not something you can do using [`async_hn!`]
/// because that handler expects to be called many times. We expect `on_open` to only be called once, so we can don't need to capture by
/// *clone-move* here just to use `data`.
///
/// ```
/// # use zero_ui_core::gesture::ClickArgs;
/// # use zero_ui_core::handler::async_hn_once;
/// # use zero_ui_core::task;
/// # fn assert_type() -> impl zero_ui_core::handler::WidgetHandler<ClickArgs> {
/// let data = vec![1, 2, 3];
/// # let
/// on_open = async_hn_once!(|_, _| {
///     task::run(async move {
///          for i in data {
///              print!("{}, ", i);
///          }    
///     }).await;
///
///     println!("Done!");
/// });
/// # on_open }
/// ```
///
/// You can still *clone-move* to have access to the variable after creating the handler, in this case the `data` will be cloned into the handler
/// but will just be moved to the other thread, avoiding a needless clone.
///
/// ```
/// # use zero_ui_core::gesture::ClickArgs;
/// # use zero_ui_core::handler::async_hn_once;
/// # use zero_ui_core::task;
/// # fn assert_type() -> impl zero_ui_core::handler::WidgetHandler<ClickArgs> {
/// let data = vec![1, 2, 3];
/// # let
/// on_open = async_hn_once!(data, |_, _| {
///     task::run(async move {
///          for i in data {
///              print!("{}, ", i);
///          }    
///     }).await;
///
///     println!("Done!");
/// });
/// println!("{:?}", data);
/// # on_open }
/// ```
#[macro_export]
macro_rules! async_hn_once {
    ($($tt:tt)+) => {
        $crate::handler::async_hn_once($crate::async_clone_move_fn_once! { $($tt)+ })
    }
}
#[doc(inline)]
pub use crate::async_hn_once;

/// Represents a weak handle to an [`AppHandler`] subscription.
pub trait AppWeakHandle: Send {
    /// Dynamic clone.
    fn clone_boxed(&self) -> Box<dyn AppWeakHandle>;

    /// Unsubscribes the [`AppHandler`].
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

/// Arguments for a call of [`AppHandler::event`].
pub struct AppHandlerArgs<'a> {
    /// Handle to the [`AppHandler`] subscription.
    pub handle: &'a dyn AppWeakHandle,
    /// If the handler is invoked in a *preview* context.
    pub is_preview: bool,
}

/// Represents an event handler in the app context.
///
/// There are different flavors of handlers, you can use macros to declare then.
/// See [`app_hn!`], [`app_hn_once!`] or [`async_app_hn!`], [`async_app_hn_once!`] to start.
pub trait AppHandler<A: Clone + 'static>: 'static {
    /// Called every time the event happens.
    ///
    /// The `handler_args` can be used to unsubscribe the handler. Async handlers are expected to schedule
    /// their tasks to run somewhere in the app, usually in the [`Updates::on_update`]. The `handle` is
    /// **not** expected to cancel running async tasks, only to drop `self` before the next event happens.
    ///
    /// [`Updates::on_update`]: crate::context::Updates::on_update
    fn event(&mut self, ctx: &mut AppContext, args: &A, handler_args: &AppHandlerArgs);
}

#[doc(hidden)]
pub struct FnMutAppHandler<H> {
    handler: H,
}
impl<A, H> AppHandler<A> for FnMutAppHandler<H>
where
    A: Clone + 'static,
    H: FnMut(&mut AppContext, &A, &dyn AppWeakHandle) + 'static,
{
    fn event(&mut self, ctx: &mut AppContext, args: &A, handler_args: &AppHandlerArgs) {
        (self.handler)(ctx, args, handler_args.handle);
    }
}
impl<H> marker::NotAsyncHn for FnMutAppHandler<H> {}
#[doc(hidden)]
pub fn app_hn<A, H>(handler: H) -> FnMutAppHandler<H>
where
    A: Clone + 'static,
    H: FnMut(&mut AppContext, &A, &dyn AppWeakHandle) + 'static,
{
    FnMutAppHandler { handler }
}

///<span data-inline></span> Declare a mutable *clone-move* app event handler.
///
/// The macro input is a closure with optional *clone-move* variables, internally it uses [`clone_move!`] so
/// the input is the same syntax.
///
/// # Examples
///
/// The example declares an event handler for the `ClickEvent`.
///
/// ```
/// # use zero_ui_core::gesture::ClickEvent;
/// # use zero_ui_core::handler::app_hn;
/// # use zero_ui_core::context::AppContext;
/// # fn assert_type(ctx: &mut AppContext) {
/// ctx.events.on_event(ClickEvent, app_hn!(|_, _, _| {
///     println!("Clicked Somewhere!");
/// }));
/// # }
/// ```
///
/// The closure input is `&mut AppContext, &A, &dyn AppWeakHandle` with `&A` equaling `&ClickArgs` for this event. Note that
/// if you want to use the event args you must annotate the input type, the context and handle type is inferred.
///
/// The handle can be used to unsubscribe the event handler, if [`unsubscribe`](AppWeakHandle::unsubscribe) is called the handler
/// will be dropped some time before the next event update.
///
/// ```
/// # use zero_ui_core::gesture::{ClickEvent, ClickArgs};
/// # use zero_ui_core::handler::app_hn;
/// # use zero_ui_core::context::AppContext;
/// # fn assert_type(ctx: &mut AppContext) {
/// ctx.events.on_event(ClickEvent, app_hn!(|ctx, args: &ClickArgs, handle| {
///     println!("Clicked {}!", args.target);
///     let _ = ctx.services;
///     handle.unsubscribe();
/// }));
/// # }
/// ```
///
/// Internally the [`clone_move!`] macro is used so you can *clone-move* variables into the handler.
///
/// ```
/// # use zero_ui_core::gesture::{ClickEvent, ClickArgs};
/// # use zero_ui_core::text::{formatx, ToText};
/// # use zero_ui_core::var::{var, Var};
/// # use zero_ui_core::context::AppContext;
/// # use zero_ui_core::handler::app_hn;
/// # fn assert_type(ctx: &mut AppContext) {
/// let foo = var("".to_text());
///
/// ctx.events.on_event(ClickEvent, app_hn!(foo, |ctx, args: &ClickArgs, _| {
///     foo.set(ctx, args.target.to_text());
/// }));
///
/// // can still use after:
/// let bar = foo.map(|c| formatx!("last click: {}", c));
///
/// # }
/// ```
///
/// In the example above only a clone of `foo` is moved into the handler. Note that handlers always capture by move, if `foo` was not
/// listed in the *clone-move* section it would not be available after the handler is created. See [`clone_move!`] for details.
#[macro_export]
macro_rules! app_hn {
    ($($tt:tt)+) => {
        $crate::handler::app_hn($crate::clone_move!{ $($tt)+ })
    }
}
#[doc(inline)]
pub use crate::app_hn;

#[doc(hidden)]
pub struct FnOnceAppHandler<H> {
    handler: Option<H>,
}
impl<A, H> AppHandler<A> for FnOnceAppHandler<H>
where
    A: Clone + 'static,
    H: FnOnce(&mut AppContext, &A) + 'static,
{
    fn event(&mut self, ctx: &mut AppContext, args: &A, handler_args: &AppHandlerArgs) {
        if let Some(handler) = self.handler.take() {
            handler(ctx, args);
            handler_args.handle.unsubscribe();
        } else {
            tracing::error!("`app_hn_once!` called after requesting unsubscribe");
        }
    }
}
impl<H> marker::OnceHn for FnOnceAppHandler<H> {}
impl<H> marker::NotAsyncHn for FnOnceAppHandler<H> {}
#[doc(hidden)]
pub fn app_hn_once<A, H>(handler: H) -> FnOnceAppHandler<H>
where
    A: Clone + 'static,
    H: FnOnce(&mut AppContext, &A) + 'static,
{
    FnOnceAppHandler { handler: Some(handler) }
}

///<span data-inline></span> Declare a *clone-move* app event handler that is only called once.
///
/// The macro input is a closure with optional *clone-move* variables, internally it uses [`clone_move!`] so
/// the input is the same syntax.
///
/// # Examples
///
/// The example captures `data` by move and then destroys it in the first call, this cannot be done using [`app_hn!`] because
/// the `data` needs to be available for all event calls. In this case the closure is only called once, subsequent events
/// are ignored by the handler and it automatically requests unsubscribe.
///
/// ```
/// # use zero_ui_core::gesture::ClickEvent;
/// # use zero_ui_core::handler::app_hn_once;
/// # use zero_ui_core::context::AppContext;
/// # fn assert_type(ctx: &mut AppContext) {
/// let data = vec![1, 2, 3];
///
/// ctx.events.on_event(ClickEvent, app_hn_once!(|_, _| {
///     for i in data {
///         print!("{}, ", i);
///     }
/// }));
/// # }
/// ```
///
/// Other then declaring a `FnOnce` this macro behaves like [`app_hn!`], so the same considerations apply. You can *clone-move* variables,
/// the type of the first closure input is `&mut AppContext` and is inferred automatically, the type if the second input is the event
/// arguments and must be annotated.
///
/// ```
/// # use zero_ui_core::gesture::{ClickArgs, ClickEvent};
/// # use zero_ui_core::handler::app_hn_once;
/// # use zero_ui_core::context::AppContext;
/// # fn assert_type(ctx: &mut AppContext) {
/// let data = vec![1, 2, 3];
///
/// ctx.events.on_event(ClickEvent, app_hn_once!(data, |ctx, args: &ClickArgs| {
///     drop(data);
/// }));
///
///  println!("{:?}", data);
/// # }
/// ```
#[macro_export]
macro_rules! app_hn_once {
    ($($tt:tt)+) => {
        $crate::handler::app_hn_once($crate::clone_move! { $($tt)+ })
    }
}
#[doc(inline)]
pub use crate::app_hn_once;

#[doc(hidden)]
pub struct AsyncFnMutAppHandler<H> {
    handler: H,
}
impl<A, F, H> AppHandler<A> for AsyncFnMutAppHandler<H>
where
    A: Clone + 'static,
    F: Future<Output = ()> + 'static,
    H: FnMut(AppContextMut, A, Box<dyn AppWeakHandle>) -> F + 'static,
{
    fn event(&mut self, ctx: &mut AppContext, args: &A, handler_args: &AppHandlerArgs) {
        let handler = &mut self.handler;
        let mut task = AppTask::new(ctx, |ctx| handler(ctx, args.clone(), handler_args.handle.clone_boxed()));
        if task.update(ctx).is_none() {
            if handler_args.is_preview {
                ctx.updates
                    .on_pre_update(app_hn!(|ctx, _, handle| {
                        if task.update(ctx).is_some() {
                            handle.unsubscribe();
                        }
                    }))
                    .permanent();
            } else {
                ctx.updates
                    .on_update(app_hn!(|ctx, _, handle| {
                        if task.update(ctx).is_some() {
                            handle.unsubscribe();
                        }
                    }))
                    .permanent();
            }
        }
    }
}
impl<H> marker::AsyncHn for AsyncFnMutAppHandler<H> {}
#[doc(hidden)]
pub fn async_app_hn<A, F, H>(handler: H) -> AsyncFnMutAppHandler<H>
where
    A: Clone + 'static,
    F: Future<Output = ()> + 'static,
    H: FnMut(AppContextMut, A, Box<dyn AppWeakHandle>) -> F + 'static,
{
    AsyncFnMutAppHandler { handler }
}

///<span data-inline></span> Declare an async *clone-move* app event handler.
///
/// The macro input is a closure with optional *clone-move* variables, internally it uses [`async_clone_move_fn!`] so
/// the input is the same syntax.
///
/// The handler generates a future for each event, the future is polled immediately if it does not finish it is scheduled
/// to update in [`on_pre_update`](crate::context::Updates::on_pre_update) or [`on_update`](crate::context::Updates::on_update) depending
/// on if the handler was assigned to a *preview* event or not.
///
/// Note that this means [`stop_propagation`](crate::event::EventArgs::stop_propagation) can only be meaningfully called before the
/// first `.await`, after the event has already propagated.
///
/// # Examples
///
/// The example declares an async event handler for the `ClickEvent`.
///
/// ```
/// # use zero_ui_core::gesture::ClickEvent;
/// # use zero_ui_core::handler::async_app_hn;
/// # use zero_ui_core::context::AppContext;
/// # use zero_ui_core::task;
/// # fn assert_type(ctx: &mut AppContext) {
/// ctx.events.on_event(ClickEvent, async_app_hn!(|_, _, _| {
///     println!("Clicked Somewhere!");
///
///     task::run(async {
///         println!("In other thread!");
///     }).await;
///
///     println!("Back in UI thread, in an app update.");
/// }));
/// # }
/// ```
///
/// The closure input is `AppContextMut, A, Box<dyn AppWeakHandle>` for all handlers and `ClickArgs` for this property. Note that
/// if you want to use the event args you must annotate the input type, the context and handle types are inferred.
///
/// The handle can be used to unsubscribe the event handler, if [`unsubscribe`](AppWeakHandle::unsubscribe) is called the handler
/// will be dropped some time before the next event update. Running tasks are not canceled by unsubscribing, the only way to *cancel*
/// then is by returning early inside the async blocks.
///
/// ```
/// # use zero_ui_core::gesture::{ClickArgs, ClickEvent};
/// # use zero_ui_core::handler::async_app_hn;
/// # use zero_ui_core::context::AppContext;
/// # use zero_ui_core::task;
/// # fn assert_type(ctx: &mut AppContext) {
/// ctx.events.on_event(ClickEvent, async_app_hn!(|ctx, args: ClickArgs, handle| {
///     println!("Clicked {}!", args.target);
///     ctx.with(|c| {  });
///     task::run(async move {
///         handle.unsubscribe();
///     });
/// }));
/// # }
/// ```
///
/// Internally the [`async_clone_move_fn!`] macro is used so you can *clone-move* variables into the handler.
///
/// ```
/// # use zero_ui_core::gesture::{ClickArgs, ClickEvent};
/// # use zero_ui_core::handler::async_app_hn;
/// # use zero_ui_core::context::AppContext;
/// # use zero_ui_core::var::{var, Var};
/// # use zero_ui_core::task;
/// # use zero_ui_core::text::{formatx, ToText};
/// # fn assert_type(ctx: &mut AppContext) {
/// let status = var("pending..".to_text());
///
/// ctx.events.on_event(ClickEvent, async_app_hn!(status, |ctx, args: ClickArgs, _| {
///     status.set(&ctx, formatx!("processing {}..", args.target));
///
///     task::run(async move {
///         println!("do something slow");
///     }).await;
///
///     status.set(&ctx, formatx!("finished {}", args.target));
/// }));
///
/// // can still use after:
/// let text = status;
///
/// # }
/// ```
///
/// In the example above only a clone of `status` is moved into the handler. Note that handlers always capture by move, if `status` was not
/// listed in the *clone-move* section it would not be available after the handler is created. See [`async_clone_move_fn!`] for details.
///
/// ## Futures and Clone-Move
///
/// You may want to always *clone-move* captures for async handlers, because they then automatically get cloned again for each event. This
/// needs to happen because you can have more then one *handler task* running at the same type, and both want access to the captured variables.
///
/// This second cloning can be avoided by using the [`async_hn_once!`] macro instead, but only if you expect a single event.
#[macro_export]
macro_rules! async_app_hn {
    ($($tt:tt)+) => {
        $crate::handler::async_app_hn($crate::async_clone_move_fn! { $($tt)+ })
    }
}
#[doc(inline)]
pub use crate::async_app_hn;

#[doc(hidden)]
pub struct AsyncFnOnceAppHandler<H> {
    handler: Option<H>,
}

impl<A, F, H> AppHandler<A> for AsyncFnOnceAppHandler<H>
where
    A: Clone + 'static,
    F: Future<Output = ()> + 'static,
    H: FnOnce(AppContextMut, A) -> F + 'static,
{
    fn event(&mut self, ctx: &mut AppContext, args: &A, handler_args: &AppHandlerArgs) {
        if let Some(handler) = self.handler.take() {
            handler_args.handle.unsubscribe();

            let mut task = AppTask::new(ctx, |ctx| handler(ctx, args.clone()));
            if task.update(ctx).is_none() {
                if handler_args.is_preview {
                    ctx.updates
                        .on_pre_update(app_hn!(|ctx, _, handle| {
                            if task.update(ctx).is_some() {
                                handle.unsubscribe();
                            }
                        }))
                        .permanent();
                } else {
                    ctx.updates
                        .on_update(app_hn!(|ctx, _, handle| {
                            if task.update(ctx).is_some() {
                                handle.unsubscribe();
                            }
                        }))
                        .permanent();
                }
            }
        } else {
            tracing::error!("`async_app_hn_once!` called after requesting unsubscribe");
        }
    }
}
impl<H> marker::AsyncHn for AsyncFnOnceAppHandler<H> {}
impl<H> marker::OnceHn for AsyncFnOnceAppHandler<H> {}
#[doc(hidden)]
pub fn async_app_hn_once<A, F, H>(handler: H) -> AsyncFnOnceAppHandler<H>
where
    A: Clone + 'static,
    F: Future<Output = ()> + 'static,
    H: FnOnce(AppContextMut, A) -> F + 'static,
{
    AsyncFnOnceAppHandler { handler: Some(handler) }
}

///<span data-inline></span> Declare an async *clone-move* app event handler that is only called once.
///
/// The macro input is a closure with optional *clone-move* variables, internally it uses [`async_clone_move_fn_once!`] so
/// the input is the same syntax.
///
/// # Example
///
/// The example captures `data` by move and then moves it again to another thread. This is not something you can do using [`async_app_hn!`]
/// because that handler expects to be called many times. We want to handle `ClickEvent` once in this example, so we can don't need
/// to capture by *clone-move* just to use `data`.
///
/// ```
/// # use zero_ui_core::gesture::ClickArgs;
/// # use zero_ui_core::handler::async_hn_once;
/// # use zero_ui_core::task;
/// # fn assert_type() -> impl zero_ui_core::handler::WidgetHandler<ClickArgs> {
/// let data = vec![1, 2, 3];
/// # let
/// on_open = async_hn_once!(|_, _| {
///     task::run(async move {
///          for i in data {
///              print!("{}, ", i);
///          }    
///     }).await;
///
///     println!("Done!");
/// });
/// # on_open }
/// ```
///
/// You can still *clone-move* to have access to the variable after creating the handler, in this case the `data` will be cloned into the handler
/// but will just be moved to the other thread, avoiding a needless clone.
///
/// ```
/// # use zero_ui_core::gesture::ClickArgs;
/// # use zero_ui_core::handler::async_hn_once;
/// # use zero_ui_core::task;
/// # fn assert_type() -> impl zero_ui_core::handler::WidgetHandler<ClickArgs> {
/// let data = vec![1, 2, 3];
/// # let
/// on_open = async_hn_once!(data, |_, _| {
///     task::run(async move {
///          for i in data {
///              print!("{}, ", i);
///          }    
///     }).await;
///
///     println!("Done!");
/// });
/// println!("{:?}", data);
/// # on_open }
/// ```
#[macro_export]
macro_rules! async_app_hn_once {
    ($($tt:tt)+) => {
        $crate::handler::async_app_hn_once($crate::async_clone_move_fn_once! { $($tt)+ })
    }
}
#[doc(inline)]
pub use crate::async_app_hn_once;

///<span data-inline></span> Cloning closure.
///
/// A common pattern when creating `'static` closures is to capture clones by `move`, this way the closure is `'static`
/// and the cloned values are still available after creating the closure. This macro facilitates this pattern.
///
/// # Example
///
/// In the example `bar` was *clone-moved* into the `'static` closure given to `foo`.
///
/// ```
/// # use zero_ui_core::handler::clone_move;
/// fn foo(mut f: impl FnMut(bool) + 'static) {
///     f(true);
/// }
///
/// let bar = "Cool!".to_owned();
/// foo(clone_move!(bar, |p| {
///     if p { println!("cloned: {}", bar) }
/// }));
///
/// println!("original: {}", bar);
/// ```
///
/// Expands to:
///
/// ```
/// # use zero_ui_core::handler::clone_move;
/// # fn foo(mut f: impl FnMut(bool) + 'static) {
/// #     f(true);
/// # }
/// # let bar = "Cool!".to_owned();
/// foo({
///     let bar = bar.clone();
///     move |p| {
///         if p { println!("cloned: {}", bar) }
///     }
/// });
/// # println!("original: {}", bar);
/// ```
///
/// # Other Patterns
///
/// Sometimes you want to clone an *inner deref* of the value, or you want the clone to be `mut`, you can annotate the
/// variables cloned to achieve these effects.
///
/// ```
/// # use zero_ui_core::handler::clone_move;
/// # use std::rc::Rc;
/// fn foo(mut f: impl FnMut(bool) + 'static) {
///     f(true);
/// }
///
/// let bar = Rc::new("Cool!".to_string());
/// foo(clone_move!(mut *bar, |p| {
///     bar.push_str("!!");
///     if p { println!("cloned String not Rc: {}", bar) }
/// }));
///
/// println!("original: {}", bar);
/// ```
///
/// Expands to:
///
/// ```
/// # use zero_ui_core::handler::clone_move;
/// # use std::rc::Rc;
/// # fn foo(mut f: impl FnMut(bool) + 'static) {
/// #     f(true);
/// # }
/// # let bar = Rc::new("Cool!".to_string());
/// foo({
///     let mut bar = (*bar).clone();
///     move |p| {
///         bar.push_str("!!");
///         if p { println!("cloned String not Rc: {}", bar) }
///     }
///});
/// # println!("original: {}", bar);
/// ```
///
/// # Async
///
/// See [`async_clone_move_fn!`](macro@crate::async_clone_move_fn) for creating `async` closures.
#[macro_export]
macro_rules! clone_move {
    ($($tt:tt)+) => { $crate::__clone_move! { [][][] $($tt)+ } }
}
#[doc(inline)]
pub use crate::clone_move;
#[doc(hidden)]
#[macro_export]
macro_rules! __clone_move {
    // match start of mut var
    ([$($done:tt)*][][] mut $($rest:tt)+) => {
        $crate::__clone_move! {
            [$($done)*]
            [mut]
            []
            $($rest)+
        }
    };

    // match one var deref (*)
    ([$($done:tt)*][$($mut:tt)?][$($deref:tt)*] * $($rest:tt)+) => {
        $crate::__clone_move! {
            [$($done)*]
            [$($mut)?]
            [$($deref)* *]
            $($rest)+
        }
    };

    // match end of a variable
    ([$($done:tt)*][$($mut:tt)?][$($deref:tt)*] $var:ident, $($rest:tt)+) => {
        $crate::__clone_move! {
            [
                $($done)*
                let $($mut)? $var = ( $($deref)* $var ).clone();
            ]
            []
            []
            $($rest)+
        }
    };

    // match start of closure
    ([$($done:tt)*][][] | $($rest:tt)+) => {
        {
            $($done)*
            move | $($rest)+
        }
    };

    // match start of closure without input
    ([$($done:tt)*][][] || $($rest:tt)+) => {
        {
            $($done)*
            move || $($rest)+
        }
    };
}

/// <span data-inline></span> Cloning async move block.
#[macro_export]
macro_rules! async_clone_move {
    ($($tt:tt)+) => {
        $crate::__async_clone_move! { [][][] $($tt)+ }
    }
}
#[doc(inline)]
pub use crate::async_clone_move;

#[doc(hidden)]
#[macro_export]
macro_rules! __async_clone_move {
    // match start of mut var
    ([$($done:tt)*][][] mut $($rest:tt)+) => {
        $crate::__async_clone_move! {
            [$($done)*]
            [mut]
            []
            $($rest)+
        }
    };

    // match one var deref (*)
    ([$($done:tt)*][$($mut:tt)?][$($deref:tt)*] * $($rest:tt)+) => {
        $crate::__async_clone_move! {
            [$($done)*]
            [$($mut)?]
            [$($deref)* *]
            $($rest)+
        }
    };

    // match end of a variable
    ([$($done:tt)*][$($mut:tt)?][$($deref:tt)*] $var:ident, $($rest:tt)+) => {
        $crate::__async_clone_move! {
            [
                $($done)*
                let $($mut)? $var = ( $($deref)* $var ).clone();
            ]
            []
            []
            $($rest)+
        }
    };

    // match block
    ([$($done:tt)*][][] { $($block:tt)+ }) => {
        {
            $($done)*
            async move { $($block)+ }
        }
    };
}

///<span data-inline></span> Cloning async closure.
///
/// This macro syntax is exactly the same as [`clone_move!`](macro@crate::clone_move), but it expands to an *async closure* that
/// captures a clone of zero or more variables and moves another clone of these variables into the returned future for each call.
///
/// # Example
///
/// In the example `bar` is cloned into the closure and then it is cloned again for each future generated by the closure.
///
/// ```
/// # use zero_ui_core::handler::async_clone_move_fn;
/// # use std::future::Future;
/// async fn foo<F: Future<Output=()>, H:  FnMut(bool) -> F + 'static>(mut f: H) {
///     f(true).await;
/// }
///
/// let bar = "Cool!".to_owned();
/// foo(async_clone_move_fn!(bar, |p| {
///     std::future::ready(()).await;
///     if p { println!("cloned: {}", bar) }
/// }));
///
/// println!("original: {}", bar);
/// ```
///
/// Expands to:
///
/// ```
/// # use zero_ui_core::handler::async_clone_move_fn;
/// # use std::future::Future;
/// # async fn foo<F: Future<Output=()>, H:  FnMut(bool) -> F + 'static>(mut f: H) {
/// #     f(true).await;
/// # }
/// # let bar = "Cool!".to_owned();
/// foo({
///     let bar = bar.clone();
///     move |p| {
///         let bar = bar.clone();
///         async move {
///             std::future::ready(()).await;
///             if p { println!("cloned: {}", bar) }
///         }
///     }
/// });
/// # println!("original: {}", bar);
/// ```
#[macro_export]
macro_rules! async_clone_move_fn {
    ($($tt:tt)+) => { $crate::__async_clone_move_fn! { [{}{}][][] $($tt)+ } }
}
#[doc(inline)]
pub use crate::async_clone_move_fn;
#[doc(hidden)]
#[macro_export]
macro_rules! __async_clone_move_fn {
    // match start of mut var
    ([$($done:tt)*][][] mut $($rest:tt)+) => {
        $crate::__async_clone_move_fn! {
            [$($done)*]
            [mut]
            []
            $($rest)+
        }
    };

    // match one var deref (*)
    ([$($done:tt)*][$($mut:tt)?][$($deref:tt)*] * $($rest:tt)+) => {
        $crate::__async_clone_move_fn! {
            [$($done)*]
            [$($mut)?]
            [$($deref)* *]
            $($rest)+
        }
    };

    // match end of a variable
    ([$($done:tt)*][$($mut:tt)?][$($deref:tt)*] $var:ident, $($rest:tt)+) => {
        $crate::__async_clone_move_fn! {
            @var
            [$($done)*]
            [$($mut)?]
            [$($deref)*]
            $var,
            $($rest)+
        }
    };

    // include one var
    (@var [ { $($closure_clones:tt)* }{ $($async_clones:tt)* } ][$($mut:tt)?][$($deref:tt)*] $var:ident, $($rest:tt)+) => {
        $crate::__async_clone_move_fn! {
            [
                {
                    $($closure_clones)*
                    let $var = ( $($deref)* $var ).clone();
                }
                {
                    $($async_clones)*
                    let $($mut)? $var = $var.clone();
                }
            ]
            []
            []
            $($rest)+
        }
    };

    // match start of closure inputs
    ([$($done:tt)*][][] | $($rest:tt)+) => {
        $crate::__async_clone_move_fn! {
            @args
            [$($done)*]
            []
            $($rest)+
        }
    };

    // match start of closure without input, the closure body is in a block
    ([ { $($closure_clones:tt)* }{ $($async_clones:tt)* } ][][] || { $($rest:tt)+ }) => {
        {
            $($closure_clones)*
            move || {
                $($async_clones)*
                async move {
                    $($rest)+
                }
            }
        }
    };
    // match start of closure without input, the closure body is **not** in a block
    ([ { $($closure_clones:tt)* }{ $($async_clones:tt)* } ][][] || $($rest:tt)+ ) => {
        {
            $($closure_clones)*
            move || {
                $($async_clones)*
                async move {
                    $($rest)+
                }
            }
        }
    };

    // match end of closure inputs, the closure body is in a block
    (@args [  { $($closure_clones:tt)* }{ $($async_clones:tt)* } ] [$($args:tt)*] | { $($rest:tt)+ }) => {
        {
            $($closure_clones)*
            move |$($args)*| {
                $($async_clones)*
                async move {
                    $($rest)+
                }
            }
        }
    };
    // match end of closure inputs, the closure body is in a block
    (@args [  { $($closure_clones:tt)* }{ $($async_clones:tt)* } ] [$($args:tt)*] | $($rest:tt)+) => {
        {
            $($closure_clones)*
            move |$($args)*| {
                $($async_clones)*
                async move {
                    $($rest)+
                }
            }
        }
    };

    // match a token in closure inputs
    (@args [$($done:tt)*] [$($args:tt)*] $arg_tt:tt $($rest:tt)+) => {
        $crate::__async_clone_move_fn! {
            @args
            [$($done)*]
            [$($args)* $arg_tt]
            $($rest)+
        }
    };
}

///<span data-inline></span> Cloning async closure that can only be called once.
///
/// This macro syntax is exactly the same as [`async_clone_move_fn!`](macro@crate::async_clone_move_fn), but it does not clone variables
/// again inside the call to move to the returned future. Because if moves the captured variables to the closure returned `Future`
/// it can only be `FnOnce`.
///
/// # Example
///
/// In the example `bar` is cloned into the closure and then moved to the future generated by the closure.
///
/// ```
/// # use zero_ui_core::handler::async_clone_move_fn;
/// # use std::future::Future;
/// async fn foo<F: Future<Output=()>, H:  FnOnce(bool) -> F + 'static>(mut f: H) {
///     f(true).await;
/// }
///
/// let bar = "Cool!".to_owned();
/// foo(async_clone_move_fn!(bar, |p| {
///     std::future::ready(()).await;
///     if p { println!("cloned: {}", bar) }
/// }));
///
/// println!("original: {}", bar);
/// ```
///
/// Expands to:
///
/// ```
/// # use zero_ui_core::handler::async_clone_move_fn;
/// # use std::future::Future;
/// # async fn foo<F: Future<Output=()>, H:  FnOnce(bool) -> F + 'static>(mut f: H) {
/// #     f(true).await;
/// # }
/// # let bar = "Cool!".to_owned();
/// foo({
///     let bar = bar.clone();
///     move |p| async move {
///         std::future::ready(()).await;
///         if p { println!("cloned: {}", bar) }
///     }
/// });
/// # println!("original: {}", bar);
/// ```
#[macro_export]
macro_rules! async_clone_move_fn_once {
    ($($tt:tt)+) => { $crate::__async_clone_move_fn_once! { [][][] $($tt)+ } }
}
#[doc(inline)]
pub use crate::async_clone_move_fn_once;
#[doc(hidden)]
#[macro_export]
macro_rules! __async_clone_move_fn_once {
    // match start of mut var
    ([$($done:tt)*][][] mut $($rest:tt)+) => {
        $crate::__async_clone_move_fn_once! {
            [$($done)*]
            [mut]
            []
            $($rest)+
        }
    };

    // match one var deref (*)
    ([$($done:tt)*][$($mut:tt)?][$($deref:tt)*] * $($rest:tt)+) => {
        $crate::__async_clone_move_fn_once! {
            [$($done)*]
            [$($mut)?]
            [$($deref)* *]
            $($rest)+
        }
    };

    // match end of a variable
    ([$($done:tt)*][$($mut:tt)?][$($deref:tt)*] $var:ident, $($rest:tt)+) => {
        $crate::__async_clone_move_fn_once! {
            [
                $($done)*
                let $($mut)? $var = ( $($deref)* $var ).clone();
            ]
            []
            []
            $($rest)+
        }
    };

    // match start of closure inputs
    ([$($done:tt)*][][] | $($rest:tt)+) => {
        $crate::__async_clone_move_fn_once! {
            @args
            [$($done)*]
            []
            $($rest)+
        }
    };

    // match start of closure without input, the closure body is in a block
    ([$($done:tt)*][][] || { $($rest:tt)+ }) => {
        {
            $($done)*
            move || {
                async move {
                    $($rest)+
                }
            }
        }
    };
    // match start of closure without input, the closure body is **not** in a block
    ([$($done:tt)*][][] || $($rest:tt)+ ) => {
        {
            $($done)*
            move || {
                async move {
                    $($rest)+
                }
            }
        }
    };

    // match end of closure inputs, the closure body is in a block
    (@args [$($done:tt)*] [$($args:tt)*] | { $($rest:tt)+ }) => {
        {
            $($done)*
            move |$($args)*| {
                async move {
                    $($rest)+
                }
            }
        }
    };
    // match end of closure inputs, the closure body is in a block
    (@args [$($done:tt)*] [$($args:tt)*] | $($rest:tt)+) => {
        {
            $($done)*
            move |$($args)*| {
                async move {
                    $($rest)+
                }
            }
        }
    };

    // match a token in closure inputs
    (@args [$($done:tt)*] [$($args:tt)*] $arg_tt:tt $($rest:tt)+) => {
        $crate::__async_clone_move_fn_once! {
            @args
            [$($done)*]
            [$($args)* $arg_tt]
            $($rest)+
        }
    };
}

#[cfg(any(test, doc, feature = "test_util"))]
impl crate::context::TestWidgetContext {
    /// Calls a [`WidgetHandler<A>`] once and blocks until the handler task is complete.
    ///
    /// This function *spins* until the handler returns `false` from [`WidgetHandler::update`]. The updates
    /// are *applied* after each try or until the `timeout` is reached. Returns an error if the `timeout` is reached.
    pub fn block_on<A>(&mut self, handler: &mut dyn WidgetHandler<A>, args: &A, timeout: Duration) -> Result<(), String>
    where
        A: Clone + 'static,
    {
        self.block_on_multi(vec![handler], args, timeout)
    }

    /// Calls multiple [`WidgetHandler<A>`] once each and blocks until all handler tasks are complete.
    ///
    /// This function *spins* until the handler returns `false` from [`WidgetHandler::update`] in all handlers. The updates
    /// are *applied* after each try or until the `timeout` is reached. Returns an error if the `timeout` is reached.
    pub fn block_on_multi<A>(&mut self, mut handlers: Vec<&mut dyn WidgetHandler<A>>, args: &A, timeout: Duration) -> Result<(), String>
    where
        A: Clone + 'static,
    {
        self.widget_context(|ctx| handlers.retain_mut(|h| h.event(ctx, args)));
        if !handlers.is_empty() {
            if !self.apply_updates().1.has_updates() {
                thread::yield_now();
            }
            let start_time = Instant::now();
            #[allow(clippy::blocks_in_if_conditions)] // false positive, see https://github.com/rust-lang/rust-clippy/issues/7580
            while {
                self.widget_context(|ctx| handlers.retain_mut(|h| h.update(ctx)));
                !handlers.is_empty()
            } {
                if Instant::now().duration_since(start_time) >= timeout {
                    return Err(format!(
                        "TestWidgetContext::block_on reached timeout of {:?} before the handler task could finish",
                        timeout
                    ));
                }

                if !self.apply_updates().1.has_updates() {
                    thread::yield_now();
                }
            }
        }
        Ok(())
    }

    /// Calls the `handler` once and [`block_on`] it with a 1 second timeout.
    ///
    /// [`block_on`]: Self::block_on.
    #[track_caller]
    pub fn doc_test<A, H>(args: A, mut handler: H)
    where
        A: Clone + 'static,
        H: WidgetHandler<A>,
    {
        Self::new().block_on(&mut handler, &args, DOC_TEST_BLOCK_ON_TIMEOUT).unwrap()
    }

    /// Calls the `handlers` once each and [`block_on_multi`] with a 1 second timeout.
    ///
    /// [`block_on_multi`]: Self::block_on_multi.
    #[track_caller]
    pub fn doc_test_multi<A>(args: A, mut handlers: Vec<Box<dyn WidgetHandler<A>>>)
    where
        A: Clone + 'static,
    {
        Self::new()
            .block_on_multi(handlers.iter_mut().map(|h| h.as_mut()).collect(), &args, DOC_TEST_BLOCK_ON_TIMEOUT)
            .unwrap()
    }
}

impl HeadlessApp {
    /// Calls a [`AppHandler<A>`] once and blocks until the update tasks started during the call complete.
    ///
    /// This function *spins* until all [update tasks] are completed. Timers or send events can
    /// be received during execution but the loop does not sleep, it just spins requesting an update
    /// for each pass.
    pub fn block_on<A>(&mut self, handler: &mut dyn AppHandler<A>, args: &A, timeout: Duration) -> Result<(), String>
    where
        A: Clone + 'static,
    {
        self.block_on_multi(vec![handler], args, timeout)
    }

    /// Calls multiple [`AppHandler<A>`] once each and blocks until all update tasks are complete.
    ///
    /// This function *spins* until all [update tasks] are completed. Timers or send events can
    /// be received during execution but the loop does not sleep, it just spins requesting an update
    /// for each pass.
    pub fn block_on_multi<A>(&mut self, handlers: Vec<&mut dyn AppHandler<A>>, args: &A, timeout: Duration) -> Result<(), String>
    where
        A: Clone + 'static,
    {
        let (pre_len, pos_len) = self.ctx().updates.handler_lens();

        let handler_args = AppHandlerArgs {
            handle: &Handle::dummy(()).downgrade(),
            is_preview: false,
        };
        for handler in handlers {
            handler.event(&mut self.ctx(), args, &handler_args);
        }

        let mut pending = self.ctx().updates.new_update_handlers(pre_len, pos_len);

        if !pending.is_empty() {
            let start_time = Instant::now();
            #[allow(clippy::blocks_in_if_conditions)] // false positive, see https://github.com/rust-lang/rust-clippy/issues/7580
            while {
                pending.retain(|h| h());
                !pending.is_empty()
            } {
                self.ctx().updates.update_ext();
                let flow = self.update(false);
                if Instant::now().duration_since(start_time) >= timeout {
                    return Err(format!(
                        "TestWidgetContext::block_on reached timeout of {:?} before the handler task could finish",
                        timeout
                    ));
                }

                use crate::app::ControlFlow;
                match flow {
                    ControlFlow::Poll => continue,
                    ControlFlow::Wait => {
                        thread::yield_now();
                        continue;
                    }
                    ControlFlow::Exit => return Ok(()),
                }
            }
        }

        Ok(())
    }

    /// Polls a `future` and updates the app repeatedly until it completes or the `timeout` is reached.
    pub fn block_on_fut<F: Future>(&mut self, future: F, timeout: Duration) -> Result<F::Output, String> {
        let slot = UpdateSlot::next();
        let waker = self.ctx().updates.sender().waker(slot);
        let mut cx = std::task::Context::from_waker(&waker);
        let start_time = Instant::now();
        crate::task::pin!(future);
        loop {
            if start_time.elapsed() >= timeout {
                return Err(format!("reached timeout `{:?}`", timeout));
            }
            match future.as_mut().poll(&mut cx) {
                std::task::Poll::Ready(r) => {
                    return Ok(r);
                }
                std::task::Poll::Pending => match self.update(false) {
                    crate::app::ControlFlow::Poll => continue,
                    crate::app::ControlFlow::Wait => {
                        thread::yield_now();
                        continue;
                    }
                    crate::app::ControlFlow::Exit => return Err("app exited".to_owned()),
                },
            }
        }
    }

    /// Calls the `handler` once and [`block_on`] it with a 1 second timeout using the default headless app.
    ///
    /// [`block_on`]: Self::block_on.
    #[track_caller]
    #[cfg(any(test, doc, feature = "test_util"))]
    #[cfg_attr(doc_nightly, doc(cfg(feature = "test_util")))]
    pub fn doc_test<A, H>(args: A, mut handler: H)
    where
        A: Clone + 'static,
        H: AppHandler<A>,
    {
        let mut app = crate::app::App::default().run_headless(false);
        app.block_on(&mut handler, &args, DOC_TEST_BLOCK_ON_TIMEOUT).unwrap();
    }

    /// Calls the `handlers` once each and [`block_on_multi`] with a 1 second timeout.
    ///
    /// [`block_on_multi`]: Self::block_on_multi.
    #[track_caller]
    #[cfg(any(test, doc, feature = "test_util"))]
    #[cfg_attr(doc_nightly, doc(cfg(feature = "test_util")))]
    pub fn doc_test_multi<A>(args: A, mut handlers: Vec<Box<dyn AppHandler<A>>>)
    where
        A: Clone + 'static,
    {
        let mut app = crate::app::App::default().run_headless(false);
        app.block_on_multi(handlers.iter_mut().map(|h| h.as_mut()).collect(), &args, DOC_TEST_BLOCK_ON_TIMEOUT)
            .unwrap()
    }
}

#[cfg(any(test, doc, feature = "test_util"))]
const DOC_TEST_BLOCK_ON_TIMEOUT: Duration = Duration::from_secs(5);

#[cfg(test)]
#[allow(dead_code)]
#[allow(clippy::ptr_arg)]
mod async_clone_move_fn_tests {
    // if it build it passes

    use std::{future::ready, rc::Rc};

    fn no_clones_no_input() {
        let _ = async_clone_move_fn!(|| ready(true).await);
    }

    fn one_clone_no_input(a: &String) {
        let _ = async_clone_move_fn!(a, || {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }

    fn one_clone_with_derefs_no_input(a: &Rc<String>) {
        let _ = async_clone_move_fn!(**a, || {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }

    fn two_derefs_no_input(a: &String, b: Rc<String>) {
        let _ = async_clone_move_fn!(a, b, || {
            let _: String = a;
            let _: Rc<String> = b;
            ready(true).await
        });
        let _ = (a, b);
    }

    fn one_input(a: &String) {
        let _ = async_clone_move_fn!(a, |_ctx: u32| {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }

    fn two_inputs(a: &String) {
        let _ = async_clone_move_fn!(a, |_b: u32, _c: Box<dyn std::fmt::Debug>| {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }
}

#[cfg(test)]
#[allow(dead_code)]
#[allow(clippy::ptr_arg)]
mod async_clone_move_fn_once_tests {
    // if it build it passes

    use std::{future::ready, rc::Rc};

    fn no_clones_no_input() {
        let _ = async_clone_move_fn_once!(|| ready(true).await);
    }

    fn one_clone_no_input(a: &String) {
        let _ = async_clone_move_fn_once!(a, || {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }

    fn one_clone_with_derefs_no_input(a: &Rc<String>) {
        let _ = async_clone_move_fn_once!(**a, || {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }

    fn two_derefs_no_input(a: &String, b: Rc<String>) {
        let _ = async_clone_move_fn_once!(a, b, || {
            let _: String = a;
            let _: Rc<String> = b;
            ready(true).await
        });
        let _ = (a, b);
    }

    fn one_input(a: &String) {
        let _ = async_clone_move_fn_once!(a, |_ctx: u32| {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }

    fn two_inputs(a: &String) {
        let _ = async_clone_move_fn_once!(a, |_b: u32, _c: Box<dyn std::fmt::Debug>| {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }
}

#[cfg(test)]
#[allow(unused)]
mod tests {
    use super::*;
    use crate::gesture::ClickArgs;

    fn test_infer<H: WidgetHandler<ClickArgs>>(handler: H) {
        let _ = handler;
    }

    fn hn_inference() {
        // if it builds it passes

        test_infer(hn!(|cx, _| {
            let _ = cx.services;
        }));

        test_infer(hn!(|cx, a: &ClickArgs| {
            let _ = cx.services;
            println!("{}", a.click_count);
        }));
    }

    fn hn_once_inference() {
        // if it builds it passes

        test_infer(hn_once!(|cx, _| {
            let _ = cx.services;
        }));

        test_infer(hn_once!(|cx, a: &ClickArgs| {
            let _ = cx.services;
            println!("{}", a.click_count);
        }))
    }

    #[test]
    fn async_hn_inference() {
        // if it builds it passes

        test_infer(async_hn!(|cx, _| {
            cx.with(|cx| {
                let _ = cx.services;
            });
        }));

        test_infer(async_hn!(|cx, a: ClickArgs| {
            cx.with(|cx| {
                let _ = cx.services;
            });
            println!("{}", a.click_count);
        }));
    }

    fn async_hn_once_inference() {
        // if it builds it passes

        test_infer(async_hn_once!(|cx, _| {
            cx.with(|cx| {
                let _ = cx.services;
            });
        }));

        test_infer(async_hn_once!(|cx, a: ClickArgs| {
            cx.with(|cx| {
                let _ = cx.services;
            });
            println!("{}", a.click_count);
        }));
    }
}
