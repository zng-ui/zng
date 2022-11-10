use std::env;
use std::format_args as f;
use std::io::Write;
use std::path::PathBuf;
use std::process::{self, Command, Stdio};

// Command line to run `do`
pub fn do_cmd() -> String {
    env::var("DO_CMD").ok().unwrap_or_else(|| "cargo do".to_owned())
}

// Run a command, args are chained, empty ("") arg strings are filtered, command streams are inherited.
pub fn cmd(cmd: &str, default_args: &[&str], user_args: &[&str]) {
    cmd_impl(cmd, default_args, user_args, &[], false)
}
// Like [`cmd`] but also sets environment variables. Empty var keys are filtered, empty values unset the variable.
pub fn cmd_env(cmd: &str, default_args: &[&str], user_args: &[&str], envs: &[(&str, &str)]) {
    cmd_impl(cmd, default_args, user_args, envs, false)
}
// Like [`cmd`] but exists the task runner if the command fails.
//pub fn cmd_req(cmd: &str, default_args: &[&str], user_args: &[&str]) {
//    cmd_impl(cmd, default_args, user_args, &[], true)
//}
// Like [`cmd_env`] but exists the task runner if the command fails.
pub fn cmd_env_req(cmd: &str, default_args: &[&str], user_args: &[&str], envs: &[(&str, &str)]) {
    cmd_impl(cmd, default_args, user_args, envs, true)
}
fn cmd_impl(cmd: &str, default_args: &[&str], user_args: &[&str], envs: &[(&str, &str)], required: bool) {
    let args: Vec<_> = default_args.iter().chain(user_args.iter()).filter(|a| !a.is_empty()).collect();

    let mut cmd = Command::new(cmd);
    cmd.args(&args[..]);
    cmd.envs(envs.iter().filter(|t| !t.0.is_empty() && !t.1.is_empty()).copied());

    for (remove, _) in envs.iter().filter(|t| !t.0.is_empty() && t.1.is_empty()) {
        cmd.env_remove(remove);
    }

    // rustup 1.25 sets these, breaks cargo +nightly
    {
        cmd.env_remove("RUSTC");
        cmd.env_remove("RUSTDOC");

        let orig_path = env::var_os("PATH").unwrap_or_default();
        let modified_split_paths =
            env::split_paths(&orig_path).filter(|path| !path.components().any(|component| component.as_os_str() == ".rustup"));
        let modified_path = env::join_paths(modified_split_paths).expect("invalid PATH");
        cmd.env("PATH", modified_path);
    }

    if TaskInfo::dump() {
        if let Some(stdout) = TaskInfo::stdout_dump() {
            cmd.stdout(Stdio::from(stdout));
        }
        if let Some(stderr) = TaskInfo::stderr_dump() {
            cmd.stderr(Stdio::from(stderr));
        }
    }

    let status = cmd.status();
    match status {
        Ok(status) => {
            if !status.success() {
                let msg = format!("task {:?} failed with {status}", TaskInfo::name());
                if required {
                    fatal(msg);
                } else {
                    error(msg);
                }
            }
        }
        Err(e) => {
            let msg = format!("task {:?} failed to run, {e}", TaskInfo::name());
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
pub fn cmd_external(cmd: &str, default_args: &[&str], user_args: &[&str]) {
    #[cfg(windows)]
    {
        let args: Vec<_> = default_args.iter().chain(user_args.iter()).filter(|a| !a.is_empty()).collect();
        // We use ping to cause a slight delay that gives time for the current
        // executable to close because the subsequent command is expected to affect
        // the current executable file.
        Command::new("cmd")
            .args(&["/C", "ping", "localhost", "-n", "3", ">", "nul", "&"])
            .arg(cmd)
            .args(&args)
            .spawn()
            .ok();
    }

    #[cfg(not(windows))]
    {
        // We assume that if not on Windows we are in a Unix based system.
        //
        // We don't need a delay in Unix because it naturally permits repointing
        // or removing a file name without affecting the current running file.
        self::cmd(cmd, default_args, user_args);
    }
}

// Removes all of the flags in `any` from `args`. Returns if found any.
pub fn take_flag(args: &mut Vec<&str>, any: &[&str]) -> bool {
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

// Removes all of the `option` values, fails with "-o <value_name>" if a value is missing.
pub fn take_option<'a>(args: &mut Vec<&'a str>, option: &[&str], value_name: &str) -> Option<Vec<&'a str>> {
    let mut i = 0;
    let mut values = vec![];
    while i < args.len() {
        if option.iter().any(|&o| args[i] == o) {
            let next_i = i + 1;
            if next_i == args.len() || args[next_i].starts_with('-') {
                fatal(f!("expected value for option {} {value_name}", args[i]));
            }

            args.remove(i); // remove option
            values.push(args.remove(i)) // take value.
        }
        i += 1;
    }

    if values.is_empty() {
        None
    } else {
        Some(values)
    }
}

// Parses the initial input. Returns ("task-name", ["task", "args"]).
pub fn args() -> (&'static str, Vec<&'static str>) {
    let mut args: Vec<_> = env::args().skip(1).collect();
    if args.is_empty() {
        return ("", vec![]);
    }
    let task = Box::leak(args.remove(0).into_boxed_str());
    let mut args = args.into_iter().map(|a| Box::leak(a.into_boxed_str()) as &'static str).collect();

    // set task name and flags
    TaskInfo::set_name(task);
    TaskInfo::set_dump(take_flag(&mut args, &["--dump"]));

    // prints header
    println(f!("{}Running{}: {}{} {:?} {:?}", c_green(), c_wb(), do_cmd(), c_w(), task, args));

    (task, args)
}

// Information about the running task.
pub struct TaskInfo {
    pub name: &'static str,
    pub dump: bool,
    pub stdout_dump: &'static str,
    pub stderr_dump: &'static str,
    dump_files: Option<std::collections::HashMap<&'static str, std::fs::File>>,
}
static TASK_INFO: std::sync::Mutex<TaskInfo> = std::sync::Mutex::new(TaskInfo {
    name: "",
    dump: false,
    stdout_dump: "dump.log",
    stderr_dump: "dump.log",
    dump_files: None,
});
impl TaskInfo {
    pub fn name() -> &'static str {
        TASK_INFO.try_lock().unwrap().name
    }

    pub fn set_name(name: &'static str) {
        TASK_INFO.try_lock().unwrap().name = name;
    }

    // Get if "--dump" redirect is enabled.
    pub fn dump() -> bool {
        TASK_INFO.try_lock().unwrap().dump
    }

    pub fn set_dump(dump: bool) {
        TASK_INFO.try_lock().unwrap().dump = dump;
    }

    // Get the stdout dump stream.
    pub fn stdout_dump() -> Option<std::fs::File> {
        let mut info = TASK_INFO.try_lock().unwrap();
        let file = info.stdout_dump;
        info.dump_file(file)
    }

    pub fn set_stdout_dump(stdout_dump: &'static str) {
        TASK_INFO.try_lock().unwrap().stdout_dump = stdout_dump;
    }

    // Get the stderr dump stream.
    pub fn stderr_dump() -> Option<std::fs::File> {
        let mut info = TASK_INFO.try_lock().unwrap();
        let file = info.stderr_dump;
        info.dump_file(file)
    }

    fn dump_file(&mut self, file: &'static str) -> Option<std::fs::File> {
        if !self.dump || file.is_empty() {
            return None;
        }

        match self.dump_files.get_or_insert_with(std::collections::HashMap::new).entry(file) {
            std::collections::hash_map::Entry::Occupied(e) => e.get().try_clone().ok(),
            std::collections::hash_map::Entry::Vacant(e) => match std::fs::File::create(file) {
                Ok(f) => e.insert(f).try_clone().ok(),
                Err(_) => None,
            },
        }
    }
}

// Get all paths to `dir/*/Cargo.toml`
pub fn top_cargo_toml(dir: &str) -> Vec<String> {
    glob(&format!("{dir}/*/Cargo.toml"))
}

/// Get all `dir/**/*`.ext files.
pub fn all_ext(dir: &str, ext: &str) -> Vec<String> {
    glob(&format!("{dir}/**/*.{ext}"))
}

// Get all `examples/*.rs` file names.
pub fn examples() -> Vec<String> {
    match glob::glob("examples/*.rs") {
        Ok(iter) => iter
            .filter_map(|r| match r {
                Ok(p) => p.file_name().map(|n| n.to_string_lossy().trim_end_matches(".rs").to_owned()),
                Err(e) => {
                    error(e);
                    None
                }
            })
            .collect(),
        Err(e) => {
            error(e);
            return vec![];
        }
    }
}

// [[bin]] names for build tests last run ("bin-name", "test_file_path").
pub fn build_test_cases() -> Vec<(String, String)> {
    match std::fs::read_to_string("target/tests/build-tests/Cargo.toml") {
        Ok(file) => {
            let mut bin_names = vec![];

            let mut lines = file.lines();
            while let Some(line) = lines.next() {
                if line == "[[bin]]" {
                    if let (Some(name_line), Some(path_line)) = (lines.next(), lines.next()) {
                        assert!(name_line.starts_with("name = "));
                        assert!(path_line.starts_with("path = "));

                        let name = name_line["name = ".len()..].trim_matches('"');
                        if name.starts_with("trybuild") {
                            let path = path_line["path = ".len()..].trim_matches('"').replace(r#"\\"#, "\\");
                            bin_names.push((name.to_owned(), path));
                        }
                    }
                }
            }

            bin_names
        }
        Err(e) => {
            error(e);
            vec![]
        }
    }
}

// Get "cdylib" crate output.
pub fn cdylib_files(path: impl Into<PathBuf>) -> Vec<PathBuf> {
    let mut path = path.into();
    let file_name = path.file_name().unwrap().to_string_lossy();

    let linux = format!("lib{file_name}.so");
    let windows = format!("{file_name}.dll");
    let macos = format!("lib{file_name}.dylib");

    let mut r = vec![];

    path.set_file_name(linux);
    if path.exists() {
        r.push(path.clone());
    }
    path.set_file_name(windows);
    if path.exists() {
        r.push(path.clone());
    }
    path.set_file_name(macos);
    if path.exists() {
        r.push(path);
    }

    r
}

/*
// Extracts the file name from path, or panics.
pub fn file_name(path: &str) -> String {
    std::path::PathBuf::from(path).file_name().unwrap().to_str().unwrap().to_owned()
}
*/

pub fn glob(pattern: &str) -> Vec<String> {
    match glob::glob(pattern) {
        Ok(iter) => iter
            .filter_map(|r| match r {
                Ok(p) => Some(p.to_string_lossy().replace("\\", "/")),
                Err(e) => {
                    error(e);
                    None
                }
            })
            .collect(),
        Err(e) => {
            error(e);
            return vec![];
        }
    }
}

pub fn println(msg: impl std::fmt::Display) {
    if let Some(mut dump) = TaskInfo::stdout_dump() {
        writeln!(dump, "{msg}").ok();
    } else {
        println!("{msg}");
    }
}
pub fn print(msg: impl std::fmt::Display) {
    if let Some(mut dump) = TaskInfo::stdout_dump() {
        write!(dump, "{msg}").ok();
    } else {
        print!("{msg}");
        std::io::stdout().lock().flush().ok();
    }
}

// Do `action` in background thread after `delay_secs`.
pub fn do_after(delay_secs: u64, action: impl FnOnce() + Send + 'static) {
    use std::thread;
    use std::time::Duration;
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(delay_secs));
        action();
    });
}

