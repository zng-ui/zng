#![cfg(feature = "crash_handler")]

//! App-process crash handler.
//!
//! See the `zng::app::crash_handler` documentation for more details.

use parking_lot::Mutex;
use std::{
    fmt,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    time::SystemTime,
};
use zng_layout::unit::TimeUnits as _;

use zng_txt::{ToTxt as _, Txt};

/// Starts the current app-process in a monitored instance.
///
/// This function takes over the first process turning it into the monitor-process, it spawns another process that
/// is the monitored app-process. If the app-process crashes it spawns a dialog-process that calls the dialog handler
/// to show an error message, upload crash reports, etc.
pub fn init(config: CrashConfig) {
    if std::env::var(APP_PROCESS) != Err(std::env::VarError::NotPresent) {
        return crash_handler_app_process(config.dump_dir.as_deref());
    }

    match std::env::var(DIALOG_PROCESS) {
        Ok(args_file) => crash_handler_dialog_process(config.dump_dir.as_deref(), config.dialog, args_file),
        Err(e) => match e {
            std::env::VarError::NotPresent => {}
            e => panic!("invalid dialog env args, {e:?}"),
        },
    }

    crash_handler_monitor_process(config.cfg_app, config.cfg_dialog);
}

/// Gets the number of crash restarts in the app-process.
///
/// Always returns zero if called in other processes.
pub fn restart_count() -> usize {
    match std::env::var(APP_PROCESS) {
        Ok(c) => c.strip_prefix("restart-").unwrap_or("0").parse().unwrap_or(0),
        Err(_) => 0,
    }
}

const APP_PROCESS: &str = "ZNG_CRASH_HANDLER_APP";
const DIALOG_PROCESS: &str = "ZNG_CRASH_HANDLER_DIALOG";
const RESPONSE_PREFIX: &str = "zng_crash_response: ";

type ConfigProcess = Vec<Box<dyn for<'a, 'b> FnMut(&'a mut std::process::Command, &'b CrashArgs) -> &'a mut std::process::Command>>;

/// Crash handler config.
pub struct CrashConfig {
    dialog: fn(CrashArgs) -> !,
    cfg_app: ConfigProcess,
    cfg_dialog: ConfigProcess,
    dump_dir: Option<PathBuf>,
}
impl CrashConfig {
    /// New with function called in the dialog-process.
    pub fn new(dialog: fn(CrashArgs) -> !) -> Self {
        Self {
            dialog,
            cfg_app: vec![],
            cfg_dialog: vec![],
            dump_dir: Some(std::env::temp_dir().join("zng_minidump")),
        }
    }

    /// Add a closure that is called just before the app-process is spawned.
    pub fn cfg_app(
        mut self,
        cfg: impl for<'a, 'b> FnMut(&'a mut std::process::Command, &'b CrashArgs) -> &'a mut std::process::Command + 'static,
    ) -> Self {
        self.cfg_app.push(Box::new(cfg));
        self
    }

    /// Add a closure that is called just before the dialog-process is spawned.
    pub fn cfg_dialog(
        mut self,
        cfg: impl for<'a, 'b> FnMut(&'a mut std::process::Command, &'b CrashArgs) -> &'a mut std::process::Command + 'static,
    ) -> Self {
        self.cfg_dialog.push(Box::new(cfg));
        self
    }

    /// Change the minidump directory.
    ///
    /// Is the temp dir by default.
    pub fn minidump_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.dump_dir = Some(dir.into());
        self
    }

    /// Do not collect a minidump.
    pub fn no_minidump(mut self) -> Self {
        self.dump_dir = None;
        self
    }
}
impl From<fn(CrashArgs) -> !> for CrashConfig {
    fn from(dialog: fn(CrashArgs) -> !) -> Self {
        Self::new(dialog)
    }
}

/// Arguments for the crash handler dialog function.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CrashArgs {
    /// Info about the app-process crashes.
    ///
    /// Has at least one entry, latest is last. Includes all crashes since the start of the monitor-process.
    pub app_crashes: Vec<CrashError>,

    /// Info about a crash in the dialog-process spawned to handle the latest app-process crash.
    ///
    /// If set this is the last chance to show something to the end user, if the current dialog crashes too
    /// the monitor-process will give up. If you started an `APP` to show a crash dialog try using a native
    /// dialog directly now, or just give up, clearly things are far from ok.
    pub dialog_crash: Option<CrashError>,
}
impl CrashArgs {
    /// Latest crash.
    pub fn latest(&self) -> &CrashError {
        self.app_crashes.last().unwrap()
    }

