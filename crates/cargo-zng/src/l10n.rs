//! Localization text scrapper.
//!
//! See the [`l10n!`] documentation for more details.
//!
//! [`l10n!`]: https://zng-ui.github.io/doc/zng/l10n/macro.l10n.html#scrap-template

use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    fmt::{self, Write as _},
    fs,
    io::{self, BufRead},
    path::{Path, PathBuf},
};

use clap::*;

use crate::{l10n::scraper::FluentTemplate, util};

mod scraper;

mod generate_util;
mod pseudo;
mod translate;

#[derive(Args, Debug)]
pub struct L10nArgs {
    /// Rust files glob or directory
    #[arg(short, long, default_value = "", value_name = "PATH", hide_default_value = true)]
    input: String,

    /// L10n resources dir
    #[arg(short, long, default_value = "", value_name = "DIR", hide_default_value = true)]
    output: String,

    /// Package to scrap and copy dependencies
    ///
    /// If set the --input and --output default is src/**.rs and l10n/
    #[arg(short, long, default_value = "", hide_default_value = true)]
    package: String,

    /// Path to Cargo.toml of crate to scrap and copy dependencies
    ///
    /// If set the --input and --output default to src/**.rs and l10n/
    #[arg(long, default_value = "", hide_default_value = true)]
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
    #[arg(short, long, default_value = "", hide_default_value = true)]
    macros: String,

    /// Generate pseudo locale from dir/lang
    ///
    /// EXAMPLE
    ///
    /// "l10n/en" generates pseudo from "l10n/en/**/*.ftl" to "l10n/pseudo"
    #[arg(long, default_value = "", value_name = "PATH", hide_default_value = true)]
    pseudo: String,
    /// Generate pseudo mirrored locale
    #[arg(long, default_value = "", value_name = "PATH", hide_default_value = true)]
    pseudo_m: String,
    /// Generate pseudo wide locale
    #[arg(long, default_value = "", value_name = "PATH", hide_default_value = true)]
    pseudo_w: String,

    /// Machine translate locale from dir/lang
    ///
    /// EXAMPLE
    ///
    /// "l10n/template" translates from "l10n/template/**/*.ftl" to a folder for each --translate-to language
    #[arg(long, default_value = "", value_name = "PATH", hide_default_value = true)]
    translate: String,

    /// Explicit source language for --translate
    ///
    /// By default is the source folder name, or English for `template`
    #[arg(long, default_value = "", value_name = "LANG", hide_default_value = true)]
    translate_from: String,

    /// Target languages for --translate
    #[arg(long, default_value = "de,es,fr,it,ja,ko,pt,zh-Hans", value_name = "LANGS")] // !!: TODO
    translate_to: String,

    /// Replace all existing machine translations with --translate
    ///
    /// By default only replaces stale translations
    #[arg(long, action)]
    translate_replace: bool,

    /// Verify that packages are scrapped and validate Fluent files
    #[arg(long, action)]
    check: bool,

    /// Require that all template keys be present in all localized files
    #[arg(long, action)]
    check_strict: bool,

    /// Use verbose output.
    #[arg(short, long, action)]
    verbose: bool,
}

pub fn run(mut args: L10nArgs) {
    if !args.package.is_empty() && !args.manifest_path.is_empty() {
        fatal!("only one of --package --manifest-path must be set")
    }

    if args.check_strict {
        args.check = true;
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
        return run_generators(&args);
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

        if let Err(e) = util::check_or_create_dir_all(args.check, &output) {
            fatal!("cannot create dir `{}`, {e}", output.display());
        }

        template.sort();

        let mut clean_files = HashSet::new();

        let r = template.write(|file, contents| {
            let file = format!("{}.ftl", if file.is_empty() { "_" } else { file });
            let output = output.join(&file);
            clean_files.insert(file);
            util::check_or_write(args.check, output, contents, args.verbose)
        });
        if let Err(e) = r {
            fatal!("error writing template files, {e}");
        }

        if args.clean_template {
            debug_assert!(!args.check);

            let cleanup = || -> std::io::Result<()> {
                for entry in std::fs::read_dir(&output)? {
                    let entry = entry?.path();
                    if entry.is_file() {
                        let name = entry.file_prefix().unwrap().to_string_lossy();
                        if name.ends_with(".ftl") && !clean_files.contains(&*name) {
                            let mut entry_file = std::fs::File::open(&entry)?;
                            if let Some(first_line) = std::io::BufReader::new(&mut entry_file).lines().next()
                                && first_line?.starts_with(FluentTemplate::AUTO_GENERATED_HEADER)
                            {
                                drop(entry_file);
                                std::fs::remove_file(entry)?;
                            }
                        }
                    }
                }
                Ok(())
            };
            if let Err(e) = cleanup() {
                error!("failed template cleanup, {e}");
            }
        }
    }

    if args.check {
        check_fluent_output(&args, output);
    }

    run_generators(&args);
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
                println!("removing `{}` to clean dependencies", dir.display());
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
                    translate: String::new(),
                    translate_from: String::new(),
                    translate_to: String::new(),
                    translate_replace: false,
                    check: args.check,
                    check_strict: args.check_strict,
                    verbose: args.verbose,
                },
                &input,
                output,
                template,
            )
        }
    }
}

