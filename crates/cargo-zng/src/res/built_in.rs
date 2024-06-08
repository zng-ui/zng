//! Built-in tools

use std::{
    env, fs,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
    process::Command,
};

use convert_case::{Case, Casing};

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

/// Env var set when it needs the tool print the help text shown in `cargo zng res --list`.
pub const ZR_HELP: &str = "ZR_HELP";

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
/// Env var set to package.name
pub const ZR_PKG_NAME: &str = "ZR_PKG_NAME";
/// Env var set to package.authors
pub const ZR_PKG_AUTHORS: &str = "ZR_PKG_AUTHORS";
/// Env var set to package.name in snake_case
pub const ZR_CRATE_NAME: &str = "ZR_CRATE_NAME";
/// Env var set to package.metadata.zng.about.qualifier
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
        copy_dir_all(&source, &target, true);
    } else if source.is_file() {
        fs::copy(source, &target).unwrap_or_else(|e| fatal!("{e}"));
        println!("{}", display_path(&target));
    } else if source.is_symlink() {
        symlink_warn(&source);
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
   | !:**/pseudo*

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

    // parse first pattern
    let selection = glob::glob(&selection).unwrap_or_else(|e| fatal!("at line {ln}, {e}"));
    // parse filter patterns
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
    // collect first matches
    let selection = {
        let mut s = vec![];
        for entry in selection {
            s.push(entry.unwrap_or_else(|e| fatal!("{e}")));
        }
        // sorted for deterministic results in case flattened files override previous
        s.sort();
        s
    };

    'apply: for source in selection {
        if source.is_dir() {
            let filters_root = source.parent().map(Path::to_owned).unwrap_or_default();
            'copy_dir: for entry in walkdir::WalkDir::new(&source).sort_by_file_name() {
                let source = entry.unwrap_or_else(|e| fatal!("cannot walkdir entry `{}`, {e}", source.display()));
                let source = source.path();
                // filters match 'entry/**'
                let match_source = source.strip_prefix(&filters_root).unwrap();
                for (filter, matches_if) in &filters {
                    if filter.matches_path(match_source) != *matches_if {
                        continue 'copy_dir;
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
                    continue 'apply;
                }
            }
            let target = target.join(source_name.as_ref());

            fs::copy(&source, &target).unwrap_or_else(|e| fatal!("cannot copy `{}` to `{}`, {e}", source.display(), target.display()));
            println!("{}", display_path(&target));
        } else if source.is_symlink() {
            symlink_warn(&source);
        }
    }
}

const RP_HELP: &str = "
Replace ${VAR} occurrences in the content

The request file:
  source/greetings.txt.zr-rp
   | Thanks for using ${ZR_APP}!

Writes the text content with ZR_APP replaced:
  target/greetings.txt
  | Thanks for using Foo App!

The parameters syntax is ${VAR[:[case]][?else]}:

${VAR}          — Replaces with the ENV var value, or fails if it is not set.
${VAR:<case>}   — Replaces with the ENV var value case converted.
${VAR:?<else>}  — If ENV is not set or is set empty uses 'else' instead.
$${VAR}         — Escapes $, replaces with '${VAR}'. 

The :<case> functions are:

:k — kebab-case
:K — UPPER-KEBAB-CASE
:s — snake_case
:S — UPPER_SNAKE_CASE
:l — lower case
:U — UPPER CASE
:T — Title Case
:c — camelCase
:P — PascalCase
:Tr — Train-Case
: — Unchanged

The fallback(else) can have nested ${VAR} patterns.

Variables:

All env variables can be used, of particular use with this tool are:

ZR_APP — package.metadata.zng.about.app or package.name
ZR_ORG — package.metadata.zng.about.org or the first package.authors
ZR_VERSION — package.version
ZR_DESCRIPTION — package.description
ZR_HOMEPAGE — package.homepage
ZR_PKG_NAME — package.name
ZR_PKG_AUTHORS — package.authors
ZR_CRATE_NAME — package.name in snake_case
ZR_QUALIFIER — package.metadata.zng.about.qualifier

See `zng::env::about` for more details about metadata vars.
See the cargo-zng crate docs for a full list of ZR vars.

";
fn rp() {
    help(RP_HELP);

    // target derived from the request place
    let content = fs::File::open(path(ZR_REQUEST)).unwrap_or_else(|e| fatal!("cannot read, {e}"));
    let target = path(ZR_TARGET);
    let target = fs::File::create(target).unwrap_or_else(|e| fatal!("cannot write, {e}"));
    let mut target = io::BufWriter::new(target);

    let mut content = io::BufReader::new(content);
    let mut line = String::new();
    let mut ln = 1;
    while content.read_line(&mut line).unwrap_or_else(|e| fatal!("cannot read, {e}")) > 0 {
        let line_r = replace(&line, 0).unwrap_or_else(|e| fatal!("line {ln}, {e}"));
        target.write_all(line_r.as_bytes()).unwrap_or_else(|e| fatal!("cannot write, {e}"));
        ln += 1;
        line.clear();
    }
    target.flush().unwrap_or_else(|e| fatal!("cannot write, {e}"));
}

