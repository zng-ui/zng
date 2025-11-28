//! IO tasks.
//!
//! Most of the types in this module are re-exported from [`futures_lite::io`].
//!
//! [`futures_lite::io`]: https://docs.rs/futures-lite/latest/futures_lite/io/index.html

use std::{
    fmt,
    io::{BufRead, ErrorKind, Read},
    pin::Pin,
    sync::Arc,
    task::{self, Poll},
    time::Duration,
};

use crate::{McWaker, Progress};

#[doc(no_inline)]
pub use futures_lite::io::{
    AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt, AsyncWrite, AsyncWriteExt, BoxedReader, BoxedWriter,
    BufReader, BufWriter, Cursor, ReadHalf, WriteHalf, copy, empty, repeat, sink, split,
};
use parking_lot::Mutex;
use std::io::{Error, Result};
use zng_time::{DInstant, INSTANT};
use zng_txt::formatx;
use zng_unit::{ByteLength, ByteUnits};
use zng_var::impl_from_and_into_var;

/// Measure read/write of an async task.
///
/// Metrics are updated after each read/write, if you read/write all bytes in one call
/// the metrics will only update once.
pub struct Measure<T> {
    task: T,
    metrics: Metrics,
    start_time: DInstant,
    last_write: DInstant,
    last_read: DInstant,
}
impl<T> Measure<T> {
    /// Start measuring a new read/write task.
    pub fn start(task: T, total_read: impl Into<ByteLength>, total_write: impl Into<ByteLength>) -> Self {
        Self::resume(task, (0, total_read), (0, total_write))
    }

    /// Continue measuring a read/write task.
    pub fn resume(
        task: T,
        read_progress: (impl Into<ByteLength>, impl Into<ByteLength>),
        write_progress: (impl Into<ByteLength>, impl Into<ByteLength>),
    ) -> Self {
        let now = INSTANT.now();
        Measure {
            task,
            metrics: Metrics {
                read_progress: (read_progress.0.into(), read_progress.1.into()),
                read_speed: 0.bytes(),
                write_progress: (write_progress.0.into(), write_progress.1.into()),
                write_speed: 0.bytes(),
                total_time: Duration::ZERO,
            },
            start_time: now,
            last_write: now,
            last_read: now,
        }
    }

    /// Current metrics.
    ///
    /// This value is updated after every read/write.
    pub fn metrics(&mut self) -> &Metrics {
        &self.metrics
    }

    /// Unwrap the inner task and final metrics.
    pub fn finish(mut self) -> (T, Metrics) {
        self.metrics.total_time = self.start_time.elapsed();
        (self.task, self.metrics)
    }
}

fn bytes_per_sec(bytes: ByteLength, elapsed: Duration) -> ByteLength {
    let bytes_per_sec = bytes.0 as u128 / elapsed.as_nanos() / Duration::from_secs(1).as_nanos();
    ByteLength(bytes_per_sec as usize)
}

impl<T: AsyncRead + Unpin> AsyncRead for Measure<T> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut task::Context<'_>, buf: &mut [u8]) -> Poll<Result<usize>> {
        let self_ = self.get_mut();
        match Pin::new(&mut self_.task).poll_read(cx, buf) {
            Poll::Ready(Ok(bytes)) => {
                if bytes > 0 {
                    let bytes = bytes.bytes();
                    self_.metrics.read_progress.0 += bytes;

                    let now = INSTANT.now();
                    let elapsed = now - self_.last_read;

                    self_.last_read = now;
                    self_.metrics.read_speed = bytes_per_sec(bytes, elapsed);

                    self_.metrics.total_time = now - self_.start_time;
                }
                Poll::Ready(Ok(bytes))
            }
            p => p,
        }
    }
}
impl<T: AsyncWrite + Unpin> AsyncWrite for Measure<T> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut task::Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        let self_ = self.get_mut();
        match Pin::new(&mut self_.task).poll_write(cx, buf) {
            Poll::Ready(Ok(bytes)) => {
                if bytes > 0 {
                    let bytes = bytes.bytes();
                    self_.metrics.write_progress.0 += bytes;

                    let now = INSTANT.now();
                    let elapsed = now - self_.last_write;

                    self_.last_write = now;
                    self_.metrics.write_speed = bytes_per_sec(bytes, elapsed);

                    self_.metrics.total_time = now - self_.start_time;
                }
                Poll::Ready(Ok(bytes))
            }
            p => p,
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.get_mut().task).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.get_mut().task).poll_close(cx)
    }
}

