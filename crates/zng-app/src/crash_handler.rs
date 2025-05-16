#![cfg(all(
    feature = "crash_handler",
    not(any(target_arch = "wasm32", target_os = "android", target_os = "ios"))
))]

//! App-process crash handler.
//!
//! See the `zng::app::crash_handler` documentation for more details.

use parking_lot::Mutex;
use std::{
    fmt,
    io::{BufRead, Write},
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
    time::SystemTime,
};
use zng_clone_move::clmv;
use zng_layout::unit::TimeUnits as _;

use zng_txt::{ToTxt as _, Txt};

/// Environment variable that causes the crash handler to not start if set.
///
/// This is particularly useful to set in debugger launch configs. Crash handler spawns
/// a different process for the app  so break points will not work.
pub const NO_CRASH_HANDLER: &str = "ZNG_NO_CRASH_HANDLER";

zng_env::on_process_start!(|process_start_args| {
    if std::env::var(NO_CRASH_HANDLER).is_ok() {
        return;
    }

    let mut config = CrashConfig::new();
    for ext in CRASH_CONFIG {
        ext(&mut config);
        if config.no_crash_handler {
            return;
        }
    }

    if process_start_args.next_handlers_count > 0 && process_start_args.yield_count < zng_env::ProcessStartArgs::MAX_YIELD_COUNT - 10 {
        // extra sure that this is the app-process
        return process_start_args.yield_once();
    }

    if std::env::var(APP_PROCESS) != Err(std::env::VarError::NotPresent) {
        return crash_handler_app_process(config.dump_dir.is_some());
    }

    match std::env::var(DIALOG_PROCESS) {
        Ok(args_file) => crash_handler_dialog_process(
            config.dump_dir.is_some(),
            config
                .dialog
                .or(config.default_dialog)
                .expect("dialog-process spawned without dialog handler"),
            args_file,
        ),
        Err(e) => match e {
            std::env::VarError::NotPresent => {}
            e => panic!("invalid dialog env args, {e:?}"),
        },
    }

    crash_handler_monitor_process(
        config.dump_dir,
        config.app_process,
        config.dialog_process,
        config.default_dialog.is_some() || config.dialog.is_some(),
    );
});

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
const DUMP_CHANNEL: &str = "ZNG_MINIDUMP_CHANNEL";
const RESPONSE_PREFIX: &str = "zng_crash_response: ";

#[linkme::distributed_slice]
static CRASH_CONFIG: [fn(&mut CrashConfig)];

/// <span data-del-macro-root></span> Register a `FnOnce(&mut CrashConfig)` closure to be
/// called on process init to configure the crash handler.
///
/// See [`CrashConfig`] for more details.
#[macro_export]
macro_rules! crash_handler_config {
    ($closure:expr) => {
        // expanded from:
        // #[linkme::distributed_slice(CRASH_CONFIG)]
        // static _CRASH_CONFIG: fn(&FooArgs) = _foo;
        // so that users don't need to depend on linkme just to call this macro.
        #[used]
        #[cfg_attr(
            any(
                target_os = "none",
                target_os = "linux",
                target_os = "android",
                target_os = "fuchsia",
                target_os = "psp"
            ),
            unsafe(link_section = "linkme_CRASH_CONFIG")
        )]
        #[cfg_attr(
            any(target_os = "macos", target_os = "ios", target_os = "tvos"),
            unsafe(link_section = "__DATA,__linkmeK3uV0Fq0,regular,no_dead_strip")
        )]
        #[cfg_attr(
            any(target_os = "uefi", target_os = "windows"),
            unsafe(link_section = ".linkme_CRASH_CONFIG$b")
        )]
        #[cfg_attr(target_os = "illumos", unsafe(link_section = "set_linkme_CRASH_CONFIG"))]
        #[cfg_attr(
            any(target_os = "freebsd", target_os = "openbsd"),
            unsafe(link_section = "linkme_CRASH_CONFIG")
        )]
        #[doc(hidden)]
        static _CRASH_CONFIG: fn(&mut $crate::crash_handler::CrashConfig) = _crash_config;
        #[doc(hidden)]
        fn _crash_config(cfg: &mut $crate::crash_handler::CrashConfig) {
            fn crash_config(cfg: &mut $crate::crash_handler::CrashConfig, handler: impl FnOnce(&mut $crate::crash_handler::CrashConfig)) {
                handler(cfg)
            }
            crash_config(cfg, $closure)
        }
    };
}
pub use crate::crash_handler_config;

