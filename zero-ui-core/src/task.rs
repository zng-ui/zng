//! Parallel async tasks and async task runners.
//!
//! Use the [`run`], [`respond`] or [`spawn`] to run parallel tasks, use [`wait`], [`io`] and [`fs`] to unblock
//! IO operations, use [`http`] for async HTTP, and use [`ui`] to create async properties.
//!
//! This module also re-exports the [`rayon`] crate for convenience.
//!
//! # Examples
//!
//! ```
//! # use zero_ui_core::{widget, UiNode, var::{var, IntoVar}, async_hn, event_property, property,
//! # gesture::{CLICK_EVENT, ClickArgs}, task::{self, rayon::prelude::*}};
//! # #[widget($crate::button)]
//! # pub mod button { }
//! # event_property! { pub fn click { event: CLICK_EVENT, args: ClickArgs, } }
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
//!         println!("sum of squares: {sum}");
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
//! The example demonstrates three different ***tasks***, the first is a [`ui::WidgetTask`] in the `async_hn` handler,
//! this task is *async* but not *parallel*, meaning that it will execute in more then one app update, but it will only execute in the app
//! main thread. This is good for coordinating UI state, like setting variables, but is not good if you want to do CPU intensive work.
//!
//! To keep the app responsive we move the computation work inside a [`run`] task, this task is *async* and *parallel*,
//! meaning it can `.await` and will execute in parallel threads. It runs in a [`rayon`] thread-pool so you can
//! easily make the task multi-threaded and when it is done it sends the result back to the widget task that is awaiting for it. We
//! resolved the responsiveness problem, but there is one extra problem to solve, how to not block one of the worker threads waiting IO.
//!
//! We want to keep the [`run`] threads either doing work or available for other tasks, but reading a file is just waiting
//! for a potentially slow external operation, so if we call [`std::fs::read_to_string`] directly we can potentially remove one of
//! the worker threads from play, reducing the overall tasks performance. To avoid this we move the IO operation inside a [`wait`]
//! task, this task is not *async* but it is *parallel*, meaning if does not block but it runs a blocking operation. It runs inside
//! a [`blocking`] thread-pool, that is optimized for waiting.
//!
//! # Async IO
//!
//! You can use [`wait`], [`io`] and [`fs`] to do async IO, and Zero-UI uses this API for internal async IO, they are just a selection
//! of external async crates re-exported for convenience.
//!
//! The [`io`] module just re-exports the [`futures-lite::io`] traits and types, adding only progress tracking. The
//! [`fs`] module is the [`async-fs`] crate. Most of the IO async operations are implemented using extensions traits
//! so we recommend blob importing [`io`] to start implementing async IO.
//!
//! ```
//! use zero_ui_core::task::{io::*, fs, rayon::prelude::*};
//!
//! async fn read_numbers() -> Vec<usize> {
//!     let mut file = fs::File::open("numbers.txt").await.unwrap();
//!     let mut raw = String::new();
//!     file.read_to_string(&mut raw).await.unwrap();
//!     raw.par_split(',').map(|s| s.trim().parse::<usize>().unwrap()).collect()
//! }
//! ```
//!
//! All the `std::fs` synchronous operations have an async counterpart in [`fs`]. For simpler one shot
//! operation it is recommended to just use `std::fs` inside [`wait`], the async [`fs`] types are not async at
//! the OS level, they only offload operations inside the same thread-pool used by [`wait`].
//!
//! # HTTP Client
//!
//! You can use [`http`] to implement asynchronous HTTP requests. Zero-Ui also uses the [`http`] module for
//! implementing operations such as loading an image from a given URL, the module is a thin wrapper around the [`isahc`] crate.
//!
//! ```
//! # use zero_ui_core::{*, var::*, handler::*, text::*, gesture::*};
//! # #[widget($crate::button)]
//! # pub mod button { }
//! # event_property! { pub fn click { event: CLICK_EVENT, args: ClickArgs, } }
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
//!             Err(e) => msg.set(&ctx, formatx!("error: {e}")),
//!         }
//!
//!         enabled.set(&ctx, true);
//!     });
//! }
//! # ; }
//! ```
//!
//! For other protocols or alternative HTTP clients you can use [external crates](#async-crates-integration).
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
//! to call its async functions from inside the tokio runtime. After you create a future from inside the runtime you can `.await` then
//! in any thread, so we recommend manually starting its runtime in a thread and then using the `tokio::runtime::Handle` to start
//! futures in the runtime.
//!
//! [`isahc`]: https://docs.rs/isahc
//! [`AppExtension`]: crate::app::AppExtension
//! [`blocking`]: https://docs.rs/blocking
//! [`futures`]: https://docs.rs/futures
//! [`async-std`]: https://docs.rs/async-std
//! [`smol`]: https://docs.rs/smol
//! [`tokio`]: https://docs.rs/tokio
//! [`futures-lite::io`]: https://docs.rs/futures-lite/*/futures_lite/io/index.html
//! [`async-fs`]: https://docs.rs/async-fs

use std::{
    fmt,
    future::Future,
    mem, panic,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::Poll,
};

use parking_lot::Mutex;

use crate::{
    crate_util::{panic_str, PanicResult},
    units::Deadline,
    var::{response_channel, response_var, ResponseVar, Var, VarValue, WithVars},
};

#[doc(no_inline)]
pub use rayon;

#[doc(no_inline)]
pub use async_fs as fs;

pub use crate::handler::async_clone_move;

pub mod channel;
pub mod io;

