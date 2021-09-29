mod tests;
mod util;
use std::format_args as f;
use util::*;

fn main() {
    let (task, args) = args();

    match task {
        "fmt" | "f" => fmt(args),
        "test" | "t" => test(args),
        "run" | "r" => run(args),
        "doc" => doc(args),
        "expand" => expand(args),
        "build" | "b" => build(args),
        "clean" => clean(args),
        "asm" => asm(args),
        "help" | "--help" => help(args),
        _ => fatal(f!("unknown task {:?}, `{} help` to list tasks", task, DO)),
    }
}

// do doc [-o, --open] [<cargo-doc-args>]
//    Generate documentation for crates in the root workspace.
fn doc(mut args: Vec<&str>) {
    let custom_open = if args.contains(&"--manifest-path") {
        if let Some(open) = args.iter_mut().find(|a| **a == "-o") {
            *open = "--open";
        }
        false
    } else {
        take_flag(&mut args, &["-o", "--open"])
    };

    cmd_env_req(
        "cargo",
        &["+nightly", "doc", "--all-features", "--no-deps", "--workspace"],
        &args,
        &[("RUSTDOCFLAGS", "--cfg doc_nightly")],
    );

    if custom_open {
        // Open the main crate.
        // based on https://github.com/rust-lang/cargo/blob/master/src/cargo/ops/cargo_doc.rs
        let path = std::env::current_dir().unwrap().join("target/doc/zero_ui/index.html");
        match std::env::var_os("BROWSER") {
            Some(browser) => {
                if let Err(e) = std::process::Command::new(&browser).arg(path).status() {
                    error(f!("couldn't open docs with {}: {}", browser.to_string_lossy(), e));
                }
            }
            None => {
                if let Err(e) = opener::open(&path) {
                    error(f!("couldn't open docs, {:?}", e));
                }
            }
        };
    }
}

