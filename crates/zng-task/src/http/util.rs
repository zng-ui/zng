use std::time::Duration;

/// Calls [`fs4::FileExt::lock_exclusive`] with a timeout.
pub fn lock_exclusive(file: &impl fs4::fs_std::FileExt, timeout: Duration) -> std::io::Result<()> {
    lock_timeout(file, |f| f.try_lock_exclusive(), timeout)
}

/// Calls [`fs4::FileExt::lock_shared`] with a timeout.
pub fn lock_shared(file: &impl fs4::fs_std::FileExt, timeout: Duration) -> std::io::Result<()> {
    lock_timeout(file, |f| f.try_lock_shared(), timeout)
}

fn lock_timeout<F: fs4::fs_std::FileExt>(
    file: &F,
    try_lock: impl Fn(&F) -> std::io::Result<bool>,
    mut timeout: Duration,
) -> std::io::Result<()> {
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

                error = Some(e)
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

/// Calls [`fs4::FileExt::unlock`] and ignores "already unlocked" errors.
pub fn unlock_ok(file: &impl fs4::fs_std::FileExt) -> std::io::Result<()> {
    if let Err(e) = file.unlock() {
        if let Some(code) = e.raw_os_error() {
            #[cfg(windows)]
            if code == 158 {
                // ERROR_NOT_LOCKED
                return Ok(());
            }

            #[cfg(unix)]
            if code == 22 {
                // EINVAL
                return Ok(());
            }
        }

        Err(e)
    } else {
        Ok(())
    }
}

/// Sets a `tracing` subscriber that writes warnings to stderr and panics on errors.
///
/// Panics if another different subscriber is already set.
#[cfg(test)]
pub fn test_log() {
    use std::sync::atomic::*;

    use std::fmt;
    use tracing::*;

    struct TestSubscriber;
    impl Subscriber for TestSubscriber {
        fn enabled(&self, metadata: &Metadata<'_>) -> bool {
            metadata.is_event() && metadata.level() < &Level::WARN
        }

        fn new_span(&self, _span: &span::Attributes<'_>) -> span::Id {
            unimplemented!()
        }

        fn record(&self, _span: &span::Id, _values: &span::Record<'_>) {
            unimplemented!()
        }

        fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id) {
            unimplemented!()
        }

        fn event(&self, event: &Event<'_>) {
            struct MsgCollector<'a>(&'a mut String);
            impl field::Visit for MsgCollector<'_> {
                fn record_debug(&mut self, field: &field::Field, value: &dyn fmt::Debug) {
                    use std::fmt::Write;
                    write!(self.0, "\n  {} = {:?}", field.name(), value).unwrap();
                }
            }

            let meta = event.metadata();
            let file = meta.file().unwrap_or("");
            let line = meta.line().unwrap_or(0);

            let mut msg = format!("[{file}:{line}]");
            event.record(&mut MsgCollector(&mut msg));

            if meta.level() == &Level::ERROR {
                panic!("[LOG-ERROR]{msg}");
            } else {
                eprintln!("[LOG-WARN]{msg}");
            }
        }

        fn enter(&self, _span: &span::Id) {
            unimplemented!()
        }
        fn exit(&self, _span: &span::Id) {
            unimplemented!()
        }
    }

    static IS_SET: AtomicBool = AtomicBool::new(false);

    if !IS_SET.swap(true, Ordering::Relaxed)
        && let Err(e) = subscriber::set_global_default(TestSubscriber)
    {
        panic!("failed to set test log subscriber, {e:?}");
    }
}

/// A temporary directory for unit tests.
///
/// Directory is "target/tmp/unit_tests/<name>" with fallback to system temporary if the target folder is not found.
///
/// Auto cleanup on drop.
#[cfg(test)]
pub struct TestTempDir {
    path: Option<std::path::PathBuf>,
}
#[cfg(test)]
impl Drop for TestTempDir {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            let _ = remove_dir_all::remove_dir_all(path);
        }
    }
}
#[cfg(test)]
impl TestTempDir {
    /// Create temporary directory for the unique test name.
    pub fn new(name: &str) -> Self {
        let path = Self::try_target().unwrap_or_else(Self::fallback).join(name);
        std::fs::create_dir_all(&path).unwrap_or_else(|e| panic!("failed to create temp `{}`, {e:?}", path.display()));
        TestTempDir { path: Some(path) }
    }
    fn try_target() -> Option<std::path::PathBuf> {
        let p = dunce::canonicalize(std::env::current_exe().ok()?).ok()?;
        // target/debug/deps/../../..
        let target = p.parent()?.parent()?.parent()?;
        if target.file_name()?.to_str()? != "target" {
            return None;
        }
        Some(target.join("tmp/unit_tests"))
    }
    fn fallback() -> std::path::PathBuf {
        tracing::warn!("using fallback temporary directory");
        std::env::temp_dir().join("zng/unit_tests")
    }

    /// Dereferences the temporary directory path.
    pub fn path(&self) -> &std::path::Path {
        self.path.as_deref().unwrap()
    }

    /// Drop `self` without removing the temporary files.
    ///
    /// Returns the path to the temporary directory.
    pub fn keep(mut self) -> std::path::PathBuf {
        self.path.take().unwrap()
    }
}
#[cfg(test)]
impl std::ops::Deref for TestTempDir {
    type Target = std::path::Path;

    fn deref(&self) -> &Self::Target {
        self.path()
    }
}
#[cfg(test)]
impl std::convert::AsRef<std::path::Path> for TestTempDir {
    fn as_ref(&self) -> &std::path::Path {
        self.path()
    }
}
#[cfg(test)]
impl<'a> From<&'a TestTempDir> for std::path::PathBuf {
    fn from(a: &'a TestTempDir) -> Self {
        a.path.as_ref().unwrap().clone()
    }
}
