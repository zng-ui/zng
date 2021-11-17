//! Extra tests.

use crate::util::error;
use regex::Regex;
use std::fs::read_to_string;

pub fn version_in_sync() {
    let version = zero_ui_version();
    let rgx = Regex::new(r#"zero-ui = "(\d+\.\d+)""#).unwrap();

    let check_file = |path| {
        let file = read_to_string(path).unwrap();
        let caps = rgx.captures(&file).unwrap_or_else(|| panic!("expected usage help in `{}`", path));
        if caps.get(1).map(|c| c.as_str()).unwrap_or_default() != version {
            error(format_args!(
                "usage example is outdated in `{}`\n   expected `zero-ui = \"{}\"'`\n   found    `{}`",
                path,
                version,
                caps.get(0).unwrap().as_str(),
            ));
        }
    };

    check_file("README.md");
    check_file("zero-ui/lib.rs");
}

fn zero_ui_version() -> String {
    let toml = read_to_string("Cargo.toml").expect("did not find `Cargo.toml`, run `do` in the project root");
    assert!(toml.contains(r#"name = "zero-ui""#), "run `do` in the project root");
    let rgx = Regex::new(r#"version = "(\d+\.\d+).*""#).unwrap();
    rgx.captures(&toml).unwrap().get(1).unwrap().as_str().to_owned()
}
