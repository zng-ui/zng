use std::{env, path::Path};

fn main() {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut lib = Path::new(&dir).join("lib");
    println!("cargo:rerun-if-changed={}", lib.display());

    #[allow(unused_variables)]
    let file = "unknown-OS";

    #[cfg(target_os = "windows")]
    let file = "zero_ui_view.dll";

    #[cfg(target_os = "linux")]
    let file = "zero_ui_view.so";

    #[cfg(target_os = "macos")]
    let file = "zero_ui_view.dylib";

    lib.set_file_name(file);

    let out_dir = env::var("OUT_DIR").unwrap();
    let out_lib = Path::new(&out_dir).join("zero_ui_view").join(file);
    
    if let Err(e) = std::fs::copy(lib, out_lib) {
        eprintln!("failed to copy `{}` to output, {}", file, e);
    }
}
