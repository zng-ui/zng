use std::{
    fmt,
    io::{self, BufRead as _, Read},
    path::PathBuf,
    process::{ChildStdout, Command, Stdio},
    sync::Arc,
};

use zng_app::handler::clmv;
use zng_task::SignalOnce;
use zng_txt::{ToTxt, Txt};
use zng_var::ResponseVar;

/// Build and return the dylib path.
pub fn build(
    manifest_dir: &str,
    package_option: &str,
    package: &str,
    bin_option: &str,
    bin: &str,
    cancel: SignalOnce,
) -> ResponseVar<Result<PathBuf, BuildError>> {
    let mut build = Command::new("cargo");
    build.arg("build").arg("--message-format").arg("json");
    if !package.is_empty() {
        build.arg(package_option).arg(package);
    }
    if !bin.is_empty() {
        build.arg(bin_option).arg(bin);
    }

    build_custom(manifest_dir, build, cancel)
}

/// Build and return the dylib path.
pub fn build_custom(manifest_dir: &str, mut build: Command, cancel: SignalOnce) -> ResponseVar<Result<PathBuf, BuildError>> {
    let manifest_path = PathBuf::from(manifest_dir).join("Cargo.toml");

    zng_task::respond(async move {
        let mut child = zng_task::wait(move || build.stdin(Stdio::null()).stderr(Stdio::piped()).stdout(Stdio::piped()).spawn()).await?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let run = zng_task::wait(clmv!(manifest_path, || run_build(manifest_path, stdout)));

        let cancel = async move {
            cancel.await;
            Err(BuildError::Cancelled)
        };

        match zng_task::any!(run, cancel).await {
            Ok(p) => {
                zng_task::spawn_wait(move || {
                    if let Err(e) = child.kill() {
                        tracing::error!("failed to kill build after hot dylib successfully built, {e}");
                    } else {
                        let _ = child.wait();
                    }
                });
                Ok(p)
            }
            Err(e) => {
                if matches!(e, BuildError::Cancelled) {
                    zng_task::spawn_wait(move || {
                        if let Err(e) = child.kill() {
                            tracing::error!("failed to kill build after cancel, {e}");
                        } else {
                            let _ = child.wait();
                        }
                    });

                    Err(e)
                } else if matches!(&e, BuildError::Io(e) if e.kind() == io::ErrorKind::UnexpectedEof) {
                    // run_build read to EOF without finding manifest_path
                    let status = zng_task::wait(move || {
                        child.kill()?;
                        child.wait()
                    });
                    match status.await {
                        Ok(status) => {
                            if status.success() {
                                Err(BuildError::ManifestPathDidNotBuild { path: manifest_path })
                            } else {
                                let mut err = String::new();
                                let mut stderr = stderr;
                                stderr.read_to_string(&mut err)?;
                                Err(BuildError::Command {
                                    status,
                                    err: err.lines().next_back().unwrap_or("").to_txt(),
                                })
                            }
                        }
                        Err(wait_e) => Err(wait_e.into()),
                    }
                } else {
                    Err(e)
                }
            }
        }
    })
}