// do test, t [-w, --workspace] [-u, --unit <unit-test>] [--test-crates]
//            [-b, --build <build-test-pat>] [--OVERWRITE]
//            [<cargo-test-args>]
//    Run all tests in root workspace and ./test-crates.
// USAGE:
//     test -u, --unit <test::path>
//        Run tests that partially match the path in the root workspace.
//     test -w, --workspace
//        Run all tests in root workspace (exclude build_tests and ./test-crates).
//     test -t, --test <integration_test_name>
//        Run the integration test named in the root workspace.
//     test --doc
//        Run all doc tests in the root workspace.
//     test --test-crates
//        Run all the ./test-crates tests.
//     test --build *
//        Run all build tests
//     test -b <build-test-pat>
//        Run build test files that match "./tests/build/<build_test_pat>.rs".
fn test(mut args: Vec<&str>) {
    let nightly = if take_flag(&mut args, &["+nightly"]) { "+nightly" } else { "" };

    if take_flag(&mut args, &["-w", "--workspace"]) {
        // exclude ./test-crates and build_tests
        if !args.iter().any(|a| *a == "--") {
            args.push("--");
        }
        args.push("--skip");
        args.push("build_tests");
        cmd_env(
            "cargo",
            &[nightly, "test", "--workspace", "--no-fail-fast", "--all-features"],
            &args,
            &[("RUST_BACKTRACE", "1")],
        );

        tests::version_in_sync();
    } else if let Some(unit_tests) = take_option(&mut args, &["-u", "--unit"], "<unit-test-name>") {
        for test_name in unit_tests {
            cmd_env(
                "cargo",
                &[
                    nightly,
                    "test",
                    "--package",
                    "zero-ui*",
                    "--lib",
                    "--no-fail-fast",
                    "--all-features",
                    test_name,
                ],
                &args,
                &[("RUST_BACKTRACE", "1")],
            );
        }
    } else if take_flag(&mut args, &["--doc"]) {
        // only doc tests for the main workspace.
        let trace = if take_flag(&mut args, &["--trace"]) { "1" } else { "" };
        cmd_env(
            "cargo",
            &[nightly, "test", "--workspace", "--no-fail-fast", "--all-features", "--doc"],
            &args,
            &[("RUST_BACKTRACE", trace)],
        );

        tests::version_in_sync();
    } else if let Some(int_tests) = take_option(&mut args, &["-t", "--test"], "<integration-test-name>") {
        // only specific integration test.
        let mut t_args = vec![nightly, "test", "--workspace", "--no-fail-fast", "--all-features"];
        for it in int_tests {
            t_args.push("--test");
            t_args.push(it);
        }
        cmd_env("cargo", &t_args, &args, &[("RUST_BACKTRACE", "1")]);
    } else if let Some(build_tests) = take_option(&mut args, &["-b", "--build"], "<build-test-pat>") {
        let build_tests_args = vec![
            nightly,
            "test",
            "--workspace",
            "--no-fail-fast",
            "--all-features",
            "--test",
            "build_tests",
        ];

        let overwrite = if take_flag(&mut args, &["--OVERWRITE"]) { "overwrite" } else { "" };

        let all_patterns = ["*", "**", "*/*", "*\\*"];
        if all_patterns.iter().any(|a| build_tests.contains(a)) {
            // all build tests.
            cmd_env("cargo", &build_tests_args, &args, &[("TRYBUILD", overwrite)]);
            return;
        } else {
            // specific test files.
            let mut args = build_tests_args;
            args.extend(&["--", "do_tasks_test_runner", "--exact", "--ignored"]);
            for test_name in build_tests {
                cmd_env("cargo", &args, &[], &[("DO_TASKS_TEST_BUILD", test_name), ("TRYBUILD", overwrite)]);
            }
        }
    } else if take_flag(&mut args, &["--examples"]) {
        // example tests
        cmd_env("cargo", &[nightly, "test", "--examples"], &args, &[("RUST_BACKTRACE", "1")]);
    } else if let Some(examples) = take_option(&mut args, &["--example"], "<NAME>") {
        // named example tests
        for example in examples {
            cmd_env("cargo", &[nightly, "--example", example], &args, &[("RUST_BACKTRACE", "1")]);
        }
    } else if take_flag(&mut args, &["--test-crates"]) {
        // all ./test-crates
        for test_crate in top_cargo_toml("test-crates") {
            cmd_env(
                "cargo",
                &[
                    nightly,
                    "test",
                    "--workspace",
                    "--no-fail-fast",
                    "--all-features",
                    "--manifest-path",
                    &test_crate,
                ],
                &args,
                &[("RUST_BACKTRACE", "1")],
            );
        }
    } else if !args.is_empty() {
        cmd("cargo", &[nightly, "test"], &args);
    } else {
        test(vec![nightly, "--workspace"]);
        test(vec![nightly, "--examples"]);
        test(vec![nightly, "--build", "*"]);
        test(vec![nightly, "--test-crates"]);
    }
}

// do run, r EXAMPLE [-p, --profile] [-t, --trace] [<cargo-run-args>]
//    Run an example in ./examples. If profiling builds in release with app_profiler.
// USAGE:
//     run some_example
//        Runs the "some_example" in debug mode.
//     run some_example --release
//        Runs the "some_example" in release mode.
//     run some_example --profile
//        Runs the "some_example" in release mode with the "app_profiler" feature.
//     run some_example --trace
//        Runs the "some_example" with `RUST_BACKTRACE=1`.
//     run * [--release] [-t, --trace]
//        Builds all examples then runs them one by one.
fn run(mut args: Vec<&str>) {
    let trace = if take_flag(&mut args, &["-t", "--trace"]) {
        ("RUST_BACKTRACE", "1")
    } else {
        ("", "")
    };

    if take_flag(&mut args, &["-p", "--profile"]) {
        take_flag(&mut args, &["--release"]);
        let rust_flags = release_rust_flags(true);
        let rust_flags = &[(rust_flags.0, rust_flags.1.as_str()), trace];
        cmd_env(
            "cargo",
            &["run", "--release", "--features", "app_profiler", "--example"],
            &args,
            rust_flags,
        );
    } else if let Some(&"*") = args.first() {
        args.remove(0);
        let release = args.contains(&"--release");
        let rust_flags = release_rust_flags(release);
        let rust_flags = &[(rust_flags.0, rust_flags.1.as_str()), trace];

        let release = if release { "--release" } else { "" };
        cmd_env("cargo", &["build", "--examples", release], &[], rust_flags);
        for example in examples() {
            cmd_env("cargo", &["run", "--example", &example, release], &[], rust_flags);
        }
    } else {
        let rust_flags = release_rust_flags(args.contains(&"--release"));
        let rust_flags = &[(rust_flags.0, rust_flags.1.as_str()), trace];
        cmd_env("cargo", &["run", "--example"], &args, rust_flags);
    }
}

