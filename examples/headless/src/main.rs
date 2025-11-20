//! Demonstrates headless apps, image and video rendering.

mod headless;
mod image;
mod video;

fn main() {
    zng::env::init!();
    // zng::view_process::default::run_same_process(run);
    run();
}
fn run() {
    match std::env::args().nth(1).unwrap_or_default().as_str() {
        "image" => image::run(),
        "video" => video::run(),
        _ => headless::run(),
    }
}

// let the caller or OS handle crashes (can also build without "crash_handler" default feature)
zng::app::crash_handler::crash_handler_config!(|cfg| cfg.no_crash_handler());
