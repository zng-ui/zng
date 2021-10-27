use std::{env, path::Path};

fn main() {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let lib = Path::new(&dir).join("lib");
    println!("cargo:rustc-link-search={}", lib.display());
}
