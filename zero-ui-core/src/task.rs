//! Parallel async tasks and async task runners.
//!
//! Use the [`run`], [`respond`] or [`spawn`] to run parallel tasks, use [`wait`] to unblock blocking IO operations, and use
//! [`WidgetTask`] to create async properties.
//!
//! This module also re-exports the [`rayon`] crate for convenience.
//!
//! # Examples
//!
//! ```
//! # use zero_ui_core::{widget, UiNode, var::{var, IntoVar}, async_hn, event_property, property,
//! # gesture::{ClickEvent, ClickArgs}, task::{self, rayon::prelude::*}};
//! # #[widget($crate::button)]
//! # pub mod button { }
//! # event_property! { pub fn click { event: ClickEvent, args: ClickArgs, } }
//! # #[property(context)]
//! # fn enabled(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode { child }
//! # fn main() {
//! let enabled = var(false);
//! button! {
//!     on_click = async_hn!(enabled, |ctx, _| {
//!         enabled.set(&ctx, false);
//!
//!         let sum_task = task::run(async {
//!             let numbers = read_numbers().await;
//!             numbers.par_iter().map(|i| i * i).sum()
//!         });
//!         let sum: usize = sum_task.await;
//!         println!("sum of squares: {}", sum);
//!
//!         enabled.set(&ctx, true);
//!     });
//!     enabled;
//! }
//! # ; }
//!
//! async fn read_numbers() -> Vec<usize> {
//!     let raw = task::wait(|| std::fs::read_to_string("numbers.txt").unwrap()).await;
//!     raw.par_split(',').map(|s| s.trim().parse::<usize>().unwrap()).collect()
//! }
//! ```
//!
//! The example demonstrates three different ***tasks***, the first is a [`WidgetTask`] in the `async_hn` handler,
//! this task is *async* but not *parallel*, meaning that it will execute in more then one app update, but it will only execute in the app
//! main thread. This is good for coordinating UI state, like setting variables, but is not good if you want to do CPU intensive work.
//!
//! To keep the app responsive we move the computation work inside a [`run`] task, this task is *async* and *parallel*,
//! meaning it can `.await` and will execute in parallel threads. It runs in a [`rayon`] thread-pool so you can
//! easily make the task multi-threaded and when it is done it sends the result back to the widget task that is awaiting for it. We
//! resolved the responsiveness problem, but there is one extra problem to solve, how to not block one of the worker threads waiting IO.
//!
//! We want to keep the [`run`] threads either doing work or available for other tasks, but reading a file is just waiting
//! for a potentially slow external operation, so if we just call `std::fs::read_to_string` directly we can potentially remove one of
//! the worker threads from play, reducing the overall tasks performance. To avoid this we move the IO operation inside a [`wait`]
//! task, this task is not *async* but it is *parallel*, meaning if does not block but it runs a blocking operation. It runs inside
//! a [`blocking`] thread-pool, that is optimized for waiting.
//!
//! # Async Crates Integration
//!
//! This module provides async parallel tasks and IO unblocking but it does not provide more elaborate async IO or async networking.
//! You can use external async crates to create these futures and then `.await` then in async code managed by Zero-Ui, but there is some
//! consideration needed. Async code needs a runtime to execute and some async functions from external crates expect their own runtime
//! to work properly, as a rule of thumb if the crate starts their own *event reactor* you can just use then without worry.
//!
//! You can use the [`futures`], [`async-std`] and [`smol`] crates without worry, they integrate well and even use the same [`blocking`]
//! thread-pool that is used in [`wait`]. Functions that require an *event reactor* start it automatically, usually at the cost of one extra
//! thread only. Just `.await` futures from these crate.
//!
//! The [`tokio`] crate on the other hand, does not integrate well. It does not start its own runtime automatically, and expects you
//! to call its async functions from inside the tokio runtime. After you created a future from inside the runtime you can `.await` then
//! in any thread at least, so if you have no alternative but to use [`tokio`] we recommend manually starting its runtime in a thread and
//! then using the `tokio::runtime::Handle` to start futures in the runtime.
//!
//! [`blocking`]: https://docs.rs/blocking
//! [`futures`]: https://docs.rs/futures
//! [`async-std`]: https://docs.rs/async-std
//! [`smol`]: https://docs.rs/smol
//! [`tokio`]: https://docs.rs/tokio

use std::{
    fmt,
    future::Future,
    io,
    pin::Pin,
    sync::Arc,
    task::{Poll, Waker},
    time::{Duration, Instant},
};

use parking_lot::Mutex;
use pin_project::pin_project;

use crate::{
    context::*,
    var::{response_channel, ResponseVar, VarValue, WithVars},
};

#[doc(no_inline)]
pub use rayon;

