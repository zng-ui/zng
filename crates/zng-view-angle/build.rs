use std::{env, fs, path::PathBuf};

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    if target_os != "windows" || !cfg!(feature = "download") {
        return;
    }

    // target/<profile>/build/zng-view-angle*/build-script-build
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    // ../../..
    let mut target_dir = out_dir.parent().unwrap().parent().unwrap().parent().unwrap();
    // cross compilation (target/<platform>/<profile>/build/zng-view-angle*/build-script-build)
    if target_dir.ends_with("build") {
        // ..
        target_dir = target_dir.parent().unwrap();
    }

    // DLLs
    const DLLS: &[&str] = &["libEGL.dll", "libGLESv2.dll"];
    let dlls: Vec<_> = DLLS.iter().map(|d| target_dir.join(d)).collect();
    if dlls.iter().all(|p| p.exists()) {
        return;
    }

    // download cache (target/tmp/zng-view-angle)
    let tmp_dir = target_dir.parent().unwrap().join("tmp/zng-view-angle");
    let tmp_dlls: Vec<_> = DLLS.iter().map(|d| tmp_dir.join(d)).collect();
    if tmp_dlls.iter().any(|p| !p.exists()) {
        let arch = match env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
            "x86_64" => "x64",
            "aarch64" => "arm64",
            a => {
                return println!("cargo:warning=cannot download angle dlls for arch {a}, unsupported");
            }
        };

        // curl
        let out_tar_gz = tmp_dir.join("out.tar.gz");
        let url = format!("https://github.com/zng-ui/build-angle/releases/download/2026-05-17/angle-{arch}-2026-05-17.tar.gz");
        let r = std::process::Command::new("curl")
            .arg("--location")
            .arg("--fail")
            .arg("--silent")
            .arg("--show-error")
            .arg("--create-dirs")
            .arg("--output")
            .arg(&out_tar_gz)
            .arg(&url)
            .status();
        match r {
            Ok(s) => {
                if !s.success() {
                    return println!("cargo:warning=cannot download angle dlls, curl exit code {:?}", s.code());
                }
            }
            Err(e) => return println!("cargo:warning=cannot download angle dlls, curl failed {e}"),
        }

        // tar
        let out_dir = tmp_dir.join("out");
        let _ = fs::create_dir_all(&out_dir);
        let r = std::process::Command::new("tar")
            .arg("-xf")
            .arg(&out_tar_gz)
            .arg("-C")
            .arg(&out_dir)
            .status();
        match r {
            Ok(s) => {
                if !s.success() {
                    return println!("cargo:warning=cannot extract angle dlls, tar exit code {:?}", s.code());
                }
            }
            Err(e) => return println!("cargo:warning=cannot extract angle dlls, tar failed {e}"),
        }

        // normalize
        let out_dir_x = out_dir.join(format!("angle-{arch}"));
        for (dll, tmp_dll) in DLLS.iter().zip(&tmp_dlls) {
            if let Err(e) = fs::rename(out_dir_x.join(dll), tmp_dll) {
                return println!("cargo:warning=cannot extract angle dlls, normalize failed {e}");
            }
        }
        let _ = fs::remove_file(&out_tar_gz);
    }

    // copy from download cache
    for (tmp_dll, dll) in tmp_dlls.into_iter().zip(dlls) {
        if let Err(e) = fs::copy(tmp_dll, dll) {
            return println!("cargo:warning=cannot copy angle dlls to target, {e}");
        }
    }
}
