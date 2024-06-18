use std::{
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
};

#[test]
fn cargo_res_statics_no_pack() {
    cargo_res("statics", Pack::No, Expect::Ok);
}

#[test]
fn cargo_res_statics_pack() {
    cargo_res("statics-pack", Pack::Yes, Expect::Ok);
}

#[test]
fn cargo_res_copy() {
    cargo_res("copy", Pack::No, Expect::Ok);
}

#[test]
fn cargo_res_recursive() {
    cargo_res("recursive", Pack::No, Expect::Ok);
}

#[test]
fn cargo_res_bash() {
    cargo_res("bash", Pack::No, Expect::Ok);
}

#[test]
fn cargo_res_custom() {
    cargo_res("custom", Pack::No, Expect::Ok);
}

#[test]
fn cargo_res_glob() {
    cargo_res("glob", Pack::No, Expect::Ok);
}

#[test]
fn cargo_res_replace() {
    cargo_res("replace", Pack::No, Expect::Ok);
}

#[test]
fn cargo_res_error_bash() {
    cargo_res("bash-error", Pack::No, Expect::Err);
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

fn cargo_res(test: &str, pack: Pack, expect: Expect) {
    let tests_dir = PathBuf::from("cargo-zng-res-tests");
    let test_dir = tests_dir.join(test);
    let source = test_dir.join("source");
    assert!(source.exists());
    let target = PathBuf::from("../target/tmp/tests/cargo_zng").join(test);
    if target.exists() {
        let _ = fs::remove_dir_all(&target);
    }
    let tool_dir = test_dir.join("tools");
    let metadata = tests_dir.join("metadata.toml");

    let error;
    let stdout;
    let stderr;
    match cargo_zng_res(&[&source, &target], &tool_dir, &metadata, matches!(pack, Pack::Yes)) {
        Ok((so, se)) => {
            error = None;
            stdout = so;
            stderr = se;
        }
        Err((e, so, se)) => {
            error = Some(e);
            stdout = so;
            stderr = se;
        }
    }
    let mut clean_stdout = String::new();
    for line in stdout.lines() {
        if line.contains("Finished") && line.contains("res build in") {
            break;
        }
        clean_stdout.push_str(line);
        clean_stdout.push('\n');
    }
    let mut clean_stderr = String::new();
    let mut copy = false;
    for line in stderr.lines() {
        if copy {
            clean_stderr.push_str(line);
            clean_stderr.push('\n');
        } else if line.contains("Running") {
            copy = true;
        }
    }

    let stdout = test_dir.join("test.stdout");
    let stderr = test_dir.join("test.stderr");
    let existing_stdout = fs::read_to_string(&stdout).unwrap_or_default();
    let existing_stderr = fs::read_to_string(&stderr).unwrap_or_default();

    fs::write(&stdout, clean_stdout.as_bytes()).unwrap();
    fs::write(&stderr, clean_stderr.as_bytes()).unwrap();

    match error {
        Some(e) => {
            assert_eq!(expect, Expect::Err, "{e}")
        }
        None => assert_eq!(expect, Expect::Ok),
    }

    pretty_assertions::assert_eq!(existing_stdout, clean_stdout, "{} changed", stdout.display());
    pretty_assertions::assert_eq!(existing_stderr, clean_stderr, "{} changed", stderr.display());

    if matches!(expect, Expect::Ok) {
        assert_dir_eq(&source.with_file_name("expected_target"), &target);
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

fn cargo_zng_res<S: AsRef<OsStr>>(
    args: &[S],
    tool_dir: &Path,
    metadata: &Path,
    pack: bool,
) -> Result<(String, String), (io::Error, String, String)> {
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("run").arg("-p").arg("cargo-zng").arg("--").arg("zng").arg("res");
    if pack {
        cmd.arg("--pack");
    }
    cmd.arg("--tool-dir").arg(tool_dir);
    cmd.arg("--metadata").arg(metadata);
    let output = cmd.args(args).output().map_err(|e| (e, String::new(), String::new()))?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if output.status.success() {
        Ok((stdout, stderr))
    } else {
        Err((
            io::Error::new(io::ErrorKind::Other, format!("error code {}", output.status.code().unwrap_or(0))),
            stdout,
            stderr,
        ))
    }
}
