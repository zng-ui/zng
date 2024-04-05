mod tests;
mod util;
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
        "publish_version_tag" => publish_version_tag(args),
        "version" => version(args),
        "help" | "--help" => help(args),
        _ => fatal(f!("unknown task {task:?}, `{} help` to list tasks", do_cmd())),
    }

    util::exit_checked();
}

// do install [-a, --accept]
//    Install `do` dependencies after confirmation.
// USAGE:
//     install
//       Shows what commands will run and asks for confirmation.
//     install --accept
//       Runs the installation commands.
fn install(mut args: Vec<&str>) {
    if take_flag(&mut args, &["-a", "--accept"]) {
        cmd("rustup", &["toolchain", "install", "nightly"], &[]);
        cmd("rustup", &["component", "add", "rustfmt"], &[]);
        cmd("rustup", &["component", "add", "clippy"], &[]);
        cmd("cargo", &["install", "cargo-expand"], &[]);
        cmd("cargo", &["install", "cargo-asm"], &[]);
    } else {
        println(f!(
            "Install cargo binaries used by `do` after confirmation.\n  ACCEPT:\n   {} install --accept\n\n  TO RUN:",
            do_cmd()
        ));
        println("   rustup toolchain install nightly");
        println("   rustup component add rustfmt");
        println("   rustup component add clippy");
        println("   cargo install cargo-expand");
        println("   cargo install cargo-asm");
    }
}