/// Spawn a parallel async task, this function is not blocking and the `task` starts executing immediately.
///
/// # Parallel
///
/// The task runs in the primary [`rayon`] thread-pool, every [`poll`](Future::poll) happens inside a call to [`rayon::spawn`].
///
/// You can use parallel iterators, `join` or any of rayon's utilities inside `task` to make it multi-threaded,
/// otherwise it will run in a single thread at a time, still not blocking the UI.
///
/// The [`rayon`] crate is re-exported in `task::rayon` for convenience.
///
/// # Async
///
/// The `task` is also a future so you can `.await`, after each `.await` the task continues executing in whatever `rayon` thread
/// is free, so the `task` should either be doing CPU intensive work or awaiting, blocking IO operations
/// block the thread from being used by other tasks reducing overall performance. You can use [`wait`] for IO
/// or blocking operations and for networking you can use any of the async crates, as long as they start their own *event reactor*.
///
/// Of course, if you know that your app is only running one task at a time you can just use the blocking `std` functions
/// directly, that will still execute in parallel. The UI runs in the main thread and the renderers
/// have their own `rayon` thread-pool, so blocking one of the task threads does not matter in a small app.
///
/// The `task` lives inside the [`Waker`] when awaiting and inside [`rayon::spawn`] when running.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::{context::WidgetContext, task::{self, rayon::iter::*}, var::{ResponseVar, response_channel}};
/// # struct SomeStruct { sum_response: ResponseVar<usize> }
/// # impl SomeStruct {
/// fn on_event(&mut self, ctx: &mut WidgetContext) {
///     let (sender, response) = response_channel(ctx);
///     self.sum_response = response;
///
///     task::spawn(async move {
///         let r = (0..1000).into_par_iter().map(|i| i * i).sum();
///
///         sender.send_response(r);
///     });
/// }
///
/// fn on_update(&mut self, ctx: &mut WidgetContext) {
///     if let Some(result) = self.sum_response.rsp_new(ctx) {
///         println!("sum of squares 0..1000: {}", result);   
///     }
/// }
/// # }
/// ```
///
/// The example uses the `rayon` parallel iterator to compute a result and uses a [`response_channel`] to send the result to the UI.
///
/// Note that this function is the most basic way to spawn a parallel task where you must setup channels to the rest of the app yourself,
/// you can use [`respond`] to avoid having to manually create a response channel, or [`run`] to `.await`
/// the result.
#[inline]
pub fn spawn<F>(task: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    RayonTask::new(task).poll()
}

/// Spawn a parallel async task that can also be `.await` for the task result.
///
/// The [`spawn`] documentation explains how `task` is *parallel* and *async*. The returned future is
/// *disconnected* from the `task` future, in that polling it does not poll the `task` future and dropping it does not cancel the task.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::{task::{self, rayon::iter::*}};
/// # struct SomeStruct { sum: usize }
/// # async fn read_numbers() -> Vec<usize> { vec![] }
/// # impl SomeStruct {
/// async fn on_event(&mut self) {
///     self.sum = task::run(async {
///         read_numbers().await.par_iter().map(|i| i * i).sum()
///     }).await;
/// }
/// # }
/// ```
///
/// The example `.await` for some numbers and then uses a parallel iterator to compute a result, this all runs in parallel
/// because it is inside a `run` task. The task result is then `.await` inside one of the UI async tasks.
#[inline]
pub async fn run<R, T>(task: T) -> R
where
    R: Send + 'static,
    T: Future<Output = R> + Send + 'static,
{
    let (sender, receiver) = flume::bounded(1);

    spawn(async move {
        let r = task.await;
        let _ = sender.send(r);
    });

    receiver.into_recv_async().await.unwrap()
}

/// Spawn a parallel async task that will send its result to a [`ResponseVar`].
///
/// The [`spawn`] documentation explains how `task` is *parallel* and *async*. The `task` starts executing immediately.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::{context::WidgetContext, task::{self, rayon::iter::*}, var::ResponseVar};
/// # struct SomeStruct { sum_response: ResponseVar<usize> }
/// # async fn read_numbers() -> Vec<usize> { vec![] }
/// # impl SomeStruct {
/// fn on_event(&mut self, ctx: &mut WidgetContext) {
///     self.sum_response = task::respond(ctx, async {
///         read_numbers().await.par_iter().map(|i| i * i).sum()
///     });
/// }
///
/// fn on_update(&mut self, ctx: &mut WidgetContext) {
///     if let Some(result) = self.sum_response.rsp_new(ctx) {
///         println!("sum of squares: {}", result);   
///     }
/// }
/// # }
/// ```
///
/// The example `.await` for some numbers and then uses a parallel iterator to compute a result. The result is send to
/// `sum_response` that is a [`ResponseVar`].
#[inline]
pub fn respond<Vw: WithVars, R, F>(vars: &Vw, task: F) -> ResponseVar<R>
where
    R: VarValue + Send + 'static,
    F: Future<Output = R> + Send + 'static,
{
    let (sender, response) = response_channel(vars);

    spawn(async move {
        let r = task.await;
        let _ = sender.send_response(r);
    });

    response
}

/// Create a parallel `task` that blocks awaiting for an IO operation, the `task` starts on the first `.await`.
///
/// # Parallel
///
/// The `task` runs in the [`blocking`] thread-pool which is optimized for awaiting blocking operations.
/// If the `task` is computation heavy you should use [`run`] and then `wait` inside that task for the
/// parts that are blocking.
///
/// # Examples
///
/// ```
/// # fn main() { }
/// # use zero_ui_core::task;
/// # struct SomeStruct { sum_response: ResponseVar<usize> }
/// # async fn example() {
/// task::wait(|| std::fs::read_to_string("file.txt")).await
/// # ; }
/// ```
///
/// The example reads a file, that is a blocking file IO operation, most of the time is spend waiting for the operating system,
/// so we offload this to a `wait` task. The task can be `.await` inside a [`run`] task or inside one of the UI tasks
/// like in a async event handler.
///
/// # Read/Write Tasks
///
/// For [`io::Read`] and [`io::Write`] operations you can also use [`ReadTask`] and [`WriteTask`] where you don't
/// have or want the full file in memory.
///
/// ```no_run
/// # use zero_ui_core::task::{self, ReadTask, WriteTask};
/// # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
/// let input_file = task::wait(|| std::fs::File::open("large-input.bin")).await?;
/// let output_file = task::wait(|| std::fs::File::create("large-output.bin")).await?;
///
/// let reader = ReadTask::new(input_file, 8);
/// let writer = WriteTask::new(input_file, 8);
/// # Ok(()) }
/// ```
///
/// [`blocking`]: https://docs.rs/blocking
#[inline]
pub async fn wait<T, F>(task: F) -> T
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    blocking::unblock(task).await
}

