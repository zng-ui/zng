//! Asynchronous task runner

use std::{
    cell::Cell,
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::{
        atomic::{self, AtomicBool},
        Arc,
    },
};

use retain_mut::RetainMut;

use crate::{
    app::EventLoopProxySync,
    context::{UnsafeWidgetContext, WidgetContext},
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
    /// # use zero_ui_core::{context::WidgetContext, var::{ResponseVar, response_channel}};
    /// # struct SomeStruct { sum_response: ResponseVar<usize> }
    /// # impl SomeStruct {
    /// fn on_event(&mut self, ctx: &mut WidgetContext) {
    ///     let (sender, response) = response_channel(ctx.vars);
    ///     self.sum_response = response;
    ///     ctx.tasks.run(move ||{
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
    pub fn run<F>(&mut self, task: F)
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
    /// # use zero_ui_core::{context::WidgetContext, var::{ResponseVar, response_channel}};
    /// # struct SomeStruct { file_response: ResponseVar<Vec<u8>> }
    /// # impl SomeStruct {
    /// fn on_event(&mut self, ctx: &mut WidgetContext) {
    ///     let (sender, response) = response_channel(ctx.vars);
    ///     self.file_response = response;
    ///     ctx.tasks.run_async(async move {
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
    pub fn run_async<F>(&mut self, task: F)
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
    pub async fn run_fut<R, T>(&mut self, task: T) -> R
    where
        R: Send + 'static,
        T: FnOnce() -> R + Send + 'static,
    {
        let (sender, receiver) = flume::bounded(1);

        self.run(move || {
            let r = task();
            let _ = sender.send(r);
        });

        receiver.into_recv_async().await.unwrap()
    }

    /// Run a CPU bound parallel task, returns a [`ResponseVar`] that will update when the task returns.
    ///
    /// This is like [`run`](Tasks::run) but with a response var result.
    #[inline]
    pub fn run_respond<R, F>(&mut self, vars: &Vars, task: F) -> ResponseVar<R>
    where
        R: VarValue + Send + 'static,
        F: FnOnce() -> R + Send + 'static,
    {
        let (sender, response) = response_channel(vars);

        self.run(move || {
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
    pub async fn run_async_fut<R, F>(&mut self, task: F) -> R
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
    pub fn run_async_respond<R, F>(&mut self, vars: &Vars, task: F) -> ResponseVar<R>
    where
        R: VarValue + Send + 'static,
        F: Future<Output = R> + Send + 'static,
    {
        let (sender, response) = response_channel(vars);

        self.run_async(async move {
            let r = task.await;
            let _ = sender.send_response(r);
        });

        response
    }

    /// Create a app thread bound future that is polled in the context of a widget.
    ///
    /// The `task` is inert and must be polled using [`WidgetTask::update`] to start,
    /// see [`WidgetTask`] for how to update.
    pub fn widget_task<R, F, T>(&mut self, task: T) -> WidgetTask<R>
    where
        R: 'static,
        F: Future<Output = R> + 'static,
        T: FnOnce(AsyncWidgetContext) -> F + 'static,
    {
        WidgetTask::new(async move { task(AsyncWidgetContext::new()).await }, self.event_loop.clone())
    }
}

/// Represents a [`Future`] running in the UI thread in the context of a widget.
///
/// The future [`Waker`](std::task::Waker), wakes the app event loop and causes an update, in a update handler
/// [`UiNode::update`] must be called, if this task waked the app the future is polled once.
pub struct WidgetTask<R> {
    future: Pin<Box<dyn Future<Output = R>>>,
    waker: Arc<EventLoopWaker>,
    result: Option<R>,
}
impl<R> WidgetTask<R> {
    fn new<F: Future<Output = R> + 'static>(future: F, event_loop: EventLoopProxySync) -> Self {
        WidgetTask {
            future: Box::pin(future),
            waker: EventLoopWaker::new(event_loop),
            result: None,
        }
    }

    /// Polls the future if needed, returns a reference to the result if the task is done.
    ///
    /// This does not poll the future if the task is done, it also only polls the future if it requested poll.
    #[inline]
    pub fn update(&mut self, ctx: &mut WidgetContext) -> Option<&R> {
        let _ = ctx; // TODO

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
    pub fn into_result(self) -> Result<R, WidgetTask<R>> {
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
            poll: AtomicBool::new(false),
        })
    }
}
impl std::task::Wake for EventLoopWaker {
    fn wake(self: Arc<Self>) {
        self.poll.store(true, atomic::Ordering::Relaxed);
        let _ = self.event_loop.send_event(crate::app::AppEvent::Update);
    }
}

fn on_click_async<F, H>(handler: H) -> impl crate::UiNode
where
    F: Future<Output = ()> + 'static,
    H: FnMut(AsyncWidgetContext, ()) -> F + 'static,
{
    OnClickAsync { tasks: vec![], handler }
}

struct OnClickAsync<H> {
    tasks: Vec<WidgetTask<()>>,
    handler: H,
}
#[crate::impl_ui_node(none)]
impl<F, H> crate::UiNode for OnClickAsync<H>
where
    F: Future<Output = ()> + 'static,
    H: FnMut(AsyncWidgetContext, ()) -> F + 'static,
{
    //fn event<A>(&mut self, ctx: &mut WidgetContext, args: &A)
    //where
    //    A: crate::event::EventUpdateArgs,
    //{
    //    if let Some(args) = crate::gesture::ClickEvent::update(args) {
    //        let mut task = ctx.tasks.widget_task(|_| (self.handler)(ctx, args));
    //        if task.update(ctx).is_none() {
    //            self.tasks.push(task);
    //        }
    //    }
    //}
    //
    //fn update(&mut self, ctx: &mut WidgetContext) {
    //    self.tasks.retain_mut(|t| t.update(ctx).is_none());
    //}
}

fn test() {
    on_click_async(move |mut ctx, args| async move {
        ctx.ctx()
            .tasks
            .run_fut(|| {
                println!("hello from thread-pool!");
            })
            .await;

        println!("hello after other thread! {:?}", args);
    });
}

/// An [`WidgetContext`] reference inside an async UI bound future.
///
/// TODO
pub struct AsyncWidgetContext {
    ctx: Rc<UnsafeWidgetContext>,
}
impl AsyncWidgetContext {
    fn new() -> Self {
        todo!()
    }

    pub fn ctx(&mut self) -> WidgetContext {
        // SAFETY: this is safe if its only called inside a future inside WidgetTask::update.
        unsafe { self.ctx.ctx() }
    }
}