// do expand [-p <crate>] [<ITEM-PATH>] [-r, --raw] [-e, --example <example>]
//           [-b, --build [-p, -pass <pass-test-name>] [-f, --fail <fail-test-name>]]
//           [<cargo-expand-args>|<cargo-args>]
//    Run "cargo expand" OR if raw is enabled, runs the unstable "--pretty=expanded" check.
// FLAGS:
//     --dump   Write the expanded Rust code to "dump.rs".
// USAGE:
//     expand some::item
//        Prints only the specified item in the main crate.
//     expand -p "other-crate" some::item
//        Prints only the specified item in the other-crate from workspace.
//     expand -e "example"
//        Prints the example.
//     expand --raw
//        Prints the entire main crate, including macro_rules!.
//     expand --build -p pass_test_name
//        Prints the build test cases that match.
fn expand(mut args: Vec<&str>) {
    if args.iter().any(|&a| a == "-b" || a == "--build") {
        // Expand build test, we need to run the test to load the bins
        // in the trybuild test crate. We also test in nightly because
        // expand is in nightly.

        let mut test_args = args.clone();
        test_args.insert(0, "+nightly");
        test(test_args);

        TaskInfo::get().stdout_dump = "dump.rs";
        for (bin_name, path) in build_test_cases() {
            let i = path.find("tests").unwrap_or_default();
            println(f!("\n//\n// {}\n//\n", &path[i..]));
            cmd(
                "cargo",
                &[
                    "expand",
                    "--manifest-path",
                    "target/tests/zero-ui/Cargo.toml",
                    "--bin",
                    &bin_name,
                    "--all-features",
                ],
                &[],
            );
        }
    } else if take_flag(&mut args, &["-e", "--example"]) {
        TaskInfo::get().stdout_dump = "dump.rs";
        cmd("cargo", &["expand", "--example"], &args);
    } else {
        TaskInfo::get().stdout_dump = "dump.rs";
        if take_flag(&mut args, &["-r", "--raw"]) {
            cmd(
                "cargo",
                &["+nightly", "rustc", "--profile=check", "--", "-Zunpretty=expanded"],
                &args,
            );
        } else {
            cmd("cargo", &["expand", "--lib", "--tests", "--all-features"], &args);
        }
    }
}

// do fmt, f [<cargo-fmt-args>] [-- <rustfmt-args>]
//    Format workspace, build test samples, test-crates and the tasks script.
fn fmt(args: Vec<&str>) {
    print("    fmt workspace ... ");
    cmd("cargo", &["fmt"], &args);
    println("done");

    print("    fmt test-crates ... ");
    for test_crate in top_cargo_toml("test-crates") {
        cmd("cargo", &["fmt", "--manifest-path", &test_crate], &args);
    }
    println("done");

    print("    fmt tests/build/**/*.rs ... ");
    let cases = all_rs("tests/build");
    let cases_str: Vec<_> = cases.iter().map(|s| s.as_str()).collect();
    cmd("rustfmt", &["--edition", "2018"], &cases_str);
    println("done");

    print("    fmt tools ... ");
    for tool_crate in top_cargo_toml("tools") {
        cmd("cargo", &["fmt", "--manifest-path", &tool_crate], &args);
    }
    println("done");
}

