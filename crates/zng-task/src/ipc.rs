#![cfg(feature = "ipc")]

//! IPC tasks.
//!
//! This module uses [`ipc_channel`] and [`duct`] crates to define a worker process that can run tasks in a separate process instance.
//!
//! Each worker process can run multiple tasks in parallel, the worker type is [`Worker`]. Note that this module does not offer a fork
//! implementation, the worker processes begin from the start state. The primary use of process tasks is to make otherwise fatal tasks
//! recoverable, if the task calls unsafe code or code that can potentially terminate the entire process it should run using a [`Worker`].
//! If you only want to recover from panics in safe code consider using [`task::run_catch`] or [`task::wait_catch`] instead.
//!
//! This module also re-exports some [`ipc_channel`] types and functions. You can send IPC channels in the task request messages, this
//! can be useful for implementing progress reporting or to transfer large byte blobs.
//!
//! [`task::run_catch`]: crate::run_catch
//! [`task::wait_catch`]: crate::wait_catch
//! [`ipc_channel`]: https://docs.rs/ipc-channel
//! [`duct`]: https://docs.rs/duct
//!
//! # Examples
//!
//! The example below demonstrates a worker-process setup that uses the same executable as the app-process.
//!
//! ```
//! # use zng_task as task;
//! #
//! fn main() {
//!     // this must be called before the app start, when the process
//!     // is a worker this function never returns.
//!     task::ipc::run_worker(worker);
//!
//!     // normal app init..
//!     # task::doc_test(false, on_click());
//! }
//!
//! // All IPC tasks for the same worker must be defined on the same type.
//! #[derive(Debug, serde::Serialize, serde::Deserialize)]
//! enum IpcRequest {
//!     Task1,
//!     Task2,
//! }
//! #[derive(Debug, serde::Serialize, serde::Deserialize)]
//! enum IpcResponse {
//!     Result1,
//!     Result2,
//! }
//!
//! // This handler is called for every worker task, in the worker-process.
//! async fn worker(args: task::ipc::RequestArgs<IpcRequest>) -> IpcResponse {
//!    println!("received request `{:?}` in worker-process #{}", &args.request, std::process::id());
//!     match args.request {
//!         IpcRequest::Task1 => IpcResponse::Result1,
//!         IpcRequest::Task2 => IpcResponse::Result2,
//!     }
//! }
//!
//! // This runs in the app-process, it starts a worker process and requests a task run.
//! async fn on_click() {
//!     println!("app-process #{} starting a worker", std::process::id());
//!     let mut worker = match task::ipc::Worker::start().await {
//!         Ok(w) => w,
//!         Err(e) => {
//!             eprintln!("error: {e}");
//!             return;
//!         },
//!     };
//!     // request a task run and await it.
//!     match worker.run(IpcRequest::Task1).await {
//!         Ok(IpcResponse::Result1) => println!("ok."),
//!         Err(e) => eprintln!("error: {e}"),
//!         _ => unreachable!(),
//!     }
//!     // multiple tasks can be requested in parallel, use `task::all!` to await ..
//!
//!     // the worker process can be gracefully shutdown, awaits all pending tasks.
//!     let _ = worker.shutdown().await;
//! }
//!
//! ```
//!
//! Note that you can setup different worker types on the same executable using [`Worker::start_with`] with a custom
//! environment variable that switches the [`run_worker`] call.
//!
//! ```
//! # //! # use zng_task as task;
//! #
//! fn run_workers() {
//!     match std::env::var("MY_APP_WORKER") {
//!         Ok(name) => match name.as_str() {
//!             "worker_a" => task::ipc::run_worker(worker_a),
//!             "worker_b" => task::ipc::run_worker(worker_b),
//!             unknown => panic!("unknown worker, {unknown:?}"),
//!         },
//!         Err(e) => match e {
//!             std::env::VarError::NotPresent => {} // not a worker run
//!             e => panic!("invalid worker name, {e}"),
//!         },
//!     }
//! }
//! 
//! async fn worker_a(args: task::ipc::RequestArgs<bool>) -> char {
//!     if args.request {
//!         'A'
//!     } else {
//!         'a'
//!     }
//! }
//! 
//! async fn worker_b(args: task::ipc::RequestArgs<char>) -> bool {
//!     args.request == 'B' || args.request == 'b'
//! }
//! 
//! fn main() {
//!     self::run_workers();
//! 
//!     // normal app init..
//!     # task::doc_test(false, on_click());
//! }
//! 
//! // And in the app side:
//! async fn on_click() {
//!     let mut worker_a = task::ipc::Worker::start_with(&[("MY_APP_WORKER", "worker_a")], &[]).await.unwrap();
//!     let r = worker_a.run(true).await.ok();
//!     assert_eq!(r, Some('A'));
//! }
//! ```

