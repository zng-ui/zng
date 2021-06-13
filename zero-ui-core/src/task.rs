//! Asynchronous task runner

use std::{future::Future, pin::Pin, task::Waker};

use crate::{
    context::*,
    var::{response_channel, ResponseVar, VarValue, Vars},
};

/// Asynchronous task runner.
///
/// An instance of this struct is available in [`AppContext`] and derived contexts, note that most
/// of utility of this `struct` is available as associated functions (no instance required).
pub struct Tasks {
    event_loop_waker: Waker,
}
/// Multi-threaded parallel tasks.
impl Tasks {
    pub(crate) fn new(event_loop_waker: Waker) -> Self {
        Tasks { event_loop_waker }
    }

    /// Run a CPU bound parallel task.
    ///
    /// The task runs in a [`rayon`] thread-pool, this function is not blocking.
    ///
    /// # Example
    ///
    /// ```
    /// # use zero_ui_core::{context::WidgetContext, task::Tasks, var::{ResponseVar, response_channel}};
    /// # struct SomeStruct { sum_response: ResponseVar<usize> }
    /// # impl SomeStruct {
    /// fn on_event(&mut self, ctx: &mut WidgetContext) {
    ///     let (sender, response) = response_channel(ctx.vars);
    ///     self.sum_response = response;
    ///     Tasks::run(move ||{
    ///         let r = (0..1000).sum();
    ///         sender.send_response(r);
    ///     });
    /// }
    ///
    /// fn on_update(&mut self, ctx: &mut WidgetContext) {
    ///     if let Some(result) = self.sum_response.response_new(ctx.vars) {
    ///         println!("sum of 0..1000: {}", result);   
    ///     }
    /// }
    /// # }
    /// ```
    #[inline]
    pub fn run<F>(task: F)
    where
        F: FnOnce() + Send + 'static,
    {
        rayon::spawn(task);
    }

    /// Run a CPU bound parallel task with a multi-threading executor.
    ///
    /// The task runs in an [`async-global-executor`] thread-pool, this function is not blocking.
    ///
    /// # Example
    ///
    /// ```
    /// # use zero_ui_core::{context::WidgetContext, task::Tasks, var::{ResponseVar, response_channel}};
    /// # struct SomeStruct { file_response: ResponseVar<Vec<u8>> }
    /// # impl SomeStruct {
    /// fn on_event(&mut self, ctx: &mut WidgetContext) {
    ///     let (sender, response) = response_channel(ctx.vars);
    ///     self.file_response = response;
    ///     Tasks::run_fut(async move {
    ///         todo!("use async_std to read a file");
    ///         let file = vec![];
    ///         sender.send_response(file);    
    ///     });
    /// }
    ///
    /// fn on_update(&mut self, ctx: &mut WidgetContext) {
    ///     if let Some(result) = self.file_response.response_new(ctx.vars) {
    ///         println!("file loaded: {} bytes", result.len());   
    ///     }
    /// }
    /// # }
    /// ```
    #[inline]
    pub fn run_fut<F>(task: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        // TODO use async_executor directly in the rayon thread-pool.
        async_global_executor::spawn(task).detach();
    }

    /// Run a CPU bound parallel task, returns a future that can be awaited on the UI
    /// thread that will poll the result of the task.
    ///
    /// This is like [`run`](Tasks::run) but with an awaitable result.
    #[inline]
    pub async fn run_async<R, T>(task: T) -> R
    where
        R: Send + 'static,
        T: FnOnce() -> R + Send + 'static,
    {
        let (sender, receiver) = flume::bounded(1);

        Tasks::run(move || {
            let r = task();
            let _ = sender.send(r);
        });

        receiver.into_recv_async().await.unwrap()
    }

    /// Run a CPU bound parallel task, returns a [`ResponseVar`] that will update when the task returns.
    ///
    /// This is like [`run`](Tasks::run) but with a response var result.
    #[inline]
    pub fn run_respond<R, F>(vars: &Vars, task: F) -> ResponseVar<R>
    where
        R: VarValue + Send + 'static,
        F: FnOnce() -> R + Send + 'static,
    {
        let (sender, response) = response_channel(vars);

        Tasks::run(move || {
            let r = task();
            let _ = sender.send_response(r);
        });

        response
    }

    /// Run a CPU bound parallel task with a multi-threading executor, returns a future that can be awaited on the UI thread
    /// that will poll the result of the task.
    ///
    /// This is like [`run_async`](Tasks::run_async) but with an awaitable result.
    #[inline]
    pub async fn run_fut_async<R, F>(task: F) -> R
    where
        R: Send + 'static,
        F: Future<Output = R> + Send + 'static,
    {
        async_global_executor::spawn(task).await
    }

    /// Run a CPU bound parallel task with a multi-threading executor, returns a [`ResponseVar`] that will update when the task returns.
    ///
    ///
    /// This is like [`run_async`](Tasks::run_async) but with a response var result.
    #[inline]
    pub fn run_fut_respond<R, F>(vars: &Vars, task: F) -> ResponseVar<R>
    where
        R: VarValue + Send + 'static,
        F: Future<Output = R> + Send + 'static,
    {
        let (sender, response) = response_channel(vars);

        Tasks::run_fut(async move {
            let r = task.await;
            let _ = sender.send_response(r);
        });

        response
    }
}

/// Single threaded async tasks.
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
