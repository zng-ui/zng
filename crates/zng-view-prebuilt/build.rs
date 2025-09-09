use std::{
    env,
    path::{Path, PathBuf},
};

use sha2::Digest;

fn main() {
    println!("cargo::rustc-check-cfg=cfg(zng_lib_embedded)");

    if !cfg!(feature = "embedded") {
        return;
    }

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_dir = Path::new(&manifest_dir);

    let mut lib = manifest_dir.join("lib");
    println!("cargo:rerun-if-changed={}", lib.display());

    // rerun-if-changed disables the default check
    //
    // if version updates
    println!("cargo:rerun-if-changed={}", manifest_dir.join("Cargo.toml").display());

    #[allow(unused_variables)]
    let file = "";

    let file = format!(
        "{}zng_view.{}{}",
        std::env::consts::DLL_PREFIX,
        std::env::var("TARGET").unwrap(),
        std::env::consts::DLL_SUFFIX
    );

    lib = lib.join(&file);

    let is_docs_rs = env::var("DOCS_RS").is_ok();

    if !lib.exists() && !is_docs_rs {
        let version = env::var("CARGO_PKG_VERSION").unwrap();
        let out_dir = Path::new(&env::var("OUT_DIR").unwrap()).join(format!("v{version}"));
        lib = out_dir.join(&file);
        if !lib.exists() {
            let lib_tar = out_dir.join(format!("{file}.tar.gz"));

            #[cfg(target_os = "macos")]
            if macos_major_version() < 11 {
                panic!("prebuilt download is only supported on macOS 11 or newer");
            }

            let url = format!("https://github.com/zng-ui/zng/releases/download/v{version}/{file}.tar.gz");
            let r = std::process::Command::new("curl")
                .arg("--location")
                .arg("--fail")
                .arg("--silent")
                .arg("--show-error")
                .arg("--create-dirs")
                .arg("--output")
                .arg(&lib_tar)
                .arg(&url)
                .status();
            match r {
                Ok(s) => {
                    if s.success() {
                        let r = std::process::Command::new("tar")
                            .arg("-xf")
                            .arg(lib_tar)
                            .arg("-C")
                            .arg(&out_dir)
                            .status();

                        match r {
                            Ok(s) => {
                                if s.success() {
                                    if !lib.exists() {
                                        println!("cargo:warning=view prebuilt not embedded, unexpected missing {}", lib.display());
                                    }
                                } else {
                                    println!(
                                        "cargo:warning=view prebuilt not embedded, failed extract, tar exit code: {:?}",
                                        s.code()
                                    );
                                }
                            }
                            Err(e) => {
                                println!("cargo:warning=view prebuilt not embedded, failed extract, {e}");
                            }
                        }
                    } else {
                        println!(
                            "cargo:warning=view prebuilt not embedded, failed download, curl exit code: {:?}",
                            s.code()
                        );
                    }
                }
                Err(e) => {
                    println!("cargo:warning=view prebuilt not embedded, failed download, {e}");
                }
            }
        }
    }

    if lib.exists() {
        println!("cargo:rustc-cfg=zng_lib_embedded");
        println!("cargo:rustc-env=ZNG_VIEW_LIB={}", dunce::canonicalize(&lib).unwrap().display());

        let lib_bytes = std::fs::read(lib).unwrap();

        // just to identify build.
        let mut hasher = sha2::Sha256::new();
        hasher.update(&lib_bytes);
        let hash = hasher.finalize();
        println!("cargo:rustc-env=ZNG_VIEW_LIB_HASH={hash:x}",);
    } else if is_docs_rs || PathBuf::from("../../tools/cargo-do").exists() {
        println!("cargo:warning=view prebuilt not embedded, missing '{file}', call `do prebuild`");
    } else {
        // exit with error also flags rerun
        panic!("view prebuilt not embedded, missing '{file}'");
    }
}

#[cfg(target_os = "macos")]
fn macos_major_version() -> u32 {
    let output = match std::process::Command::new("sw_vers").arg("-productVersion").output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("cannot retrieve macos version, {e}");
            return 0;
        }
    };

    if output.status.success() {
        let ver = String::from_utf8_lossy(&output.stdout);
        match ver.trim().split('.').next().unwrap_or("").parse() {
            Ok(v) => v,
            Err(_) => {
                eprintln!("cannot parse macos version {ver:?}");
                0
            }
        }
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        eprintln!("cannot retrieve macos version, {}", err.trim());
        0
    }
}