// do doc [-o, --open] [<cargo-doc-args>]
//        [-s, --serve]
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
fn doc(mut args: Vec<&str>) {
    let custom_open = if args.contains(&"--manifest-path") {
        if let Some(open) = args.iter_mut().find(|a| **a == "-o") {
            *open = "--open";
        }
        false
    } else {
        take_flag(&mut args, &["-o", "--open"])
    };

    let serve = take_flag(&mut args, &["-s", "--serve"]);

    let package = take_option(&mut args, &["-p", "--package"], "package");
    let mut found_package = false;

    let mut pkgs = util::glob("zng*/Cargo.toml");
    if let Some(i) = pkgs.iter().position(|p| p.ends_with("zng/Cargo.toml")) {
        let last = pkgs.len() - 1;
        pkgs.swap(i, last);
    }
    for pkg in pkgs {
        let toml = match std::fs::read_to_string(&pkg) {
            Ok(p) => p,
            Err(e) => {
                error(e);
                continue;
            }
        };

        let mut name = String::new();
        let mut rustdoc_flags = String::new();
        let mut is_in_args = false;
        let mut is_in_package = false;
        for line in toml.lines() {
            let line = line.trim();

            if line.starts_with('[') {
                is_in_package = line == "[package]";
            }

            if is_in_package && line.starts_with("name = ") {
                name = line["name = ".len()..].trim_matches('"').to_owned();
            }
            if line.starts_with("rustdoc-args = ") {
                is_in_args = !line.contains(']');
                let line = line["rustdoc-args = ".len()..].trim().trim_matches('[').trim_matches(']').trim();
                for arg in line.split(',') {
                    rustdoc_flags.push_str(arg.trim().trim_matches('"'));
                    rustdoc_flags.push(' ');
                }
            } else if is_in_args {
                is_in_args = !line.contains(']');
                let line = line.trim().trim_matches(']').trim();
                for arg in line.split(',') {
                    rustdoc_flags.push_str(arg.trim().trim_matches('"'));
                    rustdoc_flags.push(' ');
                }
            }
        }

        if name.is_empty() {
            error(f!("did not find package name for {pkg}"));
            continue;
        } else if let Some(p) = &package {
            if p[0] != name {
                continue;
            }
            found_package = true;
        }

        let mut env = vec![];
        let full_doc_flags;
        if !rustdoc_flags.is_empty() {
            if let Ok(flags) = std::env::var("RUSTDOCFLAGS") {
                full_doc_flags = format!("{flags} {rustdoc_flags}");
                env.push(("RUSTDOCFLAGS", full_doc_flags.as_str()));
            } else {
                env.push(("RUSTDOCFLAGS", rustdoc_flags.as_str()));
            }
        }

        cmd_env_req("cargo", &["doc", "--all-features", "--no-deps", "--package", &name], &args, &env);
    }

    if let Some(pkg) = &package {
        if !found_package {
            error(f!("did not find package `{}`", &pkg[0]));
            return;
        }
    }

    let server = if serve {
        Some(std::thread::spawn(|| {
            let root = std::env::current_dir().unwrap().join("target/doc/");
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
            "http://127.0.0.1:4000/zng/index.html".to_owned()
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
//     test
//        Run all unit, doc, integration and macro tests.
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
                fatal(format!("{} macro tests modified, review and commit", changes.len()));
            }
        }
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

        if !all && args.contains(&"--doc") {
            tests::version_in_sync();
        }

        cmd_env("cargo", &[nightly, "test", "--no-fail-fast", "--all-features"], &args, env);

        if all {
            // if no args we run everything.
            tests::version_in_sync();
            test(vec!["--macro", "--all"]);
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

            if p[0] == "build-time" {
                cmd(
                    "cargo",
                    &[
                        "+nightly",
                        "rustc",
                        "--profile=check",
                        "--manifest-path",
                        "profile/build-time/Cargo.toml",
                        "--",
                        "-Zunpretty=expanded",
                    ],
                    &args,
                )
            } else {
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
            }
        } else if let Some(p) = take_option(&mut args, &["-p", "--package"], "<crate-name>") {
            if p[0] == "build-time" {
                cmd(
                    "cargo",
                    &["expand", "--all-features", "--manifest-path", "profile/build-time/Cargo.toml"],
                    &args,
                );
            } else {
                cmd("cargo", &["expand", "--all-features", "-p", p[0]], &args);
            }
        } else {
            cmd("cargo", &["expand", "--lib", "--tests", "--all-features"], &args);
        }
    }
}

// do fmt, f [<cargo-fmt-args>] [-- <rustfmt-args>]
//    Format workspace, macro test samples, test-crates and the tasks script.
fn fmt(args: Vec<&str>) {
    print("    fmt workspace ... ");
    cmd("cargo", &["fmt"], &args);
    println("done");

    print("    fmt tests/build/cases/**/*.rs ... ");
    let cases = all_ext("tests/build/cases", "rs");
    let cases_str: Vec<_> = cases.iter().map(|s| s.as_str()).collect();
    cmd("rustfmt", &["--edition", "2021"], &cases_str);
    println("done");

    print("    fmt tools ... ");
    for tool_crate in top_cargo_toml("tools") {
        cmd("cargo", &["fmt", "--manifest-path", &tool_crate], &args);
    }
    println("done");
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
    cmd("cargo", &["build", "-p", "zng-view", "--profile", "prebuild"], &args);

    let files = cdylib_files("target/prebuild/zng_view");

    if files.is_empty() {
        error("no pre-build `cdylib` output found");
        return;
    }

    for file in files {
        let target = format!("zng-view-prebuilt/lib/{}", file.file_name().unwrap().to_string_lossy());
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
            if tool_.contains("/do-tasks/") {
                continue;
            }
            cmd("cargo", &["clean", "--manifest-path", &tool_], &args);
        }

        // external because it will delete self.
        let manifest_path = std::env::current_exe()
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

// do publish [--list,--bump <minor|patch> <CRATE..>, --check, --test]
//    Manage crate versions and publish.
// USAGE:
//    publish --list
//       Print all publishable crates and dependencies.
//    publish --bump minor "crate-name"
//       Increment the minor version of the crates and dependents.
//    publish --bump patch "c" --dry-run
//       Only prints the version changes.
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
        let bump = match values[0] {
            "patch" => {
                fn bump(v: &mut (u32, u32, u32)) {
                    v.2 += 1;
                }
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

        let mut crates = args;
        if crates.is_empty() {
            fatal("missing at least one crate name");
        }
        if let Some(c) = crates.iter().find(|c| c.starts_with('-')) {
            fatal(f!("expected only crate names, found {:?}", c));
        }

        let mut dependents_start = crates.len();
        let mut search = crates.clone();
        let members = util::publish_members();
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

            if published_ver != current_ver {
                // don't know how to dry-run ignoring missing dependencies.
                let mut skip = !member.dependencies.is_empty();
                // build script file because GitHub release is not created yet.
                skip |= member.name == "zng-view-prebuilt";
                if !skip {
                    cmd(
                        "cargo",
                        &["publish", "--dry-run", "--allow-dirty", "--package", member.name.as_str()],
                        &[],
                    );
                }
            }
        }
    } else if take_flag(&mut args, &["--execute"]) {
        use std::time::Duration;

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
                let interval = Duration::from_secs(1);
                while delay > Duration::ZERO {
                    print(f!("\rwaiting rate limit, will publish {:?} in {:?}", member.name, delay));
                    std::thread::sleep(interval.min(delay));
                    delay = delay.saturating_sub(interval);
                }
                print("\r                                                                              \r");

                cmd_req("cargo", &["publish", "--package", member.name.as_str()], &[]);
                count += 1;

                // https://github.com/rust-lang/crates.io/blob/main/src/rate_limiter.rs
                delay = if published_ver.is_empty() {
                    // 10 minutes for new crates
                    burst = 0;
                    Duration::from_secs(10 * 60) + interval
                } else if burst > 0 {
                    burst -= 1;
                    Duration::ZERO
                } else {
                    // 1 minute for upgrades
                    Duration::from_secs(60) + interval
                };
            }
        }

        print(f!("published {} crates.\n", count));
    }
}

// used by `workflows/release-1-test-tag.yml`
fn publish_version_tag(mut args: Vec<&str>) {
    let version = util::zng_version();
    let tag = format!("v{version}");

    if git_tag_exists(&tag) {
        fatal(f!("git tag `{tag}` already exists, bump zng version and retry"))
    }

    if take_flag(&mut args, &["--execute"]) {
        cmd_req("git", &["tag", &tag, "-m", &format!("zng version {version}")], &[]);
        cmd_req("git", &["push", "origin", &tag], &[])
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
