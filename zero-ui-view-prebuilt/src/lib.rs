//! Dynamically links to [`zero-ui-view`] pre-builds.
//!
//! [`zero-ui-view`]: https://docs.rs/zero-ui-view

use core::fmt;
use libloading::*;
use std::{env, io, path::PathBuf};

/// Call the pre-built [`init`].
///
/// [`init`]: https://docs.rs/zero-ui-view/fn.init.html
///
/// # Panics
///
/// Panics if fails to [install] the library.
///
/// [install]: ViewLib::install
pub fn init() {
    ViewLib::install().unwrap().init()
}

/// Call the pre-build [`run_same_process`].
///
/// [`run_same_process`]: https://docs.rs/zero-ui-view/fn.run_same_process.html
///
/// # Panics
///
/// Panics if fails to [install] the library.
///
/// [install]: ViewLib::install
pub fn run_same_process(run_app: impl FnOnce() + Send + 'static) -> ! {
    ViewLib::install().unwrap().run_same_process(run_app)
}

/// Dynamically linked pre-build view.
pub struct ViewLib {
    init_fn: unsafe extern "C" fn(),
    run_same_process_fn: unsafe extern "C" fn(extern "C" fn()) -> !,
    _lib: Library,
}
impl ViewLib {
    /// Extract the embedded library to the shared data directory and link to it.
    pub fn install() -> Result<Self, Error> {
        let dir = dirs::data_dir().unwrap_or_else(env::temp_dir).join("zero_ui_view");
        std::fs::create_dir_all(&dir)?;
        Self::install_to(dir)
    }

    /// Try to delete the installed library from the data directory.
    /// 
    /// See [`uninstall_from`] for details.
    /// 
    /// [`uninstall_from`]: Self::uninstall_from
    pub fn uninstall() -> Result<bool, io::Error> {
        let dir = dirs::data_dir().unwrap_or_else(env::temp_dir).join("zero_ui_view");
        Self::uninstall_from(dir)
    }

    /// Extract the embedded library to `dir` and link to it.
    ///
    /// If the library is already extracted it is reused if the SHA1 hash matches.
    pub fn install_to(dir: impl Into<PathBuf>) -> Result<Self, Error> {
        #[cfg(not(zero_ui_lib_embedded))]
        {
            let _ = dir;
            panic!("library not embedded");
        }

        #[cfg(zero_ui_lib_embedded)]
        {
            let file = Self::install_path(dir.into());

            if !file.exists() {
                std::fs::write(&file, LIB)?;
            }

            Self::link(file)
        }
    }

    /// Try to delete the installed library from the given `dir`.
    ///
    /// Returns `Ok(true)` if uninstalled, `Ok(false)` if was not installed and `Err(_)` 
    /// if is installed and failed to delete.
    /// 
    /// Note that the file is probably in use if it was installed in the current process instance, in Windows
    /// files cannot be deleted until they are released.
    pub fn uninstall_from(dir: impl Into<PathBuf>) -> Result<bool, io::Error> {
        #[cfg(not(zero_ui_lib_embedded))]
        {
            let _ = dir;
            Ok(false)
        }

        #[cfg(zero_ui_lib_embedded)]
        {
            let file = Self::install_path(dir.into());

            if file.exists() {
                std::fs::remove_file(file)?;
                Ok(true)
            } else {
                Ok(false)
            }
        }
    }

    #[cfg(zero_ui_lib_embedded)]
    fn install_path(dir: PathBuf) -> PathBuf {
        #[cfg(target_os = "windows")]
        let file_name = format!("{}.dll", LIB_NAME);
        #[cfg(target_os = "linux")]
        let file_name = format!("{}.so", LIB_NAME);
        #[cfg(target_os = "macos")]
        let file_name = format!("{}.dylib", LIB_NAME);

        dir.join(file_name)
    }

    /// Link to the pre-build library file.
    ///
    /// If the file does not have an extension searches for a file without extension then a
    /// `.dll` file in Windows, a `.so` file in Linux and a `.dylib` file in other operating systems.
    ///
    /// Note that the is only searched as described above, if it is not found an error returns immediately,
    /// the operating system library search feature is not used.
    pub fn link(view_dylib: impl Into<PathBuf>) -> Result<Self, Error> {
        let mut lib = view_dylib.into();

        if !lib.exists() && lib.extension().is_none() {
            #[cfg(target_os = "windows")]
            lib.set_extension("dll");
            #[cfg(target_os = "linux")]
            lib.set_extension("so");
            #[cfg(target_os = "macos")]
            lib.set_extension("dylib");
        }

        if lib.exists() {
            // this disables Windows DLL search feature.
            lib = lib.canonicalize()?;
        }

        if !lib.exists() {
            return Err(io::Error::new(io::ErrorKind::NotFound, format!("view library not found in `{}`", lib.display())).into());
        }

        unsafe {
            let lib = Library::new(lib)?;
            Ok(ViewLib {
                init_fn: *lib.get(b"extern_init")?,
                run_same_process_fn: *lib.get(b"extern_run_same_process")?,
                _lib: lib,
            })
        }
    }

    /// Call the pre-built [`init`].
    ///
    /// [`init`]: https://docs.rs/zero-ui-view/fn.init.html
    pub fn init(self) {
        unsafe { (self.init_fn)() }
    }

    /// Call the pre-build [`run_same_process`].
    ///
    /// [`run_same_process`]: https://docs.rs/zero-ui-view/fn.run_same_process.html
    pub fn run_same_process(self, run_app: impl FnOnce() + Send + 'static) -> ! {
        unsafe {
            use std::sync::atomic::{AtomicU8, Ordering};

            static STATE: AtomicU8 = AtomicU8::new(ST_NONE);
            const ST_NONE: u8 = 0;
            const ST_SOME: u8 = 1;
            const ST_TAKEN: u8 = 2;

            static mut RUN: Option<Box<dyn FnOnce() + Send>> = None;

            if STATE.swap(ST_SOME, Ordering::SeqCst) != ST_NONE {
                panic!("expected only one call to `run_same_process`");
            }

            RUN = Some(Box::new(run_app));

            extern "C" fn run() {
                if STATE.swap(ST_TAKEN, Ordering::SeqCst) != ST_SOME {
                    panic!("expected only one call to `run_app` closure");
                }

                let run_app = unsafe { RUN.take() }.unwrap();
                run_app();
            }

            (self.run_same_process_fn)(run)
        }
    }
}

#[cfg(zero_ui_lib_embedded)]
const LIB: &[u8] = include_bytes!(env!("ZERO_UI_VIEW_LIB"));
#[cfg(zero_ui_lib_embedded)]
const LIB_NAME: &str = concat!("zv.", env!("CARGO_PKG_VERSION"), ".", env!("ZERO_UI_VIEW_LIB_HASH"));

/// Error searching or linking to pre-build library.
#[derive(Debug)]
pub enum Error {
    /// Error searching library.
    Io(io::Error),
    /// Error loading or linking library.
    Lib(libloading::Error),
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}
impl From<libloading::Error> for Error {
    fn from(e: libloading::Error) -> Self {
        Error::Lib(e)
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{}", e),
            Error::Lib(e) => write!(f, "{}", e),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::Lib(e) => Some(e),
        }
    }
}