/// Information about the state of an async IO task.
///
/// Read is also called *receive* or *download*. Write is also called *send* or *upload*. The default
/// display print uses arrows ↓ and ↑ for read and write.
///
/// Use [`Measure`] to measure a task.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Metrics {
    /// Number of bytes read / estimated total.
    pub read_progress: (ByteLength, ByteLength),

    /// Average read speed in bytes/second.
    pub read_speed: ByteLength,

    /// Number of bytes written / estimated total.
    pub write_progress: (ByteLength, ByteLength),

    /// Average write speed in bytes/second.
    pub write_speed: ByteLength,

    /// Total time for the entire task. This will continuously increase until
    /// the task is finished.
    pub total_time: Duration,
}
impl Metrics {
    /// All zeros.
    pub fn zero() -> Self {
        Self {
            read_progress: (0.bytes(), 0.bytes()),
            read_speed: 0.bytes(),
            write_progress: (0.bytes(), 0.bytes()),
            write_speed: 0.bytes(),
            total_time: Duration::ZERO,
        }
    }
}
impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut nl = false;
        if self.read_progress.1 > 0.bytes() {
            nl = true;
            if self.read_progress.0 != self.read_progress.1 {
                write!(f, "↓ {}-{}, {}/s", self.read_progress.0, self.read_progress.1, self.read_speed)?;
                nl = true;
            } else {
                write!(f, "↓ {} . {:?}", self.read_progress.0, self.total_time)?;
            }
        }
        if self.write_progress.1 > 0.bytes() {
            if nl {
                writeln!(f)?;
            }
            if self.write_progress.0 != self.write_progress.1 {
                write!(f, "↑ {} - {}, {}/s", self.write_progress.0, self.write_progress.1, self.write_speed)?;
            } else {
                write!(f, "↑ {} . {:?}", self.write_progress.0, self.total_time)?;
            }
        }

        Ok(())
    }
}
impl_from_and_into_var! {
    fn from(metrics: Metrics) -> Progress {
        let mut status = Progress::indeterminate();
        if metrics.read_progress.1 > 0.bytes() {
            status = Progress::from_n_of(metrics.read_progress.0.0, metrics.read_progress.1.0);
        }
        if metrics.write_progress.1 > 0.bytes() {
            let w_status = Progress::from_n_of(metrics.write_progress.0.0, metrics.write_progress.1.0);
            if status.is_indeterminate() {
                status = w_status;
            } else {
                status = status.and_fct(w_status.fct());
            }
        }
        status.with_msg(formatx!("{metrics}")).with_meta_mut(|mut m| {
            m.set(*METRICS_ID, metrics);
        })
    }
}

zng_state_map::static_id! {
    /// Metrics in a [`Progress::with_meta`] metadata.
    pub static ref METRICS_ID: zng_state_map::StateId<Metrics>;
}

/// Extension methods for [`std::io::Error`] to be used with errors returned by [`McBufReader`].
pub trait McBufErrorExt {
    /// Returns `true` if this error represents the condition where there are only [`McBufReader::is_lazy`] readers
    /// left, the buffer is drained and the inner reader is not EOF.
    ///
    /// You can recover from this error by turning the reader non-lazy using [`McBufReader::set_lazy`].
    fn is_only_lazy_left(&self) -> bool;
}
impl McBufErrorExt for std::io::Error {
    fn is_only_lazy_left(&self) -> bool {
        matches!(self.kind(), ErrorKind::Other) && format!("{self:?}").contains(ONLY_NON_LAZY_ERROR_MSG)
    }
}
const ONLY_NON_LAZY_ERROR_MSG: &str = "no non-lazy readers left to read";

