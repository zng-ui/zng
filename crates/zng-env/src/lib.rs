#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Process environment directories and unique name.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::{
    fs,
    io::{self, BufRead},
    path::{Path, PathBuf},
    str::FromStr,
};

use semver::Version;
use zng_txt::Txt;
use zng_unique_id::{lazy_static, lazy_static_init};
mod process;
pub use process::*;

lazy_static! {
    static ref ABOUT: About = About::fallback_name();
}

/// Inits process metadata, calls process start handlers and defines the process lifetime in `main`.
///
/// This **must** be called in main.
///
/// Init [`about`] an [`About`] for the process metadata. See [`on_process_start!`] for process start handlers.
/// See [`on_exit`] for exit handlers called at the end of the `main` function.
///
/// # Process Start
///
/// A single Zng executable can be built with multiple components that spawn different instances
/// of the executable that must run as different processes. If the current instance is requested
/// by component `init!` runs it and exits the process, never returning flow to the normal main function.
///
/// ```
/// # mod zng { pub use zng_env as env; }
/// fn main() {
///     println!("print in all processes");
///     zng::env::init!();
///     println!("print only in the app-process");
///
///     // directories are available after `init!`.
///     let _res = zng::env::res("");
///     
///     // APP.defaults().run(...);
///
///     // on_exit handlers are called here
/// }
/// ```
#[macro_export]
macro_rules! init {
    () => {
        let _on_main_exit = $crate::init_parse!($crate);
    };
}
#[doc(hidden)]
pub use zng_env_proc_macros::init_parse;

#[doc(hidden)]
pub fn init(about: About) -> impl Drop {
    if lazy_static_init(&ABOUT, about).is_err() {
        panic!("env already inited, env::init must be the first call in the process")
    }
    process_init()
}

/// Metadata about the app and main crate.
///
/// See [`about`] for more details.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct About {
    /// package.name
    pub pkg_name: Txt,
    /// package.authors
    pub pkg_authors: Box<[Txt]>,
    /// package.name in snake_case
    pub crate_name: Txt,
    /// package.version
    pub version: Version,
    /// package.metadata.zng.about.app or `pkg_name`
    pub app: Txt,
    /// package.metadata.zng.about.org or the first `pkg_authors`
    pub org: Txt,
    /// package.metadata.zng.about.qualifier
    ///
    /// Reverse domain name notation, excluding the name of the application.
    pub qualifier: Txt,
    /// package.description
    pub description: Txt,
    /// package.homepage
    pub homepage: Txt,
    /// If package.metadata.zng.about is set on the Cargo.toml manifest.
    ///
    /// The presence of this section is used by `cargo zng res` to find the main
    /// crate if the workspace has multiple bin crates.
    pub has_about: bool,
}
impl About {
    fn fallback_name() -> Self {
        Self {
            pkg_name: Txt::from_static(""),
            pkg_authors: Box::new([]),
            version: Version::new(0, 0, 0),
            app: fallback_name(),
            crate_name: Txt::from_static(""),
            org: Txt::from_static(""),
            qualifier: Txt::from_static(""),
            description: Txt::from_static(""),
            homepage: Txt::from_static(""),
            has_about: false,
        }
    }

    /// Parse a Cargo.toml string.
    pub fn parse_manifest(cargo_toml: &str) -> Result<Self, toml::de::Error> {
        let m: Manifest = toml::from_str(cargo_toml)?;
        let mut about = About {
            crate_name: m.package.name.replace('-', "_").into(),
            pkg_name: m.package.name,
            pkg_authors: m.package.authors,
            version: m.package.version,
            description: m.package.description.unwrap_or_default(),
            homepage: m.package.homepage.unwrap_or_default(),
            app: Txt::from_static(""),
            org: Txt::from_static(""),
            qualifier: Txt::from_static(""),
            has_about: false,
        };
        if let Some(m) = m.package.metadata.and_then(|m| m.zng).and_then(|z| z.about) {
            about.has_about = true;
            about.app = m.app.unwrap_or_default();
            about.org = m.org.unwrap_or_default();
            about.qualifier = m.qualifier.unwrap_or_default();
        }
        if about.app.is_empty() {
            about.app = about.pkg_name.clone();
        }
        if about.org.is_empty() {
            about.org = about.pkg_authors.first().cloned().unwrap_or_default();
        }
        Ok(about)
    }