pub mod http;
pub mod ui;

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
///         println!("sum of squares 0..1000: {result}");   
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
///
/// # Panic Handling
///
/// If the `task` panics the panic message is logged as an error, the panic is otherwise ignored.
///
/// # Unwind Safety
///
/// This function disables the [unwind safety validation], meaning that in case of a panic shared
/// data can end-up in an invalid, but still memory safe, state. If you are worried about that only use
/// poisoning mutexes or atomics to mutate shared data or use [`run_catch`] to detect a panic or [`run`]
/// to propagate a panic.
///
/// [unwind safety validation]: std::panic::UnwindSafe
/// [`Waker`]: std::task::Waker
pub fn spawn<F>(task: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    type Fut = Pin<Box<dyn Future<Output = ()> + Send>>;

    // A future that is its own waker that polls inside the rayon primary thread-pool.
    struct RayonTask(Mutex<Option<Fut>>);
    impl RayonTask {
        fn poll(self: Arc<RayonTask>) {
            rayon::spawn(move || {
                // this `Option<Fut>` dance is used to avoid a `poll` after `Ready` or panic.
                let mut task = self.0.lock();
                if let Some(mut t) = task.take() {
                    let waker = self.clone().into();
                    let mut cx = std::task::Context::from_waker(&waker);

                    let r = panic::catch_unwind(panic::AssertUnwindSafe(move || {
                        if t.as_mut().poll(&mut cx).is_pending() {
                            *task = Some(t);
                        }
                    }));
                    if let Err(p) = r {
                        tracing::error!("panic in `task::spawn`: {}", panic_str(&p));
                    }
                }
            })
        }
    }
    impl std::task::Wake for RayonTask {
        fn wake(self: Arc<Self>) {
            self.poll()
        }
    }

    Arc::new(RayonTask(Mutex::new(Some(Box::pin(task))))).poll()
}

/// Spawn a parallel async task that can also be `.await` for the task result.
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
///
/// # Cancellation
///
/// The task starts running immediately, awaiting the returned future merely awaits for a message from the worker threads and
/// that means the `task` future is not owned by the returned future. Usually to *cancel* a future you only need to drop it,
/// in this task dropping the returned future will only drop the `task` once it reaches a `.await` point and detects that the
/// result channel is disconnected.
///
/// If you want to deterministically known that the `task` was cancelled use a cancellation signal.
///
/// # Panic Propagation
///
/// If the `task` panics the panic is re-raised in the awaiting thread using [`resume_unwind`]. You
/// can use [`run_catch`] to get the panic as an error instead.
///
/// [`resume_unwind`]: panic::resume_unwind
/// [`Waker`]: std::task::Waker
pub async fn run<R, T>(task: T) -> R
where
    R: Send + 'static,
    T: Future<Output = R> + Send + 'static,
{
    match run_catch(task).await {
        Ok(r) => r,
        Err(p) => panic::resume_unwind(p),
    }
}

/// Like [`run`] but catches panics.
///
/// This task works the same and has the same utility as [`run`], except if returns panic messages
/// as an error instead of propagating the panic.
///
/// # Unwind Safety
///
/// This function disables the [unwind safety validation], meaning that in case of a panic shared
/// data can end-up in an invalid, but still memory safe, state. If you are worried about that only use
/// poisoning mutexes or atomics to mutate shared data or discard all shared data used in the `task`
/// if this function returns an error.
///
/// [unwind safety validation]: std::panic::UnwindSafe
pub async fn run_catch<R, T>(task: T) -> PanicResult<R>
where
    R: Send + 'static,
    T: Future<Output = R> + Send + 'static,
{
    type Fut<R> = Pin<Box<dyn Future<Output = R> + Send>>;

    // A future that is its own waker that polls inside the rayon primary thread-pool.
    struct RayonCatchTask<R>(Mutex<Option<Fut<R>>>, flume::Sender<PanicResult<R>>);
    impl<R: Send + 'static> RayonCatchTask<R> {
        fn poll(self: Arc<Self>) {
            let sender = self.1.clone();
            if sender.is_disconnected() {
                return; // cancel.
            }
            rayon::spawn(move || {
                // this `Option<Fut>` dance is used to avoid a `poll` after `Ready` or panic.
                let mut task = self.0.lock();
                if let Some(mut t) = task.take() {
                    let waker = self.clone().into();
                    let mut cx = std::task::Context::from_waker(&waker);

                    let r = panic::catch_unwind(panic::AssertUnwindSafe(|| t.as_mut().poll(&mut cx)));

                    match r {
                        Ok(Poll::Ready(r)) => {
                            drop(task);
                            let _ = sender.send(Ok(r));
                        }
                        Ok(Poll::Pending) => {
                            *task = Some(t);
                        }
                        Err(p) => {
                            drop(task);
                            let _ = sender.send(Err(p));
                        }
                    }
                }
            })
        }
    }
    impl<R: Send + 'static> std::task::Wake for RayonCatchTask<R> {
        fn wake(self: Arc<Self>) {
            self.poll()
        }
    }

    let (sender, receiver) = channel::bounded(1);

    Arc::new(RayonCatchTask(Mutex::new(Some(Box::pin(task))), sender.into())).poll();

    receiver.recv().await.unwrap()
}