/// Multiple consumer buffered read.
///
/// Clone an instance to create a new consumer, already read bytes stay in the buffer until all clones have read it,
/// clones continue reading from the same offset as the reader they cloned.
///
/// A single instance of this reader behaves like a `BufReader`.
///
/// # Result
///
/// The result is *repeats* ready when `EOF` or an [`Error`] occurs, unfortunately the IO error is not cloneable
/// so the error is recreated using [`CloneableError`] for subsequent poll attempts.
///
/// The inner reader is dropped as soon as it finishes.
///
/// # Lazy Clones
///
/// You can mark clones as [lazy], lazy clones don't pull from the inner reader, only advance when another clone reads, if
/// all living clones are lazy they stop reading with an error. You can identify this custom error using the [`McBufErrorExt::is_only_lazy_left`]
/// extension method.
///
/// [lazy]: Self::set_lazy
pub struct McBufReader<S: AsyncRead> {
    inner: Arc<Mutex<McBufInner<S>>>,
    index: usize,
    lazy: bool,
}
struct McBufInner<S: AsyncRead> {
    source: Option<S>,
    waker: McWaker,
    lazy_wakers: Vec<task::Waker>,

    buf: Vec<u8>,

    clones: Vec<usize>,
    non_lazy_count: usize,

    result: ReadState,
}
impl<S: AsyncRead> McBufReader<S> {
    /// Creates a buffered reader.
    pub fn new(source: S) -> Self {
        let mut clones = Vec::with_capacity(2);
        clones.push(0);
        McBufReader {
            inner: Arc::new(Mutex::new(McBufInner {
                source: Some(source),
                waker: McWaker::empty(),
                lazy_wakers: vec![],

                buf: Vec::with_capacity(10.kilobytes().0),

                clones,
                non_lazy_count: 1,

                result: ReadState::Running,
            })),
            index: 0,
            lazy: false,
        }
    }

    /// Returns `true` if this reader does not pull from the inner reader, only advancing when a non-lazy reader advances.
    ///
    /// The initial reader is not lazy, only clones of lazy readers are lazy by default.
    pub fn is_lazy(&self) -> bool {
        self.lazy
    }

