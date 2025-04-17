use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
pub use fs4::fs_std::FileExt;

#[cfg(target_arch = "wasm32")]
pub trait FileExt {
    fn try_lock_shared(&self) -> std::io::Result<bool> {
        not_supported()
    }
    fn try_lock_exclusive(&self) -> std::io::Result<bool> {
        not_supported()
    }
    fn unlock(&self) -> std::io::Result<bool> {
        not_supported()
    }
}
#[cfg(target_arch = "wasm32")]
impl FileExt for std::fs::File {}
#[cfg(target_arch = "wasm32")]
fn not_supported() -> std::io::Result<bool> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "operation not supported on wasm yet",
    ))
}

/// Calls `fs4::FileExt::lock_exclusive` with a timeout.
pub fn lock_exclusive(file: &impl FileExt, timeout: Duration) -> std::io::Result<()> {
    lock_timeout(file, |f| f.try_lock_exclusive(), timeout)
}

/// Calls `fs4::FileExt::lock_shared` with a timeout.
pub fn lock_shared(file: &impl FileExt, timeout: Duration) -> std::io::Result<()> {
    lock_timeout(file, |f| f.try_lock_shared(), timeout)
}

#[cfg(target_arch = "wasm32")]

pub fn lock_timeout<F: FileExt>(_: &F, _: impl Fn(&F) -> std::io::Result<bool>, _: Duration) -> std::io::Result<()> {
    not_supported().map(|_| ())
}
#[cfg(not(target_arch = "wasm32"))]
pub fn lock_timeout<F: FileExt>(file: &F, try_lock: impl Fn(&F) -> std::io::Result<bool>, mut timeout: Duration) -> std::io::Result<()> {
    let mut locked_error = None;
    loop {
        let mut error = None;
        match try_lock(file) {
            Ok(true) => return Ok(()),
            Ok(false) => {}
            Err(e) => {
                if e.kind() != std::io::ErrorKind::WouldBlock
                    && e.raw_os_error() != locked_error.get_or_insert_with(fs4::lock_contended_error).raw_os_error()
                {
                    return Err(e);
                }

                error = Some(e);
            }
        }

        const INTERVAL: Duration = Duration::from_millis(10);
        timeout = timeout.saturating_sub(INTERVAL);
        if timeout.is_zero() {
            match error {
                Some(e) => return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, e)),
                None => return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, fs4::lock_contended_error())),
            }
        } else {
            std::thread::sleep(INTERVAL.min(timeout));
        }
    }
}

pub fn unlock_ok(file: &impl FileExt) -> std::io::Result<()> {
    if let Err(e) = file.unlock() {
        if let Some(_code) = e.raw_os_error() {
            #[cfg(windows)]
            if _code == 158 {
                // ERROR_NOT_LOCKED
                return Ok(());
            }

            #[cfg(unix)]
            if _code == 22 {
                // EINVAL
                return Ok(());
            }
        }

        Err(e)
    } else {
        Ok(())
    }
}
