use std::{fs, io::Write, path::PathBuf};

use tracing::{Level, Subscriber};
use tracing_subscriber::{layer::Layer, prelude::*};

pub use profile_util::*;

/// Prints `tracing` and `log` events of levels INFO, WARN and ERROR in debug builds, logs errors to `example_name.error.log` in release builds.
pub fn print_info() {
    if cfg!(debug_assertions) {
        tracing_print(Level::INFO)
    } else {
        tracing_error(Level::INFO)
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

fn tracing_error(max: Level) {
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
            let mut file = std::env::current_exe()?;
            file.set_extension(".error.log");
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
