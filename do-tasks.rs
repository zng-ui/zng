use std::env;
use std::format_args as f;

const DO: &str = "do"; // shell script that builds and runs the tasks.
const DO_RS: &str = "do-tasks.rs"; // tasks file name (this file).

fn main() {
    let (task, args) = args();
    task_header(task, &args[..]);

    match task {
        "doc" => doc(args),
        "test" => test(args),
        "run" => run(args),
        "expand" => expand(args),
        "h" | "-h" | "help" | "--help" => help(args),
        _ => fatal(f!("unknown task {:?}, \"{} help\" to list tasks", task, DO)),
    }
}

/******
 Tasks
******/

// do doc [-o, --open] [<cargo-doc-args>]
//    Generate documentation for crates in the root workspace.
fn doc(mut args: Vec<&str>) {
    if let Some(open) = args.iter_mut().find(|a| **a == "-o") {
        *open = "--open";
    }
    cmd("doc", "cargo", &["doc", "--all-features", "--no-deps", "--workspace"], &args);
}

// do test [<cargo-test-args>]
//    Run all tests in project.
fn test(args: Vec<&str>) {
    cmd("test", "cargo", &["test", "--workspace", "--no-fail-fast"], &args);
    test_crate("no-direct-dep", &args);
}
fn test_crate(crate_: &str, user_args: &[&str]) {
    cmd(
        "test",
        "cargo",
        &[
            "test",
            "--workspace",
            "--no-fail-fast",
            "--manifest-path",
            &format!("test-crates/{}/Cargo.toml", crate_),
        ],
        &user_args,
    );
}

// do run EXAMPLE [-p, --profile] [<cargo-run-args>]
//    Run an example in ./examples. If profiling builds in release with app_profiler.
fn run(mut args: Vec<&str>) {
    if take_arg(&mut args, &["-p", "--profile"]) {
        cmd(
            "run",
            "cargo",
            &["run", "--release", "--features", "app_profiler", "--example"],
            &args,
        );
    } else {
        cmd("run", "cargo", &["run", "--example"], &args);
    }
}

// do expand [-r, --raw] [<cargo-expand-args>|<cargo-args>]
//    Run "cargo expand" OR if raw is enabled, runs the unstable "--pretty=expanded" check.
fn expand(mut args: Vec<&str>) {
    if take_arg(&mut args, &["-r", "--raw"]) {
        cmd(
            "expand (raw)",
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
        cmd("expand", "cargo", &["expand"], &args);
    }
}

// do help
//    prints this help, task docs are extracted from the tasks file.
fn help(_: Vec<&str>) {
    println!("\n{}{}{} ({}{}{})", C_WB, DO_RS, C_W, C_WB, DO, C_W);
    println!("\n   Tasks for managing this project, implemented as a single Rust file.");
    println!("\nUSAGE:");
    println!("    {} TASK [<TASK-ARGS>]", DO);
    println!("\nTASKS:");

    // prints lines from this file that start with "// do " and comment lines directly after then.
    match std::fs::read_to_string(DO_RS) {
        Ok(rs) => {
            let mut expect_details = false;
            for line in rs.lines() {
                if line.starts_with("// do ") {
                    expect_details = true;
                    println!("\n    {}", &line["// do ".len()..]);
                } else if expect_details {
                    expect_details = line.starts_with("//");
                    if expect_details {
                        println!("    {}", &line["//".len()..]);
                    }
                }
            }
        }
        Err(e) => fatal(e),
    }
}

/*****
 Util
*****/

// Run a command, args are chained, empty ("") arg strings are filtered, command streams are inherited.
fn cmd(task: &str, cmd: &str, default_args: &[&str], user_args: &[&str]) {
    let args: Vec<_> = default_args.iter().chain(user_args.iter()).filter(|a| !a.is_empty()).collect();
    let status = std::process::Command::new(cmd)
        .args(&args[..])
        .status()
        .unwrap_or_else(|e| fatal(f!("task {:?} failed to run, {}", task, e)));
    if !status.success() {
        fatal(f!("task {:?} failed with exit code: {}", task, status));
    }
}

// Removes any of the flags in `any` from `args`. Returns if found any.
fn take_arg(args: &mut Vec<&str>, any: &[&str]) -> bool {
    let mut i = 0;
    let mut found = false;
    while i < args.len() {
        if any.iter().any(|&a| args[i] == a) {
            found = true;
            args.remove(i);
            continue;
        }
        i += 1;
    }
    found
}

// Parses the initial input. Returns ("task-name", ["task", "args"]).
fn args() -> (&'static str, Vec<&'static str>) {
    let mut args: Vec<_> = env::args().skip(1).collect();
    if args.is_empty() {
        fatal("missing task name")
    }
    let task = Box::leak(args.remove(0).into_boxed_str());
    let args = args.into_iter().map(|a| Box::leak(a.into_boxed_str()) as &'static str).collect();
    (task, args)
}

fn task_header(task: &str, args: &[&str]) {
    println!("{}Running{}: {}{} {:?} {:?}", C_GREEN, C_WB, DO_RS, C_W, task, args);
}

// Prints an error message, use `error(f!("{}", .."))` for formatting.
fn error(msg: impl std::fmt::Display) {
    eprintln!("{}error{}: {}{} {}", C_RED, C_WB, DO_RS, C_W, msg);
}

// Prints an [`error`] and exists with code `-1`.
fn fatal(msg: impl std::fmt::Display) -> ! {
    error(msg);
    std::process::exit(-1)
}

// ANSI colors.
const C_GREEN: &str = "\x1B[1;32m";
const C_RED: &str = "\x1B[1;31m";
const C_WB: &str = "\x1B[1;37m";
const C_W: &str = "\x1B[0m";
