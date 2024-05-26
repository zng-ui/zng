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

fn cargo_res(test: &str, pack: bool) {
    let source = PathBuf::from("cargo-zng-res-tests").join(test).join("source");
    assert!(source.exists());
    let target = PathBuf::from("../target/tmp/tests/cargo_zng");
    if target.exists() {
        let _ = fs::remove_dir_all(&target);
    }

    let output = cargo_zng_res(&[&source, &target], pack).unwrap_or_else(|e| panic!("{e}"));

    fs::write(source.join("run.stdout"), output.as_bytes()).unwrap();
    assert_dir_eq(&source.with_file_name("expected_target"), &target);
}

fn assert_dir_eq(expected: &Path, actual: &Path) {
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
            assert_dir_eq(&expected, &actual);
        }
    }
}

fn cargo_zng_res<S: AsRef<OsStr>>(args: &[S], pack: bool) -> io::Result<String> {
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("run").arg("-p").arg("cargo-zng").arg("--").arg("res");
    if pack {
        cmd.arg("--pack");
    }
    let output = cmd.args(args).output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(io::Error::new(io::ErrorKind::Other, err))
    }
}
