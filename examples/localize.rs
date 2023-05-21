#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::path::PathBuf;
use std::sync::Arc;

use zero_ui::prelude::*;

use zero_ui::core::l10n::{Langs, LangMap, L10N};

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
        // load `available_langs`
        L10N.load_dir("examples/res/localize");

        // set default lang
        L10N.app_lang().set(lang!("en-US"));

        // pre-load "en-US" resources
        let en_us = L10N.lang_resource(lang!("en-US"));
        en_us.wait().await;
        tracing::info!("starting with 'en-US' {}", en_us.status().get());
        // hold "en-US" in memory, even if not requested yet.
        en_us.perm();

        Window! {
            // l10n: Main window title
            title = l10n!("window.title", "Localize Example (template)");
            child = Stack! {
                children = ui_vec![
                    locale_menu(),
                    Button! {
                        align = Align::CENTER;
                        child = Text!(l10n!("button", "Button")); // l10n: About button
                    }
                ]
            }
        }
    })
}

fn locale_menu() -> impl UiNode {
    presenter(
        L10N.available_langs(),
        wgt_fn!(|langs: Arc<LangMap<PathBuf>>| {
            tracing::info!("{} langs available", langs.len());
            Stack! {
                align = Align::TOP_LEFT;
                direction = StackDirection::left_to_right();
                spacing = 5;
                margin = 10;
                toggle::selector = toggle::Selector::single(L10N.app_lang());
                children = langs.keys().map(|l| {
                    Toggle! {
                        child = Text!("{l}");
                        value::<Langs> = l.clone();
                    }
                }).collect::<UiNodeVec>()
            }
        }),
    )
}