/// Spawn a parallel async task that will send its result to a [`ResponseVar`].
///
/// The [`run`] documentation explains how `task` is *parallel* and *async*. The `task` starts executing immediately.
///
/// This is just a helper method that creates a [`response_channel`] and awaits for the `task` in a [`spawn`] runner.
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
///         println!("sum of squares: {result}");   
///     }
/// }
/// # }
/// ```
///
/// The example `.await` for some numbers and then uses a parallel iterator to compute a result. The result is send to
/// `sum_response` that is a [`ResponseVar<R>`].
///
/// # Cancellation
///
/// Dropping the [`ResponseVar<R>`] does not cancel the `task`, it will still run to completion.
///
/// # Panic Handling
///
/// If the `task` panics the panic is logged but otherwise ignored and the variable never responds. See
/// [`spawn`] for more information about the panic handling of this function.
///
/// # Send
///
/// The response value must be [`Send`], if the `!Send` part of the result is trivial you can use
/// [`respond_ctor`] to workaround this constrain by sending a *constructor* closure to run in the UI thread.
///
/// [`resume_unwind`]: panic::resume_unwind
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

/// Like [`respond`] but sends a response constructor closure.
///
/// The response constructor is a closure that is the result of `task`. It is send to, and evaluated in the UI thread,
/// this removes the [`Send`] constrain from the response value for cases where the expensive values of the response
/// are [`Send`], just the final response that is not.
///
/// # Examples
///
/// Construct a `!Send` struct in the UI thread using a *constructor* closure:
///
/// ```
/// # use std::rc::Rc;
/// # use zero_ui_core::task;
/// #[derive(Clone, Debug)]
/// pub struct NotSend {
///     pub send_value: bool,
///     not_send_part: Rc<()>
/// }
///
/// # fn demo(vars: &zero_ui_core::var::Vars) { let _ =
/// task::respond_ctor(vars, async {
///     let send_value = task::wait(|| true).await;
///     move || NotSend { send_value, not_send_part: Rc::new(()) }
/// })
/// # ; }
/// ```
pub fn respond_ctor<Vw: WithVars, R, C, F>(vars: &Vw, task: F) -> ResponseVar<R>
where
    R: VarValue + 'static,
    C: FnOnce() -> R + Send + 'static,
    F: Future<Output = C> + Send + 'static,
{
    let (responder, response) = response_var();
    let modify_sender = responder.modify_sender(vars);

    spawn(async move {
        let ctor = task.await;
        let _ = modify_sender.send(move |v| *v.get_mut() = crate::var::types::Response::Done(ctor()));
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
/// # Async Read/Write
///
/// For [`std::io::Read`] and [`std::io::Write`] operations you can also use [`io`] and [`fs`] alternatives when you don't
/// have or want the full file in memory or when you want to apply multiple operations to the file.
///
/// # Panic Propagation
///
/// If the `task` panics the panic is re-raised in the awaiting thread using [`resume_unwind`]. You
/// can use [`wait_catch`] to get the panic as an error instead.
///
/// [`blocking`]: https://docs.rs/blocking
/// [`resume_unwind`]: panic::resume_unwind
pub async fn wait<T, F>(task: F) -> T
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    match wait_catch(task).await {
        Ok(r) => r,
        Err(p) => panic::resume_unwind(p),
    }
}

/// Like [`wait`] but catches panics.
///
/// This task works the same and has the same utility as [`wait`], except if returns panic messages
/// as an error instead of propagating the panic.
///
/// # Unwind Safety
///
/// This function disables the [unwind safety validation], meaning that in case of a panic shared
/// data can end-up in an invalid, but still memory safe, state. If you are worried about that only use
/// poisoning mutexes or atomics to mutate shared data or discard all shared data used in the `task`
/// if this function returns an error.
///
/// [unwind safety validation]: std::panic::UnwindSafe
pub async fn wait_catch<T, F>(task: F) -> PanicResult<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    blocking::unblock(move || panic::catch_unwind(panic::AssertUnwindSafe(task))).await
}

/// Fire and forget a [`wait`] task. The `task` starts executing immediately.
///
/// # Panic Handling
///
/// If the `task` panics the panic message is logged as an error, the panic is otherwise ignored.
///
/// # Unwind Safety
///
/// This function disables the [unwind safety validation], meaning that in case of a panic shared
/// data can end-up in an invalid, but still memory safe, state. If you are worried about that only use
/// poisoning mutexes or atomics to mutate shared data or use [`wait_catch`] to detect a panic or [`wait`]
/// to propagate a panic.
///
/// [unwind safety validation]: std::panic::UnwindSafe
pub fn spawn_wait<F>(task: F)
where
    F: FnOnce() + Send + 'static,
{
    spawn(async move {
        if let Err(p) = wait_catch(task).await {
            tracing::error!("parallel `spawn_wait` task panicked: {}", panic_str(&p))
        }
    });
}

/// Blocks the thread until the `task` future finishes.
///
/// This function is useful for implementing async tests, using it in an app will probably cause
/// the app to stop responding. To test UI task use [`TestWidgetContext::block_on`] or [`HeadlessApp::block_on`].
///
/// The crate [`futures-lite`] is used to execute the task.
///
/// # Examples
///
/// Test a [`run`] call:
///
/// ```
/// use zero_ui_core::task;
/// # use zero_ui_core::units::*;
/// # async fn foo(u: u8) -> Result<u8, ()> { task::deadline(1.ms()).await; Ok(u) }
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
/// [`futures-lite`]: https://docs.rs/futures-lite/
pub fn block_on<F>(task: F) -> F::Output
where
    F: Future,
{
    futures_lite::future::block_on(task)
}

/// Continuous poll the `task` until if finishes.
///
/// This function is useful for implementing some async tests only, futures don't expect to be polled
/// continuously. This function is only available in test builds.
#[cfg(any(test, doc, feature = "test_util"))]
#[cfg_attr(doc_nightly, doc(cfg(feature = "test_util")))]
pub fn spin_on<F>(task: F) -> F::Output
where
    F: Future,
{
    pin!(task);
    block_on(poll_fn(|cx| match task.as_mut().poll(cx) {
        Poll::Ready(r) => Poll::Ready(r),
        Poll::Pending => {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }))
}

/// Executor used in async doc tests.
///
/// If `spin` is `true` the [`spin_on`] executor is used with a timeout of 500 milliseconds.
/// IF `spin` is `false` the [`block_on`] executor is used with a timeout of 5 seconds.
#[cfg(any(test, doc, feature = "test_util"))]
#[cfg_attr(doc_nightly, doc(cfg(feature = "test_util")))]
pub fn doc_test<F>(spin: bool, task: F) -> F::Output
where
    F: Future,
{
    use crate::units::TimeUnits;

    if spin {
        spin_on(with_deadline(task, 500.ms())).expect("async doc-test timeout")
    } else {
        block_on(with_deadline(task, 5.secs())).expect("async doc-test timeout")
    }
}

/// A future that is [`Pending`] once.
///
/// After the first `.await` the future is always [`Ready`].
///
/// # Warning
///
/// This does not schedule an [`wake`], if the executor does not poll this future again it will wait forever.
/// You can use [`yield_now`] to force a wake in parallel tasks or use [`AppContextMut::update`] or
/// [`WidgetContextMut::update`] to force an update in UI tasks.
///
/// [`Pending`]: std::task::Poll::Pending
/// [`Ready`]: std::task::Poll::Ready
/// [`wake`]: std::task::Waker::wake
/// [`AppContextMut::update`]: crate::context::AppContextMut::update
/// [`WidgetContextMut::update`]: crate::context::WidgetContextMut::update
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

/// A future that is [`Pending`] once and wakes the current task.
///
/// After the first `.await` the future is always [`Ready`] and on the first `.await` if calls [`wake`].
///
/// # UI Update
///
/// In UI tasks you can call [`AppContextMut::update`] or [`WidgetContextMut::update`] instead of this function
/// for a slightly increase in performance.
///
/// [`Pending`]: std::task::Poll::Pending
/// [`Ready`]: std::task::Poll::Ready
/// [`wake`]: std::task::Waker::wake
/// [`AppContextMut::update`]: crate::context::AppContextMut::update
/// [`WidgetContextMut::update`]: crate::context::WidgetContextMut::update
pub async fn yield_now() {
    struct YieldNowFut(bool);
    impl Future for YieldNowFut {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
            if self.0 {
                Poll::Ready(())
            } else {
                self.0 = true;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }

    YieldNowFut(false).await
}

/// A future that is [`Pending`] until the `deadline` is reached.
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
///     task::deadline(5.secs()).await;
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
pub async fn deadline(deadline: impl Into<Deadline>) {
    let deadline = deadline.into();
    if let Some(timeout) = deadline.time_left() {
        futures_timer::Delay::new(timeout).await
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
            (self.0)(cx)
        }
    }
    PollFn(fn_).await
}

/// Error when [`with_deadline`] reach a time limit before a task finishes.
#[derive(Debug, Clone, Copy)]
pub struct DeadlineError {
    /// The deadline that was reached.
    pub deadline: Deadline,
}
impl fmt::Display for DeadlineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "reached deadline, {:?}", self.deadline)
    }
}
impl std::error::Error for DeadlineError {}

