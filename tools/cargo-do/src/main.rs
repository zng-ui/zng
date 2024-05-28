mod readme_gen;
mod util;
mod version_doc_sync;
use std::format_args as f;
use util::*;

fn main() {
    let (task, args) = args();

    match task {
        "ra_check" => ra_check(args),
        "rust_analyzer_check" => rust_analyzer_check(args),
        "fmt" | "f" => fmt(args),
        "test" | "t" => test(args),
        "run" | "r" => run(args),
        "doc" => doc(args),
        "expand" => expand(args),
        "check" | "c" => check(args),
        "build" | "b" => build(args),
        "prebuild" => prebuild(args),
        "clean" => clean(args),
        "asm" => asm(args),
        "rust_analyzer_run" => rust_analyzer_run(args),
        "install" => install(args),
        "publish" => publish(args),
        "semver_check" => semver_check(args),
        "publish_version_tag" => publish_version_tag(args),
        "comment_feature" => comment_feature(args),
        "latest_release_changes" => latest_release_changes(args),
        "just" => just(args),
        "version" => version(args),
        "ls" => ls(args),
        "help" | "--help" => help(args),
        _ => fatal(f!("unknown task {task:?}, `{} help` to list tasks", do_cmd())),
    }

    util::exit_checked();
}

// do install [--execute]
//    Install `do` dependencies after confirmation.
// USAGE:
//     install
//       Shows what commands will run and asks for confirmation.
//     install --execute
//       Runs the installation commands.
fn install(mut args: Vec<&str>) {
    if take_flag(&mut args, &["--execute"]) {
        cmd("rustup", &["toolchain", "install", "nightly"], &[]);
        cmd("rustup", &["component", "add", "rustfmt"], &[]);
        cmd("rustup", &["component", "add", "clippy"], &[]);
        cmd("cargo", &["install", "cargo-expand"], &[]);
        cmd("cargo", &["install", "cargo-asm"], &[]);
        cmd("cargo", &["install", "cargo-about"], &[]);
        cmd("cargo", &["install", "cargo-semver-checks", "--locked"], &[])
    } else {
        println(f!(
            "Install cargo binaries used by `do` after confirmation.\n  ACCEPT:\n   {} install --execute\n\n  TO RUN:",
            do_cmd()
        ));
        println("   rustup toolchain install nightly");
        println("   rustup component add rustfmt");
        println("   rustup component add clippy");
        println("   cargo install cargo-expand");
        println("   cargo install cargo-about");
        println("   cargo install cargo-semver-checks --locked");
    }
}

