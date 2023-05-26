#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use zero_ui::prelude::*;

use zero_ui::core::l10n::{Lang, LangMap, L10N};

// l10n-### Localize Example
// l10n-### This standalone comment is added to all scraped template files.

// Run this command to scrap template:
// cargo run -p zero-ui-l10n-scraper -- -i"examples/localize*" -o"examples/res/localize"

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

        // pre-load resources for the first available lang in sys-langs.
        let (lang, handle) = L10N.wait_first(L10N.app_lang().get()).await;
        match lang {
            Some(lang) => tracing::info!("preload {lang}"),
            None => tracing::warn!("no available sys-lang resource, sys-langs: {}", L10N.app_lang().get()),
        }
        handle.perm();

        Window! {
            // l10n-# Main window title
            title = l10n!("window.title", "Localize Example (template)");
            icon = WindowIcon::render(window_icon);
            child = Stack! {
                children = ui_vec![
                    locale_menu(),
                    window_content(),
                ]
            }
        }
    })
}

fn window_icon() -> impl UiNode {
    Text! {
        zero_ui::core::image::render_retain = true;
        size = (36, 36);
        font_size = 28;
        font_weight = FontWeight::BOLD;
        txt_align = Align::CENTER;
        txt_wrap = false;
        txt = l10n!("window.icon", "Lo"); // l10n-# first syllable of "Localize"
        drop_shadow = {
            offset: (2, 2),
            blur_radius: 5,
            color: colors::BLACK,
        };
    }
}

// l10n-## Example Section

fn window_content() -> impl UiNode {
    let click_count = var(0u32);
    let click_msg = l10n!("msg/click-count", "Clicked {$n} times", n = click_count.clone());
    Stack! {
        align = Align::CENTER;
        direction = StackDirection::top_to_bottom();
        spacing = 5;
        children = ui_vec![
            Button! {
                child = Text!(l10n!("button", "Button")); // l10n-# button sets "click-count"
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
            },
        ]
    }
}

fn locale_menu() -> impl UiNode {
    presenter(
        L10N.available_langs(),
        wgt_fn!(|langs: Arc<LangMap<HashMap<Txt, PathBuf>>>| {
            let mut actual = vec![];
            let mut pseudo = vec![];
            let mut template = vec![];

            for key in langs.keys() {
                if key.language.as_str() == "template" {
                    template.push(key);
                } else if key.language.as_str() == "pseudo" {
                    pseudo.push(key);
                } else {
                    actual.push(key);
                }
            }

            tracing::info!(
                "{} langs, {} pseudo and {} template available",
                actual.len(),
                pseudo.len(),
                template.len()
            );

            actual.sort();
            pseudo.sort();
            template.sort();

            let others = pseudo.into_iter().chain(template).map(|l| (l, false));
            let options = actual.into_iter().map(|l| (l, true)).chain(others);

            let selected = L10N.app_lang().map_bidi(|l| l.first().cloned(), |l| l.clone().into());
            Stack! {
                align = Align::TOP_LEFT;
                direction = StackDirection::left_to_right();
                spacing = 5;
                margin = 10;
                toggle::selector = toggle::Selector::single_opt(selected);
                children = options.map(|(l, actual)| {
                    Toggle! {
                        text::font_style = if actual { FontStyle::Normal } else { FontStyle::Italic };
                        child = Text!("{l}");
                        value::<Lang> = l.clone();
                    }
                }).collect::<UiNodeVec>()
            }
        }),
    )
}

// l10n-### Another standalone comment, also added to the top of all template files.