// Prints an error message, use `error(f!("{}", .."))` for formatting.
pub fn error(msg: impl std::fmt::Display) {
    if let Some(mut dump) = TaskInfo::stderr_dump() {
        writeln!(dump, "{}error{}: {}{}", c_red(), c_wb(), c_w(), msg).ok();
    } else {
        eprintln!("{}error{}: {}{}", c_red(), c_wb(), c_w(), msg);
    }
}

// Prints an [`error`] and exits with code `-1`.
pub fn fatal(msg: impl std::fmt::Display) -> ! {
    error(msg);
    process::exit(-1)
}

// ANSI colors.
pub fn c_green() -> &'static str {
    color("\x1B[1;32m")
}
pub fn c_red() -> &'static str {
    color("\x1B[1;31m")
}
pub fn c_wb() -> &'static str {
    color("\x1B[1;37m")
}
pub fn c_w() -> &'static str {
    color("\x1B[0m")
}
fn color(color: &str) -> &str {
    if TaskInfo::dump() || !ansi_enabled() {
        ""
    } else {
        color
    }
}
#[allow(unreachable_code)]
fn ansi_enabled() -> bool {
    #[cfg(windows)]
    {
        use std::sync::atomic::*;

        static ENABLED: AtomicU8 = AtomicU8::new(0);

        return match ENABLED.load(Ordering::Relaxed) {
            0 => {
                let enabled = ansi_term::enable_ansi_support().is_ok();
                ENABLED.store(if enabled { 1 } else { 2 }, Ordering::Relaxed);
                enabled
            }
            n => n == 1,
        };
    }

    cfg!(unix)
}

pub fn settings_path() -> PathBuf {
    std::env::current_exe().unwrap().parent().unwrap().to_owned()
}