    #[doc(hidden)]
    #[allow(clippy::too_many_arguments)]
    pub fn macro_new(
        pkg_name: &'static str,
        pkg_authors: &[&'static str],
        crate_name: &'static str,
        (major, minor, patch, pre, build): (u64, u64, u64, &'static str, &'static str),
        app: &'static str,
        org: &'static str,
        qualifier: &'static str,
        description: &'static str,
        homepage: &'static str,
        has_about: bool,
    ) -> Self {
        Self {
            pkg_name: Txt::from_static(pkg_name),
            pkg_authors: pkg_authors.iter().copied().map(Txt::from_static).collect(),
            crate_name: Txt::from_static(crate_name),
            version: {
                let mut v = Version::new(major, minor, patch);
                v.pre = semver::Prerelease::from_str(pre).unwrap();
                v.build = semver::BuildMetadata::from_str(build).unwrap();
                v
            },
            app: Txt::from_static(app),
            org: Txt::from_static(org),
            qualifier: Txt::from_static(qualifier),
            description: Txt::from_static(description),
            homepage: Txt::from_static(homepage),
            has_about,
        }
    }
}
#[derive(serde::Deserialize)]
struct Manifest {
    package: Package,
}
#[derive(serde::Deserialize)]
struct Package {
    name: Txt,
    version: Version,
    description: Option<Txt>,
    homepage: Option<Txt>,
    authors: Box<[Txt]>,
    metadata: Option<Metadata>,
}
#[derive(serde::Deserialize)]
struct Metadata {
    zng: Option<Zng>,
}
#[derive(serde::Deserialize)]
struct Zng {
    about: Option<MetadataAbout>,
}
#[derive(serde::Deserialize)]
struct MetadataAbout {
    app: Option<Txt>,
    org: Option<Txt>,
    qualifier: Option<Txt>,
}

/// Gets metadata about the application.
///
/// The app must call [`init!`] at the beginning of the process, otherwise the metadata will fallback
/// to just a name extracted from the current executable file path.
///
/// See the [`directories::ProjectDirs::from`] documentation for more details on how this metadata is
/// used to create/find the app data directories.
///
/// [`directories::ProjectDirs::from`]: https://docs.rs/directories/5.0/directories/struct.ProjectDirs.html#method.from
pub fn about() -> &'static About {
    &ABOUT
}

fn fallback_name() -> Txt {
    let exe = current_exe();
    let exe_name = exe.file_name().unwrap().to_string_lossy();
    let name = exe_name.split('.').find(|p| !p.is_empty()).unwrap();
    Txt::from_str(name)
}

/// Gets a path relative to the package binaries.
///
/// In all platforms `bin("")` is `std::env::current_exe().parent()`.
///
/// # Panics
///
/// Panics if [`std::env::current_exe`] returns an error.
pub fn bin(relative_path: impl AsRef<Path>) -> PathBuf {
    BIN.join(relative_path)
}
lazy_static! {
    static ref BIN: PathBuf = current_exe().parent().expect("current_exe path parent is required").to_owned();
}

/// Gets a path relative to the package resources.
///
/// * The res dir can be set by [`init_res`] before any env dir is used.
/// * In all platforms if a file `bin/current_exe_name.zng_res_dir` is found it defines the res path.
/// * In `cfg(debug_assertions)` builds returns `res`.
/// * In macOS returns `bin("../Resources")`, assumes the package is deployed using a desktop `.app` folder.
/// * In iOS returns `bin("")`, assumes the package is deployed as a mobile `.app` folder.
/// * In Android returns `bin("../res")`, assumes the package is deployed as a `.apk` file.
/// * In all other Unix systems returns `bin("../etc")`, assumes the package is deployed using a `.deb` like structure.
/// * In Windows returns `bin("../res")`. Note that there is no Windows standard, make sure to install the project using this structure.
///
/// # Built Resources
///
/// In `cfg(any(debug_assertions, feature="built_res"))` builds if the `target/res/{relative_path}` path exists it
/// is returned instead. This is useful during development when the app depends on res that are generated locally and not
/// included in version control.
///
/// Note that the built resources must be packaged with the other res at the same relative location, so that release builds can find then.
pub fn res(relative_path: impl AsRef<Path>) -> PathBuf {
    res_impl(relative_path.as_ref())
}
#[cfg(any(debug_assertions, feature = "built_res"))]
fn res_impl(relative_path: &Path) -> PathBuf {
    let built = BUILT_RES.join(relative_path);
    if built.exists() {
        return built;
    }

    RES.join(relative_path)
}
#[cfg(not(any(debug_assertions, feature = "built_res")))]
fn res_impl(relative_path: &Path) -> PathBuf {
    RES.join(relative_path)
}

/// Sets a custom [`res`] path.
///
/// # Panics
///
/// Panics if not called at the beginning of the process.
pub fn init_res(path: impl Into<PathBuf>) {
    if lazy_static_init(&RES, path.into()).is_err() {
        panic!("cannot `init_res`, `res` has already inited")
    }
}

