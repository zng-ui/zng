#![cfg(ipc)]

//! Worker process tasks.
//!
//! This module defines a worker process that can run tasks in a separate process instance.
//!
//! Each worker process can run multiple tasks in parallel, the worker type is [`Worker`]. Note that this module does not offer a fork
//! implementation, the worker processes begin from the start state. The primary use of process tasks is to make otherwise fatal tasks
//! recoverable, if the task calls unsafe code or code that can potentially terminate the entire process it should run using a [`Worker`].
//! If you only want to recover from panics in safe code consider using [`task::run_catch`] or [`task::wait_catch`] instead.
//!
//! You can send [IPC channel] endpoints in the task request messages, this can be useful for implementing progress reporting,
//! you can also send [`IpcBytes`] to efficiently share large byte blobs with the worker process.
//!
//! [`task::run_catch`]: crate::run_catch
//! [`task::wait_catch`]: crate::wait_catch
//! [IPC channel]: crate::channel::ipc_unbounded
//! [`IpcBytes`]: crate::channel::IpcBytes
//!
//! # Examples
//!
//! The example below demonstrates a worker-process setup that uses the same executable as the app-process.
//!
//! ```
//! # fn main() { }
//! # mod zng { pub mod env { pub use zng_env::*; } pub mod task { pub use zng_task::*; } }
//! # fn demo() {
//! fn main() {
//!     zng::env::init!();
//!     // normal app init..
//!     # zng::task::doc_test(false, on_click());
//! }
//! # }
//!
//! mod task1 {
//! # use super::zng;
//!     use zng::{task, env};
//!
//!     const NAME: &str = "zng::example::task1";
//!
//!     env::on_process_start!(|args| {
//!         // give tracing handlers a chance to observe the worker-process
//!         if args.yield_count == 0 { return args.yield_once(); }
//          // run the worker server
//!         task::process::run_worker(NAME, work);
//!     });
//!     async fn work(args: task::process::RequestArgs<Request>) -> Response {
//!         let rsp = format!("received 'task1' request `{:?}` in worker-process #{}", &args.request.data, std::process::id());
//!         Response { data: rsp }
//!     }
//!     
//!     #[derive(Debug, serde::Serialize, serde::Deserialize)]
//!     pub struct Request { pub data: String }
//!
//!     #[derive(Debug, serde::Serialize, serde::Deserialize)]
//!     pub struct Response { pub data: String }
//!
//!     // called in app-process
//!     pub async fn start() -> task::process::Worker<Request, Response> {
//!         task::process::Worker::start(NAME).await.expect("cannot spawn 'task1'")
//!     }
//! }
//!
//! // This runs in the app-process, it starts a worker process and requests a task run.
//! async fn on_click() {
//!     println!("app-process #{} starting a worker", std::process::id());
//!     let mut worker = task1::start().await;
//!     // request a task run and await it.
//!     match worker.run(task1::Request { data: "request".to_owned() }).await {
//!         Ok(task1::Response { data }) => println!("ok. {data}"),
//!         Err(e) => eprintln!("error: {e}"),
//!     }
//!     // multiple tasks can be requested in parallel, use `task::all!` to await ..
//!
//!     // the worker process can be gracefully shutdown, awaits all pending tasks.
//!     let _ = worker.shutdown().await;
//! }
//! ```
//!
//! Note that you can setup multiple workers the same executable, as long as the `on_process_start!` call happens
//! on different modules.
//!
//! # Connect Timeout
//!
//! If the worker process takes longer than 10 seconds to connect the tasks fails. This is more then enough in most cases, but
//! it can be too little in some test runner machines. You can set the `"ZNG_TASK_WORKER_TIMEOUT"` environment variable to a custom
//! timeout in seconds. The minimum value is 1 second, set to 0 or empty use the default timeout.

use core::fmt;
use std::{marker::PhantomData, path::PathBuf, pin::Pin, sync::Arc};

use parking_lot::Mutex;
use zng_clone_move::{async_clmv, clmv};
use zng_txt::Txt;
use zng_unique_id::IdMap;
use zng_unit::TimeUnits as _;

use crate::{
    TaskPanicError,
    channel::{ChannelError, IpcSender, IpcValue},
};

