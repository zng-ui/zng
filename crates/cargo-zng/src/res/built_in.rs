//! Built-in tools

use std::{
    env, fs,
    io::{self, BufRead, Write},
    mem,
    path::{Path, PathBuf},
    process::Command,
};

use convert_case::{Case, Casing};

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
        println!("{}", display_path(&target));
        fs::create_dir(&target).unwrap_or_else(|e| {
            if e.kind() != io::ErrorKind::AlreadyExists {
                fatal!("{e}")
            }
        });
        copy_dir_all(&source, &target, true);
    } else if source.is_file() {
        println!("{}", display_path(&target));
        fs::copy(source, &target).unwrap_or_else(|e| fatal!("{e}"));
    } else if source.is_symlink() {
        symlink_warn(&source);
    } else {
        warn!("cannot copy '{}', not found", source.display());
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

    let request_path = path(ZR_REQUEST);
    let mut lines = read_lines(&request_path);
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

    let mut any = false;

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

                any = true;
                if source.is_dir() {
                    fs::create_dir_all(&target).unwrap_or_else(|e| fatal!("cannot create dir `{}`, {e}", source.display()));
                } else {
                    if let Some(p) = &target.parent() {
                        fs::create_dir_all(p).unwrap_or_else(|e| fatal!("cannot create dir `{}`, {e}", p.display()));
                    }
                    fs::copy(source, &target)
                        .unwrap_or_else(|e| fatal!("cannot copy `{}` to `{}`, {e}", source.display(), target.display()));
                }
                println!("{}", display_path(&target));
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

            any = true;
            fs::copy(&source, &target).unwrap_or_else(|e| fatal!("cannot copy `{}` to `{}`, {e}", source.display(), target.display()));
            println!("{}", display_path(&target));
        } else if source.is_symlink() {
            symlink_warn(&source);
        }
    }

    if !any {
        warn!("no match")
    }
}

const RP_HELP: &str = "
Replace ${VAR|<file|!cmd} occurrences in the content

The request file:
  source/greetings.txt.zr-rp
   | Thanks for using ${ZR_APP}!

Writes the text content with ZR_APP replaced:
  target/greetings.txt
  | Thanks for using Foo App!

The parameters syntax is ${VAR|!|<[:[case]][?else]}:

${VAR}          — Replaces with the env var value, or fails if it is not set.
${VAR:case}     — Replaces with the env var value, case converted.
${VAR:?else}    — If VAR is not set or is empty uses 'else' instead.

${<file.txt}    — Replaces with the 'file.txt' content. 
                  Paths are relative to the workspace root.
${<file:case}   — Replaces with the 'file.txt' content, case converted.
${<file:?else}  — If file cannot be read or is empty uses 'else' instead.

${!cmd -h}      — Replaces with the stdout of the bash script line. 
                  The script runs the same bash used by '.zr-sh'.
                  The script must be defined all in one line.
                  A separate bash instance is used for each occurrence.
                  The working directory is the workspace root.
${!cmd:case}    — Replaces with the stdout, case converted. 
                  If the script contains ':' quote it with double quotes\"
$!{!cmd:?else}  — If script fails or ha no stdout, uses 'else' instead.

$${VAR}         — Escapes $, replaces with '${VAR}'.

The :case functions are:

:k or :kebab  — kebab-case (cleaned)
:K or :KEBAB  — UPPER-KEBAB-CASE (cleaned)
:s or :snake  — snake_case (cleaned)
:S or :SNAKE  — UPPER_SNAKE_CASE (cleaned)
:l or :lower  — lower case
:U or :UPPER  — UPPER CASE
:T or :Title  — Title Case
:c or :camel  — camelCase (cleaned)
:P or :Pascal — PascalCase (cleaned)
:Tr or :Train — Train-Case (cleaned)
:           — Unchanged
:clean      — Cleaned
:f or :file — Sanitize file name

Cleaned values only keep ascii alphabetic first char and ascii alphanumerics, ' ', '-' and '_' other chars.
More then one case function can be used, separated by pipe ':T|f' converts to title case and sanitize for file name. 


