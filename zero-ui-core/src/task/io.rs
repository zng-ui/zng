//! IO tasks.

use std::{
    fmt,
    pin::Pin,
    task::{self, Poll},
    time::{Duration, Instant},
};

use crate::units::*;

#[doc(no_inline)]
pub use futures_lite::io::*;

/// Measure read/write of an async task.
///
/// Metrics are updated after each read/write, if you read/write all bytes in one call
/// the metrics will only update once.
pub struct Measure<T> {
    task: T,
    metrics: Metrics,
    start_time: Instant,
    last_write: Instant,
    last_read: Instant,
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
        let now = Instant::now();
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

                    let now = Instant::now();
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

                    let now = Instant::now();
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
/// Use [`Measure`] to measure a task.
#[derive(Debug, Clone, PartialEq, Eq)]
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
        let mut w = false;
        if self.read_progress.0 > 0.bytes() {
            w = true;
            if self.read_progress.0 != self.read_progress.1 {
                write!(
                    f,
                    "read: {} of {}, {}/s",
                    self.read_progress.0, self.read_progress.1, self.read_speed
                )?;
                w = true;
            } else {
                write!(f, "read {} in {:?}", self.read_progress.0, self.total_time)?;
            }
        }
        if self.write_progress.0 > 0.bytes() {
            if w {
                writeln!(f)?;
            }
            if self.write_progress.0 != self.write_progress.1 {
                write!(
                    f,
                    "write: {} of {}, {}/s",
                    self.write_progress.0, self.write_progress.1, self.write_speed
                )?;
            } else {
                write!(f, "written {} in {:?}", self.read_progress.0, self.total_time)?;
            }
        }

        Ok(())
    }
}