use core::fmt;
use std::{future::Future, marker::PhantomData, path::PathBuf, pin::Pin, sync::Arc};

use parking_lot::Mutex;
use zng_clone_move::{async_clmv, clmv};
use zng_txt::{ToTxt, Txt};
use zng_unique_id::IdMap;
use zng_unit::TimeUnits as _;

#[doc(no_inline)]
pub use ipc_channel::ipc::{bytes_channel, IpcBytesReceiver, IpcBytesSender, IpcReceiver, IpcSender};

/// Represents a type that can be an input and output of IPC workers.
///
/// # Trait Alias
///
/// This trait is used like a type alias for traits and is
/// already implemented for all types it applies to.
///
/// # Implementing
///
/// Types need to be `Debug + serde::Serialize + serde::de::Deserialize + Send + 'static` to auto-implement this trait,
/// if you want to send an external type in that does not implement all the traits
/// you may need to declare a *newtype* wrapper.
pub trait IpcValue: fmt::Debug + serde::Serialize + for<'d> serde::de::Deserialize<'d> + Send + 'static {}

impl<T: fmt::Debug + serde::Serialize + for<'d> serde::de::Deserialize<'d> + Send + 'static> IpcValue for T {}

const WORKER_VERSION: &str = "ZNG_TASK_IPC_WORKER_VERSION";
const WORKER_SERVER: &str = "ZNG_TASK_IPC_WORKER_SERVER";

/// The *App Process* and *Worker Process* must be build using the same exact version and this is
/// validated during run-time, causing a panic if the versions don't match.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Represents a running worker process.
pub struct Worker<I: IpcValue, O: IpcValue> {
    running: Option<(std::thread::JoinHandle<()>, duct::Handle)>,

    sender: ipc_channel::ipc::IpcSender<(RequestId, Request<I>)>,
    requests: Arc<Mutex<IdMap<RequestId, flume::Sender<O>>>>,

    _p: PhantomData<fn(I) -> O>,

    crash: Option<WorkerCrashError>,
}
impl<I: IpcValue, O: IpcValue> Worker<I, O> {
    /// Start a worker process implemented in the current executable.
    ///
    /// Note that the current process must call [`run_worker`] at startup to actually work.
    pub async fn start() -> std::io::Result<Self> {
        Self::start_impl(duct::cmd!(std::env::current_exe()?)).await
    }

    /// Start a worker process implemented in the current executable with custom env vars and args.
    pub async fn start_with(env_vars: &[(&str, &str)], args: &[&str]) -> std::io::Result<Self> {
        let mut worker = duct::cmd(std::env::current_exe()?, args);
        for (name, value) in env_vars {
            worker = worker.env(name, value);
        }
        Self::start_impl(worker).await
    }

    /// Start a worker process implemented in another executable with custom env vars and args.
    pub async fn start_other(worker_exe: impl Into<PathBuf>, env_vars: &[(&str, &str)], args: &[&str]) -> std::io::Result<Self> {
        let mut worker = duct::cmd(worker_exe.into(), args);
        for (name, value) in env_vars {
            worker = worker.env(name, value);
        }
        Self::start_impl(worker).await
    }

