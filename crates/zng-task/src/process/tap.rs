//! Helper types for recording stdout/err while still passing through.

use std::{
    collections::VecDeque,
    fmt,
    io::{self, BufRead as _, Read, Write as _},
    process::{ChildStderr, ChildStdout},
};

use futures_lite::{AsyncRead, AsyncReadExt};
use zng_txt::{ToTxt as _, Txt, formatx};

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

    /// Block until the child process closes stderr and attempts to parse the last panic info from it.
    ///
    /// If cannot find a panic returns `Err` with the captured stderr converted to [`Txt`].
    pub fn into_panic_blocking(self) -> Result<PanicInfo, Txt> {
        let s = self.into_string_blocking();
        match PanicInfo::find(&s) {
            Some(p) => Ok(p),
            None => Err(s.into()),
        }
    }

    /// Await until the child process closes stderr and attempts to parse the last panic info from it.
    ///
    /// If cannot find a panic returns `Err` with the captured stderr converted to [`Txt`].
    pub async fn into_panic(self) -> Result<PanicInfo, Txt> {
        blocking::unblock(move || self.into_panic_blocking()).await
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

/// Panic parsed from a `stderr` dump.
///
/// # Compatibility
///
/// The parser can seek only the latest Rust stable panic format, to ensure compatibility call
/// [`PanicInfo::set_hook`] on the child process is possible.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct PanicInfo {
    /// Name of thread that panicked.
    pub thread: Txt,
    /// Panic message.
    pub message: Txt,
    /// Path to file that defines the panic.
    pub file: Txt,
    /// Line of code that defines the panic.
    pub line: u32,
    /// Column in the line of code that defines the panic.
    pub column: u32,
    /// Widget where the panic happened.
    ///
    /// Only available in processes that use [`PanicInfo::set_hook`].
    pub widget_path: Txt,
    /// Stack backtrace.
    pub backtrace: Txt,
}

/// Alternate mode `{:#}` prints full backtrace.
impl fmt::Display for PanicInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "thread '{}'", self.thread)?;
        writeln!(f, " panicked at {}:{}:{}:", self.file, self.line, self.column)?;
        for line in self.message.lines() {
            writeln!(f, "   {line}")?;
        }
        if !self.widget_path.is_empty() {
            writeln!(f, "widget path:\n   {}", self.widget_path)?;
        }

        if f.alternate() {
            writeln!(f, "stack backtrace:\n{}", self.backtrace)
        } else {
            writeln!(f, "stack backtrace:")?;
            let mut snippet = 9;
            for frame in self.backtrace_frames().skip_while(|f| f.is_after_panic) {
                write!(f, "{frame}")?;
                if snippet > 0 {
                    let code = frame.code_snippet();
                    if !code.is_empty() {
                        snippet -= 1;
                        writeln!(f, "{code}")?;
                    }
                }
            }
            Ok(())
        }
    }
}
impl PanicInfo {
    /// Gets if `stderr` contains a panic that can be parsed by [`find`].
    ///
    /// [`find`]: Self::find
    pub fn contains(stderr: &str) -> bool {
        Self::find_impl(stderr, false).is_some()
    }

    /// Gets if `stderr` contains a panic that can be parsed by [`find`] and traced a widget/window path.
    ///
    /// [`find`]: Self::find
    pub fn contains_widget(stderr: &str) -> bool {
        match Self::find_impl(stderr, false) {
            Some(p) => !p.widget_path.is_empty(),
            None => false,
        }
    }

    /// Try parse `stderr` for the last panic printout.
    ///
    /// Only reliably works if the panic fully printed correctly and was formatted by
    /// [`PanicInfo::set_hook`].
    pub fn find(stderr: &str) -> Option<Self> {
        Self::find_impl(stderr, true)
    }

