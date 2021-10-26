use std::{env, path::Path};

fn main() {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rerun-if-changed=lib/*.lib");
    println!("cargo:rerun-if-changed=lib/*.a");
    println!("cargo:rustc-link-search={}", Path::new(&dir).join("lib").display());
}