    /// Start a worker process from a custom configured [`duct`] process.
    ///
    /// Note that the worker executable must call [`run_worker`] at startup to actually work.
    ///
    /// [`duct`]: https://docs.rs/duct/
    pub async fn start_duct(worker: duct::Expression) -> std::io::Result<Self> {
        Self::start_impl(worker).await
    }

    async fn start_impl(worker: duct::Expression) -> std::io::Result<Self> {
        let (server, name) = ipc_channel::ipc::IpcOneShotServer::<WorkerInit<I, O>>::new()?;

        let worker = worker
            .env(WORKER_VERSION, crate::ipc::VERSION)
            .env(WORKER_SERVER, name)
            .env("RUST_BACKTRACE", "full")
            .stdin_null()
            .stdout_capture()
            .stderr_capture()
            .unchecked();

        let process = crate::wait(move || worker.start()).await?;

        let r = crate::with_deadline(crate::wait(move || server.accept()), 10.secs()).await;

        let (_, (req_sender, chan_sender)) = match r {
            Ok(r) => match r {
                Ok(r) => r,
                Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::ConnectionRefused, e)),
            },
            Err(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "worker process did not connect in 10s",
                ))
            }
        };

        let (rsp_sender, rsp_recv) = ipc_channel::ipc::channel()?;
        crate::wait(move || chan_sender.send(rsp_sender)).await.unwrap();

        let requests = Arc::new(Mutex::new(IdMap::<RequestId, flume::Sender<O>>::new()));
        let receiver = std::thread::spawn(clmv!(requests, || {
            loop {
                match rsp_recv.recv() {
                    Ok((id, r)) => match requests.lock().remove(&id) {
                        Some(s) => match r {
                            Response::Out(r) => {
                                let _ = s.send(r);
                            }
                        },
                        None => tracing::error!("worker responded to unknown request #{}", id.sequential()),
                    },
                    Err(e) => match e {
                        ipc_channel::ipc::IpcError::Disconnected => {
                            requests.lock().clear();
                            break;
                        }
                        ipc_channel::ipc::IpcError::Bincode(e) => {
                            tracing::error!("worker response error, {e}")
                        }
                        ipc_channel::ipc::IpcError::Io(e) => {
                            tracing::error!("worker response io error, will shutdown, {e}");
                            break;
                        }
                    },
                }
            }
        }));

        Ok(Self {
            running: Some((receiver, process)),
            sender: req_sender,
            _p: PhantomData,
            crash: None,
            requests,
        })
    }

    /// Awaits current tasks and kills the worker process.
    pub async fn shutdown(mut self) -> std::io::Result<()> {
        if let Some((receiver, process)) = self.running.take() {
            while !self.requests.lock().is_empty() {
                crate::deadline(100.ms()).await;
            }
            let r = crate::wait(move || process.kill()).await;

            match crate::with_deadline(crate::wait(move || receiver.join()), 1.secs()).await {
                Ok(r) => {
                    if let Err(p) = r {
                        tracing::error!("worker receiver thread exited panicked, {}", crate::crate_util::panic_str(&p));
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

    /// Run a task in a free worker thread.
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
        let sender = self.sender.clone();
        let send_r = crate::wait(move || sender.send((id, request)));

        Box::pin(async move {
            if let Err(e) = send_r.await {
                requests.lock().remove(&id);
                return Err(RunError::Ser(Arc::new(e)));
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

    /// Crash error.
    ///
    /// The worker cannot be used if this is set, run requests will immediately disconnect.
    pub fn crash_error(&mut self) -> Option<&WorkerCrashError> {
        if let Some((t, _)) = &self.running {
            if t.is_finished() {
                let (t, p) = self.running.take().unwrap();

                if let Err(e) = t.join() {
                    tracing::error!("panic in worker receiver thread, {}", crate::crate_util::panic_str(&e));
                }

                if let Err(e) = p.kill() {
                    tracing::error!("error killing worker process after receiver exit, {e}");
                }

                match p.into_output() {
                    Ok(o) => {
                        self.crash = Some(WorkerCrashError {
                            status: o.status,
                            stdout: String::from_utf8_lossy(&o.stdout[..]).as_ref().to_txt(),
                            stderr: String::from_utf8_lossy(&o.stderr[..]).as_ref().to_txt(),
                        });
                    }
                    Err(e) => tracing::error!("error reading crashed worker output, {e}"),
                }
            }
        }

        self.crash.as_ref()
    }
}
impl<I: IpcValue, O: IpcValue> Drop for Worker<I, O> {
    fn drop(&mut self) {
        if let Some((receiver, process)) = self.running.take() {
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
pub fn run_worker<I, O, F>(handler: fn(RequestArgs<I>) -> F)
where
    I: IpcValue,
    O: IpcValue,
    F: Future<Output = O> + Send + Sync + 'static,
{
    if let (Ok(version), Ok(server_name)) = (std::env::var(WORKER_VERSION), std::env::var(WORKER_SERVER)) {
        if version != VERSION {
            eprintln!(
                "worker API version is not equal, app-process: {}, worker-process: {}",
                version, VERSION
            );
            std::process::exit(i32::from_le_bytes(*b"vapi"));
        }

        let app_init_sender = IpcSender::<WorkerInit<I, O>>::connect(server_name).expect("failed to connect to init channel");

        let (req_sender, req_recv) = ipc_channel::ipc::channel().unwrap();
        let (chan_sender, chan_recv) = ipc_channel::ipc::channel().unwrap();

        app_init_sender.send((req_sender, chan_sender)).unwrap();
        let rsp_sender = chan_recv.recv().unwrap();

        loop {
            match req_recv.recv() {
                Ok((id, input)) => match input {
                    Request::Run(r) => crate::spawn(async_clmv!(rsp_sender, {
                        let output = handler(RequestArgs { request: r }).await;
                        let _ = rsp_sender.send((id, Response::Out(output)));
                    })),
                },
                Err(e) => match e {
                    ipc_channel::ipc::IpcError::Bincode(e) => {
                        eprintln!("worker request error, {e}")
                    }
                    ipc_channel::ipc::IpcError::Io(e) => panic!("worker request io error, {e}"),
                    ipc_channel::ipc::IpcError::Disconnected => break,
                },
            }
        }

        std::process::exit(0);
    }
}

/// Arguments for [`run_worker`].
pub struct RequestArgs<I: IpcValue> {
    /// The task request data.
    pub request: I,
}

/// Worker run error.
#[derive(Debug, Clone)]
pub enum RunError {
    /// Lost connection with the worker process.
    ///
    /// See [`Worker::crash_error`] for the error.
    Disconnected,
    /// Error serializing request.
    Ser(Arc<bincode::Error>),
    /// Error deserializing response.
    De(Arc<bincode::Error>),
}
impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunError::Disconnected => write!(f, "worker process disconnected"),
            RunError::Ser(e) => write!(f, "error serializing request, {e}"),
            RunError::De(e) => write!(f, "error deserializing response, {e}"),
        }
    }
}
impl std::error::Error for RunError {}

/// Info about a worker process crash.
#[derive(Debug, Clone)]
pub struct WorkerCrashError {
    /// Worker process exit code.
    pub status: std::process::ExitStatus,
    /// Full capture of the worker stdout.
    pub stdout: Txt,
    /// Full capture of the worker stderr.
    pub stderr: Txt,
}
impl fmt::Display for WorkerCrashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}\nSTDOUT:\n{}\nSTDERR:\n{}", self.status, &self.stdout, &self.stderr)
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
