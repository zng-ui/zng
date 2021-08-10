//! IO tasks.

use std::{
    fmt, fs, io, panic,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use async_trait::*;
use parking_lot::Mutex;

use crate::{
    crate_util::{panic_str, PanicPayload},
    units::*,
};

use super::channel;

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
/// use zero_ui_core::task::{self, io::ReadTask, rayon::prelude::*};
///
/// let r = ReadTask::default().open("data-1gibibyte.bin").await?;
/// let mut foo = 0usize;
///
/// let mut eof = false;
/// while !eof {
///     let payload = r.read().await.map_err(|e|e.unwrap_io())?;
///     eof = payload.len() < r.payload_len().bytes();
///     foo += payload.into_par_iter().filter(|&b|b == 0xF0).count();
/// }
///
/// let file = r.stop().await.unwrap();
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
/// [`wait`]: crate::task::wait
/// [`stop`]: ReadTask::stop
/// [`BrokenPipe`]: io::ErrorKind::BrokenPipe
pub struct ReadTask<R> {
    receiver: channel::Receiver<Result<Vec<u8>, ReadTaskError>>,
    stop_recv: channel::Receiver<R>,
    payload_len: ByteLength,
    metrics: Arc<Mutex<Metrics>>,
}
impl ReadTask<()> {
    /// Start building a read task.
    ///
    /// # Examples
    ///
    /// Start a task that reads 1 mebibyte payloads and with a maximum of 8 pre-reads in the channel:
    ///
    /// ```
    /// # use zero_ui_core::{task::io::ReadTask, units::*};
    /// # fn demo(read: impl std::io::Read + Send + 'static, estimated_total: ByteLength) {
    /// let task = ReadTask::default().metrics(false).spawn(read, 0.bytes());
    /// # }
    /// ```
    ///
    /// Start a task with custom configuration:
    ///
    /// ```
    /// # use zero_ui_core::{task::io::ReadTask, units::*};
    /// # const FRAME_LEN: usize = 1024 * 1024 * 2;
    /// # const FRAME_COUNT: usize = 3;
    /// # fn demo(read: impl std::io::Read + Send + 'static, estimated_total: ByteLength) {
    /// let task = ReadTask::default()
    ///     .payload_len(FRAME_LEN.bytes())
    ///     .channel_capacity(FRAME_COUNT.min(8))
    ///     .spawn(read, estimated_total);
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
    fn spawn(builder: ReadTaskBuilder, read: R, estimated_total: ByteLength) -> Self {
        let payload_len = builder.payload_len;
        let (sender, receiver) = channel::bounded(builder.channel_capacity);
        let (stop_sender, stop_recv) = channel::bounded(1);
        let metrics = Arc::new(Mutex::new(Metrics::zero(TaskType::Read)));
        if builder.metrics {
            metrics.lock().progress.1 = estimated_total;
        }
        let metrics_send = Arc::clone(&metrics);
        let start_time = Instant::now();
        super::spawn(async move {
            let mut read = read;

            loop {
                let payload_start = if builder.metrics { Some(Instant::now()) } else { None };

                let r = super::wait_catch(move || {
                    let mut payload = vec![0u8; payload_len.0];
                    loop {
                        match read.read(&mut payload) {
                            Ok(c) => {
                                if c < payload_len.0 {
                                    payload.truncate(c);
                                }
                                return Ok((payload, read));
                            }
                            Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                                continue;
                            }
                            Err(error) => return Err((error, read)),
                        }
                    }
                })
                .await;

                // handle panic
                let r = match r {
                    Ok(r) => r,
                    Err(p) => {
                        let _ = sender.send(Err(ReadTaskError::Panic(p))).await;
                        break; // cause: panic
                    }
                };

                // handle ok or error
                match r {
                    Ok((p, r)) => {
                        read = r;

                        if let Some(payload_start) = payload_start {
                            let now = Instant::now();
                            let payload_time = now.duration_since(payload_start).as_secs_f64().round();
                            let speed = ((payload_len.0 as f64 / payload_time) as usize).bytes();
                            let total_time = now.duration_since(start_time);

                            let mut m = metrics_send.lock();
                            m.progress.0 += p.len().bytes();
                            m.speed = speed;
                            m.total_time = total_time;
                        }

                        if p.len() < payload_len.0 {
                            let _ = sender.send(Ok(p)).await;
                            let _ = stop_sender.send(read).await;
                            break; // cause: EOF
                        } else if sender.send(Ok(p)).await.is_err() {
                            let _ = stop_sender.send(read).await;
                            break; // cause: receiver dropped
                        }
                    }
                    Err((e, r)) => {
                        let _ = sender.send(Err(ReadTaskError::Io(e))).await;
                        let _ = stop_sender.send(r).await;
                        break; // cause: IO error
                    }
                }
            }

            if builder.metrics {
                metrics_send.lock().total_time = Instant::now().duration_since(start_time);
            }
        });
        ReadTask {
            receiver,
            stop_recv,
            payload_len,
            metrics,
        }
    }

    /// Maximum number of bytes per payload.
    #[inline]
    pub fn payload_len(&self) -> ByteLength {
        self.payload_len
    }

    /// Clones the current progress info.
    #[inline]
    pub fn metrics(&self) -> Metrics {
        self.metrics.lock().clone()
    }

    /// Request the next payload.
    ///
    /// The payload length can be equal to or less then [`payload_len`]. If it is less, the stream
    /// has reached `EOF` and subsequent read calls will always return the [`Closed`] error.
    ///
    /// [`payload_len`]: ReadTask::payload_len
    /// [`Closed`]: ReadTaskError::Closed
    pub async fn read(&self) -> Result<Vec<u8>, ReadTaskError> {
        self.receiver.recv().await.map_err(|_| ReadTaskError::Closed)?
    }

    /// Stops the worker task and takes back the [`io::Read`].
    ///
    /// Returns `None` the worker is already stopped due to a panic.
    pub async fn stop(self) -> Option<R> {
        drop(self.receiver);
        self.stop_recv.recv().await.ok()
    }
}
#[async_trait]
impl<R: io::Read + Send + 'static> super::ReceiverTask for ReadTask<R> {
    type Error = ReadTaskError;

    async fn recv(&self) -> Result<Vec<u8>, Self::Error> {
        self.read().await
    }

    async fn stop(self) {
        let _ = self.stop().await;
    }
}

