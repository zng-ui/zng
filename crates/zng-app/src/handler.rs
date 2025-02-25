//! Handler types and macros.

use std::any::Any;
use std::future::Future;
use std::marker::PhantomData;
use std::time::Duration;
use std::{mem, thread};

#[doc(hidden)]
pub use zng_clone_move::*;

use zng_handle::{Handle, WeakHandle};
use zng_task::{self as task, UiTask};

use crate::INSTANT;

/// Represents a handler in a widget context.
///
/// There are different flavors of handlers, you can use macros to declare then.
/// See [`hn!`], [`hn_once!`] or [`async_hn!`], [`async_hn_once!`] to start.
#[diagnostic::on_unimplemented(
    note = "use `hn!(|args: &{A}| {{ }})` to declare a widget handler from a `FnMut` closure",
    note = "use `hn_once!`, `async_hn!` or `async_hn_once!` for other closure types"
)]
pub trait WidgetHandler<A: Clone + 'static>: Any + Send {
    /// Called every time the handler's event happens in the widget context.
    ///
    /// Returns `true` when the event handler is async and it has not finished handling the event.
    ///
    /// [`update`]: WidgetHandler::update
    /// [`info`]: crate::widget::node::UiNode::info
    fn event(&mut self, args: &A) -> bool;

    /// Called every widget update.
    ///
    /// Returns `false` when all pending async tasks are completed. Note that event properties
    /// will call this method every update even if it is returning `false`.
    ///
    /// [`update`]: WidgetHandler::update
    fn update(&mut self) -> bool {
        false
    }

    /// Box the handler.
    ///
    /// The type `Box<dyn WidgetHandler<A>>` implements `WidgetHandler<A>` and just returns itself
    /// in this method, avoiding double boxing.
    fn boxed(self) -> Box<dyn WidgetHandler<A>>
    where
        Self: Sized,
    {
        Box::new(self)
    }
    /// Boxes the handler if the `feature = "dyn_closure"` is active, otherwise retains the same handler type.    
    #[cfg(feature = "dyn_closure")]
    fn cfg_boxed(self) -> Box<dyn WidgetHandler<A>>
    where
        Self: Sized,
    {
        self.boxed()
    }
    /// Boxes the handler if the `feature = "dyn_closure"` is active, otherwise retains the same handler type.
    #[cfg(not(feature = "dyn_closure"))]
    fn cfg_boxed(self) -> Self
    where
        Self: Sized,
    {
        self
    }
}
impl<A: Clone + 'static> WidgetHandler<A> for Box<dyn WidgetHandler<A>> {
    fn event(&mut self, args: &A) -> bool {
        self.as_mut().event(args)
    }

    fn update(&mut self) -> bool {
        self.as_mut().update()
    }

    fn boxed(self) -> Box<dyn WidgetHandler<A>>
    where
        Self: Sized,
    {
        self
    }
}

#[doc(hidden)]
pub struct FnMutWidgetHandler<H> {
    handler: H,
}
impl<A, H> WidgetHandler<A> for FnMutWidgetHandler<H>
where
    A: Clone + 'static,
    H: FnMut(&A) + Send + 'static,
{
    fn event(&mut self, args: &A) -> bool {
        (self.handler)(args);
        false
    }
}