/// Fire and forget a [`wait`] task. The `task` starts executing immediately.
#[inline]
pub fn spawn_wait<F>(task: F)
where
    F: FnOnce() + Send + 'static,
{
    spawn(async move { wait(task).await });
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
        AppTask::new(self, task)
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
        WidgetTask::new(self, task)
    }
}

/// Represents a [`Future`] running in the UI thread.
///
/// The future [`Waker`](std::task::Waker), wakes the app event loop and causes an update, in a update handler
/// [`update`](UiTask::update) must be called, if this task waked the app the future is polled once.
pub struct UiTask<R> {
    future: Pin<Box<dyn Future<Output = R>>>,
    event_loop_waker: Waker,
    result: Option<R>,
}
impl<R> UiTask<R> {
    /// Create a app thread bound future executor.
    ///
    /// The `task` is inert and must be polled using [`update`](UiTask::update) to start, and it must be polled every
    /// [`UiNode::update`](crate::UiNode::update) after that.
    pub fn new<F: Future<Output = R> + 'static>(updates: &Updates, task: F) -> Self {
        UiTask {
            future: Box::pin(task),
            event_loop_waker: updates.sender().waker(),
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
pub struct WidgetTask<R> {
    task: UiTask<R>,
    scope: WidgetContextScope,
}
impl<R> WidgetTask<R> {
    /// Create an app thread bound future executor that executes in the context of a widget.
    ///
    /// The `task` closure is called immediately with the [`WidgetContextMut`] that is paired with the task, it
    /// should return the task future `F` in an inert state. Calls to [`WidgetTask::update`] exclusive borrow a
    /// [`WidgetContext`] that is made available inside `F` using the [`WidgetContextMut::with`] method.
    pub fn new<F, T>(ctx: &mut WidgetContext, task: T) -> WidgetTask<R>
    where
        R: 'static,
        F: Future<Output = R> + 'static,
        T: FnOnce(WidgetContextMut) -> F,
    {
        let (scope, mut_) = WidgetContextScope::new();

        let task = scope.with(ctx, move || task(mut_));

        WidgetTask {
            task: UiTask::new(ctx.updates, task),
            scope,
        }
    }

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

/// Represents a [`Future`] running in the UI thread in the app context.
///
/// The future [`Waker`](std::task::Waker), wakes the app event loop and causes an update, a update handler
/// then calls [`update`](Self::update) and if this task waked the app the future is polled once.
pub struct AppTask<R> {
    task: UiTask<R>,
    scope: AppContextScope,
}
impl<R> AppTask<R> {
    /// Create an app thread bound future executor that executes in the app context.
    ///
    /// The `task` closure is called immediately with the [`AppContextMut`] that is paired with the task, it
    /// should return the task future `F` in an inert state. Calls to [`AppTask::update`] exclusive borrow the
    /// [`AppContext`] that is made available inside `F` using the [`AppContextMut::with`] method.
    pub fn new<F, T>(ctx: &mut AppContext, task: T) -> AppTask<R>
    where
        R: 'static,
        F: Future<Output = R> + 'static,
        T: FnOnce(AppContextMut) -> F,
    {
        let (scope, mut_) = AppContextScope::new();

        let task = scope.with(ctx, move || task(mut_));

        AppTask {
            task: UiTask::new(ctx.updates, task),
            scope,
        }
    }

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
                .on_pre_update(app_hn!(|ctx, _, handle| {
                    if self.update(ctx).is_some() {
                        handle.unsubscribe();
                    }
                }))
                .permanent();
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
            let mut cx = std::task::Context::from_waker(&waker);
            let _ = self.future.lock().as_mut().poll(&mut cx);
        })
    }
}
impl std::task::Wake for RayonTask {
    fn wake(self: Arc<Self>) {
        self.poll()
    }
}

/// A future that is [`Pending`] once.
///
/// After the first `.await` the future is always [`Ready`].
///
/// # Warning
///
/// This does not schedule an [`wake`], if the executor does not poll this future again it will wait forever.
///
/// [`Pending`]: std::task::Poll::Pending
/// [`Ready`]: std::task::Poll::Ready
/// [`wake`]: std::task::Waker::wake
pub async fn yield_one() {
    struct YieldOneFut(bool);
    impl Future for YieldOneFut {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, _: &mut std::task::Context<'_>) -> Poll<Self::Output> {
            if self.0 {
                Poll::Ready(())
            } else {
                self.0 = true;
                Poll::Pending
            }
        }
    }

    YieldOneFut(false).await
}

/// A future that is [`Pending`] until the `timeout` is reached.
///
/// # Examples
///
/// Await 5 seconds in a [`spawn`] parallel task:
///
/// ```
/// use zero_ui_core::{task, units::*};
///
/// task::spawn(async {
///     println!("waiting 5 seconds..");
///     task::timeout(5.secs()).await;
///     println!("5 seconds elapsed.")
/// });
/// ```
///
/// The timer does not block the worker thread, parallel timers use their own executor thread managed by
/// the [`futures_timer`] crate. This is not a high-resolution timer, it can elapse slightly after the time has passed.
///
/// # UI Async
///
/// This timer works in UI async tasks too, but you should use the [`Timers`] instead, as they are implemented using only
/// the app loop they use the same *executor* as the app or widget tasks.
///
/// [`Pending`]: std::task::Poll::Pending
/// [`futures_timer`]: https://docs.rs/futures-timer
/// [`Timers`]: crate::timer::Timers#async
#[inline]
pub async fn timeout(timeout: Duration) {
    futures_timer::Delay::new(timeout).await
}

