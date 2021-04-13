#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

fn main() {
    App::default().run_window(|_| {
        window! {
            size = (800, 600);

            background_color = colors::RED;
            background = linear_gradient(0.deg(), [colors::GREEN, colors::GREEN]);

            content = text! {
                text = "expect a green background";
                font_size = 32;
                font_family = ["Consolas", "monospace"];
                color = colors::WHITE;
            };
        }
    })
}
