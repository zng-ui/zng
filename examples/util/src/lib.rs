use tracing::{Level, Subscriber};
use tracing_subscriber::{layer::Layer, prelude::*};

/// Prints `tracing` and `log` events of level INFO and above.
pub fn print_info() {
    tracing_subscriber::registry()
        .with(CustomFilter)
        .with(tracing_subscriber::fmt::layer().without_time().pretty())
        .init();
}

struct CustomFilter;
impl<S: Subscriber> Layer<S> for CustomFilter {
    fn enabled(&self, metadata: &tracing::Metadata<'_>, _: tracing_subscriber::layer::Context<'_, S>) -> bool {
        if metadata.level() > &Level::INFO {
            return false;
        }

        // suppress webrender vertex debug-only warnings.
        // see: https://bugzilla.mozilla.org/show_bug.cgi?id=1615342
        if metadata.target() == "webrender::device::gl" && metadata.line() == Some(2331) {
            return false;
        }

        true
    }

    fn max_level_hint(&self) -> Option<tracing::metadata::LevelFilter> {
        Some(tracing::metadata::LevelFilter::INFO)
    }
}