/// A future that is [`Pending`] until the `deadline` has passed.
///
///  This function just calculates the [`timeout`], from the time this method is called minus `deadline`.
///
/// [`Pending`]: std::task::Poll::Pending
pub async fn deadline(deadline: Instant) {
    let now = Instant::now();
    if deadline > now {
        timeout(deadline - now).await
    }
}

/// Implement a [`Future`] from a closure.
pub async fn poll_fn<T, F: FnMut(&mut std::task::Context) -> Poll<T>>(fn_: F) -> T {
    struct PollFn<F>(F);
    impl<F> Unpin for PollFn<F> {}
    impl<T, F: FnMut(&mut std::task::Context<'_>) -> Poll<T>> Future for PollFn<F> {
        type Output = T;

        fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
            (&mut self.0)(cx)
        }
    }

    PollFn(fn_).await
}

/// Error when [`with_timeout`] or [`with_deadline`] reach a time limit before a task finishes.
#[derive(Debug, Clone, Copy)]
pub struct TimeoutError;

/// Add a [`timeout`] to a future.
///
/// Returns the `fut` output or [`TimeoutError`] if the timeout elapses first.
pub async fn with_timeout<O, F: Future<Output = O>>(fut: F, timeout: Duration) -> Result<F::Output, TimeoutError> {
    #[pin_project]
    struct WithTimeoutFut<F, T> {
        #[pin]
        fut: F,
        #[pin]
        t_fut: T,
    }
    impl<O, F: Future<Output = O>, T: Future<Output = ()>> Future for WithTimeoutFut<F, T> {
        type Output = Result<O, TimeoutError>;

        fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
            let s = self.project();
            match s.fut.poll(cx) {
                Poll::Ready(r) => Poll::Ready(Ok(r)),
                Poll::Pending => match s.t_fut.poll(cx) {
                    Poll::Ready(_) => Poll::Ready(Err(TimeoutError)),
                    Poll::Pending => Poll::Pending,
                },
            }
        }
    }
    WithTimeoutFut {
        fut,
        t_fut: self::timeout(timeout),
    }
    .await
}

/// Add a [`deadline`] to a future.
///
/// Returns the `fut` output or [`TimeoutError`] if the deadline elapses first.
pub async fn with_deadline<O, F: Future<Output = O>>(fut: F, deadline: Instant) -> Result<F::Output, TimeoutError> {
    let now = Instant::now();
    if deadline < now {
        Err(TimeoutError)
    } else {
        with_timeout(fut, deadline - now).await
    }
}

/// Async channels.
///
/// The channel can work across UI tasks and parallel tasks, it can be [`bounded`] or [`unbounded`] and is MPMC.
///
/// This module is a thin wrapper around the [`flume`] crate's channel that just limits the API
/// surface to only `async` methods. You can convert from/into that [`flume`] channel.
///
/// # Examples
///
/// ```no_run
/// use zero_ui_core::{task::{self, channel}, units::*};
///
/// let (sender, receiver) = channel::bounded(5);
///
/// task::spawn(async move {
///     task::timeout(5.secs()).await;
///     if let Err(e) = sender.send("Data!").await {
///         eprintln!("no receiver connected, did not send message: '{}'", e.0)
///     }
/// });
/// task::spawn(async move {
///     match receiver.recv().await {
///         Ok(msg) => println!("{}", msg),
///         Err(_) => eprintln!("no message in channel and no sender connected")
///     }
/// });
/// ```
///
/// [`bounded`]: channel::bounded
/// [`unbounded`]: channel::unbounded
/// [`flume`]: https://docs.rs/flume/0.10.7/flume/
pub mod channel {
    use std::{
        any::type_name,
        convert::TryFrom,
        fmt,
        time::{Duration, Instant},
    };

    pub use flume::{RecvError, RecvTimeoutError, SendError, SendTimeoutError};