/// Error from [`ReadTask::read`].
pub enum ReadTaskError {
    /// A read IO error.
    Io(io::Error),

    /// A panic in the [`io::Read`].
    ///
    /// You can propagate the panic using [`std::panic::resume_unwind`].
    Panic(PanicPayload),

    /// Lost connection with the task worker.
    ///
    /// The task worker closes on the first [`Io`] error or the first [`Panic`].
    ///
    /// [`Io`]: Self::Io
    /// [`Panic`]: Self::Panic
    Closed,
}
impl ReadTaskError {
    /// Returns the error of [`Io`] or panics.
    ///
    /// # Panics
    ///
    /// If the error is a [`Panic`] the panic is propagated using [`resume_unwind`].
    /// If the error is a [`Closed`] panics with the message `"read task worker is closed, it closes after the first error"`.
    ///
    /// [`Io`]: Self::Io
    /// [`Panic`]: Self::Panic
    /// [`Closed`]: Self::Closed
    /// [`resume_unwind`]: std::panic::resume_unwind
    #[track_caller]
    pub fn unwrap_io(self) -> io::Error {
        match self {
            ReadTaskError::Io(e) => e,
            ReadTaskError::Panic(p) => std::panic::resume_unwind(p),
            ReadTaskError::Closed => panic!("`ReadTask` worker is closed, it closes after the first error"),
        }
    }
}
impl fmt::Debug for ReadTaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadTaskError::Io(e) => f.debug_tuple("Io").field(e).finish(),
            ReadTaskError::Panic(p) => write!(f, "Panic({:?})", panic_str(p)),
            ReadTaskError::Closed => write!(f, "Closed"),
        }
    }
}
impl fmt::Display for ReadTaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadTaskError::Io(e) => write!(f, "{}", e),
            ReadTaskError::Panic(p) => write!(f, "{}", panic_str(p)),
            ReadTaskError::Closed => write!(f, "`ReadTask` worker is closed due to error or panic"),
        }
    }
}
impl std::error::Error for ReadTaskError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let ReadTaskError::Io(e) = self {
            Some(e)
        } else {
            None
        }
    }
}
impl From<io::Error> for ReadTaskError {
    fn from(e: io::Error) -> Self {
        ReadTaskError::Io(e)
    }
}

