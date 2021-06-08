//! Asynchronous task runner

use std::{
    future::Future,
    pin::Pin,
    sync::{
        atomic::{self, AtomicBool},
        Arc,
    },
};

use crate::{
    app::EventLoopProxySync,
    var::{response_channel, ResponseVar, VarValue, Vars},
};

/// Asynchronous task runner.
pub struct Tasks {
    event_loop: EventLoopProxySync,
}
impl Tasks {
    pub(super) fn new(event_loop: EventLoopProxySync) -> Self {
        Tasks { event_loop }
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

    /// Create a app thread bound future executor.
    ///
    /// The `task` is inert and must be polled using [`UiTask::update`] to start, and it must be polled every
    /// [`UiNode::update`](crate::UiNode::update) after that.
    pub fn ui_task<R, T>(&mut self, task: T) -> UiTask<R>
    where
        R: 'static,
        T: Future<Output = R> + 'static,
    {
        UiTask::new(task, self.event_loop.clone())
    }
}

/// Represents a [`Future`] running in the UI thread in the context of a widget.
///
/// The future [`Waker`](std::task::Waker), wakes the app event loop and causes an update, in a update handler
/// [`update`](UiTask::update) must be called, if this task waked the app the future is polled once.
///
/// Use the [`Tasks::ui_task`] method to create an UI task.
pub struct UiTask<R> {
    future: Pin<Box<dyn Future<Output = R>>>,
    waker: Arc<EventLoopWaker>,
    result: Option<R>,
}
impl<R> UiTask<R> {
    fn new<F: Future<Output = R> + 'static>(future: F, event_loop: EventLoopProxySync) -> Self {
        UiTask {
            future: Box::pin(future),
            waker: EventLoopWaker::new(event_loop),
            result: None,
        }
    }

    /// Polls the future if needed, returns a reference to the result if the task is done.
    ///
    /// This does not poll the future if the task is done, it also only polls the future if it requested poll.
    #[inline]
    pub fn update(&mut self) -> Option<&R> {
        if self.result.is_some() {
            return self.result.as_ref();
        }

        if self.waker.poll.swap(false, atomic::Ordering::Relaxed) {
            let waker = std::task::Waker::from(Arc::clone(&self.waker));
            match self.future.as_mut().poll(&mut std::task::Context::from_waker(&waker)) {
                std::task::Poll::Ready(r) => {
                    self.result = Some(r);
                    self.result.as_ref()
                }
                std::task::Poll::Pending => None,
            }
        } else {
            None
        }
    }

    /// Returns the result if the task is completed.
    ///
    /// This does not poll the future, you must call [`update`](Self::update) to poll until a result is available,
    /// then call this method to take ownership of the result.
    #[inline]
    pub fn into_result(self) -> Result<R, UiTask<R>> {
        if self.result.is_some() {
            Ok(self.result.unwrap())
        } else {
            Err(self)
        }
    }
}

struct EventLoopWaker {
    event_loop: EventLoopProxySync,
    poll: AtomicBool,
}
impl EventLoopWaker {
    fn new(event_loop: EventLoopProxySync) -> Arc<Self> {
        Arc::new(EventLoopWaker {
            event_loop,
            poll: AtomicBool::new(true),
        })
    }
}
impl std::task::Wake for EventLoopWaker {
    fn wake(self: Arc<Self>) {
        self.poll.store(true, atomic::Ordering::Relaxed);
        let _ = self.event_loop.send_event(crate::app::AppEvent::Update);
    }
}