const MAX_RECURSION: usize = 32;
fn replace(line: &str, recursion_depth: usize) -> Result<String, String> {
    let mut n2 = '\0';
    let mut n1 = '\0';
    let mut out = String::with_capacity(line.len());

    let mut iterator = line.char_indices();
    'main: while let Some((ci, c)) = iterator.next() {
        if n1 == '$' && c == '{' {
            out.pop();
            if n2 == '$' {
                out.push('{');
                n1 = '{';
                continue 'main;
            }

            let start = ci + 1;
            let mut depth = 0;
            let mut end = usize::MAX;
            'seek_end: for (i, c) in iterator.by_ref() {
                if c == '{' {
                    depth += 1;
                } else if c == '}' {
                    if depth == 0 {
                        end = i;
                        break 'seek_end;
                    }
                    depth -= 1;
                }
            }
            if end == usize::MAX {
                let end = (start + 10).min(line.len());
                return Err(format!("replace not closed at: ${{{}", &line[start..end]));
            } else {
                let mut var = &line[start..end];
                let mut case = "";
                let mut fallback = None;
                if let Some(i) = var.find('?') {
                    fallback = Some(&var[i + 1..]);
                    var = &var[..i];
                }
                if let Some(i) = var.find(':') {
                    case = &var[i + 1..];
                    var = &var[..i];
                }

                if let Ok(value) = env::var(var) {
                    let value = match case {
                        "k" => value.to_case(Case::Kebab),
                        "K" => value.to_case(Case::UpperKebab),
                        "s" => value.to_case(Case::Snake),
                        "S" => value.to_case(Case::UpperSnake),
                        "l" => value.to_case(Case::Lower),
                        "U" => value.to_case(Case::Upper),
                        "T" => value.to_case(Case::Title),
                        "c" => value.to_case(Case::Camel),
                        "P" => value.to_case(Case::Pascal),
                        "Tr" => value.to_case(Case::Train),
                        "" => value,
                        unknown => return Err(format!("unknown case '{unknown}'")),
                    };
                    out.push_str(&value);
                } else if let Some(fallback) = fallback {
                    if let Some(error) = fallback.strip_prefix('!') {
                        if error.contains('$') && recursion_depth < MAX_RECURSION {
                            return Err(replace(error, recursion_depth + 1).unwrap_or_else(|_| error.to_owned()));
                        } else {
                            return Err(error.to_owned());
                        }
                    } else if fallback.contains('$') && recursion_depth < MAX_RECURSION {
                        out.push_str(&replace(fallback, recursion_depth + 1)?);
                    } else {
                        out.push_str(fallback);
                    }
                } else {
                    return Err(format!("env var ${{{var}}} is not set"));
                }
            }
        } else {
            out.push(c);
        }
        n2 = n1;
        n1 = c;
    }
    Ok(out)
}

const WARN_HELP: &str = "
Print a warning message

You can combine this with '.zr-rp' tool

The request file:
  source/warn.zr-warn.zr-rp
   | ${ZR_APP}!

Prints a warning with the value of ZR_APP
";
fn warn() {
    help(WARN_HELP);
    let message = fs::read_to_string(path(ZR_REQUEST)).unwrap_or_else(|e| fatal!("{e}"));
    println!("zng-res::warning={message}");
}

const FAIL_HELP: &str = "
Print an error message and fail the build

The request file:
  some/dir/disallow.zr-fail.zr-rp
   | Don't copy ${ZR_REQUEST_DD} with a glob!

Prints an error message and fails the build if copied
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
ZR_TARGET_DIR — Target directory where resources are being built to.
ZR_CACHE_DIR — Dir to use for intermediary data for the specific request.
ZR_WORKSPACE_DIR — Cargo workspace that contains source dir. Also the working dir.
ZR_REQUEST — Request file that called the tool (.zr-sh).
ZR_REQUEST_DD — Parent dir of the request file.
ZR_TARGET — Target file implied by the request file name.
ZR_TARGET_DD — Parent dir of the target file.

ZR_FINAL — Set if the script previously printed `zng-res::on-final={args}`.

In a Cargo workspace the `zng::env::about` metadata is also set:

