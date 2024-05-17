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
pub fn build(manifest_dir: &str) -> ResponseVar<Result<PathBuf, BuildError>> {
    let manifest_path = PathBuf::from(manifest_dir).join("Cargo.toml");

    zng_task::wait_respond(move || -> Result<PathBuf, BuildError> {
        let mut build = Command::new("cargo")
            .arg("build")
            .arg("--message-format")
            .arg("json")
            .arg("-p") // !!: TODO, get this from the service.
            .arg("examples")
            .arg("--example")
            .arg("hot_reload")
            .stdin(Stdio::null())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        for line in io::BufReader::new(build.stdout.take().unwrap()).lines() {
            let line = line?;

            const COMP_ARTIFACT: &str = r#"{"reason":"compiler-artifact","#;
            const MANIFEST_FIELD: &str = r#""manifest_path":""#;
            const FILENAMES_FIELD: &str = r#""filenames":["#;

            if line.starts_with(COMP_ARTIFACT) {
                let i = match line.find(MANIFEST_FIELD) {
                    Some(i) => i,
                    None => return Err(BuildError::UnknownMessageFormat { field: MANIFEST_FIELD }),
                };
                let line = &line[i + MANIFEST_FIELD.len()..];
                let i = match line.find('"') {
                    Some(i) => i,
                    None => return Err(BuildError::UnknownMessageFormat { field: MANIFEST_FIELD }),
                };
                let line_manifest = PathBuf::from(&line[..i]);

                if line_manifest != manifest_path {
                    continue;
                }

                let line = &line[i..];
                let i = match line.find(FILENAMES_FIELD) {
                    Some(i) => i,
                    None => return Err(BuildError::UnknownMessageFormat { field: FILENAMES_FIELD }),
                };
                let line = &line[i + FILENAMES_FIELD.len()..];
                let i = match line.find(']') {
                    Some(i) => i,
                    None => return Err(BuildError::UnknownMessageFormat { field: FILENAMES_FIELD }),
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
            Err(BuildError::Cargo {
                status,
                stderr: err.to_txt(),
            })
        }
    })
}

#[derive(Debug, Clone)]
pub enum BuildError {
    Io(Arc<io::Error>),
    Cargo { status: std::process::ExitStatus, stderr: Txt },
    ManifestPathDidNotBuild { path: PathBuf },
    UnknownMessageFormat { field: &'static str },
}
impl PartialEq for BuildError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Io(l0), Self::Io(r0)) => Arc::ptr_eq(l0, r0),
            (
                Self::Cargo {
                    status: l_exit_status,
                    stderr: l_stderr,
                },
                Self::Cargo {
                    status: r_exit_status,
                    stderr: r_stderr,
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
impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildError::Io(e) => fmt::Display::fmt(e, f),
            BuildError::Cargo { .. } => {
                write!(f, "cargo build failed")
            }
            BuildError::ManifestPathDidNotBuild { path } => write!(f, "build command did not build `{}`", path.display()),
            BuildError::UnknownMessageFormat { field } => write!(f, "could not find expected `{field}` in cargo JSON message"),
        }
    }
}
impl std::error::Error for BuildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BuildError::Io(e) => Some(&**e),
            _ => None,
        }
    }
}
