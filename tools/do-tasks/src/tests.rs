//! Extra tests.

use crate::util::error;
use regex::Regex;
use std::fs::read_to_string;

pub fn version_in_sync() {
    let version = zero_ui_version();
    let rgx = Regex::new(r#"zero-ui = "(\d+\.\d+)""#).unwrap();

    let check_file = |path| {
        let path = format!("{manifest_dir}/../../{path}", manifest_dir = env!("CARGO_MANIFEST_DIR"));
        let file = read_to_string(&path).expect(&path);
        let caps = rgx.captures(&file).unwrap_or_else(|| panic!("expected usage help in `{path}`"));
        if caps.get(1).map(|c| c.as_str()).unwrap_or_default() != version {
            error(format_args!(
                "usage example is outdated in `{path}`\n   expected `zero-ui = \"{version}\"'`\n   found    `{cap}`",
                cap = caps.get(0).unwrap().as_str(),
            ));
        }
    };

    check_file("README.md");
    check_file("zero-ui/src/lib.rs");
}

fn zero_ui_version() -> String {
    let path = format!("{manifest_dir}/../../zero-ui/Cargo.toml", manifest_dir = env!("CARGO_MANIFEST_DIR"));
    let toml = read_to_string(&path).expect(&path);
    assert!(toml.contains(r#"name = "zero-ui""#), "run `do` in the project root");
    let rgx = Regex::new(r#"version = "(\d+\.\d+).*""#).unwrap();
    rgx.captures(&toml).unwrap().get(1).unwrap().as_str().to_owned()
}
