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
//! for a potentially slow external operation, so if we just call [`std::fs::read_to_string`] directly we can potentially remove one of
//! the worker threads from play, reducing the overall tasks performance. To avoid this we move the IO operation inside a [`wait`]
//! task, this task is not *async* but it is *parallel*, meaning if does not block but it runs a blocking operation. It runs inside
//! a [`blocking`] thread-pool, that is optimized for waiting.
//!
//! # Async IO
//!
//! Zero-Ui uses [`wait`], [`ReadTask`] and [`WriteTask`] to do async IO internally, the read/write tasks represent the
//! act of reading/writing a large file in segmented payloads, so that the file is not ever fully in memory. For operations
//! that have all the data required in memory we just use the `std` blocking API inside [`wait`].
//!
//! This is a different concept from other async IO implementations that try to provide an *async version* of the blocking API, if
//! you prefer that style you can use [external crates](#async-crates-integration) for async IO, most of then
//! [integrate well](#async-crates-integration) with Zero-Ui tasks.
//!
//! # HTTP Client
//!
//! Zero-Ui uses the [`http`] module for making HTTP functions such as loading an image from a given URL,
//! the [`http`] module is a thin wrapper around the [`isahc`] crate.
//!
//! ```
//! # use zero_ui_core::{*, var::*, handler::*, text::*, gesture::*};
//! # #[widget($crate::button)]
//! # pub mod button { }
//! # event_property! { pub fn click { event: ClickEvent, args: ClickArgs, } }
//! # #[property(context)]
//! # fn enabled(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode { child }
//! # fn main() {
//! let enabled = var(false);
//! let msg = var("loading..".to_text());
//! button! {
//!     on_click = async_hn!(enabled, msg, |ctx, _| {
//!         enabled.set(&ctx, false);
//!
//!         match task::http::get_text("https://httpbin.org/get").await {
//!             Ok(r) => msg.set(&ctx, r),
//!             Err(e) => msg.set(&ctx, formatx!("error: {}", e)),
//!         }
//!
//!         enabled.set(&ctx, true);
//!     });
//! }
//! # ; }
//! ```
//!
//! For multi-megabyte file transfers you can also use [`DownloadTask`] and [`UploadTask`]. For other protocols
//! or alternative HTTP clients you can use [external crates](#async-crates-integration).
//!
//! # Async Crates Integration
//!
//! You can use external async crates to create futures and then `.await` then in async code managed by Zero-Ui, but there is some
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
//! [`DownloadTask`]: crate::task::http::DownloadTask
//! [`UploadTask`]: crate::task::http::UploadTask
//! [`isahc`]: https://docs.rs/isahc
//! [`AppExtension`]: crate::app::AppExtension
//! [`blocking`]: https://docs.rs/blocking
//! [`futures`]: https://docs.rs/futures
//! [`async-std`]: https://docs.rs/async-std
//! [`smol`]: https://docs.rs/smol
//! [`tokio`]: https://docs.rs/tokio

use std::{
    fmt,
    future::Future,
    io, mem,
    pin::Pin,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    task::{Poll, Waker},
    time::{Duration, Instant},
};

use parking_lot::Mutex;

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
/// For [`io::Read`] and [`io::Write`] operations you can also use [`ReadTask`] and [`WriteTask`] when you don't
/// have or want the full file in memory. The example demonstrates a program that could be processing gibibytes of
/// data, but only allocates around 16 mebibytes for the task, in the worst case.
///
/// ```no_run
/// # use zero_ui_core::task::{self, ReadTask, WriteTask, rayon::prelude::*};
/// # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
/// // acquire files, using `wait` directly.
/// let input_file = task::wait(|| std::fs::File::open("large-input.bin")).await?;
/// let output_file = task::wait(|| std::fs::File::create("large-output.bin")).await?;
///
/// // start reading the input, immediately tries to read 8 chunks of 1 mebibyte each.
/// let r = ReadTask::default().spawn(input_file);
/// // start an idle write, with a queue of up to 8 write requests.
/// let w = WriteTask::default().spawn(output_file);
///
/// // both tasks use `wait` internally.
///
/// let mut eof = false;
/// while !eof {
///     // read 1 mebibyte, awaits here if no payload was read yet.
///     let mut data = r.read().await?;
///
///     // when EOF is reached, the data is not the full payload.
///     if data.len() < r.payload_len() {
///         eof = true;
///
///         let garbage = data.len() % 4;
///         if garbage != 0 {
///             data.truncate(data.len() - garbage);
///         }
///         
///         if data.is_empty() {
///             break;
///         }
///     }
///
///     // assuming the example is inside a `run` call,
///     // use rayon to transform the data in parallel.
///     data.par_chunks_mut(4).for_each(|c| c[3] = 255);
///     
///     // queue the data for writing, awaits here if the queue is full.
///     if w.write(data).await.is_err() {
///         // write IO error is in `finish`, error here
///         // just indicates that the task has terminated.
///         break;
///     }
/// }
///
/// // get the files back for more small operations using `wait` directly.
/// let input_file = r.stop().await?;
/// let output_file = w.finish().await?;
/// task::wait(move || output_file.sync_all()).await?;
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