    /// The transmitting end of an unbounded channel.
    ///
    /// Use [`unbounded`] to create a channel.
    pub struct UnboundSender<T>(flume::Sender<T>);
    impl<T> fmt::Debug for UnboundSender<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "UnboundSender<{}>", type_name::<T>())
        }
    }
    impl<T> Clone for UnboundSender<T> {
        fn clone(&self) -> Self {
            UnboundSender(self.0.clone())
        }
    }
    impl<T> TryFrom<flume::Sender<T>> for UnboundSender<T> {
        type Error = flume::Sender<T>;

        /// Convert to [`UnboundSender`] if the flume sender is unbound.
        fn try_from(value: flume::Sender<T>) -> Result<Self, Self::Error> {
            if value.capacity().is_none() {
                Ok(UnboundSender(value))
            } else {
                Err(value)
            }
        }
    }
    impl<T> From<UnboundSender<T>> for flume::Sender<T> {
        fn from(s: UnboundSender<T>) -> Self {
            s.0
        }
    }
    impl<T> UnboundSender<T> {
        /// Send a value into the channel.
        ///
        /// If the messages are not received they accumulate in the channel buffer.
        ///
        /// Returns an error if all receivers have been dropped.
        pub fn send(&self, msg: T) -> Result<(), SendError<T>> {
            self.0.send(msg)
        }

        /// Returns `true` if all receivers for this channel have been dropped.
        #[inline]
        pub fn is_disconnected(&self) -> bool {
            self.0.is_disconnected()
        }

        /// Returns `true` if the channel is empty.
        #[inline]
        pub fn is_empty(&self) -> bool {
            self.0.is_empty()
        }

        /// Returns the number of messages in the channel.
        #[inline]
        pub fn len(&self) -> usize {
            self.0.len()
        }
    }

    /// The transmitting end of a channel.
    ///
    /// Use [`bounded`] or [`rendezvous`] to create a channel. You can also convert an [`UnboundSender`] into this one.
    pub struct Sender<T>(flume::Sender<T>);
    impl<T> fmt::Debug for Sender<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "Sender<{}>", type_name::<T>())
        }
    }
    impl<T> Clone for Sender<T> {
        fn clone(&self) -> Self {
            Sender(self.0.clone())
        }
    }
    impl<T> From<flume::Sender<T>> for Sender<T> {
        fn from(s: flume::Sender<T>) -> Self {
            Sender(s)
        }
    }
    impl<T> From<Sender<T>> for flume::Sender<T> {
        fn from(s: Sender<T>) -> Self {
            s.0
        }
    }
    impl<T> Sender<T> {
        /// Send a value into the channel.
        ///
        /// Waits until there is space in the channel buffer.
        ///
        /// Returns an error if all receivers have been dropped.
        pub async fn send(&self, msg: T) -> Result<(), SendError<T>> {
            self.0.send_async(msg).await
        }

        /// Send a value into the channel.
        ///
        /// Waits until there is space in the channel buffer or the `timeout` is reached.
        ///
        /// Returns an error if all receivers have been dropped or the `timeout` is reached.
        pub async fn send_timeout(&self, msg: T, timeout: Duration) -> Result<(), SendTimeoutError<T>> {
            match super::with_timeout(self.send(msg), timeout).await {
                Ok(r) => match r {
                    Ok(_) => Ok(()),
                    Err(e) => Err(SendTimeoutError::Disconnected(e.0)),
                },
                Err(_t) => {
                    // TODO: wait for https://github.com/zesterer/flume/pull/84
                    //
                    todo!("wait for send_timeout_async impl")
                }
            }
        }

        /// Send a value into the channel.
        ///
        /// Waits until there is space in the channel buffer or the `deadline` has passed.
        ///
        /// Returns an error if all receivers have been dropped or the `deadline` has passed.
        pub async fn send_deadline(&self, msg: T, deadline: Instant) -> Result<(), SendTimeoutError<T>> {
            let now = Instant::now();
            if deadline < now {
                Err(SendTimeoutError::Timeout(msg))
            } else {
                self.send_timeout(msg, deadline - now).await
            }
        }

        /// Returns `true` if all receivers for this channel have been dropped.
        #[inline]
        pub fn is_disconnected(&self) -> bool {
            self.0.is_disconnected()
        }

        /// Returns `true` if the channel is empty.
        ///
        /// Note: [`rendezvous`] channels are always empty.
        #[inline]
        pub fn is_empty(&self) -> bool {
            self.0.is_empty()
        }

        /// Returns `true` if the channel is full.
        ///
        /// Note: [`rendezvous`] channels are always full and [`unbounded`] channels are never full.
        #[inline]
        pub fn is_full(&self) -> bool {
            self.0.is_full()
        }

        /// Returns the number of messages in the channel.
        #[inline]
        pub fn len(&self) -> usize {
            self.0.len()
        }

        /// If the channel is bounded, returns its capacity.
        #[inline]
        pub fn capacity(&self) -> Option<usize> {
            self.0.capacity()
        }
    }

    /// The receiving end of a channel.
    ///
    /// Use [`bounded`],[`unbounded`] or [`rendezvous`] to create a channel.
    ///
    /// # Work Stealing
    ///
    /// Cloning the receiver **does not** turn this channel into a broadcast channel.
    /// Each message will only be received by a single receiver. You can use this to
    /// to implement work stealing.
    pub struct Receiver<T>(flume::Receiver<T>);
    impl<T> fmt::Debug for Receiver<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "Receiver<{}>", type_name::<T>())
        }
    }
    impl<T> Clone for Receiver<T> {
        fn clone(&self) -> Self {
            Receiver(self.0.clone())
        }
    }
    impl<T> Receiver<T> {
        /// Wait for an incoming value from the channel associated with this receiver.
        ///
        /// Returns an error if all senders have been dropped.
        pub async fn recv(&self) -> Result<T, RecvError> {
            self.0.recv_async().await
        }

        /// Wait for an incoming value from the channel associated with this receiver.
        ///
        /// Returns an error if all senders have been dropped or the `timeout` is reached.
        pub async fn recv_timeout(&self, timeout: Duration) -> Result<T, RecvTimeoutError> {
            match super::with_timeout(self.recv(), timeout).await {
                Ok(r) => match r {
                    Ok(m) => Ok(m),
                    Err(_) => Err(RecvTimeoutError::Disconnected),
                },
                Err(_) => Err(RecvTimeoutError::Timeout),
            }
        }

        /// Wait for an incoming value from the channel associated with this receiver.
        ///  
        /// Returns an error if all senders have been dropped or the `deadline` has passed.
        pub async fn recv_deadline(&self, deadline: Instant) -> Result<T, RecvTimeoutError> {
            let now = Instant::now();
            if deadline < now {
                Err(RecvTimeoutError::Timeout)
            } else {
                self.recv_timeout(now - deadline).await
            }
        }

        /// Returns `true` if all senders for this channel have been dropped.
        #[inline]
        pub fn is_disconnected(&self) -> bool {
            self.0.is_disconnected()
        }

        /// Returns `true` if the channel is empty.
        ///
        /// Note: [`rendezvous`] channels are always empty.
        #[inline]
        pub fn is_empty(&self) -> bool {
            self.0.is_empty()
        }

        /// Returns `true` if the channel is full.
        ///
        /// Note: [`rendezvous`] channels are always full and [`unbounded`] channels are never full.
        #[inline]
        pub fn is_full(&self) -> bool {
            self.0.is_full()
        }

        /// Returns the number of messages in the channel.
        #[inline]
        pub fn len(&self) -> usize {
            self.0.len()
        }

        /// If the channel is bounded, returns its capacity.
        #[inline]
        pub fn capacity(&self) -> Option<usize> {
            self.0.capacity()
        }
    }

    /// Create a channel with no maximum capacity.
    ///
    /// Unbound channels always [`send`] messages immediately, never yielding on await.
    /// If the messages are no [received] they accumulate in the channel buffer.
    ///
    /// # Examples
    ///
    /// The example [spawns] two parallel tasks, the receiver task takes a while to start receiving but then
    /// rapidly consumes all messages in the buffer and new messages as they are send.
    ///
    /// ```no_run
    /// use zero_ui_core::{task::{self, channel}, units::*};
    ///
    /// let (sender, receiver) = channel::unbounded();
    ///
    /// task::spawn(async move {
    ///     for msg in ["Hello!", "Are you still there?"].into_iter().cycle() {
    ///         task::timeout(300.ms());
    ///         if let Err(e) = sender.send(msg) {
    ///             eprintln!("no receiver connected, the message `{}` was not send", e.0);
    ///             break;
    ///         }
    ///     }
    /// });
    /// task::spawn(async move {
    ///     task::timeout(5.secs()).await;
    ///     
    ///     loop {
    ///         match receiver.recv().await {
    ///             Ok(msg) => println!("{}", msg),
    ///             Err(_) => {
    ///                 eprintln!("no message in channel and no sender connected");
    ///                 break;
    ///             }
    ///         }
    ///     }
    /// });
    /// ```
    ///
    /// Note that you don't need to `.await` on [`send`] as there is always space in the channel buffer.
    ///
    /// [`send`]: UnboundSender::send
    /// [received]: Receiver::recv
    /// [spawns]: crate::task::spawn
    #[inline]
    pub fn unbounded<T>() -> (UnboundSender<T>, Receiver<T>) {
        let (s, r) = flume::unbounded();
        (UnboundSender(s), Receiver(r))
    }

    /// Create a channel with a maximum capacity.
    ///
    /// Bounded channels [`send`] until the channel reaches its capacity then it awaits until a message
    /// is [received] before sending another message.
    ///
    /// # Examples
    ///
    /// The example [spawns] two parallel tasks, the receiver task takes a while to start receiving but then
    /// rapidly consumes the 2 messages in the buffer and unblocks the sender to send more messages.
    ///
    /// ```no_run
    /// use zero_ui_core::{task::{self, channel}, units::*};
    ///
    /// let (sender, receiver) = channel::bounded(2);
    ///
    /// task::spawn(async move {
    ///     for msg in ["Hello!", "Data!"].into_iter().cycle() {
    ///         task::timeout(300.ms());
    ///         if let Err(e) = sender.send(msg).await {
    ///             eprintln!("no receiver connected, the message `{}` was not send", e.0);
    ///             break;
    ///         }
    ///     }
    /// });
    /// task::spawn(async move {
    ///     task::timeout(5.secs()).await;
    ///     
    ///     loop {
    ///         match receiver.recv().await {
    ///             Ok(msg) => println!("{}", msg),
    ///             Err(_) => {
    ///                 eprintln!("no message in channel and no sender connected");
    ///                 break;
    ///             }
    ///         }
    ///     }
    /// });
    /// ```
    ///
    /// [`send`]: UnboundSender::send
    /// [received]: Receiver::recv
    /// [spawns]: crate::task::spawn
    #[inline]
    pub fn bounded<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
        let (s, r) = flume::bounded(capacity);
        (Sender(s), Receiver(r))
    }

    /// Create a [`bounded`] channel with `0` capacity.
    ///
    /// Rendezvous channels always awaits until the message is [received] to *return* from [`send`], there is no buffer.
    ///
    /// # Examples
    ///
    /// The example [spawns] two parallel tasks, the sender and receiver *handshake* when transferring the message, the
    /// receiver takes 2 seconds to receive, so the sender takes 2 seconds to send.
    ///
    /// ```no_run
    /// use zero_ui_core::{task::{self, channel}, units::*};
    /// use std::time::*;
    ///
    /// let (sender, receiver) = channel::rendezvous();
    ///
    /// task::spawn(async move {
    ///     loop {
    ///         let t = Instant::now();
    ///
    ///         if let Err(e) = sender.send("the stuff").await {
    ///             eprintln!(r#"failed to send "{}", no receiver connected"#, e.0);
    ///             break;
    ///         }
    ///
    ///         assert!(Instant::now().duration_since(t) >= 2.secs());
    ///     }
    /// });
    /// task::spawn(async move {
    ///     loop {
    ///         task::timeout(2.secs()).await;
    ///
    ///         match receiver.recv().await {
    ///             Ok(msg) => println!(r#"got "{}""#, msg),
    ///             Err(_) => {
    ///                 eprintln!("no sender connected");
    ///                 break;
    ///             }
    ///         }
    ///     }
    /// });
    /// ```
    ///
    /// [`send`]: UnboundSender::send
    /// [received]: Receiver::recv
    /// [spawns]: crate::task::spawn
    #[inline]
    pub fn rendezvous<T>() -> (Sender<T>, Receiver<T>) {
        bounded::<T>(0)
    }
}

