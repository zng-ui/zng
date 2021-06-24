//! Handler types and macros.
//!
//! A handler is a closure that takes a *context* and *arguments*, the context can be [`WidgetContext`] or [`AppContext`]
//! with handler types implementing [`WidgetHandler`] or [`AppHandler`] respectively. These traits allow a single caller
//! to support multiple different flavors of handlers, both synchronous and asynchronous, and both `FnMut` and `FnOnce` all
//! by implementing a single entry point.
//!
//! Macros are provided for declaring the various flavors of handlers, [`hn!`], [`hn_once!`], [`async_hn!`], [`async_hn_once!`]
//! for widget contexts and [`app_hn!`], [`app_hn_once!`], [`async_app_hn!`], [`async_app_hn_once!`] for the app context. These
//! macros also build on top of the primitive macros [`clone_move!`], [`async_clone_move!`] and [`async_clone_move_once!`] to
//! provide a very easy way to *clone-move* captured variables into the handler.

use std::future::Future;
use std::marker::PhantomData;
use std::mem;

use retain_mut::RetainMut;

use crate::context::{AppContext, AppContextMut, UpdateArgs, WidgetContext, WidgetContextMut};
use crate::crate_util::WeakHandle;
use crate::task::{AppTask, WidgetTask};

/// Represents a handler in a widget context.
///
/// There are different flavors of handlers, you can use macros to declare then.
/// See [`hn!`], [`hn_once!`] or [`async_hn!`], [`async_hn_once!`] to start.
pub trait WidgetHandler<A: Clone + 'static>: 'static {
    /// Called every time the event happens in the widget context.
    fn event(&mut self, ctx: &mut WidgetContext, args: &A);
    /// Called every widget update.
    fn update(&mut self, ctx: &mut WidgetContext) {
        let _ = ctx;
    }
}
#[doc(hidden)]
pub struct FnMutWidgetHandler<A, H>
where
    A: Clone + 'static,
    H: FnMut(&mut WidgetContext, &A) + 'static,
{
    _p: PhantomData<A>,
    handler: H,
}
impl<A, H> WidgetHandler<A> for FnMutWidgetHandler<A, H>
where
    A: Clone + 'static,
    H: FnMut(&mut WidgetContext, &A) + 'static,
{
    fn event(&mut self, ctx: &mut WidgetContext, args: &A) {
        (self.handler)(ctx, args)
    }
}
#[doc(hidden)]
pub fn hn<A, H>(handler: H) -> FnMutWidgetHandler<A, H>
where
    A: Clone + 'static,
    H: FnMut(&mut WidgetContext, &A) + 'static,
{
    FnMutWidgetHandler { _p: PhantomData, handler }
}

/// Declare a mutable *clone-move* event handler.
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
pub struct FnOnceWidgetHandler<A, H>
where
    A: Clone + 'static,
    H: FnOnce(&mut WidgetContext, &A) + 'static,
{
    _p: PhantomData<A>,
    handler: Option<H>,
}
impl<A, H> WidgetHandler<A> for FnOnceWidgetHandler<A, H>
where
    A: Clone + 'static,
    H: FnOnce(&mut WidgetContext, &A) + 'static,
{
    fn event(&mut self, ctx: &mut WidgetContext, args: &A) {
        if let Some(handler) = self.handler.take() {
            handler(ctx, args);
        }
    }
}
#[doc(hidden)]
pub fn hn_once<A, H>(handler: H) -> FnOnceWidgetHandler<A, H>
where
    A: Clone + 'static,
    H: FnOnce(&mut WidgetContext, &A) + 'static,
{
    FnOnceWidgetHandler {
        _p: PhantomData,
        handler: Some(handler),
    }
}