fn run_build(manifest_path: PathBuf, stdout: ChildStdout) -> Result<PathBuf, BuildError> {
    for line in io::BufReader::new(stdout).lines() {
        let line = line?;

        const COMP_ARTIFACT: &str = r#"{"reason":"compiler-artifact","#;
        const MANIFEST_FIELD: &str = r#""manifest_path":""#;
        const FILENAMES_FIELD: &str = r#""filenames":["#;

        if line.starts_with(COMP_ARTIFACT) {
            let i = match line.find(MANIFEST_FIELD) {
                Some(i) => i,
                None => {
                    return Err(BuildError::UnknownMessageFormat {
                        pat: MANIFEST_FIELD.into(),
                    });
                }
            };
            let line = &line[i + MANIFEST_FIELD.len()..];
            let i = match line.find('"') {
                Some(i) => i,
                None => {
                    return Err(BuildError::UnknownMessageFormat {
                        pat: MANIFEST_FIELD.into(),
                    });
                }
            };
            let line_manifest = PathBuf::from(&line[..i]);

            if line_manifest != manifest_path {
                continue;
            }

            let line = &line[i..];
            let i = match line.find(FILENAMES_FIELD) {
                Some(i) => i,
                None => {
                    return Err(BuildError::UnknownMessageFormat {
                        pat: FILENAMES_FIELD.into(),
                    });
                }
            };
            let line = &line[i + FILENAMES_FIELD.len()..];
            let i = match line.find(']') {
                Some(i) => i,
                None => {
                    return Err(BuildError::UnknownMessageFormat {
                        pat: FILENAMES_FIELD.into(),
                    });
                }
            };

            for file in line[..i].split(',') {
                let file = PathBuf::from(file.trim().trim_matches('"'));
                if file.extension().map(|e| e != "rlib").unwrap_or(true) {
                    return Ok(file);
                }
            }
            return Err(BuildError::UnknownMessageFormat {
                pat: FILENAMES_FIELD.into(),
            });
        }
    }

    Err(BuildError::Io(Arc::new(io::Error::new(io::ErrorKind::UnexpectedEof, ""))))
}

/// Rebuild error.
#[derive(Debug, Clone)]
pub enum BuildError {
    /// Error starting, ending the build command.
    Io(Arc<io::Error>),
    /// Build command error.
    Command {
        /// Command exit status.
        status: std::process::ExitStatus,
        /// Display error.
        err: Txt,
    },
    /// Build command did not rebuild the dylib.
    ManifestPathDidNotBuild {
        /// Cargo.toml file that was expected to rebuild.
        path: PathBuf,
    },
    /// Cargo `--message-format json` did not output in an expected format.
    UnknownMessageFormat {
        /// Pattern that was not found in the message line.
        pat: Txt,
    },
    /// Error loading built library.
    Load(Arc<libloading::Error>),
    /// Build cancelled.
    Cancelled,
}
impl PartialEq for BuildError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Io(l0), Self::Io(r0)) => Arc::ptr_eq(l0, r0),
            (
                Self::Command {
                    status: l_exit_status,
                    err: l_stderr,
                },
                Self::Command {
                    status: r_exit_status,
                    err: r_stderr,
                },
            ) => l_exit_status == r_exit_status && l_stderr == r_stderr,
            _ => false,
        }
    }
}
impl From<io::Error> for BuildError {
    fn from(err: io::Error) -> Self {
        Self::Io(Arc::new(err))
    }
}
impl From<libloading::Error> for BuildError {
    fn from(err: libloading::Error) -> Self {
        Self::Load(Arc::new(err))
    }
}
impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildError::Io(e) => fmt::Display::fmt(e, f),
            BuildError::Command { status, err } => {
                write!(f, "build command failed")?;
                let mut sep = "\n";
                #[allow(unused_assignments)] // depends on cfg
                if let Some(c) = status.code() {
                    write!(f, "{sep}exit code: {c:#x}")?;
                    sep = ", ";
                }
                #[cfg(unix)]
                {
                    use std::os::unix::process::ExitStatusExt;
                    if let Some(s) = status.signal() {
                        write!(f, "{sep}signal: {s}")?;
                    }
                }
                write!(f, "\n{err}")?;

                Ok(())
            }
            BuildError::ManifestPathDidNotBuild { path } => write!(f, "build command did not build `{}`", path.display()),
            BuildError::UnknownMessageFormat { pat: field } => write!(f, "could not find expected `{field}` in cargo JSON message"),
            BuildError::Load(e) => fmt::Display::fmt(e, f),
            BuildError::Cancelled => write!(f, "build cancelled"),
        }
    }
}
impl std::error::Error for BuildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BuildError::Io(e) => Some(&**e),
            BuildError::Load(e) => Some(&**e),
            _ => None,
        }
    }
}