fn run_generators(args: &L10nArgs) {
    if !args.pseudo.is_empty() {
        pseudo::pseudo(&args.pseudo, args.check, args.verbose);
    }
    if !args.pseudo_m.is_empty() {
        pseudo::pseudo_mirr(&args.pseudo_m, args.check, args.verbose);
    }
    if !args.pseudo_w.is_empty() {
        pseudo::pseudo_wide(&args.pseudo_w, args.check, args.verbose);
    }
    if !args.translate.is_empty() {
        translate::translate(
            &args.translate,
            &args.translate_from,
            &args.translate_to,
            args.translate_replace,
            args.check,
            args.verbose,
        );
    }
}

fn check_fluent_output(args: &L10nArgs, output: &Path) {
    let read_dir = match fs::read_dir(output) {
        Ok(d) => d,
        Err(e) if matches!(e.kind(), io::ErrorKind::NotFound) => {
            if args.verbose {
                eprintln!("no fluent files to check, `{}` not found", output.display());
            }
            return;
        }
        Err(e) => fatal!("cannot read `{}`, {e}", output.display()),
    };

    // validate syntax of */*.ftl and collect entry keys
    let mut template = None;
    let mut langs = vec![];
    for lang_dir in read_dir {
        let lang_dir = lang_dir
            .unwrap_or_else(|e| fatal!("cannot read `{}`, {e}", output.display()))
            .path();
        if lang_dir.is_dir() {
            let mut files = vec![];

            for file in fs::read_dir(&lang_dir).unwrap_or_else(|e| fatal!("cannot read `{}`, {e}", lang_dir.display())) {
                let file = file.unwrap_or_else(|e| fatal!("cannot read `{}`, {e}", lang_dir.display())).path();
                if file.is_file() {
                    let content = fs::read_to_string(&file).unwrap_or_else(|e| fatal!("cannot read `{}`, {e}", file.display()));
                    let content = match fluent_syntax::parser::parse(content.as_str()) {
                        Ok(r) => r,
                        Err((_, errors)) => {
                            let e = FluentParserErrors(errors);
                            error!("cannot parse `{}`\n{e}", file.display());
                            continue;
                        }
                    };

                    let mut keys = vec![];
                    for entry in content.body {
                        if let fluent_syntax::ast::Entry::Message(m) = entry {
                            let key = m.id.name.to_owned();
                            keys.push((key, m.value.is_some()));
                            for attr in m.attributes {
                                keys.push((format!("{}.{}", m.id.name, attr.id.name), true));
                            }
                        }
                    }

                    files.push((file.file_name().unwrap().to_owned(), keys));
                }
            }

            if lang_dir.file_name().unwrap() == "template" {
                assert!(template.is_none());
                template = Some(files);
            } else {
                langs.push((lang_dir, files));
            }
        }
    }
    if util::is_failed_run() {
        return;
    }

    // check
    if let Some(template) = template {
        if langs.is_empty() {
            if args.verbose {
                eprintln!("no fluent files to compare with template");
            }
        } else {
            // faster template lookup
            let template = template
                .into_iter()
                .map(|(k, v)| (k, v.into_iter().collect::<HashMap<_, _>>()))
                .collect::<HashMap<_, _>>();

            for (lang, files) in langs {
                // match localized against template
                for (file, messages) in &files {
                    let mut errors = vec![];
                    if let Some(template_msgs) = template.get(file) {
                        for (id, has_value) in messages {
                            if let Some(template_has_value) = template_msgs.get(id) {
                                if has_value != template_has_value {
                                    if *has_value {
                                        errors.push(format!("unexpected value, `{id}` has no value in template"));
                                    } else if args.check_strict {
                                        errors.push(format!("missing value, `{id}` has value in template"));
                                    }
                                }
                            } else {
                                errors.push(format!("unknown id, `{id}` not found in template file"));
                            }
                        }
                        if args.check_strict {
                            for template_id in template_msgs.keys() {
                                if !messages.iter().any(|(i, _)| i == template_id) {
                                    errors.push(format!("missing id, `{template_id}` not found in localized file"));
                                }
                            }
                        }
                    } else {
                        errors.push("template file not found".to_owned());
                    }
                    if !errors.is_empty() {
                        let lang_path = Path::new(lang.file_name().unwrap()).join(file);
                        let template_path = Path::new("template").join(file);
                        let mut msg = format!("`{}` does not match `{}`\n", lang_path.display(), template_path.display());
                        for error in errors {
                            msg.push_str("  ");
                            msg.push_str(&error);
                            msg.push('\n');
                        }
                        error!("{msg}");
                    }
                }
                if args.check_strict {
                    for template_file in template.keys() {
                        if !files.iter().any(|(f, _)| f == template_file) {
                            let lang_path = Path::new(lang.file_name().unwrap()).join(template_file);
                            let template_path = Path::new("template").join(template_file);
                            error!(
                                "`{}` does not match `{}`\n   localized file not found",
                                lang_path.display(),
                                template_path.display()
                            );
                        }
                    }
                }
            }
        }
    } else if args.verbose {
        eprintln!("no template to compare, `{}` not found", output.join("template").display());
    }
}
struct FluentParserErrors(Vec<fluent_syntax::parser::ParserError>);
impl fmt::Display for FluentParserErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut sep = "";
        for e in &self.0 {
            write!(f, "  {sep}{e}")?;
            sep = "\n";
        }
        Ok(())
    }
}
