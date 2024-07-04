//! Localization text scrapper.
//!
//! See the [`l10n!`] documentation for more details.
//!
//! [`l10n!`]: https://zng-ui.github.io/doc/zng/l10n/macro.l10n.html#scrap-template

use std::{
    io::Write,
    path::{Path, PathBuf},
};

use clap::*;

use crate::util;

mod pseudo;
mod scraper;

#[derive(Args, Debug)]
pub struct L10nArgs {
    /// Rust files glob or directory
    #[arg(short, long, default_value = "")]
    input: String,

    /// L10n resources dir
    #[arg(short, long, default_value = "")]
    output: String,

    /// Package to scrap
    ///
    /// If set overrides --input and sets --output default to l10n/ beside src/
    #[arg(short, long, default_value = "")]
    package: String,

    /// Path to Cargo.toml of crate to scrap
    ///
    /// If set overrides --input and sets --output default to l10n/ beside src/
    #[arg(long, default_value = "")]
    manifest_path: String,

    /// Copy dependencies localization
    ///
    /// Use with --package or --manifest-path to copy {dep-pkg}/l10n/*.ftl files
    #[arg(long, action)]
    deps: bool,

    /// Custom l10n macro names, comma separated
    #[arg(short, long, default_value = "")]
    macros: String,

    /// Generate pseudo locale from dir/lang
    ///
    /// EXAMPLE
    ///
    /// "l10n/en" generates pseudo from "l10n/en.ftl" and "l10n/en/*.ftl"
    #[arg(long, default_value = "")]
    pseudo: String,
    /// Generate pseudo mirrored locale
    #[arg(long, default_value = "")]
    pseudo_m: String,
    /// Generate pseudo wide locale
    #[arg(long, default_value = "")]
    pseudo_w: String,
}

pub fn run(mut args: L10nArgs) {
    if !args.input.is_empty() as u8 + !args.package.is_empty() as u8 + !args.manifest_path.is_empty() as u8 > 1 {
        fatal!("only one of --input --package --manifest-path must be set")
    }
    if args.deps && args.package.is_empty() && args.manifest_path.is_empty() {
        fatal!("can only copy --deps with --package or --manifest-path")
    }

    let mut input = String::new();
    let mut output = args.output.replace('\\', "/");

    if !args.input.is_empty() {
        if output.is_empty() {
            fatal!("--output is required for --input")
        }

        input = args.input.replace('\\', "/");

        if !input.contains('*') && PathBuf::from(&input).is_dir() {
            input = format!("{}/**/*.rs", input.trim_end_matches('/'));
        }
    } else {
        if !args.package.is_empty() {
            if let Some(m) = crate::util::manifest_path_from_package(&args.package) {
                args.manifest_path = m;
            } else {
                fatal!("package `{}` not found in workspace", args.package);
            }
        }

        if !args.manifest_path.is_empty() {
            if !Path::new(&args.manifest_path).exists() {
                fatal!("{input} does not exist")
            }

            input = args.manifest_path.replace('\\', "/");
            if let Some(manifest_path) = input.strip_suffix("/Cargo.toml") {
                if output.is_empty() {
                    output = format!("{manifest_path}/l10n");
                }
                input = format!("{manifest_path}/src/**/*.rs");
            } else {
                fatal!("expected path to Cargo.toml manifest file");
            }
        }
    }

    if !input.is_empty() {
        println!(r#"scraping "{input}".."#);

        let custom_macro_names: Vec<&str> = args.macros.split(',').map(|n| n.trim()).collect();
        // let args = ();

        let mut template = scraper::scrape_fluent_text(&input, &custom_macro_names);
        match template.entries.len() {
            0 => println!("did not find any entry"),
            1 => println!("found 1 entry"),
            n => println!("found {n} entries"),
        }

        if !template.entries.is_empty() || !template.notes.is_empty() {
            if let Err(e) = std::fs::create_dir_all(&output) {
                fatal!("cannot create dir `{output}`, {e}");
            }

            template.sort();

            let name_empty = template.has_named_files();

            let r = template.write(|file| {
                fn box_dyn(file: std::fs::File) -> Box<dyn Write + Send> {
                    Box::new(file)
                }

                let mut output = PathBuf::from(&output);
                if file.is_empty() {
                    if name_empty {
                        output.push("template");
                        std::fs::create_dir_all(&output)?;
                        output.push("_.ftl");
                    } else {
                        output.push("template.ftl");
                    }
                } else {
                    output.push("template");
                    std::fs::create_dir_all(&output)?;
                    output.push(format!("{file}.ftl"));
                }
                std::fs::File::create(output).map(box_dyn)
            });
            if let Err(e) = r {
                fatal!("error writing template files, {e}");
            }
        }

        if args.deps {
            let mut count = 0;
            for dep in util::dependencies(&args.manifest_path) {
                let path = Path::new(&dep).with_file_name("l10n");
                if path.exists() {
                    count += 1;
                    // !!: TODO, need package name
                }
            }
            println!("found {count} dependencies with localization");
        }
    }

    if !args.pseudo.is_empty() {
        pseudo::pseudo(&args.pseudo);
    }
    if !args.pseudo_m.is_empty() {
        pseudo::pseudo_mirr(&args.pseudo_m);
    }
    if !args.pseudo_w.is_empty() {
        pseudo::pseudo_wide(&args.pseudo_w);
    }
}
