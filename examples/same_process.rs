#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

fn main() {
    if cfg!(debug_assertions) {
        zero_ui_core::app::run_same_process(app_main);
    } else {
        init_view_process();
        app_main();
    }
}

fn app_main() {
    App::default().run_window(|_| {
        window! {
            title = "Same-Process Setup Example";
            content = v_stack! {
                spacing = 5;
                items = widgets![
                    example(),
                    example(),
                ];
            };
        }
    })
}

fn example() -> impl Widget {
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
