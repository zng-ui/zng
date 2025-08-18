#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Parallel async tasks and async task runners.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::{
    fmt,
    hash::Hash,
    mem, panic,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::Poll,
};

#[doc(no_inline)]
pub use parking_lot;
use parking_lot::Mutex;

mod crate_util;

use crate::crate_util::PanicResult;
use zng_app_context::{LocalContext, app_local};
use zng_time::Deadline;
use zng_var::{ResponseVar, VarValue, response_done_var, response_var};

#[cfg(test)]
mod tests;

#[doc(no_inline)]
pub use rayon;

/// Async filesystem primitives.
///
/// This module is the [async-fs](https://docs.rs/async-fs) crate re-exported for convenience.
pub mod fs {
    #[doc(inline)]
    pub use async_fs::*;
}

pub mod channel;
pub mod io;
mod ui;

pub mod http;

pub mod ipc;

mod rayon_ctx;

pub use rayon_ctx::*;

pub use ui::*;

mod progress;
pub use progress::*;

/// Spawn a parallel async task, this function is not blocking and the `task` starts executing immediately.
///
/// # Parallel
///
/// The task runs in the primary [`rayon`] thread-pool, every [`poll`](Future::poll) happens inside a call to `rayon::spawn`.
///
/// You can use parallel iterators, `join` or any of rayon's utilities inside `task` to make it multi-threaded,
/// otherwise it will run in a single thread at a time, still not blocking the UI.
///
/// The [`rayon`] crate is re-exported in `task::rayon` for convenience and compatibility.
///
/// # Async
///
/// The `task` is also a future so you can `.await`, after each `.await` the task continues executing in whatever `rayon` thread
/// is free, so the `task` should either be doing CPU intensive work or awaiting, blocking IO operations
/// block the thread from being used by other tasks reducing overall performance. You can use [`wait`] for IO
/// or blocking operations and for networking you can use any of the async crates, as long as they start their own *event reactor*.
///
/// The `task` lives inside the [`Waker`] when awaiting and inside `rayon::spawn` when running.
///
/// # Examples
///
/// ```
/// # use zng_task::{self as task, *, rayon::iter::*};
/// # use zng_var::*;
/// # struct SomeStruct { sum_response: ResponseVar<usize> }
/// # impl SomeStruct {
/// fn on_event(&mut self) {
///     let (responder, response) = response_var();
///     self.sum_response = response;
///
///     task::spawn(async move {
///         let r = (0..1000).into_par_iter().map(|i| i * i).sum();
///
///         responder.respond(r);
///     });
/// }
///
/// fn on_update(&mut self) {
///     if let Some(result) = self.sum_response.rsp_new() {
///         println!("sum of squares 0..1000: {result}");
///     }
/// }
/// # }
/// ```
///
/// The example uses the `rayon` parallel iterator to compute a result and uses a [`response_var`] to send the result to the UI.
/// The task captures the caller [`LocalContext`] so the response variable will set correctly.
///
/// Note that this function is the most basic way to spawn a parallel task where you must setup channels to the rest of the app yourself,
/// you can use [`respond`] to avoid having to manually set a response, or [`run`] to `.await` the result.
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
/// [`rayon`]: https://docs.rs/rayon
/// [`LocalContext`]: zng_app_context::LocalContext
/// [`response_var`]: zng_var::response_var
pub fn spawn<F>(task: impl IntoFuture<IntoFuture = F>)
where
    F: Future<Output = ()> + Send + 'static,
{
    Arc::new(RayonTask {
        ctx: LocalContext::capture(),
        fut: Mutex::new(Some(Box::pin(task.into_future()))),
    })
    .poll()
}

/// Polls the `task` once immediately on the calling thread, if the `task` is pending, continues execution in [`spawn`].
pub fn poll_spawn<F>(task: impl IntoFuture<IntoFuture = F>)
where
    F: Future<Output = ()> + Send + 'static,
{
    struct PollRayonTask {
        fut: Mutex<Option<(RayonSpawnFut, Option<LocalContext>)>>,
    }
    impl PollRayonTask {
        // start task in calling thread
        fn poll(self: Arc<Self>) {
            let mut task = self.fut.lock();
            let (mut t, _) = task.take().unwrap();

            let waker = self.clone().into();

            match t.as_mut().poll(&mut std::task::Context::from_waker(&waker)) {
                Poll::Ready(()) => {}
                Poll::Pending => {
                    let ctx = LocalContext::capture();
                    *task = Some((t, Some(ctx)));
                }
            }
        }
    }
    impl std::task::Wake for PollRayonTask {
        fn wake(self: Arc<Self>) {
            // continue task in spawn threads
            if let Some((task, Some(ctx))) = self.fut.lock().take() {
                Arc::new(RayonTask {
                    ctx,
                    fut: Mutex::new(Some(Box::pin(task))),
                })
                .poll();
            }
        }
    }

    Arc::new(PollRayonTask {
        fut: Mutex::new(Some((Box::pin(task.into_future()), None))),
    })
    .poll()
}

type RayonSpawnFut = Pin<Box<dyn Future<Output = ()> + Send>>;