    /// Sets [`is_lazy`].
    ///
    /// [`is_lazy`]: Self::is_lazy
    pub fn set_lazy(&mut self, lazy: bool) {
        if self.lazy != lazy {
            if lazy {
                self.inner.lock().non_lazy_count -= 1;
            } else {
                self.inner.lock().non_lazy_count += 1;
            }
            self.lazy = lazy;
        }
    }
}
impl<S: AsyncRead> Clone for McBufReader<S> {
    fn clone(&self) -> Self {
        let mut inner = self.inner.lock();

        let offset = inner.clones[self.index];
        let index = inner.clones.len();
        inner.clones.push(offset);

        if !self.lazy {
            inner.non_lazy_count += 1;
        }

        Self {
            inner: self.inner.clone(),
            index,
            lazy: self.lazy,
        }
    }
}
impl<S: AsyncRead> Drop for McBufReader<S> {
    fn drop(&mut self) {
        let mut inner = self.inner.lock();
        inner.clones[self.index] = usize::MAX;
        if !self.lazy {
            inner.non_lazy_count -= 1;
            if inner.non_lazy_count == 0 {
                // notify lazy so they get the error.
                for waker in inner.lazy_wakers.drain(..) {
                    waker.wake();
                }
            }
        }
    }
}
impl<S: AsyncRead> AsyncRead for McBufReader<S> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut task::Context<'_>, buf: &mut [u8]) -> Poll<Result<usize>> {
        let self_ = self.as_ref();
        let mut inner = self_.inner.lock();
        let inner = &mut *inner;

        // ready data for this clone.
        let mut i = inner.clones[self_.index];
        let mut ready;

        match &inner.result {
            ReadState::Running => {
                // source has not finished yet.

                ready = &inner.buf[i..];

                if ready.is_empty() {
                    if self.lazy {
                        if inner.non_lazy_count == 0 {
                            // user can make this reader non-lazy and try again.
                            return Poll::Ready(Err(Error::other(ONLY_NON_LAZY_ERROR_MSG)));
                        } else {
                            // register waker for after non-lazy poll.
                            inner.lazy_wakers.push(cx.waker().clone());

                            // wait non-lazy to pull.
                            return Poll::Pending;
                        }
                    }

                    // time to poll source.

                    ready = &[];

                    let waker = match inner.waker.push(cx.waker().clone()) {
                        Some(w) => w,
                        None => {
                            // already polling from another clone.
                            return Poll::Pending;
                        }
                    };

                    let min_i = inner.clones.iter().copied().min().unwrap();
                    if min_i > 0 {
                        // reuse front.
                        inner.buf.copy_within(min_i.., 0);
                        inner.buf.truncate(inner.buf.len() - min_i);

                        i -= min_i;
                        for i in &mut inner.clones {
                            *i -= min_i;
                        }
                    }

                    let new_start = inner.buf.len();

                    inner.buf.resize(inner.buf.len() + buf.len().max(10.kilobytes().0), 0);

                    let mut inner_cx = task::Context::from_waker(&waker);

                    // SAFETY: we don't move `source`.
                    let source = unsafe { Pin::new_unchecked(inner.source.as_mut().unwrap()) };
                    let result = source.poll_read(&mut inner_cx, &mut inner.buf[new_start..]);

                    match result {
                        Poll::Ready(result) => {
                            // notify lazy readers.
                            for waker in inner.lazy_wakers.drain(..) {
                                waker.wake();
                            }

                            match result {
                                Ok(0) => {
                                    inner.waker.cancel();

                                    // EOF
                                    inner.buf.truncate(new_start);
                                    inner.result = ReadState::Eof;
                                    inner.source = None;

                                    // continue 'copy ready
                                }
                                Ok(read) => {
                                    inner.waker.cancel();

                                    // Read > 0
                                    inner.buf.truncate(new_start + read);
                                    ready = &inner.buf[i..];

                                    // continue 'copy ready
                                }
                                Err(e) => {
                                    inner.waker.cancel();

                                    // Error
                                    inner.result = ReadState::Err(CloneableError::new(&e));
                                    inner.buf = vec![];
                                    inner.source = None;

                                    return Poll::Ready(Err(e));
                                }
                            }
                        }

                        Poll::Pending => {
                            inner.buf.truncate(new_start);
                            return Poll::Pending;
                        }
                    }
                }
            }
            ReadState::Eof => {
                ready = &inner.buf[i..];

                // continue 'copy ready
            }
            ReadState::Err(e) => return Poll::Ready(e.err()),
        }

        // 'copy ready

        let max_ready = buf.len().min(ready.len());
        buf[..max_ready].copy_from_slice(&ready[..max_ready]);

        i += max_ready;
        inner.clones[self_.index] = i;

        Poll::Ready(Ok(max_ready))
    }
}

/// Represents the cloneable parts of an [`Error`].
///
/// Unfortunately [`Error`] does not implement clone, this is needed to implemented
/// IO futures that repeat the ready result after subsequent polls. This type partially
/// works around the issue by copying enough information to recreate an error that is still useful.
///
/// The OS error code, [`ErrorKind`] and display message are preserved. Note that this not an error type,
/// it must be converted to [`Error`] using `into` or [`err`].
///
/// [`err`]: Self::err
#[derive(Clone)]
pub struct CloneableError {
    info: ErrorInfo,
}
#[derive(Clone)]
enum ErrorInfo {
    OsError(i32),
    Other(ErrorKind, String),
}
impl CloneableError {
    /// Copy the cloneable information from the [`Error`].
    pub fn new(e: &Error) -> Self {
        let info = if let Some(code) = e.raw_os_error() {
            ErrorInfo::OsError(code)
        } else {
            ErrorInfo::Other(e.kind(), format!("{e}"))
        };

        Self { info }
    }

    /// Returns an `Err(Error)` generated from the cloneable information.
    pub fn err<T>(&self) -> Result<T> {
        Err(self.clone().into())
    }
}
impl From<CloneableError> for Error {
    fn from(e: CloneableError) -> Self {
        match e.info {
            ErrorInfo::OsError(code) => Error::from_raw_os_error(code),
            ErrorInfo::Other(kind, msg) => Error::new(kind, msg),
        }
    }
}

