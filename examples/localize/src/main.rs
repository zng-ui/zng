//! Demonstrates localization service and integration.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use zng::{
    clipboard::{COPY_CMD, PASTE_CMD},
    color::filter::drop_shadow,
    focus::{FocusClickBehavior, alt_focus_scope, focus_click_behavior},
    image,
    l10n::{LangFilePath, LangMap},
    layout::align,
    prelude::*,
};

// l10n-*-### Localize Example
// l10n-*-### This standalone comment is added to all scraped template files.
// l10n-### This standalone comment is only added to the default file.
// l10n-msg-### This standalone comment is only added to the `msg` file.

// Run this command to scrap template:
// cargo do zng l10n -p "zng-example-localize" -o "examples/localize/res/l10n"

#[cfg(not(debug_assertions))]
const EMBEDDED_L10N: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/l10n.tar.gz"));

fn main() {
    zng::env::init_res(concat!(env!("CARGO_MANIFEST_DIR"), "/res"));
    zng::env::init!();

    APP.defaults().run_window(async {
        // load `available_langs`
        #[cfg(debug_assertions)]
        L10N.load_dir(zng::env::res("l10n"));
        #[cfg(not(debug_assertions))]
        L10N.load_tar(EMBEDDED_L10N);

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
            child_top = locale_menu();
            child = window_content();
        }
    })
}

fn window_icon() -> UiNode {
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

fn window_content() -> UiNode {
    let click_count = var(0u32);
    let click_msg = l10n!("msg/click-count", "Clicked {$n} times", n = click_count.clone());

    let test_cmds = [
        NO_META_CMD,
        NOT_LOCALIZED_CMD,
        LOCALIZED_CMD,
        PRIVATE_LOCALIZED_CMD,
        L10N_FALSE_CMD,
        LOCALIZED_FILE_CMD,
        COPY_CMD,
        PASTE_CMD,
    ];
    let handles: Vec<_> = test_cmds.iter().map(|c| c.subscribe(true)).collect();
    handles.leak(); // perm enable commands for test

    Stack! {
        align = Align::CENTER;
        children_align = Align::TOP_START;
        direction = StackDirection::top_to_bottom();
        spacing = 5;
        layout::height = 400;
        children = ui_vec![
            Button! {
                child = Text!(l10n!("button", "Button")); // l10n-# button sets "click-count"
                gesture::on_any_click = hn!(|a| {
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
                children = test_cmds.into_iter().map(|c| Button!(c));
                spacing = 4;
                zng::button::style_fn = Style! {
                    layout::padding = 2;
                };
                layout::width = 200;
            },
            text::Text! {
                layout::margin = (20, 0, 0, 0);
                txt = l10n!("example-shortcuts", "Example Shortcuts:");
                font_weight = FontWeight::SEMIBOLD;
            },
            {
                let shortcut = var(gesture::Shortcuts::new());
                Button! {
                    child = zng::shortcut_text::ShortcutText! {
                        shortcut = shortcut.clone();
                        none_fn = wgt_fn!(|_| Text!(l10n!("no-shortcut", "no shortcut")));
                    };
                    on_click = hn!(|_| {
                        DIALOG.custom(shortcut_input_dialog(shortcut.clone()));
                    });
                }
            }
        ];
    }
}

/// shows current lang and allows selecting one lang.
///
/// Note that in a real UI settings page you want to allows selection of
/// multiple languages on a list that the user can sort, this way missing messages
/// of the top preference can have a better fallback.
fn locale_menu() -> UiNode {
    Container! {
        alt_focus_scope = true;
        focus_click_behavior = FocusClickBehavior::Exit;
        child =
            L10N.available_langs()
                .present(wgt_fn!(|langs: Arc<LangMap<HashMap<LangFilePath, PathBuf>>>| {
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
                        });
                    }
                })),
        ;
    }
}

// l10n--### Another standalone comment, also added to the top of the template file.

// l10n-## Commands

zng::event::command! {
    pub static NO_META_CMD;

    pub static NOT_LOCALIZED_CMD = { name: "Not Localized" };

    pub static LOCALIZED_CMD = {
        l10n!: true,
        name: "Localized",
        info: "Localized in the default file",
    };

    static PRIVATE_LOCALIZED_CMD = {
        l10n!: true,
        name: "Private",
        info: "Private command, public localization text",
    };

    pub static L10N_FALSE_CMD = {
        l10n!: false,
        name: "No L10n",
    };

    pub static LOCALIZED_FILE_CMD = {
        l10n!: "msg",
        name: "Localized File",
        info: "Localized in a named file 'msg'",
    };
}

fn shortcut_input_dialog(output: Var<gesture::Shortcuts>) -> UiNode {
    use gesture::Shortcuts;
    use keyboard::*;
    use layout::*;
    use zng::focus::*;
    use zng::shortcut_text::ShortcutText;
    let pressed = var(Shortcuts::new());
    let is_valid = var(true);
    Container! {
        // l10n-# the [<ENTER>] text must not be translated, it is replaced by a localized shortcut text widget
        child_top =
            l10n!("press-shortcut-msg", "Press the new shortcut and then press [<ENTER>]").present(wgt_fn!(|txt: Txt| {
                let mut items = ui_vec![];
                match txt.split_once("[<ENTER>]") {
                    Some((before, after)) => {
                        items.push(Text!(before.to_txt()));
                        items.push(ShortcutText!(shortcut!(Enter)));
                        items.push(Text!(after.to_txt()));
                    }
                    None => {
                        items.push(Text!(txt));
                        items.push(ShortcutText!(shortcut!(Enter)));
                    }
                }
                Wrap!(items)
            })),
        ;
        child_spacing = 20;
        child = ShortcutText! {
            shortcut = pressed.clone();
            font_size = 3.em();
            align = Align::TOP;
            when !#{is_valid.clone()} {
                font_color = colors::RED;
            }
        };

        on_pre_key_down = hn!(|args| {
            args.propagation().stop();

            match &args.key {
                Key::Enter => {
                    let shortcut = pressed.get();
                    if shortcut.is_empty() || shortcut[0].is_valid() {
                        is_valid.set(true);
                        output.set(shortcut);
                        DIALOG.respond(dialog::Response::ok());
                    } else {
                        is_valid.set(false);
                    }
                }
                Key::Escape => {
                    DIALOG.respond(dialog::Response::cancel());
                }
                _ => {
                    is_valid.set(true); // clear
                    pressed.set(args.editing_shortcut().unwrap());
                }
            }
        });
        align = Align::CENTER;
        height = 150;
        focusable = true;
        focus_on_init = true;
        widget::background_color = color::COLOR_SCHEME_VAR.map(|c| match c {
            color::ColorScheme::Dark => colors::BLACK.with_alpha(90.pct()),
            _ => colors::WHITE.with_alpha(90.pct()),
        });
        padding = 20;
    }
}