/// Declare a *clone-move* event handler that is only called once.
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
pub struct AsyncFnMutWidgetHandler<A, F, H>
where
    A: Clone + 'static,
    F: Future<Output = ()> + 'static,
    H: FnMut(WidgetContextMut, A) -> F + 'static,
{
    _a: PhantomData<A>,
    handler: H,
    tasks: Vec<WidgetTask<()>>,
}
impl<A, F, H> WidgetHandler<A> for AsyncFnMutWidgetHandler<A, F, H>
where
    A: Clone + 'static,
    F: Future<Output = ()> + 'static,
    H: FnMut(WidgetContextMut, A) -> F + 'static,
{
    fn event(&mut self, ctx: &mut WidgetContext, args: &A) {
        let handler = &mut self.handler;
        let mut task = WidgetTask::new(ctx, |ctx| handler(ctx, args.clone()));
        if task.update(ctx).is_none() {
            self.tasks.push(task);
        }
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.tasks.retain_mut(|t| t.update(ctx).is_none());
    }
}
#[doc(hidden)]
pub fn async_hn<A, F, H>(handler: H) -> AsyncFnMutWidgetHandler<A, F, H>
where
    A: Clone + 'static,
    F: Future<Output = ()> + 'static,
    H: FnMut(WidgetContextMut, A) -> F + 'static,
{
    AsyncFnMutWidgetHandler {
        _a: PhantomData,
        handler,
        tasks: vec![],
    }
}

/// Declare an async *clone-move* event handler.
///
/// The macro input is a closure with optional *clone-move* variables, internally it uses [`async_clone_move!`] so
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
/// Internally the [`async_clone_move!`] macro is used so you can *clone-move* variables into the handler.
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
/// listed in the *clone-move* section it would not be available after the handler is created. See [`async_clone_move!`] for details.
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
        $crate::handler::async_hn($crate::async_clone_move! { $($tt)+ })
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
pub struct AsyncFnOnceWidgetHandler<A, F, H>
where
    A: Clone + 'static,
    F: Future<Output = ()> + 'static,
    H: FnOnce(WidgetContextMut, A) -> F + 'static,
{
    _a: PhantomData<A>,
    state: AsyncFnOnceWhState<H>,
}

impl<A, F, H> WidgetHandler<A> for AsyncFnOnceWidgetHandler<A, F, H>
where
    A: Clone + 'static,
    F: Future<Output = ()> + 'static,
    H: FnOnce(WidgetContextMut, A) -> F + 'static,
{
    fn event(&mut self, ctx: &mut WidgetContext, args: &A) {
        if let AsyncFnOnceWhState::NotCalled(handler) = mem::replace(&mut self.state, AsyncFnOnceWhState::Done) {
            let mut task = WidgetTask::new(ctx, |ctx| handler(ctx, args.clone()));
            if task.update(ctx).is_none() {
                self.state = AsyncFnOnceWhState::Pending(task);
            }
        }
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if let AsyncFnOnceWhState::Pending(t) = &mut self.state {
            if t.update(ctx).is_some() {
                self.state = AsyncFnOnceWhState::Done;
            }
        }
    }
}
#[doc(hidden)]
pub fn async_hn_once<A, F, H>(handler: H) -> AsyncFnOnceWidgetHandler<A, F, H>
where
    A: Clone + 'static,
    F: Future<Output = ()> + 'static,
    H: FnOnce(WidgetContextMut, A) -> F + 'static,
{
    AsyncFnOnceWidgetHandler {
        _a: PhantomData,
        state: AsyncFnOnceWhState::NotCalled(handler),
    }
}

/// Declare an async *clone-move* event handler that is only called once.
///
/// The macro input is a closure with optional *clone-move* variables, internally it uses [`async_clone_move_once!`] so
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
        $crate::handler::async_hn_once($crate::async_clone_move_once! { $($tt)+ })
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
    /// The `args.handle` can be used to unsubscribe the handler. Async handlers are expected to schedule
    /// their tasks to run somewhere in the app, usually in the [`Updates::on_pre_update`]. The `handle` is
    /// **not** expected to cancel running async tasks, only to drop `self` before the next event happens.
    fn event(&mut self, ctx: &mut AppContext, args: &A, handler_args: &AppHandlerArgs);
}

#[doc(hidden)]
pub struct FnMutAppHandler<A, H>
where
    A: Clone + 'static,
    H: FnMut(&mut AppContext, &A, &dyn AppWeakHandle) + 'static,
{
    _p: PhantomData<A>,
    handler: H,
}
impl<A, H> AppHandler<A> for FnMutAppHandler<A, H>
where
    A: Clone + 'static,
    H: FnMut(&mut AppContext, &A, &dyn AppWeakHandle) + 'static,
{
    fn event(&mut self, ctx: &mut AppContext, args: &A, handler_args: &AppHandlerArgs) {
        (self.handler)(ctx, args, handler_args.handle);
    }
}
#[doc(hidden)]
pub fn app_hn<A, H>(handler: H) -> FnMutAppHandler<A, H>
where
    A: Clone + 'static,
    H: FnMut(&mut AppContext, &A, &dyn AppWeakHandle) + 'static,
{
    FnMutAppHandler { _p: PhantomData, handler }
}

