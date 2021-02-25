use std::env;
use std::format_args as f;

fn main() {
    let (task, args) = args();
    task_header(task, &args[..]);

    match task {
        // do doc [-o, --open]
        "doc" => doc(args),
        // do test
        "test" => test(args),
        // do run [-p, --profile]
        "run" => run(args),
        // do expand [-r, --raw]
        "expand" => expand(args),
        // unknown
        _ => fatal(f!("unknown task {:?}", task)),
    }
}

/******
 Tasks
******/

// do doc [-o, --open]
fn doc(mut args: Vec<&str>) {
    if let Some(open) = args.iter_mut().find(|a| **a == "-o") {
        *open = "--open";
    }
    command("doc", "cargo", &["doc", "--all-features", "--no-deps", "--workspace"], &args);
}

// do test
fn test(args: Vec<&str>) {
    command("test", "cargo", &["test", "--workspace", "--no-fail-fast"], &args);
    test_crate("no-direct-dep", &args);
}
fn test_crate(crate_: &str, user_args: &[&str]) {
    command(
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

// do run [-p, --profile]
fn run(mut args: Vec<&str>) {
    if take_arg(&mut args, &["-p", "--profile"]) {
        command(
            "run",
            "cargo",
            &["run", "--release", "--features", "app_profiler", "--example"],
            &args,
        );
    } else {
        command("run", "cargo", &["run", "--example"], &args);
    }
}

// do expand [-r, --raw]
fn expand(mut args: Vec<&str>) {
    if take_arg(&mut args, &["-r", "--raw"]) {
        command(
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
        command("expand", "cargo", &["expand"], &args);
    }
}

/*****
 Util
*****/

fn command(task: &str, cmd: &str, default_args: &[&str], user_args: &[&str]) {
    let args: Vec<_> = default_args.iter().chain(user_args.iter()).filter(|a| !a.is_empty()).collect();
    let status = std::process::Command::new(cmd)
        .args(&args[..])
        .status()
        .unwrap_or_else(|e| fatal(f!("task {:?} failed to run, {}", task, e)));
    if !status.success() {
        fatal(f!("task {:?} failed with exit code: {}", task, status));
    }
}

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
    println!("{}Running{}: tasks.rs{} {:?} {:?}", GREEN, BOLD_W, NC, task, args);
}

fn error(msg: impl std::fmt::Display) {
    eprintln!("{}error{}: tasks.rs{} {}", RED, BOLD_W, NC, msg);
}

fn fatal(msg: impl std::fmt::Display) -> ! {
    error(msg);
    std::process::exit(-1)
}

const GREEN: &str = "\x1B[1;32m";
const RED: &str = "\x1B[1;31m";
const BOLD_W: &str = "\x1B[1;37m";
const NC: &str = "\x1B[0m";