/// Represents a running buffered [`io::Read::read_to_end`] operation.
pub struct ReadTask<R: io::Read> {
    receiver: channel::Receiver<Result<Vec<u8>, ReadTaskError<R>>>,
    stop_recv: channel::Receiver<R>,
    payload_len: usize,
}
impl<R> ReadTask<R>
where
    R: io::Read + Send + 'static,
{
    /// Start the write task.
    ///
    /// The `payload_len` is the maximum number of bytes returned at a time, the `channel_capacity` is the number
    /// of pending payloads that can be pre-read. The recommended is 1 mebibyte len and 8 payloads.
    pub fn spawn(read: R, payload_len: usize, channel_capacity: usize) -> Self {
        let (sender, receiver) = channel::bounded(channel_capacity);
        let (stop_sender, stop_recv) = channel::bounded(1);
        self::spawn(async move {
            let mut read = read;

            loop {
                let r = self::wait(move || {
                    let mut payload = Vec::with_capacity(payload_len);
                    loop {
                        match read.read(&mut payload) {
                            Ok(c) => {
                                if c < payload_len {
                                    payload.truncate(c);
                                }
                                return Ok((payload, read));
                            }
                            Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                                continue;
                            }
                            Err(e) => return Err(ReadTaskError::new(Some(read), e)),
                        }
                    }
                })
                .await;

                match r {
                    Ok((p, r)) => {
                        read = r;

                        if p.len() < payload_len {
                            let _ = sender.send(Ok(p));
                            let _ = stop_sender.send(read);
                            break;
                        } else if sender.send(Ok(p)).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = sender.send(Err(e));
                        break;
                    }
                }
            }
        });
        ReadTask {
            receiver,
            stop_recv,
            payload_len,
        }
    }

    /// Maximum number of bytes per payload.
    #[inline]
    pub fn payload_len(&self) -> usize {
        self.payload_len
    }

    /// Returns the next payload.
    ///
    /// The payload length can be equal to or less then [`payload_len`]. If it is less, the stream
    /// has reached `EOF` and subsequent read calls will always return the [disconnected] error.
    ///
    /// [`payload_len`]: ReadTask::payload_len
    /// [disconnected]: ReadTaskError::is_disconnected
    pub async fn read(&self) -> Result<Vec<u8>, ReadTaskError<R>> {
        self.receiver.recv().await.map_err(|_| ReadTaskError::disconnected())?
    }

    /// Take back the [`io::Read`], any pending reads are dropped.
    pub async fn stop(self) -> Result<R, ReadTaskError<R>> {
        drop(self.receiver);
        self.stop_recv.recv().await.map_err(|_| ReadTaskError::disconnected())
    }
}