/// Declare a mutable *clone-move* app event handler.
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
pub struct FnOnceAppHandler<A, H>
where
    A: Clone + 'static,
    H: FnOnce(&mut AppContext, &A) + 'static,
{
    _p: PhantomData<A>,
    handler: Option<H>,
}
impl<A, H> AppHandler<A> for FnOnceAppHandler<A, H>
where
    A: Clone + 'static,
    H: FnOnce(&mut AppContext, &A) + 'static,
{
    fn event(&mut self, ctx: &mut AppContext, args: &A, handler_args: &AppHandlerArgs) {
        if let Some(handler) = self.handler.take() {
            handler(ctx, args);
            handler_args.handle.unsubscribe();
        } else {
            log::error!("`app_hn_once!` called after requesting unsubscribe");
        }
    }
}
#[doc(hidden)]
pub fn app_hn_once<A, H>(handler: H) -> FnOnceAppHandler<A, H>
where
    A: Clone + 'static,
    H: FnOnce(&mut AppContext, &A) + 'static,
{
    FnOnceAppHandler {
        _p: PhantomData,
        handler: Some(handler),
    }
}

/// Declare a *clone-move* app event handler that is only called once.
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
pub struct AsyncFnMutAppHandler<A, F, H>
where
    A: Clone + 'static,
    F: Future<Output = ()> + 'static,
    H: FnMut(AppContextMut, A, Box<dyn AppWeakHandle>) -> F + 'static,
{
    _a: PhantomData<A>,
    handler: H,
}
impl<A, F, H> AppHandler<A> for AsyncFnMutAppHandler<A, F, H>
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
                    .on_pre_update(move |ctx, u_args: &UpdateArgs| {
                        if task.update(ctx).is_some() {
                            u_args.unsubscribe();
                        }
                    })
                    .permanent();
            } else {
                ctx.updates
                    .on_update(move |ctx, u_args: &UpdateArgs| {
                        if task.update(ctx).is_some() {
                            u_args.unsubscribe();
                        }
                    })
                    .permanent();
            }
        }
    }
}
#[doc(hidden)]
pub fn async_app_hn<A, F, H>(handler: H) -> AsyncFnMutAppHandler<A, F, H>
where
    A: Clone + 'static,
    F: Future<Output = ()> + 'static,
    H: FnMut(AppContextMut, A, Box<dyn AppWeakHandle>) -> F + 'static,
{
    AsyncFnMutAppHandler { _a: PhantomData, handler }
}

/// Declare an async *clone-move* app event handler.
///
/// The macro input is a closure with optional *clone-move* variables, internally it uses [`async_clone_move!`] so
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
/// Internally the [`async_clone_move!`] macro is used so you can *clone-move* variables into the handler.
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
/// listed in the *clone-move* section it would not be available after the handler is created. See [`async_clone_move!`] for details.
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
        $crate::handler::async_app_hn($crate::async_clone_move! { $($tt)+ })
    }
}
#[doc(inline)]
pub use crate::async_app_hn;

#[doc(hidden)]
pub struct AsyncFnOnceAppHandler<A, F, H>
where
    A: Clone + 'static,
    F: Future<Output = ()> + 'static,
    H: FnOnce(AppContextMut, A) -> F + 'static,
{
    _a: PhantomData<A>,
    handler: Option<H>,
}