const WORKER_VERSION: &str = "ZNG_TASK_IPC_WORKER_VERSION";
const WORKER_SERVER: &str = "ZNG_TASK_IPC_WORKER_SERVER";
const WORKER_NAME: &str = "ZNG_TASK_IPC_WORKER_NAME";

const WORKER_TIMEOUT: &str = "ZNG_TASK_WORKER_TIMEOUT";

/// The *App Process* and *Worker Process* must be build using the same exact version and this is
/// validated during run-time, causing a panic if the versions don't match.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Represents a running worker process.
pub struct Worker<I: IpcValue, O: IpcValue> {
    running: Option<(std::thread::JoinHandle<()>, std::process::Child)>,

    sender: IpcSender<(RequestId, Request<I>)>,
    requests: Arc<Mutex<IdMap<RequestId, flume::Sender<O>>>>,

    _p: PhantomData<fn(I) -> O>,

    crash: Option<WorkerCrashError>,
}
impl<I: IpcValue, O: IpcValue> Worker<I, O> {
    /// Start a worker process implemented in the current executable.
    ///
    /// Note that the current process must call [`run_worker`] at startup to actually work.
    /// You can use [`zng_env::on_process_start!`] to inject startup code.
    pub async fn start(worker_name: impl Into<Txt>) -> std::io::Result<Self> {
        Self::start_impl(worker_name.into(), std::env::current_exe()?, &[], &[]).await
    }

    /// Start a worker process implemented in the current executable with custom env vars and args.
    pub async fn start_with(worker_name: impl Into<Txt>, env_vars: &[(&str, &str)], args: &[&str]) -> std::io::Result<Self> {
        Self::start_impl(worker_name.into(), std::env::current_exe()?, env_vars, args).await
    }

    /// Start a worker process implemented in another executable with custom env vars and args.
    pub async fn start_other(
        worker_name: impl Into<Txt>,
        worker_exe: impl Into<PathBuf>,
        env_vars: &[(&str, &str)],
        args: &[&str],
    ) -> std::io::Result<Self> {
        Self::start_impl(worker_name.into(), worker_exe.into(), env_vars, args).await
    }

