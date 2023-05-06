#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

/*
To collect template:

cargo run -p zero-ui-l10n-scraper -- -i"examples/localize*" -o"examples/res/localize/template.ftl"
 */

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // let rec = examples_util::record_profile("localize");

    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(async {
        Window! {
            // l10n: Main window title
            title = l10n!("window.title", "Localize Example");
            child_align = Align::CENTER;
            child = Button! {
                child = Text!(l10n!("button", "Button")); // l10n: About button
            }
        }
    })
}
