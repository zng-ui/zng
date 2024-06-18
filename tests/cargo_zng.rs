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
    new("basic", &["The App"], Expect::Ok);
}

fn new(test: &str, keys: &[&str], expect: Expect) {
    let tests_dir = PathBuf::from("cargo-zng-new-tests");
    let test_dir = tests_dir.join(test);
    let source = test_dir.join("source");
    assert!(source.exists());
    let target = PathBuf::from("../target/tmp/tests/zng_new").join(test);
    if target.exists() {
        let _ = fs::remove_dir_all(&target);
    }

    let error;
    let stdio;
    match zng_new(keys, &source) {
        Ok(s) => {
            error = None;
            stdio = s;
        }
        Err((e, s)) => {
            error = Some(e);
            stdio = s;
        }
    }

    verify_output(&test_dir, &stdio.stdout, &stdio.stderr, error, expect, &source, &target)
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
    let mut clean_stdout = String::new();
    for line in stdio.stdout.lines() {
        if line.contains("Finished") && line.contains("res build in") {
            break;
        }
        clean_stdout.push_str(line);
        clean_stdout.push('\n');
    }
    let mut clean_stderr = String::new();
    let mut copy = false;
    for line in stdio.stderr.lines() {
        if copy {
            clean_stderr.push_str(line);
            clean_stderr.push('\n');
        } else if line.contains("Running") {
            copy = true;
        }
    }

    verify_output(&test_dir, &clean_stdout, &clean_stderr, error, expect, &source, &target);
}

fn verify_output(test_dir: &Path, stdout: &str, stderr: &str, error: Option<io::Error>, expect: Expect, source: &Path, target: &Path) {
    let stdout_file = test_dir.join("test.stdout");
    let stderr_file = test_dir.join("test.stderr");
    let existing_stdout = fs::read_to_string(&stdout_file).unwrap_or_default();
    let existing_stderr = fs::read_to_string(&stderr_file).unwrap_or_default();

    fs::write(&stdout_file, stdout.as_bytes()).unwrap();
    fs::write(&stderr_file, stderr.as_bytes()).unwrap();

    match error {
        Some(e) => {
            assert_eq!(expect, Expect::Err, "{e}")
        }
        None => assert_eq!(expect, Expect::Ok),
    }

    pretty_assertions::assert_eq!(existing_stdout, stdout, "{} changed", stdout_file.display());
    pretty_assertions::assert_eq!(existing_stderr, stderr, "{} changed", stderr_file.display());

    if matches!(expect, Expect::Ok) {
        let expected_target = source.with_file_name("expected_target");
        let generate = match fs::read_dir(&expected_target) {
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
            let _ = fs::remove_dir_all(&expected_target);
            fs::rename(target, &expected_target).expect("cannot generate expected_target");
        } else {
            assert_dir_eq(&expected_target, target);
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
                let expected = fs::read_to_string(expected).unwrap();
                let actual = fs::read_to_string(actual).unwrap();
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

fn zng_res<S: AsRef<OsStr>>(args: &[S], tool_dir: &Path, metadata: &Path, pack: bool) -> Result<StdioStr, (io::Error, StdioStr)> {
    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("-p").arg("cargo-zng").arg("--").arg("zng").arg("res");
    if pack {
        cmd.arg("--pack");
    }
    cmd.arg("--tool-dir").arg(tool_dir);
    cmd.arg("--metadata").arg(metadata);
    zng(cmd, args)
}

fn zng_new<S: AsRef<OsStr>>(args: &[S], template: &Path) -> Result<StdioStr, (io::Error, StdioStr)> {
    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("-p").arg("cargo-zng").arg("--").arg("zng").arg("new");
    cmd.arg("--template").arg(template);
    zng(cmd, args)
}

fn zng<S: AsRef<OsStr>>(mut cmd: Command, args: &[S]) -> Result<StdioStr, (io::Error, StdioStr)> {
    let output = cmd.args(args).output().map_err(|e| (e, StdioStr::default()))?;
    let stdio = StdioStr::from(&output);
    if output.status.success() {
        Ok(stdio)
    } else {
        Err((
            io::Error::new(io::ErrorKind::Other, format!("error code {}", output.status.code().unwrap_or(0))),
            stdio,
        ))
    }
}

#[derive(Default)]
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
