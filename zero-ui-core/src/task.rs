//! Parallel async tasks and async task runners.
//!
//! The [`Tasks`] `struct` contains associated functions for running parallel and async tasks.
//!
//! This module also re-exports the [`rayon`] crate for convenience.

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::Waker,
};

use crate::{
    context::*,
    var::{response_channel, ResponseVar, VarValue, Vars},
};

#[doc(no_inline)]
pub use rayon;

/// Asynchronous task runner.
///
/// An instance of this struct is available in [`AppContext`] and derived contexts, note that most
/// of utility of this `struct` is available as associated functions (no instance required).
pub struct Tasks {
    event_loop_waker: Waker,
}
/// Multi-threaded parallel async tasks.
impl Tasks {
    pub(crate) fn new(event_loop_waker: Waker) -> Self {
        Tasks { event_loop_waker }
    }

    /// Spawn a parallel async task, this function is not blocking and the `task` starts executing immediately.
    ///
    /// # Parallel
    ///
    /// The task runs in the primary [`rayon`] thread-pool, every [`poll`](Future::poll) inside a call to [`rayon::spawn`].
    ///
    /// You can use parallel iterators, `join` or any of rayon's utilities inside `task` to make it multi-threaded,
    /// otherwise it will run in a single thread at a time, still not blocking the UI.
    ///
    /// The [`rayon`] crate is re-exported in `task::rayon` for convenience.
    ///
    /// # Async
    ///
    /// The `task` is also a future so you can `.await`, after each `.await` the task continues executing in whatever rayon thread
    /// is free to process it, so the `task` should either be doing CPU intensive work or awaiting, blocking IO operations
    /// block one of the task threads from being used by other tasks reducing overall performance. You can use any of the async libraries
    /// for IO, as long as they start and host their own *event reactor*. The `task` lives inside the [`Waker`] when awaiting and inside
    /// [`rayon::spawn`] when executing.
    ///
    /// Of course, if you know that your app is only running one task at a time you don't need to import an async library and `.await`
    /// just use the blocking `std` functions, that will still execute in parallel. The UI runs in the main thread and the renderers
    /// have their own rayon thread-pool, so blocking one of the task threads does not matter in a small app.
    ///
    /// # Example
    ///
    /// ```
    /// # use zero_ui_core::{context::WidgetContext, task::{Tasks, rayon::iter::*}, var::{ResponseVar, response_channel}};
    /// # struct SomeStruct { sum_response: ResponseVar<usize> }
    /// # impl SomeStruct {
    /// fn on_event(&mut self, ctx: &mut WidgetContext) {
    ///     let (sender, response) = response_channel(ctx.vars);
    ///     self.sum_response = response;
    ///
    ///     Tasks::run(async move {
    ///         let r = (0..1000).into_par_iter().map(|i| i * i).sum();
    ///
    ///         sender.send_response(r);
    ///     });
    /// }
    ///
    /// fn on_update(&mut self, ctx: &mut WidgetContext) {
    ///     if let Some(result) = self.sum_response.response_new(ctx.vars) {
    ///         println!("sum of squares 0..1000: {}", result);   
    ///     }
    /// }
    /// # }
    /// ```
    ///
    /// The example uses the `rayon` parallel iterator to compute a result and uses a [`response_channel`] to send the result to the UI.
    ///
    /// Note that this function is the most basic way to spawn a parallel task where you must setup channels to the rest of the app yourself,
    /// you can use [`Tasks::run_respond`] to avoid having to manually create a response channel, or [`Tasks::run_async`] to `.await`
    /// the result.
    #[inline]
    pub fn run<F>(task: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        RayonTask::new(task).poll()
    }

