use std::{
    collections::HashMap,
    fs,
    io::{self, BufReader, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::AtomicBool,
};

use semver::{Version, VersionReq};
use serde::Deserialize;

/// Print warning message.
macro_rules! warn {
    ($($format_args:tt)*) => {
        if $crate::util::deny_warnings() {
            error!($($format_args)*);
        }else {
            eprintln!("{} {}", $crate::util::WARN_PREFIX, format_args!($($format_args)*));
        }
    };
}

pub fn deny_warnings() -> bool {
    std::env::var("RUSTFLAGS")
        .map(|f| {
            ["--deny=warnings", "-Dwarnings", "-D warnings", "--deny warnings"]
                .iter()
                .any(|d| f.contains(d))
        })
        .unwrap_or(false)
}

/// Print error message and flags the current process as failed.
///
/// Note that this does not exit the process, use `fatal!` to exit.
macro_rules! error {
    ($($format_args:tt)*) => {
        {
            $crate::util::set_failed_run(true);
            eprintln!("{} {}", $crate::util::ERROR_PREFIX, format_args!($($format_args)*));
        }
    };
}

pub static WARN_PREFIX: &str = color_print::cstr!("<bold><yellow>warning</yellow>:</bold>");
pub static ERROR_PREFIX: &str = color_print::cstr!("<bold><red>error</red>:</bold>");

/// Print error message and exit the current process with error code.
macro_rules! fatal {
    ($($format_args:tt)*) => {
        {
            error!($($format_args)*);
            $crate::util::exit();
        }
    };
}

static RUN_FAILED: AtomicBool = AtomicBool::new(false);

/// Gets if the current process will exit with error code.
pub fn is_failed_run() -> bool {
    RUN_FAILED.load(std::sync::atomic::Ordering::SeqCst)
}

/// Sets if the current process will exit with error code.
pub fn set_failed_run(failed: bool) {
    RUN_FAILED.store(failed, std::sync::atomic::Ordering::SeqCst);
}

/// Exit the current process, with error code `102` if [`is_failed_run`].
pub fn exit() -> ! {
    if is_failed_run() {
        std::process::exit(102)
    } else {
        std::process::exit(0)
    }
}

/// Run the command with args, inherits stdout and stderr.
pub fn cmd(line: &str, args: &[&str], env: &[(&str, &str)]) -> io::Result<()> {
    cmd_impl(line, args, env, false)
}
/// Run the command with args.
pub fn cmd_silent(line: &str, args: &[&str], env: &[(&str, &str)]) -> io::Result<()> {
    cmd_impl(line, args, env, true)
}
fn cmd_impl(line: &str, args: &[&str], env: &[(&str, &str)], silent: bool) -> io::Result<()> {
    let mut line_parts = line.split(' ');
    let program = line_parts.next().expect("expected program to run");
    let mut cmd = Command::new(program);
    cmd.args(
        line_parts
            .map(|a| {
                let a = a.trim();
                if a.starts_with('"') { a.trim_matches('"') } else { a }
            })
            .filter(|a| !a.is_empty()),
    );
    cmd.args(args.iter().filter(|a| !a.is_empty()));
    for (key, val) in env.iter() {
        cmd.env(key, val);
    }

    if silent {
        let output = cmd.output()?;
        if output.status.success() {
            Ok(())
        } else {
            let mut cmd = format!("cmd failed: {line}");
            for arg in args {
                cmd.push(' ');
                cmd.push_str(arg);
            }
            cmd.push('\n');
            cmd.push_str(&String::from_utf8_lossy(&output.stderr));
            Err(io::Error::other(cmd))
        }
    } else {
        let status = cmd.status()?;
        if status.success() {
            Ok(())
        } else {
            let mut cmd = format!("cmd failed: {line}");
            for arg in args {
                cmd.push(' ');
                cmd.push_str(arg);
            }
            Err(io::Error::other(cmd))
        }
    }
}

pub fn workspace_dir() -> Option<PathBuf> {
    let output = std::process::Command::new("cargo")
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .ok()?;

    if output.status.success() {
        let cargo_path = Path::new(std::str::from_utf8(&output.stdout).unwrap().trim());
        Some(cargo_path.parent().unwrap().to_owned())
    } else {
        None
    }
}

pub fn ansi_enabled() -> bool {
    std::env::var("NO_COLOR").is_err()
}

pub fn clean_value(value: &str, required: bool) -> io::Result<String> {
    let mut first_char = false;
    let clean_value: String = value
        .chars()
        .filter(|c| {
            if first_char {
                first_char = c.is_ascii_alphabetic();
                first_char
            } else {
                *c == ' ' || *c == '-' || *c == '_' || c.is_ascii_alphanumeric()
            }
        })
        .collect();
    let clean_value = clean_value.trim().to_owned();

    if required && clean_value.is_empty() {
        if clean_value.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("cannot derive clean value from `{value}`, must contain at least one ascii alphabetic char"),
            ));
        }
        if clean_value.len() > 62 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("cannot derive clean value from `{value}`, must contain <= 62 ascii alphanumeric chars"),
            ));
        }
    }
    Ok(clean_value)
}