/// Represents a stream reader that generates an error if the source stream exceeds a limit.
///
/// Note that some bytes over the limit may be read once if the source stream is buffered.
pub struct ReadLimited<S> {
    source: S,
    limit: usize,
    on_limit: fn() -> std::io::Error,
}
impl<S> ReadLimited<S> {
    /// Construct a limited reader.
    ///
    /// The `on_limit` closure is called for every read attempt after the limit is reached.
    pub fn new(source: S, limit: ByteLength, on_limit: fn() -> std::io::Error) -> Self {
        Self {
            source,
            limit: limit.0,
            on_limit,
        }
    }

    /// New with default on limit error.
    pub fn new_default_err(source: S, limit: ByteLength) -> Self {
        Self::new(source, limit, || {
            std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "source exceeded read limit")
        })
    }
}
impl<S> AsyncRead for ReadLimited<S>
where
    S: AsyncRead,
{
    fn poll_read(self: Pin<&mut Self>, cx: &mut task::Context<'_>, mut buf: &mut [u8]) -> Poll<Result<usize>> {
        // SAFETY: we don't move anything.
        let self_ = unsafe { self.get_unchecked_mut() };

        if self_.limit == 0 {
            let err = (self_.on_limit)();
            return Poll::Ready(Err(err));
        }

        if buf.len() > self_.limit {
            buf = &mut buf[..self_.limit];
        }

        // SAFETY: we never move `source`.
        match unsafe { Pin::new_unchecked(&mut self_.source) }.poll_read(cx, buf) {
            Poll::Ready(Ok(n)) => {
                self_.limit = self_.limit.saturating_sub(n);
                Poll::Ready(Ok(n))
            }
            r => r,
        }
    }
}
impl<S> AsyncBufRead for ReadLimited<S>
where
    S: AsyncBufRead,
{
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Result<&[u8]>> {
        // SAFETY: we don't move anything.
        let self_ = unsafe { self.get_unchecked_mut() };

        if self_.limit == 0 {
            let err = (self_.on_limit)();
            return Poll::Ready(Err(err));
        }

        // SAFETY: we never move `source`.
        match unsafe { Pin::new_unchecked(&mut self_.source) }.poll_fill_buf(cx) {
            Poll::Ready(Ok(buf)) => {
                self_.limit = self_.limit.saturating_sub(buf.len());
                Poll::Ready(Ok(buf))
            }
            r => r,
        }
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        // SAFETY: we don't move anything.
        let self_ = unsafe { self.get_unchecked_mut() };
        // SAFETY: we never move `source`.
        unsafe { Pin::new_unchecked(&mut self_.source) }.consume(amt);
    }
}
impl<S> Read for ReadLimited<S>
where
    S: Read,
{
    fn read(&mut self, mut buf: &mut [u8]) -> Result<usize> {
        if self.limit == 0 {
            let err = (self.on_limit)();
            return Err(err);
        }

        if buf.len() > self.limit {
            buf = &mut buf[..self.limit];
        }

        match self.source.read(buf) {
            Ok(n) => {
                self.limit = self.limit.saturating_sub(n);
                Ok(n)
            }
            r => r,
        }
    }
}
impl<S> BufRead for ReadLimited<S>
where
    S: BufRead,
{
    fn fill_buf(&mut self) -> Result<&[u8]> {
        if self.limit == 0 {
            let err = (self.on_limit)();
            return Err(err);
        }

        match self.source.fill_buf() {
            Ok(buf) => {
                self.limit = self.limit.saturating_sub(buf.len());
                Ok(buf)
            }
            r => r,
        }
    }

    fn consume(&mut self, amount: usize) {
        self.source.consume(amount);
    }
}

