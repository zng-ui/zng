//! Built-in tools

use std::{
    env, fs,
    io::{self, BufRead},
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::Context;

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
        fs::create_dir(&target).unwrap_or_else(|e| fatal!("{e}"));
        copy_dir_all(&source, &target, true).unwrap_or_else(|e| fatal!("{e}"));
    } else {
        fs::copy(source, &target).unwrap_or_else(|e| fatal!("{e}"));
        println!("{}", display_path(&target));
    }
}

const GLOB_HELP: &str = "
Copy all matches in place

The request file:
  source/l10n/fluent-files.zr-glob
   | # localization dir
   | l10n
   | # only Fluent files
   | **/*.ftl
   | # except test locales
   | !:*pseudo*

Copies all '.ftl' not in a *pseudo* path to:
  target/l10n/

The first path pattern is required and defines the entries that
will be copied, an initial pattern with '**' flattens the matches.
The path is relative to the Cargo workspace root.

The subsequent patterns are optional and filter each file or dir selected by
the first pattern. The paths are relative to each match, if it is a file 
the filters apply to the file name only, if it is a dir the filters apply to
the dir and descendants.

The glob pattern syntax is:

    ? — matches any single character.
    * — matches any (possibly empty) sequence of characters.
   ** — matches the current directory and arbitrary subdirectories.
  [c] — matches any character inside the brackets.
[a-z] — matches any characters in the Unicode sequence.
 [!b] — negates the brackets match.

And in filter patterns only:

!:pattern — negates the entire pattern.

";
fn glob() {
    help(GLOB_HELP);

    // target derived from the request place
    let target = path(ZR_TARGET);
    let target = target.parent().unwrap();

    let mut lines = read_lines(&path(ZR_REQUEST));
    let (ln, selection) = lines
        .next()
        .unwrap_or_else(|| fatal!("expected at least one path pattern"))
        .unwrap_or_else(|e| fatal!("{e}"));

    let selection = glob::glob(&selection).unwrap_or_else(|e| fatal!("at line {ln}, {e}"));
    let mut filters = vec![];
    for r in lines {
        let (ln, filter) = r.unwrap_or_else(|e| fatal!("{e}"));
        let (filter, matches_if) = if let Some(f) = filter.strip_prefix("!:") {
            (f, false)
        } else {
            (filter.as_str(), true)
        };
        let pat = glob::Pattern::new(filter).unwrap_or_else(|e| fatal!("at line {ln}, {e}"));
        filters.push((pat, matches_if));
    }

    'selection: for entry in selection {
        let source = entry.unwrap_or_else(|e| fatal!("{e}"));

        // copy not filtered
        if source.is_dir() {
            let strip = source.parent().map(Path::to_owned).unwrap_or_default();
            'walk: for entry in walkdir::WalkDir::new(&source) {
                let source = entry.unwrap_or_else(|e| fatal!("cannot walkdir entry `{}`, {e}", source.display()));
                let source = source.path();
                // filters match 'entry/**'
                let match_source = source.strip_prefix(&strip).unwrap();
                for (filter, matches_if) in &filters {
                    if filter.matches_path(match_source) != *matches_if {
                        continue 'walk;
                    }
                }
                let target = target.join(match_source);

                if source.is_dir() {
                    fs::create_dir_all(&target).unwrap_or_else(|e| fatal!("cannot create dir `{}`, {e}", source.display()));
                } else {
                    if let Some(p) = &target.parent() {
                        fs::create_dir_all(p).unwrap_or_else(|e| fatal!("cannot create dir `{}`, {e}", p.display()));
                    }
                    fs::copy(source, &target)
                        .unwrap_or_else(|e| fatal!("cannot copy `{}` to `{}`, {e}", source.display(), target.display()));
                }
            }
        } else if source.is_file() {
            // filters match 'entry'
            let source_name = source.file_name().unwrap().to_string_lossy();
            for (filter, matches_if) in &filters {
                if filter.matches(&source_name) != *matches_if {
                    continue 'selection;
                }
            }
            let target = target.join(source_name.as_ref());

            fs::copy(&source, &target).unwrap_or_else(|e| fatal!("cannot copy `{}` to `{}`, {e}", source.display(), target.display()));
            println!("{}", display_path(&target));
        }
    }
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
Run a bash script

Script is configured using environment variables (like other tools):

ZR_SOURCE_DIR — Resources directory that is being build.
ZR_TARGET_DIR — Target directory where resources are bing built to.
ZR_CACHE_DIR — Dir to use for intermediary data for the specific request.
ZR_WORKSPACE_DIR — Cargo workspace, parent to the source dir. Also the working dir.
ZR_REQUEST — Request file that called the tool (.zr-sh).
ZR_TARGET — Target file implied by the request file name.

ZR_FINAL — Set if the script previously printed `zng-res::on-final={args}`.

Script can make requests to the resource builder by printing to stdout.
Current supported requests:

zng-res::warning={msg} — Prints the `{msg}` as a warning after the script exits.
zng-res::on-final={args} — Schedule second run with `ZR_FINAL={args}`, on final pass.

If the script fails the entire stderr is printed and the resource build fails.

Runs on $ZR_SH, $PROGRAMFILES/Git/bin/sh.exe or sh.
"#;
fn sh() {
    help(SH_HELP);
    if let Ok(sh) = env::var("ZR_SH") {
        if !sh.is_empty() {
            let sh = PathBuf::from(sh);
            if sh.exists() {
                return sh_run(sh);
            }
        }
    }

    #[cfg(windows)]
    sh_run(sh_windows().unwrap_or_else(|| fatal!("bash not found, set %ZR_SH% or install Git bash")));

    #[cfg(not(windows))]
    sh_run("sh");
}
fn sh_run(sh: impl AsRef<std::ffi::OsStr>) {
    let script = fs::read_to_string(path(ZR_REQUEST)).unwrap_or_else(|e| fatal!("{e}"));
    match Command::new(sh).arg("-c").arg(script).status() {
        Ok(s) => {
            if !s.success() {
                match s.code() {
                    Some(c) => fatal!("script failed, exit code {c}"),
                    None => fatal!("script failed"),
                }
            }
        }
        Err(e) => {
            if e.kind() == io::ErrorKind::NotFound {
                fatal!("bash not found, set $ZR_SH")
            } else {
                fatal!("{e}")
            }
        }
    }
}

#[cfg(windows)]
fn sh_windows() -> Option<PathBuf> {
    if let Ok(pf) = env::var("PROGRAMFILES") {
        let sh = PathBuf::from(pf).join("Git/bin/sh.exe");
        if sh.exists() {
            return Some(sh);
        }
    }
    if let Ok(c) = env::var("SYSTEMDRIVE") {
        let sh = PathBuf::from(c).join("Program Files (x86)/Git/bin/sh.exe");
        if sh.exists() {
            return Some(sh);
        }
    }

    None
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
    std::iter::from_fn(move || loop {
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
    })
}

fn read_path(request_file: &Path) -> io::Result<PathBuf> {
    read_line(request_file, "path").map(PathBuf::from)
}

fn copy_dir_all(from: &Path, to: &Path, trace: bool) -> anyhow::Result<()> {
    for entry in fs::read_dir(from).with_context(|| format!("cannot read_dir `{}`", from.display()))? {
        let from = entry.with_context(|| format!("cannot read_dir entry `{}`", from.display()))?.path();
        if from.is_dir() {
            let to = to.join(from.file_name().unwrap());
            fs::create_dir(&to).with_context(|| format!("cannot create_dir `{}`", to.display()))?;
            if trace {
                println!("{}", display_path(&to));
            }
            copy_dir_all(&from, &to, trace)?;
        } else if from.is_file() {
            let to = to.join(from.file_name().unwrap());
            fs::copy(&from, &to).with_context(|| format!("cannot copy `{}` to `{}`", from.display(), to.display()))?;
            if trace {
                println!("{}", display_path(&to));
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
    glob,
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