pub fn manifest_path_from_package(package: &str) -> Option<String> {
    let metadata = match Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .stderr(Stdio::inherit())
        .output()
    {
        Ok(m) => {
            if !m.status.success() {
                fatal!("cargo metadata error")
            }
            String::from_utf8_lossy(&m.stdout).into_owned()
        }
        Err(e) => fatal!("cargo metadata error, {e}"),
    };

    #[derive(Deserialize)]
    struct Metadata {
        packages: Vec<Package>,
    }
    #[derive(Deserialize)]
    struct Package {
        name: String,
        manifest_path: String,
    }
    let metadata: Metadata = serde_json::from_str(&metadata).unwrap_or_else(|e| fatal!("unexpected cargo metadata format, {e}"));

    for p in metadata.packages {
        if p.name == package {
            return Some(p.manifest_path);
        }
    }
    None
}

/// Workspace crates Cargo.toml paths.
pub fn workspace_manifest_paths() -> Vec<PathBuf> {
    let metadata = match Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .stderr(Stdio::inherit())
        .output()
    {
        Ok(m) => {
            if !m.status.success() {
                fatal!("cargo metadata error")
            }
            String::from_utf8_lossy(&m.stdout).into_owned()
        }
        Err(e) => fatal!("cargo metadata error, {e}"),
    };

    #[derive(Deserialize)]
    struct Metadata {
        packages: Vec<Package>,
    }
    #[derive(Debug, Deserialize)]
    struct Package {
        manifest_path: PathBuf,
    }

    let metadata: Metadata = serde_json::from_str(&metadata).unwrap_or_else(|e| fatal!("unexpected cargo metadata format, {e}"));

    metadata.packages.into_iter().map(|p| p.manifest_path).collect()
}

/// Workspace root and dependencies of manifest_path
pub fn dependencies(manifest_path: &str) -> (PathBuf, Vec<DependencyManifest>) {
    let metadata = match Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(manifest_path)
        .stderr(Stdio::inherit())
        .output()
    {
        Ok(m) => {
            if !m.status.success() {
                fatal!("cargo metadata error")
            }
            String::from_utf8_lossy(&m.stdout).into_owned()
        }
        Err(e) => fatal!("cargo metadata error, {e}"),
    };

    #[derive(Deserialize)]
    struct Metadata {
        packages: Vec<Package>,
        workspace_root: PathBuf,
    }
    #[derive(Debug, Deserialize)]
    struct Package {
        name: String,
        version: Version,
        dependencies: Vec<Dependency>,
        manifest_path: String,
    }
    #[derive(Debug, Deserialize)]
    struct Dependency {
        name: String,
        kind: Option<String>,
        req: VersionReq,
    }

    let metadata: Metadata = serde_json::from_str(&metadata).unwrap_or_else(|e| fatal!("unexpected cargo metadata format, {e}"));

    let manifest_path = dunce::canonicalize(manifest_path).unwrap();

    let mut dependencies: &[Dependency] = &[];

    for pkg in &metadata.packages {
        let pkg_path = Path::new(&pkg.manifest_path);
        if pkg_path == manifest_path {
            dependencies = &pkg.dependencies;
            break;
        }
    }
    if !dependencies.is_empty() {
        let mut map = HashMap::new();
        for pkg in &metadata.packages {
            map.entry(pkg.name.as_str()).or_insert_with(Vec::new).push((&pkg.version, pkg));
        }

        let mut r = vec![];
        fn collect(map: &mut HashMap<&str, Vec<(&Version, &Package)>>, dependencies: &[Dependency], r: &mut Vec<DependencyManifest>) {
            for dep in dependencies {
                if dep.kind.is_some() {
                    // skip build/dev-dependencies
                    continue;
                }
                if let Some(versions) = map.remove(dep.name.as_str()) {
                    for (version, pkg) in versions.iter() {
                        if dep.req.comparators.is_empty() || dep.req.matches(version) {
                            r.push(DependencyManifest {
                                name: pkg.name.clone(),
                                version: pkg.version.clone(),
                                manifest_path: pkg.manifest_path.as_str().into(),
                            });

                            // collect dependencies of dependencies
                            collect(map, &pkg.dependencies, r)
                        }
                    }
                }
            }
        }
        collect(&mut map, dependencies, &mut r);
        return (metadata.workspace_root, r);
    }

    (metadata.workspace_root, vec![])
}