    /// Restart the app-process with same argument as the latest crash.
    pub fn restart(&self) -> ! {
        let json_args = serde_json::to_string(&self.latest().args[..]).unwrap();
        println!("{RESPONSE_PREFIX}restart {json_args}");
        std::process::exit(0)
    }

    /// Restart the app-process with custom arguments.
    pub fn restart_with(&self, args: &[Txt]) -> ! {
        let json_args = serde_json::to_string(&args).unwrap();
        println!("{RESPONSE_PREFIX}restart {json_args}");
        std::process::exit(0)
    }

    /// Exit the monitor-process (application) with code.
    pub fn exit(&self, code: i32) -> ! {
        println!("{RESPONSE_PREFIX}exit {code}");
        std::process::exit(0)
    }
}
impl fmt::Display for CrashArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "APP CRASHES:\n")?;

        for c in self.app_crashes.iter() {
            writeln!(f, "{c}")?;
        }

        if let Some(c) = &self.dialog_crash {
            writeln!(f, "\nDIALOG CRASH:\n")?;
            writeln!(f, "{c}")?;
        }

        Ok(())
    }
}

/// Info about an app-process crash.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrashError {
    /// Crash moment.
    pub timestamp: SystemTime,
    /// Process exit code.
    pub code: Option<i32>,
    /// Unix signal that terminated the process.
    pub signal: Option<i32>,
    /// Full capture of the app stdout.
    pub stdout: Txt,
    /// Full capture of the app stderr.
    pub stderr: Txt,
    /// Arguments used.
    pub args: Box<[Txt]>,
    /// Minidump file.
    pub minidump: Option<PathBuf>,
}
/// Alternate mode `{:#}` prints plain stdout and stderr (no ANSI escape sequences).
impl fmt::Display for CrashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "timestamp: {}", self.unix_time())?;
        if let Some(c) = self.code {
            writeln!(f, "exit code: {c:#X}")?
        }
        if let Some(c) = self.signal {
            writeln!(f, "exit signal: {c}")?
        }
        if let Some(p) = self.minidump.as_ref() {
            writeln!(f, "minidump: {}", p.display())?
        }
        if f.alternate() {
            write!(f, "\nSTDOUT:\n{}\nSTDERR:\n{}\n", self.stdout_plain(), self.stderr_plain())
        } else {
            write!(f, "\nSTDOUT:\n{}\nSTDERR:\n{}\n", self.stdout, self.stderr)
        }
    }
}
impl CrashError {
    fn new(timestamp: SystemTime, code: Option<i32>, signal: Option<i32>, stdout: Txt, stderr: Txt, args: Box<[Txt]>) -> Self {
        let mut minidump = None;

        for line in stdout.lines().rev() {
            if let Some(response) = line.strip_prefix(RESPONSE_PREFIX) {
                if let Some(path) = response.strip_prefix("minidump ") {
                    let path = PathBuf::from(path);
                    if let Ok(p) = path.canonicalize() {
                        minidump = Some(p);
                    }
                    break;
                }
            }
        }

        Self {
            timestamp,
            code,
            signal,
            stdout,
            stderr,
            args,
            minidump,
        }
    }

    /// Seconds since Unix epoch.
    pub fn unix_time(&self) -> u64 {
        self.timestamp.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs()
    }

    /// Gets if `stdout` does not contain any ANSI scape sequences.
    pub fn is_stdout_plain(&self) -> bool {
        !self.stdout.contains(CSI)
    }

    /// Gets if `stderr` does not contain any ANSI scape sequences.
    pub fn is_stderr_plain(&self) -> bool {
        !self.stderr.contains(CSI)
    }

    /// Get `stdout` without any ANSI escape sequences (CSI).
    pub fn stdout_plain(&self) -> Txt {
        remove_ansi_csi(&self.stdout)
    }

    /// Get `stderr` without any ANSI escape sequences (CSI).
    pub fn stderr_plain(&self) -> Txt {
        remove_ansi_csi(&self.stderr)
    }