The fallback(:?else) can have nested ${...} patterns. 
You can set both case and else: '${VAR:case?else}'.

Variables:

All env variables can be used, of particular use with this tool are:

ZR_APP — package.metadata.zng.about.app or package.name
ZR_ORG — package.metadata.zng.about.org or the first package.authors
ZR_VERSION — package.version
ZR_DESCRIPTION — package.description
ZR_HOMEPAGE — package.homepage
ZR_LICENSE — package.license
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

                // escape ":"
                let mut search_start = 0;
                if var.starts_with('!') {
                    let mut quoted = false;
                    let mut escape_next = false;
                    for (i, c) in var.char_indices() {
                        if mem::take(&mut escape_next) {
                            continue;
                        }
                        if c == '\\' {
                            escape_next = true;
                        } else if c == '"' {
                            quoted = !quoted;
                        } else if !quoted && c == ':' {
                            search_start = i;
                            break;
                        }
                    }
                }
                if let Some(i) = var[search_start..].find(':') {
                    let i = search_start + i;
                    case = &var[i + 1..];
                    var = &var[..i];
                    if let Some(i) = case.find('?') {
                        fallback = Some(&case[i + 1..]);
                        case = &case[..i];
                    }
                }

                let value = if let Some(path) = var.strip_prefix('<') {
                    match std::fs::read_to_string(path) {
                        Ok(s) => Some(s),
                        Err(e) => {
                            error!("cannot read `{path}`, {e}");
                            None
                        }
                    }
                } else if let Some(script) = var.strip_prefix('!') {
                    match sh_run(script.to_owned(), true, None) {
                        Ok(r) => Some(r),
                        Err(e) => fatal!("{e}"),
                    }
                } else {
                    env::var(var).ok()
                };

                let value = match value {
                    Some(s) => {
                        let st = s.trim();
                        if st.is_empty() {
                            None
                        } else if st == s {
                            Some(s)
                        } else {
                            Some(st.to_owned())
                        }
                    }
                    _ => None,
                };

                if let Some(mut value) = value {
                    for case in case.split('|') {
                        value = match case {
                            "k" | "kebab" => util::clean_value(&value, false).unwrap().to_case(Case::Kebab),
                            "K" | "KEBAB" => util::clean_value(&value, false).unwrap().to_case(Case::UpperKebab),
                            "s" | "snake" => util::clean_value(&value, false).unwrap().to_case(Case::Snake),
                            "S" | "SNAKE" => util::clean_value(&value, false).unwrap().to_case(Case::UpperSnake),
                            "l" | "lower" => value.to_case(Case::Lower),
                            "U" | "UPPER" => value.to_case(Case::Upper),
                            "T" | "Title" => value.to_case(Case::Title),
                            "c" | "camel" => util::clean_value(&value, false).unwrap().to_case(Case::Camel),
                            "P" | "Pascal" => util::clean_value(&value, false).unwrap().to_case(Case::Pascal),
                            "Tr" | "Train" => util::clean_value(&value, false).unwrap().to_case(Case::Train),
                            "" => value,
                            "clean" => util::clean_value(&value, false).unwrap(),
                            "f" | "file" => sanitise_file_name::sanitise(&value),
                            unknown => return Err(format!("unknown case '{unknown}'")),
                        };
                    }
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
                    return Err(format!("${{{var}}} cannot be read or is empty"));
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
ZR_LICENSE — package.license
ZR_PKG_NAME — package.name
ZR_PKG_AUTHORS — package.authors
ZR_CRATE_NAME — package.name in snake_case
ZR_QUALIFIER — package.metadata.zng.about.qualifier

Script can make requests to the resource builder by printing to stdout.
Current supported requests:

zng-res::warning={msg} — Prints the `{msg}` as a warning after the script exits.
zng-res::on-final={args} — Schedule second run with `ZR_FINAL={args}`, on final pass.

If the script fails the entire stderr is printed and the resource build fails. Scripts run with
`set -e` by default.

Tries to run on $ZR_SH, $PROGRAMFILES/Git/bin/bash.exe, bash, sh.
"#;
fn sh() {
    help(SH_HELP);
    let script = fs::read_to_string(path(ZR_REQUEST)).unwrap_or_else(|e| fatal!("{e}"));
    sh_run(script, false, None).unwrap_or_else(|e| fatal!("{e}"));
}

fn sh_options() -> Vec<std::ffi::OsString> {
    let mut r = vec![];
    if let Ok(sh) = env::var("ZR_SH")
        && !sh.is_empty()
    {
        let sh = PathBuf::from(sh);
        if sh.exists() {
            r.push(sh.into_os_string());
        }
    }

    #[cfg(windows)]
    if let Ok(pf) = env::var("PROGRAMFILES") {
        let sh = PathBuf::from(pf).join("Git/bin/bash.exe");
        if sh.exists() {
            r.push(sh.into_os_string());
        }
    }
    #[cfg(windows)]
    if let Ok(c) = env::var("SYSTEMDRIVE") {
        let sh = PathBuf::from(c).join("Program Files (x86)/Git/bin/bash.exe");
        if sh.exists() {
            r.push(sh.into_os_string());
        }
    }

    r.push("bash".into());
    r.push("sh".into());

    r
}
pub(crate) fn sh_run(mut script: String, capture: bool, current_dir: Option<&Path>) -> io::Result<String> {
    script.insert_str(0, "set -e\n");

    for opt in sh_options() {
        let r = sh_run_try(&opt, &script, capture, current_dir)?;
        if let Some(r) = r {
            return Ok(r);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "cannot find bash, tried $ZR_SH, $PROGRAMFILES/Git/bin/bash.exe, bash, sh",
    ))
}
fn sh_run_try(sh: &std::ffi::OsStr, script: &str, capture: bool, current_dir: Option<&Path>) -> io::Result<Option<String>> {
    let mut sh = Command::new(sh);
    if let Some(d) = current_dir {
        sh.current_dir(d);
    }
    sh.arg("-c").arg(script);
    sh.stdin(std::process::Stdio::null());
    sh.stderr(std::process::Stdio::inherit());
    let r = if capture {
        sh.output().map(|o| (o.status, String::from_utf8_lossy(&o.stdout).into_owned()))
    } else {
        sh.stdout(std::process::Stdio::inherit());
        sh.status().map(|s| (s, String::new()))
    };
    match r {
        Ok((s, o)) => {
            if !s.success() {
                return Err(match s.code() {
                    Some(c) => io::Error::other(format!("script failed, exit code {c}")),
                    None => io::Error::other("script failed"),
                });
            }
            Ok(Some(o))
        }
        Err(e) => {
            if e.kind() == io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(e)
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
        sh();
    } else {
        println!("zng-res::on-final=");
    }
}

const APK_HELP: &str = r#"
Build an Android APK from a staging directory

The expected file system layout:

| apk/
| ├── lib/
| |   └── arm64-v8a
| |       └── my-app.so
| ├── assets/
| |   └── res
| |       └── zng-res.txt
| ├── res/
| |   └── android-res
| └── AndroidManifest.xml
| my-app.zr-apk

Both 'apk/' and 'my-app.zr-apk' will be replaced with the built my-app.apk

Expected .zr-apk file content:

| # Relative path to the staging directory. If not set uses ./apk if it exists
| # or the parent dir .. if it is named something.apk
| apk-dir = ./apk
|
| # Sign using the debug key. Note that if ZR_APK_KEYSTORE or ZR_APK_KEY_ALIAS are not
| # set the APK is also signed using the debug key.
| debug = true
|
| # Don't sign and don't zipalign the APK. This outputs an incomplete package that
| # cannot be installed, but can be modified such as custom linking and signing.
| raw = true
|
| # Don't tar assets. By default `assets/res` are packed as `assets/res.tar`
| # for use with `android_install_res`.
| tar-assets-res = false

APK signing is configured using these environment variables:

ZR_APK_KEYSTORE - path to the private .keystore file
ZR_APK_KEYSTORE_PASS - keystore file password
ZR_APK_KEY_ALIAS - key name in the keystore
ZR_APK_KEY_PASS - key password
"#;
fn apk() {
    help(APK_HELP);
    if std::env::var(ZR_FINAL).is_err() {
        println!("zng-res::on-final=");
        return;
    }

    // read config
    let mut apk_dir = String::new();
    let mut debug = false;
    let mut raw = false;
    let mut tar_assets = true;
    for line in read_lines(&path(ZR_REQUEST)) {
        let (ln, line) = line.unwrap_or_else(|e| fatal!("error reading .zr-apk request, {e}"));
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            let bool_value = || match value {
                "true" => true,
                "false" => false,
                _ => {
                    error!("unexpected value, line {ln}\n   {line}");
                    false
                }
            };
            match key {
                "apk-dir" => apk_dir = value.to_owned(),
                "debug" => debug = bool_value(),
                "raw" => raw = bool_value(),
                "tar-assets" => tar_assets = bool_value(),
                _ => error!("unknown key, line {ln}\n   {line}"),
            }
        } else {
            error!("syntax error, line {ln}\n{line}");
        }
    }
    let mut keystore = PathBuf::from(env::var("ZR_APK_KEYSTORE").unwrap_or_default());
    let mut keystore_pass = env::var("ZR_APK_KEYSTORE_PASS").unwrap_or_default();
    let mut key_alias = env::var("ZR_APK_KEY_ALIAS").unwrap_or_default();
    let mut key_pass = env::var("ZR_APK_KEY_PASS").unwrap_or_default();
    if keystore.as_os_str().is_empty() || key_alias.is_empty() {
        debug = true;
    }

    let mut apk_folder = path(ZR_TARGET_DD);
    let output_file;
    if apk_dir.is_empty() {
        let apk = apk_folder.join("apk");
        if apk.exists() {
            apk_folder = apk;
            output_file = path(ZR_TARGET).with_extension("apk");
        } else if apk_folder.extension().map(|e| e.eq_ignore_ascii_case("apk")).unwrap_or(false) {
            output_file = apk_folder.clone();
        } else {
            fatal!("missing ./apk")
        }
    } else {
        apk_folder = apk_folder.join(apk_dir);
        if !apk_folder.is_dir() {
            fatal!("{} not found or not a directory", apk_folder.display());
        }
        output_file = path(ZR_TARGET).with_extension("apk");
    }
    let apk_folder = apk_folder;

    // find <sdk>/build-tools
    let android_home = match env::var("ANDROID_HOME") {
        Ok(h) if !h.is_empty() => h,
        _ => fatal!("please set ANDROID_HOME to the android-sdk dir"),
    };
    let build_tools = Path::new(&android_home).join("build-tools/");
    let mut best_build = None;
    let mut best_version = semver::Version::new(0, 0, 0);

    #[cfg(not(windows))]
    const AAPT2_NAME: &str = "aapt2";
    #[cfg(windows)]
    const AAPT2_NAME: &str = "aapt2.exe";

    for dir in fs::read_dir(build_tools).unwrap_or_else(|e| fatal!("cannot read $ANDROID_HOME/build-tools/, {e}")) {
        let dir = dir
            .unwrap_or_else(|e| fatal!("cannot read $ANDROID_HOME/build-tools/ entry, {e}"))
            .path();

        if let Some(ver) = dir
            .file_name()
            .and_then(|f| f.to_str())
            .and_then(|f| semver::Version::parse(f).ok())
            && ver > best_version
            && dir.join(AAPT2_NAME).exists()
        {
            best_build = Some(dir);
            best_version = ver;
        }
    }
    let build_tools = match best_build {
        Some(p) => p,
        None => fatal!("cannot find $ANDROID_HOME/build-tools/<version>/{AAPT2_NAME}"),
    };
    let aapt2_path = build_tools.join(AAPT2_NAME);

    // temp target dir
    let temp_dir = apk_folder.with_extension("apk.tmp");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir(&temp_dir).unwrap_or_else(|e| fatal!("cannot create {}, {e}", temp_dir.display()));

    // tar assets
    let assets = apk_folder.join("assets");
    let assets_res = assets.join("res");
    if tar_assets && assets_res.exists() {
        let tar_path = assets.join("res.tar");
        let r = Command::new("tar")
            .arg("-cf")
            .arg(&tar_path)
            .arg("res")
            .current_dir(&assets)
            .status();
        match r {
            Ok(s) => {
                if !s.success() {
                    fatal!("tar failed")
                }
            }
            Err(e) => fatal!("cannot run 'tar', {e}"),
        }
        if let Err(e) = fs::remove_dir_all(&assets_res) {
            fatal!("failed tar-assets-res cleanup, {e}")
        }
    }

    // build resources
    let compiled_res = temp_dir.join("compiled_res.zip");
    let res = apk_folder.join("res");
    if res.exists() {
        let mut aapt2 = Command::new(&aapt2_path);
        aapt2.arg("compile").arg("-o").arg(&compiled_res).arg("--dir").arg(res);

        if aapt2.status().map(|s| !s.success()).unwrap_or(true) {
            fatal!("resources build failed");
        }
    }

    let manifest_path = apk_folder.join("AndroidManifest.xml");
    let manifest = fs::read_to_string(&manifest_path).unwrap_or_else(|e| fatal!("cannot read AndroidManifest.xml, {e}"));
    let manifest: AndroidManifest = quick_xml::de::from_str(&manifest).unwrap_or_else(|e| fatal!("error parsing AndroidManifest.xml, {e}"));

    // find <sdk>/platforms
    let platforms = Path::new(&android_home).join("platforms");
    let mut best_platform = None;
    let mut best_version = 0;
    for dir in fs::read_dir(platforms).unwrap_or_else(|e| fatal!("cannot read $ANDROID_HOME/platforms/, {e}")) {
        let dir = dir
            .unwrap_or_else(|e| fatal!("cannot read $ANDROID_HOME/platforms/ entry, {e}"))
            .path();

        if let Some(ver) = dir
            .file_name()
            .and_then(|f| f.to_str())
            .and_then(|f| f.strip_prefix("android-"))
            .and_then(|f| f.parse().ok())
            && manifest.uses_sdk.matches(ver)
            && ver > best_version
            && dir.join("android.jar").exists()
        {
            best_platform = Some(dir);
            best_version = ver;
        }
    }
    let platform = match best_platform {
        Some(p) => p,
        None => fatal!("cannot find $ANDROID_HOME/platforms/<version>/android.jar"),
    };

    // make apk (link)
    let apk_path = temp_dir.join("output.apk");
    let mut aapt2 = Command::new(&aapt2_path);
    aapt2
        .arg("link")
        .arg("-o")
        .arg(&apk_path)
        .arg("--manifest")
        .arg(manifest_path)
        .arg("-I")
        .arg(platform.join("android.jar"));
    if compiled_res.exists() {
        aapt2.arg(&compiled_res);
    }
    if assets.exists() {
        aapt2.arg("-A").arg(&assets);
    }
    if aapt2.status().map(|s| !s.success()).unwrap_or(true) {
        fatal!("apk linking failed");
    }

    // add libs
    let aapt_path = build_tools.join("aapt");
    for lib in glob::glob(apk_folder.join("lib/*/*.so").display().to_string().as_str()).unwrap() {
        let lib = lib.unwrap_or_else(|e| fatal!("error searching libs, {e}"));

        let lib = lib.display().to_string().replace('\\', "/");
        let lib = &lib[lib.rfind("/lib/").unwrap() + 1..];

        let mut aapt = Command::new(&aapt_path);
        aapt.arg("add").arg(&apk_path).arg(lib).current_dir(&apk_folder);
        if aapt.status().map(|s| !s.success()).unwrap_or(true) {
            fatal!("apk linking failed");
        }
    }

    let final_apk = if raw {
        apk_path
    } else {
        // align
        let aligned_apk_path = temp_dir.join("output-aligned.apk");
        let zipalign_path = build_tools.join("zipalign");
        let mut zipalign = Command::new(zipalign_path);
        zipalign.arg("-v").arg("4").arg(apk_path).arg(&aligned_apk_path);
        if zipalign.status().map(|s| !s.success()).unwrap_or(true) {
            fatal!("zipalign failed");
        }

        // sign
        let signed_apk_path = temp_dir.join("output-signed.apk");
        if debug {
            let dirs = directories::BaseDirs::new().unwrap_or_else(|| fatal!("cannot fine $HOME"));
            keystore = dirs.home_dir().join(".android/debug.keystore");
            keystore_pass = "android".to_owned();
            key_alias = "androiddebugkey".to_owned();
            key_pass = "android".to_owned();
            if !keystore.exists() {
                // generate debug.keystore
                let _ = fs::create_dir_all(keystore.parent().unwrap());
                let keytool_path = Path::new(&env::var("JAVA_HOME").expect("please set JAVA_HOME")).join("bin/keytool");
                let mut keytool = Command::new(&keytool_path);
                keytool
                    .arg("-genkey")
                    .arg("-v")
                    .arg("-keystore")
                    .arg(&keystore)
                    .arg("-storepass")
                    .arg(&keystore_pass)
                    .arg("-alias")
                    .arg(&key_alias)
                    .arg("-keypass")
                    .arg(&key_pass)
                    .arg("-keyalg")
                    .arg("RSA")
                    .arg("-keysize")
                    .arg("2048")
                    .arg("-validity")
                    .arg("10000")
                    .arg("-dname")
                    .arg("CN=Android Debug,O=Android,C=US")
                    .arg("-storetype")
                    .arg("pkcs12");

                match keytool.status() {
                    Ok(s) => {
                        if !s.success() {
                            fatal!("keytool failed generating debug keys");
                        }
                    }
                    Err(e) => fatal!("cannot run '{}', {e}", keytool_path.display()),
                }
            }
        }

        #[cfg(not(windows))]
        const APKSIGNER_NAME: &str = "apksigner";
        #[cfg(windows)]
        const APKSIGNER_NAME: &str = "apksigner.bat";

        let apksigner_path = build_tools.join(APKSIGNER_NAME);
        let mut apksigner = Command::new(&apksigner_path);
        apksigner
            .arg("sign")
            .arg("--ks")
            .arg(keystore)
            .arg("--ks-pass")
            .arg(format!("pass:{keystore_pass}"))
            .arg("--ks-key-alias")
            .arg(key_alias)
            .arg("--key-pass")
            .arg(format!("pass:{key_pass}"))
            .arg("--out")
            .arg(&signed_apk_path)
            .arg(&aligned_apk_path);

        match apksigner.status() {
            Ok(s) => {
                if !s.success() {
                    fatal!("apksigner failed")
                }
            }
            Err(e) => fatal!("cannot run '{}', {e}", apksigner_path.display()),
        }
        signed_apk_path
    };

    // finalize
    fs::remove_dir_all(&apk_folder).unwrap_or_else(|e| fatal!("apk folder cleanup failed, {e}"));
    fs::rename(final_apk, output_file).unwrap_or_else(|e| fatal!("cannot copy built apk to final place, {e}"));
    fs::remove_dir_all(&temp_dir).unwrap_or_else(|e| fatal!("temp dir cleanup failed, {e}"));
    let _ = fs::remove_file(path(ZR_TARGET));
}
#[derive(serde::Deserialize)]
#[serde(rename = "manifest")]
struct AndroidManifest {
    #[serde(rename = "uses-sdk")]
    #[serde(default)]
    pub uses_sdk: AndroidSdk,
}
#[derive(Default, serde::Deserialize)]
#[serde(rename = "uses-sdk")]
struct AndroidSdk {
    #[serde(rename(serialize = "android:minSdkVersion"))]
    pub min_sdk_version: Option<u32>,
    #[serde(rename(serialize = "android:targetSdkVersion"))]
    pub target_sdk_version: Option<u32>,
    #[serde(rename(serialize = "android:maxSdkVersion"))]
    pub max_sdk_version: Option<u32>,
}
impl AndroidSdk {
    pub fn matches(&self, version: u32) -> bool {
        if let Some(v) = self.target_sdk_version {
            return v == version;
        }
        if let Some(m) = self.min_sdk_version
            && version < m
        {
            return false;
        }
        if let Some(m) = self.max_sdk_version
            && version > m
        {
            return false;
        }
        true
    }
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

fn copy_dir_all(from: &Path, to: &Path, trace: bool) {
    for entry in walkdir::WalkDir::new(from).min_depth(1).max_depth(1).sort_by_file_name() {
        let entry = entry.unwrap_or_else(|e| fatal!("cannot walkdir entry `{}`, {e}", from.display()));
        let from = entry.path();
        let to = to.join(entry.file_name());
        if entry.file_type().is_dir() {
            fs::create_dir(&to).unwrap_or_else(|e| {
                if e.kind() != io::ErrorKind::AlreadyExists {
                    fatal!("cannot create_dir `{}`, {e}", to.display())
                }
            });
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
    ($($tool:tt),+ $(,)?) => {
        pub static BUILT_INS: &[&str] = &[
            $(stringify!($tool),)+
        ];
        static BUILT_IN_FNS: &[fn()] = &[
            $($tool,)+
        ];
    };
}
built_in! { copy, glob, rp, sh, shf, warn, fail, apk }

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
        unsafe {
            // SAFETY: potentially not safe as tests run in parallel and I don't want to audit every C dep
            // of code that runs in other tests. If a segfault happen during test run caused by this I intend
            // to print the test runner log and frame it.
            std::env::set_var("ZR_RP_TEST", "test value");
        }

        assert_eq!("", replace("", 0).unwrap());
        assert_eq!("normal text", replace("normal text", 0).unwrap());
        assert_eq!("escaped ${NOT}", replace("escaped $${NOT}", 0).unwrap());
        assert_eq!("replace 'test value'", replace("replace '${ZR_RP_TEST}'", 0).unwrap());
        assert_eq!("${} cannot be read or is empty", replace("empty '${}'", 0).unwrap_err()); // hmm
        assert_eq!(
            "${ZR_RP_TEST_NOT_SET} cannot be read or is empty",
            replace("not set '${ZR_RP_TEST_NOT_SET}'", 0).unwrap_err()
        );
        assert_eq!(
            "not set 'fallback!'",
            replace("not set '${ZR_RP_TEST_NOT_SET:?fallback!}'", 0).unwrap()
        );
        assert_eq!(
            "not set 'nested 'test value'.'",
            replace("not set '${ZR_RP_TEST_NOT_SET:?nested '${ZR_RP_TEST}'.}'", 0).unwrap()
        );
        assert_eq!("test value", replace("${ZR_RP_TEST_NOT_SET:?${ZR_RP_TEST}}", 0).unwrap());
        assert_eq!(
            "curly test value",
            replace("curly ${ZR_RP_TEST:?{not {what} {is} {going {on {here {:?}}}}}}", 0).unwrap()
        );

        assert_eq!("replace not closed at: ${MISSING", replace("${MISSING", 0).unwrap_err());
        assert_eq!("replace not closed at: ${MIS", replace("${MIS", 0).unwrap_err());
        assert_eq!("replace not closed at: ${MIS:?{", replace("${MIS:?{", 0).unwrap_err());
        assert_eq!("replace not closed at: ${MIS:?{}", replace("${MIS:?{}", 0).unwrap_err());

        assert_eq!("TEST VALUE", replace("${ZR_RP_TEST:U}", 0).unwrap());
        assert_eq!("TEST-VALUE", replace("${ZR_RP_TEST:K}", 0).unwrap());
        assert_eq!("TEST_VALUE", replace("${ZR_RP_TEST:S}", 0).unwrap());
        assert_eq!("testValue", replace("${ZR_RP_TEST:c}", 0).unwrap());
    }

    #[test]
    fn replace_cmd_case() {
        assert_eq!("cmd HELLO:?WORLD", replace("cmd ${!printf \"hello:?world\":U}", 0).unwrap(),)
    }
}