/// Sets a custom path for the "built resources" override checked by [`res`] in debug builds.
///
/// # Panics
///
/// Panics if not called at the beginning of the process.
#[cfg(any(debug_assertions, feature = "built_res"))]
pub fn init_built_res(path: impl Into<PathBuf>) {
    if lazy_static_init(&BUILT_RES, path.into()).is_err() {
        panic!("cannot `init_built_res`, `res` has already inited")
    }
}

lazy_static! {
    static ref RES: PathBuf = find_res();

    #[cfg(any(debug_assertions, feature="built_res"))]
    static ref BUILT_RES: PathBuf = PathBuf::from("target/res");
}
fn find_res() -> PathBuf {
    if let Ok(mut p) = std::env::current_exe() {
        p.set_extension("zng_res_dir");
        if let Ok(dir) = read_line(&p) {
            return dir.into();
        }
    }
    if cfg!(debug_assertions) {
        PathBuf::from("res")
    } else if cfg!(windows) {
        bin("../res")
    } else if cfg!(target_os = "macos") {
        bin("../Resources")
    } else if cfg!(target_os = "ios") {
        bin("")
    } else if cfg!(target_os = "android") {
        bin("res")
    } else if cfg!(target_family = "unix") {
        bin("../etc")
    } else {
        panic!(
            "resources dir not specified for platform {}, use a 'bin/current_exe_name.zng_res_dir' file to specify an alternative",
            std::env::consts::OS
        )
    }
}

/// Gets a path relative to the user config directory for the app.
///
/// * The config dir can be set by [`init_config`] before any env dir is used.
/// * In all platforms if a file in `res("zng_config_dir")` is found the file first line not starting with
///  `"\s#"` and non empty is used as the config path.
/// * In `cfg(debug_assertions)` builds returns `target/tmp/dev_config/`.
/// * In all platforms attempts [`directories::ProjectDirs::config_dir`] and panic if it fails.
/// * If the config dir selected by the previous method contains a `"zng_config_dir"` file it will be
///   used to redirect to another config dir, you can use this to implement config migration. Redirection only happens once.
///
/// The config directory is created if it is missing, checks once on init or first use.
///
/// [`directories::ProjectDirs::config_dir`]: https://docs.rs/directories/5.0/directories/struct.ProjectDirs.html#method.config_dir
pub fn config(relative_path: impl AsRef<Path>) -> PathBuf {
    CONFIG.join(relative_path)
}

/// Sets a custom [`original_config`] path.
///
/// # Panics
///
/// Panics if not called at the beginning of the process.
pub fn init_config(path: impl Into<PathBuf>) {
    if lazy_static_init(&ORIGINAL_CONFIG, path.into()).is_err() {
        panic!("cannot `init_config`, `original_config` has already inited")
    }
}

/// Config path before migration.
///
/// If this is equal to [`config`] the config has not migrated.
pub fn original_config() -> PathBuf {
    ORIGINAL_CONFIG.clone()
}
lazy_static! {
    static ref ORIGINAL_CONFIG: PathBuf = find_config();
}

/// Copied all config to `new_path` and saves it as the config path.
///
/// If copying and saving path succeeds make a best effort to wipe the previous config dir. If copy and save fails
/// makes a best effort to undo already made copies.
///
/// The `new_path` must not exist or be empty.
pub fn migrate_config(new_path: impl AsRef<Path>) -> io::Result<()> {
    migrate_config_impl(new_path.as_ref())
}
fn migrate_config_impl(new_path: &Path) -> io::Result<()> {
    let prev_path = CONFIG.as_path();

    if prev_path == new_path {
        return Ok(());
    }

    let original_path = ORIGINAL_CONFIG.as_path();
    let is_return = new_path == original_path;

    if !is_return && dir_exists_not_empty(new_path) {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "can only migrate to new dir or empty dir",
        ));
    }
    let created = !new_path.exists();
    if created {
        fs::create_dir_all(new_path)?;
    }

    let migrate = |from: &Path, to: &Path| {
        copy_dir_all(from, to)?;
        if fs::remove_dir_all(from).is_ok() {
            fs::create_dir(from)?;
        }

        let redirect = ORIGINAL_CONFIG.join("zng_config_dir");
        if is_return {
            fs::remove_file(redirect)
        } else {
            fs::write(redirect, to.display().to_string().as_bytes())
        }
    };

    if let Err(e) = migrate(prev_path, new_path) {
        eprintln!("migration failed, {e}");
        if fs::remove_dir_all(new_path).is_ok() && !created {
            let _ = fs::create_dir(new_path);
        }
    }

    tracing::info!("changed config dir to `{}`", new_path.display());

    Ok(())
}

