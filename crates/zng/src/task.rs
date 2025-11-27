//! Parallel async tasks and async task runners.
//!
//! Use [`run`], [`respond`] or [`spawn`] to run parallel tasks, use [`wait`], [`io`] and [`fs`] to unblock
//! IO operations and use [`http`] for async HTTP.
//!
//! All functions of this module propagate the [`LocalContext`].
//!
//! This crate also re-exports the [`rayon`] and [`parking_lot`] crates for convenience. You can use the
//! [`ParallelIteratorExt::with_ctx`] adapter in rayon iterators to propagate the [`LocalContext`]. You can
//! also use [`join`] to propagate thread context for a raw rayon join operation.
//!
//! # Examples
//!
//! ```
//! use zng::prelude::*;
//!
//! # fn example() {
//! let enabled = var(false);
//! # let _ =
//! Button! {
//!     on_click = async_hn!(enabled, |_| {
//!         enabled.set(false);
//!
//!         let sum_task = task::run(async {
//!             let numbers = read_numbers().await;
//!             numbers.par_iter().map(|i| i * i).sum()
//!         });
//!         let sum: usize = sum_task.await;
//!         println!("sum of squares: {sum}");
//!
//!         enabled.set(true);
//!     });
//!     widget::enabled = enabled;
//! }
//! # ; }
//!
//! async fn read_numbers() -> Vec<usize> {
//!     let raw = task::wait(|| std::fs::read_to_string("numbers.txt").unwrap()).await;
//!     raw.par_split(',').map(|s| s.trim().parse::<usize>().unwrap()).collect()
//! }
//! ```
//!
//! The example demonstrates three different ***tasks***, the first is a [`UiTask`] in the `async_hn` handler,
//! this task is *async* but not *parallel*, meaning that it will execute in more then one app update, but it will only execute in the
//! `on_click` context and thread. This is good for coordinating UI state, like setting variables, but is not good if you want to do CPU intensive work.
//!
//! To keep the app responsive we move the computation work inside a [`run`] task, this task is *async* and *parallel*,
//! meaning it can `.await` and will execute in parallel threads. It runs in [`rayon`] so you can
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
//! You can use [`wait`], [`io`] and [`fs`] to do async IO, Zng uses this API for internal async IO, they are just a selection
//! of external async crates re-exported for convenience and compatibility.
//!
//! The [`io`] module just re-exports the [`futures-lite::io`] traits and types, adding only progress tracking. The
//! [`fs`] module is the [`async-fs`] crate. Most of the IO async operations are implemented using extensions traits
//! so we recommend blob importing [`io`] to start implementing async IO.
//!
//! ```
//! use zng::prelude::*;
//!
//! async fn read_numbers() -> Result<Vec<usize>, Box<dyn std::error::Error + Send + Sync>> {
//!     let mut file = task::fs::File::open("numbers.txt").await?;
//!     let mut raw = String::new();
//!     file.read_to_string(&mut raw).await?;
//!     raw.par_split(',').map(|s| s.trim().parse::<usize>().map_err(Into::into)).collect()
//! }
//! ```
//!
//! All the `std::fs` synchronous operations have an async counterpart in [`fs`]. For simpler one shot
//! operation it is recommended to just use `std::fs` inside [`wait`], the async [`fs`] types are not async at
//! the OS level, they only offload operations inside the same thread-pool used by [`wait`].
//!
//! # HTTP Client
//!
//! You can use [`http`] to implement asynchronous HTTP requests. Zng also uses the [`http`] module for
//! implementing operations such as loading an image from a given URL, the module is a thin wrapper around the [`isahc`] crate.
//!
//! ```
//! use zng::prelude::*;
//!
//! # fn example() {
//! let enabled = var(false);
//! let msg = var("loading..".to_txt());
//! # let _ =
//! Button! {
//!     on_click = async_hn!(enabled, msg, |_| {
//!         enabled.set(false);
//!
//!         match task::http::get_txt("https://httpbin.org/get").await {
//!             Ok(r) => msg.set(r),
//!             Err(e) => msg.set(formatx!("error: {e}")),
//!         }
//!
//!         enabled.set(true);
//!     });
//! }
//! # ; }
//! ```
//!
//! For other protocols or alternative HTTP clients you can use [external crates](#async-crates-integration).
//!
//! # Async Crates Integration
//!
//! You can use external async crates to create futures and then `.await` then in async code managed by Zng, but there is some
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
//! External tasks also don't propagate the thread context, if you want access to app services or want to set vars inside external
//! parallel closures you must capture and load the [`LocalContext`] manually.
//!
//! [`LocalContext`]: crate::app::LocalContext
//! [`isahc`]: https://docs.rs/isahc
//! [`AppExtension`]: crate::app::AppExtension
//! [`blocking`]: https://docs.rs/blocking
//! [`futures`]: https://docs.rs/futures
//! [`async-std`]: https://docs.rs/async-std
//! [`smol`]: https://docs.rs/smol
//! [`tokio`]: https://docs.rs/tokio
//! [`futures-lite::io`]: https://docs.rs/futures-lite/*/futures_lite/io/index.html
//! [`async-fs`]: https://docs.rs/async-fs
//! [`rayon`]: https://docs.rs/rayon
//! [`parking_lot`]: https://docs.rs/parking_lot
//!
//! # Full API
//!
//! See [`zng_task`] for the full API.

pub use zng_task::{
    DeadlineError, McWaker, ParallelIteratorExt, ParallelIteratorWithCtx, Progress, ScopeCtx, SignalOnce, TaskPanicError, UiTask, all,
    all_ok, all_some, any, any_ok, any_some, block_on, channel, deadline, fs, future_fn, io, join, join_context, poll_respond, poll_spawn,
    respond, run, run_catch, scope, set_spawn_panic_handler, spawn, spawn_wait, wait, wait_catch, wait_respond, with_deadline, yield_now,
};

#[cfg(any(doc, feature = "test_util"))]
pub use zng_task::{doc_test, spin_on};

/// HTTP client.
///
/// This module provides an HTTP client API that is backend agnostic. By default it uses the system `curl` command
/// line utility with a simple cache, this can be replaced using the full API.
///
/// # Examples
///
/// Get some text:
///
/// ```
/// # use zng::task;
/// # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
/// let text = task::http::get_txt("https://httpbin.org/base64/SGVsbG8gV29ybGQ=").await?;
/// println!("{text}!");
/// # Ok(()) }
/// ``
/// # Full API
///
/// See [`zng_task::http`] for the full API.
#[cfg(feature = "http")]
pub mod http {
    pub use zng_task::http::{
        Error, Method, Request, Response, StatusCode, Uri, delete, get, get_bytes, get_json, get_txt, head, header, method, post, put,
        send, uri,
    };

    /// Remove all cached entries, or just older ones if `prune` is enabled.
    pub async fn cache_clean(prune: bool) {
        if prune {
            zng_task::http::http_cache().prune().await;
        } else {
            zng_task::http::http_cache().purge().await;
        }
    }
}

#[cfg(ipc)]
pub use zng_task::process;

#[doc(no_inline)]
pub use zng_task::{parking_lot, rayon};

pub use zng_app::widget::UiTaskWidget;
