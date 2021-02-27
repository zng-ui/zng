use std::env;
use std::format_args as f;
use std::io::Write;

const DO: &str = "do"; // shell script that builds and runs the tasks.
const DO_RS: &str = "do-tasks.rs"; // tasks file name (this file).

fn main() {
    let (task, args) = args();

    match task {
        "fmt" => fmt(args),
        "test" => test(args),
        "run" => run(args),
        "doc" => doc(args),
        "expand" => expand(args),
        "build" => build(args),
        "clean" => clean(args),
        "h" | "-h" | "help" | "--help" => help(args),
        _ => fatal(f!("unknown task {:?}, `{} help` to list tasks", task, DO)),
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
    cmd("cargo", &["doc", "--all-features", "--no-deps", "--workspace"], &args);
}

// do test [<cargo-test-args>]
//    Run all tests in project.
fn test(args: Vec<&str>) {
    cmd("cargo", &["test", "--workspace", "--no-fail-fast"], &args);
    for test_crate in top_cargo_toml("test-crates") {
        cmd(
            "cargo",
            &["test", "--workspace", "--no-fail-fast", "--manifest-path", &test_crate],
            &args,
        );
    }
}

// do run EXAMPLE [-p, --profile] [<cargo-run-args>]
//    Run an example in ./examples. If profiling builds in release with app_profiler.
//    USAGE:
//        run some_example
//           Runs the "some_example" in debug mode.
//        run some_example --release
//           Runs the "some_example" in release mode.
//        run some_example --profile
//           Runs the "some_example" in release mode with the "app_profiler" feature.
fn run(mut args: Vec<&str>) {
    if take_arg(&mut args, &["-p", "--profile"]) {
        cmd("cargo", &["run", "--release", "--features", "app_profiler", "--example"], &args);
    } else {
        cmd("cargo", &["run", "--example"], &args);
    }
}

// do expand [-p <crate>] [<ITEM-PATH>] [-r, --raw] [<cargo-expand-args>|<cargo-args>]
//    Run "cargo expand" OR if raw is enabled, runs the unstable "--pretty=expanded" check.
//    FLAGS:
//        --dump   Write the expanded Rust code to "dump.rs".
//    USAGE:
//        expand some::item
//           Prints only the specified item in the main crate.
//        expand -p "other-crate" some::item
//           Prints only the specified item in the other-crate from workspace.
//        expand --raw
//           Prints the entire main crate, including macro_rules!.
fn expand(mut args: Vec<&str>) {
    TaskInfo::get().set_stdout_dump("dump.rs");
    if take_arg(&mut args, &["-r", "--raw"]) {
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

// do fmt [<cargo-fmt-args>] [-- <rustfmt-args>]
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

    print("    fmt tests/build/**/*.rs .. ");
    let cases = all_rs("tests/build");
    let cases_str: Vec<_> = cases.iter().map(|s| s.as_str()).collect();
    cmd("rustfmt", &["--edition", "2018"], &cases_str);
    println("done");

    print(f!("    fmt {} ... ", DO_RS));
    cmd("rustfmt", &["--edition", "2018", DO_RS], &[]);
    println("done");
}

// do build [-e, --example] [--self] [--all] [<cargo-build-args>]
//    Compile the main crate and its dependencies.
// USAGE:
//    build -e <example>
//       Compile the example.
//    build --workspace
//       Compile the root workspace.
//    build --all
//       Compile the root workspace and ./test-crates.
//    build --self
//       Rebuild this tool.
fn build(mut args: Vec<&str>) {
    if take_arg(&mut args, &["--self"]) {
        // external because it will replace self.
        print(r#"   closing to rebuild, will print "rebuild finished" when done"#);
        cmd_external(
            "rustc",
            &[
                DO_RS,
                "--edition",
                "2018",
                "--out-dir",
                env!("DO_TASK_OUT"),
                "-C",
                "opt-level=3",
                "&",
                "echo",
                "rebuild finished",
            ],
            &args,
        );
    } else if take_arg(&mut args, &["--all"]) {
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

// do clean [--test-crates] [<cargo-clean-args>]
//    Remove workspace and test-crates target directories.
// USAGE:
//    clean --test-crates
//       Remove only the target directories in ./test-crates.
//    clean --doc
//       Remove only the workspace doc files.
//    clean --release
//       Remove only the release files from the target directories.
fn clean(mut args: Vec<&str>) {
    let test_crates_only = take_arg(&mut args, &["--test-crates"]);

    for crate_ in top_cargo_toml("test-crates") {
        cmd("cargo", &["clean", "--manifest-path", &crate_], &args);
    }

    if !test_crates_only {
        if args.iter().any(|&a| a == "--doc" || a == "--release") {
            cmd("cargo", &["clean"], &args);
        } else {
            print(r#"   closing to also clean self, will print "clean finished" when done"#);
            // external because it will delete self.
            cmd_external("cargo", &["clean"], &args);
        }
    }
}

// do help, h, -h, --help
//    prints this help, task docs are extracted from the tasks file.
fn help(_: Vec<&str>) {
    println(f!("\n{}{}{} ({}{}{})", c_wb(), DO, c_w(), c_wb(), DO_RS, c_w()));
    println("   Run tasks for managing this project, implemented as a Rust file.");
    println("\nUSAGE:");
    println(f!("    {} TASK [<TASK-ARGS>]", DO));
    println("\nFLAGS:");
    println(r#"    --dump   Redirect output to "dump.log" or other file specified by task."#);
    print("\nTASKS:");

    // prints lines from this file that start with "// do " and comment lines directly after then.
    match std::fs::read_to_string(DO_RS) {
        Ok(rs) => {
            let mut expect_details = false;
            for line in rs.lines() {
                if line.starts_with("// do ") {
                    expect_details = true;
                    let task_line = &line["// do ".len()..];
                    let task_name_end = task_line.find(' ').unwrap();
                    println(f!(
                        "\n    {}{}{}{}",
                        c_wb(),
                        &task_line[..task_name_end],
                        c_w(),
                        &task_line[task_name_end..]
                    ));
                } else if expect_details {
                    expect_details = line.starts_with("//");
                    if expect_details {
                        println(f!("    {}", &line["//".len()..]));
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
fn cmd(cmd: &str, default_args: &[&str], user_args: &[&str]) {
    cmd_impl(cmd, default_args, user_args, false)
}
// Like [`cmd`] but exists the task runner if the command fails.
//fn cmd_req(cmd: &str, default_args: &[&str], user_args: &[&str]) {
//    cmd_impl(cmd, default_args, user_args, true)
//}
fn cmd_impl(cmd: &str, default_args: &[&str], user_args: &[&str], required: bool) {
    let info = TaskInfo::get();
    let args: Vec<_> = default_args.iter().chain(user_args.iter()).filter(|a| !a.is_empty()).collect();

    let mut cmd = std::process::Command::new(cmd);
    cmd.args(&args[..]);

    if info.dump {
        if let Some(stdout) = info.stdout_dump() {
            cmd.stdout(std::process::Stdio::from(stdout));
        }
        if let Some(stderr) = info.stderr_dump() {
            cmd.stdout(std::process::Stdio::from(stderr));
        }
    }

    let status = cmd.status();
    match status {
        Ok(status) => {
            if !status.success() {
                let msg = format!("task {:?} failed with {}", info.name, status);
                if required {
                    fatal(msg);
                } else {
                    error(msg);
                }
            }
        }
        Err(e) => {
            let msg = format!("task {:?} failed to run, {}", info.name, e);
            if required {
                fatal(msg)
            } else {
                error(msg);
            }
        }
    }
}

// Like [`cmd`] but runs after a small delay and does not block.
// Use this for commands that need write access to the self executable.
fn cmd_external(cmd: &str, default_args: &[&str], user_args: &[&str]) {
    let args: Vec<_> = default_args.iter().chain(user_args.iter()).filter(|a| !a.is_empty()).collect();

    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(&["/C", "ping", "localhost", "-n", "3", ">", "nul", "&"])
            .arg(cmd)
            .args(&args)
            .spawn()
            .ok();
    }
    #[cfg(not(windows))]
    {
        todo!("cmd_external only implemented in windows")
    }
}

// Removes all of the flags in `any` from `args`. Returns if found any.
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
        return ("", vec![]);
    }
    let task = Box::leak(args.remove(0).into_boxed_str());
    let mut args = args.into_iter().map(|a| Box::leak(a.into_boxed_str()) as &'static str).collect();

    // set task name and flags
    let info = TaskInfo::get();
    info.name = task;
    info.dump = take_arg(&mut args, &["--dump"]);

    // prints header
    println(f!("{}Running{}: {}{} {:?} {:?}", c_green(), c_wb(), DO_RS, c_w(), task, args));

    (task, args)
}

// Information about the running task.
struct TaskInfo {
    name: &'static str,
    dump: bool,
    stdout_dump: &'static str,
    stderr_dump: &'static str,
    stdout_dump_file: Option<std::fs::File>,
    stderr_dump_file: Option<std::fs::File>,
}
static mut TASK_INFO: TaskInfo = TaskInfo {
    name: "",
    dump: false,
    stdout_dump: "dump.log",
    stderr_dump: "dump.log",
    stdout_dump_file: None,
    stderr_dump_file: None,
};
impl TaskInfo {
    fn get() -> &'static mut TaskInfo {
        unsafe { &mut TASK_INFO }
    }
    fn set_stdout_dump(&mut self, file: &'static str) {
        self.stdout_dump_file = None;
        self.stdout_dump = file;
    }
    fn stdout_dump(&mut self) -> Option<std::fs::File> {
        if self.dump && !self.stdout_dump.is_empty() {
            if self.stdout_dump_file.is_none() {
                self.stdout_dump_file = std::fs::File::create(self.stdout_dump).ok();
            }
            self.stdout_dump_file.as_ref().and_then(|f| f.try_clone().ok())
        } else {
            None
        }
    }
    fn stderr_dump(&mut self) -> Option<std::fs::File> {
        if self.dump && !self.stderr_dump.is_empty() {
            if self.stderr_dump_file.is_none() {
                if self.stderr_dump == self.stdout_dump {
                    let file = self.stdout_dump();
                    self.stderr_dump_file = file;
                } else {
                    self.stdout_dump_file = std::fs::File::create(self.stdout_dump).ok();
                }
            }
            self.stderr_dump_file.as_ref().and_then(|f| f.try_clone().ok())
        } else {
            None
        }
    }
}

// Get all paths to `dir/*/Cargo.toml`
fn top_cargo_toml(dir: &str) -> Vec<String> {
    let mut r = vec![];
    match std::fs::read_dir(dir) {
        Ok(dir) => {
            for entry in dir {
                match entry {
                    Ok(et) => {
                        let path = et.path();
                        if path.is_dir() {
                            let crate_ = path.join("Cargo.toml");
                            if crate_.exists() {
                                r.push(crate_.to_string_lossy().into_owned());
                            }
                        }
                    }
                    Err(e) => error(e),
                }
            }
        }
        Err(e) => error(e),
    }
    r
}

// Get all `dir/**/*.rs` files.
fn all_rs(dir: &str) -> Vec<String> {
    let mut r = vec![];
    glob_rs(dir.into(), &mut r);
    r
}
fn glob_rs(dir: std::path::PathBuf, r: &mut Vec<String>) {
    match std::fs::read_dir(dir) {
        Ok(dir) => {
            for entry in dir {
                match entry {
                    Ok(et) => {
                        let path = et.path();
                        if path.is_dir() {
                            glob_rs(path, r);
                        } else if let Some(ext) = path.extension() {
                            if ext == "rs" {
                                r.push(path.to_string_lossy().into_owned());
                            }
                        }
                    }
                    Err(e) => error(e),
                }
            }
        }
        Err(e) => error(e),
    }
}

fn println(msg: impl std::fmt::Display) {
    if let Some(mut dump) = TaskInfo::get().stdout_dump() {
        writeln!(dump, "{}", msg).ok();
    } else {
        println!("{}", msg);
    }
}
fn print(msg: impl std::fmt::Display) {
    if let Some(mut dump) = TaskInfo::get().stdout_dump() {
        write!(dump, "{}", msg).ok();
    } else {
        print!("{}", msg);
        std::io::stdout().lock().flush().ok();
    }
}

// Prints an error message, use `error(f!("{}", .."))` for formatting.
fn error(msg: impl std::fmt::Display) {
    if let Some(mut dump) = TaskInfo::get().stderr_dump() {
        writeln!(dump, "{}error{}: {}{} {}", c_red(), c_wb(), DO_RS, c_w(), msg).ok();
    } else {
        eprintln!("{}error{}: {}{} {}", c_red(), c_wb(), DO_RS, c_w(), msg);
    }
}

// Prints an [`error`] and exists with code `-1`.
fn fatal(msg: impl std::fmt::Display) -> ! {
    error(msg);
    std::process::exit(-1)
}

// ANSI colors.
fn c_green() -> &'static str {
    color("\x1B[1;32m")
}
fn c_red() -> &'static str {
    color("\x1B[1;31m")
}
fn c_wb() -> &'static str {
    color("\x1B[1;37m")
}
fn c_w() -> &'static str {
    color("\x1B[0m")
}
fn color(color: &str) -> &str {
    if TaskInfo::get().dump {
        ""
    } else {
        color
    }
}
