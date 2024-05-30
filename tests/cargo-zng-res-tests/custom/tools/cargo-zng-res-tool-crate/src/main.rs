use std::{env, fs, path::PathBuf};

fn main() {
    if env::var("ZR_HELP").is_ok() {
        println!(".zr-tool-crate help!");
        std::process::exit(0);
    }
    println!("tool-crate print!");
    fs::copy(path("ZR_REQUEST"), path("ZR_TARGET")).unwrap();
}

fn path(var: &str) -> PathBuf {
    env::var(var).unwrap_or_else(|_| panic!("missing {var}")).into()
}
