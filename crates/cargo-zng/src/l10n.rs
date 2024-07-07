//! Localization text scrapper.
//!
//! See the [`l10n!`] documentation for more details.
//!
//! [`l10n!`]: https://zng-ui.github.io/doc/zng/l10n/macro.l10n.html#scrap-template

use std::{
    cmp::Ordering,
    fmt::Write as _,
    fs, io,
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

    /// Package to scrap and copy dependencies
    ///
    /// If set the --input and --output default is src/**.rs and l10n/
    #[arg(short, long, default_value = "")]
    package: String,

    /// Path to Cargo.toml of crate to scrap and copy dependencies
    ///
    /// If set the --input and --output default to src/**.rs and l10n/
    #[arg(long, default_value = "")]
    manifest_path: String,

    /// Don't copy dependencies localization
    ///
    /// Use with --package or --manifest-path to copy {dep-pkg}/l10n/*.ftl files
    #[arg(long, action)]
    no_deps: bool,

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

    /// Only verify that the generated files are the same
    #[arg(long, action)]
    check: bool,
}

// !!: TODO internal crates

pub fn run(mut args: L10nArgs) {
    if !args.package.is_empty() && !args.manifest_path.is_empty() {
        fatal!("only one of --package --manifest-path must be set")
    }

    let mut input = String::new();
    let mut output = args.output.replace('\\', "/");

    if !args.input.is_empty() {
        input = args.input.replace('\\', "/");

        if !input.contains('*') && PathBuf::from(&input).is_dir() {
            input = format!("{}/**/*.rs", input.trim_end_matches('/'));
        }
    }
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

        if let Some(path) = args.manifest_path.replace('\\', "/").strip_suffix("/Cargo.toml") {
            if output.is_empty() {
                output = format!("{path}/l10n");
            }
            if input.is_empty() {
                input = format!("{path}/src/**/*.rs");
            }
        } else {
            fatal!("expected path to Cargo.toml manifest file");
        }
    }

    if !input.is_empty() {
        if output.is_empty() {
            fatal!("--output is required for --input")
        }

        if args.check {
            println!(r#"checking "{input}".."#);
        } else {
            println!(r#"scraping "{input}".."#);
        }

        let custom_macro_names: Vec<&str> = args.macros.split(',').map(|n| n.trim()).collect();
        // let args = ();

        let mut template = scraper::scrape_fluent_text(&input, &custom_macro_names);
        if !args.check {
            match template.entries.len() {
                0 => println!("did not find any entry"),
                1 => println!("found 1 entry"),
                n => println!("found {n} entries"),
            }
        }

        if !template.entries.is_empty() || !template.notes.is_empty() {
            if let Err(e) = util::check_or_create_dir_all(args.check, &output) {
                fatal!("cannot create dir `{output}`, {e}");
            }

            template.sort();

            let r = template.write(|file, contents| {
                let mut output = PathBuf::from(&output);
                output.push("template");
                util::check_or_create_dir_all(args.check, &output)?;
                output.push(format!("{}.ftl", if file.is_empty() { "_" } else { file }));
                util::check_or_write(args.check, output, contents)
            });
            if let Err(e) = r {
                fatal!("error writing template files, {e}");
            }
        }

        if !args.no_deps {
            let mut count = 0;
            for dep in util::dependencies(&args.manifest_path) {
                let dep_l10n = dep.manifest_path.with_file_name("l10n");
                let dep_l10n_reader = match fs::read_dir(&dep_l10n) {
                    Ok(d) => d,
                    Err(e) => {
                        if !matches!(e.kind(), io::ErrorKind::NotFound) {
                            error!("cannot read `{}`, {e}", dep_l10n.display());
                        }
                        continue;
                    }
                };

                let mut any = false;

                let l10n_dir = Path::new(&output);
                // get l10n_dir/{lang}/deps/dep.name/dep.version/
                let mut l10n_dir = |lang: Option<&std::ffi::OsStr>| {
                    any = true;
                    let dir = match lang {
                        Some(l) => l10n_dir.join(l).join("deps"),
                        None => l10n_dir.join("deps"),
                    };
                    let ignore_file = dir.join(".gitignore");

                    if !ignore_file.exists() {
                        // create dir and .gitignore file
                        (|| -> io::Result<()> {
                            util::check_or_create_dir_all(args.check, &dir)?;

                            let mut ignore = "# Dependency localization files\n".to_owned();

                            let output = Path::new(&output);
                            let custom_output = if output != Path::new(&args.manifest_path).with_file_name("l10n") {
                                format!(
                                    " --output \"{}\"",
                                    output.strip_prefix(std::env::current_dir().unwrap()).unwrap_or(output).display()
                                )
                                .replace('\\', "/")
                            } else {
                                String::new()
                            };
                            if !args.package.is_empty() {
                                writeln!(
                                    &mut ignore,
                                    "# Call `cargo zng l10n --package {}{custom_output}` to update",
                                    args.package
                                )
                                .unwrap();
                            } else {
                                let path = Path::new(&args.manifest_path);
                                let path = path.strip_prefix(std::env::current_dir().unwrap()).unwrap_or(path);
                                writeln!(
                                    &mut ignore,
                                    "# Call `cargo zng l10n --manifest-path \"{}\"` to update",
                                    path.display()
                                )
                                .unwrap();
                            }
                            writeln!(&mut ignore).unwrap();
                            writeln!(&mut ignore, "*").unwrap();
                            writeln!(&mut ignore, "!.gitignore").unwrap();

                            Ok(())
                        })()
                        .unwrap_or_else(|e| fatal!("cannot create `{}`, {e}", l10n_dir.display()));
                    }

                    let dir = dir.join(&dep.name).join(dep.version.to_string());
                    let _ = util::check_or_create_dir_all(args.check, &dir);

                    dir
                };

                // [(exporter_dep, ".../{lang}?/deps")]
                let mut reexport_deps = vec![];

                for dep_l10n_entry in dep_l10n_reader {
                    let dep_l10n_entry = match dep_l10n_entry {
                        Ok(e) => e.path(),
                        Err(e) => {
                            error!("cannot read `{}` entry, {e}", dep_l10n.display());
                            continue;
                        }
                    };
                    if dep_l10n_entry.is_dir() {
                        // l10n/{lang}/deps/{dep.name}/{dep.version}
                        let output_dir = l10n_dir(dep_l10n_entry.file_name());
                        let _ = util::check_or_create_dir_all(args.check, &output_dir);

                        let lang_dir_reader = match fs::read_dir(&dep_l10n_entry) {
                            Ok(d) => d,
                            Err(e) => {
                                error!("cannot read `{}`, {e}", dep_l10n_entry.display());
                                continue;
                            }
                        };

                        for lang_entry in lang_dir_reader {
                            let lang_entry = match lang_entry {
                                Ok(e) => e.path(),
                                Err(e) => {
                                    error!("cannot read `{}` entry, {e}", dep_l10n_entry.display());
                                    continue;
                                }
                            };

                            if lang_entry.is_dir() {
                                if lang_entry.file_name().map(|n| n == "deps").unwrap_or(false) {
                                    reexport_deps.push((&dep, lang_entry));
                                }
                            } else if lang_entry.is_file() && lang_entry.extension().map(|e| e == "ftl").unwrap_or(false) {
                                let _ = util::check_or_create_dir_all(args.check, &output_dir);
                                let to = output_dir.join(lang_entry.file_name().unwrap());
                                if let Err(e) = util::check_or_copy(args.check, &lang_entry, &to) {
                                    error!("cannot copy `{}` to `{}`, {e}", lang_entry.display(), to.display());
                                    continue;
                                }
                            }
                        }
                    }
                }

                reexport_deps.sort_by(|a, b| match a.0.name.cmp(&b.0.name) {
                    Ordering::Equal => b.0.version.cmp(&a.0.version),
                    o => o,
                });

                for (_, deps) in reexport_deps {
                    // dep/l10n/lang/deps/
                    let target = l10n_dir(deps.parent().and_then(|p| p.file_name()));

                    // deps/pkg-name/pkg-version/*.ftl
                    for entry in glob::glob(&deps.join("*/*/*.ftl").display().to_string()).unwrap() {
                        let entry = entry.unwrap_or_else(|e| fatal!("cannot read `{}` entry, {e}", deps.display()));
                        let target = target.join(entry.strip_prefix(&deps).unwrap());
                        if !target.exists() && entry.is_file() {
                            if let Err(e) = util::check_or_copy(args.check, &entry, &target) {
                                error!("cannot copy `{}` to `{}`, {e}", entry.display(), target.display());
                            }
                        }
                    }
                }

                count += any as u32;
            }
            println!("found {count} dependencies with localization");
        }
    }

    if !args.pseudo.is_empty() {
        pseudo::pseudo(&args.pseudo, args.check);
    }
    if !args.pseudo_m.is_empty() {
        pseudo::pseudo_mirr(&args.pseudo_m, args.check);
    }
    if !args.pseudo_w.is_empty() {
        pseudo::pseudo_wide(&args.pseudo_w, args.check);
    }
}
