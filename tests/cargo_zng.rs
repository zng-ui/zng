use std::{
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};

#[test]
fn res_statics_no_pack() {
    res("statics", Pack::No, Expect::Ok);
}

#[test]
fn res_statics_pack() {
    res("statics-pack", Pack::Yes, Expect::Ok);
}

#[test]
fn res_copy() {
    res("copy", Pack::No, Expect::Ok);
}

#[test]
fn res_recursive() {
    res("recursive", Pack::No, Expect::Ok);
}

#[test]
fn res_bash() {
    res("bash", Pack::No, Expect::Ok);
}

#[test]
fn res_custom() {
    res("custom", Pack::No, Expect::Ok);
}

#[test]
fn res_glob() {
    res("glob", Pack::No, Expect::Ok);
}

#[test]
fn res_replace() {
    res("replace", Pack::No, Expect::Ok);
}

#[test]
fn res_error_bash() {
    res("bash-error", Pack::No, Expect::Err);
}

#[test]
fn new_basic() {
    new("basic", &["The App!", r#"-s"org=The Org!""#, r#"-s"qualifier=.qual""#], Expect::Ok);
}

#[test]
fn new_post() {
    new("post", &["The App"], Expect::Ok);
}

#[test]
fn new_sanitize() {
    new("sanitize", &["The App /?/"], Expect::Ok);
}

fn new(test: &str, keys: &[&str], expect: Expect) {
    let tests_dir = PathBuf::from("./cargo-zng-new-tests");
    let test_dir = tests_dir.join(test);

    // cannot run tests inside the workspace (nested .git, cargo warnings)
    let temp = std::env::temp_dir().join("cargo-zng-new-tests").join(test);
    fs::create_dir_all(&temp).unwrap();

    let source = test_dir.join("template");
    assert!(source.exists());
    let expected_target = source.with_file_name("expected_target");

    let temp_source = temp.join("template");
    let _ = fs::remove_dir_all(&temp_source);
    fs::create_dir(&temp_source).unwrap();
    copy_dir_all(&source, &temp_source);
    let source = temp_source;
    git_init(&source).unwrap();

    let target = source.with_file_name("target");
    if target.exists() {
        let _ = fs::remove_dir_all(&target);
    }
    fs::create_dir_all(&target).unwrap();

    let error;
    let stdio;
    match zng_new(keys, &target, &source) {
        Ok(s) => {
            error = None;
            stdio = s;
        }
        Err((e, s)) => {
            error = Some(e);
            stdio = s;
        }
    }

    let _ = fs::remove_dir_all(&source);

    let target_git = target.join("the-app/.git");
    assert!(target_git.exists(), "git not inited on target {}", target_git.display());
    fs::remove_dir_all(target_git).unwrap();

    verify_output(&test_dir, &stdio, error, expect, &expected_target, &target)
}

fn res(test: &str, pack: Pack, expect: Expect) {
    let tests_dir = PathBuf::from("cargo-zng-res-tests");
    let test_dir = tests_dir.join(test);
    let source = test_dir.join("source");
    assert!(source.exists());
    let target = PathBuf::from("../target/tmp/tests/zng_res").join(test);
    if target.exists() {
        let _ = fs::remove_dir_all(&target);
    }
    fs::create_dir_all(&target).unwrap();
    let tool_dir = test_dir.join("tools");
    let metadata = tests_dir.join("metadata.toml");

    let error;
    let stdio;
    match zng_res(&[&source, &target], &tool_dir, &metadata, matches!(pack, Pack::Yes)) {
        Ok(s) => {
            error = None;
            stdio = s;
        }
        Err((e, s)) => {
            error = Some(e);
            stdio = s;
        }
    }
    verify_output(&test_dir, &stdio, error, expect, &source.with_file_name("expected_target"), &target);
}

fn verify_output(test_dir: &Path, stdio: &StdioStr, error: Option<io::Error>, expect: Expect, expected_target: &Path, target: &Path) {
    let stdout_file = test_dir.join("test.stdout");
    let stderr_file = test_dir.join("test.stderr");
    let existing_stdout = fs::read_to_string(&stdout_file).unwrap_or_default();
    let existing_stderr = fs::read_to_string(&stderr_file).unwrap_or_default();

    fs::write(&stdout_file, stdio.stdout.as_bytes()).unwrap();
    fs::write(&stderr_file, stdio.stderr.as_bytes()).unwrap();

    match error {
        Some(e) => {
            assert_eq!(
                expect,
                Expect::Err,
                "\n--stdout--\n{}\n--stderr--{}\n--error--\n{e}",
                stdio.stdout,
                stdio.stderr
            )
        }
        None => assert_eq!(expect, Expect::Ok),
    }

    pretty_assertions::assert_eq!(existing_stdout, stdio.stdout, "{} changed", stdout_file.display());
    pretty_assertions::assert_eq!(existing_stderr, stdio.stderr, "{} changed", stderr_file.display());

    if matches!(expect, Expect::Ok) {
        let generate = match fs::read_dir(expected_target) {
            Ok(mut d) => d.next().is_none(),
            Err(e) => {
                if e.kind() == io::ErrorKind::NotFound {
                    true
                } else {
                    panic!("cannot read expected_target, {e}")
                }
            }
        };

        if generate {
            let _ = fs::remove_dir_all(expected_target);
            fs::rename(target, expected_target).expect("cannot generate expected_target");
        } else {
            assert_dir_eq(expected_target, target);
        }
    }
}

fn assert_dir_eq(expected: &Path, actual: &Path) {
    check_actual_has(expected, actual);
    check_expected_has(actual, expected);

    fn check_actual_has(expected: &Path, actual: &Path) {
        for entry in fs::read_dir(expected).unwrap() {
            let expected = entry.unwrap().path();
            if expected.file_name().unwrap().to_string_lossy() == ".empty" {
                assert!(
                    fs::read_dir(actual).unwrap().next().is_none(),
                    "expected empty `{}`",
                    actual.display()
                );
                continue;
            }
            let actual = actual.join(expected.file_name().unwrap());
            assert!(actual.exists(), "expected `{}`", actual.display());
            if expected.is_dir() {
                check_actual_has(&expected, &actual);
            } else {
                let expected = fs::read_to_string(expected).unwrap().replace("\r\n", "\n");
                let actual = fs::read_to_string(actual).unwrap().replace("\r\n", "\n");
                // let expected = format!("{expected:?}");
                // let actual = format!("{actual:?}");
                pretty_assertions::assert_eq!(expected, actual, "expected file contents to match");
            }
        }
    }

    fn check_expected_has(actual: &Path, expected: &Path) {
        for entry in fs::read_dir(actual).unwrap() {
            let actual = entry.unwrap().path();
            let expected = expected.join(actual.file_name().unwrap());
            assert!(expected.exists(), "did not expect `{}`", actual.display());
            if actual.is_dir() {
                check_expected_has(&actual, &expected);
            }
        }
    }
}

fn rust_flags_allow_warnings() -> String {
    let mut flags = std::env::var("RUSTFLAGS").unwrap_or_default();
    for f in ["-Dwarnings", "--deny=warnings", "-D warnings", "--deny warnings"] {
        flags = flags.replace(f, "");
    }
    flags
}

fn zng_res<S: AsRef<OsStr>>(args: &[S], tool_dir: &Path, metadata: &Path, pack: bool) -> Result<StdioStr, (io::Error, StdioStr)> {
    zng(
        |cmd| {
            cmd.env("RUSTFLAGS", rust_flags_allow_warnings());
            cmd.arg("res");
            if pack {
                cmd.arg("--pack");
            }
            cmd.arg("--tool-dir").arg(tool_dir);
            cmd.arg("--metadata").arg(metadata);
            cmd.args(args)
        },
        |s| {
            let mut clean = String::new();
            for line in s.stdout.lines() {
                if line.contains("Finished") && line.contains("res build") {
                    let i = line.find("res build").unwrap() + "res build".len();
                    clean.push_str(&line[..i]);
                    clean.push_str(" in #ms\n         #DIR#\n");
                    break;
                } else {
                    clean.push_str(line);
                    clean.push('\n');
                }
            }
            s.stdout = clean;
        },
    )
}

fn zng_new<S: AsRef<OsStr>>(args: &[S], target: &Path, template: &Path) -> Result<StdioStr, (io::Error, StdioStr)> {
    zng(
        |cmd| cmd.current_dir(target).arg("new").arg("--template").arg(template).args(args),
        |s| {
            let mut clean = String::new();
            for line in s.stdout.lines() {
                let line = line.replace('\\', "/");
                if line.ends_with(".sh") {
                    let i = line.rfind(".zng-template").unwrap();
                    clean.push_str("#TEMP#/");
                    clean.push_str(&line[i..]);
                } else {
                    clean.push_str(&line);
                }
                clean.push('\n');
            }
            s.stdout = clean;
        },
    )
}

fn zng(setup: impl FnOnce(&mut Command) -> &mut Command, cleanup: impl FnOnce(&mut StdioStr)) -> Result<StdioStr, (io::Error, StdioStr)> {
    assert!(
        Command::new("cargo")
            .arg("build")
            .arg("-p")
            .arg("cargo-zng")
            .output()
            .expect("cannot build cargo-zng")
            .status
            .success()
    );

    let cargo_zng = dunce::canonicalize(Path::new("../target/debug").join(format!("cargo-zng{}", std::env::consts::EXE_SUFFIX))).unwrap();

    let mut cmd = Command::new(cargo_zng);
    cmd.arg("zng");
    setup(&mut cmd);
    let output = cmd.output().map_err(|e| (e, StdioStr::default()))?;
    let mut stdio = StdioStr::from(&output);
    cleanup(&mut stdio);
    if output.status.success() {
        Ok(stdio)
    } else {
        Err((io::Error::other(format!("error code {}", output.status.code().unwrap_or(0))), stdio))
    }
}

#[derive(Default, Debug)]
struct StdioStr {
    stdout: String,
    stderr: String,
}

impl From<&std::process::Output> for StdioStr {
    fn from(output: &std::process::Output) -> Self {
        Self {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }
}

enum Pack {
    Yes,
    No,
}

#[derive(PartialEq, Debug)]
enum Expect {
    Ok,
    Err,
}

fn copy_dir_all(from: &Path, to: &Path) {
    for entry in walkdir::WalkDir::new(from).min_depth(1).max_depth(1).sort_by_file_name() {
        let entry = entry.unwrap_or_else(|e| panic!("cannot walkdir entry `{}`, {e}", from.display()));
        let from = entry.path();
        let to = to.join(entry.file_name());
        if entry.file_type().is_dir() {
            fs::create_dir(&to).unwrap_or_else(|e| {
                if e.kind() != io::ErrorKind::AlreadyExists {
                    panic!("cannot create_dir `{}`, {e}", to.display())
                }
            });
            copy_dir_all(from, &to);
        } else if entry.file_type().is_file() {
            fs::copy(from, &to).unwrap_or_else(|e| panic!("cannot copy `{}` to `{}`, {e}", from.display(), to.display()));
        }
    }
}

fn git_init(dir: &Path) -> io::Result<()> {
    let mut init = Command::new("git");
    init.arg("init");
    let mut add = Command::new("git");
    add.arg("add").arg(".");
    let mut commit = Command::new("git");
    commit
        .arg("commit")
        .arg("-m")
        .arg("test")
        .env("GIT_AUTHOR_NAME", "cargo_zng::git_init")
        .env("GIT_AUTHOR_EMAIL", "test@test.com")
        .env("GIT_COMMITTER_NAME", "cargo_zng::git_init")
        .env("GIT_COMMITTER_EMAIL", "test@test.com");
    for mut cmd in [init, add, commit] {
        match cmd.current_dir(dir).output() {
            Ok(s) => {
                if !s.status.success() {
                    let stdout = String::from_utf8_lossy(&s.stdout);
                    let stderr = String::from_utf8_lossy(&s.stderr);
                    return Err(io::Error::other(format!(
                        "git exited with {}\n--stdout--\n{stdout}\n--stderr--\n{stderr}",
                        s.status.code().unwrap_or(0)
                    )));
                }
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
