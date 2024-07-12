use std::{
    env, fs,
    hash::Hasher,
    path::{Path, PathBuf},
};

use base64::Engine;
use hashers::jenkins::spooky_hash::SpookyHasher;

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
    // in case of intermittent download error
    let rerun_request = Path::new(&env::var("OUT_DIR").unwrap()).join("rerun");
    println!("cargo:rerun-if-changed={}", rerun_request.display());

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

    let mut rerun = false;

    if !lib.exists() && !is_docs_rs {
        let version = env::var("CARGO_PKG_VERSION").unwrap();
        let out_dir = Path::new(&env::var("OUT_DIR").unwrap()).join(format!("v{version}"));
        lib = out_dir.join(&file);
        if !lib.exists() {
            let lib_tar = out_dir.join(format!("{file}.tar.gz"));

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
                                        rerun = true;
                                    }
                                } else {
                                    println!(
                                        "cargo:warning=view prebuilt not embedded, failed extract, tar exit code: {:?}",
                                        s.code()
                                    );
                                    rerun = true;
                                }
                            }
                            Err(e) => {
                                println!("cargo:warning=view prebuilt not embedded, failed extract, {e}");
                                rerun = true;
                            }
                        }
                    } else {
                        println!(
                            "cargo:warning=view prebuilt not embedded, failed download, curl exit code: {:?}",
                            s.code()
                        );
                        rerun = true;
                    }
                }
                Err(e) => {
                    println!("cargo:warning=view prebuilt not embedded, failed download, {e}");
                    rerun = true;
                }
            }
        }
    }

    if lib.exists() {
        println!("cargo:rustc-cfg=zng_lib_embedded");
        println!("cargo:rustc-env=ZNG_VIEW_LIB={}", dunce::canonicalize(&lib).unwrap().display());

        let lib_bytes = std::fs::read(lib).unwrap();

        // just to identify build.
        let mut hasher = SpookyHasher::new(u64::from_le_bytes(*b"prebuild"), u64::from_le_bytes(*b"view-lib"));
        hasher.write(&lib_bytes);
        let (a, b) = hasher.finish128();
        let mut hash = [0; 16];
        hash[..8].copy_from_slice(&a.to_le_bytes());
        hash[8..].copy_from_slice(&b.to_le_bytes());

        println!(
            "cargo:rustc-env=ZNG_VIEW_LIB_HASH={}",
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash)
        );
    } else if is_docs_rs || PathBuf::from("../../tools/cargo-do").exists() {
        println!("cargo:warning=view prebuilt not embedded, missing '{file}', call `do prebuild`");
        if rerun {
            fs::write(rerun_request, "rerun").unwrap();
        }
    } else {
        // exit with error also flags rerun
        panic!("view prebuilt not embedded, missing '{file}'");
    }
}
