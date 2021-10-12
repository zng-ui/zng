#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

fn main() {
    zero_ui_view::run_same_process(app_main);
}

fn app_main() {
    App::default().run_window(|_| {
        window! {
            title = "Same-Process Setup Example";
            content = v_stack! {
                spacing = 5;
                items = widgets![
                    click_counter(),
                    click_counter(),
                    image(),
                ];
            };
        }
    });

    #[cfg(feature = "app_profiler")]
    zero_ui::core::profiler::write_profile("same_process-profile.json", false);
}

fn click_counter() -> impl Widget {
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

fn image() -> impl Widget {
    v_stack! {
        spacing = 3;
        items = widgets![
            strong("Image:"),
            image! { source = "examples/res/window/icon-bytes.png"; size = (32, 32); },
        ];
    }
}