    /// Gets if `stderr` contains a crash panic.
    pub fn has_panic(&self) -> bool {
        if self.code == Some(101) {
            CrashPanic::contains(&self.stderr_plain())
        } else {
            false
        }
    }

    /// Gets if `stderr` contains a crash panic that traced widget/window path.
    pub fn has_panic_widget(&self) -> bool {
        if self.code == Some(101) {
            CrashPanic::contains_widget(&self.stderr_plain())
        } else {
            false
        }
    }

    /// Try parse `stderr` for the crash panic.
    ///
    /// Only reliably works if the panic fully printed correctly and was formatted by the panic
    /// hook installed by `crash_handler` or by the display print of [`CrashPanic`].
    pub fn find_panic(&self) -> Option<CrashPanic> {
        if self.code == Some(101) {
            CrashPanic::find(&self.stderr_plain())
        } else {
            None
        }
    }

    /// Best attempt at generating a readable error message.
    ///
    /// Is the panic message, or the minidump exception, with the exit code and signal.
    pub fn message(&self) -> Txt {
        let mut msg = if let Some(msg) = self.find_panic().map(|p| p.message) {
            msg
        } else if let Some(msg) = self.minidump_message() {
            msg
        } else {
            "".into()
        };
        use std::fmt::Write as _;

        if let Some(c) = self.code {
            let sep = if msg.is_empty() { "" } else { "\n" };
            write!(&mut msg, "{sep}Code: {c:#X}").unwrap();
        }
        if let Some(c) = self.signal {
            let sep = if msg.is_empty() { "" } else { "\n" };
            write!(&mut msg, "{sep}Signal: {c}").unwrap();
        }
        msg.end_mut();
        msg
    }

    fn minidump_message(&self) -> Option<Txt> {
        use minidump::*;

        let dump = match Minidump::read_path(self.minidump.as_ref()?) {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("error reading minidump, {e}");
                return None;
            }
        };

        let system_info = match dump.get_stream::<MinidumpSystemInfo>() {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("error reading minidump system info, {e}");
                return None;
            }
        };
        let exception = match dump.get_stream::<MinidumpException>() {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("error reading minidump exception, {e}");
                return None;
            }
        };

        let crash_reason = exception.get_crash_reason(system_info.os, system_info.cpu);

        Some(zng_txt::formatx!("{crash_reason}"))
    }
}

const CSI: &str = "\x1b[";

/// Remove ANSI escape sequences (CSI) from `s`.
pub fn remove_ansi_csi(mut s: &str) -> Txt {
    fn is_esc_end(byte: u8) -> bool {
        (0x40..=0x7e).contains(&byte)
    }

    let mut r = String::new();
    while let Some(i) = s.find(CSI) {
        r.push_str(&s[..i]);
        s = &s[i + CSI.len()..];
        let mut esc_end = 0;
        while esc_end < s.len() && !is_esc_end(s.as_bytes()[esc_end]) {
            esc_end += 1;
        }
        esc_end += 1;
        s = &s[esc_end..];
    }
    r.push_str(s);
    r.into()
}

/// Panic parsed from a `stderr` dump.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CrashPanic {
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
    pub widget_path: Txt,
    /// Stack backtrace.
    pub backtrace: Txt,
}

/// Alternate mode `{:#}` prints full backtrace.
impl fmt::Display for CrashPanic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "thread '{}' panicked at {}:{}:{}:",
            self.thread, self.file, self.line, self.column
        )?;
        for line in self.message.lines() {
            writeln!(f, "   {line}")?;
        }
        writeln!(f, "widget path:\n   {}", self.widget_path)?;

        if f.alternate() {
            writeln!(f, "stack backtrace:\n{}", self.backtrace)
        } else {
            writeln!(f, "stack backtrace:")?;
            let mut snippet = 9;
            for frame in self.backtrace_frames().skip_while(|f| f.is_after_panic) {
                write!(f, "{frame}")?;
                if snippet > 0 {
                    snippet -= 1;
                    let code = frame.code_snippet();
                    if code.is_empty() {
                        snippet = 0;
                        continue;
                    }
                    writeln!(f, "{}", code)?;
                }
            }
            Ok(())
        }
    }
}
impl CrashPanic {
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

