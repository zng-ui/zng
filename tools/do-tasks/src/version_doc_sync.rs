//! Extra tests.

use crate::util::{error, println};
use regex::Regex;
use std::fs::read_to_string;

const CRATE_FILES: &[(&str, &[&str])] = &[
    (
        "zng",
        &[
            "README.md",
            "zng/src/lib.rs",
            "zng/src/app.rs",
            "zng/src/icon.rs",
            "zng-view/src/lib.rs",
        ],
    ),
    ("zng-view", &["zng-view/src/lib.rs"]),
];

pub fn check() {
    println("\nchecking Cargo.toml examples");

    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    for (crate_, files) in CRATE_FILES {
        let version = crate::util::crate_version(crate_);
        let rgx = Regex::new(&format!(r#"{crate_} =.+(?:version = )?"(\d+\.\d+(?:.\d+)?)".*"#)).unwrap();

        for file in *files {
            let contents = read_to_string(&format!("{manifest_dir}/../../{file}")).expect(&file);
            let caps = rgx
                .captures(&contents)
                .unwrap_or_else(|| panic!("expected Cargo.toml example in `{file}`"));

            if caps.get(1).map(|c| c.as_str()).unwrap_or_default() != version {
                error(format_args!(
                    "`{crate_}` cargo example is outdated in `{file}`\n   expected version `\"{version}\"`\n   found    `{cap}`",
                    cap = caps.get(0).unwrap().as_str(),
                ));
            } else {
                println(format!("   `{crate_}` cargo example in `{file}` ... ok"));
            }
        }
    }
}

pub fn fix() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    for (crate_, files) in CRATE_FILES {
        let version = crate::util::crate_version(crate_);
        let rgx = Regex::new(&format!(r#"{crate_} =.+(?:version = )?"(\d+\.\d+(?:.\d+)?)".*"#)).unwrap();

        for file in *files {
            let file_path = format!("{manifest_dir}/../../{file}");
            let mut contents = read_to_string(&file_path).expect(&file);
            let caps = rgx
                .captures(&contents)
                .unwrap_or_else(|| panic!("expected Cargo.toml example in `{file}`"));

            if let Some(cap1) = caps.get(1) {
                if cap1.as_str() != version {
                    contents.replace_range(cap1.range(), &version);
                    std::fs::write(file_path, contents.as_bytes()).expect(&file);
                }
            }
        }
    }
}

pub fn close_changelog() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    let changelog_path = format!("{manifest_dir}/../../CHANGELOG.md");

    let mut changelog = read_to_string(&changelog_path).expect("CHANGELOG.md");
    let title = format!("\n# {}\n\n", crate::util::crate_version("zng"));
    let unpublished = "# Unpublished\n\n";
    assert!(changelog.starts_with(unpublished));
    if !changelog.contains(&title) {
        changelog.insert_str(unpublished.len(), &title);

        std::fs::write(changelog_path, changelog.as_bytes()).expect("CHANGELOG.md");
    }
}
