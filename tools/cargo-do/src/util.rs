use std::env;
use std::format_args as f;
use std::io::Write;
use std::path::PathBuf;
use std::process::{self, Command, Stdio};
use std::sync::atomic::*;

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
pub fn cmd_req(cmd: &str, default_args: &[&str], user_args: &[&str]) {
    cmd_impl(cmd, default_args, user_args, &[], true)
}
// Like [`cmd_env`] but exists the task runner if the command fails.
pub fn cmd_env_req(cmd: &str, default_args: &[&str], user_args: &[&str], envs: &[(&str, &str)]) {
    cmd_impl(cmd, default_args, user_args, envs, true)
}
fn cmd_impl(mut cmd: &str, default_args: &[&str], user_args: &[&str], envs: &[(&str, &str)], required: bool) {
    let mut args: Vec<_> = default_args
        .iter()
        .chain(user_args.iter())
        .filter(|a| !a.is_empty())
        .map(|s| *s)
        .collect();

    if cfg!(windows) && cmd == "cargo" && default_args.first() == Some(&"+nightly") {
        // nested cargo calls don't use rustup's cargo on Windows.
        // https://github.com/rust-lang/rustup/issues/3036
        //
        // rustup run nightly cargo
        cmd = "rustup";
        args[0] = "run";
        args.insert(1, "nightly");
        args.insert(2, "cargo");
    }

    let mut cmd = Command::new(cmd);
    cmd.args(&args[..]);
    cmd.envs(envs.iter().filter(|t| !t.0.is_empty() && !t.1.is_empty()).copied());

    for (remove, _) in envs.iter().filter(|t| !t.0.is_empty() && t.1.is_empty()) {
        cmd.env_remove(remove);
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
                    set_exit_with_error();
                }
            }
        }
        Err(e) => {
            let msg = format!("task {:?} failed to run, {e}", TaskInfo::name());
            if required {
                fatal(msg)
            } else {
                error(msg);
                set_exit_with_error();
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
        // We don't need a delay in Unix because it naturally permits repainting
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

    if values.is_empty() { None } else { Some(values) }
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

// Get all `examples/*/src/main.rs` names.
pub fn examples() -> Vec<String> {
    match glob::glob("examples/*/src/main.rs") {
        Ok(iter) => iter
            .filter_map(|r| match r {
                Ok(p) => Some(p.parent()?.parent()?.file_name()?.to_string_lossy().into_owned()),
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

// [[bin]] names for macro tests last run ("bin-name", "test_file_path").
pub fn macro_test_cases() -> Vec<(String, String)> {
    match std::fs::read_to_string("target/tests/macro-tests/Cargo.toml") {
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
    set_exit_with_error();
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
    if TaskInfo::dump() || !ansi_enabled() { "" } else { color }
}
#[allow(unreachable_code)]
fn ansi_enabled() -> bool {
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }

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
    dunce::canonicalize(std::env::current_exe().unwrap())
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}

pub fn git_modified() -> Vec<PathBuf> {
    let output = Command::new("git")
        .args(&["ls-files", "-m"])
        .output()
        .expect("failed to run `git ls-files -m`");
    let output = String::from_utf8(output.stdout).unwrap();
    output.lines().map(PathBuf::from).collect()
}

pub fn print_git_diff(file: &std::path::Path) {
    Command::new("git").arg("--no-pager").arg("diff").arg(file).status().unwrap();
}

static CMD_ERROR: AtomicBool = AtomicBool::new(false);

pub fn set_exit_with_error() {
    CMD_ERROR.store(true, Ordering::Relaxed);
}

pub fn exit_checked() {
    if CMD_ERROR.load(Ordering::Relaxed) {
        std::process::exit(-1);
    } else {
        std::process::exit(0);
    }
}

pub fn crate_version(name: &str) -> String {
    let path = format!(
        "{manifest_dir}/../../crates/{name}/Cargo.toml",
        manifest_dir = env!("CARGO_MANIFEST_DIR")
    );
    let toml = std::fs::read_to_string(&path).expect(&path);
    assert!(toml.contains(&format!("name = \"{name}\"")), "run `do` in the project root");
    let rgx = regex::Regex::new(r#"version = "(\d+\.\d+.*)""#).unwrap();
    rgx.captures(&toml).unwrap().get(1).unwrap().as_str().to_owned()
}

pub fn git_tag_exists(tag: &str) -> bool {
    let output = Command::new("git")
        .args(&["tag", "--list", tag])
        .output()
        .expect("failed to run `git ls-files -m`");
    let output = String::from_utf8(output.stdout).unwrap();
    output.lines().any(|l| l == tag)
}

pub fn publish_members() -> Vec<PublishMember> {
    let mut members = vec![];
    'members: for member in top_cargo_toml("crates") {
        match std::fs::read_to_string(member) {
            Ok(file) => {
                let mut member = PublishMember {
                    name: String::new(),
                    version: (0, 0, 0),
                    dependencies: vec![],
                    features: vec![],
                };

                enum Section {
                    Package,
                    Dependencies,
                    Other,
                    Features,
                }
                let mut section = Section::Other;

                for line in file.lines() {
                    let line = line.trim();
                    if line.starts_with("#") || line.is_empty() {
                        continue;
                    }
                    if line == "[package]" {
                        section = Section::Package;
                    } else if line == "[features]" {
                        section = Section::Features;
                    } else if line.ends_with("dependencies]") {
                        section = Section::Dependencies;
                    } else if line.starts_with('[') && line.ends_with(']') {
                        section = Section::Other;
                    }

                    match section {
                        Section::Package => {
                            if let Some(name) = line.strip_prefix("name = ") {
                                member.name = name.trim_matches('"').to_owned();
                            } else if let Some(version) = line.strip_prefix("version = ") {
                                member.version = parse_publish_version(version.trim_matches('"'));
                            } else if line == "publish = false" {
                                continue 'members;
                            }
                        }
                        Section::Features => {
                            if let Some((feat, _)) = line.split_once(" = [") {
                                if feat != "default" {
                                    member.features.push(feat.trim_end().to_owned());
                                }
                            }
                        }
                        Section::Dependencies => {
                            if line.contains(r#"path = "../"#) {
                                if let Some((name, rest)) = line.split_once(" = ") {
                                    let version_match = r#"version = ""#;
                                    if let Some(i) = rest.find(version_match) {
                                        let rest = &rest[i + version_match.len()..];
                                        let i = rest.find('"').unwrap();

                                        member.dependencies.push(PublishDependency {
                                            name: name.to_owned(),
                                            version: parse_publish_version(&rest[..i]),
                                        });
                                    }
                                }
                            }
                        }
                        Section::Other => {}
                    }
                }

                if !member.name.is_empty() {
                    members.push(member);
                }
            }
            Err(e) => {
                error(e);
                continue 'members;
            }
        }
    }
    topological_sort(&mut members);
    members
}
fn parse_publish_version(version: &str) -> (u32, u32, u32) {
    fn parse(n: Option<&str>) -> u32 {
        if let Some(n) = n {
            match n.parse() {
                Ok(n) => n,
                Err(e) => {
                    error(f!("{e}, expected version #.#.# in local dependency"));
                    0
                }
            }
        } else {
            error("expected version #.#.# in local dependency");
            0
        }
    }

    let mut parts = version.split('.');

    (parse(parts.next()), parse(parts.next()), parse(parts.next()))
}
fn topological_sort(members: &mut Vec<PublishMember>) {
    let mut sort = topological_sort::TopologicalSort::<String>::new();
    for member in members.iter() {
        sort.insert(member.name.clone());
        for dep in &member.dependencies {
            sort.add_dependency(dep.name.clone(), member.name.clone());
        }
    }
    let mut sorted = Vec::with_capacity(members.len());
    while let Some(t) = sort.pop() {
        sorted.push(t);
    }
    assert!(sort.is_empty());
    members.sort_by_key(|m| sorted.iter().position(|n| n == &m.name).unwrap());
}

#[derive(Debug)]
pub struct PublishMember {
    pub name: String,
    pub version: (u32, u32, u32),
    pub dependencies: Vec<PublishDependency>,
    pub features: Vec<String>,
}
impl std::fmt::Display for PublishMember {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        writeln!(f, "name = \"{}\"", self.name)?;
        writeln!(f, "version = \"{}.{}.{}\"", self.version.0, self.version.1, self.version.2)?;
        writeln!(f, "[dependencies]")?;
        for d in &self.dependencies {
            writeln!(f, "{}", d)?;
        }
        writeln!(f, "[features]")?;
        for feat in &self.features {
            writeln!(f, "{feat} = [...]")?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct PublishDependency {
    pub name: String,
    pub version: (u32, u32, u32),
}
impl std::fmt::Display for PublishDependency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "{} = {{ path = \"../{}\", version = \"{}.{}.{}\" }}",
            self.name, self.name, self.version.0, self.version.1, self.version.2
        )
    }
}

impl PublishMember {
    pub fn write_versions(&self, versions: &std::collections::HashMap<&str, (u32, u32, u32)>, dry_run: bool) {
        if !versions.contains_key(self.name.as_str()) && !self.dependencies.iter().any(|d| versions.contains_key(d.name.as_str())) {
            return;
        }

        use std::fmt::Write as _;

        let cargo_path = PathBuf::from("crates").join(&self.name).join("Cargo.toml");
        let cargo = std::fs::read_to_string(&cargo_path).expect("failed to load Cargo.toml");
        let mut output = String::with_capacity(cargo.len());

        enum Section {
            Package,
            Dependencies,
            Other,
        }
        let mut section = Section::Other;

        'line: for line in cargo.lines() {
            let line_edit = line.trim();
            if line_edit == "[package]" {
                section = Section::Package;
            } else if line_edit.ends_with("dependencies]") {
                section = Section::Dependencies;
            } else if line_edit.starts_with('[') && line_edit.ends_with(']') {
                section = Section::Other;
            }

            match section {
                Section::Package => {
                    if line_edit.starts_with("version = ") {
                        if let Some(v) = versions.get(self.name.as_str()) {
                            write!(&mut output, "version = \"{}.{}.{}\"\n", v.0, v.1, v.2).unwrap();

                            print(f!(
                                "{} {}.{}.{} -> {}.{}.{}\n",
                                self.name,
                                self.version.0,
                                self.version.1,
                                self.version.2,
                                v.0,
                                v.1,
                                v.2,
                            ));

                            continue 'line;
                        }
                    }
                }
                Section::Dependencies => {
                    if line_edit.contains(r#"path = "../"#) {
                        if let Some((name, _)) = line_edit.split_once(" = ") {
                            if let Some(v) = versions.get(name) {
                                let version_match = r#"version = ""#;
                                if let Some(s) = line_edit.find(version_match) {
                                    let rest = &line_edit[s + version_match.len()..];
                                    let e = rest.find('"').unwrap();

                                    write!(
                                        &mut output,
                                        "{}version = \"{}.{}.{}{}\n",
                                        &line_edit[..s],
                                        v.0,
                                        v.1,
                                        v.2,
                                        &rest[e..]
                                    )
                                    .unwrap();
                                    continue 'line;
                                }
                            }
                        }
                    }
                }
                Section::Other => {}
            }

            output.push_str(line);
            output.push('\n');
        }

        if !dry_run {
            if let Err(e) = std::fs::write(cargo_path, output) {
                error(e);
            }
        }
    }
}

pub fn crates_io_latest(crate_name: &str) -> String {
    /*
    Packages with 1 character names are placed in a directory named 1.
    Packages with 2 character names are placed in a directory named 2.
    Packages with 3 character names are placed in the directory 3/{first-character} where {first-character} is the first character of the package name.
    All other packages are stored in directories named {first-two}/{second-two} where the top directory is the first two characters of the package name, and the next subdirectory is the third and fourth characters of the package name.
     */

    let index = match crate_name.len() {
        1 => "1".to_owned(),
        2 => "2".to_owned(),
        3 => format!("3/{}", &crate_name[..1]),
        _ => format!("{}/{}", &crate_name[..2], &crate_name[2..4]),
    };

    let url = format!("https://index.crates.io/{index}/{crate_name}");

    let output = std::process::Command::new("curl")
        .arg("--location")
        .arg("--fail")
        .arg("--silent")
        .arg("--show-error")
        .arg(&url)
        .output()
        .expect("failed to run `git ls-files -m`");

    let err = String::from_utf8(output.stderr).unwrap();
    if !err.is_empty() {
        if !err.contains("error: 404") {
            error(err);
        }
        String::new()
    } else {
        let output = String::from_utf8(output.stdout).unwrap();
        let latest = output.lines().last().unwrap();

        let vers_match = r#""vers":""#;
        let i = latest.find(vers_match).unwrap();

        let latest = &latest[i + vers_match.len()..];
        let i = latest.find('"').unwrap();

        latest[..i].to_owned()
    }
}

pub fn fix_git_config_name_email() {
    if !git_has_config("user.name") {
        cmd("git", &["config", "user.name"], &[get_git_log("--pretty=format:%an").as_str()]);
    }
    if !git_has_config("user.email") {
        cmd("git", &["config", "user.email"], &[get_git_log("--pretty=format:%ae").as_str()]);
    }
}

fn git_has_config(key: &str) -> bool {
    let output = std::process::Command::new("git")
        .arg("config")
        .arg(key)
        .output()
        .expect("failed to run `git config ..`");

    !String::from_utf8(output.stdout).unwrap().is_empty()
}

fn get_git_log(fmt: &str) -> String {
    let output = std::process::Command::new("git")
        .arg("log")
        .arg("-n")
        .arg("1")
        .arg(fmt)
        .output()
        .expect("failed to run `git log ..`");

    String::from_utf8(output.stdout).unwrap()
}

pub fn get_git_diff(from: &str, to: &str) -> String {
    let output = std::process::Command::new("git")
        .arg("diff")
        .arg(from)
        .arg(to)
        .arg("--name-only")
        .output()
        .expect("failed to run `git log ..`");

    String::from_utf8(output.stdout).unwrap()
}