#[doc(hidden)]
#[cfg(not(feature = "dyn_closure"))]
pub fn hn<A, H>(handler: H) -> FnMutWidgetHandler<H>
where
    A: Clone + 'static,
    H: FnMut(&A) + Send + 'static,
{
    FnMutWidgetHandler { handler }
}
#[doc(hidden)]
#[cfg(feature = "dyn_closure")]
pub fn hn<A, H>(handler: H) -> FnMutWidgetHandler<Box<dyn FnMut(&A) + Send>>
where
    A: Clone + 'static,
    H: FnMut(&A) + Send + 'static,
{
    FnMutWidgetHandler {
        handler: Box::new(handler),
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
/// # zng_app::event::event_args! { pub struct ClickArgs { pub target: zng_txt::Txt, pub click_count: usize, .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) { } } }
/// # use zng_app::handler::hn;
/// # let _scope = zng_app::APP.minimal();
/// # fn assert_type() -> impl zng_app::handler::WidgetHandler<ClickArgs> {
/// # let
/// on_click = hn!(|_| {
///     println!("Clicked!");
/// });
/// # on_click }
/// ```
///
/// The closure input is `&ClickArgs` for this property. Note that
/// if you want to use the event args you must annotate the input type, the context type is inferred.
///
/// ```
/// # #[derive(Clone)] pub struct ClickArgs { pub target: zng_txt::Txt, pub click_count: usize }
/// # use zng_app::handler::hn;
/// # let _scope = zng_app::APP.minimal();
/// # fn assert_type() -> impl zng_app::handler::WidgetHandler<ClickArgs> {
/// # let
/// on_click = hn!(|args: &ClickArgs| {
///     println!("Clicked {}!", args.click_count);
/// });
/// # on_click }
/// ```
///
/// Internally the [`clmv!`] macro is used so you can *clone-move* variables into the handler.
///
/// ```
/// # #[derive(Clone)] pub struct ClickArgs { pub target: zng_txt::Txt, pub click_count: usize }
/// # use zng_txt::formatx;
/// # use zng_var::{var, Var};
/// # use zng_app::handler::hn;
/// # let _scope = zng_app::APP.minimal();
/// # fn assert_type() -> impl zng_app::handler::WidgetHandler<ClickArgs> {
/// let foo = var(0);
///
/// // ..
///
/// # let
/// on_click = hn!(foo, |args: &ClickArgs| {
///     foo.set(args.click_count);
/// });
///
/// // can still use after:
/// let bar = foo.map(|c| formatx!("click_count: {c}"));
///
/// # on_click }
/// ```
///
/// In the example above only a clone of `foo` is moved into the handler. Note that handlers always capture by move, if `foo` was not
/// listed in the *clone-move* section it would not be available after the handler is created. See [`clmv!`] for details.
///
/// [`clmv!`]: zng_clone_move::clmv
#[macro_export]
macro_rules! hn {
    ($($tt:tt)+) => {
        $crate::handler::hn($crate::handler::clmv!{ $($tt)+ })
    }
}
#[doc(inline)]
pub use crate::hn;
use crate::{AppControlFlow, HeadlessApp};

#[doc(hidden)]
pub struct FnOnceWidgetHandler<H> {
    handler: Option<H>,
}
impl<A, H> WidgetHandler<A> for FnOnceWidgetHandler<H>
where
    A: Clone + 'static,
    H: FnOnce(&A) + Send + 'static,
{
    fn event(&mut self, args: &A) -> bool {
        if let Some(handler) = self.handler.take() {
            handler(args);
        }
        false
    }
}
#[doc(hidden)]
#[cfg(not(feature = "dyn_closure"))]
pub fn hn_once<A, H>(handler: H) -> FnOnceWidgetHandler<H>
where
    A: Clone + 'static,
    H: FnOnce(&A) + Send + 'static,
{
    FnOnceWidgetHandler { handler: Some(handler) }
}
#[doc(hidden)]
#[cfg(feature = "dyn_closure")]
pub fn hn_once<A, H>(handler: H) -> FnOnceWidgetHandler<Box<dyn FnOnce(&A) + Send>>
where
    A: Clone + 'static,
    H: FnOnce(&A) + Send + 'static,
{
    FnOnceWidgetHandler {
        handler: Some(Box::new(handler)),
    }
}

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
/// # use zng_app::handler::hn_once;
/// # let _scope = zng_app::APP.minimal();
/// # fn assert_type() -> impl zng_app::handler::WidgetHandler<()> {
/// let data = vec![1, 2, 3];
/// # let
/// on_click = hn_once!(|_| {
///     for i in data {
///         print!("{i}, ");
///     }
/// });
/// # on_click }
/// ```
///
/// Other then declaring a `FnOnce` this macro behaves like [`hn!`], so the same considerations apply. You can *clone-move* variables,
/// the type of the input is the event arguments and must be annotated.
///
/// ```
/// # use zng_app::handler::hn_once;
/// # let _scope = zng_app::APP.minimal();
/// # #[derive(Clone)]
/// # pub struct ClickArgs { click_count: usize }
/// # fn assert_type() -> impl zng_app::handler::WidgetHandler<ClickArgs> {
/// let data = vec![1, 2, 3];
/// # let
/// on_click = hn_once!(data, |args: &ClickArgs| {
///     drop(data);
/// });
///
/// println!("{data:?}");
/// # on_click }
/// ```
///
/// [`clmv!`]: zng_clone_move::clmv
#[macro_export]
macro_rules! hn_once {
    ($($tt:tt)+) => {
        $crate::handler::hn_once($crate::handler::clmv! { $($tt)+ })
    }
}
#[doc(inline)]
pub use crate::hn_once;

#[doc(hidden)]
pub struct AsyncFnMutWidgetHandler<H> {
    handler: H,
    tasks: Vec<UiTask<()>>,
}
impl<A, F, H> WidgetHandler<A> for AsyncFnMutWidgetHandler<H>
where
    A: Clone + 'static,
    F: Future<Output = ()> + Send + 'static,
    H: FnMut(A) -> F + Send + 'static,
{
    fn event(&mut self, args: &A) -> bool {
        let handler = &mut self.handler;
        let mut task = UiTask::new(Some(WIDGET.id()), handler(args.clone()));
        let need_update = task.update().is_none();
        if need_update {
            self.tasks.push(task);
        }
        need_update
    }

    fn update(&mut self) -> bool {
        self.tasks.retain_mut(|t| t.update().is_none());
        !self.tasks.is_empty()
    }
}
#[doc(hidden)]
#[cfg(not(feature = "dyn_closure"))]
pub fn async_hn<A, F, H>(handler: H) -> AsyncFnMutWidgetHandler<H>
where
    A: Clone + 'static,
    F: Future<Output = ()> + Send + 'static,
    H: FnMut(A) -> F + Send + 'static,
{
    AsyncFnMutWidgetHandler { handler, tasks: vec![] }
}

#[cfg(feature = "dyn_closure")]
type BoxedAsyncHn<A> = Box<dyn FnMut(A) -> std::pin::Pin<Box<dyn Future<Output = ()> + Send>> + Send>;

#[doc(hidden)]
#[cfg(feature = "dyn_closure")]
pub fn async_hn<A, F, H>(mut handler: H) -> AsyncFnMutWidgetHandler<BoxedAsyncHn<A>>
where
    A: Clone + 'static,
    F: Future<Output = ()> + Send + 'static,
    H: FnMut(A) -> F + Send + 'static,
{
    AsyncFnMutWidgetHandler {
        handler: Box::new(move |args| Box::pin(handler(args))),
        tasks: vec![],
    }
}

///<span data-del-macro-root></span> Declare an async *clone-move* event handler.
///
/// The macro input is a closure with optional *clone-move* variables, internally it uses [`async_clmv_fn!`] so
/// the input is the same syntax.
///
/// # Examples
///
/// The example declares an async event handler for the `on_click` property.
///
/// ```
/// # zng_app::event::event_args! { pub struct ClickArgs { pub target: zng_txt::Txt, pub click_count: usize, .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) { } } }
/// # use zng_app::handler::async_hn;
/// # use zng_task as task;
/// # let _scope = zng_app::APP.minimal();
/// # fn assert_type() -> impl zng_app::handler::WidgetHandler<ClickArgs> {
/// # let
/// on_click = async_hn!(|_| {
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
/// The closure input is `ClickArgs` for this property. Note that
/// if you want to use the event args you must annotate the input type.
///
/// ```
/// # zng_app::event::event_args! { pub struct ClickArgs { pub target: zng_txt::Txt, pub click_count: usize, .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) { } } }
/// # use zng_app::handler::async_hn;
/// # use zng_app::widget::WIDGET;
/// # let _scope = zng_app::APP.minimal();
/// # fn assert_type() -> impl zng_app::handler::WidgetHandler<ClickArgs> {
/// # let
/// on_click = async_hn!(|args: ClickArgs| {
///     println!("Clicked {} {} times!", WIDGET.id(), args.click_count);
///     
/// });
/// # on_click }
/// ```
///
/// Internally the [`async_clmv_fn!`] macro is used so you can *clone-move* variables into the handler.
///
/// ```
/// # zng_app::event::event_args! { pub struct ClickArgs { pub target: zng_txt::Txt, pub click_count: usize, .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) { } } }
/// # use zng_app::handler::async_hn;
/// # use zng_var::{var, Var};
/// # use zng_task as task;
/// # use zng_txt::formatx;
/// # let _scope = zng_app::APP.minimal();
/// # fn assert_type() -> impl zng_app::handler::WidgetHandler<ClickArgs> {
/// let enabled = var(true);
///
/// // ..
///
/// # let
/// on_click = async_hn!(enabled, |args: ClickArgs| {
///     enabled.set(false);
///
///     task::run(async move {
///         println!("do something {}", args.click_count);
///     }).await;
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
/// You may want to always *clone-move* captures for async handlers, because they then automatically get cloned again for each event. This
/// needs to happen because you can have more then one *handler task* running at the same type, and both want access to the captured variables.
///
/// This second cloning can be avoided by using the [`async_hn_once!`] macro instead, but only if you expect a single event.
///
/// [`async_clmv_fn!`]: zng_clone_move::async_clmv_fn
#[macro_export]
macro_rules! async_hn {
    ($($tt:tt)+) => {
        $crate::handler::async_hn($crate::handler::async_clmv_fn! { $($tt)+ })
    }
}
#[doc(inline)]
pub use crate::async_hn;

enum AsyncFnOnceWhState<H> {
    NotCalled(H),
    Pending(UiTask<()>),
    Done,
}
#[doc(hidden)]
pub struct AsyncFnOnceWidgetHandler<H> {
    state: AsyncFnOnceWhState<H>,
}
impl<A, F, H> WidgetHandler<A> for AsyncFnOnceWidgetHandler<H>
where
    A: Clone + 'static,
    F: Future<Output = ()> + Send + 'static,
    H: FnOnce(A) -> F + Send + 'static,
{
    fn event(&mut self, args: &A) -> bool {
        match mem::replace(&mut self.state, AsyncFnOnceWhState::Done) {
            AsyncFnOnceWhState::NotCalled(handler) => {
                let mut task = UiTask::new(Some(WIDGET.id()), handler(args.clone()));
                let is_pending = task.update().is_none();
                if is_pending {
                    self.state = AsyncFnOnceWhState::Pending(task);
                }
                is_pending
            }
            AsyncFnOnceWhState::Pending(t) => {
                self.state = AsyncFnOnceWhState::Pending(t);
                false
            }
            AsyncFnOnceWhState::Done => false,
        }
    }

    fn update(&mut self) -> bool {
        let mut is_pending = false;
        if let AsyncFnOnceWhState::Pending(t) = &mut self.state {
            is_pending = t.update().is_none();
            if !is_pending {
                self.state = AsyncFnOnceWhState::Done;
            }
        }
        is_pending
    }
}
#[doc(hidden)]
#[cfg(not(feature = "dyn_closure"))]
pub fn async_hn_once<A, F, H>(handler: H) -> AsyncFnOnceWidgetHandler<H>
where
    A: Clone + 'static,
    F: Future<Output = ()> + Send + 'static,
    H: FnOnce(A) -> F + Send + 'static,
{
    AsyncFnOnceWidgetHandler {
        state: AsyncFnOnceWhState::NotCalled(handler),
    }
}

#[cfg(feature = "dyn_closure")]
type BoxedAsyncHnOnce<A> = Box<dyn FnOnce(A) -> std::pin::Pin<Box<dyn Future<Output = ()> + Send>> + Send>;

#[doc(hidden)]
#[cfg(feature = "dyn_closure")]
pub fn async_hn_once<A, F, H>(handler: H) -> AsyncFnOnceWidgetHandler<BoxedAsyncHnOnce<A>>
where
    A: Clone + 'static,
    F: Future<Output = ()> + Send + 'static,
    H: FnOnce(A) -> F + Send + 'static,
{
    AsyncFnOnceWidgetHandler {
        state: AsyncFnOnceWhState::NotCalled(Box::new(move |args| Box::pin(handler(args)))),
    }
}

///<span data-del-macro-root></span> Declare an async *clone-move* event handler that is only called once.
///
/// The macro input is a closure with optional *clone-move* variables, internally it uses [`async_clmv_fn_once!`] so
/// the input is the same syntax.
///
/// # Examples
///
/// The example captures `data` by move and then moves it again to another thread. This is not something you can do using [`async_hn!`]
/// because that handler expects to be called many times. We expect `on_open` to only be called once, so we can don't need to capture by
/// *clone-move* here just to use `data`.
///
/// ```
/// # zng_app::event::event_args! { pub struct ClickArgs { pub target: zng_txt::Txt, pub click_count: usize, .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) { } } }
/// # use zng_app::handler::async_hn_once;
/// # use zng_task as task;
/// # let _scope = zng_app::APP.minimal();
/// # fn assert_type() -> impl zng_app::handler::WidgetHandler<ClickArgs> {
/// let data = vec![1, 2, 3];
/// # let
/// on_open = async_hn_once!(|_| {
///     task::run(async move {
///          for i in data {
///              print!("{i}, ");
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
/// # zng_app::event::event_args! { pub struct ClickArgs { pub target: zng_txt::Txt, pub click_count: usize, .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) { } } }
/// # use zng_app::handler::async_hn_once;
/// # use zng_task as task;
/// # let _scope = zng_app::APP.minimal();
/// # fn assert_type() -> impl zng_app::handler::WidgetHandler<ClickArgs> {
/// let data = vec![1, 2, 3];
/// # let
/// on_open = async_hn_once!(data, |_| {
///     task::run(async move {
///          for i in data {
///              print!("{i}, ");
///          }    
///     }).await;
///
///     println!("Done!");
/// });
/// println!("{data:?}");
/// # on_open }
/// ```
///
/// [`async_clmv_fn_once!`]: zng_clone_move::async_clmv_fn_once
#[macro_export]
macro_rules! async_hn_once {
    ($($tt:tt)+) => {
        $crate::handler::async_hn_once($crate::handler::async_clmv_fn_once! { $($tt)+ })
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
#[diagnostic::on_unimplemented(
    note = "use `app_hn!(|args: &{A}, _| {{ }})` to declare an app handler closure",
    note = "use `app_hn_once!`, `async_app_hn!` or `async_app_hn_once!` for other closure types"
)]
pub trait AppHandler<A: Clone + 'static>: Any + Send {
    /// Called every time the event happens.
    ///
    /// The `handler_args` can be used to unsubscribe the handler. Async handlers are expected to schedule
    /// their tasks to run somewhere in the app, usually in the [`UPDATES.on_update`]. The `handle` is
    /// **not** expected to cancel running async tasks, only to drop `self` before the next event happens.
    ///
    /// [`UPDATES.on_update`]: crate::update::UPDATES::on_update
    fn event(&mut self, args: &A, handler_args: &AppHandlerArgs);

    /// Boxes the handler.
    ///
    /// The type `Box<dyn AppHandler<A>>` implements `AppHandler<A>` and just returns itself
    /// in this method, avoiding double boxing.
    fn boxed(self) -> Box<dyn AppHandler<A>>
    where
        Self: Sized,
    {
        Box::new(self)
    }

    /// Boxes the handler if the `feature = "dyn_closure"` is enabled, otherwise maintain the same type.
    #[cfg(feature = "dyn_closure")]
    fn cfg_boxed(self) -> Box<dyn AppHandler<A>>
    where
        Self: Sized,
    {
        self.boxed()
    }

    /// Boxes the handler if the `feature = "dyn_closure"` is enabled, otherwise maintain the same type.    
    #[cfg(not(feature = "dyn_closure"))]
    fn cfg_boxed(self) -> Self
    where
        Self: Sized,
    {
        self
    }
}
impl<A: Clone + 'static> AppHandler<A> for Box<dyn AppHandler<A>> {
    fn event(&mut self, args: &A, handler_args: &AppHandlerArgs) {
        self.as_mut().event(args, handler_args)
    }

    fn boxed(self) -> Box<dyn AppHandler<A>> {
        self
    }
}