ZR_APP — package.metadata.zng.about.app or package.name
ZR_ORG — package.metadata.zng.about.org or the first package.authors
ZR_VERSION — package.version
ZR_DESCRIPTION — package.description
ZR_HOMEPAGE — package.homepage
ZR_PKG_NAME — package.name
ZR_PKG_AUTHORS — package.authors
ZR_CRATE_NAME — package.name in snake_case
ZR_QUALIFIER — package.metadata.zng.about.qualifier

Script can make requests to the resource builder by printing to stdout.
Current supported requests:

zng-res::warning={msg} — Prints the `{msg}` as a warning after the script exits.
zng-res::on-final={args} — Schedule second run with `ZR_FINAL={args}`, on final pass.

If the script fails the entire stderr is printed and the resource build fails.

Runs on $ZR_SH, $PROGRAMFILES/Git/bin/sh.exe or sh.
"#;
fn sh() {
    help(SH_HELP);
    sh_impl();
}
fn sh_impl() {
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

const SHF_HELP: &str = r#"
Run a bash script on the final pass

Apart from running on final this tool behaves exactly like .zr-sh
"#;
fn shf() {
    help(SHF_HELP);
    if std::env::var(ZR_FINAL).is_ok() {
        sh_impl()
    } else {
        println!("zng-res::on-final=");
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

fn copy_dir_all(from: &Path, to: &Path, trace: bool) {
    for entry in walkdir::WalkDir::new(from).min_depth(1).max_depth(1).sort_by_file_name() {
        let entry = entry.unwrap_or_else(|e| fatal!("cannot walkdir entry `{}`, {e}", from.display()));
        let from = entry.path();
        let to = to.join(entry.file_name());
        if entry.file_type().is_dir() {
            fs::create_dir(&to).unwrap_or_else(|e| fatal!("cannot create_dir `{}`, {e}", to.display()));
            if trace {
                println!("{}", display_path(&to));
            }
            copy_dir_all(from, &to, trace);
        } else if entry.file_type().is_file() {
            fs::copy(from, &to).unwrap_or_else(|e| fatal!("cannot copy `{}` to `{}`, {e}", from.display(), to.display()));
            if trace {
                println!("{}", display_path(&to));
            }
        } else if entry.file_type().is_symlink() {
            symlink_warn(entry.path())
        }
    }
}

pub(crate) fn symlink_warn(path: &Path) {
    warn!("symlink ignored in `{}`, use zr-tools to 'link'", path.display());
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
    rp,
    sh,
    shf,
    warn,
    fail,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_tests() {
        std::env::set_var("ZR_RP_TEST", "test value");

        assert_eq!("", replace("", 0).unwrap());
        assert_eq!("normal text", replace("normal text", 0).unwrap());
        assert_eq!("escaped ${NOT}", replace("escaped $${NOT}", 0).unwrap());
        assert_eq!("replace 'test value'", replace("replace '${ZR_RP_TEST}'", 0).unwrap());
        assert_eq!("env var ${} is not set", replace("empty '${}'", 0).unwrap_err()); // hmm
        assert_eq!(
            "env var ${ZR_RP_TEST_NOT_SET} is not set",
            replace("not set '${ZR_RP_TEST_NOT_SET}'", 0).unwrap_err()
        );
        assert_eq!(
            "not set 'fallback!'",
            replace("not set '${ZR_RP_TEST_NOT_SET?fallback!}'", 0).unwrap()
        );
        assert_eq!(
            "not set 'nested 'test value'.'",
            replace("not set '${ZR_RP_TEST_NOT_SET?nested '${ZR_RP_TEST}'.}'", 0).unwrap()
        );
        assert_eq!("test value", replace("${ZR_RP_TEST_NOT_SET?${ZR_RP_TEST}}", 0).unwrap());
        assert_eq!(
            "curly test value",
            replace("curly ${ZR_RP_TEST?{not {what} {is} {going {on {here {?}}}}}}", 0).unwrap()
        );

        assert_eq!("replace not closed at: ${MISSING", replace("${MISSING", 0).unwrap_err());
        assert_eq!("replace not closed at: ${MIS", replace("${MIS", 0).unwrap_err());
        assert_eq!("replace not closed at: ${MIS?{", replace("${MIS?{", 0).unwrap_err());
        assert_eq!("replace not closed at: ${MIS?{}", replace("${MIS?{}", 0).unwrap_err());

        assert_eq!("TEST VALUE", replace("${ZR_RP_TEST:U}", 0).unwrap());
        assert_eq!("TEST-VALUE", replace("${ZR_RP_TEST:K}", 0).unwrap());
        assert_eq!("TEST_VALUE", replace("${ZR_RP_TEST:S}", 0).unwrap());
        assert_eq!("testValue", replace("${ZR_RP_TEST:c}", 0).unwrap());
    }
}