/// Add a [`deadline`] to a future.
///
/// Returns the `fut` output or [`DeadlineError`] if the deadline elapses first.
pub async fn with_deadline<O, F: Future<Output = O>>(fut: F, deadline: impl Into<Deadline>) -> Result<F::Output, DeadlineError> {
    let deadline = deadline.into();
    any!(async { Ok(fut.await) }, async {
        self::deadline(deadline).await;
        Err(DeadlineError { deadline })
    })
    .await
}

/// <span data-del-macro-root></span> Pins variables on the stack.
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
///
///     task::pin!(fut);
///
///     let mut count = 0;
///     task::poll_fn(|cx| {
///         count += 1;
///         match fut.as_mut().poll(cx) {
///             Poll::Ready(r) => {
///                 println!("polled {count} times");
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
            // SAFETY: $var is moved to the stack, exclusively borrowed and shadowed
            // by the pinned borrow, there is no way to move $var.
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

/// <span data-del-macro-root></span> A future that *zips* other futures.
///
/// The macro input is a comma separated list of future expressions. The macro output is a future
/// that when ".awaited" produces a tuple of results in the same order as the inputs.
///
/// At least one input future is required and any number of futures is accepted. For more than
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
/// # task::doc_test(false, async {
/// let (a, b, c) = task::all!(
///     task::run(async { 'a' }),
///     task::wait(|| "b"),
///     async { b"c" }
/// ).await;
/// # });
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
            $(let mut $ident = (Some($fut), None);)+
            $crate::task::poll_fn(move |cx| {
                use std::task::Poll;
                use std::future::Future;

                let mut pending = false;

                $(
                    if let Some(fut) = $ident.0.as_mut() {
                        // SAFETY: the closure owns $ident and is an exclusive borrow inside a
                        // Future::poll call, so it will not move.
                        let mut fut = unsafe { std::pin::Pin::new_unchecked(fut) };
                        if let Poll::Ready(r) = fut.as_mut().poll(cx) {
                            $ident.0 = None;
                            $ident.1 = Some(r);
                        } else {
                            pending = true;
                        }
                    }
                )+

                if pending {
                    Poll::Pending
                } else {
                    Poll::Ready(($($ident.1.take().unwrap()),+))
                }
            })
        }
    }
}

