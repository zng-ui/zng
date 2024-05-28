//! Built-in tools

use std::{
    env, fs,
    io::{self, BufRead},
    path::{Path, PathBuf},
};

/// Env var set by cargo-zng to the Cargo workspace directory that is parent to the res source.
///
/// Note that the tool also runs with this dir as working directory (`current_dir`).
pub const ZR_WORKSPACE_DIR: &str = "ZR_WORKSPACE_DIR";
/// Env var set by cargo-zng to the resources source directory.
pub const ZR_SOURCE_DIR: &str = "ZR_SOURCE_DIR";
/// Env var set by cargo-zng to the resources build target directory.
pub const ZR_TARGET_DIR: &str = "ZR_TARGET_DIR";
/// Env var set by cargo-zng to dir that the tool can use to store intermediary data for the specific request.
///
/// The cache key (dir name) is a hash of source, target, request and request content only.
pub const ZR_CACHE_DIR: &str = "ZR_CACHE_DIR";

/// Env var set by cargo-zng to the request file that called the tool.
pub const ZR_REQUEST: &str = "ZR_REQUEST";
/// Env var set by cargo-zng to the target file implied by the request file name.
///
/// That is, the request filename without `.zr-{tool}` and in the equivalent target subdirectory.
pub const ZR_TARGET: &str = "ZR_TARGET";

/// Env var set by cargo-zng when it is running a tool that requested `zng-res::on-final=` again.
pub const ZR_FINAL: &str = "ZR_FINAL";

/// Env var set by cargo-zng when it needs the tool print the help text shown in `cargo zng res --list`.
pub const ZR_HELP: &str = "ZR_HELP";

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

const COPY_HELP: &str = "
Copy the file or dir

The request file:
  source/foo.txt.zr-copy
   | # comment
   | path/bar.txt

Copies `path/bar.txt` to:
  target/foo.txt

Paths are relative to the Cargo workspace root.
";
fn copy() {
    help(COPY_HELP);

    // read source
    let source = read_path(&path(ZR_REQUEST)).unwrap_or_else(|e| fatal!("{e}"));
    // target derived from the request file name
    let mut target = path(ZR_TARGET);
    // request without name "./.zr-copy", take name from source (this is deliberate not documented)
    if target.ends_with(".zr-copy") {
        target = target.with_file_name(source.file_name().unwrap());
    }

    if source.is_dir() {
        copy_dir_all(&source, &target, true).unwrap_or_else(|e| fatal!("{e}"));
    } else {
        fs::copy(source, &target).unwrap_or_else(|e| fatal!("{e}"));
        println!("{}", display_path(&target));
    }
}

const PRINT_HELP: &str = "
Print a message
";
fn print() {
    help(PRINT_HELP);
    let message = fs::read_to_string(path(ZR_REQUEST)).unwrap_or_else(|e| fatal!("{e}"));
    println!("{message}");
}

const WARN_HELP: &str = "
Print a warning message
";
fn warn() {
    help(WARN_HELP);
    let message = fs::read_to_string(path(ZR_REQUEST)).unwrap_or_else(|e| fatal!("{e}"));
    warn!("{message}");
}

const FAIL_HELP: &str = "
Print an error message and fail the build
";
fn fail() {
    help(FAIL_HELP);
    let message = fs::read_to_string(ZR_REQUEST).unwrap_or_else(|e| fatal!("{e}"));
    fatal!("{message}");
}

const SH_HELP: &str = r#"
Run a "bash" script

The script is executed using the 'xshell' crate and will work across 
platforms if it does not use any system specific executables.

Script are configured using environment variables (like other tools):

ZR_SOURCE_DIR — Resources directory that is being build.
ZR_TARGET_DIR — Target directory where resources are bing built to.
ZR_CACHE_DIR — Dir to use for intermediary data for the specific request.
ZR_WORKSPACE_DIR — Cargo workspace, parent to the source dir. Also the working dir.
ZR_REQUEST — Request file that called the tool (.zr-sh).
ZR_TARGET — Target file implied by the request file name.

ZR_FINAL — Set if the script previously printed `zng-res::on-final={args}`.

Scripts can make requests to the resource builder by printing to stdout.
Current supported requests:

zng-res::warning={msg} — Prints the `{msg}` as a warning after the script exits.
zng-res::on-final={args} — Schedule second run with `ZR_FINAL={args}`, on final pass.

If the script fails the entire stderr is printed and the resource build fails.
"#;
fn sh() {
    help(SH_HELP);
    let sh = xshell::Shell::new().unwrap_or_else(|e| fatal!("{e}"));
    let script = fs::read_to_string(path(ZR_REQUEST)).unwrap_or_else(|e| fatal!("{e}"));
    xshell::cmd!(sh, "{script}").run().unwrap_or_else(|e| fatal!("{e}"));
}

fn read_line(path: &Path, expected: &str) -> io::Result<String> {
    let file = fs::File::open(path)?;
    for line in io::BufReader::new(file).lines() {
        let line = line?;
        let line = line.trim();
        if !line.is_empty() && !line.starts_with('#') {
            return Ok(line.to_owned());
        }
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("expected {expected} in tool file content"),
    ))
}

fn read_path(request_file: &Path) -> io::Result<PathBuf> {
    read_line(request_file, "path").map(PathBuf::from)
}

fn copy_dir_all(from: &Path, to: &Path, trace: bool) -> io::Result<()> {
    for entry in fs::read_dir(from)? {
        let from = entry?.path();
        if from.is_dir() {
            let to = to.join(from.file_name().unwrap());
            fs::create_dir(&to)?;
            if trace {
                println!("{}", to.display());
            }
            copy_dir_all(&from, &to, trace)?;
        } else if from.is_file() {
            let to = to.join(from.file_name().unwrap());
            fs::copy(&from, &to)?;
            if trace {
                println!("{}", to.display());
            }
        } else {
            continue;
        }
    }
    Ok(())
}

pub const ENV_TOOL: &str = "ZNG_RES_TOOL";

macro_rules! built_in {
    ($($tool:tt,)+) => {
        pub static BUILT_INS: &[&str] = &[
            $(stringify!($tool),)+
        ];
        static BUILT_IN_FNS: &[fn()] = &[
            $($tool,)+
        ];
    };
}
built_in! {
    copy,
    print,
    warn,
    fail,
    sh,
}

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
