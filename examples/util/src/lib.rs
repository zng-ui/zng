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

    
    if metadata.target() == "webrender::device::gl" && metadata.line() == Some(2385) {
        return false;
    }

    // suppress webrender warnings:
    // 
    if metadata.target() == "webrender::device::gl" {
        // suppress vertex debug-only warnings.
        // see: https://bugzilla.mozilla.org/show_bug.cgi?id=1615342
        if metadata.line() == Some(2385) {
            return false;
        }

        // Suppress "Cropping texture upload Box2D((0, 0), (0, 1)) to None"
        // This happens when an empty frame is rendered.
        if metadata.line() == Some(4549) {
            return false;
        }
    }

    true
}
