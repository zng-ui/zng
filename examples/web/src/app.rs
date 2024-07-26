use zng::prelude::*;

pub fn run() {
    zng::env::init!();

    let _app = APP.minimal().run_headless(false);

    tracing::info!("Debug tracing logs to console");
    tracing::warn!("Warn example!");
    tracing::error!("Error example!");

    let _web_time = INSTANT.epoch();

    tracing::info!("config path: {}", zng::env::config("").display());
}
