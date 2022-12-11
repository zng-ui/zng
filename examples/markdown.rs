#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

const MD: &str = include_str!("res/markdown/sample.md");

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // let rec = examples_util::record_profile("markdown");

    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(|_| {
        window! {
            title = "Markdown Example";
            child = scroll! {
                mode = ScrollMode::VERTICAL;
                padding = 10;
                child = markdown! {
                    md = MD;
                }
            };
        }
    })
}
