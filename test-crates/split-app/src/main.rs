#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use zero_ui::prelude::*;

use std::path::PathBuf;

/// Created by `build.rs`.
fn view_process() -> PathBuf {
    let mut p = std::env::current_exe().unwrap();
    p.set_file_name("split-view");
    p
}

fn main() {
    App::default().view_process_exe(view_process()).run_window(|_| {
        window! {
            title = "My App";
            content = v_stack! {
                spacing = 5;
                items = widgets![
                    btn(),
                    btn(),
                ];
            };
        }
    })
}

fn btn() -> impl Widget {
    let t = var_from("Click Me!");
    let mut count = 0;

    button! {
        on_click = hn!(t, |ctx, _| {
            count += 1;
            let new_txt = formatx!("Clicked {} time{}!", count, if count > 1 {"s"} else {""});
            t.set(ctx, new_txt);
        });
        content = text(t);
    }
}
