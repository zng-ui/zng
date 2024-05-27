use std::{
    io,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::AtomicBool,
};

/// Print warning message.
macro_rules! warn {
    ($($format_args:tt)*) => {
        {
            $crate::util::set_failed_run(true);
            eprintln!("{} {}", $crate::util::WARN_PREFIX, format_args!($($format_args)*));
        }
    };
}

/// Print error message and flags the current process as failed.
///
/// Note that this does not exit the process, use `fatal!` to exit.
macro_rules! error {
    ($($format_args:tt)*) => {
        {
            $crate::util::set_failed_run(true);
            eprintln!("{} {}", $crate::util::ERROR_PREFIX, format_args!($($format_args)*));
        }
    };
}

pub static WARN_PREFIX: &str = color_print::cstr!("<bold><yellow>warning</yellow>:</bold>");
pub static ERROR_PREFIX: &str = color_print::cstr!("<bold><red>error</red>:</bold>");

/// Print error message and exit the current process with error code.
macro_rules! fatal {
    ($($format_args:tt)*) => {
        {
            error!($($format_args)*);
            $crate::util::exit();
        }
    };
}

static RUN_FAILED: AtomicBool = AtomicBool::new(false);

/// Gets if the current process will exit with error code.
pub fn is_failed_run() -> bool {
    RUN_FAILED.load(std::sync::atomic::Ordering::SeqCst)
}

/// Sets if the current process will exit with error code.
pub fn set_failed_run(failed: bool) {
    RUN_FAILED.store(failed, std::sync::atomic::Ordering::SeqCst);
}

/// Exit the current process, with error code `102` if [`is_failed_run`].
pub fn exit() -> ! {
    if is_failed_run() {
        std::process::exit(102)
    } else {
        std::process::exit(0)
    }
}

/// Run the command with args.
pub fn cmd(line: &str, args: &[&str], env: &[(&str, &str)]) -> io::Result<()> {
    let mut line_parts = line.split(' ');
    let program = line_parts.next().expect("expected program to run");
    let mut cmd = Command::new(program);
    cmd.args(
        line_parts
            .map(|a| {
                let a = a.trim();
                if a.starts_with('"') {
                    a.trim_matches('"')
                } else {
                    a
                }
            })
            .filter(|a| !a.is_empty()),
    );
    cmd.args(args);
    for (key, val) in env.iter() {
        cmd.env(key, val);
    }

    let status = cmd.status()?;
    if status.success() {
        Ok(())
    } else {
        let mut cmd = format!("cmd failed: {line}");
        for arg in args {
            cmd.push(' ');
            cmd.push_str(arg);
        }
        Err(io::Error::new(io::ErrorKind::Other, cmd))
    }
}

pub fn workspace_dir() -> Option<PathBuf> {
    let output = std::process::Command::new("cargo")
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .ok()?;

    if output.status.success() {
        let cargo_path = Path::new(std::str::from_utf8(&output.stdout).unwrap().trim());
        Some(cargo_path.parent().unwrap().to_owned())
    } else {
        None
    }
}
