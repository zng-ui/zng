#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

/// Created by `build.rs`.
const VIEW_PROCESS: &str = "split-view";

fn main() {
    App::default().view_process_exe(VIEW_PROCESS).run_window(|_| {
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