impl<A, F, H> AppHandler<A> for AsyncFnOnceAppHandler<A, F, H>
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
                    .on_pre_update(move |ctx, u_args| {
                        if task.update(ctx).is_some() {
                            u_args.unsubscribe();
                        }
                    })
                    .permanent();
                } else {
                    ctx.updates
                    .on_update(move |ctx, u_args| {
                        if task.update(ctx).is_some() {
                            u_args.unsubscribe();
                        }
                    })
                    .permanent();
                }
            }
        } else {
            log::error!("`async_app_hn_once!` called after requesting unsubscribe");
        }
    }
}
#[doc(hidden)]
pub fn async_app_hn_once<A, F, H>(handler: H) -> AsyncFnOnceAppHandler<A, F, H>
where
    A: Clone + 'static,
    F: Future<Output = ()> + 'static,
    H: FnOnce(AppContextMut, A) -> F + 'static,
{
    AsyncFnOnceAppHandler {
        _a: PhantomData,
        handler: Some(handler),
    }
}

/// Declare an async *clone-move* app event handler that is only called once.
///
/// The macro input is a closure with optional *clone-move* variables, internally it uses [`async_clone_move_once!`] so
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
        $crate::handler::async_app_hn_once($crate::async_clone_move_once! { $($tt)+ })
    }
}
#[doc(inline)]
pub use crate::async_app_hn_once;

/// Cloning closure.
///
/// A common pattern when creating widgets is a [variable](crate::var::var) that is shared between a property and an event handler.
/// The event handler is a closure but you cannot just move the variable, it needs to take a clone of the variable.
///
/// This macro facilitates this pattern.
///
/// # Example
///
/// ```
/// # fn main() { }
/// # use zero_ui_core::{widget, clone_move, NilUiNode, var::{var, IntoVar}, text::{Text, ToText}, context::WidgetContext};
/// #
/// # #[widget($crate::window)]
/// # pub mod window {
/// #     use super::*;
/// #
/// #     properties! {
/// #         #[allowed_in_when = false]
/// #         title(impl IntoVar<Text>);
/// #
/// #         #[allowed_in_when = false]
/// #         on_click(impl FnMut(&mut WidgetContext, ()));
/// #     }
/// #
/// #     fn new_child(title: impl IntoVar<Text>, on_click: impl FnMut(&mut WidgetContext, ())) -> NilUiNode {
/// #         NilUiNode
/// #     }
/// # }
/// #
/// # fn demo() {
/// let title = var("Click Me!".to_text());
/// window! {
///     on_click = clone_move!(title, |ctx, _| {
///         title.set(ctx.vars, "Clicked!");
///     });
///     title;
/// }
/// # ;
/// # }
/// ```
///
/// Expands to:
///
/// ```
/// # fn main() { }
/// # use zero_ui_core::{widget, clone_move, NilUiNode, var::{var, IntoVar}, text::{Text, ToText}, context::WidgetContext};
/// #
/// # #[widget($crate::window)]
/// # pub mod window {
/// #     use super::*;
/// #
/// #     properties! {
/// #         #[allowed_in_when = false]
/// #         title(impl IntoVar<Text>);
/// #
/// #         #[allowed_in_when = false]
/// #         on_click(impl FnMut(&mut WidgetContext, ()));
/// #     }
/// #
/// #     fn new_child(title: impl IntoVar<Text>, on_click: impl FnMut(&mut WidgetContext, ())) -> NilUiNode {
/// #         NilUiNode
/// #     }
/// # }
/// #
/// # fn demo() {
/// let title = var("Click Me!".to_text());
/// window! {
///     on_click = {
///         let title = title.clone();
///         move |ctx, _| {
///             title.set(ctx.vars, "Clicked!");
///         }
///     };
///     title;
/// }
/// # ;
/// # }
/// ```
///
/// # Other Patterns
///
/// Although this macro exists primarily for creating event handlers, you can use it with any Rust variable. The
/// cloned variable can be marked `mut` and you can deref `*` as many times as you need to get to the actual value you
/// want to clone.
///
/// ```
/// # use zero_ui_core::clone_move;
/// # use std::rc::Rc;
/// let foo = vec![1, 2, 3];
/// let bar = Rc::new(vec!['a', 'b', 'c']);
/// let closure = clone_move!(mut foo, *bar, || {
///     foo.push(4);
///     let cloned_vec: Vec<_> = bar;
/// });
/// assert_eq!(foo.len(), 3);
/// ```
///
/// Expands to:
///
/// ```
/// # use zero_ui_core::clone_move;
/// # use std::rc::Rc;
/// let foo = vec![1, 2, 3];
/// let bar = Rc::new(vec!['a', 'b', 'c']);
/// let closure = {
///     let mut foo = foo.clone();
///     let bar = (*bar).clone();
///     move || {
///         foo.push(4);
///         let cloned_vec: Vec<_> = bar;
///     }
/// };
/// assert_eq!(foo.len(), 3);
/// ```
///
/// # Async
///
/// See [`async_clone_move!`](macro@crate::async_clone_move) for creating `async` closures.
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
            [$($mut:tt)?]
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

