use std::{
    env,
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

    let mut lib = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("lib");

    println!("cargo:rerun-if-changed={}", lib.display());

    #[allow(unused_variables)]
    let file = "";

    #[cfg(target_os = "windows")]
    let file = "zng_view.dll";

    #[cfg(target_os = "linux")]
    let file = "libzng_view.so";

    #[cfg(target_os = "macos")]
    let file = "libzng_view.dylib";

    if file.is_empty() {
        println!("cargo:warning=view prebuilt not embedded, unsupported os");
        return;
    }

    lib = lib.join(file);

    let is_docs_rs = env::var("DOCS_RS").is_ok();

    if !lib.exists() {
        let version = env::var("CARGO_PKG_VERSION").unwrap();
        lib = home::cargo_home()
            .unwrap()
            .join(".zng-view-prebuilt")
            .join(format!("{file}.{version}.bin"));
        if !is_docs_rs && !lib.exists() {
            #[cfg(target_os = "windows")]
            let download_file = "prebuilt-windows.tar.gz";

            #[cfg(target_os = "linux")]
            let download_file = "prebuilt-ubuntu.tar.gz";

            #[cfg(target_os = "macos")]
            let download_file = "prebuilt-macos.tar.gz";

            let url = format!("https://github.com/zng-ui/zng/releases/download/v{version}/{download_file}");

            let out_dir = env::var("OUT_DIR").unwrap();
            let output = Path::new(&out_dir).join(download_file);

            let r = std::process::Command::new("curl")
                .arg("--location")
                .arg("--fail")
                .arg("--silent")
                .arg("--show-error")
                .arg("--create-dirs")
                .arg("--output")
                .arg(&output)
                .arg(&url)
                .status();
            match r {
                Ok(s) => {
                    if s.success() {
                        let r = std::process::Command::new("tar")
                            .arg("-xf")
                            .arg(output)
                            .arg("-C")
                            .arg(&out_dir)
                            .status();

                        match r {
                            Ok(s) => {
                                if s.success() {
                                    lib = Path::new(&out_dir).join(file);
                                } else {
                                    println!(
                                        "cargo:warning=view prebuilt not embedded, failed extract, tar exit code: {:?}",
                                        s.code()
                                    )
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
                        )
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
    } else {
        panic!("view prebuilt not embedded, missing '{file}', failed to download");
    }
}