/// Blocks the thread until the `task` future finishes.
///
/// This function is useful for implementing async tests, using it in an app will probably cause
/// the app to stop responding. To test UI task use [`TestWidgetContext::block_on`] or [`HeadlessApp::block_on`].
///
/// The crate [`pollster`] is used to execute the task.
///
/// # Examples
///
/// Test a [`run`] call:
///
/// ```
/// use zero_ui_core::task;
/// # use zero_ui_core::units::*;
/// # async fn foo(u: u8) -> Result<u8, ()> { task::timeout(1.ms()).await; Ok(u) }
///
/// #[test]
/// # fn __() { }
/// pub fn run_ok() {
///     let r = task::block_on(task::run(async {
///         foo(32).await
///     }));
///     
/// #   let value =
///     r.expect("foo(32) was not Ok");
/// #   assert_eq!(32, value);
/// }
/// # run_ok();
/// ```
///
/// [`TestWidgetContext::block_on`]: crate::context::TestWidgetContext::block_on
/// [`HeadlessApp::block_on`]: crate::app::HeadlessApp::block_on
/// [`pollster`]: https://docs.rs/pollster/
#[inline]
pub fn block_on<F>(task: F) -> F::Output
where
    F: Future,
{
    pollster::block_on(task)
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

/// Implements a [`Future`] from a closure.
///
/// # Examples
///
/// A future that is ready with a closure returns `Some(R)`.
///
/// ```
/// use zero_ui_core::task;
/// use std::task::Poll;
///
/// async fn ready_some<R>(mut closure: impl FnMut() -> Option<R>) -> R {
///     task::poll_fn(|cx| {
///         match closure() {
///             Some(r) => Poll::Ready(r),
///             None => Poll::Pending
///         }
///     }).await
/// }
/// ```
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
    any!(async { Ok(fut.await) }, async {
        self::timeout(timeout).await;
        Err::<F::Output, _>(TimeoutError)
    })
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

/// <span data-inline></span> Pins variables on the stack.
///
/// # Examples
///
/// Poll a `!Unpin` future using [`poll_fn`]:
///
/// ```
/// use zero_ui_core::task;
/// use std::future::Future;
/// use std::task::Poll;
///
/// async fn count_poll<F: Future>(fut: F) -> F::Output {
///     task::pin!(fut);
///     let mut count = 0;
///     task::poll_fn(|cx| {
///         count += 1;
///         match fut.as_mut().poll(cx) {
///             Poll::Ready(r) => {
///                 println!("polled {} times", count);
///                 Poll::Ready(r)
///             },
///             p => p
///         }
///     }).await
/// }
/// ```
#[macro_export]
macro_rules! pin {
    ($($var:ident),* $(,)?) => {
        $(
            let mut $var = $var;
            #[allow(unused_mut)]
            let mut $var = unsafe {
                std::pin::Pin::new_unchecked(&mut $var)
            };
        )*
    }
}
#[doc(inline)]
pub use crate::pin;

/// <span data-inline></span> A future that polls all input futures and awaits until all are complete.
///
/// The macro input is a comma separated list of future expressions. The macro output is a future
/// that when ".awaited" produces a tuple of results in the same order as the inputs.
///
/// At least one input future is required and any number of futures is accepted. For more then
/// eight futures a proc-macro is used which may cause code auto-complete to stop working in
/// some IDEs.
///
/// # Examples
///
/// Await for three different futures to complete:
///
/// ```
/// use zero_ui_core::task;
///
/// # let _ = async {
/// let (a, b, c) = task::all!(
///     task::run(async { 'a' }),
///     task::wait(|| "b"),
///     async { b"c" }
/// ).await;
/// # };
/// ```
#[macro_export]
macro_rules! all {
    ($fut0:expr $(,)?) => { $crate::__all! { fut0: $fut0; } };
    ($fut0:expr, $fut1:expr $(,)?) => {
        $crate::__all! {
            fut0: $fut0;
            fut1: $fut1;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr $(,)?) => {
        $crate::__all! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr, $fut3:expr $(,)?) => {
        $crate::__all! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
            fut3: $fut3;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr, $fut3:expr, $fut4:expr $(,)?) => {
        $crate::__all! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
            fut3: $fut3;
            fut4: $fut4;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr, $fut3:expr, $fut4:expr, $fut5:expr $(,)?) => {
        $crate::__all! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
            fut3: $fut3;
            fut4: $fut4;
            fut5: $fut5;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr, $fut3:expr, $fut4:expr, $fut5:expr, $fut6:expr $(,)?) => {
        $crate::__all! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
            fut3: $fut3;
            fut4: $fut4;
            fut5: $fut5;
            fut6: $fut6;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr, $fut3:expr, $fut4:expr, $fut5:expr, $fut6:expr, $fut7:expr $(,)?) => {
        $crate::__all! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
            fut3: $fut3;
            fut4: $fut4;
            fut5: $fut5;
            fut6: $fut6;
            fut7: $fut7;
        }
    };
    ($($fut:expr),+ $(,)?) => { $crate::task::__proc_any_all!{ $crate::__all; $($fut),+ } }
}
#[doc(inline)]
pub use crate::all;

#[doc(hidden)]
#[macro_export]
macro_rules! __all {
    ($($ident:ident: $fut:expr;)+) => {
        {
            $(let mut $ident = $crate::task::AllFut::pin($fut);)+
            $crate::task::poll_fn(move |cx| {
                use std::task::Poll;

                let pending = $($ident.poll(cx))|+;

                if pending {
                    Poll::Pending
                } else {
                    Poll::Ready(($($ident.unwrap()),+))
                }
            })
        }
    }
}
#[doc(hidden)]
pub enum AllFut<F: Future> {
    Pending(Pin<Box<F>>),
    Ready(F::Output),
    Done,
}
#[allow(missing_docs)]
impl<F: Future> AllFut<F> {
    pub fn pin(fut: F) -> Self {
        Self::Pending(Box::pin(fut))
    }

    pub fn poll(&mut self, cx: &mut std::task::Context) -> bool {
        if let Self::Pending(fut) = self {
            match fut.as_mut().poll(cx) {
                Poll::Ready(r) => {
                    *self = Self::Ready(r);
                    false
                }
                Poll::Pending => true,
            }
        } else {
            false
        }
    }

    pub fn unwrap(&mut self) -> F::Output {
        match mem::replace(self, Self::Done) {
            Self::Ready(r) => r,
            _ => unreachable!(),
        }
    }
}

/// <span data-inline></span> A future that polls all futures and awaits until any of then are complete.
///
/// The macro input is comma separated list of future expressions, the futures must
/// all have the same output type. The macro output is a future that when ".awaited" produces
/// a single output type instance returned by the first input future that completes.
///
/// At least one input future is required and any number of futures is accepted. For more then
/// eight futures a proc-macro is used which may cause code auto-complete to stop working in
/// some IDEs.
///
/// # Examples
///
/// Await for the first of three futures to complete:
///
/// ```
/// use zero_ui_core::task;
/// # let _ = async {
/// let r = task::any!(
///     task::run(async { 'a' }),
///     task::wait(|| 'b'),
///     async { 'c' }
/// ).await;
/// # };
/// ```
#[macro_export]
macro_rules! any {
    ($fut0:expr $(,)?) => { $crate::__any! { fut0: $fut0; } };
    ($fut0:expr, $fut1:expr $(,)?) => {
        $crate::__any! {
            fut0: $fut0;
            fut1: $fut1;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr $(,)?) => {
        $crate::__any! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr, $fut3:expr $(,)?) => {
        $crate::__any! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
            fut3: $fut3;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr, $fut3:expr, $fut4:expr $(,)?) => {
        $crate::__any! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
            fut3: $fut3;
            fut4: $fut4;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr, $fut3:expr, $fut4:expr, $fut5:expr $(,)?) => {
        $crate::__any! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
            fut3: $fut3;
            fut4: $fut4;
            fut5: $fut5;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr, $fut3:expr, $fut4:expr, $fut5:expr, $fut6:expr $(,)?) => {
        $crate::__any! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
            fut3: $fut3;
            fut4: $fut4;
            fut5: $fut5;
            fut6: $fut6;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr, $fut3:expr, $fut4:expr, $fut5:expr, $fut6:expr, $fut7:expr $(,)?) => {
        $crate::__any! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
            fut3: $fut3;
            fut4: $fut4;
            fut5: $fut5;
            fut6: $fut6;
            fut7: $fut7;
        }
    };
    ($($fut:expr),+ $(,)?) => { $crate::task::__proc_any_all!{ $crate::__any; $($fut),+ } }
}
#[doc(inline)]
pub use crate::any;

#[doc(hidden)]
#[macro_export]
macro_rules! __any {
    ($($ident:ident: $fut:expr;)+) => {
        {
            $(let mut $ident = Box::pin($fut);)+
            $crate::task::poll_fn(move |cx| {
                use std::task::Poll;
                use std::future::Future;
                $(
                    if let Poll::Ready(r) = $ident.as_mut().poll(cx) {
                        return Poll::Ready(r)
                    }
                )+

                Poll::Pending
            })
        }
    }
}

#[doc(hidden)]
pub use zero_ui_proc_macros::task_any_all as __proc_any_all;

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
        #[inline]
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

        /// Takes all sitting in the channel.
        #[inline]
        pub fn drain(&self) -> flume::Drain<T> {
            self.0.drain()
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
///
/// This task is recommended for buffered multi megabyte read operations, it spawns a
/// worker that uses [`wait`] to read byte payloads that can be received using [`read`].
/// If you already have all the bytes you want to write in memory, just move then to a [`wait`]
/// and use the `std` sync file operations to read then, otherwise use this struct.
///
/// You can get the [`io::Read`] back by calling [`stop`], or in most error cases.
///
/// # Examples
///
/// The example reads 1 gibibyte of data, if the storage is faster then the computation a maximum
/// of 8 megabytes only will exist in memory at a time.
///
/// ```no_run
/// # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
/// use zero_ui_core::task::{self, ReadTask, rayon::prelude::*};
/// let file = task::wait(|| std::fs::File::open("data-1gibibyte.bin")).await?;
/// let r = ReadTask::default().spawn(file);
/// let mut foo = 0usize;
///
/// let mut eof = false;
/// while !eof {
///     let payload = r.read().await?;
///     eof = payload.len() < r.payload_len();
///     foo += payload.into_par_iter().filter(|&b|b == 0xF0).count();
/// }
///
/// let file = r.stop().await?;
/// let meta = task::wait(move || file.metadata()).await?;
///
/// println!("found 0xF0 {} times in {} bytes", foo, meta.len());
/// # Ok(()) }
/// ```
///
/// # Errors
///
/// Methods of this struct return [`ReadTaskError`], on the first error the task *shuts-down* and drops the wrapped [`io::Read`],
/// subsequent send attempts return the [`BrokenPipe`] error. To recover from errors keep track of the last successful read offset,
/// then on error reacquire read access and seek that offset before starting a new [`ReadTask`].
///
/// [`read`]: ReadTask::read
/// [`stop`]: ReadTask::stop
/// [`BrokenPipe`]: io::ErrorKind::BrokenPipe
pub struct ReadTask<R> {
    receiver: channel::Receiver<Result<Vec<u8>, ReadTaskError<R>>>,
    stop_recv: channel::Receiver<R>,
    payload_len: usize,
}
impl ReadTask<()> {
    /// Start building a read task.
    ///
    /// # Examples
    ///
    /// Start a task that reads 1 mebibyte payloads and with a maximum of 8 pre-reads in the channel:
    ///
    /// ```
    /// # use zero_ui_core::task::ReadTask;
    /// # fn demo(read: impl std::io::Read + Send + 'static) {
    /// let task = ReadTask::default().spawn(read);
    /// # }
    /// ```
    ///
    /// Start a task with custom configuration:
    ///
    /// ```
    /// # use zero_ui_core::task::ReadTask;
    /// # const FRAME_LEN: usize = 1024 * 1024 * 2;
    /// # const FRAME_COUNT: usize = 3;
    /// # fn demo(read: impl std::io::Read + Send + 'static) {
    /// let task = ReadTask::default()
    ///     .payload_len(FRAME_LEN)
    ///     .channel_capacity(FRAME_COUNT.min(8))
    ///     .spawn(read);
    /// # }
    /// ```
    #[inline]
    pub fn default() -> ReadTaskBuilder {
        ReadTaskBuilder::default()
    }
}
impl<R> ReadTask<R>
where
    R: io::Read + Send + 'static,
{
    /// Start the write task.
    ///
    /// The `payload_len` is the maximum number of bytes returned at a time, the `channel_capacity` is the number
    /// of pending payloads that can be pre-read. The recommended is 1 mebibyte len and 8 payloads.
    fn spawn(builder: ReadTaskBuilder, read: R) -> Self {
        let payload_len = builder.payload_len;
        let (sender, receiver) = channel::bounded(builder.channel_capacity);
        let (stop_sender, stop_recv) = channel::bounded(1);
        self::spawn(async move {
            let mut read = read;

            loop {
                let r = self::wait(move || {
                    let mut payload = vec![0u8; payload_len];
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
                            println!("!!!");
                            let _ = sender.send(Ok(p)).await;
                            let _ = stop_sender.send(read).await;
                            break;
                        } else if sender.send(Ok(p)).await.is_err() {
                            let _ = stop_sender.send(read).await;
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = sender.send(Err(e)).await;
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

    /// Request the next payload.
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
pub struct ReadTaskError<R> {
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
/// Builds [`ReadTask`].
///
/// Use [`ReadTask::default`] to start.
#[derive(Debug, Clone)]
pub struct ReadTaskBuilder {
    payload_len: usize,
    channel_capacity: usize,
}
impl Default for ReadTaskBuilder {
    fn default() -> Self {
        ReadTaskBuilder {
            payload_len: 1024 * 1024,
            channel_capacity: 8,
        }
    }
}
impl ReadTaskBuilder {
    /// Set the byte count for each payload.
    ///
    /// Default is 1 mebibyte (`1024 * 1024`). Minimal value is 1.
    #[inline]
    pub fn payload_len(mut self, bytes: usize) -> Self {
        self.payload_len = bytes;
        self
    }

    /// Set the maximum numbers of payloads that be pre-read before the read task awaits
    /// for payloads to be removed from the channel.
    ///
    /// Default is 8. Minimal value is 0 for a [rendezvous] read.
    ///
    /// [`write`]: WriteTask::write
    /// [rendezvous]: channel::rendezvous
    #[inline]
    pub fn channel_capacity(mut self, capacity: usize) -> Self {
        self.channel_capacity = capacity;
        self
    }

    fn normalize(&mut self) {
        if self.payload_len < 1 {
            self.payload_len = 1;
        }
    }

    /// Start an idle [`ReadTask<R>`] that writes to `read`.
    #[inline]
    pub fn spawn<R>(mut self, read: R) -> ReadTask<R>
    where
        R: io::Read + Send + 'static,
    {
        self.normalize();
        ReadTask::spawn(self, read)
    }
}

/// Represents a running [`io::Write`] controller.
///
/// This task is recommended for buffered multi megabyte write operations, it spawns a
/// worker that uses [`wait`] to write received bytes that can be send using [`write`].
/// If you already have all the bytes you want to write in memory, just move then to a [`wait`]
/// and use the `std` sync file operations to write then, otherwise use this struct.
///
/// You can get the [`io::Write`] back by calling [`finish`], or in most error cases.
///
/// # Examples
///
/// The example writes 1 gibibyte of data generated in batches of 1 mebibyte, if the storage is slow a maximum
/// of 8 mebibytes only will exist in memory at a time.
///
/// ```no_run
/// # async fn compute_1mebibyte() -> Vec<u8> { vec![1; 1024 * 1024] }
/// # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
/// use zero_ui_core::task::{self, WriteTask};
///
/// let file = task::wait(|| std::fs::File::create("output.bin")).await?;
/// let w = WriteTask::default().spawn(file);
///
/// let mut total = 0usize;
/// let limit = 1024 * 1024 * 1024;
/// while total < limit {
///     let payload = compute_1mebibyte().await;
///     total += payload.len();
///
///     if w.write(payload).await.is_err() {
///         break;
///     }
/// }
///
/// let file = w.finish().await?;
/// task::wait(move || file.sync_all()).await?;
/// # Ok(()) }
/// ```
///
/// # Errors
///
/// Methods of this struct return [`WriteTaskError`], on the first error the task *shuts-down* and drops the wrapped [`io::Write`],
/// subsequent send attempts return the [`BrokenPipe`] error. To recover from errors keep track of the last successful write offset,
/// then on error reacquire write access and seek that offset before starting a new [`WriteTask`].
///
/// [`write`]: WriteTask::write
/// [`finish`]: WriteTask::finish
/// [`BrokenPipe`]: io::ErrorKind::BrokenPipe
pub struct WriteTask<W> {
    sender: channel::Sender<WriteTaskMsg>,
    finish: channel::Receiver<WriteTaskFinishMsg<W>>,
    state: Arc<WriteTaskState>,
}
impl WriteTask<()> {
    /// Start building a write task.
    ///
    /// # Examples
    ///
    /// Start a task that writes payloads and with a maximum of 8 pending writes in the channel:
    ///
    /// ```
    /// # use zero_ui_core::task::WriteTask;
    /// # fn demo(write: impl std::io::Write + Send + 'static) {
    /// let task = WriteTask::default().spawn(write);
    /// # }
    /// ```
    ///
    /// Start a task with custom configuration:
    ///
    /// ```
    /// # use zero_ui_core::task::WriteTask;
    /// # const FRAME_COUNT: usize = 3;
    /// # fn demo(write: impl std::io::Write + Send + 'static) {
    /// let task = WriteTask::default()
    ///     .channel_capacity(FRAME_COUNT.min(8))
    ///     .spawn(write);
    /// # }
    /// ```
    #[inline]
    pub fn default() -> WriteTaskBuilder {
        WriteTaskBuilder::default()
    }
}
impl<W> WriteTask<W>
where
    W: io::Write + Send + 'static,
{
    fn spawn(builder: WriteTaskBuilder, write: W) -> Self {
        let (sender, receiver) = channel::bounded(builder.channel_capacity);
        let (f_sender, f_receiver) = channel::rendezvous();
        let state = Arc::new(WriteTaskState {
            bytes_written: AtomicU64::new(0),
        });
        let t_state = Arc::clone(&state);
        self::spawn(async move {
            let mut write = write;
            let mut error = None;
            let mut error_payload = vec![];

            while let Ok(msg) = receiver.recv().await {
                match msg {
                    WriteTaskMsg::WriteAll(p) => {
                        let (w, p, r) = self::wait(move || {
                            let r = write.write_all(&p);
                            (write, p, r)
                        })
                        .await;
                        write = w;
                        match r {
                            Ok(_) => {
                                t_state.payload_written(p.len());
                            }
                            Err(e) => {
                                error = Some(e);
                                error_payload = p;
                                break;
                            }
                        }
                    }
                    WriteTaskMsg::Flush(rsp) => {
                        let (w, r) = self::wait(move || {
                            let r = write.flush();
                            (write, r)
                        })
                        .await;
                        write = w;
                        match r {
                            Ok(_) => {
                                if rsp.send(Ok(())).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                error = Some(e);
                                break;
                            }
                        }
                    }
                    WriteTaskMsg::Finish => {
                        let (w, r) = self::wait(move || {
                            let r = write.flush();
                            (write, r)
                        })
                        .await;

                        write = w;

                        match r {
                            Ok(_) => break,
                            Err(e) => error = Some(e),
                        }
                    }
                }
            }

            let _ = f_sender
                .send(WriteTaskFinishMsg {
                    write,
                    result: match error {
                        Some(e) => Err((error_payload, e)),
                        None => Ok(()),
                    },
                    receiver,
                })
                .await;
        });
        WriteTask {
            sender,
            state,
            finish: f_receiver,
        }
    }

    /// Send a bytes `payload` to the writer worker.
    ///
    /// Awaits if the channel is full, return `Ok` if the `payload` was send or the [`WriteTaskClosed`]
    /// error is the write worker has closed because of an IO error.
    ///
    /// In case of an error you must call [`finish`] to get the actual IO error.
    ///
    /// [`finish`]: WriteTask::finish
    pub async fn write(&self, payload: Vec<u8>) -> Result<(), WriteTaskClosed> {
        self.sender.send(WriteTaskMsg::WriteAll(payload)).await.map_err(|e| {
            if let WriteTaskMsg::WriteAll(payload) = e.0 {
                WriteTaskClosed { payload }
            } else {
                unreachable!()
            }
        })?;

        Ok(())
    }

    /// Awaits until all previous requested [`write`] are finished.
    ///
    /// [`write`]: Self::write
    pub async fn flush(&self) -> Result<(), WriteTaskClosed> {
        let (rsv, rcv) = channel::rendezvous();
        self.sender
            .send(WriteTaskMsg::Flush(rsv))
            .await
            .map_err(|_| WriteTaskClosed { payload: vec![] })?;

        rcv.recv().await.map_err(|_| WriteTaskClosed { payload: vec![] })?
    }

    /// Awaits until all previous requested [`write`] are finished, then closes the write worker.
    ///
    /// Returns a [`WriteTaskError`] in case the worker closed due to an IO error.
    ///
    /// [`write`]: Self::write
    pub async fn finish(self) -> Result<W, WriteTaskError<W>> {
        self.sender
            .send(WriteTaskMsg::Finish)
            .await
            .expect("`WriteTask::finish` already called");

        let msg = self.finish.recv().await.expect("`WriteTask::finish` already called");
        match msg.result {
            Ok(_) => Ok(msg.write),
            Err((payload, io_err)) => {
                let mut payloads = vec![payload];
                for msg in msg.receiver.drain() {
                    if let WriteTaskMsg::WriteAll(payload) = msg {
                        payloads.push(payload);
                    }
                }
                Err(WriteTaskError::new(msg.write, self.state.bytes_written(), payloads, io_err))
            }
        }
    }

    /// Number of bytes that where successfully written.
    #[inline]
    pub fn bytes_written(&self) -> u64 {
        self.state.bytes_written()
    }
}
struct WriteTaskState {
    bytes_written: AtomicU64,
}
impl WriteTaskState {
    fn bytes_written(&self) -> u64 {
        self.bytes_written.load(Ordering::Relaxed)
    }
    fn payload_written(&self, payload_len: usize) {
        self.bytes_written.fetch_add(payload_len as u64, Ordering::Relaxed);
    }
}

/// Builds [`WriteTask`].
///
/// Use [`WriteTask::default`] to start.
#[derive(Debug, Clone)]
pub struct WriteTaskBuilder {
    channel_capacity: usize,
}
impl Default for WriteTaskBuilder {
    fn default() -> Self {
        WriteTaskBuilder { channel_capacity: 8 }
    }
}
impl WriteTaskBuilder {
    /// Set the maximum numbers of payloads that can be pending before the [`write`]
    /// method is pending.
    ///
    /// Default is 8.
    ///
    /// [`write`]: WriteTask::write
    #[inline]
    pub fn channel_capacity(mut self, capacity: usize) -> Self {
        self.channel_capacity = capacity;
        self
    }

    /// Start an idle [`WriteTask<W>`] that writes to `write`.
    #[inline]
    pub fn spawn<W>(self, write: W) -> WriteTask<W>
    where
        W: io::Write + Send + 'static,
    {
        WriteTask::spawn(self, write)
    }
}

enum WriteTaskMsg {
    WriteAll(Vec<u8>),
    Flush(channel::Sender<Result<(), WriteTaskClosed>>),
    Finish,
}
struct WriteTaskFinishMsg<W> {
    write: W,
    result: Result<(), (Vec<u8>, io::Error)>,
    receiver: channel::Receiver<WriteTaskMsg>,
}

/// Error from [`WriteTask`].
///
/// The write task worker closes on the first IO error, the [`WriteTask`] send methods
/// return [`WriteTaskClosed`] when this happens and the [`WriteTask::finish`]
/// method returns this error that contains the actual IO error.
pub struct WriteTaskError<W> {
    /// The [`io::Write`].
    pub write: W,

    /// Number of bytes that where written before the error.
    ///
    /// Note that some bytes from the last payload where probably written too, but
    /// only confirmed written payloads are counted here.
    pub bytes_written: u64,

    /// The payloads that where not written.
    pub payloads: Vec<Vec<u8>>,
    /// The error.
    pub error: io::Error,
}
impl<W: io::Write> WriteTaskError<W> {
    fn new(write: W, bytes_written: u64, payloads: Vec<Vec<u8>>, error: io::Error) -> Self {
        Self {
            write,
            bytes_written,
            payloads,
            error,
        }
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

/// Error from [`WriteTask`].
///
/// This error is returned to indicate that the task worker has permanently stopped because
/// of an IO error. You can get the IO error by calling [`WriteTask::finish`].
pub struct WriteTaskClosed {
    /// Payload that could not be send.
    ///
    /// Is empty in case of a [`flush`] call.
    ///
    /// [`flush`]: WriteTask::flush
    payload: Vec<u8>,
}
impl fmt::Debug for WriteTaskClosed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WriteTaskDisconnected")
            .field("payload", &format!("<{} bytes>", self.payload.len()))
            .finish()
    }
}
impl fmt::Display for WriteTaskClosed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "write task worker has closed")
    }
}
impl std::error::Error for WriteTaskClosed {}

/// HTTP client.
///
/// This module is a thin wrapper around the [`isahc`] crate that just that just limits the API
/// surface to only `async` methods. You can convert from/into that [`isahc`] types and this one.
///
/// # Examples
///
/// Get some text:
///
/// ```
/// # use zero_ui_core::task;
/// # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
/// let text = task::http::get_text("https://httpbin.org/base64/SGVsbG8gV29ybGQ=").await?;
/// println!("{}!", text);
/// # Ok(()) }
/// ```
///
/// [`isahc`]: https://docs.rs/isahc
pub mod http {
    use std::convert::TryFrom;
    use std::sync::Arc;
    use std::{fmt, mem};

    pub use isahc::error::Error;
    pub use isahc::http::request::Builder as RequestBuilder;
    pub use isahc::http::{header, Request, StatusCode, Uri};

    use isahc::AsyncReadResponseExt;
    use parking_lot::{const_mutex, Mutex};

    /// Marker trait for types that try-to-convert to [`Uri`].
    ///
    /// All types `T` that match `Uri: TryFrom<T>, <Uri as TryFrom<T>>::Error: Into<isahc::http::Error>` implement this trait.
    pub trait TryUri {
        /// Tries to convert `self` into [`Uri`].
        fn try_into(self) -> Result<Uri, Error>;
    }
    impl<U> TryUri for U
    where
        isahc::http::Uri: TryFrom<U>,
        <isahc::http::Uri as TryFrom<U>>::Error: Into<isahc::http::Error>,
    {
        fn try_into(self) -> Result<Uri, Error> {
            Uri::try_from(self).map_err(|e| e.into().into())
        }
    }

    /// HTTP response.
    pub struct Response(isahc::Response<isahc::AsyncBody>);
    impl Response {
        /// Returns the [`StatusCode`].
        #[inline]
        pub fn status(&self) -> StatusCode {
            self.0.status()
        }

        /// Returns a reference to the associated header field map.
        #[inline]
        pub fn headers(&self) -> &header::HeaderMap<header::HeaderValue> {
            self.0.headers()
        }

        /// Read the response body as a string.
        pub async fn text(&mut self) -> std::io::Result<String> {
            self.0.text().await
        }

        /// Read the response body as raw bytes.
        ///
        /// Use [`DownloadTask`] to get larger files.
        pub async fn bytes(&mut self) -> std::io::Result<Vec<u8>> {
            let cap = self.0.body_mut().len().unwrap_or(1024);
            let mut bytes = Vec::with_capacity(cap as usize);
            self.0.copy_to(&mut bytes).await?;
            Ok(bytes)
        }

        /// Deserialize the response body as JSON.
        pub async fn json<O>(&mut self) -> Result<O, serde_json::Error>
        where
            O: serde::de::DeserializeOwned + std::marker::Unpin,
        {
            self.0.json().await
        }
    }
    impl From<Response> for isahc::Response<isahc::AsyncBody> {
        fn from(r: Response) -> Self {
            r.0
        }
    }

    /// HTTP request body.
    pub struct Body(isahc::AsyncBody);
    impl From<Body> for isahc::AsyncBody {
        fn from(r: Body) -> Self {
            r.0
        }
    }

    /// Send a GET request to the `uri`.
    #[inline]
    pub async fn get(uri: impl TryUri) -> Result<Response, Error> {
        isahc_client().get_async(uri.try_into()?).await.map(Response)
    }

    /// Send a GET request to the `uri` and read the response as a string.
    pub async fn get_text(uri: impl TryUri) -> Result<String, Error> {
        let mut r = get(uri).await?;
        let r = r.text().await?;
        Ok(r)
    }

    /// Send a GET request to the `uri` and read the response as raw bytes.
    pub async fn get_bytes(uri: impl TryUri) -> Result<Vec<u8>, Error> {
        let mut r = get(uri).await?;
        let r = r.bytes().await?;
        Ok(r)
    }

    /// Send a GET request to the `uri` and de-serializes the response.
    pub async fn get_json<O>(uri: impl TryUri) -> Result<O, Box<dyn std::error::Error>>
    where
        O: serde::de::DeserializeOwned + std::marker::Unpin,
    {
        let mut r = get(uri).await?;
        let r = r.json::<O>().await?;
        Ok(r)
    }

    /// Send a HEAD request to the `uri`.
    #[inline]
    pub async fn head(uri: impl TryUri) -> Result<Response, Error> {
        isahc_client().head_async(uri.try_into()?).await.map(Response)
    }

    /// Send a PUT request to the `uri` with a given request body.
    #[inline]
    pub async fn put(uri: impl TryUri, body: impl Into<Body>) -> Result<Response, Error> {
        isahc_client().put_async(uri.try_into()?, body.into().0).await.map(Response)
    }

    /// Send a POST request to the `uri` with a given request body.
    #[inline]
    pub async fn post(uri: impl TryUri, body: impl Into<Body>) -> Result<Response, Error> {
        isahc_client().post_async(uri.try_into()?, body.into().0).await.map(Response)
    }

    /// Send a DELETE request to the `uri`.
    #[inline]
    pub async fn delete(uri: impl TryUri) -> Result<Response, Error> {
        isahc_client().delete_async(uri.try_into()?).await.map(Response)
    }

    /// Send a custom [`Request`].
    #[inline]
    pub async fn send<B: Into<Body>>(request: impl Into<Request<B>>) -> Result<Response, Error> {
        isahc_client().send_async(request.into().map(|b| b.into().0)).await.map(Response)
    }

    /// The [`isahc`] client used by the functions in this module and Zero-Ui.
    ///
    /// You can replace the default client at the start of the process using [`set_isahc_client_init`].
    ///
    /// [`isahc`]: https://docs.rs/isahc
    pub fn isahc_client() -> &'static isahc::HttpClient {
        use crate::units::*;
        use isahc::config::{Configurable, RedirectPolicy};
        use once_cell::sync::Lazy;

        static SHARED: Lazy<isahc::HttpClient> = Lazy::new(|| {
            let ci = mem::replace(&mut *CLIENT_INIT.lock(), ClientInit::Inited);
            if let ClientInit::Set(init) = ci {
                init()
            } else {
                // browser defaults
                isahc::HttpClient::builder()
                    .cookies()
                    .redirect_policy(RedirectPolicy::Limit(20))
                    .connect_timeout(90.secs())
                    .auto_referer()
                    .build()
                    .unwrap()
            }
        });
        &SHARED
    }

    static CLIENT_INIT: Mutex<ClientInit> = const_mutex(ClientInit::None);

    enum ClientInit {
        None,
        Set(Box<dyn FnOnce() -> isahc::HttpClient + Send>),
        Inited,
    }

    /// Set a custom initialization function for the [`isahc_client`].
    ///
    /// The [`isahc_client`] is used by all Zero-Ui functions and is initialized on the first usage,
    /// you can use this function before any HTTP operation to replace the [`isahc`] client
    /// used by Zero-Ui.
    ///
    /// Returns an error if the [`isahc_client`] was already initialized.
    ///
    /// [`isahc`]: https://docs.rs/isahc
    pub fn set_isahc_client_init<I>(init: I) -> Result<(), I>
    where
        I: FnOnce() -> isahc::HttpClient + Send + 'static,
    {
        let mut ci = CLIENT_INIT.lock();
        if let ClientInit::Inited = &*ci {
            Err(init)
        } else {
            *ci = ClientInit::Set(Box::new(init));
            Ok(())
        }
    }

    /// Represents a running large file download.
    pub struct DownloadTask {
        payload_len: usize,
    }
    impl DownloadTask {
        /// Start building a download task using the [default client].
        ///
        /// [default client]: isahc_client
        #[inline]
        pub fn default() -> DownloadTaskBuilder {
            DownloadTaskBuilder::default()
        }

        /// Start building a download task with a custom [`isahc`] client.
        ///
        /// [`isahc`]: https://docs.rs/isahc
        #[inline]
        pub fn with_client(client: isahc::HttpClient) -> DownloadTaskBuilder {
            DownloadTaskBuilder::new(client)
        }

        fn spawn(builder: DownloadTaskBuilder, uri: Result<Uri, Error>) -> Self {
            todo!("{:?}, {:?}", builder, uri)
        }

        /// Maximum number of bytes per payload.
        #[inline]
        pub fn payload_len(&self) -> usize {
            self.payload_len
        }

        /// Pause the download.
        ///
        /// This signals the task stop downloading even if there is space in the cache, if you
        /// set `cancel_partial_payloads` any partially downloaded payload is dropped.
        ///
        /// Note that the task naturally *pauses* when the cache limit is reached if you stop calling [`download`],
        /// in this case you do not need to call `pause` or [`resume`].
        ///
        /// [`download`]: Self::download
        /// [`resume`]: Self::resume
        pub async fn pause(&self, cancel_partial_payloads: bool) {
            todo!("{}", cancel_partial_payloads)
        }

        /// Resume the download, if the connection was lost attempts to reconnect.
        pub async fn resume(&self) {
            todo!()
        }

        /// Stops the download but retains the disk cache and returns a [`FrozenDownloadTask`]
        /// that can be serialized/desterilized and resumed.
        pub async fn freeze(self) -> FrozenDownloadTask {
            todo!()
        }

        /// Stops the task, cancels download if it is not finished, clears the disk cache if any was used.
        pub async fn stop(self) {
            todo!()
        }

        /// Receive the next downloaded payload.
        ///
        /// The payloads are sequential, even if parallel downloads are enabled.
        pub async fn download(&self) -> Result<Vec<u8>, DownloadTaskError> {
            todo!()
        }
    }

    /// Builds [`DownloadTask`].
    ///
    /// Use [`DownloadTask::default`] or [`DownloadTask::with_client`] to start.
    #[derive(Clone)]
    pub struct DownloadTaskBuilder {
        client: isahc::HttpClient,
        payload_len: usize,
        channel_capacity: usize,
        parallel_count: usize,
        disk_cache_capacity: usize,
        max_speed: usize,
        request_config: Arc<dyn Fn(RequestBuilder) -> RequestBuilder + Send>,
    }
    impl fmt::Debug for DownloadTaskBuilder {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("DownloadTaskBuilder")
                .field("client", &self.client)
                .field("payload_len", &self.payload_len)
                .field("channel_capacity", &self.channel_capacity)
                .field("parallel_count", &self.parallel_count)
                .field("disk_cache_capacity", &self.disk_cache_capacity)
                .field("max_speed", &self.max_speed)
                .finish_non_exhaustive()
        }
    }
    impl Default for DownloadTaskBuilder {
        fn default() -> Self {
            Self::new(isahc_client().clone())
        }
    }
    impl DownloadTaskBuilder {
        fn new(client: isahc::HttpClient) -> Self {
            DownloadTaskBuilder {
                client,
                payload_len: 1024 * 1024,
                channel_capacity: 8,
                parallel_count: 1,
                disk_cache_capacity: 0,
                max_speed: 0,
                request_config: Arc::new(|b| b),
            }
        }

        /// Set the number of bytes in each payload.
        ///
        /// Default is one mebibyte (`1024 * 1024`).
        pub fn payload_len(mut self, len: usize) -> Self {
            self.payload_len = len;
            self
        }

        /// Set the number of downloaded payloads that can wait in memory. If this
        /// capacity is reached the disk cache is used if it is set, otherwise the download *pauses*
        /// internally until a payload is taken from the channel.
        ///
        /// Default is `8`.
        pub fn channel_capacity(mut self, capacity: usize) -> Self {
            self.channel_capacity = capacity;
            self
        }

        /// Set the number of payloads that can be downloaded in parallel, setting
        /// this to more then 1 can speedup the overall download time, if you are
        /// just downloading to a file and depending on the server.
        ///
        /// Default is `1`.
        pub fn parallel_count(mut self, count: usize) -> Self {
            self.parallel_count = count;
            self
        }

        /// Set the number of payloads that can be cached in disk. If this capacity is
        /// reached the download *pauses* and *resumes* internally.
        ///
        /// Default is `0`.
        pub fn disk_cache_capacity(mut self, payload_count: usize) -> Self {
            self.disk_cache_capacity = payload_count;
            self
        }

        /// Set the maximum download speed, in bytes per second.
        ///
        /// Default is `usize::MAX` to indicate no limit. Minimal value is `57344` (56 kibibytes/s).
        #[inline]
        pub fn max_speed(mut self, bytes_per_sec: usize) -> Self {
            self.max_speed = bytes_per_sec;
            self
        }

        /// Set a closure that configures requests generated by the download task.
        ///
        /// # Examples
        ///
        /// Set a custom header:
        ///
        /// ```
        /// # use zero_ui_core::task::http::*;
        /// # fn demo(builder: DownloadTaskBuilder) -> DownloadTaskBuilder {
        /// builder.request_config(|c| c.header("X-Foo-For", "Bar"))
        /// # }
        /// ```
        ///
        /// The closure can be called many times, specially when parallel downloads are enabled.
        /// Note that you can break the download using this, make sure that you are not changing
        /// configuration set by the [`DownloadTask`] code before use.
        #[inline]
        pub fn request_config<F>(mut self, config: F) -> Self
        where
            F: Fn(RequestBuilder) -> RequestBuilder + Send + 'static,
        {
            self.request_config = Arc::new(config);
            self
        }

        fn normalize(&mut self) {
            if self.parallel_count == 0 {
                self.parallel_count = 1;
            }
            if self.max_speed < 57344 {
                self.max_speed = 57344;
            }
        }

        /// Start downloading the `uri`.
        pub fn spawn(mut self, uri: impl TryUri) -> DownloadTask {
            self.normalize();
            DownloadTask::spawn(self, uri.try_into())
        }
    }

    /// A [`DownloadTask`] that can be *reanimated* in another instance of the app.
    pub struct FrozenDownloadTask {}
    impl FrozenDownloadTask {
        /// Attempt to continue the download task.
        pub async fn resume(self) -> Result<DownloadTask, DownloadTaskError> {
            todo!()
        }
    }

    /// An error in [`DownloadTask`] or [`FrozenDownloadTask`]
    pub struct DownloadTaskError {}

    /// Represents a running large file upload.
    pub struct UploadTask {}
    impl UploadTask {
        /// Start building an upload task using the [default client].
        ///
        /// [default client]: isahc_client
        #[inline]
        pub fn default() -> UploadTaskBuilder {
            UploadTaskBuilder::default()
        }

        /// Start building an upload task with a custom [`isahc`] client.
        ///
        /// [`isahc`]: https://docs.rs/isahc
        #[inline]
        pub fn with_client(client: isahc::HttpClient) -> UploadTaskBuilder {
            UploadTaskBuilder::new(client)
        }

        fn spawn(builder: UploadTaskBuilder, uri: Result<Uri, Error>) -> Self {
            todo!("{:?}, {:?}", builder, uri)
        }

        /// Send the next payload to upload.
        ///
        /// You can *pause* upload simply by not calling this method, if the connection was lost the task
        /// will attempt to retrieve it before continuing.
        pub async fn upload(&self, payload: Vec<u8>) -> Result<(), UploadTaskError> {
            todo!("{:?}", payload)
        }
    }

    /// Build a [`UploadTask`]
    pub struct UploadTaskBuilder {
        client: isahc::HttpClient,
        channel_capacity: usize,
        max_speed: usize,
        request_config: Arc<dyn Fn(RequestBuilder) -> RequestBuilder + Send>,
    }
    impl fmt::Debug for UploadTaskBuilder {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("UploadTaskBuilder")
                .field("client", &self.client)
                .field("channel_capacity", &self.channel_capacity)
                .field("max_speed", &self.max_speed)
                .finish_non_exhaustive()
        }
    }
    impl Default for UploadTaskBuilder {
        fn default() -> Self {
            Self::new(isahc_client().clone())
        }
    }
    impl UploadTaskBuilder {
        fn new(client: isahc::HttpClient) -> Self {
            UploadTaskBuilder {
                client,
                channel_capacity: 8,
                max_speed: 0,
                request_config: Arc::new(|b| b),
            }
        }

        /// Set the number of pending upload payloads that can wait in memory. If this
        /// capacity is reached the the [`upload`] method is pending until a payload is uploaded.
        ///
        /// Default is `8`.
        ///
        /// [`upload`]: UploadTask::upload
        pub fn channel_capacity(mut self, capacity: usize) -> Self {
            self.channel_capacity = capacity;
            self
        }

        /// Set the maximum upload speed, in bytes per second.
        ///
        /// Default is `usize::MAX` to indicate no limit. Minimal value is `57344` (56 kibibytes/s).
        #[inline]
        pub fn max_speed(mut self, bytes_per_sec: usize) -> Self {
            self.max_speed = bytes_per_sec;
            self
        }

        /// Set a closure that configures requests generated by the upload task.
        ///
        /// # Examples
        ///
        /// Set a custom header:
        ///
        /// ```
        /// # use zero_ui_core::task::http::*;
        /// # fn demo(builder: UploadTaskBuilder) -> UploadTaskBuilder {
        /// builder.request_config(|c| c.header("X-Foo-For", "Bar"))
        /// # }
        /// ```
        ///
        /// The closure can be called multiple times due to the task internal error recovery.
        ///
        /// Note that you can break the upload using this, make sure that you are not changing
        /// configuration set by the [`DownloadTask`] code before use.
        #[inline]
        pub fn request_config<F>(mut self, config: F) -> Self
        where
            F: Fn(RequestBuilder) -> RequestBuilder + Send + 'static,
        {
            self.request_config = Arc::new(config);
            self
        }

        fn normalize(&mut self) {
            if self.max_speed < 57344 {
                self.max_speed = 57344;
            }
        }

        /// Start an idle upload task to the `uri`.
        pub fn spawn(mut self, uri: impl TryUri) -> UploadTask {
            self.normalize();
            UploadTask::spawn(self, uri.try_into())
        }
    }

    /// An error in [`UploadTask`].
    pub struct UploadTaskError {}
}

#[cfg(test)]
pub mod tests {
    use crate::units::TimeUnits;
    use std::sync::atomic::AtomicBool;

    use super::*;

    fn async_test<F>(test: F) -> F::Output
    where
        F: Future,
    {
        block_on(with_timeout(test, Duration::from_secs(1))).unwrap()
    }

    #[test]
    pub fn any_one() {
        let r = async_test(async { any!(async { true }).await });

        assert!(r);
    }

    #[test]
    pub fn any_five() {
        let one_s = Duration::from_secs(1);
        let r = async_test(async {
            any!(
                async {
                    timeout(one_s).await;
                    1
                },
                async {
                    timeout(one_s).await;
                    2
                },
                async {
                    timeout(one_s).await;
                    3
                },
                async {
                    timeout(one_s).await;
                    4
                },
                async { 5 },
            )
            .await
        });

        assert_eq!(5, r);
    }

    #[test]
    pub fn any_nine() {
        let one_s = Duration::from_secs(1);
        let r = async_test(async {
            any!(
                async {
                    timeout(one_s).await;
                    1
                },
                async {
                    timeout(one_s).await;
                    2
                },
                async {
                    timeout(one_s).await;
                    3
                },
                async {
                    timeout(one_s).await;
                    4
                },
                async {
                    timeout(one_s).await;
                    5
                },
                async {
                    timeout(one_s).await;
                    6
                },
                async {
                    timeout(one_s).await;
                    7
                },
                async {
                    timeout(one_s).await;
                    8
                },
                async { 9 },
            )
            .await
        });

        assert_eq!(9, r);
    }

    #[test]
    pub fn all_one() {
        let r = async_test(async { all!(async { true }).await });

        assert!(r);
    }

    #[test]
    pub fn all_five() {
        let r = async_test(async {
            all!(
                async { 'a' },
                async {
                    yield_one().await;
                    'b'
                },
                async { 'c' },
                async {
                    yield_one().await;
                    'd'
                },
                async { 'e' },
            )
            .await
        });

        assert_eq!('a', r.0);
        assert_eq!('b', r.1);
        assert_eq!('c', r.2);
        assert_eq!('d', r.3);
        assert_eq!('e', r.4);
    }

    #[test]
    pub fn all_nine() {
        let r = async_test(async {
            all!(
                async { 'a' },
                async {
                    yield_one().await;
                    'b'
                },
                async { 'c' },
                async {
                    yield_one().await;
                    'd'
                },
                async { 'e' },
                async {
                    yield_one().await;
                    'f'
                },
                async { 'g' },
                async {
                    yield_one().await;
                    'h'
                },
                async { 'i' },
                async {
                    yield_one().await;
                    'j'
                },
            )
            .await
        });

        assert_eq!('a', r.0);
        assert_eq!('b', r.1);
        assert_eq!('c', r.2);
        assert_eq!('d', r.3);
        assert_eq!('e', r.4);
        assert_eq!('f', r.5);
        assert_eq!('g', r.6);
        assert_eq!('h', r.7);
        assert_eq!('i', r.8);
        assert_eq!('j', r.9);
    }

    #[test]
    pub fn read_task() {
        async_test(async {
            let task = ReadTask::default().payload_len(1).spawn(TestRead::default());

            timeout(10.ms()).await;

            let payload = task.read().await.unwrap();
            assert_eq!(task.payload_len(), payload.len());

            task.read().await.unwrap();

            let expected_read_calls = 8 + 2; // default capacity + 2 read calls.
            let expected_bytes_read = task.payload_len() * expected_read_calls;

            let read = task.stop().await.unwrap();

            assert_eq!(expected_read_calls, read.read_calls);
            assert_eq!(expected_bytes_read, read.bytes_read);
        })
    }

    #[test]
    pub fn read_task_error() {
        async_test(async {
            let read = TestRead::default();
            let flag = read.cause_error.clone();

            let task = ReadTask::default().payload_len(1).spawn(TestRead::default());

            timeout(10.ms()).await;

            flag.set();

            loop {
                match task.read().await {
                    Ok(p) => assert_eq!(p.len(), 1),
                    Err(e) => {
                        assert_eq!("test error", e.error.to_string());
                        assert!(!e.is_disconnected());

                        let e = task.read().await.unwrap_err();
                        assert!(e.is_disconnected());
                        break;
                    }
                }
            }

            let e = task.stop().await.unwrap_err();
            assert!(e.is_disconnected())
        })
    }

    #[derive(Default, Debug)]
    pub struct TestRead {
        bytes_read: usize,
        read_calls: usize,
        cause_stop: Arc<Flag>,
        cause_error: Arc<Flag>,
        cause_panic: Arc<Flag>,
    }
    impl io::Read for TestRead {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.read_calls += 1;
            if self.cause_stop.is_set() {
                Ok(0)
            } else if self.cause_error.is_set() {
                Err(io::Error::new(io::ErrorKind::Other, "test error"))
            } else if self.cause_panic.is_set() {
                panic!("test panic");
            } else {
                let bytes = (self.bytes_read..self.bytes_read + buf.len()).map(|u| u as u8);
                for (byte, i) in bytes.zip(buf.iter_mut()) {
                    *i = byte;
                }
                self.bytes_read += buf.len();
                Ok(buf.len())
            }
        }
    }

    #[derive(Default, Debug)]
    pub struct Flag(AtomicBool);
    impl Flag {
        pub fn set(&self) {
            self.0.store(true, Ordering::Relaxed);
        }

        pub fn is_set(&self) -> bool {
            self.0.load(Ordering::Relaxed)
        }
    }
}