#[doc(hidden)]
pub struct FnMutAppHandler<H> {
    handler: H,
}
impl<A, H> AppHandler<A> for FnMutAppHandler<H>
where
    A: Clone + 'static,
    H: FnMut(&A, &dyn AppWeakHandle) + Send + 'static,
{
    fn event(&mut self, args: &A, handler_args: &AppHandlerArgs) {
        (self.handler)(args, handler_args.handle);
    }
}
#[doc(hidden)]
#[cfg(not(feature = "dyn_closure"))]
pub fn app_hn<A, H>(handler: H) -> FnMutAppHandler<H>
where
    A: Clone + 'static,
    H: FnMut(&A, &dyn AppWeakHandle) + Send + 'static,
{
    FnMutAppHandler { handler }
}

#[cfg(feature = "dyn_closure")]
type BoxedAppHn<A> = Box<dyn FnMut(&A, &dyn AppWeakHandle) + Send>;

#[doc(hidden)]
#[cfg(feature = "dyn_closure")]
pub fn app_hn<A, H>(handler: H) -> FnMutAppHandler<BoxedAppHn<A>>
where
    A: Clone + 'static,
    H: FnMut(&A, &dyn AppWeakHandle) + Send + 'static,
{
    FnMutAppHandler {
        handler: Box::new(handler),
    }
}

