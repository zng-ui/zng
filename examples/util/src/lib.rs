use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use tracing::{Level, Subscriber};
use tracing_subscriber::{layer::Layer, prelude::*};

pub use profile_util::*;

/// Prints `tracing` and `log` events of levels INFO, WARN and ERROR in debug builds, logs to `example_name.error.log` in release builds.
pub fn print_info() {
    if cfg!(debug_assertions) {
        tracing_print(Level::INFO)
    } else {
        tracing_write(Level::ERROR)
    }
}

/// Prints `tracing` and `log` events of all levels.
pub fn print_trace() {
    tracing_print(Level::TRACE)
}

fn tracing_print(max: Level) {
    tracing_subscriber::registry()
        .with(FilterLayer(max))
        .with(tracing_subscriber::fmt::layer().without_time())
        .init();
}

fn tracing_write(max: Level) {
    tracing_subscriber::registry()
        .with(FilterLayer(max))
        .with(tracing_subscriber::fmt::layer().with_ansi(false).with_writer(ErrorLogFile::default))
        .init();
}

#[derive(Default)]
struct ErrorLogFile(Option<std::fs::File>);
impl ErrorLogFile {
    fn open(&mut self) -> std::io::Result<&mut std::fs::File> {
        if self.0.is_none() {
            let exe = std::env::current_exe()?;
            let mut i = 0;
            let mut file = exe.clone();
            file.set_extension(".error.log");
            while file.exists() {
                i += 1;
                file = exe.clone();
                file.set_extension(format!(".error.{i}.log"));
            }
            let file = std::fs::File::options().create(true).write(true).open(file)?;
            self.0 = Some(file);
        }
        Ok(self.0.as_mut().unwrap())
    }
}
impl Write for ErrorLogFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.open()?.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if let Some(f) = self.0.as_mut() {
            f.flush()?;
        }
        Ok(())
    }
}

struct FilterLayer(Level);
impl<S: Subscriber> Layer<S> for FilterLayer {
    fn enabled(&self, metadata: &tracing::Metadata<'_>, _: tracing_subscriber::layer::Context<'_, S>) -> bool {
        filter(&self.0, metadata)
    }

    fn max_level_hint(&self) -> Option<tracing::metadata::LevelFilter> {
        Some(self.0.into())
    }
}

/// Gets the temp dir for the example.
///
/// Temp files can be cleared using `cargo do clean --temp`.
pub fn temp_dir(example: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/tmp/examples")
        .join(example);
    fs::create_dir_all(&path).unwrap();
    path.canonicalize().unwrap()
}

/// Sets a panic hook that writes the panic backtrace to a log file, the panic is also written to std-err.
pub fn write_panic(log_file: impl AsRef<Path>) {
    let log_file = log_file.as_ref().to_owned();
    std::panic::set_hook(Box::new(move |info: &std::panic::PanicInfo| {
        // see `default_hook` in https://doc.rust-lang.org/src/std/panicking.rs.html#182

        let current_thread = std::thread::current();
        let name = current_thread.name().unwrap_or("<unnamed>");

        let (file, line, column) = if let Some(l) = info.location() {
            (l.file(), l.line(), l.column())
        } else {
            ("<unknown>", 0, 0)
        };

        let msg = panic_msg(info.payload());

        let backtrace = backtrace::Backtrace::new();

        let msg = format!("thread '{name}' panicked at '{msg}', {file}:{line}:{column}\n{backtrace:?}");
        std::fs::write(&log_file, msg).ok();

        std::process::exit(101) // Rust panic exit code.
    }));
}
fn panic_msg(payload: &dyn std::any::Any) -> &str {
    match payload.downcast_ref::<&'static str>() {
        Some(s) => *s,
        None => match payload.downcast_ref::<String>() {
            Some(s) => &s[..],
            None => "Box<dyn Any>",
        },
    }
}