/// Builds [`ReadTask`].
///
/// Use [`ReadTask::default`] to start.
#[derive(Debug, Clone)]
pub struct ReadTaskBuilder {
    payload_len: ByteLength,
    channel_capacity: usize,
    metrics: bool,
}
impl Default for ReadTaskBuilder {
    fn default() -> Self {
        ReadTaskBuilder {
            payload_len: 1.mebi_bytes(),
            channel_capacity: 8,
            metrics: true,
        }
    }
}
impl ReadTaskBuilder {
    /// Set the byte count for each payload.
    ///
    /// Default is 1 mebibyte (`1024 * 1024`). Minimal value is 1.
    #[inline]
    pub fn payload_len(mut self, bytes: ByteLength) -> Self {
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

    /// Enable or disable metrics collecting.
    ///
    /// This is enabled by default.
    #[inline]
    pub fn metrics(mut self, enabled: bool) -> Self {
        self.metrics = enabled;
        self
    }

    fn normalize(&mut self) {
        if self.payload_len.0 < 1 {
            self.payload_len.0 = 1;
        }
    }

    /// Start reading from `read`.
    ///
    /// The `estimated_total` is used for [`metrics`] only.
    ///
    /// [`metrics`]: Self::metrics
    #[inline]
    pub fn spawn<R>(mut self, read: R, estimated_total: ByteLength) -> ReadTask<R>
    where
        R: io::Read + Send + 'static,
    {
        self.normalize();
        ReadTask::spawn(self, read, estimated_total)
    }

    /// Start reading from the `file`.
    ///
    /// If [`metrics`] is enabled gets the file size first.
    ///
    /// [`metrics`]: Self::metrics
    pub async fn file(self, file: fs::File) -> io::Result<ReadTask<fs::File>> {
        if self.metrics {
            let (file, len) = super::wait(move || {
                let len = file.metadata()?.len() as usize;
                Ok::<_, io::Error>((file, len))
            })
            .await?;
            Ok(self.spawn(file, len.bytes()))
        } else {
            Ok(self.spawn(file, 0.bytes()))
        }
    }

    /// Start reading from the file at `path` or returns an error if the file could not be opened.
    pub async fn open(self, path: impl Into<PathBuf>) -> io::Result<ReadTask<fs::File>> {
        let path = path.into();
        let need_len = self.metrics;
        let (file, len) = super::wait(move || {
            let file = fs::File::open(path)?;
            let len = if need_len { file.metadata()?.len() as usize } else { 0 };
            Ok::<_, io::Error>((file, len))
        })
        .await?;
        Ok(self.spawn(file, len.bytes()))
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
/// use zero_ui_core::{task::{self, io::WriteTask}, units::*};
///
/// let file = task::wait(|| std::fs::File::create("output.bin")).await?;
/// let limit = 1024 * 1024 * 1024;
///
/// let w = WriteTask::default().spawn(file, limit.bytes());
///
/// let mut total = 0usize;
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
/// [`wait`]: crate::task::wait
pub struct WriteTask<W> {
    sender: channel::Sender<WriteTaskMsg>,
    finish: channel::Receiver<WriteTaskFinishMsg<W>>,
    state: Arc<WriteTaskState>,
    metrics: Arc<Mutex<Metrics>>,
}
impl WriteTask<()> {
    /// Start building a write task.
    ///
    /// # Examples
    ///
    /// Start a task that writes payloads and with a maximum of 8 pending writes in the channel:
    ///
    /// ```
    /// # use zero_ui_core::{task::io::WriteTask, units::*};
    /// # fn demo(write: impl std::io::Write + Send + 'static, estimated_total: ByteLength) {
    /// let task = WriteTask::default().metrics(false).spawn(write, 0.bytes());
    /// # }
    /// ```
    ///
    /// Start a task with custom configuration:
    ///
    /// ```
    /// # use zero_ui_core::{task::io::WriteTask, units::*};
    /// # const FRAME_COUNT: usize = 3;
    /// # fn demo(write: impl std::io::Write + Send + 'static, estimated_total: ByteLength) {
    /// let task = WriteTask::default()
    ///     .channel_capacity(FRAME_COUNT.min(8))
    ///     .spawn(write, estimated_total);
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
    fn spawn(builder: WriteTaskBuilder, write: W, estimated_total: ByteLength) -> Self {
        let (sender, receiver) = channel::bounded(builder.channel_capacity);
        let (f_sender, f_receiver) = channel::rendezvous();
        let state = Arc::new(WriteTaskState {
            bytes_written: AtomicU64::new(0),
        });
        let t_state = Arc::clone(&state);

        let metrics = Arc::new(Mutex::new(Metrics::zero(TaskType::Write)));
        if builder.metrics {
            metrics.lock().progress.1 = estimated_total;
        }
        let metrics_send = Arc::clone(&metrics);
        let start_time = Instant::now();

        super::spawn(async move {
            let mut write = write;
            let mut error = None;
            let mut error_payload = vec![];

            while let Ok(msg) = receiver.recv().await {
                match msg {
                    WriteTaskMsg::WriteAll(p) => {
                        let payload_start = if builder.metrics { Some(Instant::now()) } else { None };
                        let payload_len = p.len();

                        let r = super::wait_catch(move || {
                            let r = write.write_all(&p);
                            (write, p, r)
                        })
                        .await;

                        // handle panic.
                        let (w, p, r) = match r {
                            Ok((w, p, r)) => (w, p, r),
                            Err(p) => {
                                drop(receiver);
                                let _ = f_sender.send(WriteTaskFinishMsg::Panic(p)).await;
                                return;
                            }
                        };

                        // handle ok/io error.
                        write = w;
                        match r {
                            Ok(_) => {
                                t_state.payload_written(p.len());
                                if let Some(payload_start) = payload_start {
                                    let now = Instant::now();
                                    let payload_time = now.duration_since(payload_start).as_secs_f64().round();
                                    let speed = ((payload_len as f64 / payload_time) as usize).bytes();
                                    let total_time = now.duration_since(start_time);

                                    let mut m = metrics_send.lock();
                                    m.progress.0 += payload_len.bytes();
                                    m.speed = speed;
                                    m.total_time = total_time;
                                }
                            }
                            Err(e) => {
                                error = Some(e);
                                error_payload = p;
                                break;
                            }
                        }
                    }
                    WriteTaskMsg::Flush(rsp) => {
                        let r = super::wait_catch(move || {
                            let r = write.flush();
                            (write, r)
                        })
                        .await;

                        // handle panic.
                        let (w, r) = match r {
                            Ok((w, r)) => (w, r),
                            Err(p) => {
                                drop(receiver);
                                let _ = f_sender.send(WriteTaskFinishMsg::Panic(p)).await;
                                return;
                            }
                        };

                        // handle ok/io error.
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
                        let r = super::wait_catch(move || {
                            let r = write.flush();
                            (write, r)
                        })
                        .await;

                        // handle panic.
                        let (w, r) = match r {
                            Ok((w, r)) => (w, r),
                            Err(p) => {
                                drop(receiver);
                                let _ = f_sender.send(WriteTaskFinishMsg::Panic(p)).await;
                                return;
                            }
                        };

                        // handle ok/io error.
                        write = w;
                        match r {
                            Ok(_) => break,
                            Err(e) => error = Some(e),
                        }
                    }
                }
            }

            let payloads = Self::drain_payloads(receiver, error_payload);

            if builder.metrics {
                metrics_send.lock().total_time = Instant::now().duration_since(start_time);
            }

            // send non-panic finish message.
            let _ = f_sender.send(WriteTaskFinishMsg::Io { write, error, payloads }).await;
        });
        WriteTask {
            sender,
            state,
            finish: f_receiver,
            metrics,
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
        })
    }

    /// Clones the current progress info.
    #[inline]
    pub fn metrics(&self) -> Metrics {
        self.metrics.lock().clone()
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
        let _ = self.sender.send(WriteTaskMsg::Finish).await;

        let msg = self.finish.recv().await.unwrap();

        match msg {
            WriteTaskFinishMsg::Io { write, error, payloads } => match error {
                None => Ok(write),
                Some(error) => Err(WriteTaskError::Io {
                    write,
                    error,
                    bytes_written: self.state.bytes_written(),
                    payloads,
                }),
            },
            WriteTaskFinishMsg::Panic(panic_payload) => Err(WriteTaskError::Panic {
                panic_payload,
                bytes_written: self.state.bytes_written(),
            }),
        }
    }

    fn drain_payloads(recv: channel::Receiver<WriteTaskMsg>, error_payload: Vec<u8>) -> Vec<Vec<u8>> {
        let mut payloads = if error_payload.is_empty() { vec![] } else { vec![error_payload] };
        for msg in recv.drain() {
            if let WriteTaskMsg::WriteAll(payload) = msg {
                payloads.push(payload);
            }
        }
        payloads
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
    metrics: bool,
}
impl Default for WriteTaskBuilder {
    fn default() -> Self {
        WriteTaskBuilder {
            channel_capacity: 8,
            metrics: true,
        }
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

    ///
    #[inline]
    pub fn metrics(mut self, enabled: bool) -> Self {
        self.metrics = enabled;
        self
    }

    /// Start an idle [`WriteTask<W>`] that writes to `write`.
    #[inline]
    pub fn spawn<W>(self, write: W, estimated_total: ByteLength) -> WriteTask<W>
    where
        W: io::Write + Send + 'static,
    {
        WriteTask::spawn(self, write, estimated_total)
    }
}

enum WriteTaskMsg {
    WriteAll(Vec<u8>),
    Flush(channel::Sender<Result<(), WriteTaskClosed>>),
    Finish,
}
enum WriteTaskFinishMsg<W> {
    Io {
        write: W,
        error: Option<io::Error>,
        payloads: Vec<Vec<u8>>,
    },
    Panic(PanicPayload),
}

/// Error from [`WriteTask::finish`].
///
/// The write task worker closes on the first IO error or panic, the [`WriteTask`] send methods
/// return [`WriteTaskClosed`] when this happens and the [`WriteTask::finish`]
/// method returns this error that contains the actual error.
pub enum WriteTaskError<W> {
    /// A write error.
    Io {
        /// The [`io::Write`].
        write: W,
        /// The error.
        error: io::Error,

        /// Number of bytes that where written before the error.
        ///
        /// Note that some bytes from the last payload where probably written too, but
        /// only confirmed written payloads are counted here.
        bytes_written: u64,

        /// The payloads that where not written.
        payloads: Vec<Vec<u8>>,
    },
    /// A panic in the [`io::Write`].
    ///
    /// You can propagate the panic using [`std::panic::resume_unwind`].
    Panic {
        /// The panic message object.
        panic_payload: PanicPayload,
        /// Number of bytes that where written before the error.
        ///
        /// Note that some bytes from the last payload where probably written too, and
        /// given there was a panic some bytes could be corrupted.
        bytes_written: u64,
    },
}
impl<W> WriteTaskError<W> {
    /// Returns the error of [`Io`] or panics.
    ///
    /// # Panics
    ///
    /// If the error is a [`Panic`] the panic is propagated using [`resume_unwind`].
    ///
    /// [`Io`]: Self::Io
    /// [`Panic`]: Self::Panic
    /// [`resume_unwind`]: std::panic::resume_unwind
    #[track_caller]
    pub fn unwrap_io(self) -> io::Error {
        match self {
            Self::Io { error, .. } => error,
            Self::Panic { panic_payload, .. } => panic::resume_unwind(panic_payload),
        }
    }

    /// Returns the [`io::Write`] and error if is an [`Io`] error or panics.
    ///
    /// # Panics
    ///
    /// If the error is a [`Panic`] the panic is propagated using [`resume_unwind`].
    ///
    /// [`Io`]: Self::Io
    /// [`Panic`]: Self::Panic
    /// [`resume_unwind`]: std::panic::resume_unwind
    pub fn unwrap_write(self) -> (W, io::Error) {
        match self {
            Self::Io { write, error, .. } => (write, error),
            Self::Panic { panic_payload, .. } => panic::resume_unwind(panic_payload),
        }
    }
}
impl<W: io::Write> fmt::Debug for WriteTaskError<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Self::Io { error, bytes_written, .. } => f
                .debug_struct("Io")
                .field("error", error)
                .field("bytes_written", bytes_written)
                .finish_non_exhaustive(),
            Self::Panic {
                panic_payload: p,
                bytes_written,
            } => f
                .debug_struct("Panic")
                .field("panic_payload", &panic_str(p))
                .field("bytes_written", bytes_written)
                .finish(),
        }
    }
}
impl<W: io::Write> fmt::Display for WriteTaskError<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Self::Io { error, .. } => write!(f, "{}", error),
            Self::Panic { panic_payload: p, .. } => write!(f, "{}", panic_str(p)),
        }
    }
}
impl<W: io::Write> std::error::Error for WriteTaskError<W> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self {
            Self::Io { error, .. } => Some(error),
            Self::Panic { .. } => None,
        }
    }
}

