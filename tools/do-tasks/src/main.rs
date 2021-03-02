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
        "help" | "--help" => help(args),
        _ => fatal(f!("unknown task {:?}, `{} help` to list tasks", task, DO)),
    }
}

// do doc [-o, --open] [<cargo-doc-args>]
//    Generate documentation for crates in the root workspace.
fn doc(mut args: Vec<&str>) {
    if let Some(open) = args.iter_mut().find(|a| **a == "-o") {
        *open = "--open";
    }
    cmd("cargo", &["doc", "--all-features", "--no-deps", "--workspace"], &args);
}

// do test, t [-w, --workspace] [-u, --unit <unit-test>] [--test-crates]
//            [-b, --build [-p, -pass <pass-test-name>] [-f, --fail <fail-test-name>]]
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
//     test --build
//        Run all build tests
//     test -b -f <build_test_case>
//        Run build test files that match "./tests/build/*/fail/*<build_test_case>*.rs".
fn test(mut args: Vec<&str>) {
    if take_flag(&mut args, &["-w", "--workspace"]) {
        // exclude ./test-crates and build_tests
        if !args.iter().any(|a| *a == "--") {
            args.push("--");
        }
        args.push("--skip");
        args.push("build_tests");
        cmd("cargo", &["test", "--workspace", "--no-fail-fast", "--all-features"], &args);
    } else if let Some(unit_tests) = take_option(&mut args, &["-u", "--unit"], "<unit-test-name>") {
        // exclude ./test-crates and integration tests
        for test_name in unit_tests {
            cmd(
                "cargo",
                &["test", "--workspace", "--no-fail-fast", "--all-features", test_name],
                &args,
            );
        }
    } else if take_flag(&mut args, &["--doc"]) {
        // only doc tests for the main workspace.
        cmd(
            "cargo",
            &["test", "--workspace", "--no-fail-fast", "--all-features", "--doc"],
            &args,
        );
    } else if let Some(int_tests) = take_option(&mut args, &["-t", "--test"], "<integration-test-name>") {
        // only specific integration test.
        let mut t_args = vec!["test", "--workspace", "--no-fail-fast", "--all-features"];
        for it in int_tests {
            t_args.push("--test");
            t_args.push(it);
        }
        cmd("cargo", &t_args, &args);
    } else if take_flag(&mut args, &["-b", "--build"]) {
        // build_tests
        let fails = take_option(&mut args, &["-f", "--fail"], "<fail-test-name>").unwrap_or_default();
        let passes = take_option(&mut args, &["-p", "--pass"], "<pass-test-name>").unwrap_or_default();

        let build_tests_args = vec!["test", "--workspace", "--no-fail-fast", "--all-features", "--test", "build_tests"];

        if fails.is_empty() && passes.is_empty() {
            // all build tests.
            cmd("cargo", &build_tests_args, &args);
            return;
        }

        // specific test files.
        if !passes.is_empty() {
            let mut args = build_tests_args.clone();
            args.extend(&["--", "do_tasks_util::do_test_pass", "--exact", "--ignored"]);
            for test_name in passes {
                cmd_env("cargo", &args, &[], &[("DO_TASKS_BUILD_TEST", test_name)]);
            }
        }
        if !fails.is_empty() {
            let mut args = build_tests_args;
            args.extend(&["--", "do_tasks_util::do_test_fail", "--exact", "--ignored"]);
            for test_name in fails {
                cmd_env("cargo", &args, &[], &[("DO_TASKS_BUILD_TEST", test_name)]);
            }
        }
    } else if take_flag(&mut args, &["--test-crates"]) {
        for test_crate in top_cargo_toml("test-crates") {
            cmd(
                "cargo",
                &[
                    "test",
                    "--workspace",
                    "--no-fail-fast",
                    "--all-features",
                    "--manifest-path",
                    &test_crate,
                ],
                &args,
            );
        }
    } else {
        cmd("cargo", &["test", "--workspace", "--no-fail-fast", "--all-features"], &args);
    }
}

// do run, r EXAMPLE [-p, --profile] [<cargo-run-args>]
//    Run an example in ./examples. If profiling builds in release with app_profiler.
// USAGE:
//     run some_example
//        Runs the "some_example" in debug mode.
//     run some_example --release
//        Runs the "some_example" in release mode.
//     run some_example --profile
//        Runs the "some_example" in release mode with the "app_profiler" feature.
fn run(mut args: Vec<&str>) {
    if take_flag(&mut args, &["-p", "--profile"]) {
        cmd("cargo", &["run", "--release", "--features", "app_profiler", "--example"], &args);
    } else {
        cmd("cargo", &["run", "--example"], &args);
    }
}

// do expand [-p <crate>] [<ITEM-PATH>] [-r, --raw] [<cargo-expand-args>|<cargo-args>]
//    Run "cargo expand" OR if raw is enabled, runs the unstable "--pretty=expanded" check.
// FLAGS:
//     --dump   Write the expanded Rust code to "dump.rs".
// USAGE:
//     expand some::item
//        Prints only the specified item in the main crate.
//     expand -p "other-crate" some::item
//        Prints only the specified item in the other-crate from workspace.
//     expand --raw
//        Prints the entire main crate, including macro_rules!.
fn expand(mut args: Vec<&str>) {
    TaskInfo::get().set_stdout_dump("dump.rs");
    if take_flag(&mut args, &["-r", "--raw"]) {
        cmd(
            "cargo",
            &[
                "+nightly",
                "rustc",
                "--profile=check",
                "--",
                "-Zunstable-options",
                "--pretty=expanded",
            ],
            &args,
        );
    } else {
        cmd("cargo", &["expand", "--lib", "--tests"], &args);
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

// do build, b [-e, --example] [--all] [<cargo-build-args>]
//    Compile the main crate and its dependencies.
// USAGE:
//    build -e <example>
//       Compile the example.
//    build --workspace
//       Compile the root workspace.
//    build --all
//       Compile the root workspace and ./test-crates.
fn build(mut args: Vec<&str>) {
    if take_flag(&mut args, &["--all"]) {
        for test_crate in top_cargo_toml("test-crates") {
            cmd("cargo", &["build", "--manifest-path", &test_crate], &args);
        }
        cmd("cargo", &["build"], &args);
    } else {
        if let Some(example) = args.iter_mut().find(|a| **a == "-e") {
            *example = "--example";
        }
        cmd("cargo", &["build"], &args);
    }
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

// do help, --help
//    Prints this help.
fn help(_: Vec<&str>) {
    println(f!(
        "\n{}{}{} ({} {})",
        c_wb(),
        DO,
        c_w(),
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    ));
    println(f!("   {}", env!("CARGO_PKG_DESCRIPTION")));
    println("\nUSAGE:");
    println(f!("    {} TASK [<TASK-ARGS>]", DO));
    println("\nFLAGS:");
    println(r#"    --dump   Redirect output to "dump.log" or other file specified by task."#);
    print("\nTASKS:");

    // prints lines from this file that start with "// do " and comment lines directly after then.
    let tasks_help = include_str!(concat!(std::env!("OUT_DIR"), "\\tasks-help.stdout"));
    println(tasks_help.replace("%c_wb%", c_wb()).replace("%c_w%", c_w()));
}
