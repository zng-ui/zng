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
// cargo do zng l10n "examples/localize/src" "examples/localize/res"

fn main() {
    zng::env::init_res(concat!(env!("CARGO_MANIFEST_DIR"), "/res"));
    zng::env::init!();

    APP.defaults().run_window(async {
        // load `available_langs`
        L10N.load_dir(zng::env::res(""));

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
            child_top = locale_menu(), 0;
            child = window_content();
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

    let test_cmds = [
        NO_META_CMD,
        NOT_LOCALIZED_CMD,
        LOCALIZED_CMD,
        PRIVATE_LOCALIZED_CMD,
        L10N_FALSE_CMD,
        LOCALIZED_FILE_CMD,
    ];
    let handles: Vec<_> = test_cmds.iter().map(|c| c.subscribe(true)).collect();
    handles.leak(); // perm enable commands for test

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

            text::Text! {
                layout::margin = (20, 0, 0, 0);
                txt = l10n!("example-cmds", "Example Commands:");
                font_weight = FontWeight::SEMIBOLD;
            },
            Wrap! {
                children = test_cmds.into_iter().map(|c| Button!(c)).collect::<UiNodeVec>();
                spacing = 4;
                zng::button::style_fn = Style! {
                    layout::padding = 2;
                };
                layout::max_width = 200;
            }
        ];
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

// l10n--### Another standalone comment, also added to the top of the template file.

// l10n-## Commands

zng::event::command! {
    pub static NO_META_CMD;

    pub static NOT_LOCALIZED_CMD = {
        name: "Not Localized",
    };

    pub static LOCALIZED_CMD = {
        l10n!: true,
        name: "Localized",
        info: "Localized in the default file.",
    };

    static PRIVATE_LOCALIZED_CMD = {
        l10n!: true,
        name: "Private",
        info: "Private command, public localization text.",
    };

    pub static L10N_FALSE_CMD = {
        l10n!: false,
        name: "No L10n",
    };

    pub static LOCALIZED_FILE_CMD = {
        l10n!: "msg",
        name: "Localized File",
        info: "Localized in a named file 'msg'."
    };
}
