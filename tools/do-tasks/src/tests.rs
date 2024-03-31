//! Extra tests.

use crate::util::{error, println};
use regex::Regex;
use std::fs::read_to_string;

pub fn version_in_sync() {
    let version = zng_version();
    let rgx = Regex::new(r#"zng =.+(?:version = )?"(\d+\.\d+(?:.\d+)?)".*"#).unwrap();

    println("\nchecking cargo examples");

    let check_file = |path| {
        let path = format!("{manifest_dir}/../../{path}", manifest_dir = env!("CARGO_MANIFEST_DIR"));
        let file = read_to_string(&path).expect(&path);
        let caps = rgx.captures(&file).unwrap_or_else(|| panic!("expected cargo example in `{path}`"));
        if caps.get(1).map(|c| c.as_str()).unwrap_or_default() != version {
            error(format_args!(
                "cargo example is outdated in `{path}`\n   expected version `\"{version}\"`\n   found    `{cap}`",
                cap = caps.get(0).unwrap().as_str(),
            ));
        } else {
            println(format!("   cargo example in `{path}` ... ok"));
        }
    };

    check_file("README.md");
    check_file("zng/src/lib.rs");

    println("\n");
}

fn zng_version() -> String {
    let path = format!("{manifest_dir}/../../zng/Cargo.toml", manifest_dir = env!("CARGO_MANIFEST_DIR"));
    let toml = read_to_string(&path).expect(&path);
    assert!(toml.contains(r#"name = "zng""#), "run `do` in the project root");
    let rgx = Regex::new(r#"version = "(\d+\.\d+.*)""#).unwrap();
    rgx.captures(&toml).unwrap().get(1).unwrap().as_str().to_owned()
}
