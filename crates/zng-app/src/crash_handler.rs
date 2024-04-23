//! App-process crash handler.

use std::{
    fmt,
    io::{BufRead, BufReader},
    path::Path,
    time::SystemTime,
};
use zng_layout::unit::TimeUnits as _;

use zng_txt::Txt;

/// Starts the current app-process in a monitored instance.
///
/// This function does nothing if the current app-process is monitored, otherwise it takes over execution
/// an becomes the monitor-process, never returning.
///
/// # Examples
///
/// !!: TODO
pub fn crash_handler(config: CrashConfig) {
    match std::env::var(DIALOG_PROCESS) {
        Ok(args_file) => crash_handler_dialog_process(config.dialog, args_file),
        Err(e) => match e {
            std::env::VarError::NotPresent => {}
            e => panic!("invalid dialog args, {e:?}"),
        },
    }

    if std::env::var(APP_PROCESS) != Err(std::env::VarError::NotPresent) {
        return crash_handler_app_process();
    }

    crash_handler_monitor_process();
}

const APP_PROCESS: &str = "ZNG_CRASH_HANDLER_APP";
const DIALOG_PROCESS: &str = "ZNG_CRASH_HANDLER_DIALOG";
const RESPONSE_PREFIX: &str = "zng_crash_dialog_response: ";

/// Crash handler config.
pub struct CrashConfig {
    dialog: fn(CrashArgs) -> !,
}
impl CrashConfig {
    /// New with dialog function.
    pub fn new(dialog: fn(CrashArgs) -> !) -> Self {
        Self { dialog }
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
    /// App process exit code.
    pub code: Option<i32>,
    /// Full capture of the app stdout.
    pub stdout: Txt,
    /// Full capture of the app stderr.
    pub stderr: Txt,
    /// Arguments used.
    pub args: Box<[Txt]>,
}
impl fmt::Display for CrashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "timestamp: {:?}\ncode: {:?}\n\nSTDOUT:\n{}\n\nSTDERR:\n{}\n",
            self.timestamp, self.code, self.stdout, self.stderr
        )
    }
}

fn crash_handler_monitor_process() -> ! {
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
        match run_app_process(&exe, &args) {
            Ok((status, [stdout, stderr])) => {
                if status.success() {
                    let code = status.code().unwrap_or(0);
                    tracing::info!(
                        "crash monitor-process exiting with success code ({code}), {} crashes",
                        dialog_args.app_crashes.len()
                    );
                    std::process::exit(code);
                } else {
                    #[cfg(windows)]
                    if status.code() == Some(1) {
                        tracing::warn!(
                            "app-process exit code (1), probably killed by the system, \
                                        will exit monitor-process with the same code"
                        );
                        std::process::exit(1);
                    }
                    #[cfg(unix)]
                    if status.code().is_none() {
                        tracing::warn!(
                            "app-process exited by signal, probably killed by the user, \
                                        will exit app-process with code 1"
                        );
                        std::process::exit(1);
                    }

                    tracing::error!(
                        "app-process crashed with error code ({:?}), {} crashes previously",
                        status,
                        dialog_args.app_crashes.len()
                    );

                    let timestamp = SystemTime::now();
                    dialog_args.app_crashes.push(CrashError {
                        timestamp,
                        code: status.code(),
                        stdout: stdout.into(),
                        stderr: stderr.into(),
                        args: args.clone(),
                    });

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

                        let dialog_result = std::process::Command::new(&exe).env(DIALOG_PROCESS, &crash_file).output();

                        for _ in 0..5 {
                            if !crash_file.exists() || std::fs::remove_file(&crash_file).is_ok() {
                                break;
                            }
                            std::thread::sleep(100.ms());
                        }

                        let response = match dialog_result {
                            Ok(s) => {
                                if s.status.success() {
                                    let stdout = String::from_utf8_lossy(&s.stdout);
                                    stdout
                                        .lines()
                                        .filter_map(|l| l.trim().strip_prefix(RESPONSE_PREFIX))
                                        .last()
                                        .expect("crash dialog-process did not respond correctly")
                                        .to_owned()
                                } else {
                                    let dialog_crash = CrashError {
                                        timestamp: SystemTime::now(),
                                        code: s.status.code(),
                                        stdout: String::from_utf8_lossy(&s.stdout).into_owned().into(),
                                        stderr: String::from_utf8_lossy(&s.stderr).into_owned().into(),
                                        args: Box::new([]),
                                    };
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
fn run_app_process(exe: &Path, args: &[Txt]) -> std::io::Result<(std::process::ExitStatus, [String; 2])> {
    let mut app_process = std::process::Command::new(exe)
        .env(APP_PROCESS, "true")
        .args(args.iter())
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

fn crash_handler_app_process() {
    // !!: TODO, setup in process handlers, better panic trace

    // app-process execution happens after the `crash_handler` function returns.
}

fn crash_handler_dialog_process(dialog: fn(CrashArgs) -> !, args_file: String) -> ! {
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
