mod app;
mod button;
mod window;

use webrender::api::ColorF;

fn main() {
    app::App::new()
        .window("window1", ColorF::new(0.1, 0.2, 0.3, 1.0))
        .window("window2", ColorF::new(0.3, 0.2, 0.1, 1.0))
        .run();
}
