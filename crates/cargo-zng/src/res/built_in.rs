//! Built-in tools

use std::{
    env, fs,
    io::{self, BufRead as _},
    path::{Path, PathBuf},
};

use crate::util;

/// Env var set to the Cargo workspace directory that is parent to the res source.
///
/// Note that the tool also runs with this dir as working directory (`current_dir`).
pub const ZR_WORKSPACE_DIR: &str = "ZR_WORKSPACE_DIR";
/// Env var set to the resources source directory.
pub const ZR_SOURCE_DIR: &str = "ZR_SOURCE_DIR";
/// Env var set to the resources build target directory.
///
/// Note that this is the 'root' of the built resources, use [`ZR_TARGET_DD`] to get the
/// parent dir of the target file inside the target directory.
pub const ZR_TARGET_DIR: &str = "ZR_TARGET_DIR";
/// Env var set to dir that the tool can use to store intermediary data for the specific request.
///
/// The cache key (dir name) is a hash of source, target, request and request content only.
pub const ZR_CACHE_DIR: &str = "ZR_CACHE_DIR";

/// Env var set to the request file that called the tool.
pub const ZR_REQUEST: &str = "ZR_REQUEST";
/// Env var set to the request file parent dir.
pub const ZR_REQUEST_DD: &str = "ZR_REQUEST_DD";
/// Env var set to the target file implied by the request file name.
///
/// That is, the request filename without `.zr-{tool}` and in the equivalent target subdirectory.
pub const ZR_TARGET: &str = "ZR_TARGET";
/// Env var set to the target file parent dir.
pub const ZR_TARGET_DD: &str = "ZR_TARGET_DD";

/// Env var set when it is running a tool that requested `zng-res::on-final=` again.
pub const ZR_FINAL: &str = "ZR_FINAL";

/// Env var set when it needs the tool print the help text shown in `cargo zng res --tools`.
pub const ZR_HELP: &str = "ZR_HELP";

/// Env var set to package.metadata.zng.about.app_id or "qualifier.org.app" in snake_case
pub const ZR_APP_ID: &str = "ZR_APP_ID";
/// Env var set to package.metadata.zng.about.app or package.name
pub const ZR_APP: &str = "ZR_APP";
/// Env var set to package.metadata.zng.about.org or the first package.authors
pub const ZR_ORG: &str = "ZR_ORG";
/// Env var set to package.version
pub const ZR_VERSION: &str = "ZR_VERSION";
/// Env var set to package.description
pub const ZR_DESCRIPTION: &str = "ZR_DESCRIPTION";
/// Env var set to package.homepage
pub const ZR_HOMEPAGE: &str = "ZR_HOMEPAGE";
/// Env var set to package.license
pub const ZR_LICENSE: &str = "ZR_LICENSE";
/// Env var set to package.name
pub const ZR_PKG_NAME: &str = "ZR_PKG_NAME";
/// Env var set to package.authors
pub const ZR_PKG_AUTHORS: &str = "ZR_PKG_AUTHORS";
/// Env var set to package.name in snake_case
pub const ZR_CRATE_NAME: &str = "ZR_CRATE_NAME";
/// Env var set to package.metadata.zng.about.qualifier or the first components `ZR_APP_ID` except the last two
pub const ZR_QUALIFIER: &str = "ZR_QUALIFIER";

/// Print the help and exit if is help request.
pub fn help(help: &str) {
    if env::var(ZR_HELP).is_ok() {
        println!("{help}");
        std::process::exit(0);
    };
}

/// Get a `ZR_` path var.
pub fn path(var: &str) -> PathBuf {
    env::var(var).unwrap_or_else(|_| panic!("missing {var}")).into()
}

/// Format the path in the standard way used by cargo-zng.
pub fn display_path(p: &Path) -> String {
    let base = path(ZR_WORKSPACE_DIR);
    let r = if let Ok(local) = p.strip_prefix(base) {
        local.display().to_string()
    } else {
        p.display().to_string()
    };

    #[cfg(windows)]
    return r.replace('\\', "/");

    #[cfg(not(windows))]
    r
}

fn read_line(path: &Path, expected: &str) -> io::Result<String> {
    match read_lines(path).next() {
        Some(r) => r.map(|(_, l)| l),
        None => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("expected {expected} in tool file content"),
        )),
    }
}

fn read_lines(path: &Path) -> impl Iterator<Item = io::Result<(usize, String)>> {
    enum State {
        Open(io::Result<fs::File>),
        Lines(usize, io::Lines<io::BufReader<fs::File>>),
        End,
    }
    // start -> open
    let mut state = State::Open(fs::File::open(path));
    std::iter::from_fn(move || {
        loop {
            match std::mem::replace(&mut state, State::End) {
                State::Lines(count, mut lines) => {
                    if let Some(l) = lines.next() {
                        match l {
                            // lines -> lines
                            Ok(l) => {
                                state = State::Lines(count + 1, lines);
                                let test = l.trim();
                                if !test.is_empty() && !test.starts_with('#') {
                                    return Some(Ok((count, l)));
                                }
                            }
                            // lines -> end
                            Err(e) => {
                                return Some(Err(e));
                            }
                        }
                    }
                }
                State::Open(r) => match r {
                    // open -> lines
                    Ok(f) => state = State::Lines(1, io::BufReader::new(f).lines()),
                    // open -> end
                    Err(e) => return Some(Err(e)),
                },
                // end -> end
                State::End => return None,
            }
        }
    })
}

fn read_path(request_file: &Path) -> io::Result<PathBuf> {
    read_line(request_file, "path").map(PathBuf::from)
}

pub(crate) fn symlink_warn(path: &Path) {
    warn!("symlink ignored in `{}`, use zr-tools to 'link'", path.display());
}

pub const ENV_TOOL: &str = "ZNG_RES_TOOL";

macro_rules! built_in {
    ($($tool:tt),+ $(,)?) => {
        $(
            mod $tool;
            use $tool::$tool;
        )+

        pub static BUILT_INS: &[&str] = &[
            $(stringify!($tool),)+
        ];
        static BUILT_IN_FNS: &[fn()] = &[
            $($tool,)+
        ];
    };
}
built_in! { copy, glob, rp, sh, shf, warn, fail, apk, l10n }

pub(crate) use sh::sh_run;

pub fn run() {
    if let Ok(tool) = env::var(ENV_TOOL) {
        if let Some(i) = BUILT_INS.iter().position(|n| *n == tool.as_str()) {
            (BUILT_IN_FNS[i])();
            std::process::exit(0);
        } else {
            fatal!("`tool` is not a built-in tool");
        }
    }
}
