//! Dynamically links to [`zero-ui-view`] pre-builds.
//!
//! [`zero-ui-view`]: https://docs.rs/zero-ui-view

use libloading::*;
use std::path::PathBuf;

/// Call the pre-built [`init`].
///
/// [`init`]: https://docs.rs/zero-ui-view/fn.init.html
///
/// # Panics
///
/// Panics if fails to link to the library using the [default location].
///
/// [default location]: ViewLib::new
pub fn init() {
    ViewLib::new().unwrap().init()
}

/// Call the pre-build [`run_same_process`].
///
/// [`run_same_process`]: https://docs.rs/zero-ui-view/fn.run_same_process.html
///
/// # Panics
///
/// Panics if fails to link to the library using the [default location].
///
/// [default location]: ViewLib::new
pub fn run_same_process(run_app: impl FnOnce() + Send + 'static) -> ! {
    ViewLib::new().unwrap().run_same_process(run_app)
}

/// Dynamically linked pre-build view.
pub struct ViewLib {
    init_fn: unsafe extern "C" fn(),
    run_same_process_fn: unsafe extern "C" fn(extern "C" fn()) -> !,
    _lib: Library,
}
impl ViewLib {
    /// Link to the default file name `./zero_ui_view`.
    pub fn new() -> Result<Self, libloading::Error> {
        Self::link("zero_ui_view")
    }

    /// Link to the pre-build library file.
    ///
    /// If the file does not have an extension searches for a file without extension then a
    /// `.dll` file in Windows, a `.so` file in Linux and a `.dylib` file in other operating systems.
    /// 
    /// If the path is local, 
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