// do build, b [-e, --example] [--all] [-t, --timing] [<cargo-build-args>]
//    Compile the main crate and its dependencies.
// USAGE:
//    build -e <example>
//       Compile the example.
//    build --workspace
//       Compile the root workspace.
//    build --all
//       Compile the root workspace and ./test-crates.
//    build -p <crate> --timing
//       Compile crate in nightly and report "cargo-timing.html"
fn build(mut args: Vec<&str>) {
    let nightly = if take_flag(&mut args, &["+nightly"]) { "+nightly" } else { "" };

    let rust_flags = release_rust_flags(args.contains(&"--release"));
    let rust_flags = &[(rust_flags.0, rust_flags.1.as_str())];

    if take_flag(&mut args, &["-t", "--timing"]) {
        if let Some(example) = args.iter_mut().find(|a| **a == "-e") {
            *example = "--example";
        }
        cmd_env("cargo", &["+nightly", "build", "-Ztimings"], &args, rust_flags);
    } else if take_flag(&mut args, &["--all"]) {
        for test_crate in top_cargo_toml("test-crates") {
            cmd_env("cargo", &[nightly, "build", "--manifest-path", &test_crate], &args, rust_flags);
        }
        cmd_env("cargo", &[nightly, "build"], &args, rust_flags);
    } else {
        if let Some(example) = args.iter_mut().find(|a| **a == "-e") {
            *example = "--example";
        }
        cmd_env("cargo", &[nightly, "build"], &args, rust_flags);
    }
}
fn release_rust_flags(is_release: bool) -> (&'static str, String) {
    let mut rust_flags = ("", String::new());
    if is_release {
        // remove user name from release build, unless machine is already
        // configured to "--remap-path-prefix"
        let mut remap = String::new();
        remap.push_str("--remap-path-prefix ");
        let cargo_home = env!("CARGO_HOME");
        let i = cargo_home.find(".cargo").unwrap();
        remap.push_str(&cargo_home[..i - 1]);
        remap.push_str("=~");
        match std::env::var("RUSTFLAGS") {
            Ok(mut flags) if !flags.contains("--remap-path-prefix") => {
                flags.push(' ');
                flags.push_str(&remap);
                rust_flags = ("RUSTFLAGS", flags);
            }
            Err(std::env::VarError::NotPresent) => {
                rust_flags = ("RUSTFLAGS", remap);
            }
            _ => {}
        };
    }
    rust_flags
}

// do clean [--test-crates] [--tools] [--workspace] [<cargo-clean-args>]
//    Remove workspace, test-crates and tools target directories.
// USAGE:
//    clean --test-crates
//       Remove only the target directories in ./test-crates.
//    clean --tools
//       Remove only the target directories in ./tools.
//    clean --workspace
//       Remove only the target directory of the root workspace.
//    clean --doc
//       Remove only the doc files from the target directories.
//    clean --release
//       Remove only the release files from the target directories.
fn clean(mut args: Vec<&str>) {
    let test_crates = take_flag(&mut args, &["--test-crates"]);
    let tools = take_flag(&mut args, &["--tools"]);
    let workspace = take_flag(&mut args, &["--workspace"]);
    let all = !test_crates && !tools && !workspace;

    if all || workspace {
        cmd("cargo", &["clean"], &args);
    }
    if all || test_crates {
        for crate_ in top_cargo_toml("test-crates") {
            cmd("cargo", &["clean", "--manifest-path", &crate_], &args);
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
        cmd_external("cargo", &["clean", "--manifest-path", env!("DO_MANIFEST_PATH")], &args);
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
        let t = TaskInfo::get();
        if t.dump {
            asm_args.push("--no-color");
            t.stdout_dump = "dump.asm";
        }
    }

    util::do_after(10, || {
        println(r#"Awaiting "cargo asm", this can take a while..."#);
    });

    cmd("cargo", &asm_args, &args);
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
        DO,
        c_w(),
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    ));

    let specific_task = !args.is_empty();

    if !specific_task {
        println(f!("   {}", env!("CARGO_PKG_DESCRIPTION")));
        println("\nUSAGE:");
        println(f!("    {} TASK [<TASK-ARGS>]", DO));
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
            error(f!("task `{}` not found in help", t));
        }
    }
}