///<span data-del-macro-root></span> Declare a mutable *clone-move* app event handler.
///
/// The macro input is a closure with optional *clone-move* variables, internally it uses [`clmv!`] so
/// the input is the same syntax.
///
/// # Examples
///
/// The example declares an event handler for the `CLICK_EVENT`.
///
/// ```
/// # zng_app::event::event_args! { pub struct ClickArgs { pub target: zng_txt::Txt, pub click_count: usize, .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) { } } }
/// # zng_app::event::event! { pub static CLICK_EVENT: ClickArgs; }
/// # use zng_app::handler::app_hn;
/// # let _scope = zng_app::APP.minimal();
/// # fn assert_type() {
/// CLICK_EVENT.on_event(app_hn!(|_, _| {
///     println!("Clicked Somewhere!");
/// })).perm();
/// # }
/// ```
///
/// The closure input is `&A, &dyn AppWeakHandle` with `&A` equaling `&ClickArgs` for this event. Note that
/// if you want to use the event args you must annotate the input type, the context and handle type is inferred.
///
/// The handle can be used to unsubscribe the event handler, if [`unsubscribe`](AppWeakHandle::unsubscribe) is called the handler
/// will be dropped some time before the next event update.
///
/// ```
/// # zng_app::event::event_args! { pub struct ClickArgs { pub target: zng_txt::Txt, pub click_count: usize, .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) { } } }
/// # zng_app::event::event! { pub static CLICK_EVENT: ClickArgs; }
/// # use zng_app::handler::app_hn;
/// # let _scope = zng_app::APP.minimal();
/// # fn assert_type() {
/// CLICK_EVENT.on_event(app_hn!(|args: &ClickArgs, handle| {
///     println!("Clicked {}!", args.target);
///     handle.unsubscribe();
/// })).perm();
/// # }
/// ```
///
/// Internally the [`clmv!`] macro is used so you can *clone-move* variables into the handler.
///
/// ```
/// # zng_app::event::event_args! { pub struct ClickArgs { pub target: zng_txt::Txt, pub click_count: usize, .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) { } } }
/// # zng_app::event::event! { pub static CLICK_EVENT: ClickArgs; }
/// # use zng_txt::{formatx, ToTxt};
/// # use zng_var::{var, Var};
/// # use zng_app::handler::app_hn;
/// # let _scope = zng_app::APP.minimal();
/// # fn assert_type() {
/// let foo = var("".to_txt());
///
/// CLICK_EVENT.on_event(app_hn!(foo, |args: &ClickArgs, _| {
///     foo.set(args.target.to_txt());
/// })).perm();
///
/// // can still use after:
/// let bar = foo.map(|c| formatx!("last click: {c}"));
///
/// # }
/// ```
///
/// In the example above only a clone of `foo` is moved into the handler. Note that handlers always capture by move, if `foo` was not
/// listed in the *clone-move* section it would not be available after the handler is created. See [`clmv!`] for details.
///
/// [`clmv!`]: zng_clone_move::clmv
#[macro_export]
macro_rules! app_hn {
    ($($tt:tt)+) => {
        $crate::handler::app_hn($crate::handler::clmv!{ $($tt)+ })
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
    H: FnOnce(&A) + Send + 'static,
{
    fn event(&mut self, args: &A, handler_args: &AppHandlerArgs) {
        if let Some(handler) = self.handler.take() {
            handler(args);
            handler_args.handle.unsubscribe();
        } else {
            tracing::error!("`app_hn_once!` called after requesting unsubscribe");
        }
    }
}
#[doc(hidden)]
#[cfg(not(feature = "dyn_closure"))]
pub fn app_hn_once<A, H>(handler: H) -> FnOnceAppHandler<H>
where
    A: Clone + 'static,
    H: FnOnce(&A) + Send + 'static,
{
    FnOnceAppHandler { handler: Some(handler) }
}
#[doc(hidden)]
#[cfg(feature = "dyn_closure")]
pub fn app_hn_once<A, H>(handler: H) -> FnOnceAppHandler<Box<dyn FnOnce(&A) + Send>>
where
    A: Clone + 'static,
    H: FnOnce(&A) + Send + 'static,
{
    FnOnceAppHandler {
        handler: Some(Box::new(handler)),
    }
}

///<span data-del-macro-root></span> Declare a *clone-move* app event handler that is only called once.
///
/// The macro input is a closure with optional *clone-move* variables, internally it uses [`clmv!`] so
/// the input is the same syntax.
///
/// # Examples
///
/// The example captures `data` by move and then destroys it in the first call, this cannot be done using [`app_hn!`] because
/// the `data` needs to be available for all event calls. In this case the closure is only called once, subsequent events
/// are ignored by the handler and it automatically requests unsubscribe.
///
/// ```
/// # zng_app::event::event_args! { pub struct ClickArgs { pub target: zng_txt::Txt, pub click_count: usize, .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) { } } }
/// # zng_app::event::event! { pub static CLICK_EVENT: ClickArgs; }
/// # use zng_app::handler::app_hn_once;
/// # let _scope = zng_app::APP.minimal();
/// # fn assert_type() {
/// let data = vec![1, 2, 3];
///
/// CLICK_EVENT.on_event(app_hn_once!(|_| {
///     for i in data {
///         print!("{i}, ");
///     }
/// })).perm();
/// # }
/// ```
///
/// Other then declaring a `FnOnce` this macro behaves like [`app_hn!`], so the same considerations apply. You can *clone-move* variables,
/// the type of the input is the event arguments and must be annotated.
///
/// ```
/// # zng_app::event::event_args! { pub struct ClickArgs { pub target: zng_txt::Txt, pub click_count: usize, .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) { } } }
/// # zng_app::event::event! { pub static CLICK_EVENT: ClickArgs; }
/// # use zng_app::handler::app_hn_once;
/// # let _scope = zng_app::APP.minimal();
/// # fn assert_type() {
/// let data = vec![1, 2, 3];
///
/// CLICK_EVENT.on_event(app_hn_once!(data, |args: &ClickArgs| {
///     drop(data);
/// })).perm();
///
/// println!("{data:?}");
/// # }
/// ```
///
/// [`clmv!`]: zng_clone_move::clmv
#[macro_export]
macro_rules! app_hn_once {
    ($($tt:tt)+) => {
        $crate::handler::app_hn_once($crate::handler::clmv! { $($tt)+ })
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
    F: Future<Output = ()> + Send + 'static,
    H: FnMut(A, Box<dyn AppWeakHandle>) -> F + Send + 'static,
{
    fn event(&mut self, args: &A, handler_args: &AppHandlerArgs) {
        let handler = &mut self.handler;
        let mut task = UiTask::new(None, handler(args.clone(), handler_args.handle.clone_boxed()));
        if task.update().is_none() {
            if handler_args.is_preview {
                UPDATES
                    .on_pre_update(app_hn!(|_, handle| {
                        if task.update().is_some() {
                            handle.unsubscribe();
                        }
                    }))
                    .perm();
            } else {
                UPDATES
                    .on_update(app_hn!(|_, handle| {
                        if task.update().is_some() {
                            handle.unsubscribe();
                        }
                    }))
                    .perm();
            }
        }
    }
}
#[doc(hidden)]
#[cfg(not(feature = "dyn_closure"))]
pub fn async_app_hn<A, F, H>(handler: H) -> AsyncFnMutAppHandler<H>
where
    A: Clone + 'static,
    F: Future<Output = ()> + Send + 'static,
    H: FnMut(A, Box<dyn AppWeakHandle>) -> F + Send + 'static,
{
    AsyncFnMutAppHandler { handler }
}

#[cfg(feature = "dyn_closure")]
type BoxedAsyncAppHn<A> = Box<dyn FnMut(A, Box<dyn AppWeakHandle>) -> std::pin::Pin<Box<dyn Future<Output = ()> + Send>> + Send>;

#[doc(hidden)]
#[cfg(feature = "dyn_closure")]
pub fn async_app_hn<A, F, H>(mut handler: H) -> AsyncFnMutAppHandler<BoxedAsyncAppHn<A>>
where
    A: Clone + 'static,
    F: Future<Output = ()> + Send + 'static,
    H: FnMut(A, Box<dyn AppWeakHandle>) -> F + Send + 'static,
{
    AsyncFnMutAppHandler {
        handler: Box::new(move |args, handle| Box::pin(handler(args, handle))),
    }
}

///<span data-del-macro-root></span> Declare an async *clone-move* app event handler.
///
/// The macro input is a closure with optional *clone-move* variables, internally it uses [`async_clmv_fn!`] so
/// the input is the same syntax.
///
/// The handler generates a future for each event, the future is polled immediately if it does not finish it is scheduled
/// to update in [`on_pre_update`](crate::update::UPDATES::on_pre_update) or [`on_update`](crate::update::UPDATES::on_update) depending
/// on if the handler was assigned to a *preview* event or not.
///
/// Note that this means [`propagation`](crate::event::AnyEventArgs::propagation) can only be meaningfully stopped before the
/// first `.await`, after, the event has already propagated.
///
/// # Examples
///
/// The example declares an async event handler for the `CLICK_EVENT`.
///
/// ```
/// # zng_app::event::event_args! { pub struct ClickArgs { pub target: zng_txt::Txt, pub click_count: usize, .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) { } } }
/// # zng_app::event::event! { pub static CLICK_EVENT: ClickArgs; }
/// # use zng_app::handler::async_app_hn;
/// # use zng_task as task;
/// # let _scope = zng_app::APP.minimal();
/// # fn assert_type() {
/// CLICK_EVENT.on_event(async_app_hn!(|_, _| {
///     println!("Clicked Somewhere!");
///
///     task::run(async {
///         println!("In other thread!");
///     }).await;
///
///     println!("Back in UI thread, in an app update.");
/// })).perm();
/// # }
/// ```
///
/// The closure input is `A, Box<dyn AppWeakHandle>` for all handlers and `A` is `ClickArgs` for this example. Note that
/// if you want to use the event args you must annotate the input type, the context and handle types are inferred.
///
/// The handle can be used to unsubscribe the event handler, if [`unsubscribe`](AppWeakHandle::unsubscribe) is called the handler
/// will be dropped some time before the next event update. Running tasks are not canceled by unsubscribing, the only way to *cancel*
/// then is by returning early inside the async blocks.
///
/// ```
/// # zng_app::event::event_args! { pub struct ClickArgs { pub target: zng_txt::Txt, pub click_count: usize, .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) { } } }
/// # zng_app::event::event! { pub static CLICK_EVENT: ClickArgs; }
/// # use zng_app::handler::async_app_hn;
/// # use zng_task as task;
/// # let _scope = zng_app::APP.minimal();
/// # fn assert_type() {
/// CLICK_EVENT.on_event(async_app_hn!(|args: ClickArgs, handle| {
///     println!("Clicked {}!", args.target);
///     task::run(async move {
///         handle.unsubscribe();
///     });
/// })).perm();
/// # }
/// ```
///
/// Internally the [`async_clmv_fn!`] macro is used so you can *clone-move* variables into the handler.
///
/// ```
/// # zng_app::event::event_args! { pub struct ClickArgs { pub target: zng_txt::Txt, pub click_count: usize, .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) { } } }
/// # zng_app::event::event! { pub static CLICK_EVENT: ClickArgs; }
/// # use zng_app::handler::async_app_hn;
/// # use zng_var::{var, Var};
/// # use zng_task as task;
/// # use zng_txt::{formatx, ToTxt};
/// #
/// # let _scope = zng_app::APP.minimal();
/// # fn assert_type() {
/// let status = var("pending..".to_txt());
///
/// CLICK_EVENT.on_event(async_app_hn!(status, |args: ClickArgs, _| {
///     status.set(formatx!("processing {}..", args.target));
///
///     task::run(async move {
///         println!("do something slow");
///     }).await;
///
///     status.set(formatx!("finished {}", args.target));
/// })).perm();
///
/// // can still use after:
/// let text = status;
///
/// # }
/// ```
///
/// In the example above only a clone of `status` is moved into the handler. Note that handlers always capture by move, if `status` was not
/// listed in the *clone-move* section it would not be available after the handler is created. See [`async_clmv_fn!`] for details.
///
/// ## Futures and Clone-Move
///
/// You may want to always *clone-move* captures for async handlers, because they then automatically get cloned again for each event. This
/// needs to happen because you can have more then one *handler task* running at the same type, and both want access to the captured variables.
///
/// This second cloning can be avoided by using the [`async_hn_once!`] macro instead, but only if you expect a single event.
///
/// [`async_clmv_fn!`]: zng_clone_move::async_clmv_fn
#[macro_export]
macro_rules! async_app_hn {
    ($($tt:tt)+) => {
        $crate::handler::async_app_hn($crate::handler::async_clmv_fn! { $($tt)+ })
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
    F: Future<Output = ()> + Send + 'static,
    H: FnOnce(A) -> F + Send + 'static,
{
    fn event(&mut self, args: &A, handler_args: &AppHandlerArgs) {
        if let Some(handler) = self.handler.take() {
            handler_args.handle.unsubscribe();

            let mut task = UiTask::new(None, handler(args.clone()));
            if task.update().is_none() {
                if handler_args.is_preview {
                    UPDATES
                        .on_pre_update(app_hn!(|_, handle| {
                            if task.update().is_some() {
                                handle.unsubscribe();
                            }
                        }))
                        .perm();
                } else {
                    UPDATES
                        .on_update(app_hn!(|_, handle| {
                            if task.update().is_some() {
                                handle.unsubscribe();
                            }
                        }))
                        .perm();
                }
            }
        } else {
            tracing::error!("`async_app_hn_once!` called after requesting unsubscribe");
        }
    }
}
#[doc(hidden)]
#[cfg(not(feature = "dyn_closure"))]
pub fn async_app_hn_once<A, F, H>(handler: H) -> AsyncFnOnceAppHandler<H>
where
    A: Clone + 'static,
    F: Future<Output = ()> + Send + 'static,
    H: FnOnce(A) -> F + Send + 'static,
{
    AsyncFnOnceAppHandler { handler: Some(handler) }
}

#[cfg(feature = "dyn_closure")]
type BoxedAsyncAppHnOnce<A> = Box<dyn FnOnce(A) -> std::pin::Pin<Box<dyn Future<Output = ()> + Send>> + Send>;

#[doc(hidden)]
#[cfg(feature = "dyn_closure")]
pub fn async_app_hn_once<A, F, H>(handler: H) -> AsyncFnOnceAppHandler<BoxedAsyncAppHnOnce<A>>
where
    A: Clone + 'static,
    F: Future<Output = ()> + Send + 'static,
    H: FnOnce(A) -> F + Send + 'static,
{
    AsyncFnOnceAppHandler {
        handler: Some(Box::new(move |args| Box::pin(handler(args)))),
    }
}

///<span data-del-macro-root></span> Declare an async *clone-move* app event handler that is only called once.
///
/// The macro input is a closure with optional *clone-move* variables, internally it uses [`async_clmv_fn_once!`] so
/// the input is the same syntax.
///
/// # Examples
///
/// The example captures `data` by move and then moves it again to another thread. This is not something you can do using [`async_app_hn!`]
/// because that handler expects to be called many times. We want to handle `CLICK_EVENT` once in this example, so we can don't need
/// to capture by *clone-move* just to use `data`.
///
/// ```
/// # zng_app::event::event_args! { pub struct ClickArgs { pub target: zng_txt::Txt, pub click_count: usize, .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) { } } }
/// # use zng_app::handler::async_hn_once;
/// # use zng_task as task;
/// # let _scope = zng_app::APP.minimal();
/// # fn assert_type() -> impl zng_app::handler::WidgetHandler<ClickArgs> {
/// let data = vec![1, 2, 3];
/// # let
/// on_open = async_hn_once!(|_| {
///     task::run(async move {
///          for i in data {
///              print!("{i}, ");
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
/// # zng_app::event::event_args! { pub struct ClickArgs { pub target: zng_txt::Txt, pub click_count: usize, .. fn delivery_list(&self, _l: &mut UpdateDeliveryList) { } } }
/// # use zng_app::handler::async_hn_once;
/// # use zng_task as task;
/// # let _scope = zng_app::APP.minimal();
/// # fn assert_type() -> impl zng_app::handler::WidgetHandler<ClickArgs> {
/// let data = vec![1, 2, 3];
/// # let
/// on_open = async_hn_once!(data, |_| {
///     task::run(async move {
///          for i in data {
///              print!("{i}, ");
///          }    
///     }).await;
///
///     println!("Done!");
/// });
/// println!("{data:?}");
/// # on_open }
/// ```
///
/// [`async_clmv_fn_once!`]: zng_clone_move::async_clmv_fn_once
#[macro_export]
macro_rules! async_app_hn_once {
    ($($tt:tt)+) => {
        $crate::handler::async_app_hn_once($crate::handler::async_clmv_fn_once! { $($tt)+ })
    }
}
#[doc(inline)]
pub use crate::async_app_hn_once;
use crate::update::UPDATES;
use crate::widget::{UiTaskWidget, WIDGET};

/// Widget handler wrapper that filters the events, only delegating to `self` when `filter` returns `true`.
pub struct FilterWidgetHandler<A, H, F> {
    _args: PhantomData<fn() -> A>,
    handler: H,
    filter: F,
}
impl<A, H, F> FilterWidgetHandler<A, H, F>
where
    A: Clone + 'static,
    H: WidgetHandler<A>,
    F: FnMut(&A) -> bool + Send + 'static,
{
    /// New filter handler.
    pub fn new(handler: H, filter: F) -> Self {
        Self {
            handler,
            filter,
            _args: PhantomData,
        }
    }
}
impl<A, H, F> WidgetHandler<A> for FilterWidgetHandler<A, H, F>
where
    A: Clone + 'static,
    H: WidgetHandler<A>,
    F: FnMut(&A) -> bool + Send + 'static,
{
    fn event(&mut self, args: &A) -> bool {
        if (self.filter)(args) { self.handler.event(args) } else { false }
    }

    fn update(&mut self) -> bool {
        self.handler.update()
    }
}

/// App handler wrapper that filters the events, only delegating to `self` when `filter` returns `true`.
pub struct FilterAppHandler<A, H, F> {
    _args: PhantomData<fn() -> A>,
    handler: H,
    filter: F,
}
impl<A, H, F> FilterAppHandler<A, H, F>
where
    A: Clone + 'static,
    H: AppHandler<A>,
    F: FnMut(&A) -> bool + Send + 'static,
{
    /// New filter handler.
    pub fn new(handler: H, filter: F) -> Self {
        Self {
            handler,
            filter,
            _args: PhantomData,
        }
    }
}
impl<A, H, F> AppHandler<A> for FilterAppHandler<A, H, F>
where
    A: Clone + 'static,
    H: AppHandler<A>,
    F: FnMut(&A) -> bool + Send + 'static,
{
    fn event(&mut self, args: &A, handler_args: &AppHandlerArgs) {
        if (self.filter)(args) {
            self.handler.event(args, handler_args);
        }
    }
}

impl HeadlessApp {
    /// Calls an [`AppHandler<A>`] once and blocks until the update tasks started during the call complete.
    ///
    /// This function *spins* until all update tasks are completed. Timers or send events can
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
    /// This function *spins* until all update tasks are completed. Timers or send events can
    /// be received during execution but the loop does not sleep, it just spins requesting an update
    /// for each pass.
    pub fn block_on_multi<A>(&mut self, handlers: Vec<&mut dyn AppHandler<A>>, args: &A, timeout: Duration) -> Result<(), String>
    where
        A: Clone + 'static,
    {
        let (pre_len, pos_len) = UPDATES.handler_lens();

        let handler_args = AppHandlerArgs {
            handle: &Handle::dummy(()).downgrade(),
            is_preview: false,
        };
        for handler in handlers {
            handler.event(args, &handler_args);
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
    pub fn doc_test<A, H>(args: A, mut handler: H)
    where
        A: Clone + 'static,
        H: AppHandler<A>,
    {
        let mut app = crate::APP.minimal().run_headless(false);
        app.block_on(&mut handler, &args, DOC_TEST_BLOCK_ON_TIMEOUT).unwrap();
    }

    /// Calls the `handlers` once each and [`block_on_multi`] with a 60 seconds timeout.
    ///
    /// [`block_on_multi`]: Self::block_on_multi
    #[track_caller]
    #[cfg(any(test, doc, feature = "test_util"))]
    pub fn doc_test_multi<A>(args: A, mut handlers: Vec<Box<dyn AppHandler<A>>>)
    where
        A: Clone + 'static,
    {
        let mut app = crate::APP.minimal().run_headless(false);
        app.block_on_multi(handlers.iter_mut().map(|h| h.as_mut()).collect(), &args, DOC_TEST_BLOCK_ON_TIMEOUT)
            .unwrap()
    }
}

#[cfg(any(test, doc, feature = "test_util"))]
const DOC_TEST_BLOCK_ON_TIMEOUT: Duration = Duration::from_secs(60);