/// Error from [`WriteTask`].
///
/// This error is returned to indicate that the task worker has permanently stopped because
/// of an IO error or panic. You can get the IO error by calling [`WriteTask::finish`].
pub struct WriteTaskClosed {
    /// Payload that could not be send.
    ///
    /// Is empty in case of a [`flush`] call.
    ///
    /// [`flush`]: WriteTask::flush
    pub payload: Vec<u8>,
}
impl fmt::Debug for WriteTaskClosed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WriteTaskClosed")
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

/// Wrapper that implements [`ReadThenReceive`] for any [`std::io::Read`].
///
/// [`ReadThenReceive`]: super::ReadThenReceive
pub struct ReadThenReceive<R> {
    read: Option<R>,
}
impl<R: io::Read + Send + 'static> ReadThenReceive<R> {
    /// New [`ReadThenReceive`] implementer for `read`.
    pub fn new(read: R) -> Self {
        ReadThenReceive { read: Some(read) }
    }
}
impl ReadThenReceive<fs::File> {
    /// Open a the `path` file for reading.
    pub async fn open(path: impl Into<std::path::PathBuf>) -> std::io::Result<Self> {
        let path = path.into();
        super::wait(move || std::fs::File::open(path)).await.map(Self::new)
    }
}
#[async_trait]
impl<R: io::Read + Send + 'static> super::ReadThenReceive for ReadThenReceive<R> {
    type Error = std::io::Error;

    type Spawned = ReadTask<R>;

    async fn read_exact<const N: usize>(&mut self) -> Result<[u8; N], Self::Error> {
        let mut read = self.read.take().expect("already reading");
        let (read, buf) = super::wait(move || {
            let mut buf = [0; N];
            read.read_exact(&mut buf)?;
            Ok::<_, std::io::Error>((read, buf))
        })
        .await?;
        self.read = Some(read);
        Ok(buf)
    }

    fn spawn(self, payload_len: ByteLength, channel_capacity: usize) -> Self::Spawned {
        ReadTask::default()
            .payload_len(payload_len)
            .channel_capacity(channel_capacity)
            .metrics(false)
            .spawn(self.read.unwrap(), 0.bytes())
    }
}