    async fn start_impl(worker_name: Txt, exe: PathBuf, env_vars: &[(&str, &str)], args: &[&str]) -> std::io::Result<Self> {
        let (server, name) = ipc_channel::ipc::IpcOneShotServer::<WorkerInit<I, O>>::new()?;

        let mut worker = std::process::Command::new(dunce::canonicalize(exe)?);
        for (key, value) in env_vars {
            worker.env(key, value);
        }
        for arg in args {
            worker.arg(arg);
        }

        worker
            .env(WORKER_VERSION, crate::process::VERSION)
            .env(WORKER_SERVER, name)
            .env(WORKER_NAME, worker_name)
            .env("RUST_BACKTRACE", "full");

        let mut worker = crate::wait(move || worker.spawn()).await?;

        let timeout = match std::env::var(WORKER_TIMEOUT) {
            Ok(t) if !t.is_empty() => match t.parse::<u64>() {
                Ok(t) => t.max(1),
                Err(e) => {
                    tracing::error!("invalid {WORKER_TIMEOUT:?} value, {e}");
                    10
                }
            },
            _ => 10,
        };

        let r = crate::with_deadline(crate::wait(move || server.accept()), timeout.secs()).await;

        let (_, (req_sender, mut chan_sender)) = match r {
            Ok(r) => match r {
                Ok(r) => r,
                Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::ConnectionRefused, e)),
            },
            Err(_) => {
                let cleanup = crate::wait(move || {
                    worker.kill()?;
                    worker.wait()
                });
                match cleanup.await {
                    Ok(status) => {
                        let code = status.code().unwrap_or(0);
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            format!("worker process did not connect in {timeout}s\nworker exit code: {code}"),
                        ));
                    }
                    Err(e) => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            format!("worker process did not connect in {timeout}s\ncannot be kill worker process, {e}"),
                        ));
                    }
                }
            }
        };

        let (rsp_sender, mut rsp_recv) = crate::channel::ipc_unbounded()?;
        crate::wait(move || chan_sender.send_blocking(rsp_sender)).await.unwrap();

        let requests = Arc::new(Mutex::new(IdMap::<RequestId, flume::Sender<O>>::new()));
        let receiver = std::thread::Builder::new()
            .name("task-ipc-recv".into())
            .stack_size(256 * 1024)
            .spawn(clmv!(requests, || {
                loop {
                    match rsp_recv.recv_blocking() {
                        Ok((id, r)) => match requests.lock().remove(&id) {
                            Some(s) => match r {
                                Response::Out(r) => {
                                    let _ = s.send(r);
                                }
                            },
                            None => tracing::error!("worker responded to unknown request #{}", id.sequential()),
                        },
                        Err(e) => match e {
                            ChannelError::Disconnected { .. } => {
                                requests.lock().clear();
                                break;
                            }
                            e => {
                                tracing::error!("worker response error, will shutdown, {e}");
                                break;
                            }
                        },
                    }
                }
            }))
            .expect("failed to spawn thread");

        Ok(Self {
            running: Some((receiver, worker)),
            sender: req_sender,
            _p: PhantomData,
            crash: None,
            requests,
        })
    }

    /// Awaits current tasks and kills the worker process.
    pub async fn shutdown(mut self) -> std::io::Result<()> {
        if let Some((receiver, mut process)) = self.running.take() {
            while !self.requests.lock().is_empty() {
                crate::deadline(100.ms()).await;
            }
            let r = crate::wait(move || process.kill()).await;

            match crate::with_deadline(crate::wait(move || receiver.join()), 1.secs()).await {
                Ok(r) => {
                    if let Err(p) = r {
                        tracing::error!(
                            "worker receiver thread exited panicked, {}",
                            TaskPanicError::new(p).panic_str().unwrap_or("")
                        );
                    }
                }
                Err(_) => {
                    // timeout
                    if r.is_ok() {
                        // after awaiting kill receiver thread should join fast because disconnect breaks loop
                        panic!("worker receiver thread did not exit after worker process did");
                    }
                }
            }
            r
        } else {
            Ok(())
        }
    }

    /// Run a task in a free worker process thread.
    pub fn run(&mut self, input: I) -> impl Future<Output = Result<O, RunError>> + Send + 'static {
        self.run_request(Request::Run(input))
    }

    fn run_request(&mut self, request: Request<I>) -> Pin<Box<dyn Future<Output = Result<O, RunError>> + Send + 'static>> {
        if self.crash_error().is_some() {
            return Box::pin(std::future::ready(Err(RunError::Disconnected)));
        }

        let id = RequestId::new_unique();
        let (sx, rx) = flume::bounded(1);

        let requests = self.requests.clone();
        requests.lock().insert(id, sx);
        let mut sender = self.sender.clone();
        let send_r = crate::wait(move || sender.send_blocking((id, request)));

        Box::pin(async move {
            if let Err(e) = send_r.await {
                requests.lock().remove(&id);
                return Err(RunError::Other(Arc::new(e)));
            }

            match rx.recv_async().await {
                Ok(r) => Ok(r),
                Err(e) => match e {
                    flume::RecvError::Disconnected => {
                        requests.lock().remove(&id);
                        Err(RunError::Disconnected)
                    }
                },
            }
        })
    }

    /// Reference the crash error.
    ///
    /// The worker cannot be used if this is set, run requests will immediately disconnect.
    pub fn crash_error(&mut self) -> Option<&WorkerCrashError> {
        if let Some((t, _)) = &self.running
            && t.is_finished()
        {
            let (t, mut p) = self.running.take().unwrap();

            if let Err(e) = t.join() {
                tracing::error!(
                    "panic in worker receiver thread, {}",
                    TaskPanicError::new(e).panic_str().unwrap_or("")
                );
            }

            if let Err(e) = p.kill() {
                tracing::error!("error killing worker process after receiver exit, {e}");
            }

            match p.wait() {
                Ok(o) => {
                    self.crash = Some(WorkerCrashError { status: o });
                }
                Err(e) => tracing::error!("error reading crashed worker output, {e}"),
            }
        }

        self.crash.as_ref()
    }
}
impl<I: IpcValue, O: IpcValue> Drop for Worker<I, O> {
    fn drop(&mut self) {
        if let Some((receiver, mut process)) = self.running.take() {
            if !receiver.is_finished() {
                tracing::error!("dropped worker without shutdown");
            }
            if let Err(e) = process.kill() {
                tracing::error!("failed to kill worker process on drop, {e}");
            }
        }
    }
}