pub struct DependencyManifest {
    pub name: String,
    pub version: Version,
    pub manifest_path: PathBuf,
}

pub fn check_or_create_dir(check: bool, path: impl AsRef<Path>) -> io::Result<()> {
    if check {
        let path = path.as_ref();
        if !path.is_dir() {
            fatal!("expected `{}` dir", path.display());
        }
        Ok(())
    } else {
        fs::create_dir(path)
    }
}

pub fn check_or_create_dir_all(check: bool, path: impl AsRef<Path>) -> io::Result<()> {
    if check {
        let path = path.as_ref();
        if !path.is_dir() {
            fatal!("expected `{}` dir", path.display());
        }
        Ok(())
    } else {
        fs::create_dir_all(path)
    }
}

pub fn check_or_write(check: bool, path: impl AsRef<Path>, contents: impl AsRef<[u8]>, verbose: bool) -> io::Result<()> {
    let path = path.as_ref();
    let contents = contents.as_ref();
    if check {
        if !path.is_file() {
            fatal!("expected `{}` file", path.display());
        }
        let file = fs::File::open(path).unwrap_or_else(|e| fatal!("cannot read `{}`, {e}", path.display()));
        let mut bytes = vec![];
        BufReader::new(file)
            .read_to_end(&mut bytes)
            .unwrap_or_else(|e| fatal!("cannot read `{}`, {e}", path.display()));

        if bytes != contents {
            fatal!("file `{}` contents changed", path.display());
        } else if verbose {
            println!("file `{}` contents did not change", path.display());
        }

        Ok(())
    } else {
        if verbose {
            println!("writing `{}`", path.display());
        }
        fs::write(path, contents)
    }
}

pub fn check_or_copy(check: bool, from: impl AsRef<Path>, to: impl AsRef<Path>, verbose: bool) -> io::Result<u64> {
    let from = from.as_ref();
    let to = to.as_ref();
    if check {
        if !to.is_file() {
            fatal!("expected `{}` file", to.display());
        }

        let mut bytes = vec![];
        for path in [from, to] {
            let file = fs::File::open(path).unwrap_or_else(|e| fatal!("cannot read `{}`, {e}", path.display()));
            let mut b = vec![];
            BufReader::new(file)
                .read_to_end(&mut b)
                .unwrap_or_else(|e| fatal!("cannot read `{}`, {e}", path.display()));

            bytes.push(b);
        }

        if bytes[0] != bytes[1] {
            fatal!("file `{}` contents changed", to.display());
        } else if verbose {
            println!("file `{}` contents did not change", to.display());
        }

        Ok(bytes[1].len() as u64)
    } else {
        if verbose {
            println!("copying\n  from: `{}`\n    to: `{}`", from.display(), to.display());
        }
        fs::copy(from, to)
    }
}