    /// Like [`run`](Tasks::run) but you can `.await` for the task result.
    ///
    /// The [`run`](Tasks::run) documentation explains how `task` is *parallel* and *async*. The returned future is
    /// *disconnected* from the `task` future, in that polling it does not poll the `task` future.
    ///
    /// # Example
    ///
    /// ```
    /// # use zero_ui_core::{widget, UiNode, var::{var, IntoVar}, async_clone_move, event_property, property,
    /// # gesture::{ClickEvent, ClickArgs}, task::{Tasks, rayon::prelude::*}};
    /// # #[widget($crate::button)]
    /// # pub mod button { }
    /// # event_property! { pub fn click { event: ClickEvent, args: ClickArgs, } }
    /// # #[property(context)]
    /// # fn enabled(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode { child }
    /// # async fn read_numbers_file() -> Vec<usize> { vec![] }
    /// # fn main() {
    /// let enabled = var(false);
    /// button! {
    ///     on_click_async = async_clone_move!(enabled, |ctx, _| {
    ///         ctx.with(|ctx| enabled.set(ctx.vars, false));
    ///
    ///         let sum_task = Tasks::run_async(async {
    ///             let numbers = read_numbers_file().await;
    ///             numbers.par_iter().map(|i| i * i).sum()
    ///         });
    ///         let r: usize = sum_task.await;
    ///         println!("sum of squares: {}", r);
    ///
    ///         ctx.with(|ctx| enabled.set(ctx.vars, true));
    ///     });
    ///     enabled;
    /// }
    /// # ;
    /// # }
    /// ```
    ///
    /// The example has two `.await` calls, the first `.await` is inside the `run_async` task and waits for a file read and parse,
    /// this does not block the UI because it is running in parallel, and it does not block one of the parallel threads because the read
    /// operation is done using the OS async API. After the numbers are read, rayon's parallel iterator is used to compute squares and sum,
    /// at this point the task is potentially multi-threaded, with each of the parallel threads engaged in squaring and summing a
    /// portion of the numbers. After this the result is available in `sum_task`.
    ///
    /// The second `.await` is called in the async event handler, this event handler is **not** parallel, the code outside the `run_async`
    /// task runs in the UI thread. The `sum_task` is running from the start, the `sum_task.await` instruction only awaits for a message
    /// from the parallel task with the result, it does not poll the task.
    #[inline]
    pub async fn run_async<R, T>(task: T) -> R
    where
        R: Send + 'static,
        T: Future<Output = R> + Send + 'static,
    {
        let (sender, receiver) = flume::bounded(1);

        Tasks::run(async move {
            let r = task.await;
            let _ = sender.send(r);
        });

        receiver.into_recv_async().await.unwrap()
    }

    /// Like [`run`](Tasks::run) but the result is send to a [`ResponseVar`] when the task finishes.
    ///
    /// The [`run`](Tasks::run) documentation explains how `task` is *parallel* and *async*.
    ///
    /// # Example
    ///
    /// ```
    /// # use zero_ui_core::{context::WidgetContext, task::{Tasks, rayon::iter::*}, var::ResponseVar};
    /// # struct SomeStruct { sum_response: ResponseVar<usize> }
    /// # async fn read_numbers() -> Vec<usize> { vec![] }
    /// # impl SomeStruct {
    /// fn on_event(&mut self, ctx: &mut WidgetContext) {
    ///     self.sum_response = Tasks::run_respond(ctx.vars, async {
    ///         read_numbers().await.par_iter().map(|i| i * i).sum()
    ///     });
    /// }
    ///
    /// fn on_update(&mut self, ctx: &mut WidgetContext) {
    ///     if let Some(result) = self.sum_response.response_new(ctx.vars) {
    ///         println!("sum of squares: {}", result);   
    ///     }
    /// }
    /// # }
    /// ```
    ///
    /// The example `.await` for some numbers and then uses a parallel iterator to compute a result. The result is send to
    /// `sum_response` that is a [`ResponseVar`].
    #[inline]
    pub fn run_respond<R, F>(vars: &Vars, task: F) -> ResponseVar<R>
    where
        R: VarValue + Send + 'static,
        F: Future<Output = R> + Send + 'static,
    {
        let (sender, response) = response_channel(vars);

        Tasks::run(async move {
            let r = task.await;
            let _ = sender.send_response(r);
        });

        response
    }
}

