use std::{fmt, io, path::PathBuf, sync::Arc};

use zng_txt::{ToTxt, Txt};
use zng_var::ResponseVar;

pub fn build(manifest_dir: &str) -> ResponseVar<Result<(), BuildError>> {
    let manifest_path = format!("{manifest_dir}/Cargo.toml");

    zng_task::wait_respond(move || -> Result<(), BuildError> {
        let output = std::process::Command::new("cargo")
            .arg("build")
            .arg("--manifest-path")
            .arg(&manifest_path)
            .output()?;

        if output.status.success() {
            Ok(())
        } else {
            Err(BuildError::Cargo {
                status: output.status,
                stdout: String::from_utf8_lossy(&output.stdout).to_txt(),
                stderr: String::from_utf8_lossy(&output.stderr).to_txt(),
            })
        }
    })
}

/// Get compiled dyn lib name from manifest dir.
pub fn lib_name(manifest_dir: &str) -> Option<PathBuf> {
    let manifest_path = format!("{manifest_dir}/Cargo.toml");

    let output = std::process::Command::new("cargo")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--no-deps")
        .arg("--manifest-path")
        .arg(&manifest_path)
        .output();

    todo!()
}

#[derive(Debug, Clone)]
pub enum BuildError {
    Io(Arc<io::Error>),
    Cargo {
        status: std::process::ExitStatus,
        stdout: Txt,
        stderr: Txt,
    },
}
impl PartialEq for BuildError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Io(l0), Self::Io(r0)) => Arc::ptr_eq(l0, r0),
            (
                Self::Cargo {
                    status: l_exit_status,
                    stdout: l_stdout,
                    stderr: l_stderr,
                },
                Self::Cargo {
                    status: r_exit_status,
                    stdout: r_stdout,
                    stderr: r_stderr,
                },
            ) => l_exit_status == r_exit_status && l_stdout == r_stdout && l_stderr == r_stderr,
            _ => false,
        }
    }
}
impl From<io::Error> for BuildError {
    fn from(err: io::Error) -> Self {
        Self::Io(Arc::new(err))
    }
}
impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildError::Io(e) => fmt::Display::fmt(e, f),
            BuildError::Cargo { .. } => {
                write!(f, "cargo build failed")
            }
        }
    }
}
impl std::error::Error for BuildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BuildError::Io(e) => Some(&**e),
            BuildError::Cargo { .. } => None,
        }
    }
}
