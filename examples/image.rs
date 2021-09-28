#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

fn main() {
    zero_ui_view::init();
    App::default().run_window(|_| {
        window! {
            title = "Image Example";
            content = image! {
                source = "https://httpbin.org/image"
            };
        }
    })
}
