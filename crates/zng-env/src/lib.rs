#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/master/examples/res/image/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/master/examples/res/image/zng-logo.png")]
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
};

use zng_txt::Txt;
use zng_unique_id::{lazy_static, lazy_static_init};

/// Init [`app_unique_name`], the unique name is required by multiple others functions in this module.
///
/// If `application` it will fallback to `"{current_exe_name_without_extension}"`.
///
/// See the [`directories::ProjectDirs::from`] documentation for more details.
///
/// [`directories::ProjectDirs::from`]: https://docs.rs/directories/5.0/directories/struct.ProjectDirs.html#method.from
pub fn init(qualifier: impl Into<Txt>, organization: impl Into<Txt>, application: impl Into<Txt>) {
    if lazy_static_init(&APP_UNIQUE_NAME, (qualifier.into(), organization.into(), application.into())).is_err() {
        panic!("env already inited, env::init must be the first call in the process")
    }
}
lazy_static! {
    static ref APP_UNIQUE_NAME: (Txt, Txt, Txt) = ("".into(), "".into(), fallback_name());
}

/// Gets the `qualifier, organization, application` name of the application.
///
/// The app must call [`init`] before this, otherwise the name will fallback to `("io.crates", "zng-app", "{current_exe_name_without_extension}"`.
pub fn app_unique_name() -> (Txt, Txt, Txt) {
    APP_UNIQUE_NAME.clone()
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
/// * In all platforms if a file `bin/current_exe_name.zng_res_dir` is found the file first line not starting with
///  `"\s#"` and non empty is used as the res path.
/// * In `cfg!(debug_assertions)` builds returns `pack/dev/`, see `cargo-zng` for more details.
/// * In macOS returns `bin("../Resources")`, assumes the package is deployed using a desktop `.app` folder.
/// * In iOS returns `bin("")`, assumes the package is deployed as a mobile `.app` folder.
/// * In Android returns `bin("../res")`, assumes the package is deployed as a `.apk` file.
/// * In all other Unix systems returns `bin("../etc")`, assumes the package is deployed using a `.deb` like structure.
/// * In Windows returns `bin("../res")`. Note that there is no Windows standard, make sure to install the project using this structure.
pub fn res(relative_path: impl AsRef<Path>) -> PathBuf {
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

lazy_static! {
    static ref RES: PathBuf = find_res();
}
fn find_res() -> PathBuf {
    if let Ok(mut p) = std::env::current_exe() {
        p.set_extension("zng_res_dir");
        if let Ok(dir) = read_line(&p) {
            return dir.into();
        }
    }
    if cfg!(debug_assertions) {
        PathBuf::from("pack/dev")
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
/// * In `cfg!(debug_assertions)` builds returns `target/tmp/dev_config/`.
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

/// Sets a custom [`config`] path.
///
/// # Panics
///
/// Panics if not called at the beginning of the process.
pub fn init_config(path: impl Into<PathBuf>) {
    match lazy_static_init(&CONFIG, path.into()) {
        Ok(p) => {
            create_dir(p.to_owned());
        }
        Err(_) => panic!("cannot `init_config`, `config` has already inited"),
    }
}

lazy_static! {
    static ref CONFIG: PathBuf = create_dir(redirect_config(find_config()));
}
fn find_config() -> PathBuf {
    let cfg_dir = res("zng_config_dir");
    if let Ok(dir) = read_line(&cfg_dir) {
        return PathBuf::from(dir);
    }
    let (org, comp, app) = app_unique_name();
    if let Some(dirs) = directories::ProjectDirs::from(org.as_str(), comp.as_str(), app.as_str()) {
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
        PathBuf::from(dir)
    } else {
        cfg
    }
}

fn create_dir(dir: PathBuf) -> PathBuf {
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
/// * In `cfg!(debug_assertions)` builds returns `target/tmp/dev_cache/`.
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
            create_dir(p.to_owned());
        }
        Err(_) => panic!("cannot `init_cache`, `cache` has already inited"),
    }
}

lazy_static! {
    static ref CACHE: PathBuf = create_dir(find_cache());
}
fn find_cache() -> PathBuf {
    let cache_dir = config("zng_cache_dir");
    if let Ok(dir) = read_line(&cache_dir) {
        return PathBuf::from(dir);
    }
    let (org, comp, app) = app_unique_name();
    if let Some(dirs) = directories::ProjectDirs::from(org.as_str(), comp.as_str(), app.as_str()) {
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
