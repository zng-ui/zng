#![cfg(all(
    feature = "crash_handler",
    not(any(target_arch = "wasm32", target_os = "android", target_os = "ios"))
))]

//! App-process crash handler.
//!
//! See the `zng::app::crash_handler` documentation for more details.

use std::{
    fmt,
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
    time::SystemTime,
};
use zng_clone_move::clmv;
use zng_layout::unit::TimeUnits as _;
use zng_task::{parking_lot::Mutex, process::tap};

// TODO(breaking) remove this
use tap::contains_ansi_csi;
pub use tap::{BacktraceFrame, PanicInfo as CrashPanic, remove_ansi_csi};

use zng_txt::Txt;

/// Environment variable that causes the crash handler to not start if set.
///
/// This is particularly useful to set in debugger launch configs. Crash handler spawns
/// a different process for the app  so break points will not work.
pub const NO_CRASH_HANDLER: &str = "ZNG_NO_CRASH_HANDLER";

zng_env::on_process_start!(|process_start_args| {
    if std::env::var(NO_CRASH_HANDLER).is_ok() {
        return;
    }
    if zng_env::about().is_test {
        tracing::debug!("ignoring crash_handler because is test process");
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

#[doc(hidden)]
#[linkme::distributed_slice]
pub static CRASH_CONFIG: [fn(&mut CrashConfig)];

#[doc(hidden)]
pub use linkme as __linkme;

/// <span data-del-macro-root></span> Register a `FnOnce(&mut CrashConfig)` closure to be
/// called on process init to configure the crash handler.
///
/// See [`CrashConfig`] for more details.
#[macro_export]
macro_rules! crash_handler_config {
    ($closure:expr) => {
        // expanded from:
        #[$crate::crash_handler::__linkme::distributed_slice($crate::crash_handler::CRASH_CONFIG)]
        #[linkme(crate = $crate::crash_handler::__linkme)]
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
#[non_exhaustive]
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
#[non_exhaustive]
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
        !contains_ansi_csi(&self.stdout)
    }

    /// Gets if `stderr` does not contain any ANSI scape sequences.
    pub fn is_stderr_plain(&self) -> bool {
        !contains_ansi_csi(&self.stderr)
    }

    /// Get `stdout` without any ANSI escape sequences (CSI).
    pub fn stdout_plain(&self) -> Txt {
        if self.is_stdout_plain() {
            self.stdout.clone()
        } else {
            remove_ansi_csi(&self.stdout)
        }
    }

    /// Get `stderr` without any ANSI escape sequences (CSI).
    pub fn stderr_plain(&self) -> Txt {
        if self.is_stderr_plain() {
            self.stderr.clone()
        } else {
            remove_ansi_csi(&self.stderr)
        }
    }

    /// Gets if `stderr` contains a crash panic.
    pub fn has_panic(&self) -> bool {
        if self.code == Some(101) {
            CrashPanic::contains(&self.stderr)
        } else {
            false
        }
    }

    /// Gets if `stderr` contains a crash panic that traced widget/window path.
    pub fn has_panic_widget(&self) -> bool {
        if self.code == Some(101) {
            CrashPanic::contains_widget(&self.stderr)
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
            CrashPanic::find(&self.stderr)
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

        let exception = match dump.get_stream::<MinidumpException>() {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("error reading minidump exception, {e}");
                return None;
            }
        };

        #[cfg(debug_assertions)]
        {
            // nice error messages, but adds >1MB of binary code
            let system_info = match dump.get_stream::<MinidumpSystemInfo>() {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("error reading minidump system info, {e}");
                    return None;
                }
            };
            let crash_reason = exception.get_crash_reason(system_info.os, system_info.cpu);
            Some(zng_txt::formatx!("{crash_reason}"))
        }

        #[cfg(not(debug_assertions))]
        {
            // raw error code, only common names
            let raw = exception.raw;

            let code = raw.exception_record.exception_code;
            let addr = raw.exception_record.exception_address;

            cfg_select! {
                windows => {
                    let name = match code {
                        0xC0000005 => "ACCESS_VIOLATION",
                        0xC0000409 => "STACK_BUFFER_OVERRUN",
                        0x80000003 => "BREAKPOINT",
                        0xC000001D => "ILLEGAL_INSTRUCTION",
                        0xC0000094 => "INTEGER_DIVIDE_BY_ZERO",
                        0xC00000FD => "STACK_OVERFLOW",
                        0xC0000096 => "PRIVILEGED_INSTRUCTION",
                        0xC0000008 => "INVALID_HANDLE",
                        0xC0000135 => "DLL_NOT_FOUND",
                        _ => "",
                    };
                }
                any(target_os = "linux", target_os = "android") => {
                    let name = match code as i32 {
                        4 => "SIGILL",
                        5 => "SIGTRAP",
                        6 => "SIGABRT",
                        7 => "SIGBUS",
                        8 => "SIGFPE",
                        9 => "SIGKILL",
                        11 => "SIGSEGV",
                        13 => "SIGPIPE",
                        _ => "",
                    };
                }
                any(target_os = "macos", target_os = "ios") => {
                    let name = match code as i32 {
                        4 => "SIGILL",
                        5 => "SIGTRAP",
                        6 => "SIGABRT",
                        8 => "SIGFPE",
                        10 => "SIGBUS",
                        11 => "SIGSEGV",
                        _ => "",
                    };
                }
                _ => {
                    let name = "";
                }
            }
            if name.is_empty() {
                Some(zng_txt::formatx!("exception 0x{code:08X} at 0x{addr:X}"))
            } else {
                Some(zng_txt::formatx!("exception 0x{code:08X} ({name}) at 0x{addr:X}"))
            }
        }
    }
}

fn crash_handler_monitor_process(
    dump_dir: Option<PathBuf>,
    mut cfg_app: ConfigProcess,
    mut cfg_dialog: ConfigProcess,
    has_dialog_handler: bool,
) -> ! {
    zng_env::set_process_name("crash-handler-process");

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
            Ok((status, stdout, stderr, dump_file)) => {
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

                        if let Some(sig) = signal
                            && [2, 9, 17, 19, 23].contains(&sig)
                        {
                            tracing::warn!(
                                "app-process exited by signal ({sig}), \
                                                will exit monitor-process with code 1"
                            );
                            zng_env::exit(1);
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
                        stdout.into_txt_blocking(false),
                        stderr.into_txt_blocking(false),
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
                            Ok((
                                std::process::ExitStatus::default(),
                                tap::StdoutTap::dummy(),
                                tap::StderrTap::dummy(),
                                None,
                            ))
                        };

                        for _ in 0..5 {
                            if !crash_file.exists() || std::fs::remove_file(&crash_file).is_ok() {
                                break;
                            }
                            std::thread::sleep(100.ms());
                        }

                        let response = match dialog_result {
                            Ok((dlg_status, dlg_stdout, dlg_stderr, dlg_dump_file)) => {
                                if dlg_status.success() {
                                    let dlg_stdout = dlg_stdout.into_string_blocking(false);
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

                                        if let Some(sig) = signal
                                            && [2, 9, 17, 19, 23].contains(&sig)
                                        {
                                            tracing::warn!(
                                                "dialog-process exited by signal ({sig}), \
                                                                will exit monitor-process with code 1"
                                            );
                                            zng_env::exit(1);
                                        }
                                    }

                                    let dialog_crash = CrashError::new(
                                        SystemTime::now(),
                                        code,
                                        signal,
                                        dlg_stdout.into_txt_blocking(false),
                                        dlg_stderr.into_txt_blocking(false),
                                        dlg_dump_file,
                                        Box::new([]),
                                    );
                                    tracing::error!("crash dialog-process crashed, {dialog_crash:#}");

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
) -> std::io::Result<(std::process::ExitStatus, tap::StdoutTap, tap::StderrTap, Option<PathBuf>)> {
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
                match minidumper::Server::with_name(minidumper::SocketName::Path(&dump_channel)) {
                    Ok(mut s) => {
                        command.env(DUMP_CHANNEL, &dump_channel);
                        let shutdown = Arc::new(AtomicBool::new(false));
                        let runner = std::thread::Builder::new()
                            .name("minidumper-server".into())
                            .stack_size(512 * 1024)
                            .spawn(clmv!(shutdown, || {
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
                            }))
                            .expect("failed to spawn thread");
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

    let stdout = tap::StdoutTap::new_blocking(app_process.stdout.take().unwrap());
    let stderr = tap::StderrTap::new_blocking(app_process.stderr.take().unwrap());

    let status = app_process.wait()?;

    let mut dump_file = None;
    if let Some(s) = dump_server {
        s.shutdown.store(true, atomic::Ordering::Relaxed);
        match s.runner.join() {
            Ok(r) => dump_file = r,
            Err(p) => std::panic::resume_unwind(p),
        };
    }

    Ok((status, stdout, stderr, dump_file))
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

fn crash_handler_app_process(dump_enabled: bool) {
    CrashPanic::set_hook(|| crate::widget::WIDGET.trace_path());
    if dump_enabled {
        minidump_attach();
    }

    // app-process execution happens after this.
}

fn crash_handler_dialog_process(dump_enabled: bool, dialog: CrashDialogHandler, args_file: String) -> ! {
    zng_env::set_process_name("crash-dialog-process");

    CrashPanic::set_hook(|| crate::widget::WIDGET.trace_path());
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

fn minidump_attach() {
    let channel_name = match std::env::var(DUMP_CHANNEL) {
        Ok(n) if !n.is_empty() => PathBuf::from(n),
        _ => {
            eprintln!("expected minidump channel name, this instance will not handle crashes");
            return;
        }
    };
    let client = match minidumper::Client::with_name(minidumper::SocketName::Path(&channel_name)) {
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