// A future that is its own waker that polls inside rayon spawn tasks.
struct RayonTask {
    ctx: LocalContext,
    fut: Mutex<Option<RayonSpawnFut>>,
}
impl RayonTask {
    fn poll(self: Arc<Self>) {
        rayon::spawn(move || {
            // this `Option<Fut>` dance is used to avoid a `poll` after `Ready` or panic.
            let mut task = self.fut.lock();
            if let Some(mut t) = task.take() {
                let waker = self.clone().into();

                // load app context
                self.ctx.clone().with_context(move || {
                    let r = panic::catch_unwind(panic::AssertUnwindSafe(move || {
                        // poll future
                        if t.as_mut().poll(&mut std::task::Context::from_waker(&waker)).is_pending() {
                            // not done
                            *task = Some(t);
                        }
                    }));
                    if let Err(p) = r {
                        tracing::error!("panic in `task::spawn`: {}", crate_util::panic_str(&p));
                    }
                });
            }
        })
    }
}
impl std::task::Wake for RayonTask {
    fn wake(self: Arc<Self>) {
        self.poll()
    }
}

/// Rayon join with local context.
///
/// This function captures the [`LocalContext`] of the calling thread and propagates it to the threads that run the
/// operations.
///
/// See `rayon::join` for more details about join.
///
/// [`LocalContext`]: zng_app_context::LocalContext
pub fn join<A, B, RA, RB>(op_a: A, op_b: B) -> (RA, RB)
where
    A: FnOnce() -> RA + Send,
    B: FnOnce() -> RB + Send,
    RA: Send,
    RB: Send,
{
    self::join_context(move |_| op_a(), move |_| op_b())
}

/// Rayon join context with local context.
///
/// This function captures the [`LocalContext`] of the calling thread and propagates it to the threads that run the
/// operations.
///
/// See `rayon::join_context` for more details about join.
///
/// [`LocalContext`]: zng_app_context::LocalContext
pub fn join_context<A, B, RA, RB>(op_a: A, op_b: B) -> (RA, RB)
where
    A: FnOnce(rayon::FnContext) -> RA + Send,
    B: FnOnce(rayon::FnContext) -> RB + Send,
    RA: Send,
    RB: Send,
{
    let ctx = LocalContext::capture();
    let ctx = &ctx;
    rayon::join_context(
        move |a| {
            if a.migrated() {
                ctx.clone().with_context(|| op_a(a))
            } else {
                op_a(a)
            }
        },
        move |b| {
            if b.migrated() {
                ctx.clone().with_context(|| op_b(b))
            } else {
                op_b(b)
            }
        },
    )
}

/// Rayon scope with local context.
///
/// This function captures the [`LocalContext`] of the calling thread and propagates it to the threads that run the
/// operations.
///
/// See `rayon::scope` for more details about scope.
///
/// [`LocalContext`]: zng_app_context::LocalContext
pub fn scope<'scope, OP, R>(op: OP) -> R
where
    OP: FnOnce(ScopeCtx<'_, 'scope>) -> R + Send,
    R: Send,
{
    let ctx = LocalContext::capture();

    // Cast `&'_ ctx` to `&'scope ctx` to "inject" the context in the scope.
    // Is there a better way to do this? I hope so.
    //
    // SAFETY:
    // * We are extending `'_` to `'scope`, that is one of the documented valid usages of `transmute`.
    // * No use after free because `rayon::scope` joins all threads before returning and we only drop `ctx` after.
    let ctx_ref: &'_ LocalContext = &ctx;
    let ctx_scope_ref: &'scope LocalContext = unsafe { std::mem::transmute(ctx_ref) };

    let r = rayon::scope(move |s| {
        op(ScopeCtx {
            scope: s,
            ctx: ctx_scope_ref,
        })
    });

    drop(ctx);

    r
}

/// Represents a fork-join scope which can be used to spawn any number of tasks that run in the caller's thread context.
///
/// See [`scope`] for more details.
#[derive(Clone, Copy, Debug)]
pub struct ScopeCtx<'a, 'scope: 'a> {
    scope: &'a rayon::Scope<'scope>,
    ctx: &'scope LocalContext,
}
impl<'a, 'scope: 'a> ScopeCtx<'a, 'scope> {
    /// Spawns a job into the fork-join scope `self`. The job runs in the captured thread context.
    ///
    /// See `rayon::Scope::spawn` for more details.
    pub fn spawn<F>(self, f: F)
    where
        F: FnOnce(ScopeCtx<'_, 'scope>) + Send + 'scope,
    {
        let ctx = self.ctx;
        self.scope
            .spawn(move |s| ctx.clone().with_context(move || f(ScopeCtx { scope: s, ctx })));
    }
}

