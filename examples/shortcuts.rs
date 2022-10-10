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
    App::default().run_window(|_| {
        let shortcut_text = var(Text::empty());
        let keypress_text = var(Text::empty());
        let shortcut_error = var(false);

        // examples_util::trace_var!(ctx, ?shortcut_text);
        // examples_util::trace_var!(ctx, ?keypress_text);
        // examples_util::trace_var!(ctx, %shortcut_color);

        zero_ui::core::gesture::SHORTCUT_EVENT
            .on_pre_event(app_hn!(
                shortcut_text,
                shortcut_error,
                |ctx, args: &zero_ui::core::gesture::ShortcutArgs, _| {
                    if args.is_repeat {
                        return;
                    }
                    shortcut_text.set(ctx, args.shortcut.to_text());
                    shortcut_error.set(ctx, false);
                }
            ))
            .perm();
        zero_ui::core::keyboard::KEY_INPUT_EVENT
            .on_pre_event(app_hn!(
                shortcut_text,
                keypress_text,
                shortcut_error,
                |ctx, args: &KeyInputArgs, _| {
                    if args.is_repeat || args.state != KeyState::Pressed {
                        return;
                    }
                    let mut new_shortcut_text = "not supported";
                    if let Some(key) = args.key {
                        if key.is_modifier() {
                            new_shortcut_text = "";
                        }
                        keypress_text.set(ctx, formatx! {"{key:?}"})
                    } else {
                        keypress_text.set(ctx, formatx! {"Scan Code: {:?}", args.scan_code})
                    }

                    shortcut_text.set(ctx, new_shortcut_text);
                    shortcut_error.set(ctx, true);
                }
            ))
            .perm();

        window! {
            title = "Shortcuts Example";
            auto_size = true;
            resizable = false;
            auto_size_origin = Point::center();
            padding = 50;
            start_position = StartPosition::CenterMonitor;

            content_align = Align::CENTER;
            content = v_stack! {
                items = widgets![
                    text!{
                        align = Align::CENTER;
                        font_size = 18.pt();
                        text = "Press a shortcut:";
                    },
                    text! {
                        align = Align::CENTER;
                        margin = (10, 0);
                        font_size = 28.pt();
                        text = shortcut_text;

                        when *#{shortcut_error} {
                            color = colors::SALMON;
                        }
                    },
                    text! {
                        align = Align::CENTER;
                        font_size = 22.pt();
                        font_family = FontName::monospace();
                        color = colors::LIGHT_SLATE_GRAY;
                        text = keypress_text;
                    }
                ];
            };
        }
    })
}
