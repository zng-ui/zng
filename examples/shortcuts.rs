#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // let rec = examples_util::record_profile("shortcuts");

    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(async {
        let shortcut_text = var(Txt::empty());
        let keypress_text = var(Txt::empty());
        let shortcut_error = var(false);

        // examples_util::trace_var!(ctx, ?shortcut_text);
        // examples_util::trace_var!(ctx, ?keypress_text);
        // examples_util::trace_var!(ctx, %shortcut_color);

        zero_ui::core::gesture::SHORTCUT_EVENT
            .on_pre_event(app_hn!(
                shortcut_text,
                shortcut_error,
                |args: &zero_ui::core::gesture::ShortcutArgs, _| {
                    if args.repeat_count > 0 {
                        return;
                    }
                    shortcut_text.set(args.shortcut.to_text());
                    shortcut_error.set(false);
                }
            ))
            .perm();
        zero_ui::core::keyboard::KEY_INPUT_EVENT
            .on_pre_event(app_hn!(shortcut_text, keypress_text, shortcut_error, |args: &KeyInputArgs, _| {
                if args.repeat_count > 0 || args.state != KeyState::Pressed {
                    return;
                }
                let mut new_shortcut_text = "not supported";
                if let Some(key) = args.key {
                    if key.is_modifier() {
                        new_shortcut_text = "";
                    }
                    keypress_text.set(formatx! {"{key:?}"})
                } else {
                    keypress_text.set(formatx! {"Scan Code: {:?}", args.scan_code})
                }

                shortcut_text.set(new_shortcut_text);
                shortcut_error.set(true);
            }))
            .perm();

        Window! {
            title = "Shortcuts Example";
            auto_size = true;
            resizable = false;
            auto_size_origin = Point::center();
            padding = 50;
            start_position = StartPosition::CenterMonitor;

            child_align = Align::CENTER;
            child = Stack! {
                direction = StackDirection::top_to_bottom();
                children = ui_vec![
                    Text!{
                        align = Align::CENTER;
                        font_size = 18.pt();
                        txt = "Press a shortcut:";
                    },
                    Text! {
                        align = Align::CENTER;
                        margin = (10, 0);
                        font_size = 28.pt();
                        txt = shortcut_text;

                        when *#{shortcut_error} {
                            txt_color = colors::SALMON;
                        }
                    },
                    Text! {
                        align = Align::CENTER;
                        font_size = 22.pt();
                        font_family = FontName::monospace();
                        txt_color = colors::LIGHT_SLATE_GRAY;
                        txt = keypress_text;
                    }
                ];
            };
        }
    })
}