/// Cloning async closure.
///
/// This macro syntax is exactly the same as [`clone_move!`](macro@crate::clone_move), but it expands to an *async closure* that
/// captures a clone of zero or more variables and moves another clone of these variables into the returned future for each call.
///
/// # Example
///
/// ```
/// # fn main() { }
/// # use zero_ui_core::{widget, property, async_clone_move, UiNode, NilUiNode, var::{var, IntoVar}, text::{Text, ToText}, context::WidgetContextMut};
/// # use std::future::Future;
/// #
/// # #[property(event)]
/// # fn on_click_async<C: UiNode, F: Future<Output=()>, H: FnMut(WidgetContextMut, ()) -> F>(child: C, handler: H) -> impl UiNode { child }
/// #
/// # #[widget($crate::window)]
/// # pub mod window {
/// #     use super::*;
/// #
/// #     properties! {
/// #         #[allowed_in_when = false]
/// #         title(impl IntoVar<Text>);
/// #     }
/// #
/// #     fn new_child(title: impl IntoVar<Text>) -> NilUiNode {
/// #         NilUiNode
/// #     }
/// # }
/// # async fn delay() {
/// #   std::future::ready(true).await;
/// # }
/// #
/// # fn demo() {
/// let title = var("Click Me!".to_text());
/// window! {
///     on_click_async = async_clone_move!(title, |ctx, _| {
///         title.set(&ctx, "Clicked!");
///         delay().await;
///         title.set(&ctx, "Async Update!");
///     });
///     title;
/// }
/// # ;
/// # }
/// ```
///
/// Expands to:
///
/// ```
/// # fn main() { }
/// # use zero_ui_core::{widget, property, async_clone_move, UiNode, NilUiNode, var::{var, IntoVar}, text::{Text, ToText}, context::WidgetContextMut};
/// # use std::future::Future;
/// #
/// # #[property(event)]
/// # fn on_click_async<C: UiNode, F: Future<Output=()>, H: FnMut(WidgetContextMut, ()) -> F>(child: C, handler: H) -> impl UiNode { child }
/// #
/// # #[widget($crate::window)]
/// # pub mod window {
/// #     use super::*;
/// #
/// #     properties! {
/// #         #[allowed_in_when = false]
/// #         title(impl IntoVar<Text>);
/// #     }
/// #
/// #     fn new_child(title: impl IntoVar<Text>) -> NilUiNode {
/// #         NilUiNode
/// #     }
/// # }
/// # async fn delay() {
/// #   std::future::ready(true).await;
/// # }
/// #
/// # fn demo() {
/// let title = var("Click Me!".to_text());
/// window! {
///     on_click_async = {
///         let title = title.clone();
///         move |ctx, _| {
///             let title = title.clone();
///             async move {
///                 title.set(&ctx, "Clicked!");
///                 delay().await;
///                 title.set(&ctx, "Async Update!");
///             }
///         }
///     };
///     title;
/// }
/// # ;
/// # }
/// ```
#[macro_export]
macro_rules! async_clone_move {
    ($($tt:tt)+) => { $crate::__async_clone_move! { [{}{}][][] $($tt)+ } }
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
            [$($mut:tt)?]
            [$($deref)* *]
            $($rest)+
        }
    };

    // match end of a variable
    ([$($done:tt)*][$($mut:tt)?][$($deref:tt)*] $var:ident, $($rest:tt)+) => {
        $crate::__async_clone_move! {
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
        $crate::__async_clone_move! {
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
        $crate::__async_clone_move! {
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
        $crate::__async_clone_move! {
            @args
            [$($done)*]
            [$($args)* $arg_tt]
            $($rest)+
        }
    };
}

/// Cloning async closure that can only be called once.
///
/// This macro syntax is exactly the same as [`async_clone_move!`](macro@crate::async_clone_move), but it does not clone variables
/// again inside the call to move to the returned future. Because if moves the captured variables to the closure returned `Future`
/// it can only be `FnOnce`.
///
/// # Example
///
/// In the example `data` is clone moved to the closure and then moved in the returned `Future`, this only works because the closure
/// is a `FnOnce`.
///
/// ```
/// # use zero_ui_core::{async_clone_move_once, task};
/// # use std::future::Future;
/// fn foo<F: Future<Output=Vec<u32>>>(f: impl FnOnce(String) -> F) { }
///
/// let data = vec![1, 2, 3];
/// foo(async_clone_move_once!(data, |s| {
///     task::wait(move || println!("do async thing: {}", s)).await;
///     data
/// }))
/// ```
#[macro_export]
macro_rules! async_clone_move_once {
    ($($tt:tt)+) => { $crate::__async_clone_move_once! { [][][] $($tt)+ } }
}
#[doc(inline)]
pub use crate::async_clone_move_once;
#[doc(hidden)]
#[macro_export]
macro_rules! __async_clone_move_once {
    // match start of mut var
    ([$($done:tt)*][][] mut $($rest:tt)+) => {
        $crate::__async_clone_move_once! {
            [$($done)*]
            [mut]
            []
            $($rest)+
        }
    };

    // match one var deref (*)
    ([$($done:tt)*][$($mut:tt)?][$($deref:tt)*] * $($rest:tt)+) => {
        $crate::__async_clone_move_once! {
            [$($done)*]
            [$($mut:tt)?]
            [$($deref)* *]
            $($rest)+
        }
    };

    // match end of a variable
    ([$($done:tt)*][$($mut:tt)?][$($deref:tt)*] $var:ident, $($rest:tt)+) => {
        $crate::__async_clone_move_once! {
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
        $crate::__async_clone_move_once! {
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
        $crate::__async_clone_move_once! {
            @args
            [$($done)*]
            [$($args)* $arg_tt]
            $($rest)+
        }
    };
}

#[cfg(test)]
#[allow(dead_code)]
#[allow(clippy::ptr_arg)]
mod async_clone_move_tests {
    // if it build it passes

    use std::{future::ready, rc::Rc};

    fn no_clones_no_input() {
        let _ = async_clone_move!(|| ready(true).await);
    }

    fn one_clone_no_input(a: &String) {
        let _ = async_clone_move!(a, || {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }

    fn one_clone_with_derefs_no_input(a: &Rc<String>) {
        let _ = async_clone_move!(**a, || {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }

    fn two_derefs_no_input(a: &String, b: Rc<String>) {
        let _ = async_clone_move!(a, b, || {
            let _: String = a;
            let _: Rc<String> = b;
            ready(true).await
        });
        let _ = (a, b);
    }

    fn one_input(a: &String) {
        let _ = async_clone_move!(a, |_ctx: u32| {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }

    fn two_inputs(a: &String) {
        let _ = async_clone_move!(a, |_b: u32, _c: Box<dyn std::fmt::Debug>| {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }
}

#[cfg(test)]
#[allow(dead_code)]
#[allow(clippy::ptr_arg)]
mod async_clone_move_once_tests {
    // if it build it passes

    use std::{future::ready, rc::Rc};

    fn no_clones_no_input() {
        let _ = async_clone_move_once!(|| ready(true).await);
    }

    fn one_clone_no_input(a: &String) {
        let _ = async_clone_move_once!(a, || {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }

    fn one_clone_with_derefs_no_input(a: &Rc<String>) {
        let _ = async_clone_move_once!(**a, || {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }

    fn two_derefs_no_input(a: &String, b: Rc<String>) {
        let _ = async_clone_move_once!(a, b, || {
            let _: String = a;
            let _: Rc<String> = b;
            ready(true).await
        });
        let _ = (a, b);
    }

    fn one_input(a: &String) {
        let _ = async_clone_move_once!(a, |_ctx: u32| {
            let _: String = a;
            ready(true).await
        });
        let _ = a;
    }

    fn two_inputs(a: &String) {
        let _ = async_clone_move_once!(a, |_b: u32, _c: Box<dyn std::fmt::Debug>| {
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
