//! Pre-built [`zero-ui-view`].
//!
//! [`zero-ui-view`]: https://docs.rs/zero-ui-view

use std::sync::atomic::{AtomicU8, Ordering};

#[cfg(not(doc))]
#[link(name = "zero_ui_view", kind = "static")]
extern "C" {
    fn extern_init();
    fn extern_run_same_process(run_app: extern "C" fn()) -> !;
}

/// Call the pre-built [`init`].
///
/// [`init`]: https://docs.rs/zero-ui-view/fn.init.html
pub fn init() {
    // SAFETY: this is safe.
    #[cfg(not(doc))]
    unsafe {
        extern_init()
    }
}

/// Call the pre-build [`run_same_process`].
///
/// [`run_same_process`]: https://docs.rs/zero-ui-view/fn.run_same_process.html
pub fn run_same_process(run_app: impl FnOnce() + Send + 'static) -> ! {
    // SAFETY: access to `RUN` is atomic.

    #[cfg(not(doc))]
    unsafe {
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
        extern_run_same_process(run);
    }

    #[allow(unreachable_code)]
    {
        unreachable!("expected `extern_run_same_process` to never return");
    }
}
