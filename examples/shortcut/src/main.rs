//! Small utility that displays the pressed key gestures.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use zng::{font::FontName, layout::align, prelude::*};

fn main() {
    zng::env::init!();
    zng::view_process::default::run_same_process(app_main);
}
fn app_main() {
    APP.defaults().run_window(async {
        let shortcut_text = var(Txt::from_str(""));
        let keypress_text = var(Txt::from_str(""));
        let shortcut_error = var(false);

        // examples_util::trace_var!(ctx, ?shortcut_text);
        // examples_util::trace_var!(ctx, ?keypress_text);
        // examples_util::trace_var!(ctx, %shortcut_color);

        gesture::SHORTCUT_EVENT
            .on_pre_event(app_hn!(shortcut_text, shortcut_error, |args: &gesture::ShortcutArgs, _| {
                if args.repeat_count > 0 {
                    return;
                }
                shortcut_text.set(args.shortcut.to_txt());
                shortcut_error.set(false);
            }))
            .perm();
        keyboard::KEY_INPUT_EVENT
            .on_pre_event(app_hn!(
                shortcut_text,
                keypress_text,
                shortcut_error,
                |args: &keyboard::KeyInputArgs, _| {
                    if args.repeat_count > 0 || args.state != keyboard::KeyState::Pressed {
                        return;
                    }
                    let mut new_shortcut_text = "not supported";
                    let key = args.shortcut_key();
                    if !matches!(&key, keyboard::Key::Unidentified) {
                        if key.is_modifier() {
                            new_shortcut_text = "";
                        }
                        keypress_text.set(formatx!("{:?}", key))
                    } else {
                        keypress_text.set(formatx!("Key Code: {:?}", args.key_code))
                    }

                    shortcut_text.set(new_shortcut_text);
                    shortcut_error.set(true);
                }
            ))
            .perm();

        Window! {
            title = "Shortcut Example";
            auto_size = true;
            resizable = false;
            enabled_buttons = !window::WindowButton::MAXIMIZE;
            auto_size_origin = layout::Point::center();
            padding = 50;
            start_position = window::StartPosition::CenterMonitor;

            child_align = Align::CENTER;
            child = Stack! {
                direction = StackDirection::top_to_bottom();
                children = ui_vec![
                    Text! {
                        align = Align::CENTER;
                        font_size = 18.pt();
                        txt = "Press a shortcut:";
                    },
                    Text! {
                        align = Align::CENTER;
                        layout::margin = (10, 0);
                        font_size = 28.pt();
                        txt = shortcut_text;

                        when *#{shortcut_error} {
                            font_color = web_colors::SALMON;
                        }
                    },
                    Text! {
                        align = Align::CENTER;
                        font_size = 22.pt();
                        font_family = FontName::monospace();
                        font_color = web_colors::LIGHT_SLATE_GRAY;
                        txt = keypress_text;
                    }
                ];
            };
        }
    })
}