/// Spawn a parallel async task that can also be `.await` for the task result.
///
/// # Parallel
///
/// The task runs in the primary [`rayon`] thread-pool, every [`poll`](Future::poll) happens inside a call to `rayon::spawn`.
///
/// You can use parallel iterators, `join` or any of rayon's utilities inside `task` to make it multi-threaded,
/// otherwise it will run in a single thread at a time, still not blocking the UI.
///
/// The [`rayon`] crate is re-exported in `task::rayon` for convenience and compatibility.
///
/// # Async
///
/// The `task` is also a future so you can `.await`, after each `.await` the task continues executing in whatever `rayon` thread
/// is free, so the `task` should either be doing CPU intensive work or awaiting, blocking IO operations
/// block the thread from being used by other tasks reducing overall performance. You can use [`wait`] for IO
/// or blocking operations and for networking you can use any of the async crates, as long as they start their own *event reactor*.
///
/// The `task` lives inside the [`Waker`] when awaiting and inside `rayon::spawn` when running.
///
/// # Examples
///
/// ```
/// # use zng_task::{self as task, rayon::iter::*};
/// # struct SomeStruct { sum: usize }
/// # async fn read_numbers() -> Vec<usize> { vec![] }
/// # impl SomeStruct {
/// async fn on_event(&mut self) {
///     self.sum = task::run(async { read_numbers().await.par_iter().map(|i| i * i).sum() }).await;
/// }
/// # }
/// ```
///
/// The example `.await` for some numbers and then uses a parallel iterator to compute a result, this all runs in parallel
/// because it is inside a `run` task. The task result is then `.await` inside one of the UI async tasks. Note that the
/// task captures the caller [`LocalContext`] so you can interact with variables and UI services directly inside the task too.
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
/// If the `task` panics the panic is resumed in the awaiting thread using [`resume_unwind`]. You
/// can use [`run_catch`] to get the panic as an error instead.
///
/// [`resume_unwind`]: panic::resume_unwind
/// [`Waker`]: std::task::Waker
/// [`rayon`]: https://docs.rs/rayon
/// [`LocalContext`]: zng_app_context::LocalContext
pub async fn run<R, T>(task: impl IntoFuture<IntoFuture = T>) -> R
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
pub async fn run_catch<R, T>(task: impl IntoFuture<IntoFuture = T>) -> PanicResult<R>
where
    R: Send + 'static,
    T: Future<Output = R> + Send + 'static,
{
    type Fut<R> = Pin<Box<dyn Future<Output = R> + Send>>;

    // A future that is its own waker that polls inside the rayon primary thread-pool.
    struct RayonCatchTask<R> {
        ctx: LocalContext,
        fut: Mutex<Option<Fut<R>>>,
        sender: flume::Sender<PanicResult<R>>,
    }
    impl<R: Send + 'static> RayonCatchTask<R> {
        fn poll(self: Arc<Self>) {
            let sender = self.sender.clone();
            if sender.is_disconnected() {
                return; // cancel.
            }
            rayon::spawn(move || {
                // this `Option<Fut>` dance is used to avoid a `poll` after `Ready` or panic.
                let mut task = self.fut.lock();
                if let Some(mut t) = task.take() {
                    let waker = self.clone().into();
                    let mut cx = std::task::Context::from_waker(&waker);

                    self.ctx.clone().with_context(|| {
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
                    });
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

    Arc::new(RayonCatchTask {
        ctx: LocalContext::capture(),
        fut: Mutex::new(Some(Box::pin(task.into_future()))),
        sender: sender.into(),
    })
    .poll();

    receiver.recv().await.unwrap()
}

/// Spawn a parallel async task that will send its result to a [`ResponseVar<R>`].
///
/// The [`run`] documentation explains how `task` is *parallel* and *async*. The `task` starts executing immediately.
///
/// # Examples
///
/// ```
/// # use zng_task::{self as task, rayon::iter::*};
/// # use zng_var::*;
/// # struct SomeStruct { sum_response: ResponseVar<usize> }
/// # async fn read_numbers() -> Vec<usize> { vec![] }
/// # impl SomeStruct {
/// fn on_event(&mut self) {
///     self.sum_response = task::respond(async { read_numbers().await.par_iter().map(|i| i * i).sum() });
/// }
///
/// fn on_update(&mut self) {
///     if let Some(result) = self.sum_response.rsp_new() {
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
/// If the `task` panics the panic is logged as an error and resumed in the response var modify closure.
///
/// [`resume_unwind`]: panic::resume_unwind
/// [`ResponseVar<R>`]: zng_var::ResponseVar
/// [`response_var`]: zng_var::response_var
pub fn respond<R, F>(task: F) -> ResponseVar<R>
where
    R: VarValue,
    F: Future<Output = R> + Send + 'static,
{
    type Fut<R> = Pin<Box<dyn Future<Output = R> + Send>>;

    let (responder, response) = response_var();

    // A future that is its own waker that polls inside the rayon primary thread-pool.
    struct RayonRespondTask<R: VarValue> {
        ctx: LocalContext,
        fut: Mutex<Option<Fut<R>>>,
        responder: zng_var::ResponderVar<R>,
    }
    impl<R: VarValue> RayonRespondTask<R> {
        fn poll(self: Arc<Self>) {
            let responder = self.responder.clone();
            if responder.strong_count() == 2 {
                return; // cancel.
            }
            rayon::spawn(move || {
                // this `Option<Fut>` dance is used to avoid a `poll` after `Ready` or panic.
                let mut task = self.fut.lock();
                if let Some(mut t) = task.take() {
                    let waker = self.clone().into();
                    let mut cx = std::task::Context::from_waker(&waker);

                    self.ctx.clone().with_context(|| {
                        let r = panic::catch_unwind(panic::AssertUnwindSafe(|| t.as_mut().poll(&mut cx)));
                        match r {
                            Ok(Poll::Ready(r)) => {
                                drop(task);

                                responder.respond(r);
                            }
                            Ok(Poll::Pending) => {
                                *task = Some(t);
                            }
                            Err(p) => {
                                tracing::error!("panic in `task::respond`: {}", crate_util::panic_str(&p));
                                drop(task);
                                responder.modify(move |_| panic::resume_unwind(p));
                            }
                        }
                    });
                }
            })
        }
    }
    impl<R: VarValue> std::task::Wake for RayonRespondTask<R> {
        fn wake(self: Arc<Self>) {
            self.poll()
        }
    }

    Arc::new(RayonRespondTask {
        ctx: LocalContext::capture(),
        fut: Mutex::new(Some(Box::pin(task))),
        responder,
    })
    .poll();

    response
}

/// Polls the `task` once immediately on the calling thread, if the `task` is ready returns the response already set,
/// if the `task` is pending continues execution like [`respond`].
pub fn poll_respond<R, F>(task: impl IntoFuture<IntoFuture = F>) -> ResponseVar<R>
where
    R: VarValue,
    F: Future<Output = R> + Send + 'static,
{
    enum QuickResponse<R: VarValue> {
        Quick(Option<R>),
        Response(zng_var::ResponderVar<R>),
    }
    let task = task.into_future();
    let q = Arc::new(Mutex::new(QuickResponse::Quick(None)));
    poll_spawn(zng_clone_move::async_clmv!(q, {
        let rsp = task.await;

        match &mut *q.lock() {
            QuickResponse::Quick(q) => *q = Some(rsp),
            QuickResponse::Response(r) => r.respond(rsp),
        }
    }));

    let mut q = q.lock();
    match &mut *q {
        QuickResponse::Quick(q) if q.is_some() => response_done_var(q.take().unwrap()),
        _ => {
            let (responder, response) = response_var();
            *q = QuickResponse::Response(responder);
            response
        }
    }
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
/// # use zng_task as task;
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
/// If the `task` panics the panic is resumed in the awaiting thread using [`resume_unwind`]. You
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
    let mut ctx = LocalContext::capture();
    blocking::unblock(move || ctx.with_context(move || panic::catch_unwind(panic::AssertUnwindSafe(task)))).await
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
/// data can end-up in an invalid (still memory safe) state. If you are worried about that only use
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
            tracing::error!("parallel `spawn_wait` task panicked: {}", crate_util::panic_str(&p))
        }
    });
}

/// Like [`spawn_wait`], but the task will send its result to a [`ResponseVar<R>`].
///
/// # Cancellation
///
/// Dropping the [`ResponseVar<R>`] does not cancel the `task`, it will still run to completion.
///
/// # Panic Handling
///
/// If the `task` panics the panic is logged as an error and resumed in the response var modify closure.
pub fn wait_respond<R, F>(task: F) -> ResponseVar<R>
where
    R: VarValue,
    F: FnOnce() -> R + Send + 'static,
{
    let (responder, response) = response_var();
    spawn_wait(move || match panic::catch_unwind(panic::AssertUnwindSafe(task)) {
        Ok(r) => responder.respond(r),
        Err(p) => {
            tracing::error!("panic in `task::wait_respond`: {}", crate_util::panic_str(&p));
            responder.modify(move |_| panic::resume_unwind(p));
        }
    });
    response
}

/// Blocks the thread until the `task` future finishes.
///
/// This function is useful for implementing async tests, using it in an app will probably cause
/// the app to stop responding.
///
/// The crate [`futures-lite`] is used to execute the task.
///
/// # Examples
///
/// Test a [`run`] call:
///
/// ```
/// use zng_task as task;
/// # use zng_unit::*;
/// # async fn foo(u: u8) -> Result<u8, ()> { task::deadline(1.ms()).await; Ok(u) }
///
/// #[test]
/// # fn __() { }
/// pub fn run_ok() {
///     let r = task::block_on(task::run(async { foo(32).await }));
///
///     # let value =
///     r.expect("foo(32) was not Ok");
///     # assert_eq!(32, value);
/// }
/// # run_ok();
/// ```
///
/// [`futures-lite`]: https://docs.rs/futures-lite/
pub fn block_on<F>(task: impl IntoFuture<IntoFuture = F>) -> F::Output
where
    F: Future,
{
    futures_lite::future::block_on(task.into_future())
}

/// Continuous poll the `task` until if finishes.
///
/// This function is useful for implementing some async tests only, futures don't expect to be polled
/// continuously. This function is only available in test builds.
#[cfg(any(test, doc, feature = "test_util"))]
pub fn spin_on<F>(task: impl IntoFuture<IntoFuture = F>) -> F::Output
where
    F: Future,
{
    use std::pin::pin;

    let mut task = pin!(task.into_future());
    block_on(future_fn(|cx| match task.as_mut().poll(cx) {
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
pub fn doc_test<F>(spin: bool, task: impl IntoFuture<IntoFuture = F>) -> F::Output
where
    F: Future,
{
    use zng_unit::TimeUnits;

    if spin {
        spin_on(with_deadline(task, 500.ms())).expect("async doc-test timeout")
    } else {
        block_on(with_deadline(task, 5.secs())).expect("async doc-test timeout")
    }
}

/// A future that is [`Pending`] once and wakes the current task.
///
/// After the first `.await` the future is always [`Ready`] and on the first `.await` it calls [`wake`].
///
/// [`Pending`]: std::task::Poll::Pending
/// [`Ready`]: std::task::Poll::Ready
/// [`wake`]: std::task::Waker::wake
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
/// use zng_task as task;
/// use zng_unit::*;
///
/// task::spawn(async {
///     println!("waiting 5 seconds..");
///     task::deadline(5.secs()).await;
///     println!("5 seconds elapsed.")
/// });
/// ```
///
/// The future runs on an app provider timer executor, or on the [`futures_timer`] by default.
///
/// Note that deadlines from [`Duration`](std::time::Duration) starts *counting* at the moment this function is called,
/// not at the moment of the first `.await` call.
///
/// [`Pending`]: std::task::Poll::Pending
/// [`futures_timer`]: https://docs.rs/futures-timer
pub fn deadline(deadline: impl Into<Deadline>) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    let deadline = deadline.into();
    if zng_app_context::LocalContext::current_app().is_some() {
        DEADLINE_SV.read().0(deadline)
    } else {
        default_deadline(deadline)
    }
}

app_local! {
    static DEADLINE_SV: (DeadlineService, bool) = const { (default_deadline, false) };
}

type DeadlineService = fn(Deadline) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>>;

fn default_deadline(deadline: Deadline) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    if let Some(timeout) = deadline.time_left() {
        Box::pin(futures_timer::Delay::new(timeout))
    } else {
        Box::pin(std::future::ready(()))
    }
}

/// Deadline APP integration.
#[expect(non_camel_case_types)]
pub struct DEADLINE_APP;

impl DEADLINE_APP {
    /// Called by the app implementer to setup the [`deadline`] executor.
    ///
    /// If no app calls this the [`futures_timer`] executor is used.
    ///
    /// [`futures_timer`]: https://docs.rs/futures-timer
    ///
    /// # Panics
    ///
    /// Panics if called more than once for the same app.
    pub fn init_deadline_service(&self, service: DeadlineService) {
        let (prev, already_set) = mem::replace(&mut *DEADLINE_SV.write(), (service, true));
        if already_set {
            *DEADLINE_SV.write() = (prev, true);
            panic!("deadline service already inited for this app");
        }
    }
}

/// Implements a [`Future`] from a closure.
///
/// # Examples
///
/// A future that is ready with a closure returns `Some(R)`.
///
/// ```
/// use std::task::Poll;
/// use zng_task as task;
///
/// async fn ready_some<R>(mut closure: impl FnMut() -> Option<R>) -> R {
///     task::future_fn(|cx| match closure() {
///         Some(r) => Poll::Ready(r),
///         None => Poll::Pending,
///     })
///     .await
/// }
/// ```
pub async fn future_fn<T, F>(fn_: F) -> T
where
    F: FnMut(&mut std::task::Context) -> Poll<T>,
{
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
#[non_exhaustive]
pub struct DeadlineError {}
impl fmt::Display for DeadlineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "reached deadline")
    }
}
impl std::error::Error for DeadlineError {}

/// Add a [`deadline`] to a future.
///
/// Returns the `fut` output or [`DeadlineError`] if the deadline elapses first.
pub async fn with_deadline<O, F: Future<Output = O>>(
    fut: impl IntoFuture<IntoFuture = F>,
    deadline: impl Into<Deadline>,
) -> Result<F::Output, DeadlineError> {
    let deadline = deadline.into();
    any!(async { Ok(fut.await) }, async {
        self::deadline(deadline).await;
        Err(DeadlineError {})
    })
    .await
}

/// <span data-del-macro-root></span> A future that *zips* other futures.
///
/// The macro input is a comma separated list of future expressions. The macro output is a future
/// that when ".awaited" produces a tuple of results in the same order as the inputs.
///
/// At least one input future is required and any number of futures is accepted. For more than
/// eight futures a proc-macro is used which may cause code auto-complete to stop working in
/// some IDEs.
///
/// Each input must implement [`IntoFuture`]. Note that each input must be known at compile time, use the [`fn@all`] async
/// function to await on all futures in a dynamic list of futures.
///
/// # Examples
///
/// Await for three different futures to complete:
///
/// ```
/// use zng_task as task;
///
/// # task::doc_test(false, async {
/// let (a, b, c) = task::all!(task::run(async { 'a' }), task::wait(|| "b"), async { b"c" }).await;
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
    ($($fut:expr),+ $(,)?) => { $crate::__proc_any_all!{ $crate::__all; $($fut),+ } }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __all {
    ($($ident:ident: $fut:expr;)+) => {
        {
            $(let mut $ident = $crate::FutureOrOutput::Future(std::future::IntoFuture::into_future($fut));)+
            $crate::future_fn(move |cx| {
                use std::task::Poll;

                let mut pending = false;

                $(
                    if let $crate::FutureOrOutput::Future(fut) = &mut $ident {
                        // SAFETY: the closure owns $ident and is an exclusive borrow inside a
                        // Future::poll call, so it will not move.
                        let mut fut_mut = unsafe { std::pin::Pin::new_unchecked(fut) };
                        if let Poll::Ready(r) = fut_mut.as_mut().poll(cx) {
                            $ident = $crate::FutureOrOutput::Output(r);
                        } else {
                            pending = true;
                        }
                    }
                )+

                if pending {
                    Poll::Pending
                } else {
                    Poll::Ready(($($ident.take_output()),+))
                }
            })
        }
    }
}

#[doc(hidden)]
pub enum FutureOrOutput<F: Future> {
    Future(F),
    Output(F::Output),
    Taken,
}
impl<F: Future> FutureOrOutput<F> {
    pub fn take_output(&mut self) -> F::Output {
        match std::mem::replace(self, Self::Taken) {
            FutureOrOutput::Output(o) => o,
            _ => unreachable!(),
        }
    }
}

/// A future that awaits on all `futures` at the same time and returns all results when all futures are ready.
///
/// This is the dynamic version of [`all!`].
pub async fn all<F: IntoFuture>(futures: impl IntoIterator<Item = F>) -> Vec<F::Output> {
    let mut futures: Vec<_> = futures.into_iter().map(|f| FutureOrOutput::Future(f.into_future())).collect();
    future_fn(move |cx| {
        let mut pending = false;
        for input in &mut futures {
            if let FutureOrOutput::Future(fut) = input {
                // SAFETY: the closure owns $ident and is an exclusive borrow inside a
                // Future::poll call, so it will not move.
                let mut fut_mut = unsafe { std::pin::Pin::new_unchecked(fut) };
                if let Poll::Ready(r) = fut_mut.as_mut().poll(cx) {
                    *input = FutureOrOutput::Output(r);
                } else {
                    pending = true;
                }
            }
        }

        if pending {
            Poll::Pending
        } else {
            Poll::Ready(futures.iter_mut().map(FutureOrOutput::take_output).collect())
        }
    })
    .await
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
/// Each input must implement [`IntoFuture`] with the same `Output` type. Note that each input must be
/// known at compile time, use the [`fn@any`] async function to await on all futures in a dynamic list of futures.
///
/// # Examples
///
/// Await for the first of three futures to complete:
///
/// ```
/// use zng_task as task;
/// use zng_unit::*;
///
/// # task::doc_test(false, async {
/// let r = task::any!(
///     task::run(async {
///         task::deadline(300.ms()).await;
///         'a'
///     }),
///     task::wait(|| 'b'),
///     async {
///         task::deadline(300.ms()).await;
///         'c'
///     }
/// )
/// .await;
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
    ($($fut:expr),+ $(,)?) => { $crate::__proc_any_all!{ $crate::__any; $($fut),+ } }
}
#[doc(hidden)]
#[macro_export]
macro_rules! __any {
    ($($ident:ident: $fut:expr;)+) => {
        {
            $(let mut $ident = std::future::IntoFuture::into_future($fut);)+
            $crate::future_fn(move |cx| {
                use std::task::Poll;
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
pub use zng_task_proc_macros::task_any_all as __proc_any_all;

/// A future that awaits on all `futures` at the same time and returns the first result when the first future is ready.
///
/// This is the dynamic version of [`any!`].
pub async fn any<F: IntoFuture>(futures: impl IntoIterator<Item = F>) -> F::Output {
    let mut futures: Vec<_> = futures.into_iter().map(IntoFuture::into_future).collect();
    future_fn(move |cx| {
        for fut in &mut futures {
            // SAFETY: the closure owns $ident and is an exclusive borrow inside a
            // Future::poll call, so it will not move.
            let mut fut_mut = unsafe { std::pin::Pin::new_unchecked(fut) };
            if let Poll::Ready(r) = fut_mut.as_mut().poll(cx) {
                return Poll::Ready(r);
            }
        }
        Poll::Pending
    })
    .await
}

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
/// Each input must implement [`IntoFuture`] with the same `Output` type. Note that each input must be
/// known at compile time, use the [`fn@any_ok`] async function to await on all futures in a dynamic list of futures.
///
/// # Examples
///
/// Await for the first of three futures to complete with `Ok`:
///
/// ```
/// use zng_task as task;
/// # #[derive(Debug, PartialEq)]
/// # pub struct FooError;
/// # task::doc_test(false, async {
/// let r = task::any_ok!(
///     task::run(async { Err::<char, _>("error") }),
///     task::wait(|| Ok::<_, FooError>('b')),
///     async { Err::<char, _>(FooError) }
/// )
/// .await;
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
    ($($fut:expr),+ $(,)?) => { $crate::__proc_any_all!{ $crate::__any_ok; $($fut),+ } }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __any_ok {
    ($($ident:ident: $fut: expr;)+) => {
        {
            $(let mut $ident = $crate::FutureOrOutput::Future(std::future::IntoFuture::into_future($fut));)+
            $crate::future_fn(move |cx| {
                use std::task::Poll;

                let mut pending = false;

                $(
                    if let $crate::FutureOrOutput::Future(fut) = &mut $ident {
                        // SAFETY: the closure owns $ident and is an exclusive borrow inside a
                        // Future::poll call, so it will not move.
                        let mut fut = unsafe { std::pin::Pin::new_unchecked(fut) };
                        if let Poll::Ready(r) = fut.as_mut().poll(cx) {
                            match r {
                                Ok(r) => return Poll::Ready(Ok(r)),
                                Err(e) => {
                                    $ident = $crate::FutureOrOutput::Output(Err(e));
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
                        $($ident.take_output().unwrap_err()),+
                    )))
                }
            })
        }
    }
}

/// A future that awaits on all `futures` at the same time and returns when any future is `Ok(_)` or all are `Err(_)`.
///
/// This is the dynamic version of [`all_some!`].
pub async fn any_ok<Ok, Err, F: IntoFuture<Output = Result<Ok, Err>>>(futures: impl IntoIterator<Item = F>) -> Result<Ok, Vec<Err>> {
    let mut futures: Vec<_> = futures.into_iter().map(|f| FutureOrOutput::Future(f.into_future())).collect();
    future_fn(move |cx| {
        let mut pending = false;
        for input in &mut futures {
            if let FutureOrOutput::Future(fut) = input {
                // SAFETY: the closure owns $ident and is an exclusive borrow inside a
                // Future::poll call, so it will not move.
                let mut fut_mut = unsafe { std::pin::Pin::new_unchecked(fut) };
                if let Poll::Ready(r) = fut_mut.as_mut().poll(cx) {
                    match r {
                        Ok(r) => return Poll::Ready(Ok(r)),
                        Err(e) => *input = FutureOrOutput::Output(Err(e)),
                    }
                } else {
                    pending = true;
                }
            }
        }

        if pending {
            Poll::Pending
        } else {
            Poll::Ready(Err(futures
                .iter_mut()
                .map(|f| match f.take_output() {
                    Ok(_) => unreachable!(),
                    Err(e) => e,
                })
                .collect()))
        }
    })
    .await
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
/// Each input must implement [`IntoFuture`] with the same `Output` type. Note that each input must be
/// known at compile time, use the [`fn@any_some`] async function to await on all futures in a dynamic list of futures.
///
/// # Examples
///
/// Await for the first of three futures to complete with `Some`:
///
/// ```
/// use zng_task as task;
/// # task::doc_test(false, async {
/// let r = task::any_some!(task::run(async { None::<char> }), task::wait(|| Some('b')), async { None::<char> }).await;
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
    ($($fut:expr),+ $(,)?) => { $crate::__proc_any_all!{ $crate::__any_some; $($fut),+ } }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __any_some {
    ($($ident:ident: $fut: expr;)+) => {
        {
            $(let mut $ident = Some(std::future::IntoFuture::into_future($fut));)+
            $crate::future_fn(move |cx| {
                use std::task::Poll;

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

/// A future that awaits on all `futures` at the same time and returns when any future is `Some(_)` or all are `None`.
///
/// This is the dynamic version of [`all_some!`].
pub async fn any_some<Some, F: IntoFuture<Output = Option<Some>>>(futures: impl IntoIterator<Item = F>) -> Option<Some> {
    let mut futures: Vec<_> = futures.into_iter().map(|f| Some(f.into_future())).collect();
    future_fn(move |cx| {
        let mut pending = false;
        for input in &mut futures {
            if let Some(fut) = input {
                // SAFETY: the closure owns $ident and is an exclusive borrow inside a
                // Future::poll call, so it will not move.
                let mut fut_mut = unsafe { std::pin::Pin::new_unchecked(fut) };
                if let Poll::Ready(r) = fut_mut.as_mut().poll(cx) {
                    match r {
                        Some(r) => return Poll::Ready(Some(r)),
                        None => *input = None,
                    }
                } else {
                    pending = true;
                }
            }
        }

        if pending { Poll::Pending } else { Poll::Ready(None) }
    })
    .await
}

/// <span data-del-macro-root></span> A future that is ready when all futures are ready with an `Ok(T)` result or
/// any future is ready with an `Err(E)` result.
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
/// Each input must implement [`IntoFuture`] with the same `Output` type. Note that each input must be
/// known at compile time, use the [`fn@all_ok`] async function to await on all futures in a dynamic list of futures.
///
/// # Examples
///
/// Await for the first of three futures to complete with `Ok(T)`:
///
/// ```
/// use zng_task as task;
/// # #[derive(Debug, PartialEq)]
/// # struct FooError;
/// # task::doc_test(false, async {
/// let r = task::all_ok!(
///     task::run(async { Ok::<_, FooError>('a') }),
///     task::wait(|| Ok::<_, FooError>('b')),
///     async { Ok::<_, FooError>('c') }
/// )
/// .await;
///
/// assert_eq!(Ok(('a', 'b', 'c')), r);
/// # });
/// ```
///
/// And in if any completes with `Err(E)`:
///
/// ```
/// use zng_task as task;
/// # #[derive(Debug, PartialEq)]
/// # struct FooError;
/// # task::doc_test(false, async {
/// let r = task::all_ok!(task::run(async { Ok('a') }), task::wait(|| Err::<char, _>(FooError)), async {
///     Ok('c')
/// })
/// .await;
///
/// assert_eq!(Err(FooError), r);
/// # });
/// ```
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
    ($($fut:expr),+ $(,)?) => { $crate::__proc_any_all!{ $crate::__all_ok; $($fut),+ } }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __all_ok {
    ($($ident:ident: $fut: expr;)+) => {
        {
            $(let mut $ident = $crate::FutureOrOutput::Future(std::future::IntoFuture::into_future($fut));)+
            $crate::future_fn(move |cx| {
                use std::task::Poll;

                let mut pending = false;

                $(
                    if let $crate::FutureOrOutput::Future(fut) = &mut $ident {
                        // SAFETY: the closure owns $ident and is an exclusive borrow inside a
                        // Future::poll call, so it will not move.
                        let mut fut = unsafe { std::pin::Pin::new_unchecked(fut) };
                        if let Poll::Ready(r) = fut.as_mut().poll(cx) {
                            match r {
                                Ok(r) => {
                                    $ident = $crate::FutureOrOutput::Output(Ok(r))
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
                        $($ident.take_output().unwrap()),+
                    )))
                }
            })
        }
    }
}

/// A future that awaits on all `futures` at the same time and returns when all futures are `Ok(_)` or any future is `Err(_)`.
///
/// This is the dynamic version of [`all_ok!`].
pub async fn all_ok<Ok, Err, F: IntoFuture<Output = Result<Ok, Err>>>(futures: impl IntoIterator<Item = F>) -> Result<Vec<Ok>, Err> {
    let mut futures: Vec<_> = futures.into_iter().map(|f| FutureOrOutput::Future(f.into_future())).collect();
    future_fn(move |cx| {
        let mut pending = false;
        for input in &mut futures {
            if let FutureOrOutput::Future(fut) = input {
                // SAFETY: the closure owns $ident and is an exclusive borrow inside a
                // Future::poll call, so it will not move.
                let mut fut_mut = unsafe { std::pin::Pin::new_unchecked(fut) };
                if let Poll::Ready(r) = fut_mut.as_mut().poll(cx) {
                    match r {
                        Ok(r) => *input = FutureOrOutput::Output(Ok(r)),
                        Err(e) => return Poll::Ready(Err(e)),
                    }
                } else {
                    pending = true;
                }
            }
        }

        if pending {
            Poll::Pending
        } else {
            Poll::Ready(Ok(futures
                .iter_mut()
                .map(|f| f.take_output().unwrap_or_else(|_| unreachable!()))
                .collect()))
        }
    })
    .await
}

/// <span data-del-macro-root></span> A future that is ready when all futures are ready with `Some(T)` or when any
/// is future ready with `None`.
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
/// Each input must implement [`IntoFuture`] with the same `Output` type. Note that each input must be
/// known at compile time, use the [`fn@all_some`] async function to await on all futures in a dynamic list of futures.
///
/// # Examples
///
/// Await for the first of three futures to complete with `Some`:
///
/// ```
/// use zng_task as task;
/// # task::doc_test(false, async {
/// let r = task::all_some!(task::run(async { Some('a') }), task::wait(|| Some('b')), async { Some('c') }).await;
///
/// assert_eq!(Some(('a', 'b', 'c')), r);
/// # });
/// ```
///
/// Completes with `None` if any future completes with `None`:
///
/// ```
/// # use zng_task as task;
/// # task::doc_test(false, async {
/// let r = task::all_some!(task::run(async { Some('a') }), task::wait(|| None::<char>), async { Some('b') }).await;
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
    ($($fut:expr),+ $(,)?) => { $crate::__proc_any_all!{ $crate::__all_some; $($fut),+ } }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __all_some {
    ($($ident:ident: $fut: expr;)+) => {
        {
            $(let mut $ident = $crate::FutureOrOutput::Future(std::future::IntoFuture::into_future($fut));)+
            $crate::future_fn(move |cx| {
                use std::task::Poll;

                let mut pending = false;

                $(
                    if let $crate::FutureOrOutput::Future(fut) = &mut $ident {
                        // SAFETY: the closure owns $ident and is an exclusive borrow inside a
                        // Future::poll call, so it will not move.
                        let mut fut = unsafe { std::pin::Pin::new_unchecked(fut) };
                        if let Poll::Ready(r) = fut.as_mut().poll(cx) {
                            if r.is_none() {
                                return Poll::Ready(None);
                            }

                            $ident = $crate::FutureOrOutput::Output(r);
                        } else {
                            pending = true;
                        }
                    }
                )+

                if pending {
                    Poll::Pending
                } else {
                    Poll::Ready(Some((
                        $($ident.take_output().unwrap()),+
                    )))
                }
            })
        }
    }
}

/// A future that awaits on all `futures` at the same time and returns when all futures are `Some(_)` or any future is `None`.
///
/// This is the dynamic version of [`all_some!`].
pub async fn all_some<Some, F: IntoFuture<Output = Option<Some>>>(futures: impl IntoIterator<Item = F>) -> Option<Vec<Some>> {
    let mut futures: Vec<_> = futures.into_iter().map(|f| FutureOrOutput::Future(f.into_future())).collect();
    future_fn(move |cx| {
        let mut pending = false;
        for input in &mut futures {
            if let FutureOrOutput::Future(fut) = input {
                // SAFETY: the closure owns $ident and is an exclusive borrow inside a
                // Future::poll call, so it will not move.
                let mut fut_mut = unsafe { std::pin::Pin::new_unchecked(fut) };
                if let Poll::Ready(r) = fut_mut.as_mut().poll(cx) {
                    match r {
                        Some(r) => *input = FutureOrOutput::Output(Some(r)),
                        None => return Poll::Ready(None),
                    }
                } else {
                    pending = true;
                }
            }
        }

        if pending {
            Poll::Pending
        } else {
            Poll::Ready(Some(futures.iter_mut().map(|f| f.take_output().unwrap()).collect()))
        }
    })
    .await
}

/// A future that will await until [`set`] is called.
///
/// # Examples
///
/// Spawns a parallel task that only writes to stdout after the main thread sets the signal:
///
/// ```
/// use zng_clone_move::async_clmv;
/// use zng_task::{self as task, *};
///
/// let signal = SignalOnce::default();
///
/// task::spawn(async_clmv!(signal, {
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
impl PartialEq for SignalOnce {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for SignalOnce {}
impl Hash for SignalOnce {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state)
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
        if self.0.push(waker) { Some(self.0.clone().into()) } else { None }
    }

    /// Clear current registered wakers.
    pub fn cancel(&self) {
        self.0.cancel()
    }
}