    /// Try parse `stderr` for the crash panic.
    ///
    /// Only reliably works if the panic fully printed correctly and was formatted by the panic
    /// hook installed by `crash_handler` or by the display print of this type.
    pub fn find(stderr: &str) -> Option<Self> {
        Self::find_impl(stderr, true)
    }

    fn find_impl(stderr: &str, parse: bool) -> Option<Self> {
        let mut panic_at = usize::MAX;
        let mut widget_path = usize::MAX;
        let mut stack_backtrace = usize::MAX;
        let mut i = 0;
        for line in stderr.lines() {
            if line.starts_with("thread '") && line.contains("' panicked at ") && line.ends_with(':') {
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
        let (thread, location) = panic_str.strip_prefix("thread '").unwrap().split_once("' panicked at ").unwrap();
        let mut location = location.split(':');
        let file = location.next().unwrap_or("");
        let line: u32 = location.next().unwrap_or("0").parse().unwrap_or(0);
        let column: u32 = location.next().unwrap_or("0").parse().unwrap_or(0);

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
                    message = line.to_owned();
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
                if !line.starts_with(' ') {
                    'digit_check: for c in line.chars() {
                        if !c.is_ascii_digit() {
                            if c == ':' {
                                break 'digit_check;
                            } else {
                                break 'backtrace_seek;
                            }
                        }
                    }
                }
                i += line.len() + "\n".len();
            }
            &stderr[stack_backtrace..i]
        } else {
            ""
        };

        Some(Self {
            thread: thread.to_txt(),
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

                if is_after_panic && name == "core::panicking::panic_fmt" {
                    is_after_panic = false;
                }

                Some(r)
            }
        })
    }

    /// Reads the code line + four surrounding lines if the code file can be found.
    pub fn code_snippet(&self) -> Txt {
        if !self.file.is_empty() && self.line > 0 {
            if let Ok(file) = std::fs::File::open(&self.file) {
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
                        writeln!(&mut r, "      {:>4} > {}", line_n, line).unwrap();
                    } else {
                        writeln!(&mut r, "      {:>4} │ {}", line_n, line).unwrap();
                    }
                }

                return r.into();
            }
        }
        Txt::from("")
    }
}

