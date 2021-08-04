//! IO tasks.

use std::{
    fmt, io, panic,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use crate::crate_util::{panic_str, PanicPayload};

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
/// let file = task::wait(|| std::fs::File::open("data-1gibibyte.bin")).await?;
/// let r = ReadTask::default().spawn(file);
/// let mut foo = 0usize;
///
/// let mut eof = false;
/// while !eof {
///     let payload = r.read().await.map_err(|e|e.unwrap_io())?;
///     eof = payload.len() < r.payload_len();
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
    /// # use zero_ui_core::task::io::ReadTask;
    /// # fn demo(read: impl std::io::Read + Send + 'static) {
    /// let task = ReadTask::default().spawn(read);
    /// # }
    /// ```
    ///
    /// Start a task with custom configuration:
    ///
    /// ```
    /// # use zero_ui_core::task::io::ReadTask;
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
        super::spawn(async move {
            let mut read = read;

            loop {
                let r = super::wait_catch(move || {
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

                        if p.len() < payload_len {
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
/// use zero_ui_core::task::{self, io::WriteTask};
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
/// [`wait`]: crate::task::wait
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
    /// # use zero_ui_core::task::io::WriteTask;
    /// # fn demo(write: impl std::io::Write + Send + 'static) {
    /// let task = WriteTask::default().spawn(write);
    /// # }
    /// ```
    ///
    /// Start a task with custom configuration:
    ///
    /// ```
    /// # use zero_ui_core::task::io::WriteTask;
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
        super::spawn(async move {
            let mut write = write;
            let mut error = None;
            let mut error_payload = vec![];

            while let Ok(msg) = receiver.recv().await {
                match msg {
                    WriteTaskMsg::WriteAll(p) => {
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

            // send non-panic finish message.
            let _ = f_sender.send(WriteTaskFinishMsg::Io { write, error, payloads }).await;
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
        })
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
    payload: Vec<u8>,
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
            let task = ReadTask::default().payload_len(1).spawn(TestRead::default());

            task::timeout(10.ms()).await;

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
            let flag = Arc::clone(&read.cause_error);

            let task = ReadTask::default().payload_len(1).spawn(read);

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

            let task = ReadTask::default().payload_len(1).spawn(read);

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

            let task = WriteTask::default().spawn(write);

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

            let task = WriteTask::default().spawn(write);

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

            let task = WriteTask::default().spawn(write);

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

            let task = WriteTask::default().spawn(write);

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