type ConfigProcess = Vec<Box<dyn for<'a, 'b> FnMut(&'a mut std::process::Command, &'b CrashArgs) -> &'a mut std::process::Command>>;
type CrashDialogHandler = Box<dyn FnOnce(CrashArgs)>;

/// Crash handler config.
///
/// Use [`crash_handler_config!`] to set config.
///
/// [`crash_handler_config!`]: crate::crash_handler_config!
pub struct CrashConfig {
    default_dialog: Option<CrashDialogHandler>,
    dialog: Option<CrashDialogHandler>,
    app_process: ConfigProcess,
    dialog_process: ConfigProcess,
    dump_dir: Option<PathBuf>,
    no_crash_handler: bool,
}
impl CrashConfig {
    fn new() -> Self {
        Self {
            default_dialog: None,
            dialog: None,
            app_process: vec![],
            dialog_process: vec![],
            dump_dir: Some(zng_env::cache("zng_minidump")),
            no_crash_handler: false,
        }
    }

    /// Set the crash dialog process handler.
    ///
    /// The dialog `handler` can run an app or show a native dialog, it must use the [`CrashArgs`] process
    /// terminating methods to respond, if it returns [`CrashArgs::exit`] will run.
    ///
    /// Note that the handler does not need to actually show any dialog, it can just save crash info and
    /// restart the app for example.
    pub fn dialog(&mut self, handler: impl FnOnce(CrashArgs) + 'static) {
        if self.dialog.is_none() {
            self.dialog = Some(Box::new(handler));
        }
    }

    /// Set the crash dialog-handler used if `crash_dialog` is not set.
    ///
    /// This is used by app libraries or themes to provide a default dialog.
    pub fn default_dialog(&mut self, handler: impl FnOnce(CrashArgs) + 'static) {
        self.default_dialog = Some(Box::new(handler));
    }

    /// Add a closure that is called just before the app-process is spawned.
    pub fn app_process(
        &mut self,
        cfg: impl for<'a, 'b> FnMut(&'a mut std::process::Command, &'b CrashArgs) -> &'a mut std::process::Command + 'static,
    ) {
        self.app_process.push(Box::new(cfg));
    }

    /// Add a closure that is called just before the dialog-process is spawned.
    pub fn dialog_process(
        &mut self,
        cfg: impl for<'a, 'b> FnMut(&'a mut std::process::Command, &'b CrashArgs) -> &'a mut std::process::Command + 'static,
    ) {
        self.dialog_process.push(Box::new(cfg));
    }

    /// Change the minidump directory.
    ///
    /// Is `zng::env::cache("zng_minidump")` by default.
    pub fn minidump_dir(&mut self, dir: impl Into<PathBuf>) {
        self.dump_dir = Some(dir.into());
    }

    /// Do not collect a minidump.
    pub fn no_minidump(&mut self) {
        self.dump_dir = None;
    }

    /// Does not run with crash handler.
    ///
    /// This is equivalent of running with `NO_ZNG_CRASH_HANDLER` env var.
    pub fn no_crash_handler(&mut self) {
        self.no_crash_handler = true;
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
        zng_env::exit(0)
    }

    /// Restart the app-process with custom arguments.
    pub fn restart_with(&self, args: &[Txt]) -> ! {
        let json_args = serde_json::to_string(&args).unwrap();
        println!("{RESPONSE_PREFIX}restart {json_args}");
        zng_env::exit(0)
    }