enum ReadState {
    Running,
    Eof,
    Err(CloneableError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as task;
    use zng_unit::TimeUnits;

    #[test]
    pub fn mc_buf_reader_parallel() {
        let data = Data::new(60.kilobytes().0);

        let mut expected = vec![0; data.len];
        let _ = data.clone().blocking_read(&mut expected[..]);

        let mut a = McBufReader::new(data);
        let mut b = a.clone();
        let mut c = a.clone();

        let (a, b, c) = async_test(async move {
            let a = task::run(async move {
                let mut buf = vec![];
                a.read_to_end(&mut buf).await.unwrap();
                buf
            });
            let b = task::run(async move {
                let mut buf: Vec<u8> = vec![];
                b.read_to_end(&mut buf).await.unwrap();
                buf
            });
            let c = task::run(async move {
                let mut buf: Vec<u8> = vec![];
                c.read_to_end(&mut buf).await.unwrap();
                buf
            });

            task::all!(a, b, c).await
        });

        crate::assert_vec_eq!(expected, a);
        crate::assert_vec_eq!(expected, b);
        crate::assert_vec_eq!(expected, c);
    }

    #[test]
    pub fn mc_buf_reader_single() {
        let data = Data::new(60.kilobytes().0);

        let mut expected = vec![0; data.len];
        let _ = data.clone().blocking_read(&mut expected[..]);

        let mut a = McBufReader::new(data);

        let a = async_test(async move {
            let a = task::run(async move {
                let mut buf = vec![];
                a.read_to_end(&mut buf).await.unwrap();
                buf
            });

            a.await
        });

        crate::assert_vec_eq!(expected, a);
    }

    #[test]
    pub fn mc_buf_reader_sequential() {
        let data = Data::new(60.kilobytes().0);

        let mut expected = vec![0; data.len];
        let _ = data.clone().blocking_read(&mut expected[..]);

        let mut clones = vec![McBufReader::new(data)];
        for _ in 0..5 {
            clones.push(clones[0].clone());
        }

        let r = async_test(async move {
            let mut r = vec![];

            for mut clone in clones {
                let mut buf = vec![];
                clone.read_to_end(&mut buf).await.unwrap();
                r.push(buf);
            }

            r
        });

        for r in r {
            crate::assert_vec_eq!(expected, r);
        }
    }

    #[test]
    pub fn mc_buf_reader_completed() {
        let data = Data::new(60.kilobytes().0);
        let mut buf = Vec::with_capacity(data.len);
        let mut a = McBufReader::new(data);

        let r = async_test(async move {
            a.read_to_end(&mut buf).await.unwrap();

            let mut b = a.clone();
            buf.clear();

            b.read_to_end(&mut buf).await.unwrap();
            buf.len()
        });

        assert_eq!(0, r);
    }

    #[test]
    pub fn mc_buf_reader_error() {
        let mut data = Data::new(20.kilobytes().0);
        data.set_error();

        let mut expected = vec![0; data.len];
        let _ = data.clone().blocking_read(&mut expected[..]);

        let mut a = McBufReader::new(data);
        let mut b = a.clone();

        let (a, b) = async_test(async move {
            let a = task::run(async move {
                let mut buf = vec![];
                a.read_to_end(&mut buf).await.unwrap_err()
            });
            let b = task::run(async move {
                let mut buf: Vec<u8> = vec![];
                b.read_to_end(&mut buf).await.unwrap_err()
            });

            task::all!(a, b).await
        });

        assert_eq!(ErrorKind::InvalidData, a.kind());
        assert_eq!(ErrorKind::InvalidData, b.kind());
    }

    #[test]
    pub fn mc_buf_reader_error_completed() {
        let mut data = Data::new(20.kilobytes().0);
        data.set_error();

        let mut buf = Vec::with_capacity(data.len);
        let mut a = McBufReader::new(data);

        let (a, b) = async_test(async move {
            let a_err = a.read_to_end(&mut buf).await.unwrap_err();

            let mut b = a.clone();
            buf.clear();

            let b_err = b.read_to_end(&mut buf).await.unwrap_err();

            (a_err, b_err)
        });

        assert_eq!(ErrorKind::InvalidData, a.kind());
        assert_eq!(ErrorKind::InvalidData, b.kind());
    }

    #[test]
    pub fn mc_buf_reader_parallel_with_delay1() {
        let mut data = Data::new(60.kilobytes().0);
        data.enable_pending();

        let mut expected = vec![0; data.len];
        let _ = data.clone().blocking_read(&mut expected[..]);

        let mut a = McBufReader::new(data);
        let mut b = a.clone();
        let mut c = a.clone();

        let (a, b, c) = async_test(async move {
            let a = task::run(async move {
                let mut buf = vec![];
                a.read_to_end(&mut buf).await.unwrap();
                buf
            });
            let b = task::run(async move {
                let mut buf: Vec<u8> = vec![];
                b.read_to_end(&mut buf).await.unwrap();
                buf
            });
            let c = task::run(async move {
                let mut buf: Vec<u8> = vec![];
                c.read_to_end(&mut buf).await.unwrap();
                buf
            });

            task::all!(a, b, c).await
        });

        crate::assert_vec_eq!(expected, a);
        crate::assert_vec_eq!(expected, b);
        crate::assert_vec_eq!(expected, c);
    }

    #[test]
    pub fn mc_buf_reader_parallel_with_delay2() {
        let mut data = Data::new(60.kilobytes().0);
        data.enable_pending();

        let mut expected = vec![0; data.len];
        let _ = data.clone().blocking_read(&mut expected[..]);

        let mut a = McBufReader::new(data);
        let mut b = a.clone();
        let mut c = a.clone();

        let (a, b, c) = async_test(async move {
            let a = task::run(async move {
                let mut buf = vec![];
                a.read_to_end(&mut buf).await.unwrap();
                buf
            });
            let b = task::run(async move {
                let mut buf: Vec<u8> = vec![];
                task::deadline(5.ms()).await;
                b.read_to_end(&mut buf).await.unwrap();
                buf
            });
            let c = task::run(async move {
                let mut buf: Vec<u8> = vec![];
                c.read_to_end(&mut buf).await.unwrap();
                buf
            });

            task::all!(a, b, c).await
        });

        crate::assert_vec_eq!(expected, a);
        crate::assert_vec_eq!(expected, b);
        crate::assert_vec_eq!(expected, c);
    }

    #[derive(Clone)]
    struct Data {
        b: u8,
        len: usize,
        error: Option<CloneableError>,
        delay: Duration,
        pending: bool,
    }
    impl Data {
        pub fn new(len: usize) -> Self {
            Self {
                b: 0,
                len,
                error: None,
                delay: 0.ms(),
                pending: false,
            }
        }
        pub fn blocking_read(&mut self, buf: &mut [u8]) -> Result<usize> {
            let len = self.len;
            for b in buf.iter_mut().take(len) {
                *b = self.b;
                self.len -= 1;
                self.b = self.b.wrapping_add(1);
            }

            if len == 0
                && let Some(e) = &self.error
            {
                return e.err();
            }

            Ok(buf.len().min(len))
        }
        pub fn set_error(&mut self) {
            self.error = Some(CloneableError::new(&Error::new(ErrorKind::InvalidData, "test error")));
        }

        pub fn enable_pending(&mut self) {
            self.delay = 3.ms();
        }
    }
    impl AsyncRead for Data {
        fn poll_read(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>, buf: &mut [u8]) -> Poll<Result<usize>> {
            if self.delay > Duration::ZERO {
                self.pending = !self.pending;
                if self.pending {
                    let waker = cx.waker().clone();
                    let delay = self.delay;
                    task::spawn(async move {
                        task::deadline(delay).await;
                        waker.wake();
                    });
                    return Poll::Pending;
                }
            }

            let r = self.as_mut().blocking_read(buf);
            Poll::Ready(r)
        }
    }

    #[track_caller]
    fn async_test<F>(test: F) -> F::Output
    where
        F: Future,
    {
        task::block_on(task::with_deadline(test, 5.secs())).unwrap()
    }

    /// Assert vector equality with better error message.
    #[macro_export]
    macro_rules! assert_vec_eq {
        ($a:expr, $b: expr) => {
            match (&$a, &$b) {
                (ref a, ref b) => {
                    let len_not_eq = a.len() != b.len();
                    let mut data_not_eq = None;
                    for (i, (a, b)) in a.iter().zip(b.iter()).enumerate() {
                        if a != b {
                            data_not_eq = Some(i);
                            break;
                        }
                    }

                    if len_not_eq || data_not_eq.is_some() {
                        use std::fmt::*;

                        let mut error = format!("`{}` != `{}`", stringify!($a), stringify!($b));
                        if len_not_eq {
                            let _ = write!(&mut error, "\n  lengths not equal: {} != {}", a.len(), b.len());
                        }
                        if let Some(i) = data_not_eq {
                            let _ = write!(&mut error, "\n  data not equal at index {}: {} != {:?}", i, a[i], b[i]);
                        }
                        panic!("{error}")
                    }
                }
            }
        };
    }
}
