//! Demonstrates localization.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use zng::{
    color::filter::drop_shadow,
    focus::{alt_focus_scope, focus_click_behavior, FocusClickBehavior},
    image,
    l10n::LangMap,
    layout::align,
    prelude::*,
    widget::node::presenter,
};

// l10n-*-### Localize Example
// l10n-*-### This standalone comment is added to all scraped template files.
// l10n-### This standalone comment is only added to the default file.
// l10n-msg-### This standalone comment is only added to the `msg` file.

// Run this command to scrap template:
// cargo run -p zng-l10n-scraper -- -i"examples/localize*" -o"examples/res/localize"

use zng::view_process::prebuilt as view_process;

fn main() {
    examples_util::print_info();
    view_process::init();
    zng::app::crash_handler::init_debug();

    // let rec = examples_util::record_profile("localize");

    // view_process::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    APP.defaults().run_window(async {
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
        image::render_retain = true;
        layout::size = (36, 36);
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
                gesture::on_any_click = hn!(|a: &gesture::ClickArgs| {
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

/// shows current lang and allows selecting one lang.
///
/// Note that in a real UI settings page you want to allows selection of
/// multiple languages on a list that the user can sort, this way missing messages
/// of the top preference can have a better fallback.
fn locale_menu() -> impl UiNode {
    Container! {
        alt_focus_scope = true;
        focus_click_behavior = FocusClickBehavior::Exit;
        child = presenter(
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
                    align = Align::TOP_START;
                    direction = StackDirection::start_to_end();
                    spacing = 5;
                    layout::margin = 10;
                    toggle::selector = toggle::Selector::single_opt(selected);
                    children = options.map(|(l, actual)| {
                        Toggle! {
                            text::font_style = if actual { FontStyle::Normal } else { FontStyle::Italic };
                            child = Text!("{l}");
                            value::<zng::l10n::Lang> = l.clone();
                        }
                    }).collect::<UiNodeVec>()
                }
            }),
        )
    }
}

// l10n-### Another standalone comment, also added to the top of all template files.