    fn find_impl(stderr: &str, parse: bool) -> Option<Self> {
        let mut panic_at = usize::MAX;
        let mut widget_path = usize::MAX;
        let mut stack_backtrace = usize::MAX;
        let mut i = 0;
        for line in stderr.lines() {
            if line.starts_with("thread '") && line.contains(" panicked at ") && line.ends_with(':') {
                panic_at = i;
                widget_path = usize::MAX;
                stack_backtrace = usize::MAX;
            } else if line == "widget path:" {
                widget_path = i + "widget path:\n".len();
            } else if line == "stack backtrace:" {
                stack_backtrace = i + "stack backtrace:\n".len();
            }
            i += line.len() + "\n".len();
        }

        if panic_at == usize::MAX {
            return None;
        }

        if !parse {
            return Some(Self {
                thread: Txt::from(""),
                message: Txt::from(""),
                file: Txt::from(""),
                line: 0,
                column: 0,
                widget_path: if widget_path < stderr.len() {
                    Txt::from("true")
                } else {
                    Txt::from("")
                },
                backtrace: Txt::from(""),
            });
        }

        let panic_str = stderr[panic_at..].lines().next().unwrap();
        let (thread, location) = panic_str.strip_prefix("thread '").unwrap().split_once(" panicked at ").unwrap();
        let mut location = location.split(':');
        let file = location.next().unwrap_or("");
        let line: u32 = location.next().unwrap_or("0").parse().unwrap_or(0);
        let column: u32 = location.next().unwrap_or("0").parse().unwrap_or(0);
        let mut thread = thread.split('\'');
        let mut thread_name = thread.next().unwrap_or("<unnamed>");
        let thread_id = thread.next().unwrap_or("");
        if thread_name == "<unnamed>"
            && let Some(id) = thread_id.strip_prefix('(')
            && let Some(id) = id.strip_suffix(')')
        {
            thread_name = id;
        }

        let mut message = String::new();
        let mut sep = "";
        for line in stderr[panic_at + panic_str.len() + "\n".len()..].lines() {
            if let Some(line) = line.strip_prefix("   ") {
                message.push_str(sep);
                message.push_str(line);
                sep = "\n";
            } else {
                if message.is_empty() && line != "widget path:" && line != "stack backtrace:" {
                    // not formatted by us, probably by Rust
                    line.clone_into(&mut message);
                }
                break;
            }
        }

        let widget_path = if widget_path < stderr.len() {
            stderr[widget_path..].lines().next().unwrap().trim()
        } else {
            ""
        };

        let backtrace = if stack_backtrace < stderr.len() {
            let mut i = stack_backtrace;
            'backtrace_seek: for line in stderr[stack_backtrace..].lines() {
                let s = line.trim_start();
                if s.is_empty() {
                    break;
                } else if !s.starts_with("at ") {
                    for c in s.chars() {
                        if !c.is_ascii_digit() {
                            if c != ':' {
                                break 'backtrace_seek;
                            }
                            break;
                        }
                    }
                }

                // matches "\s*\d+:" OR "\s*at "
                i += line.len() + "\n".len();
            }
            &stderr[stack_backtrace..i]
        } else {
            ""
        };

        Some(Self {
            thread: thread_name.to_txt(),
            message: message.into(),
            file: file.to_txt(),
            line,
            column,
            widget_path: widget_path.to_txt(),
            backtrace: backtrace.to_txt(),
        })
    }

    /// Iterate over frames parsed from the `backtrace`.
    pub fn backtrace_frames(&self) -> impl Iterator<Item = BacktraceFrame> + '_ {
        BacktraceFrame::parse(&self.backtrace)
    }
}