/// Error from [`ReadTask`].
pub struct ReadTaskError<R: io::Read> {
    /// The [`io::Read`] that caused the error.
    ///
    /// Is `None` the error represents a lost connection with the task.
    pub read: Option<R>,
    /// The error.
    pub error: io::Error,
}
impl<R: io::Read> ReadTaskError<R> {
    fn disconnected() -> Self {
        Self::new(
            None,
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "`ReadTask` worker is shutdown, probably caused by an error or panic",
            ),
        )
    }

    fn new(read: Option<R>, error: io::Error) -> Self {
        Self { read, error }
    }

    /// If the error represents a lost connection with the task.
    ///
    /// This can happen after an error was already returned or if a panic killed the [`wait`] thread.
    pub fn is_disconnected(&self) -> bool {
        self.read.is_none()
    }
}
impl<R: io::Read> fmt::Debug for ReadTaskError<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.error, f)
    }
}
impl<R: io::Read> fmt::Display for ReadTaskError<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}
impl<R: io::Read> std::error::Error for ReadTaskError<R> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// Represents a running [`io::Write`] controller.
///
/// This task is recommended for buffered multi megabyte write operations, it spawns a
/// worker that uses [`wait`] to write received bytes that can be send using [`write_all`].
/// If you already have all the bytes you want to write in memory, just move then to a [`wait`]
/// and use the `std` sync file operations to write then, otherwise use this struct.
///
/// You can get the [`io::Write`] back by calling [`finish`], or in most error cases.
///
/// # Examples
///
/// The example writes 1 gibibyte of data generated in batches of 1 mebibyte, if the storage is slow a maximum
/// of 8 megabytes only will exist in memory at a time.
///
/// ```no_run
/// # async fn compute_1mebibyte() -> Vec<u8> { vec![1; 1024 * 1024] }
/// # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
/// use zero_ui_core::task::{self, WriteTask};
///
/// let file = task::wait(|| std::fs::File::create("output.bin")).await?;
/// let write = WriteTask::spawn(file, 8);
///
/// let mut total = 0usize;
/// let limit = 1024 * 1024 * 1024;
/// loop {
///     let payload = compute_1mebibyte().await;
///     total += payload.len();
///
///     write.write_all(payload).await?;
///
///     if total >= limit {
///         let file = write.finish().await?;
///         task::wait(move || file.sync_all()).await?;
///         break;
///     }
/// }
/// # Ok(()) }
/// ```
///
/// # Errors
///
/// Methods of this struct return [`WriteTaskError`], on the first error the task *shuts-down* and drops the wrapped [`io::Write`],
/// subsequent send attempts return the [`BrokenPipe`] error. To recover from errors keep track of the last successful write offset,
/// then on error reacquire write access and seek that offset before starting a new [`WriteTask`].
///
/// [`write_all`]: WriteTask::write_all
/// [`finish`]: WriteTask::finish
/// [`BrokenPipe`]: io::ErrorKind::BrokenPipe
pub struct WriteTask<W: io::Write> {
    sender: channel::Sender<WriteTaskMsg<W>>,
}
impl<W> WriteTask<W>
where
    W: io::Write + Send + 'static,
{
    /// Start the write task.
    ///
    /// The `channel_capacity` is the number of pending operations that can be in the channel.
    /// Recommended is `8` using 1 mebibyte payloads.
    pub fn spawn(write: W, channel_capacity: usize) -> Self {
        let (sender, receiver) = channel::bounded(channel_capacity);
        self::spawn(async move {
            let mut write = write;

            while let Ok(msg) = receiver.recv().await {
                match msg {
                    WriteTaskMsg::WriteAll(bytes, rsp) => {
                        let r = self::wait(move || match write.write_all(&bytes) {
                            Ok(_) => Ok(write),
                            Err(e) => Err(WriteTaskError::new(Some(write), bytes, e)),
                        })
                        .await;

                        match r {
                            Ok(w) => {
                                write = w;
                                if rsp.send(Ok(())).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                let _ = rsp.send(Err(e));
                                break;
                            }
                        }
                    }
                    WriteTaskMsg::Flush(rsp) => {
                        let r = self::wait(move || match write.flush() {
                            Ok(_) => Ok(write),
                            Err(e) => Err(WriteTaskError::new(Some(write), vec![], e)),
                        })
                        .await;

                        match r {
                            Ok(w) => {
                                write = w;
                                if rsp.send(Ok(())).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                let _ = rsp.send(Err(e));
                                break;
                            }
                        }
                    }
                    WriteTaskMsg::Finish(rsp) => {
                        let r = self::wait(move || match write.flush() {
                            Ok(_) => Ok(write),
                            Err(e) => Err(WriteTaskError::new(Some(write), vec![], e)),
                        })
                        .await;

                        let _ = rsp.send(r);
                        break;
                    }
                }
            }
        });
        WriteTask { sender }
    }

    /// Request a [`Write::write_all`] call.
    ///
    /// The recommended size for `bytes` is around 1 mebibyte.
    ///
    /// Await to get the `write_all` call result.
    ///
    /// [`Write::write_all`]: io::Write::write_all.
    pub async fn write_all(&self, bytes: Vec<u8>) -> Result<(), WriteTaskError<W>> {
        let (rsv, rcv) = channel::rendezvous();
        self.sender.send(WriteTaskMsg::WriteAll(bytes, rsv)).await.map_err(|e| {
            if let WriteTaskMsg::WriteAll(bytes, _) = e.0 {
                WriteTaskError::disconnected(bytes)
            } else {
                unreachable!()
            }
        })?;

        rcv.recv().await.map_err(|_| WriteTaskError::disconnected(vec![]))?
    }

    /// Request a [`Write::flush`] call.
    ///
    /// Await to get the `flush` call result.
    ///
    /// [`Write::flush`]: io::Write::flush
    pub async fn flush(&self) -> Result<(), WriteTaskError<W>> {
        let (rsv, rcv) = channel::rendezvous();
        self.sender
            .send(WriteTaskMsg::Flush(rsv))
            .await
            .map_err(|_| WriteTaskError::disconnected(vec![]))?;
        rcv.recv().await.map_err(|_| WriteTaskError::disconnected(vec![]))?
    }

    /// Flush and take back the [`io::Write`].
    pub async fn finish(self) -> Result<W, WriteTaskError<W>> {
        let (rsv, rcv) = channel::rendezvous();
        self.sender
            .send(WriteTaskMsg::Finish(rsv))
            .await
            .map_err(|_| WriteTaskError::disconnected(vec![]))?;

        rcv.recv().await.map_err(|_| WriteTaskError::disconnected(vec![]))?
    }
}