/// Tag for [`ReadTask`] or [`WriteTask`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskType {
    /// [`ReadTask`].
    Read,
    /// [`WriteTask`].
    Write,
}

/// Information about the state of a [`ReadTask`] or [`WriteTask`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metrics {
    /// If the task is read or write.
    pub task: TaskType,

    /// Number of bytes processed / estimated total.
    pub progress: (ByteLength, ByteLength),

    /// Average progress speed in bytes/second.
    pub speed: ByteLength,

    /// Total time for the entire task. This will continuously increase until the task is
    /// finished.
    pub total_time: Duration,
}
impl Metrics {
    /// All zeros.
    pub fn zero(task: TaskType) -> Self {
        Self {
            task,
            progress: (0.bytes(), 0.bytes()),
            speed: 0.bytes(),
            total_time: Duration::ZERO,
        }
    }
}
impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.progress.0 != self.progress.1 {
            write!(
                f,
                "{}: {} of {}, {}/s",
                match self.task {
                    TaskType::Read => "read",
                    TaskType::Write => "write",
                },
                self.progress.0,
                self.progress.1,
                self.speed
            )
        } else if self.progress.1.bytes() > 0 {
            write!(
                f,
                "{} in {:?}",
                match self.task {
                    TaskType::Read => "read",
                    TaskType::Write => "written",
                },
                self.total_time
            )
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
pub mod tests {
    use std::{future::Future, sync::atomic::AtomicBool, time::Duration};

    use super::*;
    use crate::{task, units::TimeUnits};

    #[track_caller]
    fn async_test<F>(test: F) -> F::Output
    where
        F: Future,
    {
        task::block_on(task::with_timeout(test, Duration::from_secs(1))).unwrap()
    }

    #[test]
    pub fn read_task() {
        async_test(async {
            let task = ReadTask::default().payload_len(1.bytes()).spawn(TestRead::default(), 0.bytes());

            task::timeout(10.ms()).await;

            let payload = task.read().await.unwrap();
            assert_eq!(task.payload_len().bytes(), payload.len());

            task.read().await.unwrap();

            let expected_read_calls = 8 + 2; // default capacity + 2 read calls.
            let expected_bytes_read = task.payload_len().bytes() * expected_read_calls;

            let read = task.stop().await.unwrap();

            assert_eq!(expected_read_calls, read.read_calls);
            assert_eq!(expected_bytes_read, read.bytes_read);
        })
    }

    #[test]
    pub fn read_task_error() {
        async_test(async {
            let read = TestRead::default();
            let flag = Arc::clone(&read.cause_error);

            let task = ReadTask::default().payload_len(1.bytes()).spawn(read, 0.bytes());

            task::timeout(10.ms()).await;

            flag.set();

            loop {
                match task.read().await {
                    Ok(p) => assert_eq!(p.len(), 1),
                    Err(e) => {
                        assert_eq!("test error", e.to_string());

                        let e = task.read().await.unwrap_err();
                        assert!(matches!(e, ReadTaskError::Closed));
                        break;
                    }
                }
            }

            assert!(task.stop().await.is_some());
        })
    }

    #[test]
    pub fn read_task_panic() {
        async_test(async {
            let read = TestRead::default();
            let flag = Arc::clone(&read.cause_panic);

            let task = ReadTask::default().payload_len(1.bytes()).spawn(read, 0.bytes());

            task::timeout(10.ms()).await;

            flag.set();

            loop {
                match task.read().await {
                    Ok(p) => assert_eq!(p.len(), 1),
                    Err(e) => {
                        assert!(e.to_string().contains("test panic"));

                        let e = task.read().await.unwrap_err();
                        assert!(matches!(e, ReadTaskError::Closed));
                        break;
                    }
                }
            }

            assert!(task.stop().await.is_none());
        })
    }

    #[test]
    pub fn write_task() {
        async_test(async {
            let write = TestWrite::default();

            let task = WriteTask::default().spawn(write, 0.bytes());

            for byte in 0u8..20 {
                task.write(vec![byte, byte + 100]).await.unwrap();
            }

            let write = task.finish().await.unwrap();

            assert_eq!(20, write.write_calls);
            assert_eq!(40, write.bytes_written);
            assert_eq!(1, write.flush_calls);
        })
    }

    #[test]
    pub fn write_task_flush() {
        async_test(async {
            let write = TestWrite::default();

            let task = WriteTask::default().spawn(write, 0.bytes());

            for byte in 0u8..20 {
                task.write(vec![byte, byte + 100]).await.unwrap();
            }

            task.flush().await.unwrap();
            let task_bytes_written = task.bytes_written();

            let write = task.finish().await.unwrap();

            assert_eq!(40, task_bytes_written);
            assert_eq!(2, write.flush_calls);

            assert_eq!(20, write.write_calls);
            assert_eq!(40, write.bytes_written);
        })
    }

    #[test]
    pub fn write_error() {
        async_test(async {
            let write = TestWrite::default();
            let flag = write.cause_error.clone();

            let task = WriteTask::default().spawn(write, 0.bytes());

            for byte in 0u8..20 {
                if byte == 10 {
                    flag.set();
                }
                if task.write(vec![byte, byte + 100]).await.is_err() {
                    break;
                }
            }

            let e = task.finish().await.unwrap_err();
            if let WriteTaskError::Io { bytes_written, write, .. } = &e {
                assert_eq!(write.bytes_written as u64, *bytes_written);
            } else {
                panic!("expected WriteTaskError::Io")
            }

            let (write, e) = e.unwrap_write();
            assert!(write.bytes_written > 0);
            assert_eq!("test error", e.to_string());
        })
    }

    #[test]
    pub fn write_panic() {
        async_test(async {
            let write = TestWrite::default();
            let flag = write.cause_panic.clone();

            let task = WriteTask::default().spawn(write, 0.bytes());

            for byte in 0u8..20 {
                if byte == 10 {
                    flag.set();
                }
                if task.write(vec![byte, byte + 100]).await.is_err() {
                    break;
                }
            }

            let e = task.finish().await.unwrap_err();
            if let WriteTaskError::Panic {
                bytes_written,
                panic_payload,
            } = &e
            {
                assert!(*bytes_written > 0);
                assert_eq!("test panic", panic_str(panic_payload))
            } else {
                panic!("expected WriteTaskError::Panic")
            }
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
    pub struct TestWrite {
        bytes_written: usize,
        write_calls: usize,
        flush_calls: usize,
        cause_error: Arc<Flag>,
        cause_panic: Arc<Flag>,
    }
    impl io::Write for TestWrite {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.write_calls += 1;
            if self.cause_error.is_set() {
                Err(io::Error::new(io::ErrorKind::Other, "test error"))
            } else if self.cause_panic.is_set() {
                panic!("test panic");
            } else {
                std::thread::sleep(Duration::from_millis(2));
                self.bytes_written += buf.len();
                Ok(buf.len())
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            self.flush_calls += 1;
            if self.cause_error.is_set() {
                Err(io::Error::new(io::ErrorKind::Other, "test error"))
            } else if self.cause_panic.is_set() {
                panic!("test panic");
            } else {
                Ok(())
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