/// Represents a frame parsed from a stack backtrace.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct BacktraceFrame {
    /// Position on the backtrace.
    pub n: usize,

    /// Function name.
    pub name: Txt,
    /// Source code file.
    pub file: Txt,
    /// Source code line.
    pub line: u32,

    /// If this frame is inside the Rust panic code.
    pub is_after_panic: bool,
}
impl fmt::Display for BacktraceFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{:>4}: {}", self.n, self.name)?;
        if !self.file.is_empty() {
            writeln!(f, "      at {}:{}", self.file, self.line)?;
        }
        Ok(())
    }
}
impl BacktraceFrame {
    /// Iterate over frames parsed from the `backtrace`.
    pub fn parse(mut backtrace: &str) -> impl Iterator<Item = BacktraceFrame> + '_ {
        let mut is_after_panic = backtrace.lines().any(|l| l.ends_with("core::panicking::panic_fmt"));
        std::iter::from_fn(move || {
            if backtrace.is_empty() {
                None
            } else {
                let n_name = backtrace.lines().next().unwrap();
                let (n, name) = if let Some((n, name)) = n_name.split_once(':') {
                    let n = match n.trim_start().parse() {
                        Ok(n) => n,
                        Err(_) => {
                            backtrace = "";
                            return None;
                        }
                    };
                    let name = name.trim();
                    if name.is_empty() {
                        backtrace = "";
                        return None;
                    }
                    (n, name)
                } else {
                    backtrace = "";
                    return None;
                };

                backtrace = &backtrace[n_name.len() + 1..];
                let r = if backtrace.trim_start().starts_with("at ") {
                    let file_line = backtrace.lines().next().unwrap();
                    let (file, line) = if let Some((file, line)) = file_line.rsplit_once(':') {
                        let file = file.trim_start().strip_prefix("at ").unwrap();
                        let line = match line.trim_end().parse() {
                            Ok(l) => l,
                            Err(_) => {
                                backtrace = "";
                                return None;
                            }
                        };
                        (file, line)
                    } else {
                        backtrace = "";
                        return None;
                    };

                    backtrace = &backtrace[file_line.len() + 1..];

                    BacktraceFrame {
                        n,
                        name: name.to_txt(),
                        file: file.to_txt(),
                        line,
                        is_after_panic,
                    }
                } else {
                    BacktraceFrame {
                        n,
                        name: name.to_txt(),
                        file: Txt::from(""),
                        line: 0,
                        is_after_panic,
                    }
                };

                if is_after_panic && name.ends_with("core::panicking::panic_fmt") {
                    is_after_panic = false;
                }

                Some(r)
            }
        })
    }

    /// Reads the code line + four surrounding lines if the code file can be found.
    pub fn code_snippet(&self) -> Txt {
        if !self.file.is_empty()
            && self.line > 0
            && let Ok(file) = std::fs::File::open(&self.file)
        {
            use std::fmt::Write as _;
            let mut r = String::new();

            let reader = std::io::BufReader::new(file);

            let line_s = self.line - 2.min(self.line - 1);
            let lines = reader.lines().skip(line_s as usize - 1).take(5);
            for (line, line_n) in lines.zip(line_s..) {
                let line = match line {
                    Ok(l) => l,
                    Err(_) => return Txt::from(""),
                };

                if line_n == self.line {
                    writeln!(&mut r, "      {line_n:>4} > {line}").unwrap();
                } else {
                    writeln!(&mut r, "      {line_n:>4} │ {line}").unwrap();
                }
            }

            return r.into();
        }
        Txt::from("")
    }
}
impl PanicInfo {
    /// Set a panic hook that will print panics to stderr in a format compatible with [`PanicInfo`] parsing.
    ///
    /// The `widget_trace_path` should be a closure that return `WIDGET.trace_path()` if the process can run
    /// an `APP`, otherwise it must be `Txt::default`.
    pub fn set_hook(widget_trace_path: impl Fn() -> Txt + Send + Sync + 'static) {
        std::panic::set_hook(Box::new(move |a| {
            let backtrace = std::backtrace::Backtrace::capture();
            let path = widget_trace_path();
            let panic = PanicFromHook::from_hook(a);
            eprintln!("{panic}widget path:\n   {path}\nstack backtrace:\n{backtrace}");
        }));
    }
}

#[derive(Debug)]
struct PanicFromHook {
    pub thread: Txt,
    pub msg: Txt,
    pub file: Txt,
    pub line: u32,
    pub column: u32,
}
impl PanicFromHook {
    pub fn from_hook(info: &std::panic::PanicHookInfo) -> Self {
        let current_thread = std::thread::current();
        let thread = match current_thread.name() {
            Some(n) => n.to_txt(),
            None => formatx!("{:?}", std::thread::current().id()),
        };
        let msg = Self::payload(info.payload());

        let (file, line, column) = if let Some(l) = info.location() {
            (l.file(), l.line(), l.column())
        } else {
            ("<unknown>", 0, 0)
        };
        Self {
            thread: thread.to_txt(),
            msg,
            file: file.to_txt(),
            line,
            column,
        }
    }

    fn payload(p: &dyn std::any::Any) -> Txt {
        match p.downcast_ref::<&'static str>() {
            Some(s) => s,
            None => match p.downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<dyn Any>",
            },
        }
        .to_txt()
    }
}
impl std::error::Error for PanicFromHook {}
impl fmt::Display for PanicFromHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "thread '{}' panicked at {}:{}:{}:",
            self.thread, self.file, self.line, self.column
        )?;
        for line in self.msg.lines() {
            writeln!(f, "   {line}")?;
        }
        Ok(())
    }
}