/// Single-threaded async tasks.
impl Tasks {
    /// Create a app thread bound future executor.
    ///
    /// The `task` is inert and must be polled using [`UiTask::update`] to start, and it must be polled every
    /// [`UiNode::update`](crate::UiNode::update) after that.
    pub fn ui_task<R, T>(&mut self, task: T) -> UiTask<R>
    where
        R: 'static,
        T: Future<Output = R> + 'static,
    {
        UiTask::new(task, self.event_loop_waker.clone())
    }

    /// Create an app thread bound future executor that executes in the context of a widget.
    ///
    /// The `task` closure is called immediately with the [`WidgetContextMut`] that is paired with the task, it
    /// should return the task future `F` in an inert state. Calls to [`WidgetTask::update`] exclusive borrow a
    /// [`WidgetContext`] that is made available inside `F` using the [`WidgetContextMut::with`] method.
    pub fn widget_task<R, F, T>(ctx: &mut WidgetContext, task: T) -> WidgetTask<R>
    where
        R: 'static,
        F: Future<Output = R> + 'static,
        T: FnOnce(WidgetContextMut) -> F,
    {
        let (scope, mut_) = WidgetContextScope::new();

        let task = scope.with(ctx, move || task(mut_));

        WidgetTask {
            task: ctx.tasks.ui_task(task),
            scope,
        }
    }

    /// Create an app thread bound future executor that executes in the context of a window.
    ///
    /// The `task` closure is called immediately with the [`WindowContextMut`] that is paired with the task, it
    /// should return the task future `F` in an inert state. Calls to [`WindowTask::update`] exclusive borrow a
    /// [`WindowContext`] that is made available inside `F` using the [`WindowContextMut::with`] method.
    pub fn window_task<R, F, T>(ctx: &mut WindowContext, task: T) -> WindowTask<R>
    where
        R: 'static,
        F: Future<Output = R> + 'static,
        T: FnOnce(WindowContextMut) -> F,
    {
        let (scope, mut_) = WindowContextScope::new();

        let task = scope.with(ctx, move || task(mut_));

        WindowTask {
            task: ctx.tasks.ui_task(task),
            scope,
        }
    }