/// <span data-del-macro-root></span> A future that awaits for the first future that is ready.
///
/// The macro input is comma separated list of future expressions, the futures must
/// all have the same output type. The macro output is a future that when ".awaited" produces
/// a single output type instance returned by the first input future that completes.
///
/// At least one input future is required and any number of futures is accepted. For more than
/// eight futures a proc-macro is used which may cause code auto-complete to stop working in
/// some IDEs.
///
/// If two futures are ready at the same time the result of the first future in the input list is used.
/// After one future is ready the other futures are not polled again and are dropped.
///
/// # Examples
///
/// Await for the first of three futures to complete:
///
/// ```
/// use zero_ui_core::{task, units::*};
///
/// # task::doc_test(false, async {
/// let r = task::any!(
///     task::run(async { task::deadline(300.ms()).await; 'a' }),
///     task::wait(|| 'b'),
///     async { task::deadline(300.ms()).await; 'c' }
/// ).await;
///
/// assert_eq!('b', r);
/// # });
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
            $(let mut $ident = $fut;)+
            $crate::task::poll_fn(move |cx| {
                use std::task::Poll;
                use std::future::Future;
                $(
                    // SAFETY: the closure owns $ident and is an exclusive borrow inside a
                    // Future::poll call, so it will not move.
                    let mut $ident = unsafe { std::pin::Pin::new_unchecked(&mut $ident) };
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

/// <span data-del-macro-root></span> A future that waits for the first future that is ready with an `Ok(T)` result.
///
/// The macro input is comma separated list of future expressions, the futures must
/// all have the same output `Result<T, E>` type, but each can have a different `E`. The macro output is a future
/// that when ".awaited" produces a single output of type `Result<T, (E0, E1, ..)>` that is `Ok(T)` if any of the futures
/// is `Ok(T)` or is `Err((E0, E1, ..))` is all futures are `Err`.
///
/// At least one input future is required and any number of futures is accepted. For more than
/// eight futures a proc-macro is used which may cause code auto-complete to stop working in
/// some IDEs.
///
/// If two futures are ready and `Ok(T)` at the same time the result of the first future in the input list is used.
/// After one future is ready and `Ok(T)` the other futures are not polled again and are dropped. After a future
/// is ready and `Err(E)` it is also not polled again and dropped.
///
/// # Examples
///
/// Await for the first of three futures to complete with `Ok`:
///
/// ```
/// use zero_ui_core::task;
/// # #[derive(Debug, PartialEq)]
/// # pub struct FooError;
/// # task::doc_test(false, async {
/// let r = task::any_ok!(
///     task::run(async { Err::<char, _>("error") }),
///     task::wait(|| Ok::<_, FooError>('b')),
///     async { Err::<char, _>(FooError) }
/// ).await;
///
/// assert_eq!(Ok('b'), r);
/// # });
/// ```
#[macro_export]
macro_rules! any_ok {
    ($fut0:expr $(,)?) => { $crate::__any_ok! { fut0: $fut0; } };
    ($fut0:expr, $fut1:expr $(,)?) => {
        $crate::__any_ok! {
            fut0: $fut0;
            fut1: $fut1;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr $(,)?) => {
        $crate::__any_ok! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr, $fut3:expr $(,)?) => {
        $crate::__any_ok! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
            fut3: $fut3;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr, $fut3:expr, $fut4:expr $(,)?) => {
        $crate::__any_ok! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
            fut3: $fut3;
            fut4: $fut4;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr, $fut3:expr, $fut4:expr, $fut5:expr $(,)?) => {
        $crate::__any_ok! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
            fut3: $fut3;
            fut4: $fut4;
            fut5: $fut5;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr, $fut3:expr, $fut4:expr, $fut5:expr, $fut6:expr $(,)?) => {
        $crate::__any_ok! {
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
        $crate::__any_ok! {
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
    ($($fut:expr),+ $(,)?) => { $crate::task::__proc_any_all!{ $crate::__any_ok; $($fut),+ } }
}
#[doc(inline)]
pub use crate::any_ok;

#[doc(hidden)]
#[macro_export]
macro_rules! __any_ok {
    ($($ident:ident: $fut: expr;)+) => {
        {
            $(let mut $ident = (Some($fut), None);)+
            $crate::task::poll_fn(move |cx| {
                use std::task::Poll;
                use std::future::Future;

                let mut pending = false;

                $(
                    if let Some(fut) = $ident.0.as_mut() {
                        // SAFETY: the closure owns $ident and is an exclusive borrow inside a
                        // Future::poll call, so it will not move.
                        let mut fut = unsafe { std::pin::Pin::new_unchecked(fut) };
                        if let Poll::Ready(r) = fut.as_mut().poll(cx) {
                            match r {
                                Ok(r) => return Poll::Ready(Ok(r)),
                                Err(e) => {
                                    $ident.0 = None;
                                    $ident.1 = Some(e);
                                }
                            }
                        } else {
                            pending = true;
                        }
                    }
                )+

                if pending {
                    Poll::Pending
                } else {
                    Poll::Ready(Err((
                        $($ident.1.take().unwrap()),+
                    )))
                }
            })
        }
    }
}

/// <span data-del-macro-root></span> A future that is ready when any of the futures is ready and `Some(T)`.
///
/// The macro input is comma separated list of future expressions, the futures must
/// all have the same output `Option<T>` type. The macro output is a future that when ".awaited" produces
/// a single output type instance returned by the first input future that completes with a `Some`.
/// If all futures complete with a `None` the output is `None`.
///
/// At least one input future is required and any number of futures is accepted. For more than
/// eight futures a proc-macro is used which may cause code auto-complete to stop working in
/// some IDEs.
///
/// If two futures are ready and `Some(T)` at the same time the result of the first future in the input list is used.
/// After one future is ready and `Some(T)` the other futures are not polled again and are dropped. After a future
/// is ready and `None` it is also not polled again and dropped.
///
/// # Examples
///
/// Await for the first of three futures to complete with `Some`:
///
/// ```
/// use zero_ui_core::task;
/// # task::doc_test(false, async {
/// let r = task::any_some!(
///     task::run(async { None::<char> }),
///     task::wait(|| Some('b')),
///     async { None::<char> }
/// ).await;
///
/// assert_eq!(Some('b'), r);
/// # });
/// ```
#[macro_export]
macro_rules! any_some {
    ($fut0:expr $(,)?) => { $crate::__any_some! { fut0: $fut0; } };
    ($fut0:expr, $fut1:expr $(,)?) => {
        $crate::__any_some! {
            fut0: $fut0;
            fut1: $fut1;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr $(,)?) => {
        $crate::__any_some! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr, $fut3:expr $(,)?) => {
        $crate::__any_some! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
            fut3: $fut3;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr, $fut3:expr, $fut4:expr $(,)?) => {
        $crate::__any_some! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
            fut3: $fut3;
            fut4: $fut4;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr, $fut3:expr, $fut4:expr, $fut5:expr $(,)?) => {
        $crate::__any_some! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
            fut3: $fut3;
            fut4: $fut4;
            fut5: $fut5;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr, $fut3:expr, $fut4:expr, $fut5:expr, $fut6:expr $(,)?) => {
        $crate::__any_some! {
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
        $crate::__any_some! {
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
    ($($fut:expr),+ $(,)?) => { $crate::task::__proc_any_all!{ $crate::__any_some; $($fut),+ } }
}
#[doc(inline)]
pub use crate::any_some;

#[doc(hidden)]
#[macro_export]
macro_rules! __any_some {
    ($($ident:ident: $fut: expr;)+) => {
        {
            $(let mut $ident = Some($fut);)+
            $crate::task::poll_fn(move |cx| {
                use std::task::Poll;
                use std::future::Future;

                let mut pending = false;

                $(
                    if let Some(fut) = $ident.as_mut() {
                        // SAFETY: the closure owns $ident and is an exclusive borrow inside a
                        // Future::poll call, so it will not move.
                        let mut fut = unsafe { std::pin::Pin::new_unchecked(fut) };
                        if let Poll::Ready(r) = fut.as_mut().poll(cx) {
                            if let Some(r) = r {
                                return Poll::Ready(Some(r));
                            }
                            $ident = None;
                        } else {
                            pending = true;
                        }
                    }
                )+

                if pending {
                    Poll::Pending
                } else {
                    Poll::Ready(None)
                }
            })
        }
    }
}

/// <span data-del-macro-root></span> A future that is ready when all futures are ready with an `Ok(T)` result or
/// any is ready with an `Err(E)` result.
///
/// The output type is `Result<(T0, T1, ..), E>`, the `Ok` type is a tuple with all the `Ok` values, the error
/// type is the first error encountered, the input futures must have the same `Err` type but can have different
/// `Ok` types.
///
/// At least one input future is required and any number of futures is accepted. For more than
/// eight futures a proc-macro is used which may cause code auto-complete to stop working in
/// some IDEs.
///
/// If two futures are ready and `Err(E)` at the same time the result of the first future in the input list is used.
/// After one future is ready and `Err(T)` the other futures are not polled again and are dropped. After a future
/// is ready it is also not polled again and dropped.
///
/// # Examples
///
/// Await for the first of three futures to complete with `Ok(T)`:
///
/// ```
/// use zero_ui_core::task;
/// # #[derive(Debug, PartialEq)]
/// # struct FooError;
/// # task::doc_test(false, async {
/// let r = task::all_ok!(
///     task::run(async { Ok::<_, FooError>('a') }),
///     task::wait(|| Ok::<_, FooError>('b')),
///     async { Ok::<_, FooError>('c') }
/// ).await;
///
/// assert_eq!(Ok(('a', 'b', 'c')), r);
/// # });
/// ```
///
/// And in if any completes with `Err(E)`:
///
/// ```
/// use zero_ui_core::task;
/// # #[derive(Debug, PartialEq)]
/// # struct FooError;
/// # task::doc_test(false, async {
/// let r = task::all_ok!(
///     task::run(async { Ok('a') }),
///     task::wait(|| Err::<char, _>(FooError)),
///     async { Ok('c') }
/// ).await;
///
/// assert_eq!(Err(FooError), r);
/// # });
#[macro_export]
macro_rules! all_ok {
    ($fut0:expr $(,)?) => { $crate::__all_ok! { fut0: $fut0; } };
    ($fut0:expr, $fut1:expr $(,)?) => {
        $crate::__all_ok! {
            fut0: $fut0;
            fut1: $fut1;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr $(,)?) => {
        $crate::__all_ok! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr, $fut3:expr $(,)?) => {
        $crate::__all_ok! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
            fut3: $fut3;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr, $fut3:expr, $fut4:expr $(,)?) => {
        $crate::__all_ok! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
            fut3: $fut3;
            fut4: $fut4;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr, $fut3:expr, $fut4:expr, $fut5:expr $(,)?) => {
        $crate::__all_ok! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
            fut3: $fut3;
            fut4: $fut4;
            fut5: $fut5;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr, $fut3:expr, $fut4:expr, $fut5:expr, $fut6:expr $(,)?) => {
        $crate::__all_ok! {
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
        $crate::__all_ok! {
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
    ($($fut:expr),+ $(,)?) => { $crate::task::__proc_any_all!{ $crate::__all_ok; $($fut),+ } }
}
#[doc(inline)]
pub use crate::all_ok;

#[doc(hidden)]
#[macro_export]
macro_rules! __all_ok {
    ($($ident:ident: $fut: expr;)+) => {
        {
            $(let mut $ident = (Some($fut), None);)+

            $crate::task::poll_fn(move |cx| {
                use std::task::Poll;
                use std::future::Future;

                let mut pending = false;

                $(
                    if let Some(fut) = $ident.0.as_mut() {
                        // SAFETY: the closure owns $ident and is an exclusive borrow inside a
                        // Future::poll call, so it will not move.
                        let mut fut = unsafe { std::pin::Pin::new_unchecked(fut) };
                        if let Poll::Ready(r) = fut.as_mut().poll(cx) {
                            match r {
                                Ok(r) => {
                                    $ident.0 = None;
                                    $ident.1 = Some(r);
                                },
                                Err(e) => return Poll::Ready(Err(e)),
                            }
                        } else {
                            pending = true;
                        }
                    }
                )+

                if pending {
                    Poll::Pending
                } else {
                    Poll::Ready(Ok((
                        $($ident.1.take().unwrap()),+
                    )))
                }
            })
        }
    }
}

/// <span data-del-macro-root></span> A future that is ready when all futures are ready with `Some(T)` or when any
/// is ready with `None`
///
/// The macro input is comma separated list of future expressions, the futures must
/// all have the `Option<T>` output type, but each can have a different `T`. The macro output is a future that when ".awaited"
/// produces `Some<(T0, T1, ..)>` if all futures where `Some(T)` or `None` if any of the futures where `None`.
///
/// At least one input future is required and any number of futures is accepted. For more than
/// eight futures a proc-macro is used which may cause code auto-complete to stop working in
/// some IDEs.
///
/// After one future is ready and `None` the other futures are not polled again and are dropped. After a future
/// is ready it is also not polled again and dropped.
///
/// # Examples
///
/// Await for the first of three futures to complete with `Some`:
///
/// ```
/// use zero_ui_core::task;
/// # task::doc_test(false, async {
/// let r = task::all_some!(
///     task::run(async { Some('a') }),
///     task::wait(|| Some('b')),
///     async { Some('c') }
/// ).await;
///
/// assert_eq!(Some(('a', 'b', 'c')), r);
/// # });
/// ```
///
/// Completes with `None` if any future completes with `None`:
///
/// ```
/// # use zero_ui_core::task;
/// # task::doc_test(false, async {
/// let r = task::all_some!(
///     task::run(async { Some('a') }),
///     task::wait(|| None::<char>),
///     async { Some('b') }
/// ).await;
///
/// assert_eq!(None, r);
/// # });
/// ```
#[macro_export]
macro_rules! all_some {
    ($fut0:expr $(,)?) => { $crate::__all_some! { fut0: $fut0; } };
    ($fut0:expr, $fut1:expr $(,)?) => {
        $crate::__all_some! {
            fut0: $fut0;
            fut1: $fut1;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr $(,)?) => {
        $crate::__all_some! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr, $fut3:expr $(,)?) => {
        $crate::__all_some! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
            fut3: $fut3;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr, $fut3:expr, $fut4:expr $(,)?) => {
        $crate::__all_some! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
            fut3: $fut3;
            fut4: $fut4;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr, $fut3:expr, $fut4:expr, $fut5:expr $(,)?) => {
        $crate::__all_some! {
            fut0: $fut0;
            fut1: $fut1;
            fut2: $fut2;
            fut3: $fut3;
            fut4: $fut4;
            fut5: $fut5;
        }
    };
    ($fut0:expr, $fut1:expr, $fut2:expr, $fut3:expr, $fut4:expr, $fut5:expr, $fut6:expr $(,)?) => {
        $crate::__all_some! {
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
        $crate::__all_some! {
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
    ($($fut:expr),+ $(,)?) => { $crate::task::__proc_any_all!{ $crate::__all_some; $($fut),+ } }
}
#[doc(inline)]
pub use crate::all_some;

#[doc(hidden)]
#[macro_export]
macro_rules! __all_some {
    ($($ident:ident: $fut: expr;)+) => {
        {
            $(let mut $ident = (Some($fut), None);)+
            $crate::task::poll_fn(move |cx| {
                use std::task::Poll;
                use std::future::Future;

                let mut pending = false;

                $(
                    if let Some(fut) = $ident.0.as_mut() {
                        // SAFETY: the closure owns $ident and is an exclusive borrow inside a
                        // Future::poll call, so it will not move.
                        let mut fut = unsafe { std::pin::Pin::new_unchecked(fut) };
                        if let Poll::Ready(r) = fut.as_mut().poll(cx) {
                            if r.is_none() {
                                return Poll::Ready(None);
                            }

                            $ident.0 = None;
                            $ident.1 = r;
                        } else {
                            pending = true;
                        }
                    }
                )+

                if pending {
                    Poll::Pending
                } else {
                    Poll::Ready(Some((
                        $($ident.1.take().unwrap()),+
                    )))
                }
            })
        }
    }
}

/// A future that will await until [`set`] is called.
///
/// # Examples
///
/// Spawns a parallel task that only writes to stdout after the main thread sets the signal:
///
/// ```
/// use zero_ui_core::task::{self, *};
///
/// let signal = SignalOnce::default();
///
/// task::spawn(async_clone_move!(signal, {
///     signal.await;
///     println!("After Signal!");
/// }));
///
/// signal.set();
/// ```
///
/// [`set`]: SignalOnce::set
#[derive(Default, Clone)]
pub struct SignalOnce(Arc<SignalInner>);
impl fmt::Debug for SignalOnce {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SignalOnce({})", self.is_set())
    }
}
impl SignalOnce {
    /// New unsigned.
    pub fn new() -> Self {
        Self::default()
    }

    /// New signaled.
    pub fn new_set() -> Self {
        let s = Self::new();
        s.set();
        s
    }

    /// If the signal was set.
    pub fn is_set(&self) -> bool {
        self.0.signaled.load(Ordering::Relaxed)
    }

    /// Sets the signal and awakes listeners.
    pub fn set(&self) {
        if !self.0.signaled.swap(true, Ordering::Relaxed) {
            let listeners = mem::take(&mut *self.0.listeners.lock());
            for listener in listeners {
                listener.wake();
            }
        }
    }
}
impl Future for SignalOnce {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<()> {
        if self.as_ref().is_set() {
            Poll::Ready(())
        } else {
            let mut listeners = self.0.listeners.lock();
            let waker = cx.waker();
            if !listeners.iter().any(|w| w.will_wake(waker)) {
                listeners.push(waker.clone());
            }
            Poll::Pending
        }
    }
}

#[derive(Default)]
struct SignalInner {
    signaled: AtomicBool,
    listeners: Mutex<Vec<std::task::Waker>>,
}

/// A [`Waker`] that dispatches a wake call to multiple other wakers.
///
/// This is useful for sharing one wake source with multiple [`Waker`] clients that may not be all
/// known at the moment the first request is made.
///  
/// [`Waker`]: std::task::Waker
#[derive(Clone)]
pub struct McWaker(Arc<WakeVec>);

#[derive(Default)]
struct WakeVec(Mutex<Vec<std::task::Waker>>);
impl WakeVec {
    fn push(&self, waker: std::task::Waker) -> bool {
        let mut v = self.0.lock();

        let return_waker = v.is_empty();

        v.push(waker);

        return_waker
    }

    fn cancel(&self) {
        let mut v = self.0.lock();

        debug_assert!(!v.is_empty(), "called cancel on an empty McWaker");

        v.clear();
    }
}
impl std::task::Wake for WakeVec {
    fn wake(self: Arc<Self>) {
        for w in mem::take(&mut *self.0.lock()) {
            w.wake();
        }
    }
}
impl McWaker {
    /// New empty waker.
    pub fn empty() -> Self {
        Self(Arc::new(WakeVec::default()))
    }

    /// Register a `waker` to wake once when `self` awakes.
    ///
    /// Returns `Some(self as waker)` if `self` was previously empty, if `None` is returned [`Poll::Pending`] must
    /// be returned, if a waker is returned the shared resource must be polled using the waker, if the shared resource
    /// is ready [`cancel`] must be called.
    ///
    /// [`cancel`]: Self::cancel
    pub fn push(&self, waker: std::task::Waker) -> Option<std::task::Waker> {
        if self.0.push(waker) {
            Some(self.0.clone().into())
        } else {
            None
        }
    }

    /// Clear current registered wakers.
    pub fn cancel(&self) {
        self.0.cancel()
    }
}

#[cfg(test)]
pub mod tests {
    use rayon::prelude::*;

    use super::*;
    use crate::units::TimeUnits;

    #[track_caller]
    fn async_test<F>(test: F) -> F::Output
    where
        F: Future,
    {
        block_on(with_deadline(test, 5.secs())).unwrap()
    }

    #[test]
    pub fn any_one() {
        let r = async_test(async { any!(async { true }).await });

        assert!(r);
    }

    #[test]
    pub fn any_nine() {
        let one_s = 1.secs();
        let r = async_test(async {
            any!(
                async {
                    deadline(one_s).await;
                    1
                },
                async {
                    deadline(one_s).await;
                    2
                },
                async {
                    deadline(one_s).await;
                    3
                },
                async {
                    deadline(one_s).await;
                    4
                },
                async {
                    deadline(one_s).await;
                    5
                },
                async {
                    deadline(one_s).await;
                    6
                },
                async {
                    deadline(one_s).await;
                    7
                },
                async {
                    deadline(one_s).await;
                    8
                },
                async { 9 },
            )
            .await
        });

        assert_eq!(9, r);
    }

    #[test]
    pub fn run_wake_imediatly() {
        async_test(async {
            run(async {
                yield_now().await;
            })
            .await;
        });
    }

    #[test]
    pub fn run_panic_handling() {
        async_test(async {
            let r = run_catch(async {
                run(async {
                    deadline(1.ms()).await;
                    panic!("test panic")
                })
                .await;
            })
            .await;

            assert!(r.is_err());
        })
    }

    #[test]
    pub fn run_panic_handling_parallel() {
        async_test(async {
            let r = run_catch(async {
                run(async {
                    deadline(1.ms()).await;
                    (0..100000).into_par_iter().for_each(|i| {
                        if i == 50005 {
                            panic!("test panic");
                        }
                    });
                })
                .await;
            })
            .await;

            assert!(r.is_err());
        })
    }
}
