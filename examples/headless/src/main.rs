//! Demonstrates headless apps, image and video rendering.

mod headless;
mod image;
mod video;

fn main() {
    zng::env::init!();
    match std::env::args().nth(1).unwrap_or_default().as_str() {
        "image" => image::run(),
        "video" => video::run(),
        _ => headless::run(),
    }
}
