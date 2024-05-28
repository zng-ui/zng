use std::{env, fs, path::PathBuf};

fn main() {
    if env::var("ZR_HELP").is_ok() {
        println!(".zr-tool2 help!");
        std::process::exit(0);
    }

    let message = fs::read_to_string(path("ZR_REQUEST")).unwrap_or_else(|e| panic!("{e}"));
    println!("{message} (by tool2)");
}

fn path(var: &str) -> PathBuf {
    env::var(var).unwrap_or_else(|_| panic!("missing {var}")).into()
}