#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::{prelude::*, widgets::text::properties::TextColorVar};

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    //examples_util::print_info();
    // zero_ui_view::run_same_process(app_main);

    zero_ui_view::init();

    // let rec = examples_util::record_profile("profile-shortcuts.json.gz", &[("example", "shortcuts")], |_| true);
    app_main();
    // rec.finish();
}

fn app_main() {
    App::default().run_window(|_| {
        let shortcut_text = var(Text::empty());
        let keypress_text = var(Text::empty());
        let shortcut_color = var(TextColorVar::default_value());

        // examples_util::trace_var!(ctx, ?shortcut_text);
        // examples_util::trace_var!(ctx, ?keypress_text);
        // examples_util::trace_var!(ctx, %shortcut_color);

        window! {
            title = "Shortcuts Example";
            auto_size = true;
            padding = 50;
            start_position = StartPosition::CenterMonitor;
            on_shortcut = hn!(shortcut_text, shortcut_color, |ctx, args: &ShortcutArgs| {
                shortcut_text.set(ctx.vars, args.shortcut.to_text());
                shortcut_color.set(ctx.vars, TextColorVar::default_value());
            });
            on_key_down = hn!(keypress_text, shortcut_text, shortcut_color, |ctx, args: &KeyInputArgs| {
                let mut new_shortcut_text = "not supported";
                if let Some(key) = args.key {
                    if key.is_modifier() {
                        new_shortcut_text = "";
                    }
                    keypress_text.set(ctx.vars, formatx!{"{:?}", key})
                } else {
                    keypress_text.set(ctx.vars, formatx!{"Scan Code: {:?}", args.scan_code})
                }

                shortcut_text.set(ctx.vars, new_shortcut_text);
                shortcut_color.set(ctx.vars, colors::SALMON);
            });
            content = v_stack! {
                items = widgets![
                    text!{
                        align = Alignment::CENTER;
                        font_size = 18.pt();
                        text = "Press a shortcut:";
                    },
                    text! {
                        align = Alignment::CENTER;
                        margin = (10, 0);
                        font_size = 28.pt();
                        color = shortcut_color;
                        text = shortcut_text;
                    },
                    text! {
                        align = Alignment::CENTER;
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