fn copy_dir_all(from: &Path, to: &Path) -> io::Result<()> {
    for entry in fs::read_dir(from)? {
        let from = entry?.path();
        if from.is_dir() {
            let to = to.join(from.file_name().unwrap());
            fs::create_dir(&to)?;
            copy_dir_all(&from, &to)?;
        } else if from.is_file() {
            let to = to.join(from.file_name().unwrap());
            fs::copy(&from, &to)?;
        } else {
            continue;
        }
    }
    Ok(())
}

lazy_static! {
    static ref CONFIG: PathBuf = redirect_config(original_config());
}
fn find_config() -> PathBuf {
    let cfg_dir = res("zng_config_dir");
    if let Ok(dir) = read_line(&cfg_dir) {
        return PathBuf::from(dir);
    }

    if cfg!(debug_assertions) {
        return PathBuf::from("target/tmp/dev_config/");
    }

    let a = about();
    if let Some(dirs) = directories::ProjectDirs::from(&a.qualifier, &a.org, &a.app) {
        dirs.config_dir().to_owned()
    } else {
        panic!(
            "config dir not specified for platform {}, use a '{}' file to specify an alternative",
            std::env::consts::OS,
            cfg_dir.display(),
        )
    }
}
fn redirect_config(cfg: PathBuf) -> PathBuf {
    if let Ok(dir) = read_line(&cfg.join("zng_config_dir")) {
        let dir = PathBuf::from(dir);
        if dir.exists() {
            let test_path = dir.join(".zng-config-test");
            if let Err(e) = fs::create_dir_all(&dir)
                .and_then(|_| fs::write(&test_path, "# check write access"))
                .and_then(|_| fs::remove_file(&test_path))
            {
                eprintln!("error writing to migrated `{}`, {e}", dir.display());
                tracing::error!("error writing to migrated `{}`, {e}", dir.display());
                return cfg;
            }
        } else if let Err(e) = fs::create_dir_all(&dir) {
            eprintln!("error creating migrated `{}`, {e}", dir.display());
            tracing::error!("error creating migrated `{}`, {e}", dir.display());
            return cfg;
        }
        dir
    } else {
        create_dir_opt(cfg)
    }
}

fn create_dir_opt(dir: PathBuf) -> PathBuf {
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("error creating `{}`, {e}", dir.display());
        tracing::error!("error creating `{}`, {e}", dir.display());
    }
    dir
}

/// Gets a path relative to the cache directory for the app.
///
/// * The cache dir can be set by [`init_cache`] before any env dir is used.
/// * In all platforms if a file `config("zng_cache_dir")` is found the file first line not starting with
///  `"\s#"` and non empty is used as the cache path.
/// * In `cfg(debug_assertions)` builds returns `target/tmp/dev_cache/`.
/// * In all platforms attempts [`directories::ProjectDirs::cache_dir`] and panic if it fails.
///
/// The cache dir is created if it is missing, checks once on init or first use.
///
/// [`directories::ProjectDirs::cache_dir`]: https://docs.rs/directories/5.0/directories/struct.ProjectDirs.html#method.cache_dir
pub fn cache(relative_path: impl AsRef<Path>) -> PathBuf {
    CACHE.join(relative_path)
}

/// Sets a custom [`cache`] path.
///
/// # Panics
///
/// Panics if not called at the beginning of the process.
pub fn init_cache(path: impl Into<PathBuf>) {
    match lazy_static_init(&CONFIG, path.into()) {
        Ok(p) => {
            create_dir_opt(p.to_owned());
        }
        Err(_) => panic!("cannot `init_cache`, `cache` has already inited"),
    }
}