enum WriteTaskMsg<W: io::Write> {
    WriteAll(Vec<u8>, channel::Sender<Result<(), WriteTaskError<W>>>),
    Flush(channel::Sender<Result<(), WriteTaskError<W>>>),
    Finish(channel::Sender<Result<W, WriteTaskError<W>>>),
}

/// Error from [`WriteTask`].
pub struct WriteTaskError<W: io::Write> {
    /// The [`io::Write`] that caused the error.
    ///
    /// Is `None` the error represents a lost connection with the task.
    pub write: Option<W>,
    /// The bytes that where not fully written before the error happened.
    pub payload: Vec<u8>,
    /// The error.
    pub error: io::Error,
}
impl<W: io::Write> WriteTaskError<W> {
    fn disconnected(payload: Vec<u8>) -> Self {
        Self::new(
            None,
            payload,
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "`WriteTask` worker is shutdown, probably caused by an error or panic",
            ),
        )
    }

    fn new(write: Option<W>, payload: Vec<u8>, error: io::Error) -> Self {
        Self { write, payload, error }
    }

    /// If the error represents a lost connection with the task.
    ///
    /// This can happen after an error was already returned or if a panic killed the [`wait`] thread.
    pub fn is_disconnected(&self) -> bool {
        self.write.is_none()
    }
}
impl<W: io::Write> fmt::Debug for WriteTaskError<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.error, f)
    }
}
impl<W: io::Write> fmt::Display for WriteTaskError<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}
impl<W: io::Write> std::error::Error for WriteTaskError<W> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}
