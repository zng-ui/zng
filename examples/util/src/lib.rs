use std::{fs, path::PathBuf};

use tracing::{Level, Subscriber};
use tracing_subscriber::{layer::Layer, prelude::*};

pub use profile_util::*;

/// Prints `tracing` and `log` events of levels INFO, WARN and ERROR.
pub fn print_info() {
    tracing_print(Level::INFO)
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