    /// Create an app thread bound future executor that executes in the app context.
    ///
    /// The `task` closure is called immediately with the [`AppContextMut`] that is paired with the task, it
    /// should return the task future `F` in an inert state. Calls to [`AppTask::update`] exclusive borrow the
    /// [`AppContext`] that is made available inside `F` using the [`AppContextMut::with`] method.
    pub fn app_task<R, F, T>(ctx: &mut AppContext, task: T) -> AppTask<R>
    where
        R: 'static,
        F: Future<Output = R> + 'static,
        T: FnOnce(AppContextMut) -> F,
    {
        let (scope, mut_) = AppContextScope::new();

        let task = scope.with(ctx, move || task(mut_));

        AppTask {
            task: ctx.tasks.ui_task(task),
            scope,
        }
    }
}
impl<'a, 'w> AppContext<'a, 'w> {
    /// Create an app thread bound future executor that executes in the app context.
    ///
    /// The `task` closure is called immediately with the [`AppContextMut`] that is paired with the task, it
    /// should return the task future `F` in an inert state. Calls to [`AppTask::update`] exclusive borrow the
    /// [`AppContext`] that is made available inside `F` using the [`AppContextMut::with`] method.
    #[inline]
    pub fn async_task<R, F, T>(&mut self, task: T) -> AppTask<R>
    where
        R: 'static,
        F: Future<Output = R> + 'static,
        T: FnOnce(AppContextMut) -> F,
    {
        Tasks::app_task(self, task)
    }
}
impl<'a> WindowContext<'a> {
    /// Create an app thread bound future executor that executes in the context of a window.
    ///
    /// The `task` closure is called immediately with the [`WindowContextMut`] that is paired with the task, it
    /// should return the task future `F` in an inert state. Calls to [`WindowTask::update`] exclusive borrow a
    /// [`WindowContext`] that is made available inside `F` using the [`WindowContextMut::with`] method.
    #[inline]
    pub fn async_task<R, F, T>(&mut self, task: T) -> WindowTask<R>
    where
        R: 'static,
        F: Future<Output = R> + 'static,
        T: FnOnce(WindowContextMut) -> F,
    {
        Tasks::window_task(self, task)
    }
}
impl<'a> WidgetContext<'a> {
    /// Create an app thread bound future executor that executes in the context of a widget.
    ///
    /// The `task` closure is called immediately with the [`WidgetContextMut`] that is paired with the task, it
    /// should return the task future `F` in an inert state. Calls to [`WidgetTask::update`] exclusive borrow a
    /// [`WidgetContext`] that is made available inside `F` using the [`WidgetContextMut::with`] method.
    #[inline]
    pub fn async_task<R, F, T>(&mut self, task: T) -> WidgetTask<R>
    where
        R: 'static,
        F: Future<Output = R> + 'static,
        T: FnOnce(WidgetContextMut) -> F,
    {
        Tasks::widget_task(self, task)
    }
}

/// Represents a [`Future`] running in the UI thread.
///
/// The future [`Waker`](std::task::Waker), wakes the app event loop and causes an update, in a update handler
/// [`update`](UiTask::update) must be called, if this task waked the app the future is polled once.
///
/// Use the [`Tasks::ui_task`] method to create an UI task.
pub struct UiTask<R> {
    future: Pin<Box<dyn Future<Output = R>>>,
    event_loop_waker: Waker,
    result: Option<R>,
}
impl<R> UiTask<R> {
    fn new<F: Future<Output = R> + 'static>(future: F, event_loop_waker: Waker) -> Self {
        UiTask {
            future: Box::pin(future),
            event_loop_waker,
            result: None,
        }
    }

    /// Polls the future if needed, returns a reference to the result if the task is done.
    ///
    /// This does not poll the future if the task is done.
    #[inline]
    pub fn update(&mut self) -> Option<&R> {
        if self.result.is_some() {
            return self.result.as_ref();
        }

        match self
            .future
            .as_mut()
            .poll(&mut std::task::Context::from_waker(&self.event_loop_waker))
        {
            std::task::Poll::Ready(r) => {
                self.result = Some(r);
                self.result.as_ref()
            }
            std::task::Poll::Pending => None,
        }
    }

    /// Returns the result if the task is completed.
    ///
    /// This does not poll the future, you must call [`update`](Self::update) to poll until a result is available,
    /// then call this method to take ownership of the result.
    #[inline]
    pub fn into_result(self) -> Result<R, Self> {
        if self.result.is_some() {
            Ok(self.result.unwrap())
        } else {
            Err(self)
        }
    }
}

/// Represents a [`Future`] running in the UI thread in a widget context.
///
/// The future [`Waker`](std::task::Waker), wakes the app event loop and causes an update, the widget that is running this task
/// calls [`update`](Self::update) and if this task waked the app the future is polled once.
///
/// Use the [`Tasks::widget_task`] method to create a widget task.
pub struct WidgetTask<R> {
    task: UiTask<R>,
    scope: WidgetContextScope,
}
impl<R> WidgetTask<R> {
    /// Polls the future if needed, returns a reference to the result if the task is done.
    ///
    /// This does not poll the future if the task is done, it also only polls the future if it requested poll.
    #[inline]
    pub fn update(&mut self, ctx: &mut WidgetContext) -> Option<&R> {
        let task = &mut self.task;
        self.scope.with(ctx, move || task.update())
    }