/// If the process was started by a [`Worker`] runs the worker loop and never returns. If
/// not started as worker does nothing.
///
/// The `handler` is called for each work request.
pub fn run_worker<I, O, F>(worker_name: impl Into<Txt>, handler: impl Fn(RequestArgs<I>) -> F + Send + Sync + 'static)
where
    I: IpcValue,
    O: IpcValue,
    F: Future<Output = O> + Send + Sync + 'static,
{
    let name = worker_name.into();
    if let Some(server_name) = run_worker_server(&name) {
        zng_env::init_process_name(zng_txt::formatx!("worker-process ({name}, {})", std::process::id()));

        let app_init_sender = ipc_channel::ipc::IpcSender::<WorkerInit<I, O>>::connect(server_name)
            .unwrap_or_else(|e| panic!("failed to connect to '{name}' init channel, {e}"));

        let (req_sender, mut req_recv) = crate::channel::ipc_unbounded().unwrap();
        let (chan_sender, mut chan_recv) = crate::channel::ipc_unbounded().unwrap();

        app_init_sender.send((req_sender, chan_sender)).unwrap();
        let rsp_sender = chan_recv.recv_blocking().unwrap();
        let handler = Arc::new(handler);

        loop {
            match req_recv.recv_blocking() {
                Ok((id, input)) => match input {
                    Request::Run(r) => crate::spawn(async_clmv!(handler, mut rsp_sender, {
                        let output = handler(RequestArgs { request: r }).await;
                        let _ = rsp_sender.send_blocking((id, Response::Out(output)));
                    })),
                },
                Err(e) => match e {
                    ChannelError::Disconnected { .. } => break,
                    ChannelError::Timeout => unreachable!(),
                },
            }
        }

        zng_env::exit(0);
    }
}
fn run_worker_server(worker_name: &str) -> Option<String> {
    if let Ok(w_name) = std::env::var(WORKER_NAME)
        && let Ok(version) = std::env::var(WORKER_VERSION)
        && let Ok(server_name) = std::env::var(WORKER_SERVER)
    {
        if w_name != worker_name {
            return None;
        }
        if version != VERSION {
            eprintln!("worker '{worker_name}' API version is not equal, app-process: {version}, worker-process: {VERSION}");
            zng_env::exit(i32::from_le_bytes(*b"vapi"));
        }

        Some(server_name)
    } else {
        None
    }
}

/// Arguments for [`run_worker`].
#[non_exhaustive]
pub struct RequestArgs<I: IpcValue> {
    /// The task request data.
    pub request: I,
}

/// Worker run error.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum RunError {
    /// Lost connection with the worker process.
    ///
    /// See [`Worker::crash_error`] for the error.
    Disconnected,
    /// Other error.
    Other(Arc<dyn std::error::Error + Send + Sync>),
}
impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunError::Disconnected => write!(f, "worker process disconnected"),
            RunError::Other(e) => write!(f, "run error, {e}"),
        }
    }
}
impl std::error::Error for RunError {}

/// Info about a worker process crash.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct WorkerCrashError {
    /// Worker process exit code.
    pub status: std::process::ExitStatus,
}
impl fmt::Display for WorkerCrashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.status)
    }
}
impl std::error::Error for WorkerCrashError {}

#[derive(serde::Serialize, serde::Deserialize)]
enum Request<I> {
    Run(I),
}

#[derive(serde::Serialize, serde::Deserialize)]
enum Response<O> {
    Out(O),
}

/// Large messages can only be received in a receiver created in the same process that is receiving (on Windows)
/// so we create a channel to transfer the response sender.
/// See issue: https://github.com/servo/ipc-channel/issues/277
///
/// (
///    RequestSender,
///    Workaround-sender-for-response-channel,
/// )
type WorkerInit<I, O> = (IpcSender<(RequestId, Request<I>)>, IpcSender<IpcSender<(RequestId, Response<O>)>>);

zng_unique_id::unique_id_64! {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct RequestId;
}
