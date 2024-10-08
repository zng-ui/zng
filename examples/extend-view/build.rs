fn main() {
    #[cfg(windows)]
    extract_angle_libs();
}

/// To enable ANGLE on Windows the `libEGL.dll` and `libGLESv2.dll` must be present and
/// the preference for ANGLE set on the view-process.
///
/// This method extracts bundled prebuilt libraries (taken from the Electron project).
///
/// See `src/prefer_angle.rs` for how to set the ANGLE preference.
#[cfg(windows)]
fn extract_angle_libs() {
    use std::env;

    println!("cargo:rerun-if-changed=res/angle.tar.gz");

    let target = {
        let out_dir = std::path::PathBuf::from(env::var("OUT_DIR").unwrap());
        let profile = env::var("PROFILE").unwrap();
        let mut target_dir = None;
        let mut sub_path = out_dir.as_path();
        while let Some(parent) = sub_path.parent() {
            if parent.ends_with(&profile) {
                target_dir = Some(parent);
                break;
            }
            sub_path = parent;
        }
        target_dir.expect("target not found").to_owned()
    };

    let r = std::process::Command::new("tar")
        .arg("-xf")
        .arg("res/angle.tar.gz")
        .arg("-C")
        .arg(&target)
        .status();

    match r {
        Ok(s) => {
            if s.success() {
                let egl_dll = target.join("libEGL.dll");
                let gles_dll = target.join("libGLESv2.dll");

                println!("cargo:rerun-if-changed={}", egl_dll.display());
                println!("cargo:rerun-if-changed={}", gles_dll.display());

                if !egl_dll.exists() || !gles_dll.exists() {
                    println!("cargo:warning=failed extract angle DLLs, not found");
                }
            } else {
                println!("cargo:warning=failed extract angle DLLs, tar exit code: {:?}", s.code());
            }
        }
        Err(e) => {
            println!("cargo:warning=failed extract angle DLLs, {e}");
        }
    }
}