// do doc [-o, --open] [<cargo-doc-args>]
//        [-s, --serve]
//        [--readme <crate>..]
//        [--readme-examples <example>..]
//        [--skip-deadlinks]
//
//    Generate documentation for zng crates.
//
// USAGE:
//     doc -o
//         Generate docs, then open the `zng` crate on the browser.
//     doc -s -o
//         Generate docs, then start `basic-http-server` on the docs and open
//         the served URL on the browser.
//
//         Note: `basic-http-server` can be installed with cargo,
//                it is not installed by `do install`.
//     doc --readme
//         Update READMEs tagged with `<!-- do doc --readme $tag -->` in all publish crates.
//         Tags:
//            header: Replaces the next paragraph with the shared header.
//            features: Replaces or insert the next section with the `## Cargo Features`.
//     doc --readme-examples
//         Update the examples/README.md file. Collects screenshots.
fn doc(mut args: Vec<&str>) {
    if take_flag(&mut args, &["--readme"]) {
        readme_gen::generate(args);
        return;
    }

    if take_flag(&mut args, &["--readme-examples"]) {
        readme_gen::generate_examples(args);
        return;
    }

    let custom_open = if args.contains(&"--manifest-path") {
        if let Some(open) = args.iter_mut().find(|a| **a == "-o") {
            *open = "--open";
        }
        false
    } else {
        take_flag(&mut args, &["-o", "--open"])
    };

    let skip_deadlinks = take_flag(&mut args, &["--skip-deadlinks"]);

    let serve = take_flag(&mut args, &["-s", "--serve"]);

    let package = take_option(&mut args, &["-p", "--package"], "package").map(|mut p| p.remove(0));

    fn collect_flags(toml_path: &str, package: &str) -> (String, Vec<glob::Pattern>) {
        let mut rustdoc_flags = String::new();
        let mut skip_deadlinks_globs = vec![];

        let toml = match std::fs::read_to_string(toml_path) {
            Ok(p) => p,
            Err(e) => {
                fatal(f!("Cannot read `{toml_path}`. {e}"));
            }
        };

        let mut is_in_args = false;
        let mut is_in_skip = false;
        for line in toml.lines() {
            let line = line.trim();

            let mut clean_push = |arg: &str| {
                let arg = arg.trim_matches(&[' ', '"']);
                if arg.starts_with("doc/") {
                    assert!(!package.is_empty());
                    // quick fix, docs.rs runs in the crate dir, we run in the workspace dir.
                    rustdoc_flags.push_str(&format!("crates/{package}/{arg}"));
                } else {
                    rustdoc_flags.push_str(arg);
                }
                rustdoc_flags.push(' ');
            };

            if line.starts_with("rustdoc-args = ") {
                is_in_args = !line.contains(']');
                let line = line["rustdoc-args = ".len()..].trim().trim_matches('[').trim_matches(']').trim();
                for arg in line.split(',') {
                    clean_push(arg);
                }
            } else if is_in_args {
                is_in_args = !line.contains(']');
                let line = line.trim().trim_matches(']').trim();
                for arg in line.split(',') {
                    clean_push(arg);
                }
            } else if line.starts_with("skip-deadlinks = ") {
                is_in_skip = !line.contains(']');
                let line = line["rustdoc-args = ".len()..].trim().trim_matches('[').trim_matches(']').trim();
                for g in line.split(',') {
                    skip_deadlinks_globs.push(glob::Pattern::new(g.trim_matches(&[' ', '"'])).unwrap());
                }
            } else if is_in_skip {
                is_in_skip = !line.contains(']');
                let line = line.trim().trim_matches(']').trim();
                for g in line.split(',') {
                    skip_deadlinks_globs.push(glob::Pattern::new(g.trim_matches(&[' ', '"'])).unwrap());
                }
            }
        }

        (rustdoc_flags, skip_deadlinks_globs)
    }

    let (global_rustdoc_flags, skip_deadlinks_globs) = collect_flags("Cargo.toml", "");

    let mut found_package = false;
    for member in util::publish_members() {
        if let Some(p) = &package {
            if p != &member.name {
                continue;
            }
            found_package = true;
        }

        let pkg = format!("crates/{}/Cargo.toml", member.name.as_str());
        let (rustdoc_flags, skip_deadlinks_globs) = collect_flags(pkg.as_str(), member.name.as_str());

        if !skip_deadlinks_globs.is_empty() {
            error("skip-deadlinks only supported in workspace");
        }

        let mut env = vec![];
        let full_doc_flags;
        if !rustdoc_flags.is_empty() {
            if let Ok(flags) = std::env::var("RUSTDOCFLAGS") {
                full_doc_flags = format!("{flags} {global_rustdoc_flags} {rustdoc_flags}");
                env.push(("RUSTDOCFLAGS", full_doc_flags.as_str()));
            } else {
                env.push(("RUSTDOCFLAGS", rustdoc_flags.as_str()));
            }
        }

        cmd_env_req(
            "cargo",
            &["doc", "--all-features", "--no-deps", "--package", member.name.as_str()],
            &args,
            &env,
        );
    }

    if let Some(p) = &package {
        if !found_package {
            error(f!("package `{p}` not found"));
        }
    }

    if !skip_deadlinks {
        // cargo doc does not warn about broken links in some cases, just prints `[<code>invalid</code>]`

        // cutout links that also appear in other pages or are from downstream types
        let cutout =
            regex::Regex::new(r#"id="(?:deref-met|trait-imp|synthetic-imp|blanket-imp|modules|structs|enums|statics|traits|functions).*""#)
                .unwrap();
        let broken_link1 = regex::Regex::new(r"\[<code>.+?</code>\]").unwrap();
        let broken_link2 = regex::Regex::new(r#"<a href="(\w+?::\w+?.+?)"><code>(.+?)</code>"#).unwrap();
        for html_path in util::glob("target/doc/**/*.html") {
            if skip_deadlinks_globs.iter().any(|g| g.matches(&html_path)) {
                continue;
            }

            let html = std::fs::read_to_string(&html_path).unwrap();
            let cutout = if let Some(m) = cutout.find(&html) { m.start() } else { html.len() };
            let html = &html[..cutout];

            let matches1: Vec<_> = broken_link1.find_iter(&html).map(|m| m.as_str()).collect();
            let matches2: Vec<_> = broken_link2
                .captures_iter(&html)
                .map(|m| (m.get(1).unwrap().as_str(), m.get(2).unwrap().as_str()))
                .collect();

            let mut msg = String::new();
            if !matches1.is_empty() || !matches2.is_empty() {
                msg = format!("deadlinks in `{}`:\n", &html_path["target".len()..]);
            }
            let mut sep = "";
            for m in matches1.iter() {
                use std::fmt::*;
                write!(
                    &mut msg,
                    "{sep}    {}",
                    m.replace("<code>", "`")
                        .replace("</code>", "`")
                        .replace("&lt;", "<")
                        .replace("&gt;", ">")
                        .replace("&amp;", "&")
                )
                .unwrap();
                sep = "\n";
            }
            for (path, label) in matches2.iter() {
                use std::fmt::*;
                write!(
                    &mut msg,
                    "{sep}    [`{}`]: {}",
                    label.replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&"),
                    path
                )
                .unwrap();
                sep = "\n";
            }

            if !msg.is_empty() {
                error(msg);
            }
        }
    }

    let server = if serve {
        Some(std::thread::spawn(|| {
            let root = std::env::current_dir().unwrap().join("target/");
            if let Err(e) = std::process::Command::new("basic-http-server").arg(root).status() {
                error(f!(
                    "couldn't serve docs: {e}\n\nYou can install the server with the command:\ncargo install basic-http-server"
                ));
            }
        }))
    } else {
        None
    };

    if custom_open {
        // Open the main crate.
        // based on https://github.com/rust-lang/cargo/blob/master/src/cargo/ops/cargo_doc.rs
        let path = if serve {
            // `basic-http-server` default.
            "http://127.0.0.1:4000/doc/zng/index.html".to_owned()
        } else {
            std::env::current_dir()
                .unwrap()
                .join("target/doc/zng/index.html")
                .display()
                .to_string()
        };
        match std::env::var_os("BROWSER") {
            Some(browser) => {
                if let Err(e) = std::process::Command::new(&browser).arg(path).status() {
                    error(f!("couldn't open docs with {}: {e}", browser.to_string_lossy()));
                }
            }
            None => {
                if let Err(e) = opener::open(&path) {
                    error(f!("couldn't open docs, {e:?}"));
                }
            }
        };
    }

    if let Some(s) = server {
        let _ = s.join();
    }
}

// do test, t [-u, --unit <function-path>]
//            [-t, --test <integration-test-name>]
//            [-m, --macro <file-path-pattern>]
//            [--nextest]
//            [--render [--save] [FILTER]]
//            <cargo-test-args>
//
//    Run all tests in root workspace and macro tests.
// USAGE:
//     test -u test::path::function
//        Run tests that partially match the Rust item path.
//     test -u --all
//        Run all unit tests.
//     test -t focus
//        Run all integration tests in the named test.
//     test -t --all
//        Run all integration tests.
//     test -m property/*
//        Run macro tests that match the file pattern in `tests/macro-tests/cases/`.
//     test -m --all
//        Run all macro tests.
//     test --doc
//        Run doc tests.
//     test --render
//        Run render tests.
//     test
//        Run all unit, integration, doc, render, and macro tests.
//     test --nextest
//        Run all unit and integration using 'nextest'; doc and macro tests using 'test'.
fn test(mut args: Vec<&str>) {
    let nightly = if take_flag(&mut args, &["+nightly"]) { "+nightly" } else { "" };
    let env = &[("RUST_BACKTRACE", "full")];

    if let Some(unit_tests) = take_option(&mut args, &["-u", "--unit"], "<unit-test-name>") {
        // unit tests:

        let t_args = vec![nightly, "test", "--package", "zng*", "--lib", "--no-fail-fast", "--all-features"];

        if unit_tests.contains(&"--all") || unit_tests.contains(&"*") || unit_tests.contains(&"-a") {
            cmd_env("cargo", &t_args, &args, env);
        } else {
            for test_name in unit_tests {
                let mut t_args = t_args.clone();
                t_args.push(test_name);
                cmd_env("cargo", &t_args, &args, env);
            }
        }
    } else if let Some(int_tests) = take_option(&mut args, &["-t", "--test"], "<integration-test-name>") {
        // integration tests:

        let mut t_args = vec![
            nightly,
            "test",
            "--package",
            "integration-tests",
            "--no-fail-fast",
            "--all-features",
        ];

        if !int_tests.contains(&"--all") && !int_tests.contains(&"-a") && !int_tests.contains(&"*") {
            for it in int_tests {
                t_args.push("--test");
                t_args.push(it);
            }
        }

        cmd_env("cargo", &t_args, &args, env);
    } else if take_flag(&mut args, &["-m", "--macro"]) {
        // macro tests:

        if args.len() != 1 {
            error("expected pattern, use do test --macro --all to run all macro tests");
        } else {
            let rust_flags = std::env::var("RUSTFLAGS")
                .unwrap_or_default()
                .replace("--deny=warnings", "")
                .replace("-D warnings", "")
                .replace("-Dwarnings", "");
            cmd_env(
                "cargo",
                &["run", "--package", "macro-tests"],
                &[],
                &[
                    ("RUSTFLAGS", rust_flags.as_str()),
                    (
                        "DO_TASKS_TEST_MACRO",
                        if args[0] == "--all" || args[0] == "-a" { "*" } else { args[0] },
                    ),
                ],
            );

            let mut changes = vec![];
            for m in util::git_modified() {
                if let Some(ext) = m.extension() {
                    if ext == "stderr" && m.starts_with("tests/macro-tests/cases") {
                        error(format!("macro test `{}` modified", m.display()));
                        changes.push(m);
                    }
                }
            }
            if !changes.is_empty() {
                for m in &changes {
                    util::print_git_diff(&m);
                }
                std::thread::sleep(std::time::Duration::from_millis(100)); // help GitHub log sync prints.
                fatal(format!("{} macro tests modified, review and commit", changes.len()));
            }
        }
    } else if take_flag(&mut args, &["--render"]) {
        // render tests:

        cmd("cargo", &["run", "--package", "render-tests", "--"], &args);
    } else if take_flag(&mut args, &["--examples"]) {
        // all examples

        cmd_env("cargo", &[nightly, "test", "--package", "examples", "--examples"], &args, env);
    } else if let Some(examples) = take_option(&mut args, &["--example"], "<NAME>") {
        // some examples

        let mut e_args = vec![nightly, "--package", "examples"];
        for e in examples {
            e_args.extend(&["--example", e]);
        }
        cmd_env("cargo", &e_args, &args, env);
    } else {
        // other, mostly handled by cargo.

        let all = args.is_empty();

        if !all && args.contains(&"--doc") && !args.contains(&"-p") && !args.contains(&"--package") {
            version_doc_sync::check();
        }

        if take_flag(&mut args, &["--nextest"]) {
            if !args.contains(&"--config-file") {
                let cfg_file = "target/tmp/do-nextest-config.toml";
                std::fs::create_dir_all("target/tmp/").unwrap();
                std::fs::write(
                    cfg_file,
                    b"[profile.default]\nslow-timeout = { period = \"60s\", terminate-after = 3 }",
                )
                .unwrap();
                args.push("--config-file");
                args.push(cfg_file);
            }
            cmd_env(
                "cargo",
                &[nightly, "nextest", "run", "--no-fail-fast", "--all-features", "--workspace"],
                &args,
                env,
            );
        } else {
            cmd_env(
                "cargo",
                &[nightly, "test", "--no-fail-fast", "--all-features", "--workspace"],
                &args,
                env,
            );
        }

        if all {
            // if no args we run everything.
            version_doc_sync::check();
            test(vec!["--macro", "--all"]);
            test(vec!["--render"]);
        }
    }
}

// do run, r EXAMPLE [-b, --backtrace] [--release-lto] [<cargo-run-args>]
//    Run an example in ./examples.
// USAGE:
//     run some_example
//        Runs the example in debug mode.
//     run some_example --release-lto
//        Runs the example in release with LTO mode.
//     run some_example --backtrace
//        Runs the "some_example" with `RUST_BACKTRACE=1`.
//     run --all
//        Builds all examples then runs them one by one.
fn run(mut args: Vec<&str>) {
    let trace = if take_flag(&mut args, &["-b", "--backtrace"]) {
        ("RUST_BACKTRACE", "full")
    } else {
        ("", "")
    };

    if take_flag(&mut args, &["*", "-a", "--all"]) {
        let release = args.contains(&"--release") || args.contains(&"--release-lto");

        let release: &[&str] = if release {
            if args.contains(&"--release-lto") {
                &["--profile", "release-lto"]
            } else {
                &["--release"]
            }
        } else {
            &[""]
        };
        let mut build_args = vec!["build", "--package", "examples", "--examples"];
        build_args.extend(release);
        cmd_env("cargo", &build_args, &[], &[trace]);
        for example in examples() {
            let mut ex_args = vec!["run", "--package", "examples", "--example", &example];
            ex_args.extend(release);
            cmd_env("cargo", &ex_args, &[], &[trace]);
        }
    } else {
        if take_flag(&mut args, &["--release-lto"]) {
            args.push("--profile");
            args.push("release-lto");
        }

        cmd_env("cargo", &["run", "--package", "examples", "--example"], &args, &[trace]);
    }
}

// do expand [-p <crate>] [<ITEM-PATH>] [-r, --raw] [-e, --example <example>]
//           [-m, --macro [-p, -pass <pass-test-name>] [-f, --fail <fail-test-name>]]
//           [<cargo-expand-args>|<cargo-args>]
//    Run "cargo expand" OR if raw is enabled, runs the unstable "--pretty=expanded" check.
// FLAGS:
//     --dump   Write the expanded Rust code to "dump.rs".
// USAGE:
//     expand -p crate-name item::path
//        Prints only the specified item in the crate from workspace.
//     expand -e "example"
//        Prints the example.
//     expand --raw
//        Prints the entire main crate, including macro_rules!.
//     expand --macro -p pass_test_name
//        Prints the macro test cases that match.
fn expand(mut args: Vec<&str>) {
    if args.iter().any(|&a| a == "-m" || a == "--macro") {
        // Expand macro test, we need to run the test to load the bins
        // in the trybuild test crate. We also test in nightly because
        // expand is in nightly.

        let mut test_args = args.clone();
        test_args.insert(0, "+nightly");
        test(test_args);

        TaskInfo::set_stdout_dump("dump.rs");
        for (bin_name, path) in macro_test_cases() {
            let i = path.find("tests").unwrap_or_default();
            println(f!("\n//\n// {}\n//\n", &path[i..]));
            cmd(
                "cargo",
                &[
                    "expand",
                    "--manifest-path",
                    "target/tests/build-tests/Cargo.toml",
                    "--bin",
                    &bin_name,
                    "--all-features",
                ],
                &[],
            );
        }
    } else if take_flag(&mut args, &["-e", "--example"]) {
        TaskInfo::set_stdout_dump("dump.rs");

        if take_flag(&mut args, &["-r", "--raw"]) {
            cmd(
                "cargo",
                &[
                    "+nightly",
                    "rustc",
                    "--profile=check",
                    "--package",
                    "examples",
                    "--example",
                    args.first().unwrap_or(&""),
                    "--",
                    "-Zunpretty=expanded",
                ],
                &[],
            )
        } else {
            cmd("cargo", &["expand", "--package", "examples", "--example"], &args);
        }
    } else {
        TaskInfo::set_stdout_dump("dump.rs");
        if !args.contains(&"-p") && !args.contains(&"--package") {
            error("expected crate name");
        } else if take_flag(&mut args, &["-r", "--raw"]) {
            let p = take_option(&mut args, &["-p", "--package"], "<crate-name>").unwrap();
            cmd(
                "cargo",
                &[
                    "+nightly",
                    "rustc",
                    "--profile=check",
                    "--package",
                    p[0],
                    "--",
                    "-Zunpretty=expanded",
                ],
                &args,
            );
        } else if let Some(p) = take_option(&mut args, &["-p", "--package"], "<crate-name>") {
            cmd("cargo", &["expand", "--all-features", "-p", p[0]], &args);
        } else {
            cmd("cargo", &["expand", "--lib", "--tests", "--all-features"], &args);
        }
    }
}

// do fmt, f [--check <cargo-fmt-args>] [-- <rustfmt-args>]
//    Format workspace, macro test samples, test-crates and the tasks script.
fn fmt(mut args: Vec<&str>) {
    if take_flag(&mut args, &["--check"]) {
        cmd("cargo", &["fmt", "--", "--check"], &args);
        let cases = all_ext("tests/macro-tests/cases", "rs");
        let cases_str: Vec<_> = cases.iter().map(|s| s.as_str()).collect();
        cmd("rustfmt", &["--edition", "2021", "--check"], &cases_str);
        for tool_crate in top_cargo_toml("tools") {
            cmd("cargo", &["fmt", "--manifest-path", &tool_crate, "--", "--check"], &args);
        }
    } else {
        print("    fmt workspace ... ");
        cmd("cargo", &["fmt"], &args);
        println("done");

        print("    fmt tests/macro-tests/cases/**/*.rs ... ");
        let cases = all_ext("tests/macro-tests/cases", "rs");
        let cases_str: Vec<_> = cases.iter().map(|s| s.as_str()).collect();
        cmd("rustfmt", &["--edition", "2021"], &cases_str);
        println("done");

        print("    fmt tools ... ");
        for tool_crate in top_cargo_toml("tools") {
            cmd("cargo", &["fmt", "--manifest-path", &tool_crate], &args);
        }
        println("done");
    }
}

// do check, c
//    Runs clippy on the workspace.
fn check(args: Vec<&str>) {
    cmd("cargo", &["clippy", "--no-deps", "--tests", "--workspace", "--examples"], &args);
}

// do build, b [-e, --example] [--examples] [-t, --timings] [--release-lto] [-Z*] [<cargo-build-args>]
//    Compile the main crate and its dependencies.
// USAGE:
//    build -e <example>
//       Compile the example.
//    build --examples
//       Compile all examples.
//    build -p <crate> -t
//       Compile crate and report in "target/cargo-timings"
fn build(mut args: Vec<&str>) {
    let mut nightly = if take_flag(&mut args, &["+nightly"]) { "+nightly" } else { "" };

    let mut rust_flags = ("", String::new());

    let mut prev_z = false;
    args.retain(|f| {
        if f.starts_with("-Z") {
            prev_z = true;

            if rust_flags.0.is_empty() {
                rust_flags = ("RUSTFLAGS", std::env::var("RUSTFLAGS").unwrap_or_default());
            }
            rust_flags.1.push(' ');
            rust_flags.1.push_str(f);

            if nightly.is_empty() {
                nightly = "+nightly";
            }

            false
        } else if prev_z && !f.starts_with('-') {
            prev_z = false;
            rust_flags.1.push('=');
            rust_flags.1.push_str(f);
            false
        } else {
            prev_z = false;
            true
        }
    });
    let rust_flags = &[(rust_flags.0, rust_flags.1.as_str())];

    let mut cargo_args = vec![nightly, "build"];

    if take_flag(&mut args, &["-t", "--timings"]) {
        cargo_args.push("--timings");
    }

    if take_flag(&mut args, &["-e", "--example"]) {
        cargo_args.extend(&["--package", "examples", "--example"]);
    } else if take_flag(&mut args, &["--examples"]) {
        cargo_args.extend(&["--package", "examples", "--examples"]);
    }

    if take_flag(&mut args, &["--release-lto"]) {
        args.push("--profile");
        args.push("release-lto");
    }

    cmd_env("cargo", &cargo_args, &args, rust_flags);
}

// do prebuild
//    Compile the pre-build `zng-view` release.
fn prebuild(mut args: Vec<&str>) {
    if let Some(t) = args.iter_mut().find(|a| *a == &"-t") {
        *t = "--timings";
    }
    let profile = if let Some(p) = take_option(&mut args, &["--profile"], "profile") {
        p[0]
    } else {
        "prebuild"
    };

    cmd(
        "cargo",
        &["build", "-p", "zng-view", "--profile", profile, "--features", "bundle_licenses"],
        &args,
    );

    let files = cdylib_files(format!("target/{}/zng_view", if profile == "dev" { "debug" } else { profile }));

    if files.is_empty() {
        error("no pre-built `cdylib` output found");
        return;
    }

    for file in files {
        let target = format!("crates/zng-view-prebuilt/lib/{}", file.file_name().unwrap().to_string_lossy());
        if let Err(e) = std::fs::copy(&file, &target) {
            error(f!("failed to copy pre-build lib `{}` to `{target}`, {e}", file.display()))
        }
    }

    // test build
    cmd("cargo", &["build", "-p", "zng-view-prebuilt", "--release"], &[]);
}

// do clean [--tools] [--workspace] [--release-lto] [--prebuild] [<cargo-clean-args>]
//    Remove workspace, test crates, profile crates and tools target directories.
// USAGE:
//    clean --tools
//       Remove only the target directories in ./tools.
//    clean --workspace
//       Remove only the target directory of the root workspace.
//    clean --doc
//       Remove only the doc files from the target directories.
//    clean --release
//       Remove only the release files from the target directories.
//    clean --temp, --tmp
//       Remove the temp files from the target workspace target directory.
fn clean(mut args: Vec<&str>) {
    let tools = take_flag(&mut args, &["--tools"]);
    let workspace = take_flag(&mut args, &["--workspace"]);
    let temp = take_flag(&mut args, &["--temp", "--tmp"]);
    let all = !tools && !workspace && !temp;

    let release_lto = take_flag(&mut args, &["--release-lto"]);
    let prebuild = take_flag(&mut args, &["--prebuild"]);

    if all || workspace {
        let mut args = args.clone();
        if prebuild {
            args.push("--profile");
            args.push("prebuild");
        } else if release_lto {
            args.push("--profile");
            args.push("release-lto");
        }

        cmd("cargo", &["clean"], &args);
    } else if temp {
        match std::fs::remove_dir_all("target/tmp") {
            Ok(_) => match std::fs::create_dir("target/tmp") {
                Ok(_) => println("removed `target/tmp` contents"),
                Err(_) => println("removed `target/tmp`"),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => println("did not find `target/tmp`"),
            Err(e) => error(f!("failed to cleanup temp, {e}")),
        }
    }

    if all || tools {
        for tool_ in top_cargo_toml("test-crates") {
            if tool_.contains("/cargo-do/") {
                continue;
            }
            cmd("cargo", &["clean", "--manifest-path", &tool_], &args);
        }

        // external because it will delete self.
        let manifest_path = std::env::current_exe()
            .unwrap()
            .canonicalize()
            .unwrap()
            .parent()
            .unwrap()
            .join("../../Cargo.toml")
            .canonicalize()
            .unwrap()
            .display()
            .to_string();
        cmd_external("cargo", &["clean", "--manifest-path", &manifest_path], &args);
    }
}

// do asm [r --rust] [--debug] [<FN-PATH>] [<cargo-asm-args>]
//    Run "cargo asm" after building.
// FLAGS:
//     --dump   Write the assembler to "dump.asm".
// USAGE:
//    asm <FN-PATH>
//        Print assembler for the function, build in release, or list all functions matched.
//    asm --debug <FN-PATH>
//        Print assembler for the function, or list all functions matched.
//    asm -r <FN-PATH>
//        Print source Rust code interleaved with assembler code.
fn asm(mut args: Vec<&str>) {
    let manifest_path = take_option(&mut args, &["--manifest-path"], "<Cargo.toml>").unwrap_or_default();
    let build_type = take_option(&mut args, &["--build-type"], "<debug, release>").unwrap_or_default();
    let debug = take_flag(&mut args, &["--debug"]);

    let mut asm_args = vec!["asm"];

    if debug {
        asm_args.push("--build-type");
        asm_args.push("debug");
    } else if let Some(t) = build_type.first() {
        asm_args.push("--build-type");
        asm_args.push(t);
    }

    if take_flag(&mut args, &["-r", "--rust"]) {
        asm_args.push("--rust");
    }

    if let Some(p) = manifest_path.first() {
        asm_args.push("--manifest-path");
        asm_args.push(p);
    }

    {
        if TaskInfo::dump() {
            asm_args.push("--no-color");
            TaskInfo::set_stdout_dump("dump.asm");
        }
    }

    util::do_after(10, || {
        println(r#"Awaiting "cargo asm", this can take a while..."#);
    });

    cmd("cargo", &asm_args, &args);
}

fn rust_analyzer_run(args: Vec<&str>) {
    if let Some(&"check") = args.first() {
        cmd("cargo", &["clippy", "--no-deps"], &args[1..]);
    } else {
        cmd("cargo", &args, &[]);
    }
}

fn rust_analyzer_check(mut args: Vec<&str>) {
    if !settings_path().join(".rust_analyzer_disabled").exists() {
        args.push("--message-format=json");
        check(args);
    }
}

// do ra_check [--on,--off]
//    Enables or disables rust-analyzer check.
// USAGE:
//    ra_check --on
//        Enables rust-analyzer check.
//    ra_check --off
//        Disables rust-analyzer check.
//    ra_check
//        Toggles rust-analyzer check.
fn ra_check(mut args: Vec<&str>) {
    let path = settings_path().join(".rust_analyzer_disabled");

    let enable = if take_flag(&mut args, &["--on"]) {
        true
    } else if take_flag(&mut args, &["--off"]) {
        false
    } else {
        path.exists()
    };

    if enable {
        if let Err(e) = std::fs::remove_file(path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                panic!("{e:?}")
            }
        }
        println("rust-analyzer check is enabled");
    } else {
        let _ = std::fs::File::create(path).unwrap();
        println("rust-analyzer check is disabled");
    }
}

// do publish [--list]
//            [--diff [-g glob] --all]
//            [--bump <minor|patch> <CRATE..> --diff --all --dry-run]
//            [--check]
//            [--test]
//    Manage crate versions and publish.
// USAGE:
//    publish --list
//       Print all publishable crates and dependencies.
//    publish --diff
//       Print all publishable crates that changed since last publish.
//    publish --diff --all
//       Print all changed files in publishable crates.
//    publish --bump patch "crate1" "crate2"
//       Increment the patch version of the named crates and of dependents.
//    publish --bump minor "crate1" "crate2"
//       Increment the minor of the named crates only.
//    publish --bump patch --diff
//       Increment the patch version of the --diff crates and of dependents.
//    publish --check
//       Print all publishable crates that need to be published.
//    publish --test
//       Dry run cargo publish for all crates that need to be published.
//    publish --execute
//       Publish all crates that need to be published.
//    publish --execute --no-burst
//       Publish all crates that need to be published with no rate burst.
fn publish(mut args: Vec<&str>) {
    if take_flag(&mut args, &["--list"]) {
        for member in &util::publish_members() {
            print(f!("{member}\n"));
        }
    } else if let Some(values) = take_option(&mut args, &["--bump"], "minor|patch crate") {
        let mut bump_deps = false;
        let bump = match values[0] {
            "patch" => {
                fn bump(v: &mut (u32, u32, u32)) {
                    v.2 += 1;
                }
                bump_deps = true;
                bump
            }
            "minor" => {
                fn bump(v: &mut (u32, u32, u32)) {
                    v.1 += 1;
                    v.2 = 0;
                }
                bump
            }
            unknown => fatal(f!("unknown bump level {unknown:?}")),
        };
        let dry_run = take_flag(&mut args, &["--dry-run"]);

        let all_crates = take_flag(&mut args, &["--all"]);
        let diff_crates = take_flag(&mut args, &["--diff"]);

        let mut crates = args;
        let members = util::publish_members();
        let git_diff;
        if all_crates {
            crates = members.iter().map(|m| m.name.as_str()).collect();
        } else if diff_crates {
            let published_tag = format!("v{}", util::crates_io_latest("zng"));
            let members = util::publish_members();
            git_diff = util::get_git_diff(&published_tag, "main");

            for line in git_diff.lines() {
                if let Some(name) = line.strip_prefix("crates/") {
                    let name = name.split('/').next().unwrap();
                    if members.iter().any(|m| m.name == name) {
                        crates.push(name);
                    }
                }
            }
        }

        if crates.is_empty() {
            fatal("missing at least one crate name");
        }
        if let Some(c) = crates.iter().find(|c| c.starts_with('-')) {
            fatal(f!("expected only crate names, found {:?}", c));
        }

        // include dependents.
        if !all_crates && bump_deps {
            let mut dependents_start = crates.len();
            let mut search = crates.clone();
            loop {
                for member in &members {
                    if member.dependencies.iter().any(|d| search.iter().any(|n| *n == &d.name)) {
                        if !crates.iter().any(|c| c == &member.name) {
                            crates.push(&member.name);
                        }
                    }
                }
                if dependents_start == crates.len() {
                    break;
                } else {
                    search = crates[dependents_start..].to_vec();
                    dependents_start = crates.len();
                }
            }
        }

        if let Some(i) = crates.iter().position(|c| *c == "zng-view-prebuilt") {
            // "zng-view-prebuilt" version is always equal "zng" version.
            assert!(crates.contains(&"zng"));
            crates.remove(i);
        }

        let mut new_versions = std::collections::HashMap::new();

        for crate_ in &crates {
            let member = members.iter().find(|m| &m.name == crate_).unwrap();
            let mut new_version = member.version;
            bump(&mut new_version);
            new_versions.insert(member.name.as_str(), new_version);
        }

        if crates.contains(&"zng") {
            let mut new_version = members.iter().find(|m| m.name == "zng").unwrap().version;
            let member = members.iter().find(|m| m.name == "zng-view-prebuilt").unwrap();
            bump(&mut new_version);
            new_versions.insert(member.name.as_str(), new_version);
        }

        for member in &members {
            member.write_versions(&new_versions, dry_run);
        }

        if !dry_run {
            version_doc_sync::fix();

            if crates.contains(&"zng") {
                version_doc_sync::close_changelog();
            }
        }
    } else if take_flag(&mut args, &["--diff"]) {
        let published_tag = format!("v{}", util::crates_io_latest("zng"));
        let members = util::publish_members();
        let git_diff = util::get_git_diff(&published_tag, "main");
        let mut changed = std::collections::HashMap::new();

        let glob = take_option(&mut args, &["-g"], "<glob>").map(|g| glob::Pattern::new(g[0]).unwrap());
        let all = take_flag(&mut args, &["--all"]);

        for line in git_diff.lines() {
            if let Some(g) = &glob {
                if !g.matches(line) {
                    continue;
                }
            }

            if let Some(name) = line.strip_prefix("crates/") {
                let name = name.split('/').next().unwrap();
                if members.iter().any(|m| m.name == name) {
                    let changes: &mut Vec<&str> = changed.entry(name).or_default();
                    changes.push(line);
                }
            }
        }

        let mut sep = "";
        for m in members {
            if let Some(c) = changed.get(&m.name.as_str()) {
                if all {
                    print(f!("{sep}"));
                    sep = "\n";

                    for line in c {
                        print(f!("{line}\n"));
                    }
                } else {
                    print(f!("{}\n", m.name));
                }
            }
        }
    } else if take_flag(&mut args, &["--check"]) {
        let members = util::publish_members();
        let mut count = 0;
        for member in &members {
            let published_ver = util::crates_io_latest(member.name.as_str());
            let current_ver = format!("{}.{}.{}", member.version.0, member.version.1, member.version.2);

            if published_ver != current_ver {
                print(f!("{} {} -> {}\n", member.name, published_ver, current_ver));
                count += 1;
            }
        }

        print(f!("{} of {} crates out of sync with crates.io", count, members.len()));
    } else if take_flag(&mut args, &["--test"]) {
        let members = util::publish_members();
        for member in &members {
            let published_ver = util::crates_io_latest(member.name.as_str());
            let current_ver = format!("{}.{}.{}", member.version.0, member.version.1, member.version.2);

            let exclude = [
                // curl download probably fails because we are testing the new version, not published.
                "zng-view-prebuilt",
            ];
            if published_ver != current_ver && !exclude.contains(&member.name.as_str()) {
                if member.dependencies.is_empty() {
                    cmd(
                        "cargo",
                        &["publish", "--dry-run", "--allow-dirty", "--package", member.name.as_str()],
                        &[],
                    );
                } else {
                    // don't know how to dry-run ignoring missing dependencies,
                    // this at least tests if features are enabled correctly.
                    cmd("cargo", &["check", "--package", member.name.as_str()], &[]);
                }
                cmd("cargo", &["clean"], &[]);
            }
        }
    } else if take_flag(&mut args, &["--execute"]) {
        use std::time::*;

        let members = util::publish_members();
        let mut delay = Duration::ZERO;
        let mut burst = 30;

        if take_flag(&mut args, &["--no-burst"]) {
            burst = 0;
        }

        let mut count = 0;

        for member in &members {
            let published_ver = util::crates_io_latest(member.name.as_str());
            let current_ver = format!("{}.{}.{}", member.version.0, member.version.1, member.version.2);

            if published_ver != current_ver {
                let test_start = Instant::now();
                cmd_req("cargo", &["publish", "--package", member.name.as_str(), "--dry-run"], &[]);

                delay = delay.saturating_sub(test_start.elapsed());
                if delay > Duration::ZERO {
                    print(f!("awaiting rate limit, will publish {:?} in {:?}\n", member.name, delay));
                    std::thread::sleep(delay);
                }

                cmd_req("cargo", &["publish", "--package", member.name.as_str(), "--no-verify"], &[]);
                count += 1;

                if published_ver.is_empty() {
                    cmd_req("cargo", &["owner", "--add", "github:zng-ui:owners", member.name.as_str()], &[]);
                }

                cmd("cargo", &["clean"], &[]);

                // https://github.com/rust-lang/crates.io/blob/main/src/rate_limiter.rs
                let extra = Duration::from_secs(1);
                delay = if published_ver.is_empty() {
                    // 10 minutes for new crates
                    burst = 0;
                    Duration::from_secs(10 * 60) + extra
                } else if burst > 0 {
                    burst -= 1;
                    Duration::ZERO
                } else {
                    // 1 minute for upgrades
                    Duration::from_secs(60) + extra
                };
            }
        }

        print(f!("published {} crates.\n", count));
    }
}

// do semver_check
//    Runs cargo semver-checks for each published crate.
fn semver_check(args: Vec<&str>) {
    for member in util::publish_members() {
        let published_ver = util::crates_io_latest(member.name.as_str());

        if !published_ver.is_empty() && !member.name.ends_with("-proc-macros") && !member.name.ends_with("-scraper") {
            println(member.name.as_str());
            cmd("cargo", &["semver-checks", "--package", member.name.as_str()], &args);
            for t in util::glob("target/semver-checks/*/target") {
                let _ = std::fs::remove_dir_all(t);
            }
        }
    }
}

// used by `workflows/release.yml`
fn publish_version_tag(mut args: Vec<&str>) {
    let version = util::crate_version("zng");
    let tag = format!("v{version}");

    if git_tag_exists(&tag) {
        fatal(f!("git tag `{tag}` already exists, bump zng version and retry"))
    }

    if take_flag(&mut args, &["--execute"]) {
        util::fix_git_config_name_email();
        cmd_req("git", &["tag", &tag, "-m", &format!("zng version {version}")], &[]);
        cmd_req("git", &["push", "origin", &tag], &[]);
    }
    print(f!("tag={tag}\n"));
}

// used by `workflows/release.yml`
fn comment_feature(mut args: Vec<&str>) {
    use std::fmt::*;

    let uncomment = take_flag(&mut args, &["-u", "--uncomment"]);
    let cargo = args[0];
    let feature = args[1];
    let feature_co = format!("# {feature}");

    let (find, replace) = if uncomment {
        (feature_co.as_str(), feature)
    } else {
        (feature, feature_co.as_str())
    };

    match std::fs::read_to_string(cargo) {
        Ok(file) => {
            let mut out = String::new();
            let mut in_features = false;
            let mut replaced = false;

            for line in file.lines() {
                if line == "[features]" {
                    in_features = true;
                } else if line.starts_with('[') && line.ends_with(']') {
                    in_features = false;
                } else if in_features && line.starts_with(find) {
                    write!(&mut out, "{replace}{}\n", &line[find.len()..]).unwrap();
                    replaced = true;
                    continue;
                }

                write!(&mut out, "{line}\n").unwrap();
            }

            if !replaced {
                fatal(f!("did not find `{find}` in `{cargo}`\n"));
            }

            std::fs::write(cargo, out.as_bytes()).unwrap();
        }
        Err(e) => {
            error(e);
        }
    }
}

// used by `workflows/release.yml`
fn latest_release_changes(args: Vec<&str>) {
    let changelog = match std::fs::read_to_string("CHANGELOG.md") {
        Ok(c) => c,
        Err(e) => fatal(f!("failed to read CHANGELOG.md, {e}")),
    };

    let mut changes = String::new();
    let mut started = false;
    for line in changelog.lines().skip(1) {
        if line.starts_with("# ") {
            if started {
                break;
            }
            started = true;
        } else if started {
            changes.push_str(line);
            changes.push('\n');
        }
    }
    let changes = changes.trim();

    if let Some(out) = args.first() {
        if let Err(e) = std::fs::write(out, changes.as_bytes()) {
            fatal(f!("failed to write changes, {e}"));
        }
    } else {
        println(changes)
    }
}

// do just
//    Install a shell script at the workspace root so that you only need to call `do`
fn just(_: Vec<&str>) {
    #[cfg(windows)]
    std::fs::write("do.bat", include_bytes!("do-script.bat")).unwrap_or_else(|e| util::fatal(e));

    #[cfg(not(windows))]
    {
        std::fs::write("do", include_bytes!("do-script.sh")).unwrap_or_else(|e| util::fatal(e));
        println!("$ chmod u+x do");
        let chmod = std::process::Command::new("chmod")
            .arg("u+x")
            .arg("do")
            .status()
            .unwrap_or_else(|e| util::fatal(e));
        if !chmod.success() {
            util::set_exit_with_error();
        }
    }
}

// do version
//    Prints version of Rust and components.
// USAGE:
//    version --verbose
//       Prints the full versions.
fn version(args: Vec<&str>) {
    cmd("rustc", &["--version"], &args);
    print("\n");
    cmd("cargo", &["version"], &args);
    print("\n");
    cmd("cargo", &["clippy", "--version"], &args);

    if args.contains(&"--verbose") {
        print(f!("\nRUSTFLAGS={}", std::env::var("RUSTFLAGS").unwrap_or_default()));
        print(f!("\nRUSTDOCFLAGS={}", std::env::var("RUSTDOCFLAGS").unwrap_or_default()));
        print(f!("\nCARGO_INCREMENTAL={}", std::env::var("CARGO_INCREMENTAL").unwrap_or_default()));
    }
}

fn ls(args: Vec<&str>) {
    println!("ls {:?}", args);

    for p in util::glob(&format!("{}/**", args[0])) {
        println!("{p}");
    }
}

// do help, --help [task]
//    Prints help for all tasks.
// USAGE:
//    help <task>
//        Prints only the help for the <task>
fn help(mut args: Vec<&str>) {
    println(f!(
        "\n{}{}{} ({} {})",
        c_wb(),
        do_cmd(),
        c_w(),
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    ));

    let specific_task = !args.is_empty();

    if !specific_task {
        println(f!("   {}", env!("CARGO_PKG_DESCRIPTION")));
        println("\nUSAGE:");
        println(f!("    {} TASK [<TASK-ARGS>]", do_cmd()));
        println("\nFLAGS:");
        println(r#"    --dump   Redirect output to "dump.log" or other file specified by task."#);
    }
    print("\nTASKS:");

    // prints lines from this file that start with "// do " and comment lines directly after then.
    let tasks_help = include_str!(concat!(std::env!("OUT_DIR"), "/tasks-help.stdout"));

    let mut skip = false;

    for line in tasks_help.lines() {
        if line.starts_with("--") && line.ends_with("--") {
            if specific_task {
                let name = line.trim_matches('-');
                if let Some(i) = args.iter().position(|a| a == &name) {
                    args.swap_remove(i);
                    skip = false;
                } else {
                    skip = true;
                }
            }
        } else if !skip {
            println(line.replace("%c_wb%", c_wb()).replace("%c_w%", c_w()));
        }
    }

    if specific_task && !args.is_empty() {
        println("\n");
        for t in args {
            error(f!("task `{t}` not found in help"));
        }
    }
}