/// Removes all cache files possible.
///
/// Continues removing after the first fail, returns the last error.
pub fn clear_cache() -> io::Result<()> {
    best_effort_clear(CACHE.as_path())
}
fn best_effort_clear(path: &Path) -> io::Result<()> {
    let mut error = None;

    match fs::read_dir(path) {
        Ok(cache) => {
            for entry in cache {
                match entry {
                    Ok(e) => {
                        let path = e.path();
                        if path.is_dir() {
                            if fs::remove_dir_all(&path).is_err() {
                                match best_effort_clear(&path) {
                                    Ok(()) => {
                                        if let Err(e) = fs::remove_dir(&path) {
                                            error = Some(e)
                                        }
                                    }
                                    Err(e) => {
                                        error = Some(e);
                                    }
                                }
                            }
                        } else if path.is_file() {
                            if let Err(e) = fs::remove_file(&path) {
                                error = Some(e);
                            }
                        }
                    }
                    Err(e) => {
                        error = Some(e);
                    }
                }
            }
        }
        Err(e) => {
            error = Some(e);
        }
    }

    match error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// Save `new_path` as the new cache path and make a best effort to move existing cache files.
///
/// Note that the move failure is not considered an error (it is only logged), the app is expected to
/// rebuild missing cache entries.
///
/// Note that [`cache`] will still point to the previous path on success, the app must be restarted to use the new cache.
///
/// The `new_path` must not exist or be empty.
pub fn migrate_cache(new_path: impl AsRef<Path>) -> io::Result<()> {
    migrate_cache_impl(new_path.as_ref())
}
fn migrate_cache_impl(new_path: &Path) -> io::Result<()> {
    if dir_exists_not_empty(new_path) {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "can only migrate to new dir or empty dir",
        ));
    }
    fs::create_dir_all(new_path)?;
    let write_test = new_path.join(".zng-cache");
    fs::write(&write_test, "# zng cache dir".as_bytes())?;
    fs::remove_file(&write_test)?;

    fs::write(config("zng_cache_dir"), new_path.display().to_string().as_bytes())?;

    tracing::info!("changed cache dir to `{}`", new_path.display());

    let prev_path = CACHE.as_path();
    if prev_path == new_path {
        return Ok(());
    }
    if let Err(e) = best_effort_move(prev_path, new_path) {
        eprintln!("failed to migrate all cache files, {e}");
        tracing::error!("failed to migrate all cache files, {e}");
    }

    Ok(())
}

fn dir_exists_not_empty(dir: &Path) -> bool {
    match fs::read_dir(dir) {
        Ok(dir) => {
            for entry in dir {
                match entry {
                    Ok(_) => return true,
                    Err(e) => {
                        if e.kind() != io::ErrorKind::NotFound {
                            return true;
                        }
                    }
                }
            }
            false
        }
        Err(e) => e.kind() != io::ErrorKind::NotFound,
    }
}

fn best_effort_move(from: &Path, to: &Path) -> io::Result<()> {
    let mut error = None;

    match fs::read_dir(from) {
        Ok(cache) => {
            for entry in cache {
                match entry {
                    Ok(e) => {
                        let from = e.path();
                        if from.is_dir() {
                            let to = to.join(from.file_name().unwrap());
                            if let Err(e) = fs::rename(&from, &to).or_else(|_| {
                                fs::create_dir(&to)?;
                                best_effort_move(&from, &to)?;
                                fs::remove_dir(&from)
                            }) {
                                error = Some(e)
                            }
                        } else if from.is_file() {
                            let to = to.join(from.file_name().unwrap());
                            if let Err(e) = fs::rename(&from, &to).or_else(|_| {
                                fs::copy(&from, &to)?;
                                fs::remove_file(&from)
                            }) {
                                error = Some(e);
                            }
                        }
                    }
                    Err(e) => {
                        error = Some(e);
                    }
                }
            }
        }
        Err(e) => {
            error = Some(e);
        }
    }

    match error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

lazy_static! {
    static ref CACHE: PathBuf = create_dir_opt(find_cache());
}
fn find_cache() -> PathBuf {
    let cache_dir = config("zng_cache_dir");
    if let Ok(dir) = read_line(&cache_dir) {
        return PathBuf::from(dir);
    }

    if cfg!(debug_assertions) {
        return PathBuf::from("target/tmp/dev_cache/");
    }

    let a = about();
    if let Some(dirs) = directories::ProjectDirs::from(&a.qualifier, &a.org, &a.app) {
        dirs.cache_dir().to_owned()
    } else {
        panic!(
            "cache dir not specified for platform {}, use a '{}' file to specify an alternative",
            std::env::consts::OS,
            cache_dir.display(),
        )
    }
}

fn current_exe() -> PathBuf {
    std::env::current_exe().expect("current_exe path is required")
}

fn read_line(path: &Path) -> io::Result<String> {
    let file = fs::File::open(path)?;
    for line in io::BufReader::new(file).lines() {
        let line = line?;
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        return Ok(line.into());
    }
    Err(io::Error::new(io::ErrorKind::UnexpectedEof, "no uncommented line"))
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn parse_manifest() {
        init!();
        let a = about();
        assert_eq!(a.pkg_name, "zng-env");
        assert_eq!(a.app, "zng-env");
        assert_eq!(&a.pkg_authors[..], &[Txt::from("The Zng Project Developers")]);
        assert_eq!(a.org, "The Zng Project Developers");
    }
}
