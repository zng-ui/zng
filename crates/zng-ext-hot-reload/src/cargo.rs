use std::{
    fmt,
    io::{self, BufRead as _, Read},
    path::PathBuf,
    process::{Command, Stdio},
    sync::Arc,
};

use zng_txt::{ToTxt, Txt};
use zng_var::ResponseVar;

/// Build and return the dylib path.
pub fn build(manifest_dir: &str, package: &str, bin_option: &str, bin: &str) -> ResponseVar<Result<PathBuf, BuildError>> {
    let manifest_path = PathBuf::from(manifest_dir).join("Cargo.toml");

    let mut build = Command::new("cargo");
    build.arg("build").arg("--message-format").arg("json");
    if !package.is_empty() {
        build.arg("--package").arg(package);
    }
    if !bin.is_empty() {
        build.arg(bin_option).arg(bin);
    }

    zng_task::wait_respond(move || -> Result<PathBuf, BuildError> {
        let mut build = build.stdin(Stdio::null()).stderr(Stdio::piped()).stdout(Stdio::piped()).spawn()?;

        for line in io::BufReader::new(build.stdout.take().unwrap()).lines() {
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
                        })
                    }
                };
                let line = &line[i + MANIFEST_FIELD.len()..];
                let i = match line.find('"') {
                    Some(i) => i,
                    None => {
                        return Err(BuildError::UnknownMessageFormat {
                            pat: MANIFEST_FIELD.into(),
                        })
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
                        })
                    }
                };
                let line = &line[i + FILENAMES_FIELD.len()..];
                let i = match line.find(']') {
                    Some(i) => i,
                    None => {
                        return Err(BuildError::UnknownMessageFormat {
                            pat: FILENAMES_FIELD.into(),
                        })
                    }
                };

                for file in line[..i].split(',') {
                    let file = PathBuf::from(file.trim().trim_matches('"'));
                    if file.extension().map(|e| e != "rlib").unwrap_or(true) {
                        build.kill()?;
                        return Ok(file);
                    }
                }
            }
        }

        let status = build.wait()?;
        if status.success() {
            Err(BuildError::ManifestPathDidNotBuild { path: manifest_path })
        } else {
            let mut err = String::new();
            build.stderr.take().unwrap().read_to_string(&mut err)?;
            Err(BuildError::Command {
                status,
                err: err.lines().next_back().unwrap_or("").to_txt(),
            })
        }
    })
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
                #[allow(unused_assignments)]
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
