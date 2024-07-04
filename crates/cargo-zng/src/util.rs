use std::{
    io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::AtomicBool,
};

use serde::Deserialize;

/// Print warning message.
macro_rules! warn {
    ($($format_args:tt)*) => {
        {
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

/// Run the command with args, inherits stdout and stderr.
pub fn cmd(line: &str, args: &[&str], env: &[(&str, &str)]) -> io::Result<()> {
    cmd_impl(line, args, env, false)
}
/// Run the command with args.
pub fn cmd_silent(line: &str, args: &[&str], env: &[(&str, &str)]) -> io::Result<()> {
    cmd_impl(line, args, env, true)
}
fn cmd_impl(line: &str, args: &[&str], env: &[(&str, &str)], silent: bool) -> io::Result<()> {
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

    if silent {
        let output = cmd.output()?;
        if output.status.success() {
            Ok(())
        } else {
            let mut cmd = format!("cmd failed: {line}");
            for arg in args {
                cmd.push(' ');
                cmd.push_str(arg);
            }
            cmd.push('\n');
            cmd.push_str(&String::from_utf8_lossy(&output.stderr));
            Err(io::Error::new(io::ErrorKind::Other, cmd))
        }
    } else {
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

pub fn ansi_enabled() -> bool {
    std::env::var("NO_COLOR").is_err()
}

pub fn clean_value(value: &str, required: bool) -> io::Result<String> {
    let mut first_char = false;
    let clean_value: String = value
        .chars()
        .filter(|c| {
            if first_char {
                first_char = c.is_ascii_alphabetic();
                first_char
            } else {
                *c == ' ' || *c == '-' || *c == '_' || c.is_ascii_alphanumeric()
            }
        })
        .collect();
    let clean_value = clean_value.trim().to_owned();

    if required && clean_value.is_empty() {
        if clean_value.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("cannot derive clean value from `{value}`, must contain at least one ascii alphabetic char"),
            ));
        }
        if clean_value.len() > 62 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("cannot derive clean value from `{value}`, must contain <= 62 ascii alphanumeric chars"),
            ));
        }
    }
    Ok(clean_value)
}

pub fn manifest_path_from_package(package: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct Metadata {
        packages: Vec<Package>,
    } 
    #[derive(Deserialize)]
    struct Package {
        name: String,
        manifest_path: String,
    }

    let metadata = match Command::new("cargo").args(&["metadata", "--format-version", "1",  "--no-deps"]).stderr(Stdio::inherit()).output() {
        Ok(m) => {
            if !m.status.success() {
                fatal!("cargo metadata error")
            }
            String::from_utf8_lossy(&m.stdout).into_owned()
        },
        Err(e) => fatal!("cargo metadata error, {e}"),
    };

    let metadata: Metadata = serde_json::from_str(&metadata).unwrap_or_else(|e| fatal!("unexpected cargo metadata format, {e}"));

    for p in metadata.packages {
        if p.name == package {
            return Some(p.manifest_path);
        }
    }
    None
}