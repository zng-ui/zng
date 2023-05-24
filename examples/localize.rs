#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use zero_ui::prelude::*;

use zero_ui::core::l10n::{LangMap, Langs, L10N};

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
        let en_us = L10N.lang_resource(lang!("en-US"), "");
        en_us.wait().await;
        tracing::info!("starting with 'en-US' {}", en_us.status().get());
        // hold "en-US" in memory, even if not requested yet.
        en_us.perm();

        Window! {
            // l10n: Main window title
            title = l10n!("window.title", "Localize Example (template)");
            icon = WindowIcon::render(window_icon);
            child = Stack! {
                children = ui_vec![
                    locale_menu(),
                    examples(),
                ]
            }
        }
    })
}

fn examples() -> impl UiNode {
    let click_count = var(0u32);
    let click_msg = l10n!("click-count", "Clicked {$n} times", n = click_count.clone());
    Stack! {
        align = Align::CENTER;
        direction = StackDirection::top_to_bottom();
        spacing = 5;
        children = ui_vec![
            Button! {
                child = Text!(l10n!("button", "Button")); // l10n: button sets "click-count"
                on_any_click = hn!(|a: &ClickArgs| {
                    if a.is_primary() {
                        click_count.set(click_count.get() + 1);
                    } else if a.is_context() {
                        click_count.set(0u32);
                    }
                });
            },
            Text! {
                txt = click_msg;
            }
        ]
    }
}

fn window_icon() -> impl UiNode {
    Text! {
        size = (36, 36);
        font_size = 28;
        font_weight = FontWeight::BOLD;
        txt_align = Align::CENTER;
        txt = l10n!("window.icon", "Lo"); // l10n: first syllable of "Localize"
        drop_shadow = {
            offset: (2, 2),
            blur_radius: 5,
            color: colors::BLACK,
        };
    }
}

fn locale_menu() -> impl UiNode {
    presenter(
        L10N.available_langs(),
        wgt_fn!(|langs: Arc<LangMap<HashMap<Txt, PathBuf>>>| {
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
