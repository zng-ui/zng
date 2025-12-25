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

use crate::{l10n::scraper::FluentTemplate, util};

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
    /// Use with --package or --manifest-path to not copy {dep-pkg}/l10n/*.ftl files
    #[arg(long, action)]
    no_deps: bool,

    /// Don't scrap `#.#.#-local` dependencies
    ///
    /// Use with --package or --manifest-path to not scrap local dependencies.
    #[arg(long, action)]
    no_local: bool,

    /// Don't scrap the target package.
    ///
    /// Use with --package or --manifest-path to only scrap dependencies.
    #[arg(long, action)]
    no_pkg: bool,

    /// Remove all previously copied dependency localization files.
    #[arg(long, action)]
    clean_deps: bool,

    /// Remove all previously scraped resources before scraping.
    #[arg(long, action)]
    clean_template: bool,

    /// Same as --clean-deps --clean-template
    #[arg(long, action)]
    clean: bool,

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

    /// Use verbose output.
    #[arg(short, long, action)]
    verbose: bool,
}

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
            fatal!("`{}` does not exist", args.manifest_path)
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

    if args.check {
        args.clean = false;
        args.clean_deps = false;
        args.clean_template = false;
    } else if args.clean {
        args.clean_deps = true;
        args.clean_template = true;
    }

    if args.verbose {
        println!(
            "input: `{input}`\noutput: `{output}`\nclean_deps: {}\nclean_template: {}",
            args.clean_deps, args.clean_template
        );
    }

    if input.is_empty() {
        return run_pseudo(args);
    }

    if output.is_empty() {
        fatal!("--output is required for --input")
    }

    let input = input;
    let output = Path::new(&output);

    let mut template = FluentTemplate::default();

    check_scrap_package(&args, &input, output, &mut template);

    if !template.entries.is_empty() || !template.notes.is_empty() {
        if let Err(e) = util::check_or_create_dir_all(args.check, output) {
            fatal!("cannot create dir `{}`, {e}", output.display());
        }

        let output = output.join("template");
        if args.clean_template {
            debug_assert!(!args.check);
            if args.verbose {
                println!("removing `{}` to clean template", output.display());
            }
            if let Err(e) = fs::remove_dir_all(&output)
                && !matches!(e.kind(), io::ErrorKind::NotFound)
            {
                error!("cannot remove `{}`, {e}", output.display());
            }
        }
        if let Err(e) = util::check_or_create_dir_all(args.check, &output) {
            fatal!("cannot create dir `{}`, {e}", output.display());
        }

        template.sort();

        let r = template.write(|file, contents| {
            let output = output.join(format!("{}.ftl", if file.is_empty() { "_" } else { file }));
            util::check_or_write(args.check, output, contents, args.verbose)
        });
        if let Err(e) = r {
            fatal!("error writing template files, {e}");
        }
    }

    run_pseudo(args);
}

fn check_scrap_package(args: &L10nArgs, input: &str, output: &Path, template: &mut FluentTemplate) {
    // scrap the target package
    if !args.no_pkg {
        if args.check {
            println!(r#"checking "{input}".."#);
        } else {
            println!(r#"scraping "{input}".."#);
        }

        let custom_macro_names: Vec<&str> = args.macros.split(',').map(|n| n.trim()).collect();
        let t = scraper::scrape_fluent_text(input, &custom_macro_names);
        if !args.check {
            match t.entries.len() {
                0 => println!("  did not find any entry"),
                1 => println!("  found 1 entry"),
                n => println!("  found {n} entries"),
            }
        }
        template.extend(t);
    }

    // cleanup dependencies
    if args.clean_deps {
        for entry in glob::glob(&format!("{}/*/deps", output.display()))
            .unwrap_or_else(|e| fatal!("cannot cleanup deps in `{}`, {e}", output.display()))
        {
            let dir = entry.unwrap_or_else(|e| fatal!("cannot cleanup deps, {e}"));
            if args.verbose {
                println!("removing `{}` to clean deps", dir.display());
            }
            if let Err(e) = std::fs::remove_dir_all(&dir)
                && !matches!(e.kind(), io::ErrorKind::NotFound)
            {
                error!("cannot remove `{}`, {e}", dir.display());
            }
        }
    }

    // collect dependencies
    let mut local = vec![];
    if !args.no_deps {
        let mut count = 0;
        let (workspace_root, deps) = util::dependencies(&args.manifest_path);
        for dep in deps {
            if dep.version.pre.as_str() == "local" && dep.manifest_path.starts_with(&workspace_root) {
                local.push(dep);
                continue;
            }

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

            // get l10n_dir/{lang}/deps/dep.name/dep.version/
            let mut l10n_dir = |lang: Option<&std::ffi::OsStr>| {
                any = true;
                let dir = output.join(lang.unwrap()).join("deps");

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
                                "# Call `cargo zng l10n --package {}{custom_output} --no-pkg --no-local --clean-deps` to update",
                                args.package
                            )
                            .unwrap();
                        } else {
                            let path = Path::new(&args.manifest_path);
                            let path = path.strip_prefix(std::env::current_dir().unwrap()).unwrap_or(path);
                            writeln!(
                                &mut ignore,
                                "# Call `cargo zng l10n --manifest-path \"{}\" --no-pkg --no-local --clean-deps` to update",
                                path.display()
                            )
                            .unwrap();
                        }
                        writeln!(&mut ignore).unwrap();
                        writeln!(&mut ignore, "*").unwrap();
                        writeln!(&mut ignore, "!.gitignore").unwrap();

                        if let Err(e) = fs::write(&ignore_file, ignore.as_bytes()) {
                            fatal!("cannot write `{}`, {e}", ignore_file.display())
                        }

                        Ok(())
                    })()
                    .unwrap_or_else(|e| fatal!("cannot create `{}`, {e}", output.display()));
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
                            if let Err(e) = util::check_or_copy(args.check, &lang_entry, &to, args.verbose) {
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
                    if !target.exists()
                        && entry.is_file()
                        && let Err(e) = util::check_or_copy(args.check, &entry, &target, args.verbose)
                    {
                        error!("cannot copy `{}` to `{}`, {e}", entry.display(), target.display());
                    }
                }
            }

            count += any as u32;
        }
        println!("found {count} dependencies with localization");
    }

    // scrap local dependencies
    if !args.no_local {
        for dep in local {
            let manifest_path = dep.manifest_path.display().to_string();
            let input = manifest_path.replace('\\', "/");
            let input = input.strip_suffix("/Cargo.toml").unwrap();
            let input = format!("{input}/src/**/*.rs");
            check_scrap_package(
                &L10nArgs {
                    input: String::new(),
                    output: String::new(),
                    package: String::new(),
                    manifest_path,
                    no_deps: true,
                    no_local: true,
                    no_pkg: false,
                    clean_deps: false,
                    clean_template: false,
                    clean: false,
                    macros: args.macros.clone(),
                    pseudo: String::new(),
                    pseudo_m: String::new(),
                    pseudo_w: String::new(),
                    check: args.check,
                    verbose: args.verbose,
                },
                &input,
                output,
                template,
            )
        }
    }
}

fn run_pseudo(args: L10nArgs) {
    if !args.pseudo.is_empty() {
        pseudo::pseudo(&args.pseudo, args.check, args.verbose);
    }
    if !args.pseudo_m.is_empty() {
        pseudo::pseudo_mirr(&args.pseudo_m, args.check, args.verbose);
    }
    if !args.pseudo_w.is_empty() {
        pseudo::pseudo_wide(&args.pseudo_w, args.check, args.verbose);
    }
}
