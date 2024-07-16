use std::{fs, io, path::Path};

use clap::*;

use crate::util;

#[derive(Args, Debug, Default)]
pub struct FmtArgs {
    /// Only check if files are formatted
    #[arg(long, action)]
    check: bool,

    /// Format the crate identified by Cargo.toml
    #[arg(long)]
    manifest_path: Option<String>,

    /// Format the workspace crate identified by package name
    #[arg(long)]
    package: Option<String>,

    /// Format all files matched by glob
    #[arg(long)]
    files: Option<String>,
}

pub fn run(mut args: FmtArgs) {
    let check = if args.check { "--check" } else { "" };

    if let Some(glob) = args.files {
        for file in glob::glob(&glob).unwrap_or_else(|e| fatal!("{e}")) {
            let file = file.unwrap_or_else(|e| fatal!("{e}"));
            if let Err(e) = util::cmd("rustfmt", &["--edition", "2021", check, &file.as_os_str().to_string_lossy()], &[]) {
                fatal!("{e}");
            }
            if let Err(e) = custom_fmt(&file, args.check) {
                fatal!("error formatting `{}`, {e}", file.display());
            }
        }
    }

    if let Some(pkg) = args.package {
        if args.manifest_path.is_some() {
            fatal!("expected only one of --package, --manifest-path");
        }
        match util::manifest_path_from_package(&pkg) {
            Some(m) => args.manifest_path = Some(m),
            None => fatal!("package `{pkg}` not found in workspace"),
        }
    }

    if let Some(path) = args.manifest_path {
        if let Err(e) = util::cmd("cargo fmt --manifest-path", &[&path, check], &[]) {
            fatal!("{e}");
        }

        let files = Path::new(&path)
            .parent()
            .unwrap()
            .join("**/*.rs")
            .display()
            .to_string()
            .replace('\\', "/");
        for file in glob::glob(&files).unwrap_or_else(|e| fatal!("{e}")) {
            let file = file.unwrap_or_else(|e| fatal!("{e}"));
            if let Err(e) = custom_fmt(&file, args.check) {
                fatal!("error formatting `{}`, {e}", file.display());
            }
        }
    } else {
        if let Err(e) = util::cmd("cargo fmt", &[check], &[]) {
            fatal!("{e}");
        }

        for path in util::workspace_manifest_paths() {
            let files = path.parent().unwrap().join("**/*.rs").display().to_string().replace('\\', "/");
            for file in glob::glob(&files).unwrap_or_else(|e| fatal!("{e}")) {
                let file = file.unwrap_or_else(|e| fatal!("{e}"));
                if let Err(e) = custom_fmt(&file, args.check) {
                    fatal!("error formatting `{}`, {e}", file.display());
                }
            }
        }
    }
}

fn custom_fmt(file: &Path, check: bool) -> io::Result<()> {
    let _file = fs::read_to_string(file)?;
    let _ = check;
    // !!: TODO find macros, extract Rust parts from macros, use rustfmt stdin.
    Ok(())
}
