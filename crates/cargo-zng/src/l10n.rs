//! Localization text scrapper.
//!
//! See the [`l10n!`] documentation for more details.
//!
//! [`l10n!`]: https://zng-ui.github.io/doc/zng/l10n/macro.l10n.html#scrap-template

use std::{
    cmp::Ordering,
    fs,
    io::{self, Write},
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

    /// Verify that the generated files are the same
    #[arg(long, action)]
    check: bool, // !!: TODO
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
                let l10n_dir = Path::new(&args.manifest_path).with_file_name("l10n");

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
                            fs::create_dir_all(&dir)?;
                            let mut f = io::BufWriter::new(fs::File::options().create(true).truncate(true).write(true).open(ignore_file)?);
                            writeln!(&mut f, "# Dependency localization files")?;
                            if !args.package.is_empty() {
                                writeln!(&mut f, "#Call `cargo zng l10n --package {}` to update", args.package)?;
                            } else {
                                let path = Path::new(&args.manifest_path)
                                    .strip_prefix(std::env::current_dir().unwrap())
                                    .unwrap();
                                writeln!(&mut f, "#Call `cargo zng l10n --manifest-path {}` to update", path.display())?;
                            }
                            writeln!(&mut f)?;
                            writeln!(&mut f, "*")?;
                            writeln!(&mut f, "!.gitignore")?;
                            f.flush()?;
                            Ok(())
                        })()
                        .unwrap_or_else(|e| fatal!("cannot create `{}`, {e}", l10n_dir.display()));
                    }

                    let dir = dir.join(&dep.name).join(dep.version.to_string());
                    let _ = fs::create_dir_all(&dir);

                    dir
                };

                // [(exporter_dep, has_lang, ".../{lang}?/deps")]
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
                        if dep_l10n_entry.file_name().map(|n| n == "deps").unwrap_or(false) {
                            reexport_deps.push((&dep, false, dep_l10n_entry));
                            continue;
                        }

                        // l10n/{lang}/deps/{dep.name}/{dep.version}
                        let output_dir = l10n_dir(dep_l10n_entry.file_name());
                        let _ = fs::create_dir_all(&output_dir);

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
                                    reexport_deps.push((&dep, true, lang_entry));
                                }
                            } else if lang_entry.is_file() && lang_entry.extension().map(|e| e == "ftl").unwrap_or(false) {
                                let _ = fs::create_dir_all(&output_dir);
                                let to = output_dir.join(lang_entry.file_name().unwrap());
                                if let Err(e) = fs::copy(&lang_entry, &to) {
                                    error!("cannot copy `{}` to `{}`, {e}", lang_entry.display(), to.display());
                                    continue;
                                }
                            }
                        }
                    } else if dep_l10n_entry.is_file() && dep_l10n_entry.extension().map(|e| e == "ftl").unwrap_or(false) {
                        // l10n/deps/{dep.name}/{dep.version}/
                        let to = l10n_dir(None);
                        if let Err(e) = fs::copy(&dep_l10n_entry, &to) {
                            error!("cannot copy `{}` to `{}`, {e}", dep_l10n_entry.display(), to.display());
                            continue;
                        }
                    }
                }

                reexport_deps.sort_by(|a, b| match a.0.name.cmp(&b.0.name) {
                    Ordering::Equal => b.0.version.cmp(&a.0.version),
                    o => o,
                });

                for (_, has_lang, deps) in reexport_deps {
                    let target = l10n_dir(if has_lang {
                        // dep/l10n/lang/deps/
                        deps.parent().and_then(|p| p.file_name())
                    } else {
                        // dep/l10n/deps/
                        None
                    });

                    // deps/pkg-name/pkg-version/*.ftl
                    for entry in glob::glob(&deps.join("*/*/*.ftl").display().to_string()).unwrap() {
                        let entry = entry.unwrap_or_else(|e| fatal!("cannot read `{}` entry, {e}", deps.display()));
                        let target = target.join(entry.strip_prefix(&deps).unwrap());
                        if !target.exists() && entry.is_file() {
                            if let Err(e) = fs::copy(&entry, &target) {
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
        pseudo::pseudo(&args.pseudo);
    }
    if !args.pseudo_m.is_empty() {
        pseudo::pseudo_mirr(&args.pseudo_m);
    }
    if !args.pseudo_w.is_empty() {
        pseudo::pseudo_wide(&args.pseudo_w);
    }
}