fn crash_handler_monitor_process(mut cfg_app: ConfigProcess, mut cfg_dialog: ConfigProcess) -> ! {
    // monitor-process:
    tracing::info!("crash monitor-process is running");

    let exe = std::env::current_exe()
        .and_then(|p| p.canonicalize())
        .expect("failed to get the current executable");

    let mut args: Box<[_]> = std::env::args().map(Txt::from).collect();

    let mut dialog_args = CrashArgs {
        app_crashes: vec![],
        dialog_crash: None,
    };
    loop {
        let mut app_process = std::process::Command::new(&exe);
        for cfg in &mut cfg_app {
            cfg(&mut app_process, &dialog_args);
        }
        match run_process(
            app_process
                .env(APP_PROCESS, format!("restart-{}", dialog_args.app_crashes.len()))
                .args(args.iter()),
        ) {
            Ok((status, [stdout, stderr])) => {
                if status.success() {
                    let code = status.code().unwrap_or(0);
                    tracing::info!(
                        "crash monitor-process exiting with success code ({code}), {} crashes",
                        dialog_args.app_crashes.len()
                    );
                    std::process::exit(code);
                } else {
                    let code = status.code();
                    #[allow(unused_mut)]
                    let mut signal = None::<i32>;

                    #[cfg(windows)]
                    if code == Some(1) {
                        tracing::warn!(
                            "app-process exit code (1), probably killed by the system, \
                                        will exit monitor-process with the same code"
                        );
                        std::process::exit(1);
                    }
                    #[cfg(unix)]
                    if code.is_none() {
                        use std::os::unix::process::ExitStatusExt as _;
                        signal = status.signal();

                        if let Some(sig) = signal {
                            if [2, 9, 17, 19, 23].contains(&sig) {
                                tracing::warn!(
                                    "app-process exited by signal ({sig}), \
                                                will exit monitor-process with code 1"
                                );
                                std::process::exit(1);
                            }
                        }
                    }

                    tracing::error!(
                        "app-process crashed with exit code ({:#X}), signal ({:#?}), {} crashes previously",
                        code.unwrap_or(0),
                        signal.unwrap_or(0),
                        dialog_args.app_crashes.len()
                    );

                    let timestamp = SystemTime::now();

                    dialog_args
                        .app_crashes
                        .push(CrashError::new(timestamp, code, signal, stdout.into(), stderr.into(), args.clone()));

                    // show dialog, retries once if dialog crashes too.
                    for _ in 0..2 {
                        // serialize app-crashes to a temp JSON file
                        let timestamp_nanos = timestamp.duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
                        let mut timestamp = timestamp_nanos;
                        let mut retries = 0;
                        let crash_file = loop {
                            let path = std::env::temp_dir().join(format!("zng-crash-errors-{timestamp:#x}"));
                            match std::fs::File::create_new(&path) {
                                Ok(f) => match serde_json::to_writer(std::io::BufWriter::new(f), &dialog_args) {
                                    Ok(_) => break path,
                                    Err(e) => {
                                        if e.is_io() {
                                            if retries > 20 {
                                                panic!("error writing crash errors, {e}");
                                            } else if retries > 5 {
                                                timestamp += 1;
                                            }
                                            std::thread::sleep(100.ms());
                                        } else {
                                            panic!("error serializing crash errors, {e}");
                                        }
                                    }
                                },
                                Err(e) => {
                                    if e.kind() == std::io::ErrorKind::AlreadyExists {
                                        timestamp += 1;
                                    } else {
                                        if retries > 20 {
                                            panic!("error creating crash errors file, {e}");
                                        } else if retries > 5 {
                                            timestamp += 1;
                                        }
                                        std::thread::sleep(100.ms());
                                    }
                                }
                            }
                            retries += 1;
                        };

                        let mut dialog_process = std::process::Command::new(&exe);
                        for cfg in &mut cfg_dialog {
                            cfg(&mut dialog_process, &dialog_args);
                        }
                        let dialog_result = run_process(dialog_process.env(DIALOG_PROCESS, &crash_file));

                        for _ in 0..5 {
                            if !crash_file.exists() || std::fs::remove_file(&crash_file).is_ok() {
                                break;
                            }
                            std::thread::sleep(100.ms());
                        }

                        let response = match dialog_result {
                            Ok((dlg_status, [dlg_stdout, dlg_stderr])) => {
                                if dlg_status.success() {
                                    dlg_stdout
                                        .lines()
                                        .filter_map(|l| l.trim().strip_prefix(RESPONSE_PREFIX))
                                        .last()
                                        .expect("crash dialog-process did not respond correctly")
                                        .to_owned()
                                } else {
                                    let code = dlg_status.code();
                                    #[allow(unused_mut)]
                                    let mut signal = None::<i32>;

                                    #[cfg(windows)]
                                    if code == Some(1) {
                                        tracing::warn!(
                                            "dialog-process exit code (1), probably killed by the system, \
                                                        will exit monitor-process with the same code"
                                        );
                                        std::process::exit(1);
                                    }
                                    #[cfg(unix)]
                                    if code.is_none() {
                                        use std::os::unix::process::ExitStatusExt as _;
                                        signal = status.signal();

                                        if let Some(sig) = signal {
                                            if [2, 9, 17, 19, 23].contains(&sig) {
                                                tracing::warn!(
                                                    "dialog-process exited by signal ({sig}), \
                                                                will exit monitor-process with code 1"
                                                );
                                                std::process::exit(1);
                                            }
                                        }
                                    }

                                    let dialog_crash = CrashError::new(
                                        SystemTime::now(),
                                        code,
                                        signal,
                                        dlg_stdout.into(),
                                        dlg_stderr.into(),
                                        Box::new([]),
                                    );
                                    tracing::error!("crash dialog-process crashed, {dialog_crash}");

                                    if dialog_args.dialog_crash.is_none() {
                                        dialog_args.dialog_crash = Some(dialog_crash);
                                        continue;
                                    } else {
                                        let latest = dialog_args.latest();
                                        eprintln!("{latest}");
                                        std::process::exit(latest.code.unwrap_or(1));
                                    }
                                }
                            }
                            Err(e) => panic!("error running dialog-process, {e}"),
                        };

                        if let Some(args_json) = response.strip_prefix("restart ") {
                            args = serde_json::from_str(args_json).expect("crash dialog-process did not respond 'restart' correctly");
                            break;
                        } else if let Some(code) = response.strip_prefix("exit ") {
                            let code: i32 = code.parse().expect("crash dialog-process did not respond 'code' correctly");
                            std::process::exit(code);
                        } else {
                            panic!("crash dialog-process did not respond correctly")
                        }
                    }
                }
            }
            Err(e) => panic!("error running app-process, {e}"),
        }
    }
}
fn run_process(command: &mut std::process::Command) -> std::io::Result<(std::process::ExitStatus, [String; 2])> {
    let mut app_process = command
        .env("RUST_BACKTRACE", "full")
        .env("CLICOLOR_FORCE", "1")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let stdout = capture_and_print(app_process.stdout.take().unwrap(), false);
    let stderr = capture_and_print(app_process.stderr.take().unwrap(), false);

    let status = app_process.wait()?;

    let stdout = match stdout.join() {
        Ok(r) => r,
        Err(p) => std::panic::resume_unwind(p),
    };
    let stderr = match stderr.join() {
        Ok(r) => r,
        Err(p) => std::panic::resume_unwind(p),
    };

    Ok((status, [stdout, stderr]))
}
fn capture_and_print(stream: impl std::io::Read + Send + 'static, is_err: bool) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut capture = String::new();

        let mut reader = BufReader::new(stream);

        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(n) => {
                    if n > 0 {
                        if is_err {
                            eprint!("{line}");
                        } else {
                            print!("{line}");
                        }
                        capture.push_str(&line);
                        line.clear();
                    } else {
                        break;
                    }
                }
                Err(e) => panic!("{} read error, {}", if is_err { "stderr" } else { "stdout" }, e),
            }
        }

        capture
    })
}