    /// Exit the monitor-process (application) with code.
    pub fn exit(&self, code: i32) -> ! {
        println!("{RESPONSE_PREFIX}exit {code}");
        zng_env::exit(0)
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
    /// Operating system.
    ///
    /// See [`std::env::consts::OS`] for details.
    pub os: Txt,
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
    fn new(
        timestamp: SystemTime,
        code: Option<i32>,
        signal: Option<i32>,
        stdout: Txt,
        stderr: Txt,
        minidump: Option<PathBuf>,
        args: Box<[Txt]>,
    ) -> Self {
        Self {
            timestamp,
            code,
            signal,
            stdout,
            stderr,
            args,
            minidump,
            os: std::env::consts::OS.into(),
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
                    let code = frame.code_snippet();
                    if !code.is_empty() {
                        snippet -= 1;
                        writeln!(f, "{}", code)?;
                    }
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

fn crash_handler_monitor_process(
    dump_dir: Option<PathBuf>,
    mut cfg_app: ConfigProcess,
    mut cfg_dialog: ConfigProcess,
    has_dialog_handler: bool,
) -> ! {
    // monitor-process:
    tracing::info!("crash monitor-process is running");

    let exe = std::env::current_exe()
        .and_then(dunce::canonicalize)
        .expect("failed to get the current executable");

    let mut args: Box<[_]> = std::env::args().skip(1).map(Txt::from).collect();

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
            dump_dir.as_deref(),
            app_process
                .env(APP_PROCESS, format!("restart-{}", dialog_args.app_crashes.len()))
                .args(args.iter()),
        ) {
            Ok((status, [stdout, stderr], dump_file)) => {
                if status.success() {
                    let code = status.code().unwrap_or(0);
                    tracing::info!(
                        "crash monitor-process exiting with success code ({code}), {} crashes",
                        dialog_args.app_crashes.len()
                    );
                    zng_env::exit(code);
                } else {
                    let code = status.code();
                    #[allow(unused_mut)] // Windows has no signal
                    let mut signal = None::<i32>;

                    #[cfg(windows)]
                    if code == Some(1) {
                        tracing::warn!(
                            "app-process exit code (1), probably killed by the system, \
                                        will exit monitor-process with the same code"
                        );
                        zng_env::exit(1);
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
                                zng_env::exit(1);
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

                    dialog_args.app_crashes.push(CrashError::new(
                        timestamp,
                        code,
                        signal,
                        stdout.into(),
                        stderr.into(),
                        dump_file,
                        args.clone(),
                    ));

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

                        let dialog_result = if has_dialog_handler {
                            let mut dialog_process = std::process::Command::new(&exe);
                            for cfg in &mut cfg_dialog {
                                cfg(&mut dialog_process, &dialog_args);
                            }
                            run_process(dump_dir.as_deref(), dialog_process.env(DIALOG_PROCESS, &crash_file))
                        } else {
                            Ok((std::process::ExitStatus::default(), [String::new(), String::new()], None))
                        };

                        for _ in 0..5 {
                            if !crash_file.exists() || std::fs::remove_file(&crash_file).is_ok() {
                                break;
                            }
                            std::thread::sleep(100.ms());
                        }

                        let response = match dialog_result {
                            Ok((dlg_status, [dlg_stdout, dlg_stderr], dlg_dump_file)) => {
                                if dlg_status.success() {
                                    dlg_stdout
                                        .lines()
                                        .filter_map(|l| l.trim().strip_prefix(RESPONSE_PREFIX))
                                        .next_back()
                                        .unwrap_or("exit 0")
                                        .to_owned()
                                } else {
                                    let code = dlg_status.code();
                                    #[allow(unused_mut)] // Windows has no signal
                                    let mut signal = None::<i32>;

                                    #[cfg(windows)]
                                    if code == Some(1) {
                                        tracing::warn!(
                                            "dialog-process exit code (1), probably killed by the system, \
                                                        will exit monitor-process with the same code"
                                        );
                                        zng_env::exit(1);
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
                                                zng_env::exit(1);
                                            }
                                        }
                                    }

                                    let dialog_crash = CrashError::new(
                                        SystemTime::now(),
                                        code,
                                        signal,
                                        dlg_stdout.into(),
                                        dlg_stderr.into(),
                                        dlg_dump_file,
                                        Box::new([]),
                                    );
                                    tracing::error!("crash dialog-process crashed, {dialog_crash}");

                                    if dialog_args.dialog_crash.is_none() {
                                        dialog_args.dialog_crash = Some(dialog_crash);
                                        continue;
                                    } else {
                                        let latest = dialog_args.latest();
                                        eprintln!("{latest}");
                                        zng_env::exit(latest.code.unwrap_or(1));
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
                            zng_env::exit(code);
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
fn run_process(
    dump_dir: Option<&Path>,
    command: &mut std::process::Command,
) -> std::io::Result<(std::process::ExitStatus, [String; 2], Option<PathBuf>)> {
    struct DumpServer {
        shutdown: Arc<AtomicBool>,
        runner: std::thread::JoinHandle<Option<PathBuf>>,
    }
    let mut dump_server = None;
    if let Some(dump_dir) = dump_dir {
        match std::fs::create_dir_all(dump_dir) {
            Ok(_) => {
                let uuid = uuid::Uuid::new_v4();
                let dump_file = dump_dir.join(format!("{}.dmp", uuid.simple()));
                let dump_channel = std::env::temp_dir().join(format!("zng-crash-{}", uuid.simple()));
                match minidumper::Server::with_name(dump_channel.as_path()) {
                    Ok(mut s) => {
                        command.env(DUMP_CHANNEL, &dump_channel);
                        let shutdown = Arc::new(AtomicBool::new(false));
                        let runner = std::thread::spawn(clmv!(shutdown, || {
                            let created_file = Arc::new(Mutex::new(None));
                            if let Err(e) = s.run(
                                Box::new(MinidumpServerHandler {
                                    dump_file,
                                    created_file: created_file.clone(),
                                }),
                                &shutdown,
                                None,
                            ) {
                                tracing::error!("minidump server exited with error, {e}");
                            }
                            created_file.lock().take()
                        }));
                        dump_server = Some(DumpServer { shutdown, runner });
                    }
                    Err(e) => tracing::error!("failed to spawn minidump server, will not enable crash handling, {e}"),
                }
            }
            Err(e) => tracing::error!("cannot create minidump dir, will not enable crash handling, {e}"),
        }
    }

    let mut app_process = command
        .env("RUST_BACKTRACE", "full")
        .env("CLICOLOR_FORCE", "1")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let stdout = capture_and_print(app_process.stdout.take().unwrap(), false);
    let stderr = capture_and_print(app_process.stderr.take().unwrap(), true);

    let status = app_process.wait()?;

    let stdout = match stdout.join() {
        Ok(r) => r,
        Err(p) => std::panic::resume_unwind(p),
    };
    let stderr = match stderr.join() {
        Ok(r) => r,
        Err(p) => std::panic::resume_unwind(p),
    };

    let mut dump_file = None;
    if let Some(s) = dump_server {
        s.shutdown.store(true, atomic::Ordering::Relaxed);
        match s.runner.join() {
            Ok(r) => dump_file = r,
            Err(p) => std::panic::resume_unwind(p),
        };
    }

    Ok((status, [stdout, stderr], dump_file))
}
struct MinidumpServerHandler {
    dump_file: PathBuf,
    created_file: Arc<Mutex<Option<PathBuf>>>,
}
impl minidumper::ServerHandler for MinidumpServerHandler {
    fn create_minidump_file(&self) -> Result<(std::fs::File, PathBuf), std::io::Error> {
        let file = std::fs::File::create_new(&self.dump_file)?;
        Ok((file, self.dump_file.clone()))
    }

    fn on_minidump_created(&self, result: Result<minidumper::MinidumpBinary, minidumper::Error>) -> minidumper::LoopAction {
        match result {
            Ok(b) => *self.created_file.lock() = Some(b.path),
            Err(e) => tracing::error!("failed to write minidump file, {e}"),
        }
        minidumper::LoopAction::Exit
    }

    fn on_message(&self, _: u32, _: Vec<u8>) {}

    fn on_client_connected(&self, num_clients: usize) -> minidumper::LoopAction {
        if num_clients > 1 {
            tracing::error!("expected only one minidump client, {num_clients} connected, exiting server");
            minidumper::LoopAction::Exit
        } else {
            minidumper::LoopAction::Continue
        }
    }

    fn on_client_disconnected(&self, num_clients: usize) -> minidumper::LoopAction {
        if num_clients != 0 {
            tracing::error!("expected only one minidump client disconnect, {num_clients} still connected");
        }
        minidumper::LoopAction::Exit
    }
}
fn capture_and_print(mut stream: impl std::io::Read + Send + 'static, is_err: bool) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut capture = vec![];
        let mut buffer = [0u8; 32];
        loop {
            match stream.read(&mut buffer) {
                Ok(n) => {
                    if n == 0 {
                        break;
                    }

                    let new = &buffer[..n];
                    capture.write_all(new).unwrap();
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
        }
        String::from_utf8_lossy(&capture).into_owned()
    })
}

fn crash_handler_app_process(dump_enabled: bool) {
    tracing::info!("app-process is running");

    std::panic::set_hook(Box::new(panic_handler));
    if dump_enabled {
        minidump_attach();
    }

    // app-process execution happens after this.
}

fn crash_handler_dialog_process(dump_enabled: bool, dialog: CrashDialogHandler, args_file: String) -> ! {
    tracing::info!("crash dialog-process is running");

    std::panic::set_hook(Box::new(panic_handler));
    if dump_enabled {
        minidump_attach();
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

    dialog(serde_json::from_str(&args).expect("error deserializing args"));
    CrashArgs {
        app_crashes: vec![],
        dialog_crash: None,
    }
    .exit(0)
}

fn panic_handler(info: &std::panic::PanicHookInfo) {
    let backtrace = std::backtrace::Backtrace::capture();
    let path = crate::widget::WIDGET.trace_path();
    let panic = PanicInfo::from_hook(info);
    eprintln!("{panic}widget path:\n   {path}\nstack backtrace:\n{backtrace}");
}

fn minidump_attach() {
    let channel_name = match std::env::var(DUMP_CHANNEL) {
        Ok(n) if !n.is_empty() => PathBuf::from(n),
        _ => {
            eprintln!("expected minidump channel name, this instance will not handle crashes");
            return;
        }
    };
    let client = match minidumper::Client::with_name(channel_name.as_path()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("failed to connect minidump client, this instance will not handle crashes, {e}");
            return;
        }
    };
    struct Handler(minidumper::Client);
    // SAFETY: on_crash does the minimal possible work
    unsafe impl crash_handler::CrashEvent for Handler {
        fn on_crash(&self, context: &crash_handler::CrashContext) -> crash_handler::CrashEventResult {
            crash_handler::CrashEventResult::Handled(self.0.request_dump(context).is_ok())
        }
    }
    let handler = match crash_handler::CrashHandler::attach(Box::new(Handler(client))) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("failed attach minidump crash handler, this instance will not handle crashes, {e}");
            return;
        }
    };

    *CRASH_HANDLER.lock() = Some(handler);
}
static CRASH_HANDLER: Mutex<Option<crash_handler::CrashHandler>> = Mutex::new(None);

#[derive(Debug)]
struct PanicInfo {
    pub thread: Txt,
    pub msg: Txt,
    pub file: Txt,
    pub line: u32,
    pub column: u32,
}
impl PanicInfo {
    pub fn from_hook(info: &std::panic::PanicHookInfo) -> Self {
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
