#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    // examples_util::print_info();
    // zero_ui_view::run_same_process(app_main);
    
    zero_ui_view::init();

    // let rec = examples_util::record_profile("profile-button.json.gz", &[("example", "button")], |_| true);
    app_main();
    // rec.finish();
}

fn app_main() {
    App::default().run_window(|_| {
        window! {
            title = "Button Example";
            content = v_stack! {
                spacing = 5;
                items = widgets![
                    example(),
                    example(),
                    disabled(),
                    image_button()
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
        on_double_click = hn!(|_, _| println!("double click!"));
        on_triple_click = hn!(|_, _| println!("triple click!"));
        on_context_click = hn!(|_, _| println!("context click!"));
        content = text(t);
    }
}

fn disabled() -> impl Widget {
    button! {
        on_click = hn!(|_, _| panic!("disabled button"));
        enabled = false;
        content = text("Disabled");
        id = "disabled-btn"
    }
}

fn image_button() -> impl Widget {
    button! {
        on_click = hn!(|_, _| println!("Clicked image button"));
        content = h_stack! {
            items = widgets![
                image! { source = "examples/res/window/icon-bytes.png"; size = (16, 16); },
                text("Click Me!")
            ];
            spacing = 5;
        };
    }
}
