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
        "help" | "--help" => help(args),
        _ => fatal(f!("unknown task {:?}, `{} help` to list tasks", task, do_cmd())),
    }
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
//    Generate documentation for zero-ui crates.
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
        &["+nightly", "doc", "--all-features", "--no-deps", "--package", "zero-ui*"],
        &args,
        &[("RUSTDOCFLAGS", "--cfg doc_nightly --cfg do_doc")],
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

/// do test, t [-u, --unit <function-path>]
///            [-t, --test <integration-test-name>]
///            [-b, --build <file-path-pattern> [--OVERWRITE]]
///            <cargo-test-args>
///
///    Run all tests in root workspace and build tests.
/// USAGE:
///     test -u test::path::function
///        Run tests that partially match the Rust item path.
///     test -u *
///        Run all unit tests.
///     test -t focus
///        Run all integration tests in the named test.
///     test -t *
///        Run all integration tests.
///     test -b property/*
///        Run build tests that match the file pattern in `tests/build/cases/`.
///     test -b *
///        Run all build tests.
///     test --doc
///        Run doc tests.
///     test
///        Run all unit, doc, integration and build tests.
fn test(mut args: Vec<&str>) {
    let nightly = if take_flag(&mut args, &["+nightly"]) { "+nightly" } else { "" };
    let env = &[("RUST_BACKTRACE", "1")];

    if let Some(unit_tests) = take_option(&mut args, &["-u", "--unit"], "<unit-test-name>") {
        // unit tests:

        let t_args = vec![
            nightly,
            "test",
            "--package",
            "zero-ui*",
            "--lib",
            "--no-fail-fast",
            "--all-features",
        ];

        if unit_tests.contains(&"*") {
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

        if !int_tests.contains(&"*") {
            for it in int_tests {
                t_args.push("--test");
                t_args.push(it);
            }
        }

        cmd_env("cargo", &t_args, &args, env);
    } else if take_flag(&mut args, &["-b", "--build"]) {
        // build tests:

        let overwrite = if take_flag(&mut args, &["--OVERWRITE"]) { "overwrite" } else { "" };

        if args.len() != 1 {
            error("expected pattern, use do test -b * to run all build tests");
        } else {
            cmd_env(
                "cargo",
                &["run", "--package", "build-tests"],
                &[],
                &[("TRYBUILD", overwrite), ("DO_TASKS_TEST_BUILD", args[0])],
            );
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

        if !all && take_flag(&mut args, &["--doc"]) {
            tests::version_in_sync();
        }

        cmd_env("cargo", &[nightly, "test", "--no-fail-fast", "--all-features"], &args, env);

        if all {
            // if no args we run everything.
            tests::version_in_sync();
            test(vec!["--build", "*"]);
        }
    }
}

// do run, r EXAMPLE [-b, --backtrace] [<cargo-run-args>]
//    Run an example in ./examples.
// USAGE:
//     run some_example
//        Runs the example in debug mode.
//     run some_example --release
//        Runs the example in release mode.
//     run some_example --backtrace
//        Runs the "some_example" with `RUST_BACKTRACE=1`.
//     run *
//        Builds all examples then runs them one by one.
fn run(mut args: Vec<&str>) {
    let trace = if take_flag(&mut args, &["-b", "--backtrace"]) {
        ("RUST_BACKTRACE", "1")
    } else {
        ("", "")
    };

    if let Some(&"*") = args.first() {
        args.remove(0);
        let release = args.contains(&"--release");
        let rust_flags = release_rust_flags(release);
        let rust_flags = &[(rust_flags.0, rust_flags.1.as_str()), trace];

        let release = if release { "--release" } else { "" };
        cmd_env("cargo", &["build", "--package", "examples", "--examples", release], &[], rust_flags);
        for example in examples() {
            cmd_env(
                "cargo",
                &["run", "--package", "examples", "--example", &example, release],
                &[],
                rust_flags,
            );
        }
    } else {
        let rust_flags = release_rust_flags(args.contains(&"--release"));
        let rust_flags = &[(rust_flags.0, rust_flags.1.as_str()), trace];
        cmd_env("cargo", &["run", "--package", "examples", "--example"], &args, rust_flags);
    }
}

// do expand [-p <crate>] [<ITEM-PATH>] [-r, --raw] [-e, --example <example>]
//           [-b, --build [-p, -pass <pass-test-name>] [-f, --fail <fail-test-name>]]
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
                    "target/tests/build-tests/Cargo.toml",
                    "--bin",
                    &bin_name,
                    "--all-features",
                ],
                &[],
            );
        }
    } else if take_flag(&mut args, &["-e", "--example"]) {
        TaskInfo::get().stdout_dump = "dump.rs";
        cmd("cargo", &["expand", "--package", "examples", "--example"], &args);
    } else {
        TaskInfo::get().stdout_dump = "dump.rs";
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

    print("    fmt tests/build/cases/**/*.rs ... ");
    let cases = all_rs("tests/build/cases");
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

// do build, b [-e, --example] [--examples] [-t, --timing] [<cargo-build-args>]
//    Compile the main crate and its dependencies.
// USAGE:
//    build -e <example>
//       Compile the example.
//    build --examples
//       Compile all examples.
//    build -p <crate> --timing
//       Compile crate in nightly and report "cargo-timing.html"
fn build(mut args: Vec<&str>) {
    let mut cargo_args = vec![];

    let timing = take_flag(&mut args, &["-t", "--timing"]);
    if take_flag(&mut args, &["+nightly"]) || timing {
        cargo_args.push("+nightly");
    }

    cargo_args.push("build");

    let rust_flags = release_rust_flags(args.contains(&"--release"));
    let rust_flags = &[(rust_flags.0, rust_flags.1.as_str())];

    if timing {
        cargo_args.push("-Ztimings");
    }

    if take_flag(&mut args, &["-e", "--example"]) {
        cargo_args.extend(&["--package", "examples", "--example"]);
    } else if take_flag(&mut args, &["--examples"]) {
        cargo_args.extend(&["--package", "examples", "--examples"]);
    }

    cmd_env("cargo", &cargo_args, &args, rust_flags);
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

// do prebuild
//    Compile the pre-build `zero-ui-view` release.
fn prebuild(args: Vec<&str>) {
    cmd("cargo", &["build", "-p", "zero-ui-view", "--release"], &args);

    let files = cdylib_files("target/release/zero_ui_view");

    if files.is_empty() {
        error("no pre-build `cdylib` output found");
        return;
    }

    for file in files {
        let target = format!("zero-ui-view-prebuilt/lib/{}", file.file_name().unwrap().to_string_lossy());
        if let Err(e) = std::fs::copy(&file, &target) {
            error(f!("failed to copy pre-build lib `{}` to `{}`, {}", file.display(), target, e))
        }
    }

    // test build
    cmd("cargo", &["build", "-p", "zero-ui-view-prebuilt", "--release"], &[]);
}

// do clean [--tools] [--workspace] [<cargo-clean-args>]
//    Remove workspace, test-crates and tools target directories.
// USAGE:
//    clean --tools
//       Remove only the target directories in ./tools.
//    clean --workspace
//       Remove only the target directory of the root workspace.
//    clean --doc
//       Remove only the doc files from the target directories.
//    clean --release
//       Remove only the release files from the target directories.
fn clean(mut args: Vec<&str>) {
    let tools = take_flag(&mut args, &["--tools"]);
    let workspace = take_flag(&mut args, &["--workspace"]);
    let all = !tools && !workspace;

    if all || workspace {
        cmd("cargo", &["clean"], &args);
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
                panic!("{:?}", e)
            }
        }
        println("rust-analyzer check is enabled");
    } else {
        let _ = std::fs::File::create(path).unwrap();
        println("rust-analyzer check is disabled");
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
            error(f!("task `{}` not found in help", t));
        }
    }
}