fn crash_handler_app_process(dump_dir: Option<&Path>) {
    tracing::info!("app-process is running");

    std::panic::set_hook(Box::new(panic_handler));
    if let Some(dir) = dump_dir {
        if let Err(e) = std::fs::create_dir_all(dir) {
            tracing::error!("failed to create minidump dir, minidump may not collect on crash, {e}");
        }
        minidump_attach(dir);
    }

    // app-process execution happens after the `crash_handler` function returns.
}

fn crash_handler_dialog_process(dump_dir: Option<&Path>, dialog: fn(CrashArgs) -> !, args_file: String) -> ! {
    tracing::info!("crash dialog-process is running");

    std::panic::set_hook(Box::new(panic_handler));
    if let Some(dir) = dump_dir {
        minidump_attach(dir);
    }

    let mut retries = 0;
    let args = loop {
        match std::fs::read_to_string(&args_file) {
            Ok(args) => break args,
            Err(e) => {
                if e.kind() != std::io::ErrorKind::NotFound && retries < 10 {
                    retries += 1;
                    continue;
                }
                panic!("error reading args file, {e}");
            }
        }
    };

    dialog(serde_json::from_str(&args).expect("error deserializing args"))
}

fn panic_handler(info: &std::panic::PanicInfo) {
    let backtrace = std::backtrace::Backtrace::capture();
    let path = crate::widget::WIDGET.trace_path();
    let panic = PanicInfo::from_hook(info);
    eprintln!("{panic}widget path:\n   {path}\nstack backtrace:\n{backtrace}");
}

fn minidump_attach(dump_dir: &Path) {
    let handler = breakpad_handler::BreakpadHandler::attach(
        dump_dir,
        breakpad_handler::InstallOptions::BothHandlers,
        Box::new(|minidump_path: std::path::PathBuf| {
            println!("{RESPONSE_PREFIX}minidump {}", minidump_path.display());
        }),
    )
    .unwrap();
    *BREAKPAD_HANDLER.lock() = Some(handler);
}
static BREAKPAD_HANDLER: Mutex<Option<breakpad_handler::BreakpadHandler>> = Mutex::new(None);

#[derive(Debug)]
struct PanicInfo {
    pub thread: Txt,
    pub msg: Txt,
    pub file: Txt,
    pub line: u32,
    pub column: u32,
}
impl PanicInfo {
    pub fn from_hook(info: &std::panic::PanicInfo) -> Self {
        let current_thread = std::thread::current();
        let thread = current_thread.name().unwrap_or("<unnamed>");
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
impl std::error::Error for PanicInfo {}
impl fmt::Display for PanicInfo {
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
