use std::{env, hash::Hasher, path::Path};

use base64::Engine;
use hashers::jenkins::spooky_hash::SpookyHasher;

fn main() {
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
        println!("cargo:warning=view prebuilt not embedded, unsuported os");
        return;
    }

    lib = lib.join(file);

    if !lib.exists() {
        let version = env::var("CARGO_PKG_VERSION").unwrap();
        lib = home::cargo_home()
            .unwrap()
            .join(".zng-view-prebuilt")
            .join(format!("{file}.{version}.bin"));
        if !lib.exists() {
            let url = format!("https://github.com/zng-ui/zng/releases/download/v{version}/{file}");

            let temp_file = Path::new(&env::var("OUT_DIR").unwrap()).join(format!("{file}.{version}.tmp"));

            let r = std::process::Command::new("curl")
                .arg("--location")
                .arg("--fail")
                .arg("--silent")
                .arg("--show-error")
                .arg("--create-dirs")
                .arg("--output")
                .arg(&temp_file)
                .arg(&url)
                .status();
            match r {
                Ok(s) => {
                    if s.success() {
                        if let Err(e) = std::fs::copy(&temp_file, &lib) {
                            println!("cargo:warning=view prebuilt not embedded, failed copy, {e}");
                        } else {
                            let _ = std::fs::remove_file(temp_file);
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
        println!("cargo:rustc-env=ZNG_VIEW_LIB={}", lib.canonicalize().unwrap().display());

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
    } else {
        println!("cargo:warning=missing `{file}`, run `do prebuild` in local builds");
        println!("cargo:warning=view prebuilt not embedded, missing '{file}', failed to download, in local builds call `do prebuild`");
    }
}
