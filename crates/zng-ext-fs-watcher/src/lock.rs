use std::{fs::File, time::Duration};

/// Calls `fs4::FileExt::lock_exclusive` with a timeout.
pub fn lock_exclusive(file: &File, timeout: Duration) -> std::io::Result<()> {
    lock_timeout(file, |f| f.try_lock(), timeout)
}

/// Calls `fs4::FileExt::lock_shared` with a timeout.
pub fn lock_shared(file: &File, timeout: Duration) -> std::io::Result<()> {
    lock_timeout(file, |f| f.try_lock_shared(), timeout)
}

pub fn lock_timeout(
    file: &File,
    try_lock: impl Fn(&File) -> Result<(), std::fs::TryLockError>,
    mut timeout: Duration,
) -> std::io::Result<()> {
    loop {
        let mut error = None;
        match try_lock(file) {
            Ok(()) => return Ok(()),
            Err(std::fs::TryLockError::WouldBlock) => {}
            Err(e) => {
                error = Some(e);
            }
        }

        const INTERVAL: Duration = Duration::from_millis(10);
        timeout = timeout.saturating_sub(INTERVAL);
        if timeout.is_zero() {
            match error {
                Some(e) => return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, e)),
                None => return Err(std::io::Error::new(std::io::ErrorKind::WouldBlock, "lock timeout")),
            }
        } else {
            std::thread::sleep(INTERVAL.min(timeout));
        }
    }
}

pub fn unlock_ok(file: &File) -> std::io::Result<()> {
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
