use std::fs;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../split-view/Cargo.lock");

    Command::new("cargo")
        .args(&["build", "--manifest-path", "../split-view/Cargo.toml"])
        .output()
        .unwrap();

    let source = if cfg!(windows) {
        "../split-view/target/debug/split-view.exe"
    } else {
        "../split-view/target/debug/split-view"
    };

    if cfg!(debug_assertions) {
        fs::copy(source, "./target/debug/split-view").unwrap();
    } else {
        fs::copy(source, "./target/release/split-view").unwrap();
    }
}