    /// Returns the result if the task is completed.
    ///
    /// This does not poll the future, you must call [`update`](Self::update) to poll until a result is available,
    /// then call this method to take ownership of the result.
    #[inline]
    pub fn into_result(self) -> Result<R, Self> {
        if self.task.result.is_some() {
            Ok(self.task.result.unwrap())
        } else {
            Err(self)
        }
    }
}

/// Represents a [`Future`] running in the UI thread in a window context.
///
/// The future [`Waker`](std::task::Waker), wakes the app event loop and causes an update, the window that is running this task
/// calls [`update`](Self::update) and if this task waked the app the future is polled once.
///
/// Use the [`Tasks::window_task`] method to create a window task.
pub struct WindowTask<R> {
    task: UiTask<R>,
    scope: WindowContextScope,
}
impl<R> WindowTask<R> {
    /// Polls the future if needed, returns a reference to the result if the task is done.
    ///
    /// This does not poll the future if the task is done, it also only polls the future if it requested poll.
    #[inline]
    pub fn update(&mut self, ctx: &mut WindowContext) -> Option<&R> {
        let task = &mut self.task;
        self.scope.with(ctx, move || task.update())
    }

    /// Returns the result if the task is completed.
    ///
    /// This does not poll the future, you must call [`update`](Self::update) to poll until a result is available,
    /// then call this method to take ownership of the result.
    #[inline]
    pub fn into_result(self) -> Result<R, Self> {
        if self.task.result.is_some() {
            Ok(self.task.result.unwrap())
        } else {
            Err(self)
        }
    }
}

/// Represents a [`Future`] running in the UI thread in the app context.
///
/// The future [`Waker`](std::task::Waker), wakes the app event loop and causes an update, a update handler
/// then calls [`update`](Self::update) and if this task waked the app the future is polled once.
///
/// Use the [`Tasks::app_task`] method to create an app task.
pub struct AppTask<R> {
    task: UiTask<R>,
    scope: AppContextScope,
}
impl<R> AppTask<R> {
    /// Polls the future if needed, returns a reference to the result if the task is done.
    ///
    /// This does not poll the future if the task is done, it also only polls the future if it requested poll.
    #[inline]
    pub fn update(&mut self, ctx: &mut AppContext) -> Option<&R> {
        let task = &mut self.task;
        self.scope.with(ctx, move || task.update())
    }

    /// Returns the result if the task is completed.
    ///
    /// This does not poll the future, you must call [`update`](Self::update) to poll until a result is available,
    /// then call this method to take ownership of the result.
    #[inline]
    pub fn into_result(self) -> Result<R, Self> {
        if self.task.result.is_some() {
            Ok(self.task.result.unwrap())
        } else {
            Err(self)
        }
    }
}
impl AppTask<()> {
    /// Schedule the app task to run to completion.
    pub fn run(mut self, updates: &mut Updates) {
        if self.task.result.is_none() {
            updates
                .on_pre_update(move |ctx, args| {
                    if self.update(ctx).is_some() {
                        args.unsubscribe();
                    }
                })
                .forget();
        }
    }
}

/// A future that is its own waker that polls inside the rayon primary thread-pool.
struct RayonTask {
    future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>,
}
impl RayonTask {
    fn new<F>(future: F) -> Arc<Self>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        Arc::new(RayonTask {
            future: Mutex::new(Box::pin(future)),
        })
    }

    fn poll(self: Arc<RayonTask>) {
        rayon::spawn(move || {
            let waker = self.clone().into();
            if let Ok(mut t) = self.future.lock() {
                let mut cx = std::task::Context::from_waker(&waker);
                let _ = t.as_mut().poll(&mut cx);
            }
        })
    }
}
impl std::task::Wake for RayonTask {
    fn wake(self: Arc<Self>) {
        self.poll()
    }
}
