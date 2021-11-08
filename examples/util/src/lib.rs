use tracing::{Level, Subscriber};
use tracing_subscriber::{layer::Layer, prelude::*};

mod profiler;
pub use profiler::*;

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
        .with(tracing_subscriber::fmt::layer().without_time().pretty())
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
fn filter(level: &Level, metadata: &tracing::Metadata) -> bool {
    if metadata.level() > level {
        return false;
    }

    // suppress webrender vertex debug-only warnings.
    // see: https://bugzilla.mozilla.org/show_bug.cgi?id=1615342
    if metadata.target() == "webrender::device::gl" && metadata.line() == Some(2331) {
        return false;
    }

    true
}
