//! Small utility that displays the pressed key gestures.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use zng::{font::FontName, gesture::Shortcuts, layout::align, prelude::*};

fn main() {
    zng::env::init!();
    zng::view_process::prebuilt::run_same_process(app_main);
}
fn app_main() {
    APP.defaults().run_window(async {
        let shortcut = var(Shortcuts::default());
        let keypress_text = var(Txt::from_str(""));

        // examples_util::trace_var!(ctx, ?shortcut_text);
        // examples_util::trace_var!(ctx, ?keypress_text);
        // examples_util::trace_var!(ctx, %shortcut_color);

        gesture::SHORTCUT_EVENT
            .on_pre_event(app_hn!(shortcut, |args: &gesture::ShortcutArgs, _| {
                if args.repeat_count > 0 {
                    return;
                }
                shortcut.set([args.shortcut.clone()]);
            }))
            .perm();
        keyboard::KEY_INPUT_EVENT
            .on_pre_event(app_hn!(shortcut, keypress_text, |args: &keyboard::KeyInputArgs, _| {
                if args.repeat_count > 0 || args.state != keyboard::KeyState::Pressed {
                    return;
                }
                let key = args.shortcut_key();
                if !matches!(&key, keyboard::Key::Unidentified) {
                    keypress_text.set(formatx!("{:?}", key))
                } else {
                    keypress_text.set(formatx!("Key Code: {:?}", args.key_code))
                }

                shortcut.set([]);
            }))
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
                    zng::shortcut_text::ShortcutText! {
                        shortcut;

                        layout::height = 2.em();
                        align = Align::CENTER;
                        layout::margin = (10, 0);
                        font_size = 28.pt();
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
