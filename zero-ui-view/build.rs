fn main() {
    if cfg!(feature = "software") {
        if cfg!(target_env = "msvc") {
            use std::{env, process::Command};

            fn check_env(var: &str) -> bool {
                if let Ok(path) = env::var(var) {
                    let out = Command::new(path).arg("--version").output();
                    matches!(out, Ok(out) if out.stdout.starts_with(b"clang version"))
                } else {
                    false
                }
            }
            if !check_env("CC") || !check_env("CXX") {
                println!(
                    r#"cargo:warning=zero-ui-view feature "software" disabled because `CC`, `CXX` are not set to `clang-cl`, \
                            this is required to build on Windows MSVC, see crate docs"#
                );

                return;
            }
        }

        println!("cargo:rustc-cfg=software");
    }
}
