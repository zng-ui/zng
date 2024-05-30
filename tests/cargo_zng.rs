use std::{
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
};

#[test]
fn cargo_res_statics_no_pack() {
    cargo_res("statics", false);
}

#[test]
fn cargo_res_statics_pack() {
    cargo_res("statics-pack", true);
}

#[test]
fn cargo_res_copy() {
    cargo_res("copy", false);
}

#[test]
fn cargo_res_recursive() {
    cargo_res("recursive", false);
}

#[test]
fn cargo_res_bash() {
    cargo_res("bash", false);
}

#[test]
fn cargo_res_custom() {
    cargo_res("custom", false);
}

#[test]
fn cargo_res_glob() {
    cargo_res("glob", false);
}

#[test]
fn cargo_res_replace() {
    cargo_res("replace", false);
}

fn cargo_res(test: &str, pack: bool) {
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

    let (stdout, stderr) = cargo_zng_res(&[&source, &target], &tool_dir, &metadata, pack).unwrap_or_else(|e| panic!("{e}"));
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
        } else if line.trim_start().starts_with("Running") {
            copy = true;
        }
    }

    let stdout = test_dir.join("test.stdout");
    let stderr = test_dir.join("test.stderr");
    let existing_stdout = fs::read_to_string(&stdout).unwrap_or_default();
    let existing_stderr = fs::read_to_string(&stderr).unwrap_or_default();

    fs::write(&stdout, clean_stdout.as_bytes()).unwrap();
    fs::write(&stderr, clean_stderr.as_bytes()).unwrap();

    pretty_assertions::assert_eq!(existing_stdout, clean_stdout, "{} changed", stdout.display());
    pretty_assertions::assert_eq!(existing_stderr, clean_stderr, "{} changed", stderr.display());

    assert_dir_eq(&source.with_file_name("expected_target"), &target);
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
                assert_eq!(expected, actual, "expected file contents to match");
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

fn cargo_zng_res<S: AsRef<OsStr>>(args: &[S], tool_dir: &Path, metadata: &Path, pack: bool) -> io::Result<(String, String)> {
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("run").arg("-p").arg("cargo-zng").arg("--").arg("res");
    if pack {
        cmd.arg("--pack");
    }
    cmd.arg("--tool-dir").arg(tool_dir);
    cmd.arg("--metadata").arg(metadata);
    let output = cmd.args(args).output()?;
    if output.status.success() {
        Ok((
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ))
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(io::Error::new(io::ErrorKind::Other, err))
    }
}
