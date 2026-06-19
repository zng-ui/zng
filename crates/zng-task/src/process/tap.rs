//! Helper types for recording stdout/err while still passing through.

use std::{
    collections::VecDeque,
    fmt,
    io::{self, Read, Write as _},
    process::{ChildStderr, ChildStdout},
};

use futures_lite::{AsyncRead, AsyncReadExt};
use zng_txt::Txt;

/// Record stdout of a child process while also passing though the output to the running process output.
///
/// Both blocking and async APIs are provided, the blocking API is slightly more efficient.
pub struct StdoutTap(StdTap<false>);
impl fmt::Debug for StdoutTap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("StdoutTap").finish_non_exhaustive()
    }
}
impl StdoutTap {
    /// Start recording and passing.
    pub fn new_blocking(stream: ChildStdout) -> Self {
        Self(StdTap::new_blocking(stream))
    }

    /// Start recording and passing.
    pub fn new(stream: super::ChildStdout) -> Self {
        Self(StdTap::new(stream))
    }

    /// Placeholder tap that records nothing.
    pub fn null() -> Self {
        Self(StdTap::null())
    }

    /// Block until the child process closes stdout and converts the capture to string.
    pub fn into_string_blocking(self) -> String {
        self.0.into_string_blocking()
    }

    /// Await until the child process closes stdout and converts the capture to string.
    pub async fn into_string(self) -> String {
        blocking::unblock(move || self.into_string_blocking()).await
    }

    /// Block until the child process closes stdout and converts the capture to [`Txt`].
    pub fn into_txt_blocking(self) -> Txt {
        self.0.into_txt_blocking()
    }

    /// Await until the child process closes stdout and converts the capture to [`Txt`].
    pub async fn into_txt(self) -> Txt {
        blocking::unblock(move || self.into_txt_blocking()).await
    }
}

/// Record stderr of a child process while also passing though the output to the running process output.
///
/// Both blocking and async APIs are provided, the blocking API is slightly more efficient.
pub struct StderrTap(StdTap<false>);
impl fmt::Debug for StderrTap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("StderrTap").finish_non_exhaustive()
    }
}
impl StderrTap {
    /// Start recording and passing.
    pub fn new_blocking(stream: ChildStderr) -> Self {
        Self(StdTap::new_blocking(stream))
    }

    /// Start recording and passing.
    pub fn new(stream: super::ChildStderr) -> Self {
        Self(StdTap::new(stream))
    }

    /// Placeholder tap that records nothing.
    pub fn null() -> Self {
        Self(StdTap::null())
    }

    /// Block until the child process closes stderr and converts the capture to string.
    pub fn into_string_blocking(self) -> String {
        self.0.into_string_blocking()
    }

    /// Await until the child process closes stderr and converts the capture to string.
    pub async fn into_string(self) -> String {
        blocking::unblock(move || self.into_string_blocking()).await
    }

    /// Block until the child process closes stderr and converts the capture to [`Txt`].
    pub fn into_txt_blocking(self) -> Txt {
        self.0.into_txt_blocking()
    }

    /// Await until the child process closes stderr and converts the capture to [`Txt`].
    pub async fn into_txt(self) -> Txt {
        blocking::unblock(move || self.into_txt_blocking()).await
    }
}

struct StdTap<const E: bool>(Option<std::thread::JoinHandle<VecDeque<u8>>>);

impl<const E: bool> StdTap<E> {
    fn new_blocking(std_stream: impl Read + Send + 'static) -> Self {
        Self(Some(tap(std_stream, E)))
    }

    fn new(stream: impl AsyncRead + Send + Unpin + 'static) -> Self {
        Self(Some(tap_async(stream, E)))
    }

    fn null() -> Self {
        Self(None)
    }

    fn capture(self) -> VecDeque<u8> {
        match self.0 {
            Some(j) => match j.join() {
                Ok(d) => d,
                Err(p) => std::panic::resume_unwind(p),
            },
            None => VecDeque::new(),
        }
    }

    fn into_string_blocking(self) -> String {
        deque_to_string(self.capture())
    }

    fn into_txt_blocking(self) -> Txt {
        self.into_string_blocking().into()
    }
}

fn tap(mut stream: impl Read + Send + 'static, is_err: bool) -> std::thread::JoinHandle<VecDeque<u8>> {
    tap_thread(is_err)
        .spawn(move || tap_read_loop(&mut stream, is_err))
        .expect("failed to spawn thread")
}
fn tap_thread(is_err: bool) -> std::thread::Builder {
    std::thread::Builder::new()
        .name(format!("{}-reader", if is_err { "stderr" } else { "stdout" }))
        .stack_size(256 * 1024)
}
fn tap_read_loop(stream: &mut dyn Read, is_err: bool) -> VecDeque<u8> {
    let mut tap = Tap::new();
    loop {
        let r = stream.read(&mut tap.buffer);
        if tap.push(r, is_err) {
            break;
        }
    }
    tap.rec
}

fn tap_async(mut stream: impl AsyncRead + Send + Unpin + 'static, is_err: bool) -> std::thread::JoinHandle<VecDeque<u8>> {
    tap_thread(is_err)
        .spawn(move || tap_async_read_loop(&mut stream, is_err))
        .expect("failed to spawn thread")
}

fn tap_async_read_loop(stream: &mut (dyn AsyncRead + Unpin), is_err: bool) -> VecDeque<u8> {
    let mut tap = Tap::new();
    loop {
        let r = crate::block_on(stream.read(&mut tap.buffer));
        if tap.push(r, is_err) {
            break;
        }
    }
    tap.rec
}
struct Tap {
    rec: VecDeque<u8>,
    buffer: [u8; 16_384],
}
impl Tap {
    fn new() -> Self {
        Self {
            rec: VecDeque::with_capacity(16_384),
            buffer: [0; 16_384],
        }
    }

    fn push(&mut self, read_r: io::Result<usize>, is_err: bool) -> bool {
        const MAX_CAPTURE: usize = 8_388_608;

        match read_r {
            Ok(n) => {
                if n == 0 {
                    return true;
                }

                let new = &self.buffer[..n];
                let next_len = self.rec.len() + new.len();
                if next_len > MAX_CAPTURE {
                    let overflow = self.rec.len() + new.len() - MAX_CAPTURE;
                    self.rec.drain(..overflow);
                }
                self.rec.extend(new);

                let r = if is_err {
                    let mut s = std::io::stderr();
                    s.write_all(new).and_then(|_| s.flush())
                } else {
                    let mut s = std::io::stdout();
                    s.write_all(new).and_then(|_| s.flush())
                };
                if let Err(e) = r {
                    panic!("{} write error, {}", if is_err { "stderr" } else { "stdout" }, e)
                }
            }
            Err(e) => panic!("{} read error, {}", if is_err { "stderr" } else { "stdout" }, e),
        }

        false
    }
}

fn deque_to_string(deq: VecDeque<u8>) -> String {
    let deq: Vec<u8> = deq.into();
    match String::from_utf8_lossy(&deq) {
        std::borrow::Cow::Borrowed(_) => {
            // SAFETY: from_utf8_lossy only returns `Borrowed` when the input is valid utf-8
            unsafe { String::from_utf8_unchecked(deq) }
        }
        std::borrow::Cow::Owned(s) => s,
    }
}
